use super::*;
use crate::parser;

#[test]
fn script_parser_parse() {
    let test = "\
# Comment 1
# Comment 2
# Comment 3
@variable=value
GET http://{{host}}.com HTTP/1.1
Accept: *#/*
# Commented Header
Content-Type: {{ content_type }}

{
    \"fieldA\": \"value1\"
}

> {%
    console.log('Success!');
%}

###

# Request Comment 2
#
GET http://example.com/{{url_param}}
Accept: */*

###

";
    let files = ScriptParser::parse(Rule::file, test);
    if let Err(e) = &files {
        println!("{}", e);
    }
    assert!(files.is_ok());

    let file = files.unwrap().next();
    let mut request_scripts = file.unwrap().into_inner();

    let request_script = request_scripts.next().unwrap();

    assert_eq!(
        request_script.as_str(),
        "\
@variable=value
GET http://{{host}}.com HTTP/1.1
Accept: *#/*
# Commented Header
Content-Type: {{ content_type }}

{
    \"fieldA\": \"value1\"
}

> {%
    console.log('Success!');
%}"
    );

    let mut request_script_parts = request_script.into_inner();

    let request_variable_declarations = request_script_parts.next().unwrap();
    assert_eq!(request_variable_declarations.as_str(), "@variable=value\n");

    let method = request_script_parts.next().unwrap();

    assert_eq!(method.as_str(), "GET");

    let request_target = request_script_parts.next().unwrap();

    assert_eq!(request_target.as_str(), "http://{{host}}.com");

    let header_field = request_script_parts.next().unwrap();
    assert_eq!(header_field.as_str(), "Accept: *#/*");
    let other_header_field = request_script_parts.next().unwrap();
    assert_eq!(
        other_header_field.as_str(),
        "Content-Type: {{ content_type }}"
    );

    let request_script = request_scripts.next().unwrap();

    assert_eq!(
        request_script.as_str(),
        "###

# Request Comment 2
#
GET http://example.com/{{url_param}}
Accept: */*

"
    );

    let mut request_script_parts = request_script.into_inner();
    let _name = request_script_parts.next().unwrap();
    let _method = request_script_parts.next().unwrap();
    let _request_target = request_script_parts.next().unwrap();
    let _header_field = request_script_parts.next().unwrap();
    let body = request_script_parts.next();
    assert_eq!(body, None);
}

#[test]
fn min_file() {
    let test = "POST http://example.com HTTP/1.1\n";

    let file = ScriptParser::parse(Rule::file, test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());
}

#[test]
fn weird_file() {
    let test = "\
POST http://example.com HTTP/1.1

{}

> {% console.log('no'); %}";

    let file = parser::parse(PathBuf::default(), test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());
}

#[test]
fn empty_body_with_handler() {
    let test = "\
POST http://example.com HTTP/1.1
Accept: */*

> {%
    console.log('cool');
%}
###
";

    let file = parser::parse(PathBuf::default(), test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());
}

#[test]
fn new_line_in_request_body_file() {
    let test = "\
POST http://example.com HTTP/1.1
Accept: */*

{
    \"test\": \"a\",
    \"what\": [

    ]
}


> {%
    console.log('cool');
%}

###
";

    let file = parser::parse(PathBuf::default(), test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());
}

#[test]
fn request_script() {
    let test = "\
GET http://{{host}}.com HTTP/1.1
Accept: *#/*
# Commented Header
Content-Type: {{ content_type }}

{
    \"fieldA\": {{
    content_type
    }}
}

> {%
    console.log('Success!');
%}";
    let request_script = ScriptParser::parse(Rule::request_script, test);
    if let Err(e) = &request_script {
        println!("{}", e);
    }

    assert!(request_script.is_ok());
}

#[test]
fn request() {
    let test = "\
GET http://{{host}}.com HTTP/1.1
Accept: */*
Content-Type: {{ content_type }}
Content-Type2: {{ content_type2 }}
";
    let request = ScriptParser::parse(Rule::request, test);
    if let Err(e) = &request {
        println!("{:?}", e);
    }

    assert!(request.is_ok());
}

#[test]
fn response_handler() {
    let test = "\
> {%
 console.log('hi');
%}
";
    let handler = ScriptParser::parse(Rule::response_handler, test);
    if let Err(e) = &handler {
        println!("{:?}", e);
    }

    assert!(handler.is_ok());
}

#[test]
fn pre_request_handler() {
    let test = "\
< {%
 console.log('hi');
%}
";
    let handler = ScriptParser::parse(Rule::pre_request_handler, test);
    if let Err(e) = &handler {
        println!("{:?}", e);
    }

    assert!(handler.is_ok());
}

#[test]
fn response_handler_with_comment() {
    let test = "\
POST http://httpbin.org/post

{}

# should be fine > {% %}
> {%
  console.log('hi');
%}
";
    let file = ScriptParser::parse(Rule::file, test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());
}

#[test]
fn mixing_body_and_headers() {
    let test = "\
GET http://example.com HTTP/1.1
header: some-value";

    let file = parser::parse(PathBuf::default(), test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());

    let request = &file.unwrap().request_scripts[0].request;

    assert!(&request.headers[0].field_name == "header");
    assert!(&request.body.is_none());
}

#[test]
fn alot_of_whitespaces() {
    let test = "      POST       http://example.com     HTTP/1.1     \n";

    let file = ScriptParser::parse(Rule::file, test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());
}

#[test]
fn multiline_request_line() {
    let test = r#"GET https://httpbin.org/get
         ?request=2
         HTTP/1.0
     "#;

    let file = ScriptParser::parse(Rule::request_line, test);
    if let Err(e) = &file {
        println!("{:?}", e);
    }

    assert!(file.is_ok());

    let mut pairs = file.unwrap();

    let method = pairs.next().unwrap().as_str();
    assert_eq!(method, "GET");
    let uri = pairs
        .next()
        .unwrap()
        .as_str()
        .replace(|c: char| c.is_whitespace(), "");
    assert_eq!(&uri, "https://httpbin.org/get?request=2");
    assert!(pairs.next().is_none())
}

#[test]
fn request_variable_declarations() {
    let test = "\
@a=y
@b = ywae
@c = \"w\"
@d = {{x}} + y
";
    let handler = ScriptParser::parse(Rule::request_variable_declarations, test);
    if let Err(e) = &handler {
        println!("{:?}", e);
    }

    assert!(handler.is_ok());
    let pairs =
        request_variable_declaration_from_pair("/".into(), handler.unwrap().next().unwrap());

    assert_eq!(pairs.len(), 4);
}

#[test]
fn request_with_variables_and_pre_request_handler() {
    let test = "\
@var = {{variable}} + 1
# comment
@w = y

@variable2 = {{var}} + 1
#comment
< {%
    client.log(\"hello\");
%}
# Comment

GET http://{{host}}/get?value=10
my-header: {{variable2}}

> {%
    client.log(\"world\");
%}

";
    let files = ScriptParser::parse(Rule::file, test);
    if let Err(e) = &files {
        println!("{}", e);
    }
    assert!(files.is_ok());
    let mut files = files.unwrap();
    let file = files.next().unwrap();
    let mut request_scripts = file.into_inner();

    let mut request_script = request_scripts.next().unwrap().into_inner();

    let variable_declarations = request_script.next().unwrap();
    let variable_declarations =
        request_variable_declaration_from_pair("/".into(), variable_declarations);
    assert_eq!(variable_declarations.len(), 3);
    let pre_request_handler = request_script.next().unwrap();
    assert_eq!(
        pre_request_handler.into_inner().next().unwrap().as_str(),
        "client.log(\"hello\");"
    );
    let method = request_script.next().unwrap();
    assert_eq!(method.as_str(), "GET");
    let target = request_script.next().unwrap();
    assert_eq!(target.as_str(), "http://{{host}}/get?value=10");

    let my_header = request_script.next().unwrap();
    assert_eq!(my_header.as_str(), "my-header: {{variable2}}");
    let response_handler = request_script.next().unwrap();
    assert_eq!(
        response_handler.into_inner().next().unwrap().as_str(),
        "client.log(\"world\");"
    );
    assert_eq!(request_script.next(), None);
}
