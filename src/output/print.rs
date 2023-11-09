use std::{fmt, io::Write};

use crate::{
    http,
    output::{prettify_response_body, FormatItem, Output},
    script_engine::report::{TestResult, TestsReport},
    Result,
};

pub struct FormattedOutput<'a, W: Write> {
    writer: &'a mut W,
    request_format: Vec<FormatItem>,
    response_format: Vec<FormatItem>,
}

impl<'a, W: Write> FormattedOutput<'a, W> {
    pub fn new(
        writer: &mut W,
        request_format: Vec<FormatItem>,
        response_format: Vec<FormatItem>,
    ) -> FormattedOutput<W> {
        FormattedOutput {
            writer,
            request_format,
            response_format,
        }
    }
}

fn format_headers(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .map(|(key, value)| format!("{}: {}\n", key, value))
        .collect()
}

fn format_body(body: &Option<String>) -> String {
    match body {
        Some(body) => prettify_response_body(body),
        None => String::from(""),
    }
}

fn format_tests(report: &TestsReport) -> String {
    use std::fmt::Write;

    let mut output = String::new();
    if report.is_empty() {
        return output;
    }

    for (test, result) in report.all() {
        write!(&mut output, "Test `{test}`: ").unwrap();
        match result {
            TestResult::Error { error } => writeln!(&mut output, "FAILED with {error}").unwrap(),
            TestResult::Success => writeln!(&mut output, "OK").unwrap(),
        }
    }

    output
}

impl<'a, W: Write> Output for FormattedOutput<'a, W> {
    fn response(&mut self, response: &http::Response, tests: &TestsReport) -> Result<()> {
        if self.response_format.is_empty() {
            return Ok(());
        }

        let http::Response {
            headers,
            version,
            status,
            body,
            ..
        } = response;

        for format_item in &self.response_format {
            let to_write = match format_item {
                FormatItem::FirstLine => format!("{} {}", version, status),
                FormatItem::Headers => format_headers(headers),
                FormatItem::Body => format_body(body),
                FormatItem::Chars(s) => s.clone(),
                FormatItem::Tests => format_tests(tests),
                FormatItem::Name => continue,
            };

            if to_write.is_empty() {
                continue;
            }

            write!(self.writer, "{to_write}")?;
        }
        Ok(())
    }

    fn request(&mut self, request: &http::Request, request_name: &str) -> Result<()> {
        if self.request_format.is_empty() {
            return Ok(());
        }
        let http::Request {
            method,
            target,
            headers,
            body,
            ..
        } = request;

        for format_item in &self.request_format {
            let to_write = match format_item {
                FormatItem::FirstLine => format!("{} {}", method, target),
                FormatItem::Headers => format_headers(headers),
                FormatItem::Body => format_body(body),
                FormatItem::Chars(s) => s.clone(),
                FormatItem::Tests => continue,
                FormatItem::Name => format!("[{request_name}]"),
            };

            if to_write.is_empty() {
                continue;
            }

            write!(self.writer, "{to_write}")?;
        }
        Ok(())
    }
}

impl fmt::Display for http::Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            http::Version::Http09 => write!(f, "HTTP/0.9"),
            http::Version::Http10 => write!(f, "HTTP/1.0"),
            http::Version::Http11 => write!(f, "HTTP/1.1"),
            http::Version::Http2 => write!(f, "HTTP/2"),
            http::Version::Http3 => write!(f, "HTTP/3"),
        }
    }
}
