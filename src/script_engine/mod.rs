use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_json::Map;

use crate::{
    environment::EnvironmentProvider,
    http, parser,
    parser::Selection,
    script_engine::{boa::BoaScriptEngine, report::TestsReport},
    Result,
};

pub mod boa;
pub mod report;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct Value<S> {
    pub state: S,
}

#[derive(Debug)]
pub struct Processed {
    pub value: String,
}

#[derive(Debug)]
#[allow(unused)]
pub enum Unprocessed {
    WithInline {
        value: String,
        inline_scripts: Vec<InlineScript>,
        selection: Selection,
    },
    WithoutInline(String, Selection),
}

#[derive(Debug)]

pub struct InlineScript {
    pub script: String,
    pub placeholder: String,
    #[allow(unused)]
    pub selection: Selection,
}

pub fn create_script_engine(environment: &mut dyn EnvironmentProvider) -> Result<BoaScriptEngine> {
    BoaScriptEngine::new(environment.snapshot())
}

pub struct Script<'a> {
    #[allow(unused)]
    pub selection: Selection,
    pub src: &'a str,
}

impl<'a> Script<'a> {
    pub fn internal_script(src: &str) -> Script {
        Script {
            src,
            selection: Selection::none(),
        }
    }
}

pub trait ScriptEngine {
    fn execute_script(&mut self, script: &Script) -> Result<String>;

    fn reset(&mut self) -> Result<()>;

    fn snapshot(&mut self) -> Result<serde_json::Value>;

    fn report(&mut self) -> Result<TestsReport>;

    fn define_variable(&mut self, name: &str, value: &str) -> Result<()>;
    fn pre_handle(&mut self, script: &Script, request: &parser::Request) -> Result<()>;
    fn handle(&mut self, script: &Script, response: &http::Response) -> Result<()>;

    fn resolve_request_variable(&mut self, name: &str) -> Result<String>;

    fn process(&mut self, value: Value<Unprocessed>) -> Result<Value<Processed>> {
        match value {
            Value {
                state:
                    Unprocessed::WithInline {
                        value,
                        inline_scripts,
                        selection: _selection,
                    },
            } => {
                let mut interpolated = value;
                for inline_script in inline_scripts {
                    let placeholder = inline_script.placeholder.clone();
                    let result = self.resolve_request_variable(&inline_script.script)?;
                    interpolated = interpolated.replacen(placeholder.as_str(), result.as_str(), 1);
                }

                Ok(Value {
                    state: Processed {
                        value: interpolated,
                    },
                })
            }
            Value {
                state: Unprocessed::WithoutInline(value, _),
            } => Ok(Value {
                state: Processed { value },
            }),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Response {
    body: Option<String>,
    headers: Map<String, serde_json::Value>,
    status: u16,
}

impl From<&http::Response> for Response {
    fn from(response: &http::Response) -> Self {
        let mut headers = Map::new();
        for (key, value) in response.headers.as_slice() {
            headers.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        Response {
            body: response.body.clone(),
            headers,
            status: response.status_code,
        }
    }
}

fn handle(engine: &mut dyn ScriptEngine, script: &Script, response: &http::Response) -> Result<()> {
    inject(engine, response)?;
    engine.execute_script(script)?;
    Ok(())
}

fn inject(engine: &mut dyn ScriptEngine, response: &http::Response) -> Result<()> {
    let response: Response = response.into();

    let script = format!(
        "var response = {};",
        serde_json::to_string(&response).unwrap()
    );
    engine.execute_script(&Script::internal_script(&script))?;
    if let Some(body) = response.body {
        if let Ok(serde_json::Value::Object(response_body)) = serde_json::from_str(body.as_str()) {
            let script = format!(
                "response.body = {};",
                serde_json::to_string(&response_body).unwrap()
            );
            engine
                .execute_script(&Script::internal_script(&script))
                .unwrap();
        }
    }
    Ok(())
}

pub fn inject_variable(
    engine: &mut impl ScriptEngine,
    name: &str,
    value: Value<Processed>,
) -> Result<()> {
    engine.define_variable(name, &value.state.value)?;
    Ok(())
}
