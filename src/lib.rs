use std::borrow::BorrowMut;

use color_eyre::{eyre::Context, Report};

pub use crate::{
    environment::{EnvironmentFileProvider, EnvironmentProvider, StaticEnvironmentProvider},
    source::SourceProvider,
};
use crate::{
    executor::Executor,
    http::{reqwest::ReqwestHttpClient, HttpClient},
    output::Output,
    script_engine::{boa::BoaScriptEngine, create_script_engine, ScriptEngine},
};

mod environment;
mod executor;
mod http;
pub mod output;
pub(crate) mod parser;
mod script_engine;
pub mod source;

pub type Error = Report;
pub type Result<T> = color_eyre::Result<T>;

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

pub struct Runtime<'a, E, O> {
    engine: BoaScriptEngine,
    environment: &'a mut E,
    output: &'a mut O,
    client: ReqwestHttpClient,
}

impl<'a, E, O> Runtime<'a, E, O>
where
    E: EnvironmentProvider,
    O: Output,
{
    pub fn new(environment: &'a mut E, output: &'a mut O, config: ClientConfig) -> Result<Self> {
        let engine = create_script_engine(environment)?;
        let client = ReqwestHttpClient::create(config);

        Ok(Runtime {
            output,
            environment,
            engine,
            client,
        })
    }

    pub async fn execute(&mut self, mut source_provider: impl SourceProvider) -> Result<()> {
        let mut errors = vec![];

        let engine = &mut self.engine;
        let output = self.output.borrow_mut();
        let client = &self.client;

        for source in source_provider.requests() {
            let (name, report) = Executor::new(source)
                .execute(client, engine, output)
                .await?;

            errors.extend(report.failed().map(|(k, _)| (name.clone(), k.clone())));

            engine.reset().unwrap();
        }

        let snapshot = engine
            .snapshot()
            .with_context(|| "Error creating snapshot")?;

        self.environment
            .save(&snapshot)
            .with_context(|| "Error writing snapshot")?;

        if !errors.is_empty() {
            return Err(produce_error(errors));
        }
        Ok(())
    }
}

fn produce_error(errors: Vec<(String, String)>) -> Report {
    let reports = errors
        .into_iter()
        .map(|(request, test)| Report::msg(format!("{request}: {test}")))
        .collect::<Vec<_>>();

    let mut reports = reports.into_iter().rev();
    let first = reports.next().unwrap();

    let chain = reports.fold(first, |acc, next| acc.wrap_err(next));

    Report::wrap_err(chain, "Some of the unit tests failed")
}
