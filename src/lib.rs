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

pub struct Runtime<'a, E, O: ?Sized> {
    engine: BoaScriptEngine,
    environment: &'a mut E,
    output: &'a mut O,
    client: ReqwestHttpClient,
}

impl<'a, E, O> Runtime<'a, E, O>
where
    E: EnvironmentProvider,
    O: Output + ?Sized,
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
        let engine = &mut self.engine;
        let output = self.output.borrow_mut();
        let client = &self.client;

        let mut files_requests_tests = vec![];

        for source in source_provider.requests() {
            let (_, report) = Executor::new(source)
                .execute(client, engine, output)
                .await?;

            files_requests_tests.push((
                source.source_name().to_owned(),
                source.request_name(),
                report,
            ));

            engine.reset()?;
        }

        output.tests(files_requests_tests)?;

        let snapshot = engine
            .snapshot()
            .with_context(|| "Error creating snapshot")?;

        self.environment
            .save(&snapshot)
            .with_context(|| "Error writing snapshot")?;

        Ok(())
    }
}
