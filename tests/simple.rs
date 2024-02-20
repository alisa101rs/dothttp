use dothttp::{source::FileSourceProvider, ClientConfig, Runtime, StaticEnvironmentProvider};
use serde_json::json;

use crate::common::{formatter, MockHttpBin};

mod common;
#[tokio::test]
async fn test_simple_get() {
    let mut server = MockHttpBin::start().await;
    let mut output = formatter();
    let mut environment = StaticEnvironmentProvider::new(
        json!({ "host": format!("{}", server.addr), "variable": "42" }),
    );
    let mut runtime = Runtime::new(&mut environment, &mut output, ClientConfig::default()).unwrap();

    let result = runtime
        .execute(FileSourceProvider::new("tests/requests/simple-get.http", Some(1)).unwrap())
        .await;

    assert!(result.is_ok(), "Failed test:\n{}", output.into_writer().0);

    assert_eq!(server.requests().await.len(), 1);
}

#[tokio::test]
async fn test_simple_post() {
    let mut server = MockHttpBin::start().await;
    let mut output = formatter();
    let mut environment = StaticEnvironmentProvider::new(
        json!({ "host": format!("{}", server.addr), "variable": "42", "another_variable": "9000" }),
    );
    let mut runtime = Runtime::new(&mut environment, &mut output, ClientConfig::default()).unwrap();

    let result = runtime
        .execute(FileSourceProvider::new("tests/requests/simple-post.http", Some(1)).unwrap())
        .await;

    assert!(result.is_ok(), "Failed test:\n{}", output.into_writer().0);

    assert_eq!(server.requests().await.len(), 1);
}
