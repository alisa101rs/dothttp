use std::borrow::BorrowMut;

use anyhow::{anyhow, Context};

pub use crate::{
    environment::{EnvironmentFileProvider, EnvironmentProvider, StaticEnvironmentProvider},
    source::SourceProvider,
};
use crate::{
    http::{reqwest::ReqwestHttpClient, HttpClient, Request},
    output::Output,
    parser::{Header, RequestScript},
    script_engine::{create_script_engine, report::TestsReport, ScriptEngine},
};

mod environment;
mod http;
pub mod output;
pub(crate) mod parser;
mod script_engine;
pub mod source;

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

pub struct Runtime<'a, E> {
    engine: Box<dyn ScriptEngine>,
    environment: &'a mut E,
    output: &'a mut dyn Output,
    client: Box<ReqwestHttpClient>,
}

impl<'a, E> Runtime<'a, E>
where
    E: EnvironmentProvider,
{
    pub fn new(
        environment: &'a mut E,
        output: &'a mut dyn Output,
        config: ClientConfig,
    ) -> Result<Runtime<'a, E>> {
        let engine = create_script_engine(environment)?;
        let client = Box::new(ReqwestHttpClient::create(config));

        Ok(Runtime {
            output,
            environment,
            engine,
            client,
        })
    }

    pub async fn execute(&mut self, mut source_provider: impl SourceProvider) -> Result<()> {
        let mut errors = vec![];

        let engine = &mut *self.engine;
        let output = self.output.borrow_mut();
        let client = &self.client;

        for source in source_provider.requests() {
            let request_name = Self::section_name(source.name, source.script, source.index);
            let request = process(engine, &source.script.request)
                .with_context(|| format!("Failed processing request {request_name}"))?;
            output
                .request(&request, &request_name)
                .with_context(|| format!("Failed outputting request {request_name}"))?;
            let response = client
                .execute(&request)
                .await
                .with_context(|| format!("Error executing request {request_name}"))?;

            let report = if let Some(parser::Handler { script, selection }) = &source.script.handler
            {
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

                test_report
            } else {
                TestsReport::default()
            };

            output.response(&response, &report).with_context(|| {
                format!("Error outputting response for request {request_name}",)
            })?;

            engine.reset().unwrap();
        }

        let snapshot = engine
            .snapshot()
            .with_context(|| "Error creating snapshot")?;

        self.environment
            .save(&snapshot)
            .with_context(|| "Error writing snapshot")?;

        if !errors.is_empty() {
            let failed_tests = errors.join(", ");
            return Err(anyhow! { "failed tests {failed_tests}" });
        }
        Ok(())
    }

    fn section_name(request_module_name: &str, request: &RequestScript, index: usize) -> String {
        if let Some(name) = &request.name {
            format!("{request_module_name} / {name}")
        } else {
            format!("{request_module_name} / #{}", index + 1)
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
            .value
            .replace(|c: char| c.is_whitespace(), ""),
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
