# Quote/proc_macro2 Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all string-based code generation in `src/backend/rust.rs` with `quote`/`proc_macro2` token stream generation.

**Architecture:** Bottom-up incremental migration. Each code generator method changes from writing to `BufWriter<File>` to returning `proc_macro2::TokenStream`. Old and new methods coexist during migration with `gen_` prefix for new ones. The top-level `RustOutput::run()` collects token streams, parses with `syn::parse_file()`, formats with `prettyplease::unparse()`, and writes to disk.

**Tech Stack:** `quote`, `proc_macro2`, `syn`, `prettyplease`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Add new dependencies |
| `src/backend/rust.rs` | All code generation (single file, ~1500 lines) |
| `src/skeleton/generated.rs` | Template for generated parser (will be absorbed into rust.rs) |
| `src/skeleton/parser.rs` | Template for parser.rs (will be absorbed into rust.rs) |
| `src/skeleton/lexer.rs` | Template for lexer.rs (will be absorbed into rust.rs) |

---

### Task 1: Add dependencies and parse+format pipeline

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/backend/rust.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

Add to `[dependencies]` section in `Cargo.toml`:
```toml
quote = "1"
proc-macro2 = "1"
syn = { version = "2", features = ["full", "parsing"] }
prettyplease = "0.2"
```

- [ ] **Step 2: Add imports to rust.rs**

Add at the top of `src/backend/rust.rs`:
```rust
use proc_macro2::TokenStream;
use quote::quote;
use syn;
```

- [ ] **Step 3: Add parse+format helper function**

Add a helper function in `src/backend/rust.rs` that parses a string into a `syn::File` and formats it:
```rust
fn format_rust_source(source: &str) -> String {
    let file = syn::parse_file(source).expect("generated code should be valid Rust");
    prettyplease::unparse(&file)
}
```

- [ ] **Step 4: Integrate parse+format into RustOutput::run**

In `RustOutput::run()`, change the `output_generated` path to collect the output into a string first, then parse and format it before writing. Currently `output_generated` writes directly to a `BufWriter<File>`. Change the flow:

Instead of writing `generated.rs` directly via BufWriter, collect all output into a `String`, then:
```rust
let generated_source = // collected output string
let formatted = format_rust_source(&generated_source);
generated_file.write_all(formatted.as_bytes())?;
```

The simplest approach: change `output_generated` and all its callees to write to a `String` instead of `BufWriter<File>`, then format and write. To do this with minimal changes, replace the `BufWriter<File>` with a `Vec<u8>` (which also implements `Write`), then convert to string for formatting.

In `RustOutput::run()`:
```rust
let mut generated_buf: Vec<u8> = Vec::new();
{
    let mut generated_file = BufWriter::new(&mut generated_buf);
    Self::output_generated(cst, sema, file, &mut generated_file)?;
}
let generated_source = String::from_utf8(generated_buf).unwrap();
let formatted = format_rust_source(&generated_source);
let mut generated_file = BufWriter::new(std::fs::File::create(output.join("generated.rs"))?);
generated_file.write_all(formatted.as_bytes())?;
```

Note: `BufWriter<&mut Vec<u8>>` implements `Write`, so all the existing `output_` methods still work without changes.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass. If any fail, it's because `prettyplease` reformatted the generated code in a way that changes runtime semantics (unlikely). If failures occur, investigate and fix.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/backend/rust.rs
git commit -m "Add quote/proc_macro2/syn/prettyplease deps and parse+format pipeline"
```

---

### Task 2: Add gen_ helpers alongside existing helpers

**Files:**
- Modify: `src/backend/rust.rs`

This task adds new `gen_` helper functions that return `TokenStream` alongside the existing string-based helpers. The old helpers remain until all callers are migrated.

- [ ] **Step 1: Add gen_pattern function**

The existing `Generator::pattern()` returns a `String` like `"Token::A\n| Token::B"`. The new version returns a `TokenStream`:

```rust
fn gen_pattern(tokens: &BTreeSet<TokenName<'_>>) -> TokenStream {
    let idents: Vec<_> = tokens.iter().map(|s| {
        let ident = proc_macro2::Ident::new(s.0, proc_macro2::Span::call_site());
        quote!(Token::#ident)
    }).collect();
    quote!(#(#idents)|*)
}
```

- [ ] **Step 2: Add gen_error_message function**

The existing `Generator::error()` returns a formatted error message string. The new version returns a `TokenStream` (a string literal):

```rust
fn gen_error_message<'a>(
    tokens: &BTreeSet<TokenName<'a>>,
    token_symbols: &FxHashMap<&str, &str>,
) -> TokenStream {
    let expected: Vec<_> = tokens
        .iter()
        .filter_map(|s| token_symbols.get(s.0.as_ref()))
        .map(|sym| {
            if sym.starts_with('<') && sym.ends_with('>') && sym.len() > 2 {
                sym.to_string()
            } else {
                format!("'{}'", sym)
            }
        })
        .collect();
    gen_syntax_error_message(&expected)
}

fn gen_syntax_error_message(expected: &[String]) -> TokenStream {
    let msg = if expected.is_empty() {
        "invalid syntax".to_string()
    } else if expected.len() == 1 {
        format!("invalid syntax, expected: {}", expected[0])
    } else {
        format!("invalid syntax, expected one of: {}", expected.join(", "))
    };
    let escaped = escape(&msg);
    quote!(#escaped)
}
```

- [ ] **Step 3: Add self_ident and parser_ident helper functions**

Since `parser_name` was previously a `&str` parameter (either `"self"` or `"parser"`), it now becomes an `Ident`. Provide helper functions:

```rust
fn self_ident() -> proc_macro2::Ident {
    quote::format_ident!("self")
}

fn parser_ident() -> proc_macro2::Ident {
    quote::format_ident!("parser")
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass. These are new functions, not yet called.

- [ ] **Step 5: Commit**

```bash
git add src/backend/rust.rs
git commit -m "Add gen_ helper functions alongside existing string-based helpers"
```

---

### Task 3: Convert output_parser_callbacks to gen_parser_callbacks

**Files:**
- Modify: `src/backend/rust.rs`

This is a self-contained method that generates the `ParserCallbacks` trait or impl block. It doesn't call other generator methods, so it's a good first conversion target.

- [ ] **Step 1: Write gen_parser_callbacks function**

Create `gen_parser_callbacks` that returns `TokenStream` instead of writing to BufWriter. The method currently:
1. Writes a trait or impl header
2. Writes standard method bodies (create_tokens, create_diagnostic, predicate_skip)
3. Writes per-rule node creation methods (if trait)
4. Writes per-rule node deletion methods (if trait)
5. Writes predicate method declarations/definitions
6. Writes action method declarations/definitions
7. Writes assertion method declarations/definitions
8. Writes closing `}`

For the trait case, generate:
```rust
quote! {
    #[allow(clippy::ptr_arg)]
    pub trait ParserCallbacks<'a> {
        type Diagnostic;
        type Context;
        // ... methods
    }
}
```

For the impl case, generate similarly. Dynamic method names use `quote::format_ident!`.

Key conversions:
- `format!("create_node_{rule_name}")` → `quote::format_ident!("create_node_{}", rule_name)`
- `format!("delete_node_{rule_name}")` → `quote::format_ident!("delete_node_{}", rule_name)`
- `format!("predicate_{rule}_{num}")` → `quote::format_ident!("predicate_{}_{}", rule, num)`
- `format!("action_{rule}_{num}")` → `quote::format_ident!("action_{}_{}", rule, num)`
- `format!("assertion_{rule}_{num}")` → `quote::format_ident!("assertion_{}_{}", rule, num)`

The `is_trait` flag determines whether we generate a trait with `;` bodies or an impl with `todo!()` bodies. Handle this by building method bodies conditionally.

- [ ] **Step 2: Wire gen_parser_callbacks into output_generated**

In `output_generated`, replace the call to `Self::output_parser_callbacks(output, ...)` with:
```rust
let callbacks_tokens = Self::gen_parser_callbacks(sema, is_trait, rule_names);
output.write_all(callbacks_tokens.to_string().as_bytes())?;
```

This is the bridge pattern: `gen_` returns `TokenStream`, caller converts to string and writes.

- [ ] **Step 3: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/backend/rust.rs
git commit -m "Convert output_parser_callbacks to gen_parser_callbacks returning TokenStream"
```

---

### Task 4: Convert simple code generators (elision, node_kind, cst_close, parts)

**Files:**
- Modify: `src/backend/rust.rs`

Convert these methods from `output_xxx(..., &mut BufWriter) -> io::Result<()>` to `gen_xxx(...) -> TokenStream`:
- `output_node_kind_decl` → `gen_node_kind_decl`
- `output_cst_close` → `gen_cst_close`
- `output_elision_init` → `gen_elision_init`
- `output_elision_check` → `gen_elision_check`
- `output_parts` → `gen_parts`

These are the simplest generators — they produce small, self-contained code fragments.

- [ ] **Step 1: Convert gen_node_kind_decl**

Current code:
```rust
fn output_node_kind_decl(output, has_rule_rename, name, level, is_decl) -> io::Result<()> {
    if has_rule_rename {
        output.write_all(format!(
            "{}node_kind = Rule::{};\n",
            if is_decl { "let mut " } else { "" },
            snake_to_pascal_case(name),
        ).indent(level).as_bytes())?;
    }
    Ok(())
}
```

New code:
```rust
fn gen_node_kind_decl(has_rule_rename: bool, name: &str, is_decl: bool) -> TokenStream {
    if !has_rule_rename {
        return TokenStream::new();
    }
    let rule_variant = snake_to_pascal_case_ident(name);
    if is_decl {
        quote! { let mut node_kind = Rule::#rule_variant; }
    } else {
        quote! { node_kind = Rule::#rule_variant; }
    }
}
```

Where `snake_to_pascal_case_ident` is:
```rust
fn snake_to_pascal_case_ident(name: &str) -> proc_macro2::Ident {
    quote::format_ident!("{}", snake_to_pascal_case(name))
}
```

- [ ] **Step 2: Convert gen_cst_close**

Current code writes a formatted block with `close`/`close_root`, `create_node`, and optional `lhs = closed;`. Convert to a `TokenStream` using the same logic. `parser_name` becomes an `Ident` parameter.

- [ ] **Step 3: Convert gen_elision_init and gen_elision_check**

These produce code based on `RuleNodeElision` enum variants. Each variant produces different code. Convert the match arms to `quote!` blocks.

- [ ] **Step 4: Convert gen_parts**

This generates a public method like `pub fn parse_{name}(mut self, diags: &mut Vec<Diagnostic>) -> Cst<'a>`. Convert `name` to ident with `format_ident!`.

- [ ] **Step 5: Wire all converted generators into their callers**

Update `output_normal_rule`, `output_left_recursive_rule`, `output_rule`, and `output_generated` to call the `gen_` versions and write the TokenStream to the BufWriter via the bridge pattern.

- [ ] **Step 6: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/backend/rust.rs
git commit -m "Convert simple code generators (elision, node_kind, cst_close, parts) to TokenStream"
```

---

### Task 5: Convert output_regex and output_recovering_operation

**Files:**
- Modify: `src/backend/rust.rs`

This is the largest and most complex conversion (~460 lines combined). The method dispatches on `Regex` enum variants and recursively generates code.

- [ ] **Step 1: Convert gen_recovering_operation**

This is called by Star/Plus/Optional handling. It generates a `loop { match ... { ... } }` pattern with recovery logic. Key elements:
- `gen_pattern()` for first/follow/recovery sets
- `gen_error_message()` for error messages
- `parser_name` as `Ident`
- The `ordered_choice_return` and `is_loop` parameters control which code paths are generated

Build the match arms and loop structure with `quote!`. For the recovery set, conditionally include a match arm using `if !recovery.is_empty()`.

- [ ] **Step 2: Convert Regex::Name arm**

Generates either a rule call (`parser.rule_{name}(diags)`) or an `expect!`/`try_expect!` macro call. Uses `format_ident!("rule_{}", name)` and `format_ident!("{}", token_name)`.

- [ ] **Step 3: Convert Regex::Symbol arm**

Similar to Name but for token symbols. Generates `expect!`/`try_expect!` macro calls.

- [ ] **Step 4: Convert simple statement arms**

Convert these straightforward arms:
- `Action` → method call: `parser.action_{rule}_{num}(diags);`
- `Assertion` → if-let with method call
- `NodeRename` → assignment: `node_kind = Rule::PascalName;`
- `NodeElision` → assignment: `elide = true;`
- `NodeMarker` → let binding: `let m{number} = parser.mark(diags);`
- `NodeCreation` → open_before + close + create_node calls
- `Commit` → assignment: `parser.in_ordered_choice = false;`
- `Return` → if block with return
- `Predicate` → empty (no code generated)

- [ ] **Step 5: Convert structural arms**

Convert these arms that compose sub-generators:
- `Concat` → iterate operands, recurse
- `Paren` → recurse into inner
- `Alternation` → match expression with predict sets
- `OrderedChoice` → state save/restore with if-let chain
- `Star`, `Plus`, `Optional` → call gen_recovering_operation

- [ ] **Step 6: Wire gen_regex into callers**

Update `output_normal_rule` and `output_left_recursive_rule` to call `gen_regex` and write via bridge pattern.

- [ ] **Step 7: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/backend/rust.rs
git commit -m "Convert output_regex and output_recovering_operation to TokenStream"
```

---

### Task 6: Convert rule generators (normal_rule, left_recursive_rule, rule)

**Files:**
- Modify: `src/backend/rust.rs`

- [ ] **Step 1: Convert gen_normal_rule**

This composes `gen_elision_init`, `gen_node_kind_decl`, `gen_regex`, and `gen_elision_check` into a function body. Since all sub-generators now return `TokenStream`, composition is straightforward with `quote! { #tokens1 #tokens2 ... }`.

- [ ] **Step 2: Convert gen_left_recursive_rule**

This is the most complex generator. It:
1. Defines an inner `rec` function
2. Generates match arms for right-recursive and non-recursive branches
3. Generates a loop for left-recursive branches
4. Calls `rec` from the outer method

Key challenges:
- Building the `rec` function signature with conditional `min_bp` parameter
- The `call_rec` closure that generates recursive calls — convert to a regular function
- Building match arms inside the loop

Build incrementally: first the `rec` function header, then the right/non-recursive match, then the left-recursive loop.

- [ ] **Step 3: Convert gen_rule**

This generates the `fn rule_{name}(&mut self, diags: &mut Vec<...>) -> Option<()>` method with optional `#[allow(unused_assignments)]` attribute. Composes `gen_normal_rule` or `gen_left_recursive_rule` output.

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/backend/rust.rs
git commit -m "Convert rule generators to TokenStream"
```

---

### Task 7: Convert output_generated and skeleton/generated.rs template

**Files:**
- Modify: `src/backend/rust.rs`
- Remove: `src/skeleton/generated.rs` (absorbed into rust.rs as quote code)

This is the second-largest conversion. The 615-line `generated.rs` template is currently loaded via `include_str!` and used as a `format!` string with `{0}` through `{6}` positional arguments.

- [ ] **Step 1: Break the skeleton template into logical sections**

Identify and extract these sections from `generated.rs`:
1. `err!` macro definition
2. `Rule` enum (with dynamic variants)
3. `NodeRef`, `CstIndex`, `Node` types (static)
4. `MarkOpened`, `MarkClosed`, `MarkTruncation` types (static)
5. `CstChildren` iterator (static)
6. `Span` type alias (static)
7. `CstData` struct and impl (static)
8. `Cst` struct and impl (static)
9. `Rule::fmt::Debug` impl (with dynamic match arms)
10. `expect!` and `try_expect!` macros (static)
11. `ParserState` struct (static)
12. `Parser` struct (static)
13. `Parser` impl methods — most are static, but some have dynamic parts:
    - `is_skipped` — uses skip pattern (`{1}`)
    - `advance` / `init_skip` — uses skip pattern
    - `create_node` — uses rules_create match (`{5}`)
    - `delete_node` — uses rules_delete match (`{6}`)
14. `parse_rule`, `parse`, and start rule method (with dynamic start rule name)

- [ ] **Step 2: Write gen_ functions for each section**

For static sections, write the code as `quote!` blocks directly. For dynamic sections, build TokenStreams with interpolation.

Key dynamic sections:
- **Rule enum**: `quote! { pub enum Rule { #(#rule_variants,)* } }`
- **Rule Debug impl**: match arms from `rules_fmt`
- **create_node**: match arms from `rules_create`
- **delete_node**: match arms from `rules_delete`
- **skip pattern**: `quote! { | Token::#(#skip_idents)* }` in the relevant match arms
- **start rule**: `format_ident!("rule_{}", start_rule)` and `format_ident!("{}", start_rule_pascal)`

- [ ] **Step 3: Write gen_generated function**

Compose all section generators into the final TokenStream. This replaces the `format!(include_str!(...), args)` call.

- [ ] **Step 4: Wire into RustOutput::run**

Update `RustOutput::run` to call `gen_generated` and collect the TokenStream. Since we already have the parse+format pipeline from Task 1, just feed the TokenStream's string representation through it.

- [ ] **Step 5: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 6: Delete src/skeleton/generated.rs**

The template is now code, not a file. Remove it.

- [ ] **Step 7: Commit**

```bash
git add src/backend/rust.rs
git rm src/skeleton/generated.rs
git commit -m "Convert generated.rs skeleton template to quote and remove template file"
```

---

### Task 8: Convert output_parser and output_lexer (skeleton templates)

**Files:**
- Modify: `src/backend/rust.rs`
- Remove: `src/skeleton/parser.rs`
- Remove: `src/skeleton/lexer.rs`

- [ ] **Step 1: Convert gen_parser**

The `parser.rs` skeleton is only 8 lines and mostly static. Convert to:
```rust
fn gen_parser() -> TokenStream {
    quote! {
        use super::lexer::{Token, tokenize};
        use codespan_reporting::diagnostic::Label;
        pub type Diagnostic = codespan_reporting::diagnostic::Diagnostic<()>;
        include!(concat!(env!("OUT_DIR"), "/generated.rs"));
    }
}
```

Wait — the comment `// TODO: change if codespan_reporting is not used` is in the original. Include it.

- [ ] **Step 2: Convert gen_lexer**

The `lexer.rs` skeleton is ~53 lines. The dynamic part is the token enum with `#[token("...")]` attributes. Convert to:
```rust
fn gen_lexer(token_enumerators: TokenStream) -> TokenStream {
    quote! {
        // ... static parts ...
        #[derive(Logos, Debug, PartialEq, Copy, Clone)]
        #[logos(error = LexerError)]
        pub enum Token {
            #token_enumerators
        }
        // ... static parts ...
    }
}
```

The `token_enumerators` TokenStream is built by iterating over token declarations and generating `#[token("...")]` attributes and variant names.

- [ ] **Step 3: Wire into RustOutput::run**

Update `RustOutput::run` to call `gen_parser()` and `gen_lexer()` when creating the initial files, writing their TokenStreams through the parse+format pipeline.

- [ ] **Step 4: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 5: Delete skeleton files**

```bash
git rm src/skeleton/parser.rs src/skeleton/lexer.rs
```

Also remove `src/skeleton/` directory if empty.

- [ ] **Step 6: Commit**

```bash
git add src/backend/rust.rs
git rm src/skeleton/parser.rs src/skeleton/lexer.rs
git commit -m "Convert parser/lexer skeleton templates to quote and remove template files"
```

---

### Task 9: Finalize — remove old methods and Indent trait

**Files:**
- Modify: `src/backend/rust.rs`
- Modify: `src/backend/mod.rs` (if needed)

- [ ] **Step 1: Remove all `output_` methods**

Delete all old methods that wrote to `BufWriter`:
- `output_parser_callbacks` (replaced by `gen_parser_callbacks`)
- `output_node_kind_decl` (replaced by `gen_node_kind_decl`)
- `output_cst_close` (replaced by `gen_cst_close`)
- `output_elision_init` (replaced by `gen_elision_init`)
- `output_elision_check` (replaced by `gen_elision_check`)
- `output_normal_rule` (replaced by `gen_normal_rule`)
- `output_left_recursive_rule` (replaced by `gen_left_recursive_rule`)
- `output_recovering_operation` (replaced by `gen_recovering_operation`)
- `output_regex` (replaced by `gen_regex`)
- `output_rule` (replaced by `gen_rule`)
- `output_parts` (replaced by `gen_parts`)
- `output_generated` (replaced by `gen_generated`)
- `output_parser` (replaced by `gen_parser`)
- `output_lexer` (replaced by `gen_lexer`)

- [ ] **Step 2: Remove old helpers**

Delete:
- `Generator` trait and its `impl` for `BTreeSet<TokenName>`
- `syntax_error_message` function
- `Indent` trait and its implementations

- [ ] **Step 3: Remove unused imports**

Remove imports no longer needed:
- `std::io::{BufWriter, Write}` (if no longer used)
- `std::collections::BTreeSet` (check if still used)

- [ ] **Step 4: Simplify RustOutput::run**

The `run` method should now:
1. Call `gen_generated()` to get the full `generated.rs` TokenStream
2. Format with `format_rust_source()`
3. Write to file
4. If parser.rs/lexer.rs don't exist, generate them similarly

No more BufWriter threading.

- [ ] **Step 5: Rename gen_ methods to final names (optional)**

If desired, rename `gen_xxx` to just `xxx` since the old `output_xxx` are gone.

- [ ] **Step 6: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Remove old string-based generators, Indent trait, and skeleton files"
```

---

### Task 10: Verify and clean up

**Files:**
- All files

- [ ] **Step 1: Run full workspace test suite**

Run: `cargo test --workspace`
Expected: All tests pass.

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy --workspace`
Expected: No new warnings.

- [ ] **Step 3: Spot-check generated code**

Run the lelwel CLI on one of the example grammars and inspect the generated `generated.rs` to verify it's readable and well-formatted.

```bash
cargo run --features=cli --bin=llw -- examples/calc/src/calc.llw -o /tmp/lelwel-test
cat /tmp/lelwel-test/generated.rs | head -50
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Clean up and verify quote migration"
```
