use dothttp::{source::FileSourceProvider, ClientConfig, Runtime, StaticEnvironmentProvider};
use serde_json::json;

use crate::common::{formatter, MockHttpBin};

mod common;

#[tokio::test]
async fn multi_post() {
    let mut server = MockHttpBin::start().await;
    let mut output = formatter();
    let mut environment = StaticEnvironmentProvider::new(
        json!({ "host": format!("127.0.0.1:{}", server.addr.port()) }),
    );
    let mut runtime = Runtime::new(&mut environment, &mut output, ClientConfig::default()).unwrap();
    let result = runtime
        .execute(FileSourceProvider::new("tests/requests/multi.http", None).unwrap())
        .await;

    assert!(result.is_ok(), "Failed test:\n{}", output.into_writer().0);

    assert_eq!(server.requests().await.len(), 3);

    assert_eq!(environment.snapshot().get("output"), Some(&"true".into()),);
}
