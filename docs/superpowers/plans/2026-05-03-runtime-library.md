# Runtime Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract static parser infrastructure from generated code into a reusable `lelwel` runtime crate, adding generics for `TokenType` and `RuleType`.

**Architecture:** The current monolithic `lelwel` crate (code generator) is renamed to `lelwel-codegen` and moved to a subdirectory. A new `lelwel` crate is created with the generic runtime types and traits (`TokenType`, `RuleType`, `ParserHooks`, `Parser`, CST types). The code generator (`lelwel-codegen`) is updated to emit code that uses these runtime types and implements the `Rules` and `ParserCallbacks` traits. A blanket impl bridges `ParserCallbacks` to `ParserHooks`.

**Tech Stack:** Rust, cargo workspaces, `proc-macro2`/`quote`/`syn`/`prettyplease` (code generation)

---

## File Structure

### New files
- `lelwel/Cargo.toml` — runtime crate manifest
- `lelwel/src/lib.rs` — re-exports
- `lelwel/src/types.rs` — `NodeRef`, `CstIndex`, `Span`, `MarkOpened`, `MarkClosed`, `MarkTruncation`
- `lelwel/src/parser.rs` — `Parser` struct, `ParserState`, `ParserHooks` trait, `TokenType` trait, `RuleType` trait, `err!` macro
- `lelwel/src/cst.rs` — `Node<T, R>`, `CstData<T, R>`, `Cst<'a, T, R>`, `CstChildren<'a, T, R>`

### Moved files (current `src/` → `lelwel-codegen/src/`)
- Entire current `src/` directory moves to `lelwel-codegen/src/`

### Modified files
- `Cargo.toml` — becomes workspace-only
- `lelwel-codegen/Cargo.toml` — renamed from current root `Cargo.toml`
- `lelwel-codegen/src/lib.rs` — update `build` function, adjust module paths if needed
- `lelwel-codegen/src/backend/rust.rs` — major changes to generated code structure
- All example `Cargo.toml` — add `lelwel` dep, change build-dep to `lelwel-codegen`
- All example `build.rs` — `lelwel_codegen::build(...)`
- All example `parser.rs` — update types and imports
- All example `lexer.rs` — update `Span`/`Diagnostic` imports
- `README.md` — update quickstart, crate names, usage docs

### Test files to update
- `tests/frontend.rs` — may need regeneration
- All example `tests/test.rs` — should work without changes (they use `Parser::new().parse()`)
- `scripts/update_grammar.sh` — update if paths change

---

## Task 1: Create workspace structure

**Files:**
- Modify: `Cargo.toml`
- Create: `lelwel-codegen/Cargo.toml`
- Create: `lelwel/Cargo.toml`
- Move: `src/` → `lelwel-codegen/src/`

- [ ] **Step 1: Create the `lelwel-codegen/` directory and move `src/`**

```bash
mkdir -p lelwel-codegen
git mv src lelwel-codegen/src
```

No other files move — the bins, frontend code, etc. all stay under `lelwel-codegen/`.

- [ ] **Step 2: Create `lelwel-codegen/Cargo.toml`**

Copy the current root `Cargo.toml` content, changing the package name:

```toml
[package]
name = "lelwel-codegen"
version = "0.10.4"
description = "Resilient LL(1) parser generator"
repository = "https://github.com/0x2a-42/lelwel"
readme = "README.md"
license = "MIT OR Apache-2.0"
edition = "2024"
keywords = ["parser", "generator", "LL", "grammar"]
categories = ["parsing"]

[lints.rust]
rust-2018-idioms = { level = "deny" }

[dependencies]
logos = "0.16"
codespan-reporting = "0.13"
rustc-hash = "2.1"
quote = "1"
proc-macro2 = "1"
syn = { version = "2", features = ["full", "parsing"] }
prettyplease = "0.2"
codespan-lsp = { version = "0.13", optional = true }
clap = { version = "4.5", features = ["cargo"], optional = true }
lsp-types = { version = "0.95", optional = true }
lsp-types_old = { package = "lsp-types", version = "0.91", optional = true }
lsp-server = { version = "0.7", optional = true }
serde = { version = "1.0", optional = true }
serde_json = { version = "1.0", optional = true }
dprint-core = { version = "0.67", optional = true }
wasm-bindgen = { version = "0.2", optional = true }

[features]
cli = ["clap", "dprint-core"]
lsp = ["lsp-server", "codespan-lsp", "lsp-types", "lsp-types_old", "serde", "serde_json", "dprint-core"]
wasm = ["wasm-bindgen"]

[[bin]]
name = "llw"
required-features = ["cli"]

[[bin]]
name = "lelwel-ls"
required-features = ["lsp"]

[lib]
crate-type = ["lib", "cdylib"]
```

- [ ] **Step 3: Create the root workspace `Cargo.toml`**

Replace the current root `Cargo.toml` with a workspace-only config:

```toml
[workspace]
members = [
    "lelwel-codegen",
    "lelwel",
    "examples/c",
    "examples/calc",
    "examples/json",
    "examples/l",
    "examples/lua",
    "examples/oberon0",
    "examples/python2",
    "examples/toml",
    "examples/wgsl",
    "examples/brainfuck"
]
```

- [ ] **Step 4: Create `lelwel/Cargo.toml` for the runtime crate**

```toml
[package]
name = "lelwel"
version = "0.10.4"
description = "Resilient LL(1) parser runtime library"
repository = "https://github.com/0x2a-42/lelwel"
license = "MIT OR Apache-2.0"
edition = "2024"
keywords = ["parser", "LL", "grammar"]
categories = ["parsing"]

[lints.rust]
rust-2018-idioms = { level = "deny" }

[dependencies]
```

No dependencies — the runtime crate is zero-dep.

- [ ] **Step 5: Update `lelwel-codegen/src/lib.rs`**

The `VERSION` const and `build` function need updating. The public API changes from `lelwel::build` to `lelwel_codegen::build`. Change:

```rust
const VERSION: &str = "0.10.4";
```

And the `build` function should remain as-is (it's already `pub fn build(path: &str)`). No change needed to the function signature, but the crate name changes.

- [ ] **Step 6: Verify the workspace builds**

```bash
cargo check -p lelwel-codegen
```

Expected: compiles successfully (the codegen crate hasn't changed functionally, just moved).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Restructure: rename lelwel crate to lelwel-codegen, create workspace"
```

---

## Task 2: Create the runtime crate skeleton

**Files:**
- Create: `lelwel/src/lib.rs`
- Create: `lelwel/src/types.rs`
- Create: `lelwel/src/cst.rs`
- Create: `lelwel/src/parser.rs`

- [ ] **Step 1: Create `lelwel/src/types.rs` with unparameterized types**

These types don't depend on `T` or `R` and are identical to the current generated versions:

```rust
pub type Span = core::ops::Range<usize>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeRef(pub usize);

impl NodeRef {
    pub const ROOT: NodeRef = NodeRef(0);
}

#[cfg(target_pointer_width = "64")]
#[derive(Copy, Clone)]
pub struct CstIndex([u8; 6]);

#[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
#[derive(Copy, Clone)]
pub struct CstIndex(usize);

impl core::fmt::Debug for CstIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        usize::from(*self).fmt(f)
    }
}

impl From<CstIndex> for usize {
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn from(value: CstIndex) -> Self {
        let [b0, b1, b2, b3, b4, b5] = value.0;
        usize::from_le_bytes([b0, b1, b2, b3, b4, b5, 0, 0])
    }
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    #[inline]
    fn from(value: CstIndex) -> Self {
        value.0
    }
}

impl From<usize> for CstIndex {
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn from(value: usize) -> Self {
        let [b0, b1, b2, b3, b4, b5, b6, b7] = value.to_le_bytes();
        debug_assert!(b6 == 0 && b7 == 0);
        Self([b0, b1, b2, b3, b4, b5])
    }
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    #[inline]
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MarkOpened(pub usize);

#[derive(Clone, Copy)]
pub(crate) struct MarkClosed(pub usize);

#[derive(Clone)]
pub(crate) struct MarkTruncation {
    pub node_count: usize,
    pub token_count: usize,
    pub non_skip_len: usize,
}
```

- [ ] **Step 2: Create `lelwel/src/cst.rs` with generic CST types**

```rust
use crate::types::*;
use crate::parser::{TokenType, RuleType};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Node<T: TokenType, R: RuleType> {
    Rule(R, CstIndex),
    Token(T, CstIndex),
}

#[derive(Default)]
pub struct CstChildren<'a, T: TokenType, R: RuleType> {
    iter: core::slice::Iter<'a, Node<T, R>>,
    offset: usize,
}

impl<T: TokenType, R: RuleType> Iterator for CstChildren<'_, T, R> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.offset;
        self.offset += 1;
        if let Some(node) = self.iter.next() {
            if let Node::Rule(_, end_offset) = node {
                let end_offset = usize::from(*end_offset);
                if end_offset > 0 {
                    self.iter.nth(end_offset.saturating_sub(1));
                    self.offset += end_offset;
                }
            }
            Some(NodeRef(offset))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct CstData<T: TokenType, R: RuleType> {
    pub spans: Vec<Span>,
    pub nodes: Vec<Node<T, R>>,
    pub token_count: usize,
    pub non_skip_len: usize,
}

impl<T: TokenType, R: RuleType> CstData<T, R> {
    pub(crate) fn new(spans: Vec<Span>) -> Self {
        let nodes = Vec::with_capacity(spans.len() * 2);
        Self {
            spans,
            nodes,
            token_count: 0,
            non_skip_len: 0,
        }
    }

    pub(crate) fn open(&mut self) -> MarkOpened {
        let mark = MarkOpened(self.nodes.len());
        self.nodes.push(Node::Rule(R::error(), 0.into()));
        self.non_skip_len = self.nodes.len();
        mark
    }

    pub(crate) fn close(&mut self, mark: MarkOpened, rule: R) -> MarkClosed {
        let len = self.non_skip_len - 1;
        self.nodes[mark.0] = Node::Rule(
            rule,
            if mark.0 > len {
                self.non_skip_len += mark.0 - len;
                0
            } else {
                len - mark.0
            }
            .into(),
        );
        MarkClosed(mark.0)
    }

    pub(crate) fn close_root(&mut self, mark: MarkOpened, rule: R) -> MarkClosed {
        self.nodes[mark.0] = Node::Rule(rule, (self.nodes.len() - 1 - mark.0).into());
        MarkClosed(mark.0)
    }

    pub(crate) fn advance(&mut self, token: T, skip: bool) {
        self.nodes.push(Node::Token(token, self.token_count.into()));
        self.token_count += 1;
        if !skip {
            self.non_skip_len = self.nodes.len();
        }
    }

    pub(crate) fn open_before(&mut self, mark: MarkClosed) -> MarkOpened {
        self.nodes.insert(mark.0, Node::Rule(R::error(), 0.into()));
        self.non_skip_len += 1;
        MarkOpened(mark.0)
    }

    pub(crate) fn mark(&self) -> MarkClosed {
        MarkClosed(self.nodes.len())
    }

    pub(crate) fn mark_truncation(&self) -> MarkTruncation {
        MarkTruncation {
            node_count: self.nodes.len(),
            token_count: self.token_count,
            non_skip_len: self.non_skip_len,
        }
    }

    pub(crate) fn truncate(&mut self, mark: MarkTruncation) {
        self.nodes.truncate(mark.node_count);
        self.token_count = mark.token_count;
        self.non_skip_len = mark.non_skip_len;
    }

    pub fn children(&self, node_ref: NodeRef) -> CstChildren<'_, T, R> {
        let iter = if let Node::Rule(_, end_offset) = self.nodes[node_ref.0] {
            self.nodes[node_ref.0 + 1..node_ref.0 + usize::from(end_offset) + 1].iter()
        } else {
            core::slice::Iter::default()
        };
        CstChildren {
            iter,
            offset: node_ref.0 + 1,
        }
    }

    pub fn get(&self, node_ref: NodeRef) -> Node<T, R> {
        self.nodes[node_ref.0]
    }

    pub fn span(&self, node_ref: NodeRef) -> Span {
        fn find_token<'a, T: TokenType, R: RuleType>(mut iter: impl Iterator<Item = &'a Node<T, R>>) -> Option<usize> {
            iter.find_map(|node| match node {
                Node::Rule(..) => None,
                Node::Token(_, idx) => Some(usize::from(*idx)),
            })
        }
        match self.nodes[node_ref.0] {
            Node::Token(_, idx) => self.spans[usize::from(idx)].clone(),
            Node::Rule(_, end_offset) => {
                let end = node_ref.0 + usize::from(end_offset);
                let first = find_token(self.nodes[node_ref.0 + 1..=end].iter());
                let last = find_token(self.nodes[node_ref.0 + 1..=end].iter().rev());
                if let (Some(first), Some(last)) = (first, last) {
                    self.spans[first].start..self.spans[last].end
                } else {
                    let offset = find_token(self.nodes[..node_ref.0].iter().rev())
                        .map_or(0, |before| self.spans[before].end);
                    offset..offset
                }
            }
        }
    }

    pub fn match_token(&self, node_ref: NodeRef, matched_token: T) -> Option<Span> {
        match self.nodes[node_ref.0] {
            Node::Token(token, idx) if token == matched_token => {
                Some(self.spans[usize::from(idx)].clone())
            }
            _ => None,
        }
    }

    pub fn match_rule(&self, node_ref: NodeRef, matched_rule: R) -> bool {
        matches!(self.nodes[node_ref.0], Node::Rule(rule, _) if rule == matched_rule)
    }
}

#[derive(Debug)]
pub struct Cst<'a, T: TokenType, R: RuleType> {
    pub source: &'a str,
    pub data: CstData<T, R>,
}

impl<'a, T: TokenType, R: RuleType> Cst<'a, T, R> {
    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn into_data(self) -> CstData<T, R> {
        self.data
    }

    pub fn children(&self, node_ref: NodeRef) -> CstChildren<'_, T, R> {
        self.data.children(node_ref)
    }

    pub fn get(&self, node_ref: NodeRef) -> Node<T, R> {
        self.data.get(node_ref)
    }

    pub fn span(&self, node_ref: NodeRef) -> Span {
        self.data.span(node_ref)
    }

    pub fn match_token(&self, node_ref: NodeRef, matched_token: T) -> Option<(&'a str, Span)> {
        self.data.match_token(node_ref, matched_token).map(|span| (&self.source[span.clone()], span))
    }

    pub fn match_rule(&self, node_ref: NodeRef, matched_rule: R) -> bool {
        self.data.match_rule(node_ref, matched_rule)
    }

    pub fn span_text(&self, span_idx: CstIndex) -> &'a str {
        &self.source[self.data.spans[usize::from(span_idx)].clone()]
    }
}

impl<T: TokenType + core::fmt::Debug, R: RuleType> core::fmt::Display for Cst<'_, T, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const DEPTH: &str = "    ";
        fn rec<T: TokenType + core::fmt::Debug, R: RuleType>(
            cst: &Cst<'_, T, R>,
            f: &mut core::fmt::Formatter<'_>,
            node_ref: NodeRef,
            indent: usize,
        ) -> core::fmt::Result {
            match cst.get(node_ref) {
                Node::Rule(rule, _) => {
                    let span = cst.span(node_ref);
                    writeln!(f, "{}{rule:?} [{span:?}]", DEPTH.repeat(indent))?;
                    for child_node_ref in cst.children(node_ref) {
                        rec(cst, f, child_node_ref, indent + 1)?;
                    }
                    Ok(())
                }
                Node::Token(token, idx) => {
                    let span = &cst.data.spans[usize::from(idx)];
                    writeln!(
                        f,
                        "{}{:?} {:?} [{:?}]",
                        DEPTH.repeat(indent),
                        token,
                        &cst.source[span.clone()],
                        span,
                    )
                }
            }
        }
        rec(self, f, NodeRef::ROOT, 0)
    }
}
```

Note: `RuleType` requires `Debug` bound for the `Display` impl. `TokenType` requires `Debug` for the `Display` impl's token formatting. These are already in the trait bounds.

- [ ] **Step 3: Create `lelwel/src/parser.rs` with traits and `Parser` struct**

This is the largest file. It contains `TokenType`, `RuleType`, `ParserHooks`, `Parser`, `ParserState`, and the `err!` macro. The key design is that `Parser` is generic over `T: TokenType` and `R: RuleType`, with its `impl` block requiring `Self: ParserHooks<'a, T, R>`.

See the spec for the full method signatures. The implementation mirrors the current `gen_generated` output but with `T` and `R` generic parameters. All methods currently on `impl<'a> Parser<'a>` move here, except `rule_*`, `parse`, `parse_*`, `create_node` (dispatch), `delete_node` (dispatch), `create_node_error` (method), `close_error_node` (calls `create_node_error`), `is_skipped` (replaced by `token.is_skip()`).

For `advance` and `init_skip`, replace `Token::Error | skip_patterns` with `token.is_skip()`.

For `close_error_node`, replace `self.create_node_error(...)` with `ParserHooks::create_node_error(self, ...)`.

The `Parser` struct fields change:
- `context` becomes `<Self as ParserHooks<'a, T, R>>::Context`
- All `Diagnostic` references become `<Self as ParserHooks<'a, T, R>>::Diagnostic`
- `Node` becomes `Node<T, R>`, `Cst` becomes `Cst<'a, T, R>`, etc.

This is a substantial but mechanical translation from the existing generated code. Write it all out completely.

- [ ] **Step 4: Create `lelwel/src/lib.rs`**

```rust
#![forbid(unsafe_code)]

mod types;
mod cst;
mod parser;

pub use types::*;
pub use cst::*;
pub use parser::*;
```

- [ ] **Step 5: Verify the runtime crate compiles**

```bash
cargo check -p lelwel
```

Expected: compiles with no errors (there will be `dead_code` warnings for `MarkOpened` etc. since they're `pub(crate)`, that's fine).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Add lelwel runtime crate with generic Parser, CST types, and traits"
```

---

## Task 3: Update code generator to emit new structure

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs`

This is the largest task. The `RustOutput::gen_generated` method needs to be rewritten to emit:
1. `impl TokenType for Token` (with `is_skip`, `is_eof`, `eof`)
2. `Rule` enum and `impl RuleType for Rule`
3. `ParserCallbacks<'a>` trait (with user-overridable methods + grammar-specific defaults)
4. Blanket `impl<'a, P> ParserHooks<'a, Token, Rule> for P where P: ParserCallbacks<'a>`
5. `Rules<'a>` trait and `impl Rules for Parser<'a, Token, Rule>`
6. `expect!` / `try_expect!` macros
7. `impl<'a> ParserCallbacks<'a> for Parser<'a, Token, Rule>` scaffold (default impl)

Instead of generating CST types, `Parser` struct, `NodeRef`, etc., those are now imported from `lelwel`.

The `gen_parser` method (which generates `parser.rs` when it doesn't exist) also needs updating.

The `gen_lexer` method needs updating to import `Span` from `lelwel` instead of `parser`.

- [ ] **Step 1: Rewrite `gen_generated` body**

The new output should be a `TokenStream` containing:

```rust
use lelwel::{TokenType, RuleType, ParserHooks, Parser, NodeRef, CstIndex, Node, ...};

// impl TokenType for Token { ... }
// #[derive(...)] pub enum Rule { ... }
// impl RuleType for Rule { ... }
// impl Display for Rule { ... }
// macro_rules! err { ... }
// macro_rules! expect { ... }
// macro_rules! try_expect { ... }
// pub trait ParserCallbacks<'a> { ... }
// impl<'a, P> ParserHooks<'a, Token, Rule> for P where P: ParserCallbacks<'a> { ... }
// pub trait Rules<'a>: ParserCallbacks<'a> + Sized { ... }
// impl<'a> Rules<'a> for Parser<'a, Token, Rule> { ... }
// Default impl ParserCallbacks for Parser (scaffold)
```

Key changes from current `gen_generated`:
- Remove all CST type definitions (`NodeRef`, `CstIndex`, `Node`, `CstData`, `Cst`, `CstChildren`, `MarkOpened`, `MarkClosed`, `MarkTruncation`, `ParserState`)
- Remove `Parser` struct definition
- Remove `impl Parser` block (all runtime methods)
- Remove `err!` body? No — `err!` stays, but it references `ParserHooks::create_diagnostic` which resolves through the blanket impl
- Add `use lelwel::{...};` imports
- Add `impl TokenType for Token { fn is_skip(&self) -> bool { matches!(self, Token::Error | ...) } fn is_eof(&self) -> bool { matches!(self, Token::EOF | ...) } fn eof() -> Self { Token::EOF } }`
- Add `impl RuleType for Rule { fn error() -> Self { Rule::Error } }`
- Add `ParserCallbacks<'a>` trait with all user-overridable + grammar-specific methods
- Add blanket `ParserHooks` impl
- Add `Rules<'a>` trait and impl

For the `rule_*` methods, they become methods on the `Rules` trait impl instead of on `impl Parser`. The method signatures change:
- `fn rule_foo(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>)` 
  instead of `fn rule_foo(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>)`
- `self.current` still works (pub field)
- `self.advance(...)`, `self.open(...)`, etc. still work (methods on `Parser` require `Self: ParserHooks`)

For left-recursive rule `rec` functions, they reference `Parser<'a, Token, Rule>` directly and `&mut Parser<'a, Token, Rule>` as the parser parameter.

The `rule_*` method signatures reference `ParserCallbacks<'a>::Diagnostic` which, through the blanket impl, equals `ParserHooks<'a, Token, Rule>::Diagnostic`.

For `parse` and `parse_*` entry points, they live in the `Rules` trait impl:

```rust
fn parse(self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> Cst<'a, Token, Rule> {
    self.end_of_input = Token::EOF;
    self.parse_with(|parser, diags| parser.rule_start(diags), diags, Rule::Start)
}
```

- [ ] **Step 2: Rewrite `gen_parser` (scaffold for `parser.rs`)**

The scaffold needs to:
1. Import from `lelwel` crate instead of defining types locally
2. Use `Parser<'a, Token, Rule>` instead of `Parser<'a>`
3. Implement `ParserCallbacks<'a>` instead of current `ParserCallbacks<'a>`

- [ ] **Step 3: Rewrite `gen_lexer`**

The lexer scaffold needs to import `Span` from `lelwel` instead of `parser`:

```rust
use lelwel::Span;
```

Instead of:

```rust
use super::parser::Span;
```

- [ ] **Step 4: Update `RustOutput::run`**

The `run` function should still generate `generated.rs` and optionally `parser.rs`/`lexer.rs`. No structural changes needed, just make sure the output is valid.

- [ ] **Step 5: Verify codegen crate compiles**

```bash
cargo check -p lelwel-codegen
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Update codegen to emit runtime-based generated code"
```

---

## Task 4: Update example crates

**Files:**
- Modify: all `examples/*/Cargo.toml`
- Modify: all `examples/*/build.rs`
- Modify: all `examples/*/src/parser.rs`
- Modify: all `examples/*/src/lexer.rs`

This is a batch update of all example crates. The changes are mechanical:

### `Cargo.toml` changes
Add `lelwel` as a dependency, change `lelwel` build-dep to `lelwel-codegen`:

```toml
[dependencies]
lelwel = { path = "../../lelwel" }
logos = "0.16"
codespan-reporting = "0.13"

[build-dependencies]
lelwel-codegen = { path = "../../lelwel-codegen" }
```

### `build.rs` changes
```rust
fn main() {
    lelwel_codegen::build("src/grammar.llw");
}
```

### `parser.rs` changes
- Add `use lelwel::*;` at the top
- Change `Parser<'a>` to `Parser<'a, Token, Rule>` in impl blocks
- Change `ParserCallbacks<'a>` to `ParserCallbacks<'a>` (same name, but now from generated code with different methods)
- Remove local `Span` type alias if it was importing from this module (it's now from `lelwel`)
- The `Diagnostic` type alias stays (it's codespan-reporting specific)

### `lexer.rs` changes
- Change `use super::parser::{Diagnostic, Span};` to `use lelwel::Span;` and keep `Diagnostic` import from codespan
  OR if `Diagnostic` was from `parser`, import it from the appropriate place

Actually, the generated `lexer.rs` currently starts with `use super::parser::{Diagnostic, Span};`. After the change, `Span` comes from `lelwel` and `Diagnostic` is defined in `parser.rs`. But the lexer scaffold is only generated when `lexer.rs` doesn't exist. So we need to update the `gen_lexer` function AND update the existing example `lexer.rs` files.

For existing example `lexer.rs` files:
- Change `use crate::parser::{Diagnostic, Span};` → `use crate::parser::Diagnostic;` + `use lelwel::Span;`

- [ ] **Step 1: Update `examples/calc/Cargo.toml`**
- [ ] **Step 2: Update `examples/calc/build.rs`**
- [ ] **Step 3: Update `examples/calc/src/parser.rs`**
- [ ] **Step 4: Update `examples/calc/src/lexer.rs`**
- [ ] **Step 5: Repeat for all 10 example crates** (json, l, lua, oberon0, python2, toml, wgsl, c, brainfuck)

For each example that has a test, also check if the test needs updating.

- [ ] **Step 6: Verify all examples build**

```bash
cargo build --workspace
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Update all example crates to use lelwel runtime + lelwel-codegen"
```

---

## Task 5: Update tests and verify

**Files:**
- Possibly modify: `tests/frontend.rs`
- Possibly modify: `scripts/update_grammar.sh`
- All example test data if needed

- [ ] **Step 1: Regenerate `tests/frontend.rs`**

```bash
cd tests && bash generate.sh
```

The `tests/frontend.rs` tests the codegen frontend, not the generated runtime code, so they should pass without changes. But regenerate to be safe.

- [ ] **Step 2: Run all tests**

```bash
cargo test --workspace
```

Expected: all tests pass. If any fail, debug and fix.

- [ ] **Step 3: Check for any remaining references to old types**

Search for any remaining references to `lelwel::build` (should be `lelwel_codegen::build`), or types that should now come from the runtime crate.

```bash
rg "lelwel::build" examples/
rg "super::parser::Span" examples/
rg "use super::parser" examples/
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Verify and fix all tests after runtime library migration"
```

---

## Task 6: Update documentation

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README.md Quickstart section**

The Quickstart example needs to reflect:
1. `lelwel` as a runtime dependency, `lelwel-codegen` as a build dependency
2. `use lelwel::*;` in user code
3. `Parser<'a, Token, Rule>` instead of `Parser<'a>`
4. `lelwel_codegen::build(...)` in `build.rs`
5. Updated `ParserCallbacks` trait signature

Update the code examples in README.md accordingly. The dependency section changes from:

```toml
[dependencies]
logos = "0.16"
codespan-reporting = "0.13"

[build-dependencies]
lelwel = "0.10"
```

to:

```toml
[dependencies]
lelwel = "0.10"
logos = "0.16"
codespan-reporting = "0.13"

[build-dependencies]
lelwel-codegen = "0.10"
```

The Rust example code changes to add `use lelwel::*;` and use `Parser<'_, Token, Rule>`.

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "Update README.md for runtime library architecture"
```

---

## Task 7: Final cleanup and verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace --all-features
```

- [ ] **Step 2: Check for clippy warnings**

```bash
cargo clippy --workspace --all-features
```

- [ ] **Step 3: Verify generated code compiles for a specific example**

Pick the `json` example (simplest) and the `c` example (most complex, uses Context):

```bash
cargo build -p lelwel-json
cargo build -p lelwel-c
```

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "Final fixes and cleanup"
```