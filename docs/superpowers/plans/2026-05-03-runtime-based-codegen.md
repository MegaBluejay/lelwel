# Runtime-Based Codegen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modify the lelwel code generator to emit code that uses the `lelwel` runtime crate instead of generating monolithic types.

**Architecture:** The `gen_generated` function currently emits all types inline (Parser, Node, Cst, etc.). We change it to `use lelwel::{...}` and only generate grammar-specific items (Rule enum, TokenType impl, ParserCallbacks trait, Rules trait, etc.). The runtime crate provides the generic Parser, Cst, etc.

**Tech Stack:** Rust, proc-macro2/quote for code generation

---

## Key Design Decisions

1. **`Rules<'a>` trait** has abstract `rule_*` method signatures only (no default `parse` body because `self.end_of_input`/`self.parse_with` can't be expressed on arbitrary `Self: ParserCallbacks<'a>`)
2. **`parse` and `parse_*` methods** go in an inherent `impl` block on `Parser<'a, Token, Rule, Ctx>` where `Self: ParserHooks + Rules<'a>`
3. **`left-recursive rule `rec` functions** become generic over `Ctx` with `where Parser<'b, Token, Rule, Ctx>: ParserHooks<'b, Token, Rule>`
4. **`Rule::Error`** is always the first variant in the Rule enum
5. **`impl TokenType for Token`** is in `generated.rs` (Token is in scope from lexer)
6. **`impl RuleType for Rule`** is in `generated.rs`
7. **User `parser.rs` files** need manual updates (they reference old import paths)
8. **Example projects** need `lelwel` added as a dependency in `Cargo.toml`

---

### Task 1: Modify `gen_generated` to emit runtime-based code

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs` (the `gen_generated` method, lines ~1240-1917)

This is the largest task. The `gen_generated` method must be rewritten to:
1. Add `use lelwel::{TokenType, RuleType, ParserHooks, Parser, NodeRef, CstIndex, Node, Cst, CstChildren, Span, MarkOpened, MarkClosed, MarkTruncation, err};` at the top
2. Generate `impl TokenType for Token` with `is_skip`, `is_eof`, `eof`
3. Generate `Rule` enum with `Error` always first
4. Generate `impl RuleType for Rule`
5. Generate `impl Debug for Rule` (same as current)
6. Generate `ParserCallbacks<'a>` trait (with grammar-specific callbacks)
7. Generate blanket `impl ParserHooks<'a, Token, Rule> for P where P: ParserCallbacks<'a>`
8. Generate `Rules<'a>` trait (abstract rule methods)
9. Generate `impl Rules<'a> for Parser<'a, Token, Rule, Ctx>` (rule method bodies)
10. Generate inherent `impl Parser<'a, Token, Rule, Ctx>` with `parse` and `parse_*`
11. Generate `expect!` and `try_expect!` macros
12. Remove all the old monolithic types (NodeRef, CstIndex, Node, CstData, Cst, CstChildren, Span, MarkOpened, MarkClosed, MarkTruncation, ParserState, Parser struct, impl Parser block, err! macro)

- [ ] **Step 1: Restructure `gen_generated` - add import statement and TokenType impl**

Replace the entire `gen_generated` body. First part: `use` statement and `impl TokenType for Token`.

The `use` statement:
```rust
use lelwel::{TokenType, RuleType, ParserHooks, Parser, NodeRef, CstIndex, Node, Cst, CstChildren, Span, MarkOpened, MarkClosed, MarkTruncation, err};
```

The `impl TokenType for Token`:
```rust
impl TokenType for Token {
    #[inline]
    fn is_skip(&self) -> bool {
        matches!(self, Token::Error | Token::Whitespace | ...)
    }
    #[inline]
    fn is_eof(&self) -> bool {
        matches!(self, Token::EOF | Token::EOFExpr | ...)
    }
    #[inline]
    fn eof() -> Self {
        Token::EOF
    }
}
```

Where `is_skip` includes `Token::Error` plus all skip tokens from `sema.skipped`, and `is_eof` includes `Token::EOF` plus all `Token::EOF*` variants from `sema.parts`.

- [ ] **Step 2: Generate Rule enum with Error first, RuleType impl, Debug impl**

The Rule enum must always have `Error` as the first variant, then other rule names from `rule_names` (excluding `error` since it's already first). The `is_start` rule becomes a special `Part` variant if parts exist.

- [ ] **Step 3: Generate ParserCallbacks trait**

The trait includes `create_tokens`, `create_diagnostic`, `predicate_skip` (from ParserHooks), plus grammar-specific `create_node_*`, `delete_node_*`, `predicate_*`, `action_*`, `assertion_*` methods.

- [ ] **Step 4: Generate blanket impl ParserHooks for ParserCallbacks**

This dispatches `create_node`, `create_node_error`, `delete_node` through match arms on `Rule` variants.

- [ ] **Step 5: Generate Rules trait and impl Rules for Parser**

The `Rules<'a>` trait defines abstract `rule_*` method signatures. The `impl Rules<'a> for Parser<'a, Token, Rule, Ctx>` provides the bodies.

- [ ] **Step 6: Generate parse/parse_* methods as inherent impl on Parser**

The `parse` method and `parse_*` part methods go in an inherent `impl` block on `Parser<'a, Token, Rule, Ctx>` with appropriate bounds.

- [ ] **Step 7: Generate expect!/try_expect! macros**

These remain in generated code referencing `Token::$variant` and `err!` from `lelwel`.

- [ ] **Step 8: Remove old monolithic type emissions**

Remove all the old inline type definitions that are now in the runtime crate.

---

### Task 2: Modify `gen_rule` method signatures

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs` (the `gen_rule` method)

Change the method signature from:
```rust
fn #rule_fn(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> #ret_type
```
to:
```rust
fn #rule_fn(&mut self, diags: &mut Vec<<Self as ParserHooks<'a, Token, Rule>>::Diagnostic>) -> #ret_type
```

This affects `gen_rule` and `gen_left_recursive_rule` which reference `ParserCallbacks` type.

- [ ] **Step 1: Update `gen_rule` to use `ParserHooks` diagnostic type**

- [ ] **Step 2: Update `gen_left_recursive_rule` - change `rec` function signature**

Change `Parser<'a>` to `Parser<'b, Token, Rule, Ctx>` in the `rec` local function, with generic `'b` and `Ctx` and `where Parser<'b, Token, Rule, Ctx>: ParserHooks<'b, Token, Rule>` bound.

- [ ] **Step 3: Update `gen_recovering_operation` and `gen_regex` to use `ParserHooks` diagnostic type**

Where `<<Self as ParserCallbacks<'a>>::Diagnostic>` appears, change to `<<Self as ParserHooks<'a, Token, Rule>>::Diagnostic>`.

---

### Task 3: Modify `gen_parser` (scaffold `parser.rs`)

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs` (the `gen_parser` method)

Change the scaffold to import from `lelwel` and use `Parser<'a, Token, Rule, ()>`.

- [ ] **Step 1: Update `gen_parser` imports and scaffold structure**

Add `use lelwel::{ParserHooks, Parser, Span, Cst};` and keep existing imports. The `impl ParserCallbacks<'a> for Parser<'a, Token, Rule, ()>` uses `Parser<'a, Token, Rule, ()>`.

---

### Task 4: Modify `gen_parser_callbacks` for new trait shape

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs` (the `gen_parser_callbacks` method)

Both `is_trait=true` and `is_trait=false` cases need updating.

- [ ] **Step 1: Update `is_trait=true` (trait definition) case**

Add `create_tokens`, `create_diagnostic`, `predicate_skip` from ParserHooks. Change `ParserCallbacks` trait to use concrete `Token` and `Rule` types.

- [ ] **Step 2: Update `is_trait=false` (scaffold impl) case**

Change `Parser<'a>` to `Parser<'a, Token, Rule, ()>`. Use `ParserCallbacks<'a>` trait. Keep default implementations for predicates/actions/assertions with `todo!()`.

---

### Task 5: Modify `gen_lexer` to update imports

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs` (the `gen_lexer` method)

Change `use super::parser::{Diagnostic, Span};` to `use super::parser::Diagnostic;` and `use lelwel::Span;`.

- [ ] **Step 1: Update `gen_lexer` import statements**

---

### Task 6: Modify `gen_parts` and `gen_cst_close` for new types

**Files:**
- Modify: `lelwel-codegen/src/backend/rust.rs`

- [ ] **Step 1: Update `gen_parts` - change return type from `Cst<'a>` to `Cst<'a, Token, Rule>`, change `parse_rule` to `parse_with`**

- [ ] **Step 2: Update `gen_cst_close` - ensure it references `ParserHooks` for `create_node_error` dispatch if needed**

Currently `gen_cst_close` generates `parser.create_node_error(NodeRef(closed.0), diags)` or `#parser_name.create_node_error(...)`. In the new design, `create_node_error` is a method on `Parser` that calls `ParserHooks::create_node_error`, which dispatches via blanket impl to `ParserCallbacks::create_node_error`. The generated code stays the same since `self.create_node_error()` is resolved through the trait.

---

### Task 7: Update an example project and test compilation

**Files:**
- Modify: `examples/calc/Cargo.toml` (add `lelwel` dependency)
- Modify: `examples/calc/src/parser.rs` (update imports and impl)
- Modify: `examples/calc/src/lexer.rs` (update imports)

- [ ] **Step 1: Add `lelwel` dependency to `examples/calc/Cargo.toml`**

Add: `lelwel = { path = "../../lelwel" }`

- [ ] **Step 2: Update `examples/calc/src/parser.rs` for new API**

Change `use crate::lexer::{Token, tokenize};` to keep, add `use lelwel::{ParserHooks, Parser, Span, Cst};`, change `impl<'a> ParserCallbacks<'a> for Parser<'a>` to use `Parser<'a, Token, Rule, ()>`.

- [ ] **Step 3: Update `examples/calc/src/lexer.rs` imports**

Change `use crate::parser::{Diagnostic, Span};` to `use crate::parser::Diagnostic;` and `use lelwel::Span;`.

- [ ] **Step 4: Build the calc example and fix any compilation errors**

Run: `cargo build -p lelwel-calc`

- [ ] **Step 5: Verify generated code by inspecting `target/debug/build/lelwel-calc-*/out/generated.rs`**

Run the build, then check the generated code looks correct.

---

### Task 8: Update remaining example projects

**Files:**
- All example `Cargo.toml`, `parser.rs`, and `lexer.rs` files

- [ ] **Step 1: Update each example project similarly to the calc example**

For each example: add `lelwel` dependency, update `parser.rs` and `lexer.rs` imports.

- [ ] **Step 2: Build all examples and verify they compile**

Run: `cargo build` for each example.

---

### Task 9: Run test suite

- [ ] **Step 1: Run `cargo test` from the workspace root**

Verify all tests pass after the changes.

- [ ] **Step 2: Fix any failing tests**

## Important Implementation Notes

### Blanket impl `create_node` dispatch

The `create_node` method in the blanket impl dispatches to grammar-specific `create_node_*` methods:
```rust
fn create_node(&mut self, rule: Rule, node_ref: NodeRef, diags: &mut Vec<Self::Diagnostic>) {
    match rule {
        Rule::BinaryExpr => self.create_node_binary_expr(node_ref, diags),
        Rule::Error => self.create_node_error(node_ref, diags),
        // ... all rule variants
    }
}
```

Every rule in `rule_names` gets a match arm, including `Error` which dispatches to `create_node_error`.

### Blanket impl `delete_node` dispatch

The `delete_node` method only has arms for rules used in ordered choices:
```rust
fn delete_node(&mut self, rule: Rule, _node_ref: NodeRef) {
    match rule {
        // ... only rules with in_choice=true
        _ => {}
    }
}
```

If ALL rules are in ordered choices, no `_ => {}` wildcard is needed.

### `rec` function in left-recursive rules

The `rec` local function must be generic over lifetime and Ctx:
```rust
fn rec<'b, Ctx>(
    parser: &mut Parser<'b, Token, Rule, Ctx>,
    diags: &mut Vec<<Parser<'b, Token, Rule, Ctx> as ParserHooks<'b, Token, Rule>>::Diagnostic>,
    min_bp: usize,
    mut lhs: MarkClosed,
) -> Option<()> 
where
    Parser<'b, Token, Rule, Ctx>: ParserHooks<'b, Token, Rule>,
{
```

### `impl Display for Rule` → `impl Debug for Rule`

Keep the current `Debug` impl for `Rule` (not `Display`). It maps pascal-case variants to their snake-case string representations.

### `Predicate_skip` in `ParserCallbacks`

The `predicate_skip` method takes `Token` (concrete type) instead of generic `T`:
```rust
fn predicate_skip(&self, _token: Token) -> bool { false }
```

This is because `ParserCallbacks` is grammar-specific and knows the concrete `Token` type.