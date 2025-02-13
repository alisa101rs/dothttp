use super::*;
use crate::parser;
use insta::assert_debug_snapshot;

fn ast(src: &str) -> miette::Result<ast::File> {
    let mut parsed_tree =
        grammar::ScriptParser::parse(grammar::Rule::file, src).map_err(super::error::map_it)?;
    let ast: ast::File = ast::File::from_pest(&mut parsed_tree)
        .into_diagnostic()
        .wrap_err("Failed to convert input file to ast representation even though request file was parsed successfuly. It is probably a bug in dothttp.")?;
    Ok(ast)
}

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
    let ast = ast(&test).unwrap();
    assert_debug_snapshot!(ast);
    let parsed = parser::parse(&test).unwrap();
    assert_debug_snapshot!(parsed);
}

#[test]
fn min_file() {
    let test = "POST http://example.com HTTP/1.1\n";

    let ast = ast(&test).unwrap();
    assert_debug_snapshot!(ast);
    let parsed = parser::parse(&test).unwrap();
    assert_debug_snapshot!(parsed);
}

#[test]
fn weird_file() {
    let test = "\
POST http://example.com HTTP/1.1

{}

> {% console.log('no'); %}";

    let ast = ast(&test).unwrap();
    assert_debug_snapshot!(ast);
    let parsed = parser::parse(&test).unwrap();
    assert_debug_snapshot!(parsed);
}
