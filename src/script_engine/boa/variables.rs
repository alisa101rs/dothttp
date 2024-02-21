use boa_engine::{
    object::ObjectInitializer, property::Attribute, Context, JsNativeError, JsObject, JsResult,
    JsString, JsValue, NativeFunction,
};

use crate::script_engine::boa::map_js_error;

/// ```typescript
/// interface Variables {
///     /**
///      * Saves variable with name 'varName' and sets its value to 'varValue'.
///      */
///     set(varName: string, varValue: string): void;
///
///     /**
///      * Returns value of variable 'varName'.
///      */
///     get(varName: string): string;
///
///     /**
///      * Checks no variables are defined.
///      */
///     isEmpty(): boolean;
///
///     /**
///      * Removes variable 'varName'.
///      * @param varName {string}
///      */
///     clear(varName: string): void;
///
///     /**
///      * Removes all variables.
///      */
///     clearAll(): void;
/// }
/// ```
pub struct Variables;

impl Variables {
    pub fn create<T: VariableHolder>(context: &mut Context) -> crate::Result<JsValue> {
        T::register_holder(context)?;

        let mut global = ObjectInitializer::new(context);

        global.property("__name", JsString::from(T::NAME), Attribute::default());
        global.function(NativeFunction::from_fn_ptr(Variables::get), "get", 1);
        global.function(NativeFunction::from_fn_ptr(Variables::set), "set", 2);
        global.function(NativeFunction::from_fn_ptr(Variables::clear), "clear", 1);
        global.function(
            NativeFunction::from_fn_ptr(Variables::is_empty),
            "isEmpty",
            0,
        );
        global.function(
            NativeFunction::from_fn_ptr(Variables::clear_all),
            "clearAll",
            0,
        );

        Ok(JsValue::Object(global.build()))
    }

    fn get_name(this: &JsValue, ctx: &mut Context) -> JsResult<JsString> {
        let Some(this) = this.as_object() else {
            return Err(JsNativeError::typ()
                .with_message("'this' is not a Variables object")
                .into());
        };
        Ok(this.get("__name", ctx)?.as_string().unwrap().clone())
    }

    pub(crate) fn get(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let name = Self::get_name(this, ctx)?;
        let Some(JsValue::String(key)) = args.first() else {
            return Ok(JsValue::Undefined);
        };

        if let Ok(JsValue::Object(env)) = ctx.global_object().get(name, ctx) {
            let value = env.get(key.clone(), ctx)?;
            if !value.is_null_or_undefined() {
                return Ok(value);
            }
        }

        Ok(JsValue::Undefined)
    }
    fn set(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
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
        let name = Self::get_name(this, ctx)?;
        if let Ok(JsValue::Object(snapshot)) = ctx.global_object().get(name, ctx) {
            snapshot.set(key.clone(), value.clone(), false, ctx)?;
        }

        Ok(JsValue::Undefined)
    }

    fn clear(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let Some(JsValue::String(key)) = args.first() else {
            return Err(JsNativeError::typ()
                .with_message("One argument is required")
                .into());
        };
        let name = Self::get_name(this, ctx)?;

        if let Ok(JsValue::Object(snapshot)) = ctx.global_object().get(name, ctx) {
            snapshot.delete_property_or_throw(key.clone(), ctx)?;
        }

        Ok(JsValue::Undefined)
    }

    fn clear_all(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let name = Self::get_name(this, ctx)?.to_std_string_escaped();

        ctx.global_object()
            .set(name, JsValue::Object(JsObject::default()), false, ctx)?;

        Ok(JsValue::Undefined)
    }

    fn is_empty(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let name = Self::get_name(this, ctx)?.to_std_string_escaped();
        if let Ok(JsValue::Object(snapshot)) = ctx.global_object().get(name, ctx) {
            return match JsValue::Object(snapshot).to_json(ctx)? {
                serde_json::Value::Object(m) => Ok(JsValue::Boolean(m.is_empty())),
                _ => Ok(true.into()),
            };
        }

        Ok(true.into())
    }
}

pub(crate) trait VariableHolder {
    const NAME: &'static str;
    fn register_holder(ctx: &mut Context) -> crate::Result<()> {
        ctx.register_global_property(
            Self::NAME,
            JsValue::Object(JsObject::default()),
            Attribute::default() | Attribute::WRITABLE,
        )
        .map_err(map_js_error)?;

        Ok(())
    }
    fn set_variable(key: &str, value: &str, ctx: &mut Context) -> crate::Result<()> {
        let obj = ctx
            .global_object()
            .get(Self::NAME, ctx)
            .unwrap()
            .as_object()
            .cloned()
            .unwrap();
        obj.set(key, value, false, ctx).map_err(map_js_error)?;

        Ok(())
    }
    fn get_variable(key: &str, ctx: &mut Context) -> Option<JsValue> {
        let obj = ctx
            .global_object()
            .get(Self::NAME, ctx)
            .unwrap()
            .as_object()
            .cloned()
            .unwrap_or_else(JsObject::default);

        obj.get(key, ctx).ok().and_then(|it| {
            if it.is_null() || it.is_undefined() {
                None
            } else {
                Some(it)
            }
        })
    }
    fn get_values(ctx: &mut Context) -> crate::Result<JsObject> {
        Ok(ctx
            .global_object()
            .get(Self::NAME, ctx)
            .unwrap()
            .as_object()
            .cloned()
            .unwrap_or_else(JsObject::default))
    }
}

#[cfg(test)]
mod tests {
    use boa_engine::{property::Attribute, Context, Source};

    use super::{VariableHolder, Variables};
    use crate::{script_engine::boa::map_js_error, Result};

    pub struct TestHolder;
    impl VariableHolder for TestHolder {
        const NAME: &'static str = "__test";
    }
    #[test]
    fn full_interface() -> Result<()> {
        let mut ctx = Context::default();
        let global = Variables::create::<TestHolder>(&mut ctx)?;
        ctx.register_global_property("global", global.clone(), Attribute::default())
            .map_err(map_js_error)?;

        let empty = ctx
            .eval(Source::from_bytes("global.isEmpty()"))
            .map_err(map_js_error)?
            .to_boolean();
        assert!(empty);

        let result = ctx
            .eval(Source::from_bytes("global.clearAll()"))
            .map_err(map_js_error)?;
        assert!(result.is_undefined());

        let result = ctx
            .eval(Source::from_bytes(
                "global.set(\"variable\", \"from_test\")",
            ))
            .map_err(map_js_error)?;
        assert!(result.is_undefined());

        let value = ctx
            .eval(Source::from_bytes("global.get(\"variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_string(), "{:?} is not string", value);

        assert_eq!(
            value.to_string(&mut ctx).unwrap().to_std_string_escaped(),
            "from_test"
        );

        let empty = ctx
            .eval(Source::from_bytes("global.isEmpty()"))
            .map_err(map_js_error)?
            .to_boolean();
        assert!(!empty);

        let value = ctx
            .eval(Source::from_bytes("global.get(\"another_variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_undefined());

        let result = ctx
            .eval(Source::from_bytes("global.clear(\"variable\")"))
            .map_err(map_js_error)?;
        assert!(result.is_undefined());

        let empty = ctx
            .eval(Source::from_bytes("global.isEmpty()"))
            .map_err(map_js_error)?
            .to_boolean();
        assert!(empty);

        let value = ctx
            .eval(Source::from_bytes("global.get(\"variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_undefined());

        ctx
            .eval(Source::from_bytes(
                "global.set(\"variable\", \"from_test\"); global.set(\"another_variable\", \"from_test\");",
            ))
            .map_err(map_js_error)?;

        let empty = ctx
            .eval(Source::from_bytes("global.isEmpty()"))
            .map_err(map_js_error)?
            .to_boolean();
        assert!(!empty);

        let value = ctx
            .eval(Source::from_bytes("global.get(\"variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_string());

        assert_eq!(
            value.to_string(&mut ctx).unwrap().to_std_string_escaped(),
            "from_test"
        );

        let value = ctx
            .eval(Source::from_bytes("global.get(\"another_variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_string());

        assert_eq!(
            value.to_string(&mut ctx).unwrap().to_std_string_escaped(),
            "from_test"
        );

        let result = ctx
            .eval(Source::from_bytes("global.clearAll()"))
            .map_err(map_js_error)?;
        assert!(result.is_undefined());

        let value = ctx
            .eval(Source::from_bytes("global.get(\"variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_undefined());

        let value = ctx
            .eval(Source::from_bytes("global.get(\"another_variable\")"))
            .map_err(map_js_error)?;
        assert!(value.is_undefined());

        let empty = ctx
            .eval(Source::from_bytes("global.isEmpty()"))
            .map_err(map_js_error)?
            .to_boolean();
        assert!(empty);

        Ok(())
    }
}
