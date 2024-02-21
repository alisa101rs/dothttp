use std::{
    collections::HashMap, future::IntoFuture, io, io::Write, net::SocketAddr, str::from_utf8,
};

use axum::{
    body::Bytes,
    extract::Query,
    http::request::Parts,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use dothttp::output::{parse_format, print::FormattedOutput};
use http::header::CONTENT_TYPE;
use serde_json::json;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub fn formatter() -> FormattedOutput<DebugWriter> {
    let writer = DebugWriter(String::new());
    FormattedOutput::new(
        writer,
        parse_format("%R\n").unwrap(),
        parse_format("%R\n%H\n%B\n%T\n").unwrap(),
    )
}

pub struct DebugWriter(pub String);

impl Write for DebugWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let DebugWriter(inner) = self;
        let buf = from_utf8(buf).unwrap();
        inner.push_str(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct MockHttpBin {
    pub addr: SocketAddr,
    handle: tokio::task::JoinHandle<Result<(), io::Error>>,
    requests: Receiver<(Parts, Bytes)>,
}

impl MockHttpBin {
    #[must_use]
    pub async fn start() -> Self {
        let (tx, requests) = channel(64);
        let router = Router::new()
            .route("/get", get(mock_get))
            .route("/post", post(mock_post))
            .layer(Extension(tx));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();

        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(axum::serve(listener, router).into_future());

        MockHttpBin {
            handle,
            requests,
            addr,
        }
    }

    pub async fn requests(&mut self) -> Vec<(Parts, Bytes)> {
        let mut output = vec![];
        self.requests.recv_many(&mut output, 64).await;

        output
    }
}

impl Drop for MockHttpBin {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn mock_get(
    Extension(channel): Extension<Sender<(Parts, Bytes)>>,
    parts: Parts,
    Query(args): Query<HashMap<String, String>>,
    body: Bytes,
) -> impl IntoResponse {
    channel.send((parts.clone(), body)).await.unwrap();

    let headers = collect_headers(&parts);
    let url = parts.uri.to_string();

    axum::Json(json!({
        "args": args,
        "headers": headers ,
        "url": url
    }))
}

async fn mock_post(
    Extension(channel): Extension<Sender<(Parts, Bytes)>>,
    Query(args): Query<HashMap<String, String>>,
    parts: Parts,
    body: Bytes,
) -> impl IntoResponse {
    channel.send((parts.clone(), body.clone())).await.unwrap();

    let headers = collect_headers(&parts);
    let data = String::from_utf8_lossy(body.as_ref());
    let url = parts.uri.to_string();
    let json = collect_json(&parts, &body);

    axum::Json(json!({
        "args": args,
        "url": url,
        "headers": headers,
        "data" : data.as_ref(),
        "json": json
    }))
}

fn collect_headers(parts: &Parts) -> HashMap<String, String> {
    parts
        .headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).to_string(),
            )
        })
        .collect()
}

fn collect_json(parts: &Parts, body: &Bytes) -> serde_json::Value {
    let Some(content_type) = parts.headers.get(CONTENT_TYPE) else {
        return json!({});
    };
    if !content_type.as_bytes().starts_with(b"application/json") {
        return json!({});
    }

    serde_json::from_slice(body.as_ref()).unwrap_or_else(|_| json!({}))
}
