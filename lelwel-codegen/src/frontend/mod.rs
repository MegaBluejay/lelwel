pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod sema;

pub use lelwel::{CstIndex, MarkClosed, MarkOpened, MarkTruncation, NodeRef, Span};
pub use lexer::Token;
pub use parser::{Rule, Rules};
pub use parser::ParserCallbacks as ParserCallbacks;

pub type Cst<'a> = lelwel::Cst<'a, Token, Rule>;
pub type CstChildren<'a> = lelwel::CstChildren<'a, Token, Rule>;
pub type CstData = lelwel::CstData<Token, Rule>;
pub type Node = lelwel::Node<Token, Rule>;
