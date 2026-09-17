mod lexer;
mod parser;

use std::io::BufWriter;

use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::Config;
use codespan_reporting::term::termcolor::NoColor;
use lexer::LexState;
use parser::*;

pub use lexer::Token;
pub use parser::LexCall;

fn render(source: &str, diags: &[Diagnostic], cst: &Cst<'_>) -> Vec<String> {
    let mut writer = NoColor::new(BufWriter::new(Vec::new()));
    let config = Config::default();
    let file = SimpleFile::new("<input>", source);
    for diag in diags.iter() {
        term::emit_to_write_style(&mut writer, &config, &file, diag).unwrap();
    }

    vec![
        format!("{cst}"),
        String::from_utf8(writer.into_inner().into_inner().unwrap()).unwrap(),
    ]
}

/// Parses `source` using on-the-fly lexing and returns the CST tree and the
/// emitted diagnostics, mirroring the pattern used by the other examples.
pub fn generate_syntax_tree(source: &str) -> Vec<String> {
    let mut diags = vec![];
    let cst = Parser::new(source, &mut diags).parse(&mut diags);
    render(source, &diags, &cst)
}

/// Parses `source` and returns the CST tree, the diagnostics and the audit
/// log of every `lex` call, including calls that were rolled back by ordered
/// choice. The log is what proves tokens are re-lexed after a backtrack, and
/// it carries the expected set each call was given.
pub fn parse_with_log(source: &str) -> (Vec<String>, Vec<LexCall>) {
    let mut diags = vec![];
    let log = LexLog::default();
    let cst = Parser::new_with_context(source, &mut diags, log.clone(), LexState::default())
        .parse(&mut diags);
    (render(source, &diags, &cst), log.calls.borrow().clone())
}
