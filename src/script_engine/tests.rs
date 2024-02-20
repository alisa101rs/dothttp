use serde_json::json;

use crate::{
    http::{Response, Version},
    script_engine::{create_script_engine, inject, Script},
    StaticEnvironmentProvider,
};

#[test]
fn test_syntax_error() {
    let mut env = StaticEnvironmentProvider::new(json!({}));
    let mut engine = create_script_engine(&mut env).unwrap();

    let result = engine.execute_script(&Script::internal_script("..test"));

    assert!(
        result.is_err(),
        "Should've been an error, but instead got:\n {:#?}",
        result
    );
}

#[test]
fn test_parse_error() {
    let mut env = StaticEnvironmentProvider::new(json!({}));
    let mut engine = create_script_engine(&mut env).unwrap();

    let result = engine.execute_script(&Script::internal_script(".test"));

    assert!(
        result.is_err(),
        "Should've been an error, but instead got:\n {:#?}",
        result
    );
    if let Err(error) = result {
        // Different engine return different errors, checking both
        assert!(
            error.to_string().contains("ParsingError") || error.to_string().contains("SyntaxError"),
            "Should've been a parsing error, but instead got:\n {:#?}",
            error
        );
    }
}

#[test]
fn test_initialize() {
    let mut env = StaticEnvironmentProvider::new(json! ( { "a": "1"} ));
    let mut engine = create_script_engine(&mut env).unwrap();

    let result = engine.execute_script(&Script::internal_script("a"));

    assert!(result.is_ok());

    if let Ok(result_value) = result {
        assert!(result_value == "1");
    }
}

#[test]
fn test_reset() {
    let mut env = StaticEnvironmentProvider::new(json! ( { "a": "1"} ));
    let mut engine = create_script_engine(&mut env).unwrap();

    engine
        .execute_script(&Script::internal_script(
            r#"client.global.set("test", "test_value")"#,
        ))
        .unwrap();

    engine.reset().unwrap();

    let result = engine.execute_script(&Script::internal_script("test"));

    assert!(result.is_ok());

    if let Ok(result_value) = result {
        assert_eq!(result_value, "test_value");
    }
}

#[test]
fn test_headers_available_in_response() {
    let mut env = StaticEnvironmentProvider::new(json!({}));
    let mut engine = create_script_engine(&mut env).unwrap();

    let headers = vec![("X-Auth-Token".to_string(), "SomeTokenValue".to_string())];

    let response = Response {
        version: Version::Http09,
        headers,
        body: Some("{}".to_string()),
        status_code: 0,
        status: "".to_string(),
    };

    inject(engine.as_mut(), &response).unwrap();

    let result = engine
        .execute_script(&Script::internal_script("response.headers['X-Auth-Token']"))
        .unwrap();

    assert_eq!("SomeTokenValue", result);
}
