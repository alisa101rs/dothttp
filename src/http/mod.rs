use std::future::Future;

use crate::{parser, ClientConfig, Result};

pub mod reqwest;

#[derive(Clone, Copy, Debug)]
pub enum Method {
    Get,
    Post,
    Delete,
    Put,
    Patch,
    Options,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub method: Method,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub version: Version,
    pub status_code: u16,
    pub status: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Version {
    Http09,
    Http10,
    Http11,
    Http2,
    Http3,
}

pub trait HttpClient {
    fn create(config: ClientConfig) -> Self
    where
        Self: Sized;

    fn execute(&self, request: &Request) -> impl Future<Output = Result<Response>>;
}

impl From<&parser::Method> for Method {
    fn from(method: &parser::Method) -> Self {
        match method {
            parser::Method::Get(_) => Method::Get,
            parser::Method::Post(_) => Method::Post,
            parser::Method::Delete(_) => Method::Delete,
            parser::Method::Put(_) => Method::Put,
            parser::Method::Patch(_) => Method::Patch,
            parser::Method::Options(_) => Method::Options,
        }
    }
}
