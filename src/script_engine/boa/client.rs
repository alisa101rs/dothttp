use boa_engine::{
    js_string, object::ObjectInitializer, Context, JsError, JsNativeError, JsResult, JsValue,
    NativeFunction,
};
use serde_json::json;

use crate::script_engine::{
    boa,
    boa::{variables::Variables, Environment},
};

pub struct Client;

impl Client {
    pub fn create(context: &mut Context) -> crate::Result<JsValue> {
        let mut client = ObjectInitializer::new(context);

        client.function(NativeFunction::from_fn_ptr(Client::log), "log", 1);
        client.function(NativeFunction::from_fn_ptr(Client::test), "test", 2);
        client.function(NativeFunction::from_fn_ptr(Client::assert), "assert", 2);

        let client = client.build();

        client
            .create_data_property(
                "global".to_string(),
                Variables::create::<Environment>(context)?,
                context,
            )
            .map_err(boa::map_js_error)?;

        Ok(JsValue::Object(client))
    }

    fn test(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let Some(JsValue::String(test_name)) = args.first() else {
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
        let Some(JsValue::Boolean(condition)) = args.first() else {
            return Err(JsNativeError::typ()
                .with_message("Expected to get assert condition")
                .into());
        };

        let message = args
            .get(1)
            .and_then(|it| it.as_string())
            .cloned()
            .unwrap_or_else(|| js_string!("Assertion failed"));

        if !condition {
            return Err(JsError::from_opaque(JsValue::String(
                format!("Assertion failed: {}", message.to_std_string_escaped()).into(),
            )));
        }

        Ok(JsValue::Null)
    }

    fn log(_this: &JsValue, args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        if let Some(JsValue::String(message)) = args.first() {
            println!("{}", message.to_std_string_escaped())
        }
        Ok(JsValue::Undefined)
    }

    pub fn timestamp() -> JsValue {
        use std::time::{SystemTime, UNIX_EPOCH};

        use boa_engine::JsBigInt;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        JsValue::BigInt(JsBigInt::new(timestamp.as_secs()))
    }

    pub fn iso_timestamp() -> JsValue {
        use boa_engine::JsString;
        use chrono::prelude::Local;

        JsValue::String(JsString::from(Local::now().to_rfc3339()))
    }
}
