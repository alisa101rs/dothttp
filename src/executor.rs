use color_eyre::eyre::Context;

use crate::{
    http::{HttpClient, Request, Response},
    output::Output,
    parser::{self, Header},
    script_engine::{self, report::TestsReport, ScriptEngine},
    source::SourceItem,
    Result,
};

pub(crate) struct Executor<'a> {
    source: SourceItem<'a>,
}

impl<'a> Executor<'a> {
    pub(crate) fn new(source: SourceItem<'a>) -> Self {
        Self { source }
    }

    fn request_name(&self) -> String {
        format!(
            "{} / {}",
            self.source.source_name(),
            self.source.request_name()
        )
    }

    fn process_variables(&self, engine: &mut impl ScriptEngine) -> Result<()> {
        for (variable, value) in &self.source.script.request_variables {
            let processed = engine.process(value.into())?;
            script_engine::inject_variable(engine, variable, processed)?;
        }

        Ok(())
    }

    fn process_header(
        &self,
        engine: &mut impl ScriptEngine,
        header: &Header,
    ) -> Result<(String, String)> {
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
        &self,
        engine: &mut impl ScriptEngine,
        headers: &[Header],
    ) -> Result<Vec<(String, String)>> {
        headers
            .iter()
            .map(|header| self.process_header(engine, header))
            .collect()
    }

    fn pre_process_request(&self, engine: &mut impl ScriptEngine) -> Result<()> {
        let Some(parser::Handler { script, selection }) = &self.source.script.pre_request_handler
        else {
            return Ok(());
        };

        engine
            .pre_handle(
                &script_engine::Script {
                    selection: selection.clone(),
                    src: script.as_str(),
                },
                &self.source.script.request,
            )
            .with_context(|| format!("Error pre handling request {}", self.request_name()))?;

        Ok(())
    }

    fn process_request(&self, engine: &mut impl ScriptEngine) -> Result<Request> {
        let parser::Request {
            method,
            target,
            headers,
            body,
            ..
        } = &self.source.script.request;
        let headers = self.process_headers(engine, headers)?;

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

    fn response_handler(
        &self,
        response: &Response,
        engine: &mut impl ScriptEngine,
    ) -> Result<TestsReport> {
        let Some(parser::Handler { script, selection }) = &self.source.script.handler else {
            return Ok(TestsReport::default());
        };

        engine
            .handle(
                &script_engine::Script {
                    selection: selection.clone(),
                    src: script.as_str(),
                },
                response,
            )
            .with_context(|| {
                format!(
                    "Error handling response for request {}",
                    self.request_name()
                )
            })?;

        engine.report().context("failed to get test report")
    }

    pub(crate) async fn execute<O: Output + ?Sized>(
        &mut self,
        client: &impl HttpClient,
        engine: &mut impl ScriptEngine,
        output: &mut O,
    ) -> Result<(String, TestsReport)> {
        let name = self.request_name();
        self.process_variables(engine)?;
        self.pre_process_request(engine)?;
        let request = self.process_request(engine)?;

        output.request(&request, &name)?;

        let response = client.execute(&request).await?;
        let report = self.response_handler(&response, engine)?;

        output.response(&response, &report)?;

        Ok((name, report))
    }
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
