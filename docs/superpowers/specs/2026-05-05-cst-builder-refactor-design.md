# CST Builder Refactoring Design

## Problem

The lelwel runtime library's `CstData` is tightly coupled to the `Parser` implementation. The parser directly calls `CstData::advance(token, skip)` with skip tracking logic baked in, and `CstData` maintains `non_skip_len` as part of its internal state. This makes it impossible to use the parser with alternative CST representations such as `cstree`.

## Solution

Introduce a `CstBuilder` trait that abstracts tree construction. The `Parser` becomes generic over a `CstBuilder` implementation. A `LelwelBuilder<B>` wrapper provides the skip-buffering semantics for all backends. Two implementations:

1. `CstData` (existing, default) — produces `Cst<'a, T, R>`
2. cstree's `GreenNodeBuilder` — produces `cstree::GreenNode`

Key principle: the `CstBuilder` trait has NO notion of skip tokens. Skip buffering/handling is layered on top via the wrapper, so every backend gets consistent skip semantics.

## CstBuilder Trait

```rust
pub trait CstBuilder {
    type Token;
    type Rule;
    type Mark: Copy;
    type Checkpoint: Copy;
    type Output;

    fn token(&mut self, kind: Self::Token, text: &str);
    fn start_rule(&mut self) -> Self::Mark;
    fn end_rule(&mut self, mark: Self::Mark, rule: Self::Rule);
    fn mark(&self) -> Self::Mark;
    fn start_rule_before(&mut self, mark: Self::Mark) -> Self::Mark;
    fn checkpoint(&self) -> Self::Checkpoint;
    fn revert_to(&mut self, checkpoint: Self::Checkpoint);
    fn finish(self) -> Self::Output;
}
```

No `skip` parameter on `token()`. Each backend treats all tokens equally; skip handling is done by the wrapper.

## LelwelBuilder Wrapper

```rust
pub struct LelwelBuilder<'s, B: CstBuilder> {
    inner: B,
    source: &'s str,
    buffer: Vec<(B::Token, usize, usize)>,  // (kind, start, end)
    start_idx: usize,                       // flush frontier
    pub in_ordered_choice: bool,            // moved from Parser
}
```

The buffer accumulates skip tokens for exclusion from `close()` child counts. What happens after `flush()` depends on mode:

- **Ordered choice** (`in_ordered_choice = true`): Append-only. Flushed tokens stay in the buffer so `restore()` can re-emit them by rewinding `start_idx`.
- **Normal** (`in_ordered_choice = false`): Flushed tokens are drained from the buffer. No backtracking is possible, so retention is unnecessary.

### Flush

```rust
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
```

Copies all unflushed buffer entries into the inner builder via `inner.token()`. In ordered choice mode, tokens stay in the buffer — `start_idx` moves to `buffer.len()` and future `restore()` can rewind it for re-emission. In normal mode, the buffer is drained — no backtracking can occur, so retention serves no purpose.

### Advance

```rust
fn advance(&mut self, kind: B::Token, skip: bool, span: Range<usize>) {
    if skip {
        self.buffer.push((kind, span.start, span.end));
    } else {
        self.flush();
        self.inner.token(kind, &self.source[span]);
    }
}
```

Skip tokens are appended to the buffer. Non-skip tokens flush first (committing all pending skips to the tree), then emit the non-skip token.

### Open / Close

```rust
fn open(&mut self) -> B::Mark {
    self.flush();
    self.inner.start_rule()
}

fn close(&mut self, mark: B::Mark, rule: B::Rule) {
    self.inner.end_rule(mark, rule);
}

fn close_root(&mut self, mark: B::Mark, rule: B::Rule) {
    self.flush();
    self.inner.end_rule(mark, rule);
}
```

`open()` flushes so the mark position includes all preceding content. `close()` does NOT flush — trailing skip tokens in the buffer stay excluded from this rule's child count (because they haven't been committed to the tree yet). `close_root()` flushes (the root includes everything).

### Mark / Checkpoint / Open Before

```rust
fn mark(&mut self) -> B::Mark {
    self.flush();
    self.inner.mark()
}

fn state(&self) -> (B::Checkpoint, usize, usize) {
    (self.inner.checkpoint(), self.start_idx, self.buffer.len())
}

fn restore(&mut self, checkpoint: B::Checkpoint, start_idx: usize, buffer_len: usize) {
    self.buffer.truncate(buffer_len);
    self.start_idx = start_idx;
    self.inner.revert_to(checkpoint);
}

fn start_rule_before(&mut self, mark: B::Mark) -> B::Mark {
    self.flush();
    self.inner.start_rule_before(mark)
}
```

`mark()` flushes (position must be accurate). `state()` captures inner builder checkpoint PLUS `start_idx` and `buffer.len()` — the full wrapper state. `restore()` restores all three: truncating the buffer, restoring the flush frontier, and reverting the inner builder.

`state()` and `restore()` are only called during ordered choice (where `in_ordered_choice` is `true`). In normal mode, the buffer is drained after each `flush()` so there's nothing to restore.

### Revert behavior

When restoring state, `buffer.truncate(len)` removes tokens added after the checkpoint. `start_idx` is restored to the checkpoint's value. If those tokens were already flushed (via a non-skip advance after the checkpoint), `inner.revert_to()` removes them from the tree. On the next `flush()`, `buffer[start_idx..]` is re-copied into the tree — the tokens re-enter from the buffer, matching the restored parser position.

This works because:
- In ordered choice mode, the buffer is append-only until a revert
- Revert restores the flush frontier + buffer length + inner state
- Next flush re-emits the correct tokens into the tree

## CstData Implements CstBuilder

```rust
impl<T: TokenType, R: RuleType> CstBuilder for CstData<T, R> {
    type Token = T;
    type Rule = R;
    type Mark = usize;       // unified: nodes.len() position
    type Checkpoint = MarkTruncation;
    type Output = CstData<T, R>;

    fn token(&mut self, kind: T, _text: &str) {
        self.nodes.push(Node::Token(kind, self.token_count.into()));
        self.token_count += 1;
        // All tokens through the trait are "non-skip" from CstData's perspective.
        // Skip exclusion is handled by the wrapper never calling token() for
        // rule-trailing skips until close_root.
    }

    fn start_rule(&mut self) -> usize {
        let pos = self.nodes.len();
        self.nodes.push(Node::Rule(R::error(), 0.into()));
        // No non_skip tracking needed — all tokens are "non-skip"
        pos
    }

    fn end_rule(&mut self, mark: usize, rule: R) {
        let len = self.nodes.len();
        self.nodes[mark] = Node::Rule(
            rule,
            if mark >= len { 0 } else { (len - 1 - mark) as u16 }.into(),
        );
    }

    fn mark(&self) -> usize { self.nodes.len() }
    fn checkpoint(&self) -> MarkTruncation { /* saves node_count, token_count */ }
    fn revert_to(&mut self, cp: MarkTruncation) { /* truncates nodes, token_count */ }
    fn finish(self) -> CstData<T, R> { self }
}
```

Since the wrapper handles skip buffering, `CstData` no longer needs `non_skip_len`. All tokens arriving via `token()` are equally "children" — trailing exclusion is done by the wrapper's `close()` not flushing. The `MarkTruncation` simplifies to `(node_count, token_count)`.

After parsing, `Cst` is constructed from `CstData` + source.

## Cstree Implements CstBuilder

```rust
impl<S: Syntax> CstBuilder for CstreeBuilder<S> {
    type Token = S;
    type Rule = S;
    type Mark = cstree::build::Checkpoint;
    type Checkpoint = cstree::build::Checkpoint;
    type Output = cstree::green::GreenNode;

    fn token(&mut self, kind: S, text: &str) {
        if let Some(static_text) = kind.static_text() {
            debug_assert_eq!(static_text, text);
            self.inner.static_token(kind);
        } else {
            self.inner.token(kind, text);
        }
    }

    fn start_rule(&mut self) -> Checkpoint { self.inner.checkpoint() }
    fn end_rule(&mut self, mark: Checkpoint, rule: S) {
        self.inner.start_node_at(mark, rule);
        self.inner.finish_node();
    }
    fn start_rule_before(&mut self, mark: Checkpoint) -> Checkpoint { mark }
    fn mark(&self) -> Checkpoint { self.inner.checkpoint() }
    fn checkpoint(&self) -> Checkpoint { self.inner.checkpoint() }
    fn revert_to(&mut self, cp: Checkpoint) { self.inner.revert_to(cp); }
    fn finish(self) -> GreenNode { self.inner.finish().0 }
}
```

`start_rule()` records a checkpoint. `end_rule()` wraps everything since that checkpoint into a node via `start_node_at`. `start_rule_before()` is a no-op — `end_rule()` handles wrapping regardless of which marker method was used.

## Parser Genericity

`Parser` is generic over `B: CstBuilder`. The `Cst<'a, T, R>` field becomes `LelwelBuilder<'a, B>`. The `in_ordered_choice` field moves from `Parser` to `LelwelBuilder`. The parser calls `self.builder.advance(token, skip, span)`, `self.builder.open()`, etc. Generated code accesses `self.builder.in_ordered_choice` directly.

`parse_with()` returns `B::Output`:
- For built-in CST: `B = CstData<T, R>`, output is `Cst<'a, T, R>` (constructed by wrapping the finished `CstData`)
- For cstree: `B = CstreeBuilder<S>`, output is `cstree::GreenNode`

## Testing Strategy

### Two-level testing

**Level 1 — Existing golden tests (unchanged):** Each example's existing `.tree` + `.diag` golden tests continue to validate the built-in `CstData` backend. These remain the reference. The test infrastructure (`generate_syntax_tree` → `vec![tree_string, diag_string]`) stays identical.

**Level 2 — Cross-backend equivalence:** A new test function at the runtime level (or per-example) parses the same input with both backends and asserts structural equivalence. The cstree output is converted to the same `.tree` display format via a `Display`-like walker:

```rust
fn fmt_green_node(f: &mut fmt::Formatter, node: &GreenNode, source: &str, indent: usize) {
    for child in node.children() {
        match child {
            GreenChild::Token(token) => {
                let kind: S = token.kind();
                let text = &source[token.text_range()];
                let range = token.text_range();
                writeln!(f, "{}{kind:?} {text:?} [{:?}]", "    ".repeat(indent), range)?;
            }
            GreenChild::Node(node) => {
                let kind: S = node.kind();
                let range = node.text_range();
                writeln!(f, "{}{kind:?} [{:?}]", "    ".repeat(indent), range)?;
                fmt_green_node(f, node, source, indent + 1)?;
            }
        }
    }
}
```

This produces a format equivalent to `Cst::Display`. The test then:

```rust
#[test]
fn cross_backend() {
    // Parse with built-in backend — this is the reference
    let (ref_cst, diags) = parse_with_builtin(source);
    let ref_tree = format!("{ref_cst}");
    assert_eq!(ref_tree, golden_tree);  // existing golden check

    // Parse with cstree backend — must produce same tree
    let green_node = parse_with_cstree(source);
    let cstree_tree = fmt_green_node_to_string(&green_node, source);
    assert_eq!(cstree_tree, ref_tree);  // or: assert_eq!(cstree_tree, golden_tree);
}
```

The assertion chain is: cstree output == built-in output == golden file. This ensures both backends produce structurally identical trees without maintaining duplicate golden files.

Per-example test code can optionally in-line the cross-backend check into each existing test:

```rust
macro_rules! check {
    ($file:literal) => {
        let source = &include_str!(concat!("data/", $file, ".json"));
        let (cst, cstree_node, diags) = parse_both_backends(source, &mut vec![]);
        assert_eq!(format!("{cst}"), golden_tree);
        assert_eq!(fmt_cstree(&cstree_node, source), golden_tree);
        assert_eq!(format_diags(&diags), golden_diag);
    };
}
```

A helper `parse_both_backends` wraps a single parse call that builds two trees simultaneously using a composite builder, or runs two independent parses (simpler, and safe since parsing is deterministic).

## Migration Path

1. Add `CstBuilder` trait
2. Implement `CstBuilder` for CstData (no more `non_skip_len` in close)
3. Add `LelwelBuilder` wrapper with `in_ordered_choice` + two-mode flush
4. Move `in_ordered_choice` from `Parser` to `LelwelBuilder`, update codegen accesses
5. Make Parser generic over `B: CstBuilder` via `LelwelBuilder<B>`
6. Implement `CstBuilder` for cstree's `GreenNodeBuilder`
7. Add tests for cstree backend
8. Clean up: remove `non_skip_len` from CstData
