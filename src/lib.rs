use std::{
    borrow::BorrowMut,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::{
    http_client::{reqwest::ReqwestHttpClient, HttpClient},
    output::Output,
    parser::{parse, Header},
    script_engine::{create_script_engine, ScriptEngine},
};

mod http_client;
pub mod output;
mod parser;
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
        let env_file = match read_to_string(env_file) {
            Ok(script) => Ok(script),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::write(env_file, "{}").unwrap();
                Ok("{}".to_string())
            }
            Err(e) => Err(e),
        }?;

        let snapshot = match read_to_string(snapshot_file) {
            Ok(script) => Ok(script),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("{}".to_string()),
            Err(e) => Err(e),
        }?;

        let engine = create_script_engine(&env_file, env, &snapshot);
        let client = Box::new(ReqwestHttpClient::create(config));

        Ok(Runtime {
            output,
            snapshot_file: PathBuf::from(snapshot_file),
            engine,
            client,
        })
    }

    pub fn execute(&mut self, script_file: &Path, request: Option<usize>) -> Result<()> {
        let file = read_to_string(&script_file)
            .with_context(|| format!("Failed opening script file: {:?}", script_file))?;
        let file = &mut parse(script_file.to_path_buf(), file.as_str())
            .with_context(|| format!("Failed parsing file: {:?}", script_file))?;

        let request_scripts = file.request_scripts(request);

        let engine = &mut *self.engine;
        let outputter = self.output.borrow_mut();
        let client = &self.client;

        for (index, request_script) in request_scripts.enumerate() {
            let request = process(engine, &request_script.request)
                .with_context(|| format!("Failed processing request #{index}"))?;
            outputter
                .request(&request)
                .with_context(|| format!("Failed outputting request #{index}"))?;

            let response = client
                .execute(&request)
                .with_context(|| format!("Error executing request #{index}"))?;
            outputter
                .response(&response)
                .with_context(|| format!("Error outputting response for request #{index}",))?;

            if let Some(parser::Handler { script, selection }) = &request_script.handler {
                engine
                    .handle(
                        &script_engine::Script {
                            selection: selection.clone(),
                            src: script.as_str(),
                        },
                        &response,
                    )
                    .with_context(|| format!("Error handling response for request #{index}",))?;
            }

            engine.reset().unwrap();
        }
        let snapshot = engine
            .snapshot()
            .with_context(|| "Error creating snapshot")?;

        std::fs::write(self.snapshot_file.as_path(), snapshot)
            .with_context(|| "Error writing snapshot")?;

        Ok(())
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

impl From<&parser::Method> for Method {
    fn from(method: &parser::Method) -> Self {
        match method {
            parser::Method::Get(_) => Method::Get,
            parser::Method::Post(_) => Method::Post,
            parser::Method::Delete(_) => Method::Delete,
            parser::Method::Put(_) => Method::Put,
            parser::Method::Patch(_) => Method::Patch,
            parser::Method::Options(_) => Method::Options,
        }
    }
}

pub struct Request {
    pub method: Method,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

pub enum Method {
    Get,
    Post,
    Delete,
    Put,
    Patch,
    Options,
}

pub enum Version {
    Http09,
    Http2,
    Http10,
    Http11,
}

pub struct Response {
    pub version: Version,
    pub status_code: u16,
    pub status: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}
