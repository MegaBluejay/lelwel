use crate::lexer::{LexState, Token};
use codespan_reporting::diagnostic::Label;
use std::cell::RefCell;
use std::rc::Rc;

pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;

/// One call into `lex`, as recorded by [`LexLog`].
///
/// `expected` is the set of tokens the parser was ready to accept at that
/// point. The generated parser passes it down from its predict and follow
/// sets: a single token for `expect!`, the predict set of an alternation or
/// ordered choice branch, or the follow set of a loop or optional. An empty
/// slice means the parser asked for a token without an expectation, as it
/// does when checking for the end of input.
pub type LexCall = (usize, Token, &'static [Token]);

/// Shared audit log of lexer activity.
///
/// Every call into `lex` records the byte offset it started from, the token
/// it produced, and the expected set it was given. Attempts that ordered
/// choice later rolled back are included. This is how the tests can prove
/// that a failed branch is re-lexed from scratch instead of reusing cached
/// tokens, and that the generated predict and follow sets are the ones the
/// grammar implies.
///
/// The log lives in the `Context` rather than in the lexer `State` on
/// purpose: `Context` is *not* restored when ordered choice backtracks,
/// while `State` is. A log stored in `State` would be rolled back together
/// with the lexer position and hide the re-lexing.
#[derive(Debug, Default, Clone)]
pub struct LexLog {
    /// Every call the lexer served, in order.
    pub calls: Rc<RefCell<Vec<LexCall>>>,
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

impl<'a> ParserCallbacks<'a> for Parser<'a> {
    type Diagnostic = Diagnostic;
    // The log keeps every lexer call that happened, including rolled back
    // ones, so tests can verify that tokens are actually re-lexed.
    type Context = LexLog;
    // The lexer state is stored in the parser itself so it can be
    // snapshotted and restored when ordered choice backtracks.
    type State = LexState;

    /// No tokens are lexed upfront. The lexer runs on demand through `lex`.
    fn create_tokens(
        _context: &mut Self::Context,
        _source: &'a str,
        _diags: &mut Vec<Diagnostic>,
    ) -> (Vec<Token>, Vec<Span>) {
        (Vec::new(), Vec::new())
    }

    /// Lexes the next token, advancing the lexer state.
    fn lex(
        &mut self,
        expected: &'static [Token],
        diags: &mut Vec<Diagnostic>,
    ) -> Option<(Token, Span)> {
        // The parser only asks for a new token once it has consumed all
        // lexed ones, so we must resume exactly after the last cached token.
        // If an ordered choice rolled back the token list but failed to
        // restore the lexer state, the offsets would disagree and this
        // assertion would fire.
        let resume_at = if self.tokens.is_empty() {
            0
        } else {
            self.cst.data.spans[self.tokens.len() - 1].end
        };
        assert_eq!(
            self.state.offset, resume_at,
            "lexer state is out of sync with the lexed tokens",
        );

        let rest = &self.cst.source[self.state.offset..];
        let Some(first) = rest.chars().next() else {
            return None; // end of input
        };

        let (token, len) = match first {
            'a'..='z' | 'A'..='Z' => {
                let len = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .count();
                (Token::Id, len)
            }
            '0'..='9' => {
                let len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
                (Token::Num, len)
            }
            '+' => (Token::Plus, 1),
            '-' => (Token::Minus, 1),
            '*' => (Token::Star, 1),
            '=' => (Token::Eq, 1),
            ';' => (Token::Semi, 1),
            ',' => (Token::Comma, 1),
            '(' => (Token::LParen, 1),
            ')' => (Token::RParen, 1),
            c if c.is_whitespace() => {
                let len = rest.chars().take_while(|c| c.is_whitespace()).count();
                (Token::Whitespace, len)
            }
            invalid => {
                diags.push(
                    Diagnostic::error()
                        .with_message(format!("invalid character {invalid:?}"))
                        .with_label(Label::primary(
                            (),
                            self.state.offset..self.state.offset + invalid.len_utf8(),
                        )),
                );
                (Token::Error, invalid.len_utf8())
            }
        };

        let span = self.state.offset..self.state.offset + len;
        self.state.offset += len;
        self.context
            .calls
            .borrow_mut()
            .push((span.start, token, expected));
        Some((token, span))
    }

    fn create_diagnostic(&self, span: Span, message: String) -> Diagnostic {
        Diagnostic::error()
            .with_message(message)
            .with_label(Label::primary((), span))
    }
}
