use crate::lexer::{Token, tokenize};
use codespan_reporting::diagnostic::Label;

pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

trait ParserExt<'a>: Sized {
    fn lookahead(&self) -> Vec<Token>;
}

impl<'a, B: CstBuilder<Token=Token, Rule=Rule>> ParserExt<'a> for Parser<'a, B, ()> {
    fn lookahead(&self) -> Vec<Token> {
        self.tokens[self.pos..]
            .iter()
            .filter(|tok| !tok.is_skip() && **tok != Token::Newline)
            .copied()
            .collect()
    }
}

impl<'a, B: CstBuilder<Token=Token, Rule=Rule>> ParserCallbacks<'a> for Parser<'a, B, ()> {
    type Diagnostic = Diagnostic;
    type Context = ();

    fn create_tokens(
        _context: &mut <Self as ParserCallbacks<'a>>::Context,
        source: &str,
        diags: &mut Vec<Diagnostic>,
    ) -> (Vec<Token>, Vec<Span>) {
        tokenize(source, diags)
    }
    fn create_diagnostic(&self, span: Span, message: String) -> Diagnostic {
        Diagnostic::error()
            .with_message(message)
            .with_label(Label::primary((), span))
    }
    fn predicate_table_1(&self) -> bool {
        // don't skip whitespaces
        self.tokens.get(self.pos + 1).copied() != Some(Token::LBrak)
    }
    fn predicate_array_1(&self) -> bool {
        let la = self.lookahead();
        la.first() != Some(&Token::RBrak)
    }
    fn predicate_array_2(&self) -> bool {
        let la = self.lookahead();
        la.first() != Some(&Token::RBrak) && la.get(1) != Some(&Token::RBrak)
    }
    fn predicate_array_3(&self) -> bool {
        let la = self.lookahead();
        la.first() == Some(&Token::Comma)
    }
    fn assertion_array_table_1(&self) -> Option<Diagnostic> {
        // don't skip whitespaces
        if self.tokens.get(self.pos + 1).copied() != Some(Token::RBrak) {
            return Some(
                Diagnostic::error()
                    .with_message("invalid syntax, expected: ']]'")
                    .with_label(Label::primary((), self.span())),
            );
        }
        None
    }
}
