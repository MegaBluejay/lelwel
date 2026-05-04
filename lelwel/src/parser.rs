use core::fmt::Debug;

use crate::types::*;
use crate::cst::*;

pub trait TokenType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn is_skip(&self) -> bool;
    fn is_eof(&self) -> bool;
    fn eof() -> Self;
}

pub trait RuleType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn error() -> Self;
}

pub trait ParserHooks<'a, T: TokenType, R: RuleType> {
    type Diagnostic;
    type Context;

    fn create_tokens(ctx: &mut Self::Context, source: &'a str, diags: &mut Vec<Self::Diagnostic>) -> (Vec<T>, Vec<Span>);
    fn create_diagnostic(&self, span: Span, message: String) -> Self::Diagnostic;
    fn predicate_skip(&self, _token: T) -> bool {
        false
    }
    fn create_node(&mut self, _rule: R, _node_ref: NodeRef, _diags: &mut Vec<Self::Diagnostic>) {}
    fn create_node_error(&mut self, _node_ref: NodeRef, _diags: &mut Vec<Self::Diagnostic>) {}
    fn delete_node(&mut self, _rule: R, _node_ref: NodeRef) {}
}

pub struct ParserState<T> {
    pos: usize,
    current: T,
    truncation_mark: MarkTruncation,
    diag_count: usize,
}

pub struct Parser<'a, T: TokenType, R: RuleType, Ctx> {
    pub cst: Cst<'a, T, R>,
    pub tokens: Vec<T>,
    pub pos: usize,
    pub current: T,
    pub end_of_input: T,
    pub max_offset: usize,
    pub context: Ctx,
    pub(crate) error_node: Option<MarkOpened>,
    #[allow(dead_code)]
    pub in_ordered_choice: bool,
    pub error_since_advance: bool,
}

#[macro_export]
macro_rules! err {
    [$self:expr, $msg:literal] => {
        $self.create_diagnostic($self.span(), String::from($msg))
    }
}

#[allow(clippy::while_let_loop, dead_code, unused_parens)]
impl<'a, T, R, Ctx> Parser<'a, T, R, Ctx>
where
    T: TokenType,
    R: RuleType,
    Self: ParserHooks<'a, T, R>,
    Ctx: From<<Self as ParserHooks<'a, T, R>>::Context>,
    <Self as ParserHooks<'a, T, R>>::Context: From<Ctx>,
{
    fn active_error(&self) -> bool {
        self.error_node.is_some() || self.error_since_advance
    }
    fn error(
        &mut self,
        diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>,
        diag: <Self as ParserHooks<'a, T, R>>::Diagnostic,
    ) {
        if self.active_error() {
            return;
        }
        self.error_since_advance = true;
        diags.push(diag);
    }
    fn advance(&mut self, error: bool, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) {
        if !error {
            self.close_error_node(diags);
            self.error_since_advance = false;
        }
        self.cst.data.advance(self.current, false);
        loop {
            self.pos += 1;
            match self.tokens.get(self.pos) {
                Some(token) if token.is_skip() || self.predicate_skip(*token) => {
                    self.cst.data.advance(*token, true);
                    continue;
                }
                Some(token) => {
                    self.current = *token;
                    break;
                }
                None => {
                    self.current = self.end_of_input;
                    break;
                }
            }
        }
    }
    fn init_skip(&mut self) {
        loop {
            match self.tokens.get(self.pos) {
                Some(token) if token.is_skip() || self.predicate_skip(*token) => {
                    self.pos += 1;
                    self.cst.data.advance(*token, true);
                    continue;
                }
                Some(token) => {
                    self.current = *token;
                    break;
                }
                None => {
                    self.current = self.end_of_input;
                    break;
                }
            }
        }
    }
    fn advance_with_error(
        &mut self,
        diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>,
        diag: <Self as ParserHooks<'a, T, R>>::Diagnostic,
    ) {
        self.error(diags, diag);
        if self.error_node.is_none() {
            self.error_node = Some(self.cst.data.open());
        }
        self.advance(true, diags);
    }
    fn peek(&self, lookahead: usize) -> T {
        self.tokens
            .iter()
            .skip(self.pos)
            .filter(|token| !token.is_skip())
            .nth(lookahead)
            .map_or(self.end_of_input, |it| *it)
    }
    fn peek_left(&self, lookbehind: usize) -> T {
        self.tokens
            .iter()
            .take(self.pos + 1)
            .rev()
            .filter(|token| !token.is_skip())
            .nth(lookbehind)
            .map_or(self.end_of_input, |it| *it)
    }
    fn close_error_node(&mut self, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) {
        if let Some(error_node) = self.error_node {
            self.cst.data.close(error_node, R::error());
            <Self as ParserHooks<'a, T, R>>::create_node_error(self, NodeRef(error_node.0), diags);
            self.error_node = None;
        }
    }
    fn open(&mut self, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) -> MarkOpened {
        self.close_error_node(diags);
        self.cst.data.open()
    }
    fn open_before(&mut self, mark: MarkClosed, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) -> MarkOpened {
        self.close_error_node(diags);
        self.cst.data.open_before(mark)
    }
    fn close(&mut self, mark: MarkOpened, rule: R, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) -> MarkClosed {
        self.close_error_node(diags);
        self.cst.data.close(mark, rule)
    }
    fn close_root(&mut self, mark: MarkOpened, rule: R, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) -> MarkClosed {
        self.close_error_node(diags);
        self.cst.data.close_root(mark, rule)
    }
    fn mark(&mut self, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>) -> MarkClosed {
        self.close_error_node(diags);
        self.cst.data.mark()
    }
    fn span(&self) -> Span {
        self.cst.data.spans
            .get(self.pos)
            .map_or(self.max_offset..self.max_offset, |span| span.clone())
    }
    fn get_state(&self, diags: &[<Self as ParserHooks<'a, T, R>>::Diagnostic]) -> ParserState<T> {
        ParserState {
            pos: self.pos,
            current: self.current,
            truncation_mark: self.cst.data.mark_truncation(),
            diag_count: diags.len(),
        }
    }
    fn set_state(
        &mut self,
        state: &ParserState<T>,
        diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>,
    ) {
        self.pos = state.pos;
        self.current = state.current;
        diags.truncate(state.diag_count);
        for i in state.truncation_mark.node_count..self.cst.data.nodes.len() {
            if let Node::Rule(rule, _) = self.cst.data.nodes[i] {
                <Self as ParserHooks<'a, T, R>>::delete_node(self, rule, NodeRef(i));
            }
        }
        self.cst.data.truncate(state.truncation_mark.clone());
    }
    pub fn new_with_context(
        source: &'a str,
        diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>,
        context: Ctx,
    ) -> Self {
        let mut ctx = <Self as ParserHooks<'a, T, R>>::Context::from(context);
        let (tokens, spans) = <Self as ParserHooks<'a, T, R>>::create_tokens(&mut ctx, source, diags);
        let max_offset = source.len();
        Self {
            current: T::eof(),
            end_of_input: T::eof(),
            cst: Cst { data: CstData::new(spans), source },
            tokens,
            pos: 0,
            max_offset,
            context: Ctx::from(ctx),
            error_node: None,
            in_ordered_choice: false,
            error_since_advance: false,
        }
    }
    pub fn new(
        source: &'a str,
        diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>,
    ) -> Self
    where
        Ctx: Default,
    {
        #[allow(clippy::unit_arg)]
        Self::new_with_context(source, diags, Ctx::default())
    }
    pub fn parse_with(
        mut self,
        start_rule: impl FnOnce(&mut Self, &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>),
        diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>,
        root: R,
    ) -> Cst<'a, T, R> {
        let token_count = self.tokens.len();
        let m = self.open(diags);
        self.init_skip();

        start_rule(&mut self, diags);

        self.close_error_node(diags);
        if self.pos != token_count {
            self.error(diags, err![self, "invalid syntax, expected: <end of file>"]);
            let error_tree = self.open(diags);
            while self.pos < token_count {
                let token = self.tokens[self.pos];
                self.cst.data.advance(token, token.is_skip());
                self.pos += 1;
            }
            self.cst.data.close(error_tree, R::error());
            <Self as ParserHooks<'a, T, R>>::create_node_error(&mut self, NodeRef(error_tree.0), diags);
        }

        let closed = self.cst.data.close_root(m, root);
        <Self as ParserHooks<'a, T, R>>::create_node(&mut self, root, NodeRef(closed.0), diags);
        self.cst
    }
}