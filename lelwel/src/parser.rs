use core::fmt::Debug;
use core::ops::Range;

use crate::cst::*;
use crate::types::*;

pub trait TokenType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn is_skip(&self) -> bool;
    fn is_eof(&self) -> bool;
    fn eof() -> Self;
}

pub trait RuleType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn error() -> Self;
}

pub trait CstBuilder {
    type Token: Copy;
    type Rule;
    type Mark: Copy;
    type Checkpoint: Copy;
    type Output;

    fn new(spans: Vec<Span>) -> Self;
    fn token(&mut self, kind: Self::Token, text: &str);
    fn start_rule(&mut self) -> Self::Mark;
    fn end_rule(&mut self, mark: Self::Mark, rule: Self::Rule);
    fn mark(&self) -> Self::Mark;
    fn start_rule_before(&mut self, mark: Self::Mark) -> Self::Mark;
    fn checkpoint(&self) -> Self::Checkpoint;
    fn revert_to(&mut self, checkpoint: Self::Checkpoint);
    fn iterate_removed(&self, _checkpoint: Self::Checkpoint, _f: &mut dyn FnMut(Self::Rule, NodeRef)) {}
    fn node_ref(&self, _mark: Self::Mark) -> Option<NodeRef> { None }
    fn finish(self) -> Self::Output;
}

pub struct LelwelBuilder<'s, B: CstBuilder> {
    inner: B,
    source: &'s str,
    buffer: Vec<(B::Token, usize, usize)>,
    start_idx: usize,
    pub in_ordered_choice: bool,
}

impl<'s, B: CstBuilder> LelwelBuilder<'s, B> {
    pub fn new(inner: B, source: &'s str) -> Self {
        LelwelBuilder { inner, source, buffer: Vec::new(), start_idx: 0, in_ordered_choice: false }
    }

    pub fn into_inner(self) -> B { self.inner }
    pub fn source(&self) -> &'s str { self.source }
    pub fn node_ref(&self, mark: B::Mark) -> Option<NodeRef> { self.inner.node_ref(mark) }

    fn flush(&mut self) {
        for (kind, start, end) in &self.buffer[self.start_idx..] {
            let text = &self.source[*start..*end];
            self.inner.token(*kind, text);
        }
        if self.in_ordered_choice {
            self.start_idx = self.buffer.len();
        } else {
            self.buffer.clear();
            self.start_idx = 0;
        }
    }

    pub fn advance(&mut self, kind: B::Token, skip: bool, span: Range<usize>) {
        if skip {
            self.buffer.push((kind, span.start, span.end));
        } else {
            self.flush();
            self.inner.token(kind, &self.source[span]);
        }
    }

    pub fn start_rule(&mut self) -> B::Mark {
        self.flush();
        self.inner.start_rule()
    }

    pub fn end_rule(&mut self, mark: B::Mark, rule: B::Rule) -> B::Mark {
        self.inner.end_rule(mark, rule);
        mark
    }

    pub fn end_rule_root(&mut self, mark: B::Mark, rule: B::Rule) -> B::Mark {
        self.flush();
        self.inner.end_rule(mark, rule);
        mark
    }

    pub fn mark(&mut self) -> B::Mark {
        self.flush();
        self.inner.mark()
    }

    pub fn start_rule_before(&mut self, mark: B::Mark) -> B::Mark {
        self.flush();
        self.inner.start_rule_before(mark)
    }

    pub fn state(&self) -> (B::Checkpoint, usize, usize) {
        (self.inner.checkpoint(), self.start_idx, self.buffer.len())
    }

    pub fn restore(
        &mut self,
        checkpoint: B::Checkpoint,
        start_idx: usize,
        buffer_len: usize,
        on_delete: &mut dyn FnMut(B::Rule, NodeRef),
    ) {
        self.inner.iterate_removed(checkpoint, on_delete);
        self.inner.revert_to(checkpoint);
        self.buffer.truncate(buffer_len);
        self.start_idx = start_idx;
    }

    pub fn finish(self) -> B::Output { self.inner.finish() }
}

pub trait ParserHooks<'a, T: TokenType, R: RuleType> {
    type Diag;
    type Ctx;

    fn create_tokens_hook(
        ctx: &mut Self::Ctx,
        source: &'a str,
        diags: &mut Vec<Self::Diag>,
    ) -> (Vec<T>, Vec<Span>);
    fn create_diagnostic_hook(&self, span: Span, message: String) -> Self::Diag;
    fn predicate_skip_hook(&self, _token: T) -> bool {
        false
    }
    fn create_node(&mut self, _rule: R, _node_ref: NodeRef, _diags: &mut Vec<Self::Diag>) {}
    fn create_node_error_hook(&mut self, _node_ref: NodeRef, _diags: &mut Vec<Self::Diag>) {}
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
        $self.create_diagnostic_hook($self.span(), String::from($msg))
    }
}

pub trait ParserArgs {
    type Diag;
}

impl<'a, T, R, Ctx> ParserArgs for Parser<'a, T, R, Ctx>
where
    T: TokenType,
    R: RuleType,
    Self: ParserHooks<'a, T, R>,
{
    type Diag = <Self as ParserHooks<'a, T, R>>::Diag;
}

type Diag<P> = <P as ParserArgs>::Diag;

#[allow(clippy::while_let_loop, dead_code, unused_parens)]
impl<'a, T, R, Ctx> Parser<'a, T, R, Ctx>
where
    T: TokenType,
    R: RuleType,
    Self: ParserHooks<'a, T, R>,
{
    pub fn active_error(&self) -> bool {
        self.error_node.is_some() || self.error_since_advance
    }

    pub fn error(&mut self, diags: &mut Vec<Diag<Self>>, diag: Diag<Self>) {
        if self.active_error() {
            return;
        }
        self.error_since_advance = true;
        diags.push(diag);
    }
    pub fn advance(&mut self, error: bool, diags: &mut Vec<Diag<Self>>) {
        if !error {
            self.close_error_node(diags);
            self.error_since_advance = false;
        }
        self.cst.data.advance(self.current, false);
        loop {
            self.pos += 1;
            match self.tokens.get(self.pos) {
                Some(token) if token.is_skip() || self.predicate_skip_hook(*token) => {
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
                Some(token) if token.is_skip() || self.predicate_skip_hook(*token) => {
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
    pub fn advance_with_error(&mut self, diags: &mut Vec<Diag<Self>>, diag: Diag<Self>) {
        self.error(diags, diag);
        if self.error_node.is_none() {
            self.error_node = Some(self.cst.data.open());
        }
        self.advance(true, diags);
    }
    pub fn peek(&self, lookahead: usize) -> T {
        self.tokens
            .iter()
            .skip(self.pos)
            .filter(|token| !token.is_skip())
            .nth(lookahead)
            .map_or(self.end_of_input, |it| *it)
    }
    pub fn peek_left(&self, lookbehind: usize) -> T {
        self.tokens
            .iter()
            .take(self.pos + 1)
            .rev()
            .filter(|token| !token.is_skip())
            .nth(lookbehind)
            .map_or(self.end_of_input, |it| *it)
    }
    pub fn close_error_node(&mut self, diags: &mut Vec<Diag<Self>>) {
        if let Some(error_node) = self.error_node {
            self.cst.data.close(error_node, R::error());
            self.create_node_error_hook(NodeRef(error_node.0), diags);
            self.error_node = None;
        }
    }
    pub fn open(&mut self, diags: &mut Vec<Diag<Self>>) -> MarkOpened {
        self.close_error_node(diags);
        self.cst.data.open()
    }
    pub fn open_before(&mut self, mark: MarkClosed, diags: &mut Vec<Diag<Self>>) -> MarkOpened {
        self.close_error_node(diags);
        self.cst.data.open_before(mark)
    }
    pub fn close(&mut self, mark: MarkOpened, rule: R, diags: &mut Vec<Diag<Self>>) -> MarkClosed {
        self.close_error_node(diags);
        self.cst.data.close(mark, rule)
    }
    pub fn close_root(
        &mut self,
        mark: MarkOpened,
        rule: R,
        diags: &mut Vec<Diag<Self>>,
    ) -> MarkClosed {
        self.close_error_node(diags);
        self.cst.data.close_root(mark, rule)
    }
    pub fn mark(&mut self, diags: &mut Vec<Diag<Self>>) -> MarkClosed {
        self.close_error_node(diags);
        self.cst.data.mark()
    }
    pub fn span(&self) -> Span {
        self.cst
            .data
            .spans
            .get(self.pos)
            .map_or(self.max_offset..self.max_offset, |span| span.clone())
    }
    pub fn get_state(&self, diags: &[Diag<Self>]) -> ParserState<T> {
        ParserState {
            pos: self.pos,
            current: self.current,
            truncation_mark: self.cst.data.mark_truncation(),
            diag_count: diags.len(),
        }
    }
    pub fn set_state(&mut self, state: &ParserState<T>, diags: &mut Vec<Diag<Self>>) {
        self.pos = state.pos;
        self.current = state.current;
        diags.truncate(state.diag_count);
        for i in state.truncation_mark.node_count..self.cst.data.nodes.len() {
            if let Node::Rule(rule, _) = self.cst.data.nodes[i] {
                self.delete_node(rule, NodeRef(i));
            }
        }
        self.cst.data.truncate(state.truncation_mark.clone());
    }
    pub fn parse_with(
        mut self,
        start_rule: impl FnOnce(&mut Self, &mut Vec<Diag<Self>>),
        diags: &mut Vec<Diag<Self>>,
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
            self.create_node_error_hook(NodeRef(error_tree.0), diags);
        }

        let closed = self.cst.data.close_root(m, root);
        self.create_node(root, NodeRef(closed.0), diags);
        self.cst
    }
}

impl<'a, T, R, Ctx> Parser<'a, T, R, Ctx>
where
    T: TokenType,
    R: RuleType,
    Self: ParserHooks<'a, T, R>,
    Ctx: From<<Self as ParserHooks<'a, T, R>>::Ctx>,
    <Self as ParserHooks<'a, T, R>>::Ctx: From<Ctx>,
{
    pub fn new_with_context(source: &'a str, diags: &mut Vec<Diag<Self>>, context: Ctx) -> Self {
        let mut ctx = <Self as ParserHooks<'a, T, R>>::Ctx::from(context);
        let (tokens, spans) = Self::create_tokens_hook(&mut ctx, source, diags);
        let max_offset = source.len();
        Self {
            current: T::eof(),
            end_of_input: T::eof(),
            cst: Cst {
                data: CstData::new(spans),
                source,
            },
            tokens,
            pos: 0,
            max_offset,
            context: Ctx::from(ctx),
            error_node: None,
            in_ordered_choice: false,
            error_since_advance: false,
        }
    }
    pub fn new(source: &'a str, diags: &mut Vec<Diag<Self>>) -> Self
    where
        Ctx: Default,
    {
        #[allow(clippy::unit_arg)]
        Self::new_with_context(source, diags, Ctx::default())
    }
}
