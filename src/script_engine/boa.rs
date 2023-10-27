use std::convert::From;

use anyhow::{anyhow, Context as _};
use boa_engine::{
    js_string,
    object::{FunctionObjectBuilder, ObjectInitializer},
    property::{Attribute, PropertyDescriptor},
    Context, JsBigInt, JsError, JsNativeError, JsResult, JsString, JsValue, NativeFunction, Source,
};
use rand::{distributions::DistString, prelude::Distribution};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    http::Response,
    script_engine::{handle, report::TestsReport, Script, ScriptEngine},
    Result,
};

pub struct BoaScriptEngine {
    context: Context<'static>,
    environment: Value,
}

impl BoaScriptEngine {
    pub fn new(environment: Value, snapshot: Value) -> Result<BoaScriptEngine> {
        let context = Context::default();

        let mut engine = BoaScriptEngine {
            context,
            environment,
        };

        Self::register_global_environment(&mut engine.context, &engine.environment, &snapshot)?;
        Self::register_global_json_object("_tests", &json!({}), &mut engine.context)?;

        Self::register_client_object(&mut engine.context)?;
        Self::register_random_object(&mut engine.context)?;

        Ok(engine)
    }

    fn register_global_environment(
        context: &mut Context,
        environment: &Value,
        snapshot: &Value,
    ) -> Result<()> {
        for (k, v) in environment
            .as_object()
            .ok_or_else(|| anyhow!("Expected environment to be an object"))?
        {
            Self::register_global_json_object(k, v, context)?;
        }

        for (k, v) in snapshot
            .as_object()
            .ok_or_else(|| anyhow!("Expected snapshot to be an object"))?
        {
            if k == "client" {
                return Err(anyhow!(
                    "Can't register environment value with the name `client`"
                ));
            }
            Self::register_global_json_object(k, v, context)?;
        }

        Self::register_global_json_object("_env", environment, context)?;
        Self::register_global_json_object("_snapshot", &snapshot, context)?;

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
        match self
            .context
            .eval(Source::from_bytes(script.src))
            .and_then(|value| value.to_string(&mut self.context))
        {
            Ok(r) => Ok(r.to_std_string_escaped()),
            Err(er) => Err(anyhow!("Error executing script: {er}")),
        }
    }

    fn empty(&self) -> String {
        String::from("{}")
    }

    fn reset(&mut self) -> Result<()> {
        let snapshot = self.snapshot()?;

        *self = BoaScriptEngine::new(std::mem::take(&mut self.environment), snapshot)?;
        Ok(())
    }

    fn snapshot(&mut self) -> Result<Value> {
        let snapshot = self
            .context
            .global_object()
            .get("_snapshot", &mut self.context)
            .unwrap();

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

    fn handle(&mut self, request_script: &Script, response: &Response) -> Result<()> {
        handle(self, request_script, response)
    }
}

struct Client;
impl Client {
    fn create(context: &mut Context) -> Result<JsValue> {
        let mut client = ObjectInitializer::new(context);

        client.function(NativeFunction::from_fn_ptr(Client::log), "log", 1);
        client.function(NativeFunction::from_fn_ptr(Client::test), "test", 2);
        client.function(NativeFunction::from_fn_ptr(Client::assert), "assert", 2);

        let client = client.build();

        client
            .create_data_property("global".to_string(), Global::create(context)?, context)
            .map_err(map_js_error)?;

        Ok(JsValue::Object(client))
    }

    fn test(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let Some(JsValue::String(test_name)) = args.get(0) else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get test name")
                .into());
        };

        let Some(JsValue::Object(test_function)) = args.get(1) else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get test function")
                .into());
        };

        let result = test_function.call(this, &[], context);
        let tests_container = context
            .global_object()
            .get("_tests", context)
            .expect("valid environment")
            .as_object()
            .cloned()
            .expect("valid environment");

        let result = match result {
            Ok(_) => {
                json!( { "result": "success" } )
            }
            Err(er) => {
                let error = er.to_string();
                json!( { "result": "error", "error": error })
            }
        };

        tests_container.set(
            test_name.clone(),
            JsValue::from_json(&result, context).expect("valid json"),
            false,
            context,
        )?;

        Ok(JsValue::Undefined)
    }

    fn assert(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let Some(JsValue::Boolean(condition)) = args.get(0) else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get assert condition")
                .into());
        };

        let message = args
            .get(1)
            .and_then(|it| it.as_string())
            .cloned()
            .unwrap_or_else(|| js_string!("Assertion failed").into());

        if !condition {
            return Err(JsError::from_opaque(JsValue::String(
                format!("Assertion failed: {}", message.to_std_string_escaped()).into(),
            )));
        }

        Ok(JsValue::Null)
    }

    fn log(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        if let Some(JsValue::String(message)) = args.get(0) {
            println!("{}", message.to_std_string_escaped())
        }
        Ok(JsValue::Undefined)
    }

    fn timestamp() -> JsValue {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        JsValue::BigInt(JsBigInt::new(timestamp.as_secs()))
    }

    fn iso_timestamp() -> JsValue {
        use chrono::prelude::Local;

        JsValue::String(JsString::from(Local::now().to_rfc3339()))
    }
}

struct Global;

impl Global {
    fn create(context: &mut Context) -> Result<JsValue> {
        let mut global = ObjectInitializer::new(context);

        global.function(NativeFunction::from_fn_ptr(Global::get), "get", 1);
        global.function(NativeFunction::from_fn_ptr(Global::set), "set", 2);

        Ok(JsValue::Object(global.build()))
    }
    fn get(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let Some(JsValue::String(key)) = args.get(0) else {
            return Ok(JsValue::Null);
        };

        if let Some(JsValue::Object(snapshot)) = ctx.global_object().get("_snapshot", ctx).ok() {
            let value = snapshot.get(key.clone(), ctx)?;
            if !value.is_null_or_undefined() {
                return Ok(value);
            }
        }

        if let Some(JsValue::Object(env)) = ctx.global_object().get("_env", ctx).ok() {
            return env.get(key.clone(), ctx);
        }

        Ok(JsValue::Null)
    }
    fn set(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        if args.len() < 2 {
            return Ok(JsValue::Undefined);
        }

        let JsValue::String(key) = &args[0] else {
            return Ok(JsValue::Undefined);
        };
        let value = &args[1];

        if value.is_undefined() {
            return Ok(JsValue::Undefined);
        }

        if let Some(JsValue::Object(snapshot)) = ctx.global_object().get("_snapshot", ctx).ok() {
            snapshot.set(key.clone(), value.clone(), false, ctx)?;
        }

        Ok(JsValue::Undefined)
    }
}

struct Random;

impl Random {
    const ALPHABETIC: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    const HEXADECIMAL: &'static [u8] = b"0123456789ABCDEF";

    fn create(context: &mut Context) -> Result<JsValue> {
        let uuid =
            FunctionObjectBuilder::new(context, NativeFunction::from_fn_ptr(Random::uuid)).build();
        let integer =
            FunctionObjectBuilder::new(context, NativeFunction::from_fn_ptr(Random::integer))
                .build();
        let float =
            FunctionObjectBuilder::new(context, NativeFunction::from_fn_ptr(Random::float)).build();
        let email =
            FunctionObjectBuilder::new(context, NativeFunction::from_fn_ptr(Random::email)).build();

        let mut random = ObjectInitializer::new(context);

        random.accessor("uuid", Some(uuid), None, Attribute::READONLY);
        random.accessor("integer", Some(integer), None, Attribute::READONLY);
        random.accessor("float", Some(float), None, Attribute::READONLY);
        random.accessor("email", Some(email), None, Attribute::READONLY);
        random.function(
            NativeFunction::from_fn_ptr(Random::alphabetic),
            "alphabetic",
            1,
        );
        random.function(
            NativeFunction::from_fn_ptr(Random::alphanumeric),
            "alphanumeric",
            1,
        );
        random.function(
            NativeFunction::from_fn_ptr(Random::hexadecimal),
            "hexadecimal",
            1,
        );

        Ok(JsValue::Object(random.build()))
    }

    /// This dynamic variable generates a new UUID-v4
    fn uuid(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        Ok(JsValue::from(JsString::from(Uuid::new_v4().to_string())))
    }

    /// This dynamic variable generates a random email
    fn email(_this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let mut rng = rand::thread_rng();
        let mut output = String::new();
        rand::distributions::Alphanumeric.append_string(&mut rng, &mut output, 12);
        output.push('@');
        rand::distributions::Alphanumeric.append_string(&mut rng, &mut output, 6);
        output.push('.');
        let alpha = rand::distributions::Uniform::new('a', 'z');
        output.push(alpha.sample(&mut rng));
        output.push(alpha.sample(&mut rng));

        Ok(JsValue::from(JsString::from(output)))
    }

    /// This dynamic variable generates random sequence of letters, digits of length `length`
    fn alphanumeric(
        _this: &JsValue,
        args: &[JsValue],
        _context: &mut Context,
    ) -> JsResult<JsValue> {
        let Some(&JsValue::Integer(len)) = args.get(0) else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get an integer")
                .into());
        };
        let mut rng = rand::thread_rng();

        let output = rand::distributions::Alphanumeric.sample_string(&mut rng, len as usize);

        Ok(JsValue::from(JsString::from(output)))
    }

    /// This dynamic variable generates random sequence of uppercase and lowercase letters of length `length`
    fn alphabetic(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let Some(&JsValue::Integer(mut len)) = args.get(0) else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get an integer")
                .into());
        };
        let mut rng = rand::thread_rng();

        let dist = rand::distributions::Uniform::new(0, Self::ALPHABETIC.len());

        let mut output = String::new();

        while len > 0 {
            output.push(Self::ALPHABETIC[dist.sample(&mut rng)] as char);
            len -= 1;
        }

        Ok(JsValue::from(JsString::from(output)))
    }
    /// This dynamic variable generates random hexadecimal string of length `length`
    fn hexadecimal(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        let Some(&JsValue::Integer(mut len)) = args.get(0) else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get an integer")
                .into());
        };
        let mut rng = rand::thread_rng();

        let dist = rand::distributions::Uniform::new(0, Self::HEXADECIMAL.len());

        let mut output = String::new();

        while len > 0 {
            output.push(Self::HEXADECIMAL[dist.sample(&mut rng)] as char);
            len -= 1;
        }

        Ok(JsValue::from(JsString::from(output)))
    }

    /// This dynamic variable generates random integer
    fn integer(_this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let accessor = FunctionObjectBuilder::new(
            context,
            NativeFunction::from_copy_closure(|_this, args, _ctx| {
                Ok(JsValue::Integer(match args.len() {
                    1 => {
                        let &JsValue::Integer(max) = &args[0] else {
                            return Err(JsNativeError::typ()
                                .with_message("Expected to get an integer")
                                .into());
                        };
                        rand::distributions::Uniform::new(0, max).sample(&mut rand::thread_rng())
                    }
                    2 => {
                        let &JsValue::Integer(min) = &args[0] else {
                            return Err(JsNativeError::typ()
                                .with_message("Expected to get an integer")
                                .into());
                        };
                        let &JsValue::Integer(max) = &args[1] else {
                            return Err(JsNativeError::typ()
                                .with_message("Expected to get an integer")
                                .into());
                        };
                        rand::distributions::Uniform::new(min, max).sample(&mut rand::thread_rng())
                    }
                    _ => rand::random::<i32>(),
                }))
            }),
        )
        .build();

        let to_string = FunctionObjectBuilder::new(
            context,
            NativeFunction::from_copy_closure(|_this, _args, _ctx| {
                Ok(JsValue::Integer(rand::random()))
            }),
        )
        .build();

        accessor.insert_property(
            "toString",
            PropertyDescriptor::builder()
                .configurable(false)
                .enumerable(false)
                .writable(false)
                .value(to_string)
                .build(),
        );

        Ok(JsValue::from(accessor))
    }

    /// This dynamic variable generates random float
    fn float(_this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let accessor = FunctionObjectBuilder::new(
            context,
            NativeFunction::from_copy_closure(|_this, args, _ctx| {
                Ok(JsValue::Rational(match args.len() {
                    1 => {
                        let &JsValue::Rational(max) = &args[0] else {
                            return Err(JsNativeError::typ()
                                .with_message("Expected to get an integer")
                                .into());
                        };
                        rand::distributions::Uniform::new(0.0f64, max)
                            .sample(&mut rand::thread_rng())
                    }
                    2 => {
                        let &JsValue::Rational(min) = &args[0] else {
                            return Err(JsNativeError::typ()
                                .with_message("Expected to get an integer")
                                .into());
                        };
                        let &JsValue::Rational(max) = &args[1] else {
                            return Err(JsNativeError::typ()
                                .with_message("Expected to get an integer")
                                .into());
                        };
                        rand::distributions::Uniform::new(min, max).sample(&mut rand::thread_rng())
                    }
                    _ => rand::random::<f64>(),
                }))
            }),
        )
        .build();

        let to_string = FunctionObjectBuilder::new(
            context,
            NativeFunction::from_copy_closure(|_this, _args, _ctx| {
                Ok(JsValue::Rational(rand::random()))
            }),
        )
        .build();

        accessor.insert_property(
            "toString",
            PropertyDescriptor::builder()
                .configurable(false)
                .enumerable(false)
                .writable(false)
                .value(to_string)
                .build(),
        );

        Ok(JsValue::from(accessor))
    }
}

fn map_js_error(error: JsError) -> anyhow::Error {
    anyhow!("{error}")
}
