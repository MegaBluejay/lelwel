use core::fmt::Debug;
use core::ops::Range;

use crate::types::*;

pub trait TokenType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn is_skip(&self) -> bool;
    fn is_eof(&self) -> bool;
    fn eof() -> Self;
}

pub trait RuleType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn error() -> Self;
}

/// Abstract interface for CST tree construction.
///
/// Has no notion of skip/trivia tokens — all tokens arriving via [`token()`](CstBuilder::token)
/// are treated equally as children. Skip buffering is layered on top via [`LelwelBuilder`].
pub trait CstBuilder {
    type Token: Copy;
    type Rule;
    type Mark: Copy;
    type Checkpoint: Copy;
    type Output;

    fn new(spans: Vec<Span>) -> Self;
    fn token(&mut self, kind: Self::Token, text: &str);
    /// Like [`token()`](CstBuilder::token) but the caller indicates this is a skip/trivia token.
    /// The default implementation delegates to [`token()`](CstBuilder::token).
    /// Backends like [`CstData`] use this to exclude skip tokens from child counts.
    fn token_skip(&mut self, kind: Self::Token, text: &str) {
        self.token(kind, text);
    }
    fn start_rule(&mut self) -> Self::Mark;
    fn end_rule(&mut self, mark: Self::Mark, rule: Self::Rule);
    /// Like [`end_rule()`](CstBuilder::end_rule) but for the root node.
    /// Some backends need different handling (all content must be included).
    fn end_rule_root(&mut self, mark: Self::Mark, rule: Self::Rule) {
        self.end_rule(mark, rule);
    }
    fn mark(&self) -> Self::Mark;
    fn start_rule_before(&mut self, mark: Self::Mark) -> Self::Mark;
    fn checkpoint(&self) -> Self::Checkpoint;
    fn revert_to(&mut self, checkpoint: Self::Checkpoint);
    fn iterate_removed(&self, _checkpoint: Self::Checkpoint, _f: &mut dyn FnMut(Self::Rule, NodeRef)) {}
    fn node_ref(&self, _mark: Self::Mark) -> Option<NodeRef> { None }
    fn finish(self) -> Self::Output;
}

/// Wraps a [`CstBuilder`] and adds skip-token buffering.
///
/// # Flush policy
///
/// Tokens marked as "skip" are buffered and only committed to the inner builder
/// when a non-skip token arrives ([`advance`](LelwelBuilder::advance)). This means
/// trailing skip tokens inside a rule are excluded from the rule's child count:
///
/// - [`end_rule`](LelwelBuilder::end_rule) does **not** flush — trailing skips in
///   the buffer stay uncommitted.
/// - [`end_rule_root`](LelwelBuilder::end_rule_root) **does** flush — the root node
///   must include all content.
/// - All "opening" operations ([`start_rule`](LelwelBuilder::start_rule),
///   [`mark`](LelwelBuilder::mark), [`start_rule_before`](LelwelBuilder::start_rule_before))
///   flush, ensuring the position snapshot is accurate.
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
    pub fn inner(&self) -> &B { &self.inner }
    pub fn node_ref(&self, mark: B::Mark) -> Option<NodeRef> { self.inner.node_ref(mark) }

fn flush(&mut self) {
        for (kind, start, end) in &self.buffer[self.start_idx..] {
            let text = &self.source[*start..*end];
            self.inner.token_skip(*kind, text);
        }
        if self.in_ordered_choice {
            self.start_idx = self.buffer.len();
        } else {
            self.buffer.clear();
            self.start_idx = 0;
        }
    }

    /// Advance by one token. If `skip` is true, the token is buffered.
    /// If `skip` is false, buffered skips are flushed first, then the token is emitted.
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

    /// Close a rule. Does **not** flush — trailing skip tokens in the buffer
    /// stay uncommitted so they are excluded from this rule's child count.
    pub fn end_rule(&mut self, mark: B::Mark, rule: B::Rule) -> B::Mark {
        self.inner.end_rule(mark, rule);
        mark
    }

/// Close the root rule. **Does** flush — the root must include all content
    /// including trailing skip tokens.
    pub fn end_rule_root(&mut self, mark: B::Mark, rule: B::Rule) -> B::Mark {
        self.flush();
        self.inner.end_rule_root(mark, rule);
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

    pub fn collect_removed(&self, checkpoint: B::Checkpoint) -> Vec<(B::Rule, NodeRef)> {
        let mut result = Vec::new();
        self.inner.iterate_removed(checkpoint, &mut |rule, nr| result.push((rule, nr)));
        result
    }

    pub fn restore(
        &mut self,
        checkpoint: B::Checkpoint,
        start_idx: usize,
        buffer_len: usize,
    ) {
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

pub struct ParserState<B: CstBuilder> {
    pos: usize,
    current: B::Token,
    checkpoint: B::Checkpoint,
    start_idx: usize,
    buffer_len: usize,
    diag_count: usize,
}

pub struct Parser<'a, B: CstBuilder, Ctx> {
    pub builder: LelwelBuilder<'a, B>,
    pub tokens: Vec<B::Token>,
    pub pos: usize,
    pub current: B::Token,
    pub end_of_input: B::Token,
    pub max_offset: usize,
    pub context: Ctx,
    pub(crate) error_node: Option<B::Mark>,
    pub error_since_advance: bool,
    spans: Vec<Span>,
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

impl<'a, B, Ctx> ParserArgs for Parser<'a, B, Ctx>
where
    B: CstBuilder,
    B::Token: TokenType,
    B::Rule: RuleType,
    Self: ParserHooks<'a, B::Token, B::Rule>,
{
    type Diag = <Self as ParserHooks<'a, B::Token, B::Rule>>::Diag;
}

type Diag<P> = <P as ParserArgs>::Diag;

#[allow(clippy::while_let_loop, dead_code, unused_parens)]
impl<'a, B, Ctx> Parser<'a, B, Ctx>
where
    B: CstBuilder,
    B::Token: TokenType,
    B::Rule: RuleType,
    Self: ParserHooks<'a, B::Token, B::Rule>,
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
        let span = self.spans.get(self.pos).cloned().unwrap_or(self.max_offset..self.max_offset);
        self.builder.advance(self.current, false, span);
        loop {
            self.pos += 1;
            match self.tokens.get(self.pos) {
                Some(token) if token.is_skip() || self.predicate_skip_hook(*token) => {
                    let span = self.spans.get(self.pos).cloned().unwrap_or(self.max_offset..self.max_offset);
                    self.builder.advance(*token, true, span);
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
                    let span = self.spans.get(self.pos).cloned().unwrap_or(self.max_offset..self.max_offset);
                    self.builder.advance(*token, true, span);
                    self.pos += 1;
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
            self.error_node = Some(self.builder.start_rule());
        }
        self.advance(true, diags);
    }

    pub fn peek(&self, lookahead: usize) -> B::Token {
        self.tokens
            .iter()
            .skip(self.pos)
            .filter(|token| !token.is_skip())
            .nth(lookahead)
            .map_or(self.end_of_input, |it| *it)
    }

    pub fn peek_left(&self, lookbehind: usize) -> B::Token {
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
            self.builder.end_rule(error_node, B::Rule::error());
            if let Some(nr) = self.builder.node_ref(error_node) {
                self.create_node_error_hook(nr, diags);
            }
            self.error_node = None;
        }
    }

    pub fn open(&mut self, diags: &mut Vec<Diag<Self>>) -> B::Mark {
        self.close_error_node(diags);
        self.builder.start_rule()
    }

    pub fn open_before(&mut self, mark: B::Mark, diags: &mut Vec<Diag<Self>>) -> B::Mark {
        self.close_error_node(diags);
        self.builder.start_rule_before(mark)
    }

    pub fn close(&mut self, mark: B::Mark, rule: B::Rule, diags: &mut Vec<Diag<Self>>) -> B::Mark {
        self.close_error_node(diags);
        let closed = self.builder.end_rule(mark, rule);
        if let Some(nr) = self.builder.node_ref(closed) {
            self.create_node(rule, nr, diags);
        }
        closed
    }

    pub fn close_root(&mut self, mark: B::Mark, rule: B::Rule, diags: &mut Vec<Diag<Self>>) -> B::Mark {
        self.close_error_node(diags);
        let closed = self.builder.end_rule_root(mark, rule);
        if let Some(nr) = self.builder.node_ref(closed) {
            self.create_node(rule, nr, diags);
        }
        closed
    }

    pub fn mark(&mut self, diags: &mut Vec<Diag<Self>>) -> B::Mark {
        self.close_error_node(diags);
        self.builder.mark()
    }

    pub fn span(&self) -> Span {
        self.spans.get(self.pos).cloned().unwrap_or(self.max_offset..self.max_offset)
    }

    pub fn get_state(&self, diags: &[Diag<Self>]) -> ParserState<B> {
        let (checkpoint, start_idx, buffer_len) = self.builder.state();
        ParserState {
            pos: self.pos,
            current: self.current,
            checkpoint,
            start_idx,
            buffer_len,
            diag_count: diags.len(),
        }
    }

    pub fn set_state(&mut self, state: &ParserState<B>, diags: &mut Vec<Diag<Self>>) {
        self.pos = state.pos;
        self.current = state.current;
        diags.truncate(state.diag_count);
        let removed = self.builder.collect_removed(state.checkpoint);
        for (rule, node_ref) in removed {
            self.delete_node(rule, node_ref);
        }
        self.builder.restore(state.checkpoint, state.start_idx, state.buffer_len);
    }

    pub fn parse_with(
        mut self,
        start_rule: impl FnOnce(&mut Self, &mut Vec<Diag<Self>>),
        diags: &mut Vec<Diag<Self>>,
        root: B::Rule,
    ) -> B::Output {
        let token_count = self.tokens.len();
        let m = self.builder.start_rule();
        self.init_skip();

        start_rule(&mut self, diags);

        self.close_error_node(diags);
        if self.pos != token_count {
            self.error(diags, err![self, "invalid syntax, expected: <end of file>"]);
            let error_tree = self.builder.start_rule();
            while self.pos < token_count {
                let token = self.tokens[self.pos];
                let span = self.spans[self.pos].clone();
                self.builder.advance(token, token.is_skip(), span);
                self.pos += 1;
            }
            self.builder.end_rule(error_tree, B::Rule::error());
            if let Some(nr) = self.builder.node_ref(error_tree) {
                self.create_node_error_hook(nr, diags);
            }
        }

        let closed = self.builder.end_rule_root(m, root);
        if let Some(nr) = self.builder.node_ref(closed) {
            self.create_node(root, nr, diags);
        }
        self.builder.finish()
    }
}

impl<'a, B, Ctx> Parser<'a, B, Ctx>
where
    B: CstBuilder,
    B::Token: TokenType,
    B::Rule: RuleType,
    Self: ParserHooks<'a, B::Token, B::Rule>,
    Ctx: From<<Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx>,
    <Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx: From<Ctx>,
{
    pub fn new_with_context(source: &'a str, diags: &mut Vec<Diag<Self>>, context: Ctx) -> Self {
        let mut ctx = <Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx::from(context);
        let (tokens, spans) = Self::create_tokens_hook(&mut ctx, source, diags);
        let max_offset = source.len();
        let inner = B::new(spans.clone());
        let builder = LelwelBuilder::new(inner, source);
        Self {
            current: B::Token::eof(),
            end_of_input: B::Token::eof(),
            builder,
            tokens,
            spans,
            pos: 0,
            max_offset,
            context: Ctx::from(ctx),
            error_node: None,
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

// CstData-specific parser impl block removed — examples use builder.inner() directly