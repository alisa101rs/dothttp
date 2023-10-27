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

impl<'a, W: Write> Output for FormattedOutput<'a, W> {
    fn section(&mut self, name: &str) -> Result<()> {
        writeln!(self.writer, "[{name}]")?;
        Ok(())
    }

    fn response(&mut self, response: &http::Response) -> Result<()> {
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
            };

            if to_write.is_empty() {
                continue;
            }

            write!(self.writer, "{to_write}")?;
        }
        Ok(())
    }

    fn request(&mut self, request: &http::Request) -> Result<()> {
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
            };

            if to_write.is_empty() {
                continue;
            }

            write!(self.writer, "{to_write}")?;
        }
        Ok(())
    }

    fn tests(&mut self, report: &TestsReport) -> Result<()> {
        if report.is_empty() {
            return Ok(());
        }

        for (test, result) in report.all() {
            write!(self.writer, "Test `{test}`: ")?;
            match result {
                TestResult::Error { error } => writeln!(self.writer, "FAILED with {error}")?,
                TestResult::Success => writeln!(self.writer, "OK")?,
            }
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
