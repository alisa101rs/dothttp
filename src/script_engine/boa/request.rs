use boa_engine::{
    object::ObjectInitializer, property::Attribute, Context, JsResult, JsValue, NativeFunction,
};

use crate::{
    parser,
    script_engine::boa::{
        map_js_error,
        variables::{VariableHolder, Variables},
        Environment,
    },
};

///
/// ```typescript
/// declare const request: HttpClientRequest;
///
/// interface HttpClientRequest {
///     /**
///      * Information about current request body
///      */
///     body: RequestBody;
///
///     /**
///      * Information about current request URL
///      */
///     url: RequestUrl;
///     /**
///      * Environment used for sending this request
///      */
///     environment: RequestEnvironment
///
///     /**
///      * Current request variables, which can be updated in Pre-request handler script.
///      * Those variables are not shared between requests.
///      */
///     variables: RequestVariables
///
///     /**
///      * Header of the current request.
///      */
///     headers: RequestHeaders
/// }
/// interface RequestHeaders {
///     /**
///      * Array of all headers
///      */
///     all(): [RequestHeader]
///
///     /**
///      * Searches header by its name, returns null is there is not such header.
///      * @param name header name for searching
///      */
///     findByName(name: string): RequestHeader | null
/// }
///
/// /**
///  * Information about request header
///  */
/// interface RequestHeader {
///     /**
///      * Header name
///      */
///     name: string
///     /**
///      * Gets raw header value, without any substituted variable. So, all {{var}} parts will stay unchanged.
///      */
///     getRawValue(): string;
///
///     /**
///      * Tries substitute known variables inside header value and returns the result. All known {{var}} will be replaced by theirs values.
///      * Unknown {{var}} will stay unchanged.
///      */
///     tryGetSubstitutedValue(): string;
/// }
///
/// ```
///
pub struct Request;

impl Request {
    pub fn register(context: &mut Context, _request: &parser::Request) -> crate::Result<()> {
        let request_environment = RequestEnvironment::create(context)?;
        let request_variables = RequestVariables::create(context)?;
        let mut obj = ObjectInitializer::new(context);

        obj.property("environment", request_environment, Attribute::default());
        obj.property("variables", request_variables, Attribute::default());

        let obj = obj.build();

        context
            .register_global_property("request", obj, Attribute::default())
            .map_err(map_js_error)?;

        Ok(())
    }
}

/// ```typescript
/// /**
///  * Environment used for sending request.
///  * Contains environment variables from http-client.env.json and http-client.private.env.json files.
///  */
/// interface RequestEnvironment {
///     /**
///      * Retrieves variable value by its name. Returns null if there is no such variable.
///      * @param name variable name.
///      */
///     get(name: string): string | null
/// }
/// ```
struct RequestEnvironment;

impl RequestEnvironment {
    pub fn create(context: &mut Context) -> crate::Result<JsValue> {
        let mut obj = ObjectInitializer::new(context);
        obj.function(NativeFunction::from_fn_ptr(Self::get), "get", 1);
        Ok(JsValue::Object(obj.build()))
    }

    pub fn get(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let Some(JsValue::String(key)) = args.first() else {
            return Ok(JsValue::Undefined);
        };
        let key = key.to_std_string_escaped();

        if let Some(value) = Environment::get_variable(&key, ctx) {
            if !value.is_null_or_undefined() {
                return Ok(value);
            }
        }

        Ok(JsValue::Undefined)
    }
}

pub struct RequestVariables;

impl RequestVariables {
    pub fn create(context: &mut Context) -> crate::Result<JsValue> {
        Variables::create::<Self>(context)
    }
}

impl VariableHolder for RequestVariables {
    const NAME: &'static str = "__request_variables";
}
