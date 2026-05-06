mod lexer;
mod parser;

use codespan_reporting::diagnostic::Severity;
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::{self, Config};
use lelwel::Parser;
use lelwel::{Cst, Node, NodeRef};
use lexer::Token;
use parser::*;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

fn to_value(cst: &Cst<'_, Token, Rule>, node_ref: NodeRef) -> Option<Value> {
    match cst.get(node_ref) {
        Node::Rule(rule, _) => match rule {
            Rule::File => cst
                .children(node_ref)
                .find_map(|child_node_ref| to_value(cst, child_node_ref)),
            Rule::Literal => to_value(cst, cst.children(node_ref).next()?),
            Rule::Array => Some(Value::Array(
                cst.children(node_ref)
                    .filter_map(|child_node_ref| to_value(cst, child_node_ref))
                    .collect(),
            )),
            Rule::Object => {
                let mut members = HashMap::new();
                for mut member_node_refs in cst
                    .children(node_ref)
                    .filter(|&child_node_ref| cst.match_rule(child_node_ref, Rule::Member))
                    .map(|child_node_ref| cst.children(child_node_ref))
                {
                    let Some(key) = member_node_refs
                        .find_map(|member_node_ref| cst.match_token(member_node_ref, Token::String))
                        .map(|(key_str, _)| key_str[1..key_str.len() - 1].to_owned())
                    else {
                        continue;
                    };
                    let Some(val) =
                        member_node_refs.find_map(|member_node_ref| to_value(cst, member_node_ref))
                    else {
                        continue;
                    };
                    members.insert(key, val);
                }
                Some(Value::Object(members))
            }
            _ => None,
        },
        Node::Token(token, idx) => match token {
            Token::String => {
                let val = cst.span_text(idx);
                Some(Value::String(val[1..val.len() - 1].to_owned()))
            }
            Token::Number => Some(Value::Number(str::parse(cst.span_text(idx)).ok()?)),
            Token::True => Some(Value::Bool(true)),
            Token::False => Some(Value::Bool(false)),
            Token::Null => Some(Value::Null),
            _ => None,
        },
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        std::process::exit(1);
    }

    let source = std::fs::read_to_string(&args[1])?;
    let mut diags = vec![];
    let cst = Parser::new(&source, &mut diags).parse(&mut diags);
    let val = to_value(&cst, NodeRef::ROOT);
    println!("{cst}");
    println!("{val:#?}");

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = Config::default();
    let file = SimpleFile::new(&args[1], &source);
    for diag in diags.iter() {
        term::emit_to_write_style(&mut writer.lock(), &config, &file, diag).unwrap();
    }
    if diags.iter().any(|d| d.severity == Severity::Error) {
        std::process::exit(1);
    }
    Ok(())
}
