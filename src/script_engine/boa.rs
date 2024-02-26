mod client;
mod random;
mod request;
mod variables;

use std::convert::From;

use boa_engine::{property::Attribute, Context, JsError, JsValue, Source};
use client::Client;
use color_eyre::eyre::{anyhow, Context as _};
use random::Random;
use serde_json::{json, Value};

use crate::{
    http::Response,
    parser,
    script_engine::{
        boa::{
            request::{Request, RequestVariables},
            variables::VariableHolder,
        },
        handle,
        report::TestsReport,
        Script, ScriptEngine,
    },
    Result,
};

pub struct BoaScriptEngine {
    context: Context<'static>,
    environment: Value,
}

impl BoaScriptEngine {
    pub fn new(environment: Value) -> Result<BoaScriptEngine> {
        let context = Context::default();

        let mut engine = BoaScriptEngine {
            context,
            environment,
        };

        Self::register_client_object(&mut engine.context)?;
        Self::register_random_object(&mut engine.context)?;
        VariableBlock::register_holder(&mut engine.context)?;
        Self::register_global_environment(&mut engine.context, &engine.environment)?;
        Self::register_global_json_object("_tests", &json!({}), &mut engine.context)?;

        Ok(engine)
    }

    fn register_global_environment(context: &mut Context, environment: &Value) -> Result<()> {
        Environment::register_holder(context)?;

        for (k, v) in environment
            .as_object()
            .ok_or_else(|| anyhow!("Expected environment to be an object"))?
        {
            if k == "client" {
                return Err(anyhow!(
                    "Can't register environment value with the name `client`"
                ));
            }
            if let Some(value) = v.as_str() {
                Environment::set_variable(k, value, context)?;
            }
        }

        Ok(())
    }

    fn register_global_json_object(key: &str, value: &Value, context: &mut Context) -> Result<()> {
        let value = JsValue::from_json(value, context).map_err(map_js_error)?;

        context
            .register_global_property(key.to_owned(), value, Attribute::WRITABLE)
            .map_err(map_js_error)
            .context("could not register global property")?;
        Ok(())
    }

    fn register_client_object(context: &mut Context) -> Result<()> {
        let client = Client::create(context)?;

        context
            .register_global_property("client".to_string(), client, Default::default())
            .map_err(map_js_error)
            .context("could not register global property")?;

        Ok(())
    }

    fn register_random_object(context: &mut Context) -> Result<()> {
        let random = Random::create(context)?;

        context
            .register_global_property("$random", random, Attribute::READONLY)
            .map_err(map_js_error)?;

        context
            .register_global_property("$timestamp", Client::timestamp(), Attribute::READONLY)
            .map_err(map_js_error)?;

        context
            .register_global_property(
                "$isoTimestamp",
                Client::iso_timestamp(),
                Attribute::READONLY,
            )
            .map_err(map_js_error)?;
        Ok(())
    }
}

impl ScriptEngine for BoaScriptEngine {
    fn execute_script(&mut self, script: &Script) -> Result<String> {
        execute_script(&mut self.context, script.src)
    }

    fn empty(&self) -> String {
        String::from("{}")
    }

    fn reset(&mut self) -> Result<()> {
        let snapshot = self.snapshot()?;

        *self = BoaScriptEngine::new(snapshot)?;
        Ok(())
    }

    fn snapshot(&mut self) -> Result<Value> {
        let snapshot = JsValue::Object(Environment::get_values(&mut self.context)?);

        let json = snapshot
            .to_json(&mut self.context)
            .expect("a valid json from js runtime");

        Ok(json)
    }

    fn report(&mut self) -> Result<TestsReport> {
        let tests = self
            .context
            .global_object()
            .get("_tests", &mut self.context)
            .expect("valid environment");

        let serialized = tests.to_json(&mut self.context).expect("valid json");

        Ok(serde_json::from_value(serialized).expect("valid model"))
    }

    fn define_variable(&mut self, name: &str, value: &str) -> Result<()> {
        VariableBlock::set_variable(name, value, &mut self.context)?;
        Ok(())
    }
    fn pre_handle(&mut self, script: &Script, request: &parser::Request) -> Result<()> {
        Request::register(&mut self.context, request)?;

        self.execute_script(&Script::internal_script(script.src))?;

        Ok(())
    }
    fn handle(&mut self, request_script: &Script, response: &Response) -> Result<()> {
        handle(self, request_script, response)
    }
    fn resolve_request_variable(&mut self, name: &str) -> Result<String> {
        resolve_request_variable(&mut self.context, name)
    }
}

fn execute_script(ctx: &mut Context, source: &str) -> Result<String> {
    match ctx
        .eval(Source::from_bytes(source))
        .and_then(|value| value.to_string(ctx))
    {
        Ok(r) => Ok(r.to_std_string_escaped()),
        Err(er) => Err(anyhow!("Error executing script: {er}")),
    }
}

fn resolve_request_variable(ctx: &mut Context, name: &str) -> Result<String> {
    if name.starts_with('$') {
        return execute_script(ctx, name);
    }

    let mut value = None;

    value = value.or(RequestVariables::get_variable(name, ctx));
    value = value.or(VariableBlock::get_variable(name, ctx));
    value = value.or(Environment::get_variable(name, ctx));

    if let Some(value) = value {
        return value
            .to_string(ctx)
            .map(|it| it.to_std_string_escaped())
            .map_err(map_js_error);
    }

    // `{{$name}}`
    Ok(format!("{{{{{name}}}}}"))
}

struct Environment;
impl VariableHolder for Environment {
    const NAME: &'static str = "_env";
}

struct VariableBlock;

impl VariableHolder for VariableBlock {
    const NAME: &'static str = "__variables";
}

fn map_js_error(error: JsError) -> crate::Error {
    anyhow!("{error}")
}
