use std::borrow::BorrowMut;

use dothttp::{
    output::{parse_format, print::FormattedOutput},
    ClientConfig, Runtime,
};
use httpmock::{Method::POST, MockServer};

use crate::common::{create_file, DebugWriter};

mod common;

#[test]
fn multi_post() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(POST).path("/multi_post_first");
        then.status(200)
            .header("date", "")
            .body(r#"{"value": true}"#);
    });

    server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/multi_get_second");
        then.status(200)
            .header("date", "")
            .body(r#"{"value": false}"#);
    });

    server.mock(|when, then| {
        when.method(httpmock::Method::GET).path("/multi_get_third");
        then.status(204).header("date", "");
    });

    let env = "dev";

    let snapshot_file = create_file("{}");
    let env_file = create_file("{}");
    let script_file = create_file(&format!(
        "\
POST http://localhost:{port}/multi_post_first

{{
    \"test\": \"body\"
}}

###

GET http://localhost:{port}/multi_get_second
###
GET http://localhost:{port}/multi_get_third\
        ",
        port = server.port(),
    ));
    let writer = &mut DebugWriter(String::new());
    let request_format = "%N\n%R\n";
    let response_format = "%R\n%H\n%B\n";
    let mut outputter = FormattedOutput::new(
        writer,
        parse_format(request_format).unwrap(),
        parse_format(response_format).unwrap(),
    );

    let mut runtime = Runtime::new(
        env,
        &snapshot_file,
        &env_file,
        outputter.borrow_mut(),
        ClientConfig::default(),
    )
    .unwrap();

    runtime
        .execute(Some(script_file.to_path_buf()), None)
        .unwrap();

    let DebugWriter(buf) = writer;

    debug_assert_eq!(
        *buf,
        format!(
            "\
[{filename} / #1]
POST http://localhost:{port}/multi_post_first
HTTP/1.1 200 OK
date: \n\
content-length: 15\
\n\n\
{{
  \"value\": true
}}
[{filename} / #2]
GET http://localhost:{port}/multi_get_second
HTTP/1.1 200 OK
date: \n\
content-length: 16\
\n\n\
{{
  \"value\": false
}}
[{filename} / #3]
GET http://localhost:{port}/multi_get_third
HTTP/1.1 204 No Content
date: \n\
\n\n",
            port = server.port(),
            filename = script_file.file_name().unwrap().to_str().unwrap()
        )
    );
}
