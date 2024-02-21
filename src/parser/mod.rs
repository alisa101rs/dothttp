#[cfg(test)]
pub mod tests;

use std::{
    error, fmt,
    fmt::{Display, Formatter},
    path::PathBuf,
};

use pest::{error::LineColLocation, iterators::Pair, Parser, Span};
use pest_derive::Parser;

use crate::Result;

macro_rules! find_rule {
    ($pairs: expr, $rule: pat) => {
        $pairs.find_map(|pair| match pair.as_rule() {
            $rule => Some(pair),
            _ => None,
        })
    };
}

#[derive(Parser)]
#[grammar = "parser/parser.pest"]
struct ScriptParser;

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub selection: Selection,
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.message.as_str())
    }
}

fn invalid_pair(expected: Rule, got: Rule) -> ! {
    panic!("Wrong pair. Expected: {:?}, Got: {:?}", expected, got)
}

trait FromPair {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self;
}

trait ToSelection {
    fn to_selection(self, filename: PathBuf) -> Selection;
}

impl ToSelection for LineColLocation {
    fn to_selection(self, filename: PathBuf) -> Selection {
        match self {
            LineColLocation::Pos((line, col)) => Selection {
                filename,
                start: Position { line, col },
                end: Position { line, col },
            },
            LineColLocation::Span((start_line, start_col), (end_line, end_col)) => Selection {
                filename,
                start: Position {
                    line: start_line,
                    col: start_col,
                },
                end: Position {
                    line: end_line,
                    col: end_col,
                },
            },
        }
    }
}

impl FromPair for Handler {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        let selection = pair.as_span().to_selection(filename);

        let script = match pair.as_rule() {
            Rule::response_handler | Rule::pre_request_handler => pair
                .into_inner()
                .find_map(|pair| match pair.as_rule() {
                    Rule::handler_script_string => Some(pair.as_str()),
                    _ => None,
                })
                .unwrap()
                .to_string(),
            _ => invalid_pair(Rule::response_handler, pair.as_rule()),
        };

        Handler { selection, script }
    }
}

impl FromPair for Method {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        let selection = pair.as_span().to_selection(filename);
        match pair.as_rule() {
            Rule::method => match pair.as_str() {
                "GET" => Method::Get(selection),
                "POST" => Method::Post(selection),
                "DELETE" => Method::Delete(selection),
                "PUT" => Method::Put(selection),
                "PATCH" => Method::Patch(selection),
                "OPTIONS" => Method::Options(selection),
                _ => panic!("Unsupported method: {}", pair.as_str()),
            },
            _ => invalid_pair(Rule::method, pair.as_rule()),
        }
    }
}

impl FromPair for Value {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        match (pair.as_rule(), pair.as_str()) {
            (
                Rule::request_target
                | Rule::field_value
                | Rule::request_body
                | Rule::request_variable_value,
                string,
            ) => {
                let selection = pair.as_span().to_selection(filename.clone());
                let inline_scripts = pair
                    .into_inner()
                    .filter(|pair| pair.as_rule() == Rule::inline_script)
                    .map(|pair| InlineScript::from_pair(filename.clone(), pair))
                    .collect::<Vec<InlineScript>>();

                if !inline_scripts.is_empty() {
                    Value {
                        state: Unprocessed::WithInline {
                            value: string.to_string(),
                            inline_scripts,
                            selection,
                        },
                    }
                } else {
                    Value {
                        state: Unprocessed::WithoutInline(string.to_string(), selection),
                    }
                }
            }
            _ => invalid_pair(Rule::request_target, pair.as_rule()),
        }
    }
}

impl FromPair for InlineScript {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::inline_script => InlineScript {
                selection: pair.as_span().to_selection(filename),
                placeholder: pair.as_str().to_string(),
                script: pair
                    .into_inner()
                    .map(|pair| pair.as_str())
                    .last()
                    .unwrap()
                    .to_string(),
            },
            _ => invalid_pair(Rule::inline_script, pair.as_rule()),
        }
    }
}

impl FromPair for Header {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::header_field => {
                let selection = pair.as_span().to_selection(filename.clone());
                let mut pairs = pair.into_inner();
                let field_name = find_rule!(pairs, Rule::field_name)
                    .unwrap()
                    .as_str()
                    .to_owned();
                let field_value = find_rule!(pairs, Rule::field_value).unwrap();
                let field_value = Value::from_pair(filename.clone(), field_value);

                Header {
                    selection,
                    field_name,
                    field_value,
                }
            }
            _ => invalid_pair(Rule::header_field, pair.as_rule()),
        }
    }
}

impl FromPair for Request {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::request_script => {
                let selection = pair.as_span().to_selection(filename.clone());
                let mut pairs = pair.into_inner();
                Request {
                    selection,
                    method: pairs
                        .clone() // clone in order to be able to iterate over it again, if no method is found
                        .find_map(|pair| match pair.as_rule() {
                            Rule::method => Some(Method::from_pair(filename.clone(), pair)),
                            _ => None,
                        })
                        .unwrap_or(Method::Get(Selection::none())),
                    target: pairs
                        .find_map(|pair| match pair.as_rule() {
                            Rule::request_target => Some(Value::from_pair(filename.clone(), pair)),
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("Couldn't find target in request script")),
                    headers: pairs
                        .clone()
                        .filter_map(|pair| match pair.as_rule() {
                            Rule::header_field => Some(Header::from_pair(filename.clone(), pair)),
                            _ => None,
                        })
                        .collect::<Vec<Header>>(),
                    body: {
                        let pair = pairs.find_map(|pair| match pair.as_rule() {
                            Rule::request_body => Some(pair),
                            _ => None,
                        });
                        pair.map(|pair| Value::from_pair(filename, pair))
                    },
                }
            }
            _ => invalid_pair(Rule::request_script, pair.as_rule()),
        }
    }
}

impl FromPair for RequestScript {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::request_script => RequestScript {
                name: {
                    let mut pairs = pair.clone().into_inner();
                    let pair = pairs.find_map(|pair| match pair.as_rule() {
                        Rule::request_separator_with_name => Some(pair),
                        _ => None,
                    });

                    pair.map(|pair| pair.as_str().strip_prefix("###").unwrap().trim().to_owned())
                        .and_then(|it| if it.is_empty() { None } else { Some(it) })
                },
                selection: pair.as_span().to_selection(filename.clone()),
                request_variables: {
                    let declarations = find_rule!(
                        pair.clone().into_inner(),
                        Rule::request_variable_declarations
                    );
                    declarations
                        .map(|it| request_variable_declaration_from_pair(filename.clone(), it))
                        .unwrap_or_else(Vec::new)
                },
                pre_request_handler: {
                    let mut pairs = pair.clone().into_inner();
                    let pair = pairs.find_map(|pair| match pair.as_rule() {
                        Rule::pre_request_handler => Some(pair),
                        _ => None,
                    });
                    pair.map(|pair| Handler::from_pair(filename.clone(), pair))
                },
                handler: {
                    let mut pairs = pair.clone().into_inner();
                    let pair = pairs.find_map(|pair| match pair.as_rule() {
                        Rule::response_handler => Some(pair),
                        _ => None,
                    });
                    pair.map(|pair| Handler::from_pair(filename.clone(), pair))
                },
                request: Request::from_pair(filename, pair),
            },
            _ => invalid_pair(Rule::request_script, pair.as_rule()),
        }
    }
}

fn request_variable_declaration_from_pair(
    filename: PathBuf,
    pair: Pair<'_, Rule>,
) -> Vec<(String, Value)> {
    let mut output = vec![];
    let declarations = pair.into_inner();

    for declaration in declarations {
        let mut declaration = declaration.into_inner();
        let name = declaration.next().expect("request_variable_name");
        let name = name.as_str().to_owned();
        let value = declaration.next().expect("request_variable_value");
        let value = Value::from_pair(filename.clone(), value);
        output.push((name, value));
    }

    output
}

impl FromPair for File {
    fn from_pair(filename: PathBuf, pair: Pair<'_, Rule>) -> Self {
        match pair.as_rule() {
            Rule::file => File {
                request_scripts: pair
                    .into_inner()
                    .filter(|pair| pair.as_rule() == Rule::request_script)
                    .map(|pair| RequestScript::from_pair(filename.clone(), pair))
                    .collect::<Vec<RequestScript>>(),
            },
            _ => invalid_pair(Rule::file, pair.as_rule()),
        }
    }
}

impl ToSelection for Span<'_> {
    fn to_selection(self, filename: PathBuf) -> Selection {
        let (start_line, start_col) = self.start_pos().line_col();
        let (end_line, end_col) = self.end_pos().line_col();
        Selection {
            filename,
            start: Position {
                line: start_line,
                col: start_col,
            },
            end: Position {
                line: end_line,
                col: end_col,
            },
        }
    }
}

pub fn parse(filename: PathBuf, source: &str) -> Result<File> {
    Ok(ScriptParser::parse(Rule::file, source)
        .map_err(|error| Error {
            message: error.to_string(),
            selection: error.line_col.to_selection(filename.clone()),
        })?
        .map(|pair| File::from_pair(filename.clone(), pair))
        .last()
        .unwrap())
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.state {
            Unprocessed::WithInline { value, .. } => f.write_str(value),
            Unprocessed::WithoutInline(value, _) => f.write_str(value),
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
        selection: Selection,
    },
    WithoutInline(String, Selection),
}

#[derive(Debug)]
pub struct InlineScript {
    pub script: String,
    pub placeholder: String,
    pub selection: Selection,
}

#[derive(Debug)]
pub struct File {
    pub request_scripts: Vec<RequestScript>,
}

#[derive(Debug)]
pub struct RequestScript {
    pub name: Option<String>,
    pub request: Request,
    pub request_variables: Vec<(String, Value)>,
    pub pre_request_handler: Option<Handler>,
    pub handler: Option<Handler>,
    pub selection: Selection,
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub target: Value,
    pub headers: Vec<Header>,
    pub body: Option<Value>,
    pub selection: Selection,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Method {
    Get(Selection),
    Post(Selection),
    Delete(Selection),
    Put(Selection),
    Patch(Selection),
    Options(Selection),
}

#[derive(Debug)]
pub struct Header {
    pub field_name: String,
    pub field_value: Value,
    pub selection: Selection,
}

#[derive(Debug, Clone)]
pub struct Handler {
    pub script: String,
    pub selection: Selection,
}

impl Selection {
    pub fn none() -> Selection {
        Selection {
            filename: PathBuf::default(),
            start: Position { line: 0, col: 0 },
            end: Position { line: 0, col: 0 },
        }
    }
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

#[derive(Debug, PartialEq, Clone)]
pub struct Selection {
    pub filename: PathBuf,
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}
