use crate::lexer::{Token, tokenize};
use codespan_reporting::diagnostic::Label;

pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

impl<'a, B: CstBuilder<Token=Token, Rule=Rule>> ParserCallbacks<'a> for Parser<'a, B, ()> {
    type Diagnostic = Diagnostic;
    type Context = ();

    fn create_tokens(
        _context: &mut <Self as ParserCallbacks<'a>>::Context,
        source: &'a str,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>,
    ) -> (Vec<Token>, Vec<Span>) {
        tokenize(source, diags)
    }
    fn create_diagnostic(
        &self,
        span: Span,
        message: String,
    ) -> <Self as ParserCallbacks<'a>>::Diagnostic {
        <Self as ParserCallbacks<'a>>::Diagnostic::error()
            .with_message(message)
            .with_label(Label::primary((), span))
    }
}
