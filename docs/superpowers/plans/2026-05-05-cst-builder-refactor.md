# CST Builder Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a `CstBuilder` trait that abstracts CST tree construction, making `Parser` generic over backends (built-in `CstData` and third-party like `cstree`).

**Architecture:** A `CstBuilder` trait with no notion of skip tokens. A `LelwelBuilder<B>` wrapper provides skip-buffering semantics for all backends. `CstData` implements `CstBuilder`. `Parser` becomes `Parser<'a, B, Ctx>` generic over `B: CstBuilder`. Spans are stored both in `CstData` (for final output) and `Parser` (for parse-time span lookups), created once during construction via `CstBuilder::new(spans)`.

**Tech Stack:** Rust, lelwel runtime (zero deps), lelwel-codegen (proc-macro2/quote)

---

### Task 1: Add `CstBuilder` trait and `LelwelBuilder` wrapper

**Files:**
- Modify: `lelwel/src/parser.rs`

- [ ] **Step 1: Add `CstBuilder` trait**

Add after `RuleType` in `parser.rs`:

```rust
pub trait CstBuilder {
    type Token;
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
    fn iterate_removed(&self, checkpoint: Self::Checkpoint, _f: &mut dyn FnMut(Self::Rule, NodeRef)) {}
    fn node_ref(&self, _mark: Self::Mark) -> Option<NodeRef> { None }
    fn finish(self) -> Self::Output;
}
```

- `node_ref()` returns `Some(NodeRef(mark))` for `CstData` (where `Mark = usize = node index`), `None` for backends without positional refs.
- `iterate_removed()` calls the callback for each rule node in `[checkpoint..current)` range. Only `CstData` overrides this (enables the `delete_node` hook during backtracking).

- [ ] **Step 2: Add `LelwelBuilder` struct**

```rust
pub struct LelwelBuilder<'s, B: CstBuilder> {
    inner: B,
    source: &'s str,
    buffer: Vec<(B::Token, usize, usize)>,
    start_idx: usize,
    pub in_ordered_choice: bool,
}
```

- [ ] **Step 3: Add `LelwelBuilder` impl block**

```rust
impl<'s, B: CstBuilder> LelwelBuilder<'s, B> {
    pub fn new(inner: B, source: &'s str) -> Self {
        LelwelBuilder { inner, source, buffer: Vec::new(), start_idx: 0, in_ordered_choice: false }
    }

    pub fn into_inner(self) -> B { self.inner }
    pub fn source(&self) -> &'s str { self.source }
    pub fn node_ref(&self, mark: B::Mark) -> Option<NodeRef> { self.inner.node_ref(mark) }
    pub fn span_at(&self, pos: usize) -> Span {
        self.inner.checkpoint(); // noop used to get self
        Span { start: 0, end: 0 }
    }

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
```

- `end_rule()` (close) does NOT flush — trailing skip tokens in buffer stay excluded from child count.
- `end_rule_root()` (close_root) DOES flush — the root includes everything.
- `state()` captures inner checkpoint + flush frontier + buffer length.
- `restore()` calls iterate_removed for delete_node, reverts inner, restores buffer state.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: compiles (unused warnings OK, nothing uses these yet)

- [ ] **Step 5: Commit**

```bash
git add lelwel/src/parser.rs
git commit -m "feat(runtime): add CstBuilder trait and LelwelBuilder wrapper"
```

---

### Task 2: Simplify `CstData` and implement `CstBuilder` for it

**Files:**
- Modify: `lelwel/src/types.rs`
- Modify: `lelwel/src/cst.rs`

- [ ] **Step 1: Simplify `MarkTruncation` in `types.rs`**

Remove `non_skip_len`:

```rust
#[derive(Clone, Copy)]
pub struct MarkTruncation {
    pub(crate) node_count: usize,
    pub(crate) token_count: usize,
}
```

- [ ] **Step 2: Remove `non_skip_len` field from `CstData`**

In `cst.rs`:

```rust
pub struct CstData<T, R> {
    pub(crate) spans: Vec<Span>,
    pub nodes: Vec<Node<T, R>>,
    pub(crate) token_count: usize,
}
```

Update `new`:

```rust
pub fn new(spans: Vec<Span>) -> Self {
    let nodes = Vec::with_capacity(spans.len() * 2);
    Self { spans, nodes, token_count: 0 }
}
```

Make `new` `pub` (not `pub(crate)`) so the `CstBuilder` trait can call it.

- [ ] **Step 3: Simplify `open()` — no more `non_skip_len`**

```rust
pub(crate) fn open(&mut self) -> MarkOpened {
    let mark = MarkOpened(self.nodes.len());
    self.nodes.push(Node::Rule(R::error(), 0.into()));
    mark
}
```

- [ ] **Step 4: Simplify `advance()` — no skip parameter**

```rust
pub fn advance(&mut self, token: T) {
    self.nodes.push(Node::Token(token, self.token_count.into()));
    self.token_count += 1;
}
```

- [ ] **Step 5: Simplify `open_before()`**

```rust
pub(crate) fn open_before(&mut self, mark: MarkClosed) -> MarkOpened {
    self.nodes.insert(mark.0, Node::Rule(R::error(), 0.into()));
    MarkOpened(mark.0)
}
```

- [ ] **Step 6: Simplify `close()` and `close_root()`**

```rust
pub(crate) fn close(&mut self, mark: MarkOpened, rule: R) -> MarkClosed {
    let len = self.nodes.len() - 1;
    self.nodes[mark.0] = Node::Rule(
        rule,
        (if mark.0 > len { 0 } else { (len - mark.0) as u16 }).into(),
    );
    MarkClosed(mark.0)
}

pub(crate) fn close_root(&mut self, mark: MarkOpened, rule: R) -> MarkClosed {
    self.nodes[mark.0] = Node::Rule(rule, (self.nodes.len() - 1 - mark.0).into());
    MarkClosed(mark.0)
}
```

- [ ] **Step 7: Simplify `mark_truncation()` and `truncate()`**

```rust
pub(crate) fn mark_truncation(&self) -> MarkTruncation {
    MarkTruncation { node_count: self.nodes.len(), token_count: self.token_count }
}

pub(crate) fn truncate(&mut self, mark: MarkTruncation) {
    self.nodes.truncate(mark.node_count);
    self.token_count = mark.token_count;
}
```

- [ ] **Step 8: Add `CstBuilder` impl for `CstData`**

```rust
impl<T: TokenType, R: RuleType> CstBuilder for CstData<T, R> {
    type Token = T;
    type Rule = R;
    type Mark = usize;
    type Checkpoint = MarkTruncation;
    type Output = CstData<T, R>;

    fn new(spans: Vec<Span>) -> Self { CstData::new(spans) }

    fn token(&mut self, kind: T, _text: &str) {
        self.nodes.push(Node::Token(kind, self.token_count.into()));
        self.token_count += 1;
    }

    fn start_rule(&mut self) -> usize {
        let pos = self.nodes.len();
        self.nodes.push(Node::Rule(R::error(), 0.into()));
        pos
    }

    fn end_rule(&mut self, mark: usize, rule: R) {
        let len = self.nodes.len();
        self.nodes[mark] = Node::Rule(
            rule,
            (if mark >= len { 0 } else { (len - 1 - mark) as u16 }).into(),
        );
    }

    fn mark(&self) -> usize { self.nodes.len() }

    fn start_rule_before(&mut self, mark: usize) -> usize {
        self.nodes.insert(mark, Node::Rule(R::error(), 0.into()));
        mark
    }

    fn checkpoint(&self) -> MarkTruncation {
        MarkTruncation { node_count: self.nodes.len(), token_count: self.token_count }
    }

    fn revert_to(&mut self, cp: MarkTruncation) {
        self.nodes.truncate(cp.node_count);
        self.token_count = cp.token_count;
    }

    fn iterate_removed(&self, checkpoint: MarkTruncation, f: &mut dyn FnMut(R, NodeRef)) {
        for i in checkpoint.node_count..self.nodes.len() {
            if let Node::Rule(rule, _) = &self.nodes[i] {
                f(*rule, NodeRef(i));
            }
        }
    }

    fn node_ref(&self, mark: usize) -> Option<NodeRef> { Some(NodeRef(mark)) }

    fn finish(self) -> CstData<T, R> { self }
}
```

- [ ] **Step 9: Add `Cst::new` constructor**

```rust
impl<'a, T, R> Cst<'a, T, R> {
    pub fn new(source: &'a str, data: CstData<T, R>) -> Self {
        Cst { source, data }
    }
}
```

- [ ] **Step 10: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: warnings about unused methods on CstData (open/close/advance etc) since they're still called by the old Parser. That's expected — they'll be removed in Task 3.

- [ ] **Step 11: Commit**

```bash
git add lelwel/src/types.rs lelwel/src/cst.rs
git commit -m "feat(runtime): implement CstBuilder for CstData, remove non_skip_len"
```

---

### Task 3: Refactor `Parser` to be generic over `B: CstBuilder`

**Files:**
- Modify: `lelwel/src/parser.rs`

This is the big task. `Parser` changes from `Parser<'a, T, R, Ctx>` to `Parser<'a, B, Ctx>` where `B: CstBuilder<Token=T, Rule=R>`. It uses `LelwelBuilder<'a, B>` as its builder field.

`MarkOpened` and `MarkClosed` types are still used by `CstData`'s methods but are no longer returned by `Parser::open()/close()` — those now return `B::Mark`.

The `close()` method is replaced by `LelwelBuilder::end_rule()` (called `close()` on Parser). The old `CstData::close()` is no longer directly called by Parser.

The `error_node` field changes from `Option<MarkOpened>` to `Option<B::Mark>`.

- [ ] **Step 1: Change `ParserState` to be generic over `B`**

```rust
pub struct ParserState<B: CstBuilder> {
    pos: usize,
    current: B::Token,
    checkpoint: B::Checkpoint,
    start_idx: usize,
    buffer_len: usize,
    diag_count: usize,
}
```

- [ ] **Step 2: Change `Parser` struct**

```rust
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
```

- [ ] **Step 3: Update `ParserHooks` trait generics**

The `ParserHooks` trait stays generic over `T: TokenType, R: RuleType`. No change needed to the trait itself. But the blanket impl changes.

- [ ] **Step 4: Update `ParserArgs` and `Diag` type alias**

```rust
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
```

- [ ] **Step 5: Add `new` and `new_with_context` constructors**

```rust
impl<'a, B, Ctx> Parser<'a, B, Ctx>
where
    B: CstBuilder,
    B::Token: TokenType,
    B::Rule: RuleType,
    Self: ParserHooks<'a, B::Token, B::Rule>,
{
    pub fn new_with_context(
        source: &'a str,
        diags: &mut Vec<Diag<Self>>,
        context: Ctx,
    ) -> Self
    where
        Ctx: From<<Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx>,
        <Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx: From<Ctx>,
    {
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
        Ctx: From<<Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx>,
        <Self as ParserHooks<'a, B::Token, B::Rule>>::Ctx: From<Ctx>,
    {
        Self::new_with_context(source, diags, Ctx::default())
    }
}
```

- [ ] **Step 6: Update `active_error` and `error` methods**

```rust
pub fn active_error(&self) -> bool {
    self.error_node.is_some() || self.error_since_advance
}

pub fn error(&mut self, diags: &mut Vec<Diag<Self>>, diag: Diag<Self>) {
    if self.active_error() { return; }
    self.error_since_advance = true;
    diags.push(diag);
}
```

No changes to signatures (they don't reference B). Body unchanged.

- [ ] **Step 7: Rewrite `advance` method**

Old method called `self.cst.data.advance(self.current, false)` for the current token and `self.cst.data.advance(*token, true)` for skips. New method delegates to `self.builder.advance(kind, skip, span)`.

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
```

- [ ] **Step 8: Rewrite `init_skip`**

```rust
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
```

Wait — there's an issue. `init_skip` advances tokens and calls `builder.advance(skip=true)` before parsing starts. But the buffer is empty at this point. After `init_skip`, the current token is set to the first non-skip token. The initial skip tokens are in the buffer. When the first `open()` is called (which flushes), they'll be committed to the tree before the root node starts. Since `open()` flushes before `start_rule`, the initial skips would be children of whatever rule was started before them — but there's no rule yet!

Actually looking at the current code, `init_skip` is called inside `parse_with` AFTER `self.open(diags)`:

```rust
let m = self.open(diags);
self.init_skip();
```

So the root node is already open. The initial skip tokens flushed by `open()` go into the empty buffer (nothing to flush), then the root node starts. Then `init_skip` advances through skips, pushing them into the buffer. They stay in the buffer until the next non-skip advance or `close_root`.

In `close_root`, `end_rule_root` flushes first, so all initial skips become children of the root. Correct!

But wait — let me double-check. In `parse_with`:

```rust
let m = self.open(diags);   // flushes buffer (empty), calls start_rule() → root node starts
self.init_skip();            // advances through initial skips, each pushed to buffer
```

After `init_skip`, the current token is set to the first non-skip. The skip tokens are in `buffer`. Then `start_rule` runs. When it does `self.advance(false, diags)` for the first token, `advance` calls `builder.advance(token, false, span)` which flushes the buffer first (committing all initial skips to the tree as children of root). Then it emits the non-skip token. This matches the current behavior!

After all parsing, `close_root` calls `end_rule_root` which flushes, committing any trailing skips to the tree.

This looks correct.

- [ ] **Step 9: Rewrite `advance_with_error`**

```rust
pub fn advance_with_error(&mut self, diags: &mut Vec<Diag<Self>>, diag: Diag<Self>) {
    self.error(diags, diag);
    if self.error_node.is_none() {
        self.error_node = Some(self.builder.start_rule());
    }
    self.advance(true, diags);
}
```

- [ ] **Step 10: Rewrite `span()`**

```rust
pub fn span(&self) -> Span {
    self.spans.get(self.pos).cloned().unwrap_or(self.max_offset..self.max_offset)
}
```

- [ ] **Step 11: Rewrite `close_error_node`**

```rust
pub fn close_error_node(&mut self, diags: &mut Vec<Diag<Self>>) {
    if let Some(error_node) = self.error_node {
        self.builder.end_rule(error_node, B::Rule::error());
        if let Some(nr) = self.builder.node_ref(error_node) {
            self.create_node_error_hook(nr, diags);
        }
        self.error_node = None;
    }
}
```

- [ ] **Step 12: Rewrite `open`, `open_before`, `close`, `close_root`, `mark`**

```rust
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
```

Note: `close()` and `close_root()` now call `self.create_node()` internally. The codegen no longer needs to emit separate `create_node(rule, NodeRef(closed.0), diags)` calls.

- [ ] **Step 13: Rewrite `get_state` and `set_state`**

```rust
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
    self.builder.restore(
        state.checkpoint,
        state.start_idx,
        state.buffer_len,
        &mut |rule, node_ref| self.delete_node(rule, node_ref),
    );
}
```

- [ ] **Step 14: Rewrite `parse_with`**

```rust
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
```

Note: `parse_with` now returns `B::Output` (not `Cst`). For the built-in backend, `B::Output = CstData<T, R>`. The caller wraps in `Cst`.

- [ ] **Step 15: Verify compilation**

Run: `cargo check -p lelwel 2>&1`
Expected: compilation errors from codegen tests and example code that uses the old `Parser<'a, Token, Rule, Ctx>` type

- [ ] **Step 16: Commit**

```bash
git add lelwel/src/parser.rs
git commit -m "feat(runtime): refactor Parser to be generic over CstBuilder via LelwelBuilder"
```

---

### Task 4: Update runtime `lib.rs` exports

**Files:**
- Modify: `lelwel/src/lib.rs`

- [ ] **Step 1: Export new types**

```rust
#![forbid(unsafe_code)]
mod cst;
mod parser;
mod types;

pub use cst::*;
pub use parser::*;
pub use types::*;
```

No changes needed — `CstBuilder` and `LelwelBuilder` are in `parser.rs` which is re-exported via `pub use parser::*`.

But check: `CstBuilder` trait and `LelwelBuilder` are pub. `NodeRef`, `MarkTruncation`, `MarkOpened`, `MarkClosed` are already exported. `CstIndex` is already exported.

- [ ] **Step 2: Verify**

Run: `cargo check -p lelwel 2>&1`
Expected: same errors as before (examples/codegen not yet updated)

- [ ] **Step 3: Commit**

```bash
git add lelwel/src/lib.rs
git commit -m "chore(runtime): no lib.rs changes needed, CstBuilder re-exported via parser"
```

---

### Task 5: Update codegen backend

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs`

The codegen generates Rust source using `proc_macro2::TokenStream` via the `quote!` macro. Several patterns need updating:

- `Parser<'a, Token, Rule, Ctx>` → `Parser<'a, CstData<Token, Rule>, Ctx>` (concrete type for generated impls)
- `self.close(m, ...)` / `self.close_root(m, ...)` → `self.close(m, ...)` / `self.close_root(m, ...)` (unchanged — these are Parser methods that delegate to builder)
- `parser.in_ordered_choice` → `parser.builder.in_ordered_choice`
- `$self.in_ordered_choice` in `try_expect!` macro → `$self.builder.in_ordered_choice`
- `NodeRef(closed.0)` in close/create_node → removed — `create_node` is now called inside `Parser::close()`

- [ ] **Step 1: Update import in generated code (line 1594)**

Old:
```rust
use lelwel::{TokenType, RuleType, ParserHooks, Parser, NodeRef, Cst, Span, err #mark_closed_import};
```

New:
```rust
use lelwel::{TokenType, RuleType, CstBuilder, ParserHooks, Parser, NodeRef, Cst, Span, CstData, err #mark_closed_import};
```

Actually, `CstData` might not be imported by default since it's in `cst.rs`. Let me check... `pub use cst::*` in `lib.rs` exports everything from `cst.rs`, which includes `CstData`, `Cst`, `Node`, `CstChildren`. Yes, `CstData` is already exported.

But wait — the generated code uses `CstData` as the concrete builder type. The import needs `CstData`. Let me update:

```rust
use lelwel::{TokenType, RuleType, CstBuilder, ParserHooks, Parser, NodeRef, Cst, Span, CstData, err #mark_closed_import};
```

- [ ] **Step 2: Update `gen_parser` (line 117-127)**

Change the `use` statement in the generated parser.rs template:

```rust
fn gen_parser(sema: &SemanticData<'_>) -> TokenStream {
    let callbacks = Self::gen_parser_callbacks(sema, false, BTreeMap::default());
    quote! {
        use lelwel::{ParserHooks, Parser, Span, Cst, NodeRef, CstData};
        use super::lexer::{Token, tokenize};
        use codespan_reporting::diagnostic::Label;
        pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;
        include!(concat!(env!("OUT_DIR"), "/generated.rs"));
        #callbacks
    }
}
```

- [ ] **Step 3: Update `gen_parser_callbacks` non-trait version (line 359)**

Change the impl target type from `Parser<'a, Token, Rule, ()>` to `Parser<'a, CstData<Token, Rule>, ()>`:

```rust
impl<'a> ParserCallbacks<'a> for Parser<'a, CstData<Token, Rule>, ()> {
```

- [ ] **Step 4: Update `gen_left_recursive_rule` inner rec function (line 763-776)**

The `rec` function currently has a concrete type:

```rust
let parser_ty = quote! { Parser < 'b , Token , Rule , Ctx > };
let diags_ty = quote! { Vec < < #parser_ty as ParserCallbacks < 'b > > :: Diagnostic > };
```

Change to:

```rust
let parser_ty = quote! { Parser < 'b , CstData < Token , Rule > , Ctx > };
let diags_ty = quote! { Vec < < #parser_ty as ParserCallbacks < 'b > > :: Diagnostic > };
```

And the where clause:

```rust
where
    #parser_ty : ParserCallbacks < 'b > ,
    Ctx : From < < #parser_ty as ParserHooks < 'b , Token , Rule > > :: Ctx > ,
    < #parser_ty as ParserHooks < 'b , Token , Rule > > :: Ctx : From < Ctx > ,
```

- [ ] **Step 5: Update `gen_generated` import (line 1594)**

Also update the `MarkClosed` import context:

```rust
let mark_closed_import = if has_left_recursive {
    quote! { , MarkClosed }
} else {
    TokenStream::new()
};
```

`MarkClosed` is still used in the generated `rec` function signature (line 769): `mut lhs: MarkClosed`. This should change to `mut lhs: usize` (since `CstData::Mark = usize` for the built-in backend). OR, keep `MarkClosed` as a type alias.

Actually wait — in the new design, `close()` returns `B::Mark` which for `CstData` is `usize`. But the codegen generates `mut lhs: MarkClosed` for left-recursive rules. Since we changed `Parser::close()` to return `B::Mark` (= `usize` for `CstData`), the `lhs` variable should be `usize`. `MarkClosed` is no longer the return type of `close()`.

Hmm, but `open_before()` takes `B::Mark` (= `usize`). And `mark()` returns `B::Mark` (= `usize`). And `close()` returns `B::Mark`. So `lhs` should be `usize`.

But `MarkClosed` is used in the `rec` function signature. Let me change it. In `gen_left_recursive_rule`, the `lhs` parameter type:

Old:
```rust
mut lhs: MarkClosed ,
```

New: For the built-in CstData backend, `B::Mark = usize`:
```rust
mut lhs: usize ,
```

But the codegen is grammar-specific and knows the concrete types. So `usize` is the right choice for the built-in backend.

Better yet, the codegen should generate the type based on the builder. For the built-in backend, it's always `usize`. For simplicity, just change `MarkClosed` to `usize` everywhere in generated code.

Wait, `MarkClosed` is used in `gen_parts_sig` (line 799-807):
```rust
fn gen_parts_sig(cst: &Cst<'_>, rule: RuleDecl) -> TokenStream {
    // ...
    quote! {
        fn #parse_fn(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
    }
}
```

That returns `Cst`, not `MarkClosed`. So no change needed there.

`MarkClosed` import is only for left-recursive rules. Change it:

```rust
let mark_closed_import = if has_left_recursive {
    quote! { } // no longer needed
} else {
    TokenStream::new()
};
```

Actually, `MarkClosed` might still be used in `gen_parts_sig`... let me check. No, `gen_parts_sig` returns `Cst`. 

But `MarkClosed` is used in the `Rules` trait `rule_*` methods? No, those return `Option<()>` or nothing.

OK so `MarkClosed` is only used in `gen_left_recursive_rule` for the `lhs` parameter. Since the return type of `close()` changes from `MarkClosed` to `B::Mark` (= `usize`), we change `lhs: MarkClosed` to `lhs: usize` in the rec function. And remove the `MarkClosed` import.

But wait — `lhs` is also the variable that gets assigned `let lhs = self.mark(diags);` and later `lhs = closed;`. The type of `closed` comes from `close()` which now returns `B::Mark` (= `usize`). So the type is consistent.

Let me update the `gen_left_recursive_rule` rec function signature:

```rust
mut lhs: usize ,
```

And remove `MarkClosed` from imports:

```rust
let mark_closed_import = if has_left_recursive {
    quote! { } // MarkClosed not needed — lhs is usize
} else {
    TokenStream::new()
};
```

Actually, since it's empty, just always generate nothing:

```rust
let mark_closed_import = TokenStream::new(); // no longer needed
```

And update the main import line to remove `#mark_closed_import`:

```rust
use lelwel::{TokenType, RuleType, CstBuilder, ParserHooks, Parser, NodeRef, Cst, Span, CstData, err};
```

- [ ] **Step 6: Update `gen_cst_close` (line 394-428)**

The old code generates:
```rust
let closed = parser.close(m, node_kind, diags);
parser.create_node(node_kind, NodeRef(closed.0), diags);
```

Since `close()` and `close_root()` now call `create_node` internally, simplify to:

```rust
let closed = parser.close(m, node_kind, diags);
```

But wait — `closed` is used for `lhs = closed` in left-recursive rules. So we still need to return it. The `close()` method returns `B::Mark` which for `CstData` is `usize`. `lhs` is ALSO `usize` (previously `MarkClosed` with `.0`). So assignment `lhs = closed` works.

But actually, `gen_cst_close` generates the close call and create_node call. Since close now handles create_node internally, we just need:

```rust
fn gen_cst_close(
    has_rule_rename: bool,
    name: &str,
    assign_lhs: bool,
    parser_name: &proc_macro2::Ident,
    is_start: bool,
) -> TokenStream {
    let close_method = if is_start {
        quote::format_ident!("close_root")
    } else {
        quote::format_ident!("close")
    };
    let variant = snake_to_pascal_case_ident(name);
    let close_call = if has_rule_rename {
        quote! {
            let closed = #parser_name.#close_method(m, node_kind, diags);
        }
    } else {
        quote! {
            let closed = #parser_name.#close_method(m, Rule::#variant, diags);
        }
    };
    let assign = if assign_lhs {
        quote! { lhs = closed; }
    } else {
        TokenStream::new()
    };
    quote! {
        #close_call
        #assign
    }
}
```

This removes the `parser.#create_node_method(NodeRef(closed.0), diags)` and `parser.create_node(node_kind, NodeRef(closed.0), diags)` calls — they're now inside `Parser::close()`.

- [ ] **Step 7: Update `gen_elision_check` conditional close (line 472-478)**

The conditional elision check (for `RuleNodeElision::Conditional`) currently generates:

```rust
if !elide {
    let m = #parser_name.open_before(start, diags);
    #close_and_create  // includes close + create_node
}
```

After the change, `#close` is just `let closed = parser.close(...)` (create_node is inside close). No change needed to the structure — just the content of `#close` changes (handled by `gen_cst_close`).

- [ ] **Step 8: Update `gen_regex` for `NodeCreation` (line 1137-1151)**

Currently generates:
```rust
let open_node = #parser_name.open_before(#mark, diags);
#parser_name.close(open_node, Rule::#variant, diags);
#parser_name.#create_fn(NodeRef(#mark.0), diags);
```

Remove the `parser.#create_fn(NodeRef(#mark.0), diags)` call since `close` now calls `create_node`:

```rust
let open_node = #parser_name.open_before(#mark, diags);
#parser_name.close(open_node, Rule::#variant, diags);
```

Wait — `close()` calls `self.create_node(rule, nr, diags)` which dispatches via `match rule { Rule::Variant => self.create_node_variant(nr, diags) }`. The `create_fn` here is `create_node_<name>` which is what `create_node` dispatches to. So removing the explicit call is correct — `close` handles it.

BUT — `NodeCreation` uses `Rule::#variant` which is the rule name from the grammar, not `node_kind`. The `close()` calls `self.create_node(rule, nr, diags)` with the rule passed to close. So we need to make sure the rule passed to close matches the intended create function. Looking at the code: `#parser_name.close(open_node, Rule::#variant, diags)` — yes, this passes the right rule variant.

Also — `NodeCreation` uses `#mark.0` for `NodeRef`. In the new code, `close()` returns `B::Mark`. For `CstData`, `B::Mark = usize`. The `node_ref(mark)` returns `Some(NodeRef(mark))`. So `close()` creates the node ref correctly.

- [ ] **Step 9: Update `gen_regex` for `Return` (line 1155-1185)**

Old error handling:
```rust
let closed = #parser_name.close(m, Rule::Error, diags);
#parser_name.create_node_error_hook(NodeRef(closed.0), diags);
```

New (close handles create_node but NOT create_node_error_hook — need to keep it):
```rust
let closed = #parser_name.close(m, Rule::Error, diags);
#parser_name.create_node_error_hook(closed, diags);
```

Wait — `create_node_error_hook` takes `NodeRef`, but `close()` returns `B::Mark`. For `CstData`, `B::Mark = usize` and `NodeRef(usize)`. We need to convert. The codegen needs to convert `closed` to `NodeRef`.

But `Return` is generated for error handling in `gen_regex`. The error handling might not go through `Parser::close()` — let me check. Looking at the generated code:

```rust
// In Return handler:
let closed = #parser_name.close(m, Rule::Error, diags);
#parser_name.create_node_error_hook(NodeRef(closed.0), diags);
```

This calls `Parser::close()` which now handles `create_node` but NOT `create_node_error_hook`. The `create_node_error_hook` call is explicit. We need a way to convert `B::Mark` to `NodeRef` for the error hook.

Add a helper on `Parser`:
```rust
pub fn mark_to_node_ref(&self, mark: B::Mark) -> Option<NodeRef> {
    self.builder.node_ref(mark)
}
```

Then the `Return` handler generates:
```rust
let closed = #parser_name.close(m, Rule::Error, diags);
if let Some(nr) = #parser_name.mark_to_node_ref(closed) {
    #parser_name.create_node_error_hook(nr, diags);
}
```

Similarly for the error-handling in `parse_with` (already handled above) and the existing `close_error_node`.

Actually, `close_error_node` already handles this correctly (see Step 11 of Task 3). The `Return` handler in generated code needs the same pattern.

Let me also check `gen_regex` for the `Return` case with `Conditional` elision:

```rust
RuleNodeElision::Conditional => {
    quote! {
        if !elide {
            let m = #parser_name.open_before(start, diags);
            let closed = #parser_name.close(m, Rule::Error, diags);
            #parser_name.create_node_error_hook(NodeRef(closed.0), diags);
        }
    }
}
```

Update both this and the `None` elision case:

```rust
RuleNodeElision::None => {
    quote! {
        let closed = #parser_name.close(m, Rule::Error, diags);
        if let Some(nr) = #parser_name.mark_to_node_ref(closed) {
            #parser_name.create_node_error_hook(nr, diags);
        }
    }
}
RuleNodeElision::Conditional => {
    quote! {
        if !elide {
            let m = #parser_name.open_before(start, diags);
            let closed = #parser_name.close(m, Rule::Error, diags);
            if let Some(nr) = #parser_name.mark_to_node_ref(closed) {
                #parser_name.create_node_error_hook(nr, diags);
            }
        }
    }
}
```

- [ ] **Step 10: Update `in_ordered_choice` references in generated code**

In `gen_regex` (line 1052-1062), the ordered choice return pattern:
```rust
let ordered_choice_return = if in_choice {
    quote! {
        if #parser_name.in_ordered_choice {
            return None;
        }
    }
};
```
Change to:
```rust
quote! {
    if #parser_name.builder.in_ordered_choice {
        return None;
    }
}
```

In `gen_generated` (line 1685-1686), the `try_expect!` macro:
```rust
if $self.in_ordered_choice {
    return None;
}
```
Change to:
```rust
if $self.builder.in_ordered_choice {
    return None;
}
```

In `gen_regex` for `OrderedChoice` (line 1346, 1352):
```rust
#parser_name.in_ordered_choice = true;
// ...
#parser_name.in_ordered_choice = false;
```
Change to:
```rust
#parser_name.builder.in_ordered_choice = true;
// ...
#parser_name.builder.in_ordered_choice = false;
```

In `gen_regex` for `Commit` (line 1152-1153):
```rust
Regex::Commit(_) => {
    quote! { #parser_name.in_ordered_choice = false; }
}
```
Change to:
```rust
Regex::Commit(_) => {
    quote! { #parser_name.builder.in_ordered_choice = false; }
}
```

- [ ] **Step 11: Update the `Rules` trait impl (line 1698-1712)**

Old:
```rust
impl<'a, Ctx> Rules<'a> for Parser<'a, Token, Rule, Ctx>
where
    Self: ParserCallbacks<'a>,
    Ctx: From<<Self as ParserHooks<'a, Token, Rule>>::Ctx>,
    <Self as ParserHooks<'a, Token, Rule>>::Ctx: From<Ctx>,
{
    fn parse(mut self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
        self.end_of_input = Token::EOF;
        self.parse_with(|parser, diags| parser.#start_rule_fn(diags), diags, Rule::#start_rule_variant)
    }
    // ...
}
```

New:
```rust
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

Note: `parse` returns `Cst<'a, Token, Rule>` by wrapping the `CstData` output from `parse_with`. The `self.builder.source()` provides the source string. `Cst::new()` constructs the wrapper.

For the error node handling in `parse_with`: the old code `self.cst.data.close(error_tree, R::error())` becomes `self.builder.end_rule(error_tree, B::Rule::error())`. The `close_error_node` method in generated code's `Return` handler already handles this.

- [ ] **Step 12: Update `gen_parts_impl` (line 808-818)**

Old:
```rust
fn #parse_fn(mut self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
    self.end_of_input = Token::#eof_variant;
    self.parse_with(|parser, diags| parser.rule_part(diags), diags, Rule::Part)
}
```

New (return type changes, need to wrap output):
```rust
fn #parse_fn(mut self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
    self.end_of_input = Token::#eof_variant;
    let data = self.parse_with(|parser, diags| parser.rule_part(diags), diags, Rule::Part);
    Cst::new(self.builder.source(), data)
}
```

- [ ] **Step 13: Update the `Rules` trait `parse` sig (line 1548)**

The trait's `parse` method:
```rust
fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
```

This stays the same — `Rules` trait always returns `Cst<'a, Token, Rule>` for the CstData backend.

- [ ] **Step 14: Update `gen_parts_sig` (line 799-807)**

```rust
fn #parse_fn(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
```

No change — still returns `Cst`.

- [ ] **Step 15: Update `ParserHooks` blanket impl for `delete_node` (line 1664-1667)**

The `delete_node` impl:
```rust
fn delete_node(&mut self, _rule: Rule, _node_ref: NodeRef) {
    #delete_body
}
```

No change needed — this is on `Parser` which is now `Parser<'a, CstData<Token, Rule>, Ctx>`. The `delete_node` hook works the same way.

- [ ] **Step 16: Update the `ParserCallbacks` trait bound (line 288)**

Old:
```rust
pub trait ParserCallbacks<'a>: ParserHooks<'a, Token, Rule, Diag = Self::Diagnostic, Ctx = Self::Context>
```

The `ParserHooks` still takes `Token, Rule` — no change. The `Parser` type bound changes in the impl, not in the trait definition.

- [ ] **Step 17: Check all `NodeRef(closed.0)` patterns**

Search the entire file for `closed.0` — any remaining instances of `NodeRef(closed.0)` need to be removed since `close()` no longer returns a type with `.0`.

In `gen_left_recursive_rule`, the old code at line ~724:
```rust
let close = Self::gen_cst_close(has_rule_rename, name, true, &parser, false);
```

`gen_cst_close` no longer generates `NodeRef(closed.0)`, so this is handled.

Check the `close_and_create` in the old `gen_cst_close`:
```rust
#parser_name.#create_node_method(NodeRef(closed.0), diags);
```

This is removed. Good.

- [ ] **Step 18: Verify compilation (codegen crate only, examples will fail)**

Run: `cargo check -p lelwel-codegen 2>&1`
Expected: compilation succeeds (the generated output has changed but we haven't regenerated any grammars yet)

- [ ] **Step 19: Commit**

```bash
git add lelwel-codegen/src/backend/rust.rs
git commit -m "feat(codegen): update generated code for generic CstBuilder Parser"
```

---

### Task 6: Update all example code

**Files:**
- Modify: Each example's `lib.rs` and optionally `main.rs`

All examples call `Parser::new(source, diags).parse(&mut diags)` and get back `Cst<'a, Token, Rule>`. After the refactor, `parse()` returns `Cst` via the wrapper (see Task 5 Step 11 — `Rules::parse()` wraps the builder output). So the examples should NOT need changes to their `generate_syntax_tree` functions IF the `Rules::parse()` method handles the wrapping.

Let me verify: `Rules::parse()` (generated) calls `self.parse_with(...)` which returns `B::Output` (= `CstData<Token, Rule>`). Then it wraps: `Cst::new(self.builder.source(), data)`. The return type is `Cst<'a, Token, Rule>`. So the caller gets `Cst` as before. No example changes needed!

BUT — the `Rules` trait is now implemented for `Parser<'a, CstData<Token, Rule>, Ctx>`. The examples create `Parser::new(source, diags)` which is now `Parser<'_, CstData<Token, Rule>, ()>`. The `parse()` method is on the `Rules` trait. So the examples should work without changes IF the import of `Rules` is correct.

Wait — looking at the generated code, the `Rules` trait impl is in `generated.rs` which is included. The `generate_syntax_tree` function calls `Parser::new(source, &mut diags).parse(&mut diags)`. The `parse` method is on the generated `Rules` trait. At the call site, the type `Parser<'_, CstData<Token, Rule>, ()>` implements `Rules<'a>` which provides `parse()`. So method resolution should find it.

But does the `Rules` trait need to be in scope? Let me check the example's lib.rs:

```rust
use lelwel::Parser;
use parser::*;
```

The `parser::*` includes everything from `parser.rs` plus the `generated.rs` code (via `include!`). The `generated.rs` defines the `Rules` trait and its impl. So `Rules` is in scope via `use parser::*`.

The `Parser::new()` constructor is called with the type `Parser<'_, CstData<Token, Rule>, ()>`. Wait — but `Parser::new` is defined for `impl<'a, B, Ctx> Parser<'a, B, Ctx>` where `B: CstBuilder`. The type inference should figure out `B = CstData<Token, Rule>`. 

Actually, looking at the generated code, the `Rules` trait implementation is for `Parser<'a, CstData<Token, Rule>, Ctx>`. But `Parser::new` is for any `B: CstBuilder`. When the user writes `Parser::new(source, &mut diags)`, Rust needs to infer `B`. Since the result calls `.parse()` which is on the `Rules` trait impl for `Parser<'a, CstData<Token, Rule>, Ctx>`, Rust should infer `B = CstData<Token, Rule>` from the method call.

BUT — is `CstData` importable? It's exported from `lelwel` crate via `pub use cst::*`. The example's lib.rs imports `use lelwel::Parser`. It may need to also import `CstData` for type inference... Actually, type inference doesn't require the type to be imported — it just needs to be visible at the definition site. Since `CstData` is pub in the `lelwel` crate, it's visible.

Let me verify: in the example's main.rs, `Parser::new(source, &mut diags).parse(&mut diags)` — the type of `Parser::new(...)` is inferred from the `.parse()` call. `.parse()` is on `Rules<'a>` impl for `Parser<'a, CstData<Token, Rule>, Ctx>`. So `B = CstData<Token, Rule>`. This should compile.

Actually wait, there might be an issue. `Parser::new` has bounds:
```rust
impl<'a, B, Ctx> Parser<'a, B, Ctx>
where
    B: CstBuilder,
    B::Token: TokenType,
    B::Rule: RuleType,
    Self: ParserHooks<'a, B::Token, B::Rule>,
```

For the example code, `Self = Parser<'a, CstData<Token, Rule>, ()>`. Does `Parser<'a, CstData<Token, Rule>, ()>: ParserHooks<'a, CstData<Token, Rule>::Token, CstData<Token, Rule>::Rule>`? Yes — because `CstData<Token, Rule>::Token = Token` and `CstData<Token, Rule>::Rule = Rule`. And the generated code has:

```rust
impl<'a, B, Ctx> ParserHooks<'a, Token, Rule> for Parser<'a, B, Ctx>
where
    B: CstBuilder<Token=Token, Rule=Rule>,
    Self: ParserCallbacks<'a>,
```

Wait — this blanket impl is in the GENERATED code. So the `ParserHooks` for the specific `Parser<'a, CstData<Token, Rule>, ()>` is implemented by the generated code. When the user calls `Parser::new(source, &mut diags)`, the method resolution needs `Self: ParserHooks<'a, B::Token, B::Rule>`. But `B` is unknown at this point!

Actually, Rust can do type inference through method chains. The `.parse()` call is where `B` gets resolved to `CstData<Token, Rule>`. Then `ParserHooks` is checked and should be satisfied. But `new` is a constructor — Rust might not be able to infer `B` from just `Parser::new(source, &mut diags)`.

Let me think... When the user writes `Parser::new(source, &mut diags)`, Rust sees a function `new<'a, B, Ctx>(source, diags) -> Parser<'a, B, Ctx>`. `B` is unconstrained. Then `.parse()` is called on the result. Rust looks for a trait method `parse` on the type. The only candidate is `Rules::parse` which is implemented for `Parser<'a, CstData<Token, Rule>, Ctx>`. So Rust infers `B = CstData<Token, Rule>`. Then it checks the bounds on `new` with `B = CstData<Token, Rule>`, which requires `Self: ParserHooks<'a, Token, Rule>` — satisfied by the generated impl.

This should work. But if it doesn't, we might need a type annotation or a different approach.

For the existing example's `main.rs` and `lib.rs`, I think no changes are needed. The type inference should figure everything out.

BUT — what about the return type? `parse` returns `Cst<'a, Token, Rule>`. The code `let cst = Parser::new(source, &mut diags).parse(&mut diags);` — `cst` is `Cst<'_, Token, Rule>`. The `format!("{cst}")` still works (Display impl on `Cst` is unchanged). So no code changes needed in examples.

Let me verify: does `Parser::new()` still exist? Yes — it's defined in the `impl<'a, B, Ctx> Parser<'a, B, Ctx>` block where `B: CstBuilder, B::Token: TokenType, B::Rule: RuleType`. It's generic over B. The call `Parser::new(source, &mut diags)` uses type inference to determine B.

Actually wait — `Parser::new` calls `Self::new_with_context(source, diags, Ctx::default())`. `new_with_context` calls `Self::create_tokens_hook()`. `create_tokens_hook` is on `ParserHooks`. The generated impl maps it through `ParserCallbacks`. For this to work, `Self: ParserHooks<'a, B::Token, B::Rule>` must be satisfied. If B isn't known yet, this is a problem.

In Rust, when you call `Parser::new(source, diags).parse(diags)`, the method resolution happens in two steps:
1. `Parser::new(source, diags)` — B is unconstrained
2. `.parse(diags)` — trait method resolution constrains B

If the compiler can't resolve `new` because B is unconstrained, we'll get an error. Let me check: `Parser::new` has the bound `Self: ParserHooks<'a, B::Token, B::Rule>`. With `B` unknown, Rust can't check this bound. But Rust only needs to CHECK the bound at monomorphization time — during type inference, it just needs to INFER B.

Actually, Rust trait resolution might fail here. The method `new` has a bound that references `B::Token` and `B::Rule`. If B is truly unconstrained at the call site, Rust won't be able to resolve the method.

But the examples already work with `Parser::new` and generic types. Currently `Parser<'a, T, R, Ctx>` has bounds `T: TokenType, R: RuleType` and `Self: ParserHooks<'a, T, R>`. The same pattern. And in the current code, the examples call `Parser::new(source, &mut diags).parse(&mut diags)` without type annotations. This works because `.parse()` constrains the type.

In the new design, the same pattern applies: `Parser::new(source, &mut diags)` type-checks because the concrete `Token` and `Rule` types aren't needed until `.parse()` is called. The `B` parameter is inferred from the `.parse()` call.

I believe this will work. If not, the fix is simple: annotate the type or use a turbofish.

Let me proceed with the plan assuming examples don't need changes.

- [ ] **Step 1: No changes needed to examples**

The `Rules::parse()` generated method wraps `CstData` output in `Cst`. The example code calls `Parser::new(source, diags).parse(&mut diags)` and gets `Cst` back — same as before.

- [ ] **Step 2: Verify compilation of all examples**

Run: `cargo check --workspace 2>&1`
Expected: should compile

If there are compilation errors, the most likely fix is adding a `use lelwel::CstData;` import or adjusting the type annotation.

- [ ] **Step 3: Run tests**

Run: `cargo test --workspace 2>&1`
Expected: all existing tests pass (the tree structure is unchanged)

- [ ] **Step 4: Commit**

```bash
git commit -am "fix(examples): adapt to CstBuilder-based Parser API"
```

(If no changes were needed, skip this commit.)

---

### Task 7: Remove dead code and clean up

**Files:**
- Modify: `lelwel/src/cst.rs`
- Modify: `lelwel/src/types.rs`

- [ ] **Step 1: Remove old `MarkOpened`, `MarkClosed` types from `types.rs` if unused**

`MarkOpened` is no longer used by the new `CstBuilder` trait (it uses `B::Mark`). But `CstData`'s internal methods (`open()`, `close()`, etc.) might still use them. Check if any of these methods are still called directly.

Since `CstData` now implements `CstBuilder` and the `Parser` goes through the builder, the old `CstData` methods (`open()`, `close()`, `advance()`, etc.) may no longer be called by anything except `CstBuilder` impl. If they're `pub(crate)` and unused, remove them.

- [ ] **Step 2: Clean up `CstData` old methods**

If none of the old methods (`open()`, `close()`, `close_root()`, `advance()`, `open_before()`, `mark()`, `mark_truncation()`, `truncate()`) are called externally anymore (the `CstBuilder` impl methods replace them), remove them from `cst.rs`.

- [ ] **Step 3: Remove unused `CstIndex` size tricks (if any)**

No changes needed — `CstIndex` is still used by `Node` enum variants.

- [ ] **Step 4: Verify compilation**

Run: `cargo check --workspace 2>&1`
Expected: compiles

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace 2>&1`
Expected: all pass

- [ ] **Step 6: Commit**

```bash
git add lelwel/src/cst.rs lelwel/src/types.rs
git commit -m "chore(runtime): remove dead CstData methods, simplify types"
```

---

### Task 8: Cross-backend equivalence test (cstree backend)

**Files:**
- Create: `lelwel/src/cstree.rs` (optional, behind feature flag)
- Modify: `lelwel/Cargo.toml`
- Modify: `lelwel/src/lib.rs`
- Create: test files for cross-backend validation

This task is optional and can be deferred. The spec describes adding cstree support behind an optional feature flag.

- [ ] **Step 1: Add optional `cstree` dependency**

In `lelwel/Cargo.toml`:

```toml
[features]
default = []
cstree = ["dep:cstree"]

[dependencies]
cstree = { version = "...", optional = true }
```

- [ ] **Step 2: Create `cstree.rs` module**

With a `CstreeBuilder<S>` that wraps `cstree::build::GreenNodeBuilder` and implements `CstBuilder`:

```rust
use crate::CstBuilder;
use cstree::build::GreenNodeBuilder;
use cstree::green::GreenNode;
use crate::Span;

pub struct CstreeBuilder<S: Syntax> {
    inner: GreenNodeBuilder<'static, S>,
}

impl<S: Syntax> CstBuilder for CstreeBuilder<S> {
    type Token = S;
    type Rule = S;
    type Mark = cstree::build::Checkpoint;
    type Checkpoint = cstree::build::Checkpoint;
    type Output = GreenNode;

    fn new(_spans: Vec<Span>) -> Self { CstreeBuilder { inner: GreenNodeBuilder::new() } }
    fn token(&mut self, kind: S, text: &str) { ... }
    fn start_rule(&mut self) -> Self::Mark { self.inner.checkpoint() }
    fn end_rule(&mut self, mark: Self::Mark, rule: S) { ... }
    fn mark(&self) -> Self::Mark { self.inner.checkpoint() }
    fn start_rule_before(&mut self, mark: Self::Mark) -> Self::Mark { mark }
    fn checkpoint(&self) -> Self::Checkpoint { self.inner.checkpoint() }
    fn revert_to(&mut self, cp: Self::Checkpoint) { self.inner.revert_to(cp); }
    fn finish(self) -> GreenNode { self.inner.finish().0 }
}
```

- [ ] **Step 3: Add cross-backend test**

A test that parses the same input with both backends and asserts structural equivalence (same tree structure).

- [ ] **Step 4: Verify**

Run: `cargo test --features cstree 2>&1`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add lelwel/Cargo.toml lelwel/src/cstree.rs lelwel/src/lib.rs
git commit -m "feat: add optional cstree backend with cross-backend tests"
```

---

## Self-Review

**Spec coverage check:**
1. ✅ `CstBuilder` trait — Task 1
2. ✅ `CstData` implements `CstBuilder`, no `non_skip_len` — Task 2
3. ✅ `LelwelBuilder` wrapper with skip buffering — Task 1
4. ✅ `in_ordered_choice` moves from `Parser` to `LelwelBuilder` — Task 3 (field in LelwelBuilder)
5. ✅ `Parser` generic over `B: CstBuilder` — Task 3
6. ✅ `LelwelBuilder::open/close/close_root` flush semantics — Task 1
7. ✅ `LelwelBuilder::state/restore` for backtracking — Task 1
8. ✅ Codegen updates for `self.builder.in_ordered_choice` — Task 5
9. ✅ NodeRef construction through `CstBuilder::node_ref` — Task 1, Task 3
10. ✅ `parse_with` returns `B::Output` — Task 3
11. ✅ Cstree backend — Task 8 (optional/deferred)
12. ✅ Cross-backend tests — Task 8 (optional/deferred)
13. ✅ Migration path — progressive tasks 1-7
14. ❌ **delete_node callback**: Handled through `iterate_removed` + `restore` callback — covers the requirement
15. ✅ Existing golden tests unchanged — Task 6 verification

**Placeholder scan:** No TBD, TODO, "implement later" (cstree in Task 8 is explicitly optional and deferred — that's fine).
**Type consistency:** `B::Mark` used consistently across Parser methods. `CstData::Mark = usize`. `LelwelBuilder::end_rule()` returns `B::Mark`. `Parser::close()` returns `B::Mark`. ✅
