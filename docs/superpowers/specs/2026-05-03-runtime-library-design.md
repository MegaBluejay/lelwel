# Lelwel Runtime Library Design

## Goal

Extract the static infrastructure from the generated `generated.rs` into a reusable runtime crate (`lelwel`), so that grammar-specific generated code is as small as possible and the bulk of the parser machinery lives in a versioned library.

## Crate Structure

Root `Cargo.toml` is workspace-only (no root crate). Three categories of members:

| Crate | Path | Purpose | Dependencies |
|-------|------|---------|--------------|
| `lelwel` | `lelwel/` | Runtime library (generic parser, CST, traits) | none (or minimal) |
| `lelwel-codegen` | `lelwel-codegen/` | Code generator (moved from current `src/`) | `quote`, `proc-macro2`, `syn`, `prettyplease`, `logos`, `codespan-reporting`, `rustc-hash`, `clap` (opt), etc. |
| `examples/*` | `examples/` | Unchanged structure | `lelwel` (dep), `lelwel-codegen` (build-dep) |

The current `lelwel` crate (the code generator) is renamed to `lelwel-codegen` and moved to `lelwel-codegen/`. A new `lelwel/` crate is created for the runtime library.

## Core Traits

### `TokenType` (runtime crate)

Grammar-agnostic interface for token enumeration. Replaces the current pattern matches on `Token::Error | Token::Whitespace` etc.

```rust
pub trait TokenType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn is_skip(&self) -> bool;
    fn is_eof(&self) -> bool;
    fn eof() -> Self;
}
```

The generated `impl TokenType for Token` matches declared skip tokens and `Error` in `is_skip()`, all EOF variants in `is_eof()`, and returns `Token::EOF` from `eof()`.

### `RuleType` (runtime crate)

Grammar-agnostic interface for rule enumeration.

```rust
pub trait RuleType: Copy + Clone + PartialEq + Eq + Debug + 'static {
    fn error() -> Self;
}
```

The generated `impl RuleType for Rule` returns `Rule::Error` from `error()`.

### `ParserHooks` (runtime crate, internal)

The interface the generic `Parser<_>` depends on. Not intended for direct user implementation — users implement `ParserCallbacks` instead and a blanket impl provides `ParserHooks`.

```rust
pub trait ParserHooks<'a, T: TokenType, R: RuleType> {
    type Diagnostic;
    type Context;

    fn create_tokens(ctx: &mut Self::Context, source: &'a str, diags: &mut Vec<Self::Diagnostic>) -> (Vec<T>, Vec<Span>);
    fn create_diagnostic(&self, span: Span, message: String) -> Self::Diagnostic;
    fn predicate_skip(&self, token: T) -> bool;
    fn create_node(&mut self, rule: R, node_ref: NodeRef, diags: &mut Vec<Self::Diagnostic>);
    fn create_node_error(&mut self, node_ref: NodeRef, diags: &mut Vec<Self::Diagnostic>);
    fn delete_node(&mut self, rule: R, node_ref: NodeRef);
}
```

### `ParserCallbacks` (generated code)

The user-facing trait. Includes all user-overridable methods. Since it's generated per-grammar, `Token` and `Rule` are concrete types in scope. The grammar-specific callback methods (`predicate_*`, `action_*`, `assertion_*`, `create_node_*`, `delete_node_*`) have defaults (empty body or `todo!()`).

```rust
pub trait ParserCallbacks<'a> {
    type Diagnostic;
    type Context;

    // User-overridable hooks (forwarded by blanket impl to ParserHooks)
    fn create_tokens(ctx: &mut Self::Context, source: &'a str, diags: &mut Vec<Self::Diagnostic>) -> (Vec<Token>, Vec<Span>);
    fn create_diagnostic(&self, span: Span, message: String) -> Self::Diagnostic;
    fn predicate_skip(&self, token: Token) -> bool { false }

    // Grammar-specific callbacks with defaults
    fn create_node_x(&mut self, _node_ref: NodeRef, _diags: &mut Vec<Self::Diagnostic>) {}
    fn delete_node_x(&mut self, _node_ref: NodeRef) {}
    fn predicate_y(&self) -> bool { todo!() }
    fn action_z(&mut self, _diags: &mut Vec<Self::Diagnostic>) { todo!() }
    fn assertion_w(&self) -> Option<Self::Diagnostic> { todo!() }
}
```

### Blanket `ParserHooks` impl (generated code)

```rust
impl<'a, P> ParserHooks<'a, Token, Rule> for P
where
    P: ParserCallbacks<'a>,
{
    type Diagnostic = <P as ParserCallbacks<'a>>::Diagnostic;
    type Context = <P as ParserCallbacks<'a>>::Context;

    fn create_tokens(ctx: &mut Self::Context, source: &'a str, diags: &mut Vec<Self::Diagnostic>) -> (Vec<Token>, Vec<Span>) {
        <P as ParserCallbacks<'a>>::create_tokens(ctx, source, diags)
    }
    fn create_diagnostic(&self, span: Span, message: String) -> Self::Diagnostic {
        <P as ParserCallbacks<'a>>::create_diagnostic(self, span, message)
    }
    fn predicate_skip(&self, token: Token) -> bool {
        <P as ParserCallbacks<'a>>::predicate_skip(self, token)
    }
    fn create_node(&mut self, rule: Rule, node_ref: NodeRef, diags: &mut Vec<Self::Diagnostic>) {
        match rule {
            Rule::X => self.create_node_x(node_ref, diags),
            // ... all rules with @create annotations ...
        }
    }
    fn create_node_error(&mut self, node_ref: NodeRef, diags: &mut Vec<Self::Diagnostic>) {
        // generated error node logic
    }
    fn delete_node(&mut self, rule: Rule, node_ref: NodeRef) {
        match rule {
            Rule::X => self.delete_node_x(node_ref),
            // ... rules used in ordered choices ...
            _ => {}
        }
    }
}
```

Key point: `create_node`, `create_node_error`, and `delete_node` dispatch to grammar-specific methods (`create_node_x`, etc.) defined on `ParserCallbacks`. The blanket impl provides the routing; `ParserCallbacks` provides the per-rule endpoints (with defaults the user can override).

### `Rules` trait (generated code)

Since `Parser` is defined in the runtime crate, generated code cannot add inherent methods to it. The `Rules` trait provides all grammar-specific methods as trait methods:

```rust
pub trait Rules<'a>: ParserCallbacks<'a> + Sized {
    fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
    fn parse_expr(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
    // ... part parse methods ...

    fn rule_start(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_expr(&mut self, diags: &mut Vec<Self::Diagnostic>);
    // ... all rule methods ...
}

impl<'a> Rules<'a> for Parser<'a, Token, Rule> {
    fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
        self.parse_with(|p, d| p.rule_start(d), diags, Rule::Start, Token::EOF)
    }
    fn rule_start(&mut self, diags: &mut Vec<Self::Diagnostic>) { /* ... */ }
    fn rule_expr(&mut self, diags: &mut Vec<Self::Diagnostic>) { /* ... */ }
    // ...
}
```

The `parse` default implementation does NOT live in the trait — it's a concrete impl because it references concrete `Rule::Start` and `Token::EOF`. The `parse_with` method is on `Parser` in the runtime crate and takes a start-rule closure.

## Generic Types

All CST and parser infrastructure types become generic over `T: TokenType` and `R: RuleType`:

| Current type | New type |
|---|---|
| `Node` | `Node<T, R>` |
| `CstData` | `CstData<T, R>` |
| `Cst<'a>` | `Cst<'a, T, R>` |
| `CstChildren<'a>` | `CstChildren<'a, T, R>` |
| `Parser<'a>` | `Parser<'a, T, R>` |
| `ParserState` | `ParserState<T>` |
| `NodeRef` | `NodeRef` (unchanged) |
| `CstIndex` | `CstIndex` (unchanged) |
| `Span` | `Span` (unchanged) |
| `MarkOpened` | `MarkOpened` (unchanged) |
| `MarkClosed` | `MarkClosed` (unchanged) |
| `MarkTruncation` | `MarkTruncation` (unchanged) |

`ParserState` needs `T` because it stores `current: T`.

`CstChildren` needs `T, R` because it iterates over `Node<T, R>`.

## Method Changes in `impl Parser`

### `advance` / `init_skip` (runtime crate)

Current code matches on `Token::Error | Token::Whitespace` etc. Replaced with `TokenType::is_skip()`:

```rust
fn advance(&mut self, diags: &mut Vec<...>) {
    // ...
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
```

### `is_skipped` (removed)

Replaced by `TokenType::is_skip()`. No more `Parser::is_skipped` static method.

### `create_node` / `delete_node` (removed from `impl Parser`)

These become `ParserHooks::create_node` / `ParserHooks::delete_node`, dispatched by the blanket impl. The runtime `Parser` calls `self.create_node(rule, ...)` and `self.delete_node(rule, ...)` which resolves via the `ParserHooks` blanket impl.

### `create_node_error` (removed from `impl Parser`)

Same pattern — becomes `ParserHooks::create_node_error`.

### `new` / `new_with_context` (runtime crate)

Use `T::eof()` for the initial `end_of_input` field and `Self::create_tokens()` for tokenization. The `end_of_input` field is a public field that `Rules::parse*` entry points override to the grammar-specific EOF variant before calling `parse_with`.

Sequence: constructor creates `Parser` with `end_of_input: T::eof()` → `Rules::parse*` sets `self.end_of_input = Token::EOFExpr` (or whatever) → calls `self.parse_with(...)` which calls `init_skip()`.

```rust
pub fn new_with_context(source: &'a str, diags: &mut Vec<...>, context: <Self as ParserHooks>::Context) -> Self {
    let (tokens, spans) = Self::create_tokens(&mut context, source, diags);
    Self {
        current: T::eof(),
        end_of_input: T::eof(),
        cst: Cst { data: CstData::new(spans), source },
        tokens,
        pos: 0,
        max_offset: source.len(),
        context,
        error_node: None,
        in_ordered_choice: false,
        error_since_advance: false,
    }
}
```

### `parse_with` (runtime crate, new method)

New generic entry point replacing the old `parse_rule`:

```rust
pub fn parse_with(
    mut self,
    start_rule: impl FnOnce(&mut Self, &mut Vec<...>),
    diags: &mut Vec<...>,
    root: R,
) -> Cst<'a, T, R> {
    let token_count = self.tokens.len();
    let m = self.open(diags);
    self.init_skip();
    start_rule(&mut self, diags);
    self.close_error_node(diags);
    // ... trailing error handling ...
    let closed = self.cst.data.close_root(m, root);
    self.create_node(root, NodeRef(closed.0), diags);
    self.cst
}
```

This is called by `Rules::parse` and `Rules::parse_*`.

### `parse` (removed from runtime `impl Parser`)

No longer a method on `Parser`. The `Rules` trait provides `parse(self, diags)` which calls `parse_with`.

## Macros

### `err!` macro (runtime crate)

Currently:
```rust
macro_rules! err {
    [$self:expr, $msg:literal] => {
        $self.create_diagnostic($self.span(), String::from($msg))
    }
}
```

Remains in generated code because it references `create_diagnostic` which is resolved via `ParserHooks` blanket impl. Actually, since `ParserHooks` is in the runtime crate and `err!` calls a method on `self`, it could live in the runtime crate. But the macro is used in generated rule code, so it needs to be in scope. It could be exported from the runtime crate.

### `expect!` / `try_expect!` macros (generated code)

These reference `Token::$variant`, so they remain grammar-specific and in generated code:

```rust
macro_rules! expect {
    ($token:ident, $msg:literal, $self:expr, $diags:expr) => {
        if let Token::$token = $self.current {
            $self.advance(false, $diags);
        } else {
            $self.error($diags, err![$self, $msg]);
        }
    };
}
```

## What Lives Where

### Runtime crate (`lelwel`)

| Item | Notes |
|------|-------|
| `TokenType` trait | `is_skip()`, `is_eof()`, `eof()` |
| `RuleType` trait | `error()` |
| `ParserHooks` trait | Associated types + hook methods |
| `Parser<'a, T, R>` struct | Generic over T and R |
| `impl<'a, T, R> Parser` (with `Self: ParserHooks` bound) | All runtime methods (advance, open, close, error, etc.) |
| `Node<T, R>` | Generic CST node |
| `CstData<T, R>` | Mutable CST builder |
| `Cst<'a, T, R>` | Immutable CST view |
| `CstChildren<'a, T, R>` | CST children iterator |
| `NodeRef`, `CstIndex` | Unchanged |
| `Span` | Unchanged (`core::ops::Range<usize>`) |
| `MarkOpened`, `MarkClosed`, `MarkTruncation` | Unchanged |
| `ParserState<T>` | Has `current: T` field |
| `err!` macro | Can be in runtime crate |

### Generated code (`generated.rs`)

| Item | Notes |
|------|-------|
| `Token` enum | Grammar-specific, from lexer |
| `impl TokenType for Token` | Generated from grammar |
| `Rule` enum | Grammar-specific variants |
| `impl RuleType for Rule` | Returns `Rule::Error` |
| `ParserCallbacks<'a>` trait | User-overridable methods + grammar callbacks with defaults |
| `impl<'a, P> ParserHooks<'a, Token, Rule> for P where P: ParserCallbacks<'a>` | Blanket impl |
| `Rules<'a>` trait | `parse`, `parse_*`, `rule_*` methods |
| `impl<'a> Rules<'a> for Parser<'a, Token, Rule>` | Generated rule logic |
| `impl<'a> ParserCallbacks<'a> for Parser<'a, Token, Rule>` | Default impl (scaffold for user) |
| `expect!` / `try_expect!` macros | Reference `Token` variants |
| `impl Display for Rule` | Debug formatting |

### No longer in generated code (moved to runtime)

All of the CST infrastructure: `NodeRef`, `CstIndex`, `Node`, `MarkOpened`, `MarkClosed`, `MarkTruncation`, `CstChildren`, `CstData`, `Cst`, `ParserState`, `Parser` struct definition, `Span`, `err!` macro.

### User's `parser.rs` (unchanged pattern)

```rust
use lelwel::*;
use crate::lexer::{Token, tokenize};
use codespan_reporting::diagnostic::Label;

pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

impl<'a> ParserCallbacks<'a> for Parser<'a, Token, Rule> {
    type Diagnostic = Diagnostic;
    type Context = ();

    fn create_tokens(_ctx: &mut Self::Context, source: &'a str, diags: &mut Vec<Diagnostic>) -> (Vec<Token>, Vec<Span>) {
        tokenize(source, diags)
    }
    fn create_diagnostic(&self, span: Span, msg: String) -> Diagnostic {
        Diagnostic::error().with_message(msg).with_label(Label::primary((), span))
    }
    // + any grammar-specific overrides
}
```

Note: the user now needs `use lelwel::*;` to import the runtime types.

### User's `lexer.rs` (minor changes)

The `Token` enum needs `impl TokenType` — this is generated in `generated.rs`, so the user doesn't write it manually. The user's `lexer.rs` changes: `Span` is now imported from the `lelwel` crate instead of `parser.rs`, and `Diagnostic` continues to come from `codespan_reporting`.

### `build.rs` changes

```rust
fn main() {
    lelwel_codegen::build("src/grammar.llw");
}
```

Changes from `lelwel::build` to `lelwel_codegen::build`.

### Example `Cargo.toml` changes

```toml
[dependencies]
lelwel = { path = "../.." }
logos = "0.16"
codespan-reporting = "0.13"

[build-dependencies]
lelwel-codegen = { path = "../../lelwel-codegen" }
```

## Edge Cases and Details

### Left-recursive rules and `rec` helper

The `rec` function for left-recursive rules currently references `Parser<'a>` and `ParserCallbacks<'a>` types. In the new design, `rec` is defined inside the `Rules` impl block as a local function, referencing `Parser<'a, Token, Rule>` directly. No changes needed beyond the generic type parameters.

### EOF variants for part rules

Currently `Token::EOFExpr` etc. are grammar-specific. The `Rules::parse_expr` method sets `self.end_of_input = Token::EOFExpr` before calling `parse_with`. `TokenType::is_eof()` returns true for all EOF variants including the grammar-specific ones. The generic `TokenType::eof()` provides the default `Token::EOF` for the constructor.

### `Diagnostic` type in `advance` and other runtime methods

Runtime methods like `advance(&mut self, diags: &mut Vec<<Self as ParserHooks<'a, T, R>>::Diagnostic>)` use the associated type from `ParserHooks`. These verbose signatures are internal to the runtime crate and not seen by users.

### `Cst` match methods

`CstData::match_token` and `CstData::match_rule` take concrete `Token` and `Rule` parameters. In the generic version, these become `T` and `R` parameters, which is straightforward.

### Visibility

`Parser` fields like `cst`, `current`, `pos`, `tokens` need to remain accessible for:
1. `ParserCallbacks` implementations (e.g., `self.peek()`, `self.cst.children()`)
2. `Rules` impl (generated code accesses these)
3. `rec` helper functions

All these are in the same crate (the user's crate) as the `impl Rules` and `impl ParserCallbacks`, so `pub` fields suffice. The runtime crate can keep `Parser` fields `pub` as they are today.

### Generated scaffold for `impl ParserCallbacks`

The code generator currently writes a scaffold `impl ParserCallbacks for Parser` when `parser.rs` doesn't exist yet. This scaffold includes `create_tokens`, `create_diagnostic`, and default bodies for predicates/actions/assertions. This continues — the scaffold is still generated, just now implementing `ParserCallbacks<'a>` instead of `ParserCallbacks<'a>` (same trait name, different shape).

### Generated lexer

The generated `lexer.rs` is mostly unchanged. It defines `Token` and `tokenize`. After the change, `impl TokenType for Token` would be generated alongside `Token` in `generated.rs` (or in `lexer.rs`). Since `Token` is defined in `lexer.rs` and `impl TokenType for Token` needs to be in scope for `Parser<..., Token, ...>`, it should be in `lexer.rs` or `generated.rs`. Best to put it in `generated.rs` alongside `Rule` and the traits, since it's generated from grammar info.

Wait — but `lexer.rs` is hand-written (or scaffolded once). The user may modify `Token`. The `impl TokenType for Token` needs to match `Token`'s variants. If the user adds a skip token by hand, they need to update `is_skip()`. This is a potential footgun.

Decision: generate `impl TokenType for Token` in `generated.rs`. It's derived from grammar declarations and matches what the parser expects. Users who modify `Token` also need to update the `impl TokenType` — this is analogous to the current situation where modifying `Token` requires the grammar to match.

## Migration Summary

1. Create `lelwel/` directory for the runtime crate
2. Move current `src/` to `lelwel-codegen/src/` (rename package to `lelwel-codegen`)
3. Create `lelwel/src/lib.rs` with all the generic types and traits
4. Update `lelwel-codegen/src/backend/rust.rs` to generate the new structure
5. Update root `Cargo.toml` to workspace-only
6. Update all example `Cargo.toml` and `build.rs` files
7. Update example `parser.rs` and `lexer.rs` files for new imports and types
8. Ensure all tests pass