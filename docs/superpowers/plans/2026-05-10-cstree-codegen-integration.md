# Cstree Codegen Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the lelwel codegen generate generic parser code that works with both `CstData<T,R>` and `GreenNodeBuilder<S>` backends.

**Architecture:** `CstBuilder` gains `T,R` type parameters (replacing associated types), a GAT `Output<'a>`, and `finish(self, src) -> Output<'a>`. `LelwelBuilder`/`Parser` gain `T,R` params. `GreenNodeBuilder` impl uses `Into<S>` for zero-cost conversion. Codegen generates `SyntaxKind` (unified token+rule enum with `derive(Syntax)`) when `cstree` feature is enabled.

**Tech Stack:** Rust, lelwel runtime, lelwel-codegen (proc-macro2/quote), cstree

---

### Task 1: Refactor CstBuilder trait — type params, GAT Output, finish takes source

**Files:**
- Modify: `lelwel/src/parser.rs`

- [ ] **Step 1: Change `CstBuilder` trait**

Replace associated types `Token` and `Rule` with generic type params `T: Copy, R`. Add GAT `type Output<'a>` and change `fn finish(self)` to `fn finish<'a>(self, src: &'a str) -> Self::Output<'a>`.

Current:
```rust
pub trait CstBuilder {
    type Token: Copy;
    type Rule;
    type Mark: Copy;
    type Checkpoint: Copy;
    type Output;
    fn new(spans: Vec<Span>) -> Self;
    fn token(&mut self, kind: Self::Token, text: &str);
    fn token_skip(&mut self, kind: Self::Token, text: &str) { self.token(kind, text); }
    fn start_rule(&mut self) -> Self::Mark;
    fn end_rule(&mut self, mark: Self::Mark, rule: Self::Rule);
    fn end_rule_root(&mut self, mark: Self::Mark, rule: Self::Rule) { self.end_rule(mark, rule); }
    fn mark(&self) -> Self::Mark;
    fn start_rule_before(&mut self, mark: Self::Mark) -> Self::Mark;
    fn checkpoint(&self) -> Self::Checkpoint;
    fn revert_to(&mut self, checkpoint: Self::Checkpoint);
    fn iterate_removed(&self, _checkpoint: Self::Checkpoint, _f: &mut dyn FnMut(Self::Rule, NodeRef)) {}
    fn node_ref(&self, _mark: Self::Mark) -> Option<NodeRef> { None }
    fn finish(self) -> Self::Output;
}
```

New:
```rust
pub trait CstBuilder<T: Copy, R> {
    type Mark: Copy;
    type Checkpoint: Copy;
    type Output<'a>;

    fn new(spans: Vec<Span>) -> Self;
    fn token(&mut self, kind: T, text: &str);
    fn token_skip(&mut self, kind: T, text: &str) { self.token(kind, text); }
    fn start_rule(&mut self) -> Self::Mark;
    fn end_rule(&mut self, mark: Self::Mark, rule: R);
    fn end_rule_root(&mut self, mark: Self::Mark, rule: R) { self.end_rule(mark, rule); }
    fn mark(&self) -> Self::Mark;
    fn start_rule_before(&mut self, mark: Self::Mark) -> Self::Mark;
    fn checkpoint(&self) -> Self::Checkpoint;
    fn revert_to(&mut self, checkpoint: Self::Checkpoint);
    fn iterate_removed(&self, _checkpoint: Self::Checkpoint, _f: &mut dyn FnMut(R, NodeRef)) {}
    fn node_ref(&self, _mark: Self::Mark) -> Option<NodeRef> { None }
    fn finish<'a>(self, src: &'a str) -> Self::Output<'a>;
}
```


- [ ] **Step 2: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: errors — `CstData` impl, `GreenNodeBuilder` impl, `LelwelBuilder`, `Parser` all need updating

- [ ] **Step 3: Commit**

```bash
git add lelwel/src/parser.rs
git commit -m "feat(runtime): refactor CstBuilder — type params T,R, GAT Output, finish takes source"
```

---

### Task 2: Update LelwelBuilder — add T,R type params + PhantomData

**Files:**
- Modify: `lelwel/src/parser.rs`

- [ ] **Step 1: Update `LelwelBuilder` struct**

```rust
use core::marker::PhantomData;

pub struct LelwelBuilder<'s, B: CstBuilder<T, R>, T: Copy, R> {
    inner: B,
    source: &'s str,
    buffer: Vec<(T, usize, usize)>,
    start_idx: usize,
    pub in_ordered_choice: bool,
    _rule: PhantomData<R>,
}
```

- [ ] **Step 2: Update `LelwelBuilder` impl block**

Change `impl<'s, B: CstBuilder> LelwelBuilder<'s, B>` to `impl<'s, B: CstBuilder<T, R>, T: Copy, R> LelwelBuilder<'s, B, T, R>`.

Update all method signatures that reference `B::Token` → `T` and `B::Rule` → `R`:

```rust
impl<'s, B: CstBuilder<T, R>, T: Copy, R> LelwelBuilder<'s, B, T, R> {
    pub fn new(inner: B, source: &'s str) -> Self {
        LelwelBuilder { inner, source, buffer: Vec::new(), start_idx: 0, in_ordered_choice: false, _rule: PhantomData }
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

    pub fn advance(&mut self, kind: T, skip: bool, span: Range<usize>) {
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

    pub fn end_rule(&mut self, mark: B::Mark, rule: R) -> B::Mark {
        self.inner.end_rule(mark, rule);
        mark
    }

    pub fn end_rule_root(&mut self, mark: B::Mark, rule: R) -> B::Mark {
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

    pub fn collect_removed(&self, checkpoint: B::Checkpoint) -> Vec<(R, NodeRef)> {
        let mut result = Vec::new();
        self.inner.iterate_removed(checkpoint, &mut |rule, nr| result.push((rule, nr)));
        result
    }

    pub fn restore(&mut self, checkpoint: B::Checkpoint, start_idx: usize, buffer_len: usize) {
        self.inner.revert_to(checkpoint);
        self.buffer.truncate(buffer_len);
        self.start_idx = start_idx;
    }

    pub fn finish(self) -> B::Output<'static> {
        // Note: 'static lifetime for the output — the source lifetime is handled
        // by the caller which knows the actual lifetime. The GAT 'a on finish
        // will be inferred at the call site.
        todo!()
    }
}
```

Wait — `finish` has a lifetime issue. `B::Output<'a>` has a GAT lifetime, but `LelwelBuilder` doesn't carry the source lifetime as a GAT param. Let me think...

Actually, `LelwelBuilder` already has `'s` as the source lifetime. The `finish` method should pass the source through:

```rust
pub fn finish(self) -> B::Output<'s> {
    self.inner.finish(self.source)
}
```

This works because `'s` is the concrete lifetime of the source, and we pass it to `finish` which returns `B::Output<'s>`. No `todo!()` needed — just call `self.inner.finish(self.source)`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: errors — `Parser` still uses old `CstBuilder` without `T,R`, `CstData` and `GreenNodeBuilder` impls need updating

- [ ] **Step 4: Commit**

```bash
git add lelwel/src/parser.rs
git commit -m "feat(runtime): add T,R type params + PhantomData to LelwelBuilder"
```

---

### Task 3: Update Parser — add T,R type params

**Files:**
- Modify: `lelwel/src/parser.rs`

- [ ] **Step 1: Update `ParserState`**

```rust
pub struct ParserState<B: CstBuilder<T, R>, T: Copy, R> {
    pos: usize,
    current: T,
    checkpoint: B::Checkpoint,
    start_idx: usize,
    buffer_len: usize,
    diag_count: usize,
}
```

- [ ] **Step 2: Update `Parser` struct**

```rust
pub struct Parser<'a, B: CstBuilder<T, R>, T: Copy, R, Ctx> {
    pub builder: LelwelBuilder<'a, B, T, R>,
    pub tokens: Vec<T>,
    pub pos: usize,
    pub current: T,
    pub end_of_input: T,
    pub max_offset: usize,
    pub context: Ctx,
    pub(crate) error_node: Option<B::Mark>,
    pub error_since_advance: bool,
    spans: Vec<Span>,
    _rule: PhantomData<R>,
}
```

- [ ] **Step 3: Update `ParserArgs` trait and `Diag` type alias**

```rust
pub trait ParserArgs {
    type Diag;
}

impl<'a, B, T, R, Ctx> ParserArgs for Parser<'a, B, T, R, Ctx>
where
    B: CstBuilder<T, R>,
    T: TokenType,
    R: RuleType,
    Self: ParserHooks<'a, T, R>,
{
    type Diag = <Self as ParserHooks<'a, T, R>>::Diag;
}

type Diag<P> = <P as ParserArgs>::Diag;
```

- [ ] **Step 4: Update `Parser` impl blocks**

Change all `impl<'a, B, Ctx> Parser<'a, B, Ctx>` to `impl<'a, B, T, R, Ctx> Parser<'a, B, T, R, Ctx>` with appropriate where clauses. Update all method bodies:

- Remove references to `B::Token` → use `T` directly
- Remove references to `B::Rule` → use `R` directly
- `end_of_input` stays as `T` type
- `self.builder.finish()` → `self.builder.finish()` (the new `LelwelBuilder::finish` passes source through)

Key methods to update:

```rust
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
        self.builder.end_rule(error_node, R::error());
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

pub fn close(&mut self, mark: B::Mark, rule: R, diags: &mut Vec<Diag<Self>>) -> B::Mark {
    self.close_error_node(diags);
    let closed = self.builder.end_rule(mark, rule);
    if let Some(nr) = self.builder.node_ref(closed) {
        self.create_node(rule, nr, diags);
    }
    closed
}

pub fn close_root(&mut self, mark: B::Mark, rule: R, diags: &mut Vec<Diag<Self>>) -> B::Mark {
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

pub fn get_state(&self, diags: &[Diag<Self>]) -> ParserState<B, T, R> {
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

pub fn set_state(&mut self, state: &ParserState<B, T, R>, diags: &mut Vec<Diag<Self>>) {
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
    root: R,
) -> B::Output<'a> {
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
        self.builder.end_rule(error_tree, R::error());
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
```

Update `new_with_context` and `new`:

```rust
impl<'a, B, T, R, Ctx> Parser<'a, B, T, R, Ctx>
where
    B: CstBuilder<T, R>,
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
        let inner = B::new(spans.clone());
        let builder = LelwelBuilder::new(inner, source);
        Self {
            current: T::eof(),
            end_of_input: T::eof(),
            builder,
            tokens,
            spans,
            pos: 0,
            max_offset,
            context: Ctx::from(ctx),
            error_node: None,
            error_since_advance: false,
            _rule: PhantomData,
        }
    }

    pub fn new(source: &'a str, diags: &mut Vec<Diag<Self>>) -> Self
    where
        Ctx: Default,
    {
        Self::new_with_context(source, diags, Ctx::default())
    }
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: errors — `CstData` impl and `GreenNodeBuilder` impl need updating

- [ ] **Step 6: Commit**

```bash
git add lelwel/src/parser.rs
git commit -m "feat(runtime): add T,R type params to Parser and ParserState"
```

---

### Task 4: Update CstData CstBuilder impl

**Files:**
- Modify: `lelwel/src/cst.rs`

- [ ] **Step 1: Update `CstBuilder` impl for `CstData`**

Change to match the new trait signature with type params, GAT Output, and `finish` that wraps in `Cst`:

```rust
impl<T: TokenType, R: RuleType> CstBuilder<T, R> for CstData<T, R> {
    type Mark = usize;
    type Checkpoint = MarkTruncation;
    type Output<'a> = Cst<'a, T, R>;

    fn new(spans: Vec<Span>) -> Self {
        Self { spans, nodes: Vec::new(), token_count: 0, non_skip_len: 0 }
    }

    fn token(&mut self, kind: T, _text: &str) {
        self.nodes.push(Node::Token(kind, self.token_count.into()));
        self.token_count += 1;
        self.non_skip_len = self.nodes.len();
    }

    fn token_skip(&mut self, kind: T, _text: &str) {
        self.nodes.push(Node::Token(kind, self.token_count.into()));
        self.token_count += 1;
    }

    fn start_rule(&mut self) -> usize {
        let pos = self.nodes.len();
        self.nodes.push(Node::Rule(R::error(), 0.into()));
        self.non_skip_len = self.nodes.len();
        pos
    }

    fn end_rule(&mut self, mark: usize, rule: R) {
        let len = self.non_skip_len - 1;
        self.nodes[mark] = Node::Rule(
            rule,
            (if mark >= len { 0 } else { len - mark }).into(),
        );
    }

    fn end_rule_root(&mut self, mark: usize, rule: R) {
        self.non_skip_len = self.nodes.len();
        self.end_rule(mark, rule);
    }

    fn mark(&self) -> usize { self.nodes.len() }

    fn start_rule_before(&mut self, mark: usize) -> usize {
        self.nodes.insert(mark, Node::Rule(R::error(), 0.into()));
        self.non_skip_len += 1;
        mark
    }

    fn checkpoint(&self) -> MarkTruncation {
        MarkTruncation { node_count: self.nodes.len(), token_count: self.token_count }
    }

    fn revert_to(&mut self, cp: MarkTruncation) {
        self.nodes.truncate(cp.node_count);
        self.token_count = cp.token_count;
        self.non_skip_len = self.nodes.len();
    }

    fn iterate_removed(&self, checkpoint: MarkTruncation, f: &mut dyn FnMut(R, NodeRef)) {
        for i in checkpoint.node_count..self.nodes.len() {
            if let Node::Rule(rule, _) = &self.nodes[i] {
                f(*rule, NodeRef(i));
            }
        }
    }

    fn node_ref(&self, mark: usize) -> Option<NodeRef> { Some(NodeRef(mark)) }

    fn finish<'a>(self, src: &'a str) -> Cst<'a, T, R> {
        Cst::new(src, self)
    }
}
```

- [ ] **Step 2: Remove old finish method**

The old `Cst::new` is already public. The `finish` method now calls it directly. Make sure `Cst::new` is still `pub`.

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: errors — `GreenNodeBuilder` impl still needs updating

- [ ] **Step 4: Commit**

```bash
git add lelwel/src/cst.rs
git commit -m "feat(runtime): update CstData CstBuilder impl — type params, GAT Output, finish wraps Cst"
```

---

### Task 5: Update GreenNodeBuilder CstBuilder impl

**Files:**
- Modify: `lelwel/src/cstree.rs`

- [ ] **Step 1: Update `CstBuilder` impl for `GreenNodeBuilder`**

Change to match the new trait signature with `Into<S>` bounds:

```rust
use cstree::build::{Checkpoint, GreenNodeBuilder, NodeCache};
use cstree::green::GreenNode;
use cstree::interning::TokenInterner;
use cstree::Syntax;

use crate::{CstBuilder, Span};

impl<S: Syntax, T: Into<S>, R: Into<S>> CstBuilder<T, R> for GreenNodeBuilder<'static, 'static, S> {
    type Mark = Checkpoint;
    type Checkpoint = Checkpoint;
    type Output<'a> = (GreenNode, Option<NodeCache<'static, TokenInterner>>);

    fn new(_spans: Vec<Span>) -> Self {
        GreenNodeBuilder::new()
    }

    fn token(&mut self, kind: T, text: &str) {
        self.token(kind.into(), text);
    }

    fn start_rule(&mut self) -> Checkpoint {
        self.checkpoint()
    }

    fn end_rule(&mut self, mark: Checkpoint, rule: R) {
        self.start_node_at(mark, rule.into());
        self.finish_node();
    }

    fn mark(&self) -> Checkpoint {
        self.checkpoint()
    }

    fn start_rule_before(&mut self, mark: Checkpoint) -> Checkpoint {
        mark
    }

    fn checkpoint(&self) -> Checkpoint {
        self.checkpoint()
    }

    fn revert_to(&mut self, cp: Checkpoint) {
        self.revert_to(cp);
    }

    fn finish<'a>(self, _src: &'a str) -> Self::Output<'a> {
        self.finish()
    }
}
```

- [ ] **Step 2: Update unit tests**

The unit tests use `CstBuilder::finish()` which now takes source. Update:

```rust
// Old:
let (tree, _cache) = b.finish();
// New:
let (tree, _cache) = b.finish("");
```

Update all 4 test functions accordingly.

- [ ] **Step 3: Verify compilation and tests**

Run: `cargo test -p lelwel --features cstree 2>&1`
Expected: compiles, 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add lelwel/src/cstree.rs
git commit -m "feat(runtime): update GreenNodeBuilder CstBuilder impl — Into<S> conversions, GAT Output"
```

---

### Task 6: Update generated code — generic Parser type, Rules impl, gen_lexer repr(u32)

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs`

This task updates the generated code to use the new generic `Parser<'a, B, Token, Rule, Ctx>` and makes the `Rules` impl generic over `B: CstBuilder<Token, Rule>`. It also adds `#[repr(u32)]` + explicit discriminants to `Token` (in `gen_lexer`) and `Rule` (in `gen_generated`) when the `cstree` feature is enabled.

- [ ] **Step 1: Update `gen_parser` — Parser type in generated code**

In `gen_parser()` (line ~117), change the generated `use` statement. Old:

```rust
use lelwel::{ParserHooks, Parser, Span, Cst, NodeRef, CstData};
```

New — `Parser` no longer needs `CstData` explicitly, and `Cst` is still used for the output type:

```rust
use lelwel::{ParserHooks, Parser, Span, Cst, NodeRef};
```

- [ ] **Step 2: Update `gen_generated` — generated.rs imports**

In `gen_generated` (line ~1594), update the generated `use` line:

Old:
```rust
use lelwel::{TokenType, RuleType, CstBuilder, ParserHooks, Parser, NodeRef, Cst, Span, CstData, err #mark_closed_import};
```

New:
```rust
use lelwel::{TokenType, RuleType, CstBuilder, ParserHooks, Parser, NodeRef, Cst, Span, err #mark_closed_import};
```

- [ ] **Step 3: Update `gen_parser_callbacks` — impl target type**

Change from:
```rust
impl<'a, B: CstBuilder<Token=Token, Rule=Rule>> ParserCallbacks<'a> for Parser<'a, B, ()> {
```

To:
```rust
impl<'a, B: CstBuilder<Token, Rule>> ParserCallbacks<'a> for Parser<'a, B, Token, Rule, ()> {
```

- [ ] **Step 4: Update the `Rules` trait**

Change from specific `CstData` impl to generic `B: CstBuilder<Token, Rule>`:

Old:
```rust
pub trait Rules<'a>: ParserCallbacks<'a> + Sized {
    fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
    // ... rule methods
}

impl<'a, Ctx> Rules<'a> for Parser<'a, CstData<Token, Rule>, Ctx>
where
    Self: ParserCallbacks<'a>,
    Ctx: From<<Self as ParserHooks<'a, Token, Rule>>::Ctx>,
    <Self as ParserHooks<'a, Token, Rule>>::Ctx: From<Ctx>,
{
    fn parse(mut self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
        self.end_of_input = Token::EOF;
        let data = self.parse_with(
            |parser, diags| parser.#start_rule_fn(diags),
            diags,
            Rule::#start_rule_variant,
        );
        Cst::new(self.builder.source(), data)
    }
    // ...
}
```

New — generic over `B`, uses `B::Output<'a>` as return type:
```rust
pub trait Rules<'a>: ParserCallbacks<'a> + Sized {
    type Output;
    fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Self::Output;
    // ... rule methods stay same
}

impl<'a, B: CstBuilder<Token, Rule>, Ctx> Rules<'a> for Parser<'a, B, Token, Rule, Ctx>
where
    Self: ParserCallbacks<'a>,
{
    type Output = B::Output<'a>;

    fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Self::Output {
        self.end_of_input = Token::EOF;
        self.parse_with(
            |parser, diags| parser.#start_rule_fn(diags),
            diags,
            Rule::#start_rule_variant,
        )
    }
    // ... rule methods — same body, no change needed
}
```

The rule methods (`rule_file`, etc.) stay the same — they use `self.open(diags)`, `self.close(m, Rule::Variant, diags)`, etc. which are generic over `B, T, R`.

- [ ] **Step 5: Update `gen_parts_impl`**

Change from:
```rust
fn #parse_fn(mut self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
    self.end_of_input = Token::#eof_variant;
    let data = self.parse_with(|parser, diags| parser.rule_part(diags), diags, Rule::Part);
    Cst::new(self.builder.source(), data)
}
```

To:
```rust
fn #parse_fn(mut self, diags: &mut Vec<Self::Diagnostic>) -> <Self as Rules>::Output {
    self.end_of_input = Token::#eof_variant;
    self.parse_with(|parser, diags| parser.rule_part(diags), diags, Rule::Part)
}
```

- [ ] **Step 6: Update `gen_left_recursive_rule` inner rec function**

The `rec` function type references change from `Parser<'b, Token, Rule, Ctx>` to `Parser<'b, CstData<Token, Rule>, Token, Rule, Ctx>` at minimum. But since the `Rules` trait is now generic, this actually simplifies:

Old:
```rust
let parser_ty = quote! { Parser < 'b , CstData < Token , Rule > , Ctx > };
```

New:
```rust
let parser_ty = quote! { Parser < 'b , CstData < Token , Rule > , Token , Rule , Ctx > };
```

- [ ] **Step 7: Update the `ParserHooks` blanket impl in generated code**

This impl is at line ~1629 in the old code. It's already generic over `B`, just needs the updated bounds:

Old:
```rust
impl<'a, B, Ctx> ParserHooks<'a, Token, Rule> for Parser<'a, B, Ctx>
where
    B: CstBuilder<Token=Token, Rule=Rule>,
    Self: ParserCallbacks<'a>,
```

New:
```rust
impl<'a, B, Ctx, T, R> ParserHooks<'a, Token, Rule> for Parser<'a, B, Token, Rule, Ctx>
where
    B: CstBuilder<Token, Rule>,
    Self: ParserCallbacks<'a>,
```

Wait — `Token` and `Rule` are concrete types in scope (they're generated enums in `generated.rs`). The `ParserHooks` trait is parameterized over `T: TokenType, R: RuleType`, so `impl ParserHooks<'a, Token, Rule> for ...` binds `T = Token` and `R = Rule`. This doesn't need to change — `Token` and `Rule` are the specific enums. The `B` parameter is the only new flexibility. Let me keep this simpler:

```rust
impl<'a, B: CstBuilder<Token, Rule>, Ctx> ParserHooks<'a, Token, Rule> for Parser<'a, B, Token, Rule, Ctx>
where
    Self: ParserCallbacks<'a>,
{
    // body unchanged
}
```

- [ ] **Step 8: Verify compilation (codegen crate only)**

Run: `cargo check -p lelwel-codegen 2>&1`
Expected: compiles (examples will fail until generated.rs is regenerated)

- [ ] **Step 9: Commit**

```bash
git add lelwel-codegen/src/backend/rust.rs
git commit -m "feat(codegen): generic Rules impl, updated Parser type references"
```

---

### Task 7: Add SyntaxKind generation

**Files:**
- Create: `lelwel-codegen/src/backend/cstree.rs`
- Modify: `lelwel-codegen/src/backend/rust.rs`

- [ ] **Step 1: Create `lelwel-codegen/src/backend/cstree.rs`**

This file contains the `SyntaxKind` generation logic, gated by the `cstree` feature.

```rust
use proc_macro2::TokenStream;
use quote::quote;

use crate::frontend::ast::*;
use crate::frontend::sema::*;
use crate::frontend::{Cst, NodeRef};

/// Generate SyntaxKind enum + From impls + compile-time assertions.
/// Called from gen_generated when `cstree` feature is enabled.
pub fn gen_syntax_kind(
    cst: &Cst<'_>,
    sema: &SemanticData<'_>,
    file: File,
    token_count: usize,
    rule_names: &std::collections::BTreeMap<String, bool>,
) -> TokenStream {
    // Collect token variant names (in discriminant order)
    let mut token_variants = Vec::new();
    let mut token_discriminants = Vec::new();
    let mut static_text_attrs = Vec::new();

    // EOF is always discriminant 0
    token_variants.push(quote::format_ident!("EOF"));
    token_discriminants.push(0u32);

    for (i, token) in file.token_decls(cst).enumerate() {
        let name = token.name(cst).unwrap();
        let ident = quote::format_ident!("{}", name.0);
        token_variants.push(ident.clone());
        token_discriminants.push((i + 1) as u32);

        // Check if token has a static text symbol
        if let Some((symbol, _)) = token.symbol(cst) {
            if !symbol.is_empty()
                && !(symbol.starts_with("'<") && symbol.ends_with(">'") && symbol.len() > 4)
            {
                let text = &symbol[1..symbol.len() - 1];
                static_text_attrs.push(quote! {
                    #[static_text(#text)]
                    #ident,
                });
                continue;
            }
        }
        static_text_attrs.push(quote! { #ident, });
    }

    // Error variant — last token variant, emitted separately with explicit discriminant
    let error_discriminant = token_discriminants.last().copied().unwrap_or(0) + 1;
    token_variants.push(quote::format_ident!("Error"));
    token_discriminants.push(error_discriminant);

    // Rule variants — prefixed with "Rule" to avoid collisions with token names
    let rule_token_count = token_variants.len();
    let rule_variants: Vec<_> = rule_names
        .keys()
        .filter(|n| **n != "error")
        .map(|n| {
            let pascal = snake_to_pascal_case(n);
            quote::format_ident!("Rule{}", pascal)
        })
        .collect();
    let rule_error_ident = quote::format_ident!("RuleError");

    // Build From<Token> impl
    let from_token_variants: Vec<_> = file.token_decls(cst).map(|token| {
        let name = token.name(cst).unwrap();
        let ident = quote::format_ident!("{}", name.0);
        quote! { Token::#ident => SyntaxKind::#ident }
    }).collect();

    let from_token_impl = quote! {
        impl From<Token> for SyntaxKind {
            fn from(t: Token) -> Self {
                match t {
                    Token::EOF => SyntaxKind::EOF,
                    #(#from_token_variants,)*
                    Token::Error => SyntaxKind::Error,
                }
            }
        }
    };

    // Build From<Rule> impl — rule variants map to Rule-prefixed SyntaxKind names
    let from_rule_variants: Vec<_> = rule_variants.iter().map(|ident| {
        // Strip the "Rule" prefix to get back the original Rule variant name
        let rule_name = ident.to_string().strip_prefix("Rule").unwrap();
        let rule_ident = quote::format_ident!("{}", rule_name);
        quote! { Rule::#rule_ident => SyntaxKind::#ident }
    }).collect();

    let from_rule_impl = quote! {
        impl From<Rule> for SyntaxKind {
            fn from(r: Rule) -> Self {
                match r {
                    Rule::Error => SyntaxKind::RuleError,
                    #(#from_rule_variants,)*
                }
            }
        }
    };

    // Build compile-time assertions (token variants only — discriminants match)
    let token_assertions: Vec<_> = std::iter::once(quote::format_ident!("EOF"))
        .chain(file.token_decls(cst).map(|token| {
            let name = token.name(cst).unwrap();
            quote::format_ident!("{}", name.0)
        }))
        .chain(std::iter::once(quote::format_ident!("Error")))
        .map(|ident| {
            quote! {
                assert!(Token::#ident as u32 == SyntaxKind::#ident as u32);
            }
        })
        .collect();

    let rule_assertions: Vec<_> = std::iter::once(quote::format_ident!("RuleError"))
        .chain(rule_variants.iter().cloned())
        .map(|ident| {
            // Rule variant in SyntaxKind: RuleError, RuleFile, etc.
            // Rule variant in Rule enum: Error, File, etc.
            let rule_name = ident.to_string();
            let rule_ident = quote::format_ident!("{}", rule_name.strip_prefix("Rule").unwrap());
            quote! {
                assert!(Rule::#rule_ident as u32 == SyntaxKind::#ident as u32);
            }
        })
        .collect();

    // Rule discriminants — Error variant first (to match Rule's Error = TOKEN_COUNT)
    let rule_discriminants: Vec<_> = std::iter::once((quote::format_ident!("RuleError"), rule_token_count as u32))
        .chain(rule_variants.iter().enumerate().map(|(i, ident)| {
            (ident.clone(), (rule_token_count + 1 + i) as u32)
        }))
        .map(|(ident, disc)| {
            quote! { #ident = #disc }
        })
        .collect();

quote! {
        #[repr(u32)]
        #[derive(cstree::Syntax)]
pub enum SyntaxKind {
            EOF = 0,
            #(#static_text_attrs)*
            Error = #error_discriminant,
            #(#rule_discriminants,)*
        }

        #from_token_impl
        #from_rule_impl

        const _: () = {
            #(#token_assertions)*
            #(#rule_assertions)*
        };
    }
}

fn snake_to_pascal_case(name: &str) -> String {
    let mut res = String::new();
    let mut upper = true;
    for c in name.chars() {
        if upper {
            res.push(c.to_ascii_uppercase());
            upper = false;
        } else if c == '_' {
            upper = true;
        } else {
            res.push(c);
        }
    }
    res
}
```

- [ ] **Step 2: Hook into `gen_generated`**

In `rust.rs`, add a call to `gen_syntax_kind` at the end of `gen_generated`, gated by `#[cfg(feature = "cstree")]`:

```rust
#[cfg(feature = "cstree")]
{
    use crate::backend::cstree::gen_syntax_kind;
    let syntax_kind = gen_syntax_kind(cst, sema, file, token_count, &rule_names);
    // Append syntax_kind to the generated token stream
}
```

- [ ] **Step 3: Add new module to `lelwel-codegen/src/backend/mod.rs`**

```rust
pub mod rust;
#[cfg(feature = "cstree")]
pub mod cstree;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p lelwel-codegen --features cstree 2>&1`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add lelwel-codegen/src/backend/
git commit -m "feat(codegen): generate SyntaxKind enum with derive(Syntax) and From impls"
```

---

### Task 8: Add repr(u32) + explicit discriminants to Token and Rule

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs`

When the `cstree` feature is enabled, `gen_lexer` should add `#[repr(u32)]` and explicit discriminants to `Token`, and `gen_generated` should do the same for `Rule`.

- [ ] **Step 1: Update `gen_lexer` — add repr(u32) + discriminants when cstree feature enabled**

In `gen_lexer()`, change `#[derive(Logos, Debug, PartialEq, Eq, Copy, Clone)]` to conditionally add `#[repr(u32)]`:

```rust
// At the top of gen_lexer
let repr_attr = if cfg!(feature = "cstree") {
    quote! { #[repr(u32)] }
} else {
    TokenStream::new()
};
```

And add explicit discriminants to each token variant when cstree is enabled:

```rust
// When building token_variants:
let discriminant = if cfg!(feature = "cstree") {
    // Compute the discriminant based on order
    quote! { = #disc_val }
} else {
    TokenStream::new()
};
```

This requires tracking the discriminant value per variant: EOF=0, then each token variant = 1, 2, 3... Error = last.

- [ ] **Step 2: Update `gen_generated` — add repr(u32) + discriminants to Rule when cstree enabled**

When building the `Rule` enum output, determine the starting discriminant:
- Without cstree: no repr, no explicit discriminants (same as today)
- With cstree: `#[repr(u32)]`, `Error = TOKEN_COUNT`, then subsequent rule variants = `TOKEN_COUNT + 1`, `TOKEN_COUNT + 2`, ...

The `TOKEN_COUNT` here must match what `SyntaxKind` uses (total number of Token variants including EOF and Error). In `gen_syntax_kind`, this is `rule_token_count` (= token_variants.len() after EOF + user tokens + Error).

```rust
#[repr(u32)]
pub enum Rule {
    Error = TOKEN_COUNT,
    #(#other_rule_variants_with_discriminants,)*
}
```

- [ ] **Step 3: Add cstree feature to lelwel-codegen Cargo.toml**

```toml
[features]
cstree = []
```

This is an empty feature — no extra dependencies needed for the codegen crate itself. The `cstree` crate dependency is only needed by the runtime (`lelwel` crate) which already has it as optional.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p lelwel-codegen --features cstree 2>&1`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add lelwel-codegen/src/backend/rust.rs lelwel-codegen/Cargo.toml
git commit -m "feat(codegen): repr(u32) + explicit discriminants for Token, Rule, SyntaxKind"
```

---

### Task 9: Verify end-to-end compilation and tests

- [ ] **Step 1: Check all examples compile**

Run: `cargo check --workspace 2>&1`
Expected: all crates compile

- [ ] **Step 2: Run all tests**

Run: `cargo test --workspace 2>&1`
Expected: all tests pass (existing golden tests unchanged)

- [ ] **Step 3: Run lelwel tests with cstree feature**

Run: `cargo test -p lelwel --features cstree 2>&1`
Expected: 4 cstree unit tests pass

- [ ] **Step 4: Commit any fixups**

```bash
git commit -am "fix: adapt examples to updated Parser/Rules generics"
```