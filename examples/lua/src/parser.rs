use crate::ast::{AstNode, Exp};
use crate::lexer::{Token, tokenize};
use codespan_reporting::diagnostic::Label;

pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

impl<'a> ParserCallbacks<'a> for Parser<'a, CstData<Token, Rule>, ()> {
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

    fn create_node_expstat(&mut self, node: NodeRef, diags: &mut Vec<Diagnostic>) {
        let inner = self.builder.inner();
        inner.children(node).for_each(|c| {
            if let Some(exp) = Exp::cast(inner, c) {
                match exp {
                    Exp::Callexp(_) => {}
                    _ => {
                        diags.push(
                            Diagnostic::error()
                                .with_message("unexpected expression kind")
                                .with_label(Label::primary((), inner.span(c)))
                                .with_note("note: expected call expression"),
                        );
                    }
                }
            }
        });
    }
    fn create_node_assignstat(&mut self, node: NodeRef, diags: &mut Vec<Diagnostic>) {
        let inner = self.builder.inner();
        inner.children(node).for_each(|c| {
            if let Some(exp) = Exp::cast(inner, c) {
                match exp {
                    Exp::Nameexp(_) | Exp::Indexexp(_) | Exp::Fieldexp(_) => {}
                    _ => {
                        diags.push(
                            Diagnostic::error()
                                .with_message("unexpected expression kind")
                                .with_label(Label::primary((), inner.span(c)))
                                .with_note("note: expected name, index, or field expression"),
                        );
                    }
                }
            }
        });
    }
    fn create_node_attrib(&mut self, node: NodeRef, diags: &mut Vec<Diagnostic>) {
        let inner = self.builder.inner();
        inner
            .children(node)
            .find_map(|n| inner.match_token(n, Token::Name).map(|span| (&self.builder.source()[span.clone()], span)))
            .inspect(|(value, span)| {
                if *value != "const" && *value != "close" {
                    diags.push(
                        Diagnostic::error()
                            .with_message(format!("unexpected attribute name: '{value}'"))
                            .with_label(Label::primary((), span.clone()))
                            .with_note("note: expected 'const' or 'close'"),
                    );
                }
            });
    }

    fn predicate_forstat_1(&self) -> bool {
        self.peek(1) == Token::Equal
    }
    fn predicate_pars_1(&self) -> bool {
        self.peek(1) != Token::Ellipsis
    }
    fn predicate_fieldlist_1(&self) -> bool {
        self.peek(1) != Token::RBrace
    }
    fn predicate_field_1(&self) -> bool {
        self.peek(1) == Token::Equal
    }
}
