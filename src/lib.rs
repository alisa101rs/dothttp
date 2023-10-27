use std::{
    borrow::BorrowMut,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use serde_json::Value;

use crate::{
    http::{reqwest::ReqwestHttpClient, HttpClient, Request},
    output::Output,
    parser::{parse, Header, RequestScript},
    script_engine::{create_script_engine, ScriptEngine},
};

mod http;
pub mod output;
pub(crate) mod parser;
mod script_engine;

pub type Result<T> = anyhow::Result<T>;

pub struct ClientConfig {
    pub ssl_check: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self { ssl_check: true }
    }
}

impl ClientConfig {
    pub fn new(ssl_check: bool) -> Self {
        Self { ssl_check }
    }
}

pub struct Runtime<'a> {
    engine: Box<dyn ScriptEngine>,
    snapshot_file: PathBuf,
    output: &'a mut dyn Output,
    client: Box<dyn HttpClient>,
}

impl<'a> Runtime<'a> {
    pub fn new(
        env: &str,
        snapshot_file: &Path,
        env_file: &Path,
        output: &'a mut dyn Output,
        config: ClientConfig,
    ) -> Result<Runtime<'a>> {
        let Value::Object(mut environment) =
            read_json_content(env_file).context("environment deserialization")?
        else {
            return Err(anyhow!("Expected environment file to be a map"));
        };

        let environment = environment
            .remove(env)
            .unwrap_or_else(|| serde_json::json!({}));

        let snapshot = read_json_content(snapshot_file).context("snapshot deserialization")?;

        let engine = create_script_engine(environment, snapshot)?;
        let client = Box::new(ReqwestHttpClient::create(config));

        Ok(Runtime {
            output,
            snapshot_file: PathBuf::from(snapshot_file),
            engine,
            client,
        })
    }

    pub fn execute(
        &mut self,
        files: impl IntoIterator<Item = PathBuf>,
        request: Option<usize>,
    ) -> Result<()> {
        let mut errors = vec![];

        let engine = &mut *self.engine;
        let output = self.output.borrow_mut();
        let client = &self.client;

        for script_file in files {
            let file = fs::read_to_string(&script_file)
                .with_context(|| format!("Failed opening script file: {:?}", script_file))?;
            let file = &mut parse(script_file.to_path_buf(), file.as_str())
                .with_context(|| format!("Failed parsing file: {:?}", script_file))?;

            let request_scripts = file.request_scripts(request);

            for (index, request_script) in request_scripts {
                let request_name = Self::section_name(&script_file, request_script, index);
                output.section(&request_name)?;

                let request = process(engine, &request_script.request)
                    .with_context(|| format!("Failed processing request {request_name}"))?;
                output
                    .request(&request)
                    .with_context(|| format!("Failed outputting request {request_name}"))?;

                let response = client
                    .execute(&request)
                    .with_context(|| format!("Error executing request {request_name}"))?;
                output.response(&response).with_context(|| {
                    format!("Error outputting response for request {request_name}",)
                })?;

                if let Some(parser::Handler { script, selection }) = &request_script.handler {
                    engine
                        .handle(
                            &script_engine::Script {
                                selection: selection.clone(),
                                src: script.as_str(),
                            },
                            &response,
                        )
                        .with_context(|| {
                            format!("Error handling response for request {request_name}",)
                        })?;

                    let test_report = engine.report().context("failed to get test report")?;
                    errors.extend(test_report.failed().map(|(k, _)| k.clone()));
                    output
                        .tests(&test_report)
                        .context("Failed outputting tests report")?;
                }

                engine.reset().unwrap();
            }
        }

        let snapshot = engine
            .snapshot()
            .with_context(|| "Error creating snapshot")?;

        fs::write(
            self.snapshot_file.as_path(),
            serde_json::to_vec(&snapshot).unwrap(),
        )
        .with_context(|| "Error writing snapshot")?;

        if !errors.is_empty() {
            let failed_tests = errors.join(", ");
            return Err(anyhow! { "failed tests {failed_tests}" });
        }
        Ok(())
    }

    fn section_name(file: &Path, request: &RequestScript, index: usize) -> String {
        let filename = file
            .file_name()
            .and_then(|it| it.to_str())
            .unwrap_or_else(|| "");
        if let Some(name) = &request.name {
            format!("{filename} / {name}")
        } else {
            format!("{filename} / #{}", index + 1)
        }
    }
}

fn process_header(engine: &mut dyn ScriptEngine, header: &Header) -> Result<(String, String)> {
    let Header {
        field_name,
        field_value,
        ..
    } = header;
    engine
        .process(field_value.into())
        .map(|value| (field_name.clone(), value.state.value))
}

fn process_headers(
    engine: &mut dyn ScriptEngine,
    headers: &[Header],
) -> Result<Vec<(String, String)>> {
    headers
        .iter()
        .map(|header| process_header(engine, header))
        .collect()
}

fn process(engine: &mut dyn ScriptEngine, request: &parser::Request) -> Result<Request> {
    let parser::Request {
        method,
        target,
        headers,
        body,
        ..
    } = request;
    let headers = process_headers(engine, headers)?;
    Ok(Request {
        method: method.into(),
        target: engine
            .process(target.into())
            .with_context(|| format!("Failed processing: {}", target))?
            .state
            .value,
        headers,
        body: match body {
            None => None,
            Some(body) => Some(engine.process(body.into())?.state.value),
        },
    })
}

impl From<&parser::InlineScript> for script_engine::InlineScript {
    fn from(inline_script: &parser::InlineScript) -> Self {
        let parser::InlineScript {
            script,
            placeholder,
            selection,
        } = inline_script;
        script_engine::InlineScript {
            script: script.clone(),
            placeholder: placeholder.clone(),
            selection: selection.clone(),
        }
    }
}

impl From<&parser::Unprocessed> for script_engine::Unprocessed {
    fn from(state: &parser::Unprocessed) -> Self {
        match state {
            parser::Unprocessed::WithInline {
                value,
                inline_scripts,
                selection,
            } => script_engine::Unprocessed::WithInline {
                value: value.clone(),
                inline_scripts: inline_scripts.iter().map(|script| script.into()).collect(),
                selection: selection.clone(),
            },
            parser::Unprocessed::WithoutInline(value, selection) => {
                script_engine::Unprocessed::WithoutInline(value.clone(), selection.clone())
            }
        }
    }
}

impl From<&parser::Value> for script_engine::Value<script_engine::Unprocessed> {
    fn from(value: &parser::Value) -> Self {
        let parser::Value { state } = value;
        script_engine::Value {
            state: state.into(),
        }
    }
}

fn read_json_content(path: &Path) -> Result<Value> {
    match fs::read(path) {
        Ok(data) => Ok(serde_json::from_slice(&data).context("json deserialization")?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(e) => Err(anyhow!("IO Error: {e}")),
    }
}
