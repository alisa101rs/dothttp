pub mod print;

mod ci;
#[cfg(test)]
mod tests;

use std::fmt;

use color_eyre::eyre::anyhow;

pub use self::{ci::CiOutput, print::FormattedOutput};
use crate::{
    http::{Method, Request, Response},
    script_engine::report::TestsReport,
    Result,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum FormatItem {
    FirstLine,
    Headers,
    Body,
    Tests,
    Name,
    Chars(String),
}

pub fn parse_format(format: &str) -> Result<Vec<FormatItem>> {
    let mut result = Vec::new();
    let mut marker = false;
    let mut buff = String::new();
    for ch in format.chars() {
        if marker {
            marker = false;
            let action = match ch {
                '%' => None,
                'R' => Some(FormatItem::FirstLine),
                'H' => Some(FormatItem::Headers),
                'B' => Some(FormatItem::Body),
                'T' => Some(FormatItem::Tests),
                'N' => Some(FormatItem::Name),
                _ => return Err(anyhow!("Invalid formatting character '{}'", ch)),
            };
            if let Some(a) = action {
                if !buff.is_empty() {
                    result.push(FormatItem::Chars(buff));
                    buff = String::new();
                }
                result.push(a);
            } else {
                buff.push(ch);
            }
        } else if ch == '%' {
            marker = true;
        } else {
            buff.push(ch);
        }
    }
    if !buff.is_empty() {
        result.push(FormatItem::Chars(buff));
    }
    Ok(result)
}

fn prettify_response_body(body: &str) -> String {
    match serde_json::from_str(body) {
        Ok(serde_json::Value::Object(response_body)) => {
            serde_json::to_string_pretty(&response_body).unwrap()
        }
        _ => String::from(body),
    }
}

pub trait Output {
    fn response(&mut self, response: &Response, tests: &TestsReport) -> Result<()>;
    fn request(&mut self, request: &Request, request_name: &str) -> Result<()>;
    fn tests(&mut self, tests: Vec<(String, String, TestsReport)>) -> Result<()>;

    fn exit_code(&mut self) -> std::process::ExitCode {
        std::process::ExitCode::SUCCESS
    }
}

impl Output for Box<dyn Output> {
    fn response(&mut self, response: &Response, tests: &TestsReport) -> Result<()> {
        (**self).response(response, tests)
    }

    fn request(&mut self, request: &Request, request_name: &str) -> Result<()> {
        (**self).request(request, request_name)
    }

    fn tests(&mut self, tests: Vec<(String, String, TestsReport)>) -> Result<()> {
        (**self).tests(tests)
    }

    fn exit_code(&mut self) -> std::process::ExitCode {
        (**self).exit_code()
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let method = match *self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Delete => "DELETE",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Options => "OPTIONS",
        };
        f.write_str(method)
    }
}
