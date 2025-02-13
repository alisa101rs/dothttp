#[cfg(test)]
pub mod tests;

use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use from_pest::FromPest;
use miette::{bail, Context, IntoDiagnostic, Result};
use pest::Parser;

mod grammar {
    use pest_derive::Parser;
    use std::fmt;

    #[derive(Parser)]
    #[grammar = "parser/parser.pest"]
    pub struct ScriptParser;

    impl fmt::Display for Rule {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            let ident = match self {
                Rule::EOI => "end of file",
                Rule::INPUT => "any input character",
                Rule::COMMENT => "comment",
                Rule::SP => "whitespace",
                Rule::WHITESPACE => "whiltespace",
                Rule::ALPHA => "latin character",
                Rule::DIGIT => "digit",
                Rule::IDENT => "identifier",
                Rule::EMPTY_LINE => "empty line",
                Rule::INDENT => "indented line",
                Rule::INDENTED_NEWLINE => "indented line",
                Rule::request_separator => "###",
                Rule::request_line => "request target",
                Rule::http_version => "http version",
                Rule::method_name => "http method",
                Rule::method => "http method",
                Rule::request_target => "URL",
                Rule::origin_form => "URL in origin form",
                Rule::absolute_form => "URL in absolute form",
                Rule::hier_part => "authority and path",
                Rule::asterisk_form => "*",
                Rule::scheme => "https|http scheme",
                Rule::authority => "authority",
                Rule::port => "port",
                Rule::host => "hostname",
                Rule::ipv6_input_char => "ipv6 character",
                Rule::ipv6_address => "ipv6 address",
                Rule::ipv4_or_reg_name_char => "URL character",
                Rule::ipv4_or_reg_name => "domain or ipv3",
                Rule::absolute_path => "absolute path",
                Rule::path_separator => "path separator",
                Rule::path_segment_char => "path character",
                Rule::path_segment => "path segment",
                Rule::query_char => "query character",
                Rule::query_with_newline => "query string",
                Rule::query => "query string",
                Rule::fragment_char => "fragment character",
                Rule::fragment_with_newline => "fragment string",
                Rule::fragment => "fragment string",
                Rule::headers => "headers",
                Rule::header_field => "header",
                Rule::header_name => "header name",
                Rule::header_value => "header value",
                Rule::message_body => "request body",
                Rule::messages => "request body",
                Rule::message_line => "request body line",
                Rule::input_file_ref => "reference to input body",
                Rule::file_path => "file path",
                Rule::multipart_form_data => "multipart body",
                Rule::multipart_field => "multipart field",
                Rule::boundary => "multipart boundary",
                Rule::response_handler => "response handler",
                Rule::response_handler_script => "response handler script",
                Rule::response_handler_ref => "response handler reference",
                Rule::handler_script => "request handler",
                Rule::request_handler => "request handler",
                Rule::request_handler_script => "request handler script",
                Rule::request_handler_ref => "request handler reference",
                Rule::pre_handler_script => "request handler script",
                Rule::request_variable_name => "inline variable name",
                Rule::request_variable_value => "inline variable value",
                Rule::request_variable_declaration => "inline variable declaration",
                Rule::request_variable_declarations => "inline variables block",
                Rule::response_reference => "response reference",
                Rule::request => "request",
                Rule::request_script_with_sep => "request with separator",
                Rule::request_script => "request",
                Rule::file => "request file",
            };
            write!(fmt, "{ident}")
        }
    }
}

pub mod ast {
    #![allow(dead_code)]
    use super::{InlineScript, Unprocessed, Value};
    use derive_more::AsRef;
    use from_pest::{FromPest, Void};
    use miette::{miette, LabeledSpan};
    use pest::{iterators::Pairs, Span};
    use pest_ast::FromPest;

    use super::grammar::Rule;

    macro_rules! simple {
        ($name: ident, $rule: path, $attrs: meta) => {
            #[derive(Debug, FromPest)]
            #[pest_ast(rule($rule))]
            pub struct $name<'pest>(
                #[pest_ast(outer($attrs))] pub &'pest str,
                #[pest_ast(outer(with(position)))] pub (usize, usize),
            );

            impl AsRef<str> for $name<'_> {
                fn as_ref(&self) -> &str {
                    self.0
                }
            }

            impl $name<'_> {
                pub fn labeled_span(&self) -> miette::LabeledSpan {
                    miette::LabeledSpan::new(
                        Some(stringify!($name).to_owned()),
                        self.1 .0,
                        self.1 .1,
                    )
                }

                pub fn try_parse<T, E>(
                    &self,
                    parse: impl FnOnce(&str) -> Result<T, E>,
                ) -> miette::Result<T>
                where
                    E: std::fmt::Debug + std::fmt::Display,
                {
                    let msg = match parse(&self.0) {
                        Ok(res) => return Ok(res),
                        Err(msg) => msg,
                    };

                    Err(miette!(
                        help = "",
                        labels = vec![self.labeled_span()],
                        "{msg}"
                    ))
                }

                pub fn to_value(&self) -> miette::Result<Value> {
                    let owned = self.0.to_owned();
                    if !self.0.contains("{{") {
                        return Ok(Value {
                            state: Unprocessed::WithoutInline(owned),
                        });
                    };
                    let mut inline_scripts = vec![];
                    let mut offset = 0;
                    let mut s = self.0;
                    while let Some(start) = s.find("{{") {
                        let Some(end) = s.find("}}") else {
                            return Err(miette!(
                                help = "add missing }}",
                                labels = vec![LabeledSpan::at_offset(
                                    self.1 .0 + offset + start,
                                    "here"
                                )],
                                "unmatched inline script start '{{'"
                            ));
                        };
                        let placeholder = s[start..end + 2].to_owned();
                        let script = s[start + 2..end].to_owned();
                        inline_scripts.push(InlineScript {
                            script,
                            placeholder,
                        });

                        if s.len() == end + 2 {
                            break;
                        }

                        offset += end + 2;
                        s = &s[end + 2..];
                    }

                    Ok(Value {
                        state: Unprocessed::WithInline {
                            value: owned,
                            inline_scripts,
                        },
                    })
                }
            }
        };
    }

    fn get_str(v: Span<'_>) -> &'_ str {
        v.as_str()
    }

    fn get_trimmed(v: Span) -> &str {
        v.as_str().trim()
    }

    fn request_name(v: Span) -> &str {
        v.as_str().strip_prefix("###").unwrap()
    }

    fn position(v: Span) -> (usize, usize) {
        (v.start(), v.end() - v.start())
    }

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::file))]
    pub struct File<'pest> {
        pub scripts: Vec<(Option<RequestSeparator<'pest>>, RequestScript<'pest>)>,
        _separator: Option<RequestSeparator<'pest>>,
        _eoi: Eoi,
    }

    simple!(
        RequestSeparator,
        Rule::request_separator,
        with(request_name)
    );
    simple!(Method, Rule::method, with(get_trimmed));
    simple!(Target, Rule::request_target, with(get_str));
    simple!(HttpVersion, Rule::http_version, with(get_trimmed));

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::request_script))]
    pub struct RequestScript<'pest> {
        #[pest_ast(outer(with(position)))]
        pub request_position: (usize, usize),
        pub variables: Vec<VariableDeclaration<'pest>>,
        pub request_handler: Option<RequestHandler<'pest>>,
        pub method: Method<'pest>,
        pub target: Target<'pest>,
        pub version: Option<HttpVersion<'pest>>,

        pub headers: Vec<Header<'pest>>,
        pub body: Option<RequestBodies<'pest>>,
        pub response_handler: Option<ResponseHandler<'pest>>,
        pub response_reference: Option<ResponseReference<'pest>>,
    }

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::header_field))]
    pub struct Header<'pest> {
        pub name: HeaderName<'pest>,
        pub value: HeaderValue<'pest>,
    }
    simple!(HeaderName, Rule::header_name, with(get_trimmed));
    simple!(HeaderValue, Rule::header_value, with(get_trimmed));

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::request_variable_declaration))]
    pub struct VariableDeclaration<'pest> {
        pub name: VariableName<'pest>,
        pub value: VariableValue<'pest>,
    }
    simple!(VariableName, Rule::request_variable_name, with(get_trimmed));
    simple!(
        VariableValue,
        Rule::request_variable_value,
        with(get_trimmed)
    );

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::request_handler))]
    pub struct RequestHandler<'pest> {
        #[pest_ast(inner())]
        pub script: pest::Span<'pest>,
    }

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::response_handler))]
    pub struct ResponseHandler<'pest> {
        #[pest_ast(inner())]
        pub script: pest::Span<'pest>,
    }

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::response_reference))]
    pub struct ResponseReference<'pest> {
        #[pest_ast(inner())]
        path: Span<'pest>,
    }

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::message_body))]
    pub enum RequestBodies<'pest> {
        Body(RequestBody<'pest>),
        MultipartFormdata(MultipartRequestBody),
    }

    simple!(RequestBody, Rule::messages, with(get_str));

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::multipart_form_data))]
    pub struct MultipartRequestBody {
        ignore: Ignore,
    }

    #[derive(Debug)]
    struct Ignore;

    impl<'pest> FromPest<'pest> for Ignore {
        type Rule = Rule;
        type FatalError = Void;
        fn from_pest(
            pest: &mut Pairs<'pest, Self::Rule>,
        ) -> Result<Self, from_pest::ConversionError<Self::FatalError>> {
            for _ in pest {}
            Ok(Self)
        }
    }

    #[derive(Debug, FromPest)]
    #[pest_ast(rule(Rule::EOI))]
    struct Eoi;
}

pub mod error {
    use super::grammar::Rule;
    use miette::{miette, LabeledSpan};
    use std::fmt::Write;

    pub fn map_it(error: pest::error::Error<Rule>) -> miette::Report {
        let position = error.location;

        let span = match position {
            pest::error::InputLocation::Pos(o) => LabeledSpan::new(Some("here".to_owned()), o, 1),
            pest::error::InputLocation::Span((s, e)) => {
                LabeledSpan::new(Some("here".to_owned()), s, e - s)
            }
        };

        let help = match error.variant {
            pest::error::ErrorVariant::ParsingError {
                positives,
                negatives,
            } => parsing_error_help(positives, negatives),
            pest::error::ErrorVariant::CustomError { message } => message,
        };

        miette! {
            labels = vec![span],
            help = help,
            "Failed to parse request file"
        }
    }

    fn parsing_error_help(positives: Vec<Rule>, negatives: Vec<Rule>) -> String {
        let mut help = String::new();

        let t = [(positives, "expected"), (negatives, "unexpected")];

        for (tokens, msg) in t {
            if tokens.is_empty() {
                continue;
            }

            if tokens.len() == 1 {
                write!(help, "{msg} token: ").unwrap();
            } else {
                write!(help, "{msg} one of: ").unwrap();
            }
            for (token, sep) in interleave(tokens.into_iter(), ", ", "") {
                write!(help, "{token}{sep}").unwrap();
            }
        }

        help
    }

    fn interleave<T, E, I>(mut iter: I, middle: E, last: E) -> impl Iterator<Item = (T, E)>
    where
        I: Iterator<Item = T>,
        E: Copy,
    {
        let mut next = iter.next();
        std::iter::from_fn(move || {
            if next.is_none() {
                return None;
            }
            let to_emit = next.take().unwrap();
            next = iter.next();
            if next.is_none() {
                Some((to_emit, last))
            } else {
                Some((to_emit, middle))
            }
        })
    }
}
pub fn parse(source: &str) -> Result<File> {
    let mut parsed_tree =
        grammar::ScriptParser::parse(grammar::Rule::file, source).map_err(error::map_it)?;
    let ast: ast::File = ast::File::from_pest(&mut parsed_tree)
        .into_diagnostic()
        .wrap_err("Failed to convert input file to ast representation even though request file was parsed successfuly. It is probably a bug in dothttp.")?;

    let file = File::try_from(ast).context("while converting ast representation to inner types")?;
    Ok(file)
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.state {
            Unprocessed::WithInline { value, .. } => f.write_str(value),
            Unprocessed::WithoutInline(value) => f.write_str(value),
        }
    }
}

#[derive(Debug)]
pub struct Value {
    pub state: Unprocessed,
}

#[derive(Debug)]
pub enum Unprocessed {
    WithInline {
        value: String,
        inline_scripts: Vec<InlineScript>,
    },
    WithoutInline(String),
}

impl Unprocessed {
    pub fn value(&self) -> &str {
        match self {
            Unprocessed::WithInline { value, .. } => value,
            Unprocessed::WithoutInline(value) => value,
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct InlineScript {
    pub script: String,
    pub placeholder: String,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct File {
    pub request_scripts: Vec<RequestScript>,
}

impl File {
    pub fn request_scripts(
        &self,
        request: Option<usize>,
    ) -> impl Iterator<Item = (usize, &RequestScript)> {
        let mut scripts = self
            .request_scripts
            .iter()
            .enumerate()
            .filter(move |&(index, _)| (request.is_none() || Some(index + 1) == request))
            .peekable();

        match scripts.peek() {
            Some(_) => scripts,
            None => panic!("Couldn't find any scripts in our file at the given line number"),
        }
    }
}

impl<'a> TryFrom<ast::File<'a>> for File {
    type Error = miette::Report;

    fn try_from(value: ast::File<'a>) -> Result<Self, Self::Error> {
        let mut request_scripts = vec![];
        for (name, script) in value.scripts {
            let script =
                RequestScript::from_ast(name.map(|it| it.0), script).wrap_err("parsing request")?;
            request_scripts.push(script);
        }

        Ok(Self { request_scripts })
    }
}

#[derive(Debug)]
pub struct RequestScript {
    pub name: Option<String>,
    pub request: Request,
    pub request_variables: Vec<(String, Value)>,
    pub pre_request_handler: Option<Handler>,
    pub handler: Option<Handler>,
}

impl RequestScript {
    fn from_ast(name: Option<&str>, value: ast::RequestScript<'_>) -> Result<Self> {
        let name = name.map(ToOwned::to_owned);
        let method = Method::from_str(value.method.as_ref())
            .into_diagnostic()
            .wrap_err("invalid request method")?;
        let target = value
            .target
            .to_value()
            .wrap_err("while parsing request 'target'")?;
        let headers = value
            .headers
            .into_iter()
            .map(|h| {
                Ok(Header {
                    field_name: h.name.as_ref().to_owned(),
                    field_value: h.value.to_value().wrap_err_with(|| {
                        format!("while parsing request header '{}'", h.name.as_ref())
                    })?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let body = value.body.map(Self::map_body).transpose()?;
        let pre_request_handler = value.request_handler.map(|it| Handler {
            script: it.script.as_str().to_owned(),
        });
        let handler = value.response_handler.map(|it| Handler {
            script: it.script.as_str().to_owned(),
        });
        let request_variables = value
            .variables
            .into_iter()
            .map(|v| {
                Ok((
                    v.name.as_ref().to_owned(),
                    v.value.to_value().wrap_err_with(|| {
                        format!("while parsing request variable '{}'", v.name.as_ref())
                    })?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            name,
            request: Request {
                method,
                target,
                headers,
                body,
            },
            request_variables,
            pre_request_handler,
            handler,
        })
    }

    fn map_body(body: ast::RequestBodies) -> Result<Value> {
        match body {
            ast::RequestBodies::Body(b) => {
                Ok(b.to_value().wrap_err("while parsing request body")?)
            }
            ast::RequestBodies::MultipartFormdata(_) => {
                bail!("Multipart Formadata is not yet supported")
            }
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub target: Value,
    pub headers: Vec<Header>,
    pub body: Option<Value>,
}

#[derive(PartialEq, Debug, Clone, strum::EnumString)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Delete,
    Put,
    Patch,
    Options,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Header {
    pub field_name: String,
    pub field_value: Value,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Handler {
    pub script: String,
}
