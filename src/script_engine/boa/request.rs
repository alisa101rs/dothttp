use std::fmt::Write;

use boa_engine::{
    object::ObjectInitializer, property::Attribute, Context, JsNativeError, JsResult, JsValue,
    NativeFunction,
};

use crate::{
    parser,
    script_engine::boa::{
        map_js_error, resolve_request_variable,
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
///
///
pub struct Request;

impl Request {
    pub fn register(context: &mut Context, request: &parser::Request) -> crate::Result<()> {
        let request_environment = RequestEnvironment::create(context)?;
        let request_variables = RequestVariables::create(context)?;
        let url = ResolvableValue::create(request.target.state.value(), context)?;
        let headers = Headers::create(&request.headers, context)?;
        let body = if let Some(body) = &request.body {
            ResolvableValue::create(body.state.value(), context)?
        } else {
            JsValue::Null
        };

        let mut obj = ObjectInitializer::new(context);

        obj.property("environment", request_environment, Attribute::default());
        obj.property("variables", request_variables, Attribute::default());
        obj.property("url", url, Attribute::default());
        obj.property("body", body, Attribute::default());
        obj.property("headers", headers, Attribute::default());

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

trait ResolvableInterface {
    fn initialize(obj: &mut ObjectInitializer, value: &str) {
        obj.property("__resolvable", value, Attribute::default());
        obj.function(
            NativeFunction::from_fn_ptr(Self::try_get_substituted),
            "tryGetSubstituted",
            0,
        );
        obj.function(
            NativeFunction::from_fn_ptr(Self::try_get_substituted),
            "tryGetSubstitutedValue",
            0,
        );
        obj.function(
            NativeFunction::from_fn_ptr(Self::get_raw_value),
            "getRawValue",
            0,
        );
        obj.function(
            NativeFunction::from_fn_ptr(Self::get_raw_value),
            "getRaw",
            0,
        );
    }
    fn try_get_substituted(
        this: &JsValue,
        _args: &[JsValue],
        ctx: &mut Context,
    ) -> JsResult<JsValue> {
        let error = || {
            JsNativeError::typ()
                .with_message("not a valid object")
                .into()
        };

        let Some(value) = this.as_object() else {
            return Err(error());
        };
        let Some(raw) = value
            .get("__resolvable", ctx)
            .ok()
            .and_then(|it| it.as_string().cloned())
        else {
            return Err(error());
        };

        let value = raw.to_std_string_escaped();

        if !value.contains("{{") {
            return Ok(value.into());
        }

        let mut resolved = String::new();

        let mut rest = value.as_str();

        while !rest.is_empty() {
            if let Some(start) = rest.find("{{") {
                let before = &rest[..start];
                write!(resolved, "{before}").unwrap();

                match rest.find("}}") {
                    None => {
                        write!(resolved, "{rest}").unwrap();
                        rest = "";
                    }
                    Some(end) => {
                        let variable = rest[start + 2..end].trim();
                        rest = &rest[end + 2..];
                        match resolve_request_variable(ctx, variable) {
                            Ok(result) => {
                                write!(resolved, "{result}").unwrap();
                            }
                            Err(_) => {
                                write!(resolved, "{{{{{variable}}}}}").unwrap();
                            }
                        }
                    }
                }
            } else {
                write!(resolved, "{rest}").unwrap();
                rest = "";
            }
        }

        Ok(resolved.into())
    }

    fn get_raw_value(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        if let Some(value) = this.as_object() {
            if let Some(raw) = value
                .get("__resolvable", ctx)
                .ok()
                .and_then(|it| it.as_string().cloned())
            {
                return Ok(raw.clone().into());
            }
        }

        Err(JsNativeError::typ()
            .with_message("not a valid object")
            .into())
    }
}

pub struct ResolvableValue;

impl ResolvableValue {
    pub fn create(placeholder: &str, ctx: &mut Context) -> crate::Result<JsValue> {
        let mut obj = ObjectInitializer::new(ctx);
        Self::initialize(&mut obj, placeholder);

        Ok(JsValue::Object(obj.build()))
    }
}

impl ResolvableInterface for ResolvableValue {}

/// ```typescript
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
/// ```
pub struct Headers;

impl Headers {
    fn create(headers: &[parser::Header], ctx: &mut Context) -> crate::Result<JsValue> {
        let headers: Vec<_> = headers
            .iter()
            .map(|it| {
                Header::create(&it.field_name, it.field_value.state.value(), ctx)
                    .map(|h| (it.field_name.clone(), h))
            })
            .collect::<crate::Result<_>>()?;
        let mut headers_obj = ObjectInitializer::new(ctx);
        for (name, value) in headers {
            headers_obj.property(name, value, Attribute::default());
        }
        let headers = headers_obj.build();

        let mut obj = ObjectInitializer::new(ctx);
        obj.property("__headers", headers, Attribute::default());
        obj.function(
            NativeFunction::from_fn_ptr(Self::find_by_name),
            "findByName",
            1,
        );

        Ok(JsValue::Object(obj.build()))
    }

    fn find_by_name(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        if args.is_empty() {
            return Err(JsNativeError::typ()
                .with_message("invalid amount of arguments")
                .into());
        }
        if let Some(name) = args[0].as_string() {
            let h = this.as_object().cloned().unwrap();
            let headers = h.get("__headers", ctx)?.as_object().cloned().unwrap();

            if let Ok(header) = headers.get(name.clone(), ctx) {
                if !header.is_null_or_undefined() {
                    return Ok(header);
                }
            }
        }

        Ok(JsValue::Null)
    }
}

/// Information about request header
/// ```typescript
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
/// ```
pub struct Header;

impl Header {
    pub fn create(name: &str, value: &str, ctx: &mut Context) -> crate::Result<JsValue> {
        let mut obj = ObjectInitializer::new(ctx);
        <Self as ResolvableInterface>::initialize(&mut obj, value);
        obj.property("name", name, Attribute::default());

        Ok(JsValue::Object(obj.build()))
    }
}

impl ResolvableInterface for Header {}

#[cfg(test)]
mod tests {
    use boa_engine::{property::Attribute, Context, Source};

    use crate::{
        parser,
        parser::InlineScript,
        script_engine::boa::{
            random::Random,
            request::{Headers, ResolvableValue},
            variables::VariableHolder,
            Environment,
        },
    };

    #[test]
    fn resolvable_value() {
        let mut ctx = Context::default();
        Environment::register_holder(&mut ctx).unwrap();

        let v = ResolvableValue::create("///{{test}}_{{test}}///", &mut ctx).unwrap();
        ctx.register_global_property("value", v, Attribute::default())
            .unwrap();

        let result = ctx.eval(Source::from_bytes("value.getRaw()"));
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .to_string(&mut ctx)
                .unwrap()
                .to_std_string_escaped(),
            "///{{test}}_{{test}}///"
        );

        Environment::set_variable("test", "123", &mut ctx).unwrap();
        let result = ctx.eval(Source::from_bytes("value.tryGetSubstituted()"));
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .to_string(&mut ctx)
                .unwrap()
                .to_std_string_escaped(),
            "///123_123///"
        );
    }

    #[test]
    fn headers() {
        let mut ctx = Context::default();
        Environment::register_holder(&mut ctx).unwrap();

        let headers = vec![parser::Header {
            field_name: "test".to_string(),
            field_value: parser::Value {
                state: parser::Unprocessed::WithInline {
                    value: "{{test}}".to_string(),
                    inline_scripts: vec![InlineScript {
                        script: "test".to_string(),
                        placeholder: "{{test}}".to_string(),
                        selection: Default::default(),
                    }],
                    selection: Default::default(),
                },
            },
            selection: parser::Selection::default(),
        }];

        let v = Headers::create(&headers, &mut ctx).unwrap();
        ctx.register_global_property("headers", v, Attribute::default())
            .unwrap();

        let result = ctx.eval(Source::from_bytes("headers.findByName(\"test\")"));
        assert!(result.is_ok());

        Environment::set_variable("test", "123", &mut ctx).unwrap();
        let result = ctx.eval(Source::from_bytes("headers.findByName(\"test\").name"));
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .to_string(&mut ctx)
                .unwrap()
                .to_std_string_escaped(),
            "test"
        );

        let result = ctx.eval(Source::from_bytes(
            "headers.findByName(\"test\").getRawValue()",
        ));
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .to_string(&mut ctx)
                .unwrap()
                .to_std_string_escaped(),
            "{{test}}"
        );

        let result = ctx.eval(Source::from_bytes(
            "headers.findByName(\"test\").tryGetSubstitutedValue()",
        ));
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .to_string(&mut ctx)
                .unwrap()
                .to_std_string_escaped(),
            "123"
        );
    }

    #[test]
    fn resolvable_value_with_special() {
        let mut ctx = Context::default();
        Environment::register_holder(&mut ctx).unwrap();

        let v = ResolvableValue::create("{{ $random.integer }}", &mut ctx).unwrap();
        ctx.register_global_property("value", v, Attribute::default())
            .unwrap();

        let result = ctx.eval(Source::from_bytes("value.getRaw()"));
        assert!(result.is_ok());
        assert_eq!(
            result
                .unwrap()
                .to_string(&mut ctx)
                .unwrap()
                .to_std_string_escaped(),
            "{{ $random.integer }}"
        );

        let result = ctx.eval(Source::from_bytes("value.tryGetSubstituted()"));
        assert!(result.is_ok());
        assert!(result
            .clone()
            .unwrap()
            .to_string(&mut ctx)
            .unwrap()
            .to_std_string_escaped()
            .parse::<i32>()
            .is_err(),);

        let random = Random::create(&mut ctx).unwrap();
        ctx.register_global_property("$random", random, Attribute::default())
            .unwrap();
        let result = ctx.eval(Source::from_bytes("value.tryGetSubstituted()"));
        assert!(result.is_ok());
        assert!(result
            .clone()
            .unwrap()
            .to_string(&mut ctx)
            .unwrap()
            .to_std_string_escaped()
            .parse::<i32>()
            .is_ok(),);
    }
}
