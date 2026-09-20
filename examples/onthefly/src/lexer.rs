//! The token type used by the generated parser.
//!
//! This is normally produced by `logos`, but this example intentionally uses a
//! hand written lexer so the on-the-fly path is easy to follow.
#[derive(Debug, enumset::EnumSetType)]
#[allow(clippy::upper_case_acronyms)]
pub enum Token {
    EOF,
    Error,
    Whitespace,
    Id,
    Num,
    Plus,
    Minus,
    Star,
    Eq,
    Semi,
    Comma,
    LParen,
    RParen,
}

/// The lexer state that lives inside `Parser.state`.
///
/// It has to implement `Clone` because the parser snapshots it when an ordered
/// choice backtracks. When a branch fails, the parser restores both the tokens
/// that were lexed in the failed branch and this state, so the next branch
/// observes the exact same lexer position and produces the same tokens again.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LexState {
    /// Byte offset of the next token in the source.
    pub(crate) offset: usize,
}
