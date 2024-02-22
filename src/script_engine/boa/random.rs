use boa_engine::{
    object::{FunctionObjectBuilder, ObjectInitializer},
    property::{Attribute, PropertyDescriptor},
    Context, JsNativeError, JsResult, JsString, JsValue, NativeFunction,
};
use rand::distributions::{DistString, Distribution};
use uuid::Uuid;

pub struct Random;

impl Random {
    const ALPHABETIC: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    const HEXADECIMAL: &'static [u8] = b"0123456789ABCDEF";

    pub fn create(context: &mut Context) -> crate::Result<JsValue> {
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
        let Some(&JsValue::Integer(len)) = args.first() else {
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
        let Some(&JsValue::Integer(mut len)) = args.first() else {
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
        let Some(&JsValue::Integer(mut len)) = args.first() else {
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
