use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    str::FromStr,
};

use anyhow::Context;
use http::Uri;
use reqwest::{header::HeaderMap, Client, RequestBuilder, Url};

use crate::{
    http::{ClientConfig, HttpClient, Method, Request, Response, Version},
    Result,
};

pub struct ReqwestHttpClient {
    client: Client,
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::create(ClientConfig::default())
    }
}

impl HttpClient for ReqwestHttpClient {
    fn create(config: ClientConfig) -> ReqwestHttpClient
    where
        Self: Sized,
    {
        let client = Client::builder()
            .danger_accept_invalid_certs(config.ssl_check)
            .build()
            .unwrap();

        ReqwestHttpClient { client }
    }

    async fn execute(&self, request: &Request) -> Result<Response> {
        let Request {
            method,
            target,
            headers,
            body,
        } = request;
        let mut request_builder = self
            .client
            .request(method.into(), get_request_target(target)?);
        request_builder = set_headers(headers, request_builder);
        if let Some(body) = body {
            request_builder = set_body(body, request_builder);
        }
        let response = request_builder.send().await?;

        map_reqwest_response(response).await
    }
}

fn get_request_target(target: &str) -> Result<Url> {
    let target = if target.starts_with("http://") || target.starts_with("https://") {
        Cow::Borrowed(target)
    } else {
        Cow::Owned(format!("http://{target}"))
    };

    let parsed = Uri::from_str(target.as_ref()).context("Invalid URI")?;

    let schema = parsed.scheme().map(|it| it.as_str()).unwrap_or("http");
    let authority = parsed
        .authority()
        .map(|it| it.as_str())
        .unwrap_or("0.0.0.0");
    let path = parsed.path_and_query().map(|it| it.as_str()).unwrap_or("/");

    let formatted = format!("{schema}://{authority}{path}");

    Ok(Url::from_str(&formatted).expect("to be correct"))
}

fn set_headers(
    headers: &[(String, String)],
    mut request_builder: RequestBuilder,
) -> RequestBuilder {
    for (key, value) in headers {
        request_builder = request_builder.header(key, value);
    }
    request_builder
}

impl From<&Method> for reqwest::Method {
    fn from(method: &Method) -> Self {
        match method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Delete => reqwest::Method::DELETE,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Options => reqwest::Method::OPTIONS,
        }
    }
}

struct Headers(Vec<(String, String)>);

async fn map_reqwest_response(response: reqwest::Response) -> Result<Response> {
    let Headers(headers) = response.headers().try_into()?;
    Ok(Response {
        version: response.version().into(),
        status_code: response.status().as_u16(),
        status: response.status().to_string(),
        headers,
        body: match response.text().await? {
            body if !body.is_empty() => Some(body),
            _ => None,
        },
    })
}

impl TryFrom<&HeaderMap> for Headers {
    type Error = anyhow::Error;

    fn try_from(value: &HeaderMap) -> Result<Self> {
        let mut headers = vec![];
        for (header_name, header_value) in value.iter() {
            headers.push((header_name.to_string(), header_value.to_str()?.to_string()))
        }
        Ok(Headers(headers))
    }
}

impl From<reqwest::Version> for Version {
    fn from(value: reqwest::Version) -> Self {
        match value {
            reqwest::Version::HTTP_09 => Version::Http09,
            reqwest::Version::HTTP_10 => Version::Http10,
            reqwest::Version::HTTP_11 => Version::Http11,
            reqwest::Version::HTTP_2 => Version::Http2,
            reqwest::Version::HTTP_3 => Version::Http3,
            _ => unreachable!(),
        }
    }
}

fn set_body(body: &str, mut request_builder: RequestBuilder) -> RequestBuilder {
    let body = body.trim();
    request_builder = request_builder.body::<reqwest::Body>(body.to_string().into());
    request_builder
}
