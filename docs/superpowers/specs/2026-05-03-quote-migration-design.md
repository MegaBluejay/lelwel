# Refactor Rust Backend to Use quote/proc_macro2/syn

## Goal

Replace all string-based code generation in `src/backend/rust.rs` with `quote`/`proc_macro2` token stream generation. The generated output must remain semantically equivalent.

## Approach

Bottom-up incremental migration (Approach A). Each step produces a working state where `cargo test --workspace` passes. Old methods coexist with new ones during migration; new methods use a `gen_` prefix. Old `output_` methods are removed once all their callers are converted.

Two phases:
- **Phase 1 (this migration):** Literal conversion — generators return `proc_macro2::TokenStream`, use `quote!` blocks. Functionally equivalent to current output.
- **Phase 2 (follow-up):** Type tightening — return specific `syn` types (`syn::ItemFn`, `syn::ImplItem`, etc.) and use `parse_quote!`. Catches invalid code at generation time.

## Architecture Change

**Current:** Methods take `output: &mut BufWriter<File>` and call `output.write_all(format!(...).as_bytes())`.

**Target:** Methods return `TokenStream`. The top-level `RustOutput::run()` collects all token streams, converts to string (via `prettyplease` formatting), and writes to file once.

**New dependencies:**
- `quote` — quasi-quoting for TokenStream generation
- `proc_macro2` — TokenStream type usable outside proc_macro context
- `syn` — for `parse_file()` to validate and format generated code (Phase 2 may use more syn types)
- `prettyplease` — format the generated output for readability

**Removed:**
- `Indent` trait (quote handles structure; prettyplease handles formatting)
- `BufWriter<File>` threading through all generator methods

## Key Design Decisions

1. **Dynamic identifiers** use `quote::format_ident!("rule_{name}")` by default. `proc_macro2::Ident::new()` only if `format_ident!` is unsuitable for a specific case.

2. **`parser_name` parameter** changes from `&str` to `proc_macro2::Ident`. Two constant idents: `SELF` and `PARSER`.

3. **Output formatting** uses `prettyplease::unparse()` on the final TokenStream. This ensures readability and validates that generated code parses as valid Rust.

4. **Skeleton templates** (`src/skeleton/generated.rs`, `parser.rs`, `lexer.rs`) are converted from `include_str!` format strings to `quote!` blocks, broken into logical sections with their own `gen_xxx` functions.

5. **Old/new coexistence:** During migration, old `output_` methods and new `gen_` methods coexist. Callers of still-unconverted `output_` methods get a temporary bridge: `output.write_all(tokens.to_string().as_bytes())`. Old helpers (`Generator::pattern()`, `syntax_error_message()`) stay until all callers are migrated, then are removed.

## Migration Steps

Each step is followed by `cargo test --workspace`.

### Step 1: Add dependencies, introduce parse+format at top level
- Add `quote`, `proc_macro2`, `syn`, `prettyplease` to `Cargo.toml`
- In `RustOutput::run()`, after generating the output string, parse it with `syn::parse_file()` and format with `prettyplease::unparse()` before writing to disk
- This immediately reformats all generated code. Fix any tests that assert on specific formatting
- This validates the parse→format pipeline works before any quote conversion begins
- Rationale: during incremental conversion, helpers produce TokenStream → `.to_string()` → concatenated with still-string code → `syn::parse_file()` → `prettyplease::unparse()`. Getting this step in first isolates formatting-related test failures

### Step 2: Convert helper functions (no I/O)
- Add `gen_pattern() -> TokenStream` alongside `Generator::pattern()`
- Add `gen_error() -> TokenStream` alongside `Generator::error()`
- Add `gen_syntax_error_message() -> TokenStream` alongside `syntax_error_message()`
- `snake_to_pascal_case` and `escape` stay as-is (pure string utilities, not code generation)

### Step 3: Convert simple code generators
- `output_node_kind_decl` → `gen_node_kind_decl` returns `TokenStream`
- `output_cst_close` → `gen_cst_close` returns `TokenStream`
- `output_elision_init` → `gen_elision_init` returns `TokenStream`
- `output_elision_check` → `gen_elision_check` returns `TokenStream`
- `output_parts` → `gen_parts` returns `TokenStream`
- `output_parser_callbacks` → `gen_parser_callbacks` returns `TokenStream`

### Step 4: Convert `output_regex` and all its match arms
The biggest method (~460 lines). Convert each `Regex::*` arm:
- `Name`, `Symbol` → expect/try_expect macro invocations
- `Action`, `Assertion`, `Predicate` → method call expressions
- `NodeRename`, `NodeElision`, `NodeMarker`, `NodeCreation`, `Commit`, `Return` → statements
- `Alternation` → match expression
- `OrderedChoice` → state save/restore with if-let chain
- `Star`, `Plus`, `Optional` → loop with match (via `gen_recovering_operation`)
- `Concat`, `Paren` → recursive composition

Includes converting `output_recovering_operation` → `gen_recovering_operation`.

### Step 5: Convert rule generators
- `output_normal_rule` → `gen_normal_rule` returns `TokenStream`
- `output_left_recursive_rule` → `gen_left_recursive_rule` returns `TokenStream`
- `output_rule` → `gen_rule` returns `TokenStream`

### Step 6: Convert top-level generators
- `output_generated` → `gen_generated` builds the full module content
- Convert `skeleton/generated.rs` from format string template to `quote!` blocks
  - Break into sections: err macro, Rule enum, CstData, Parser struct, etc.
  - Each section gets its own `gen_xxx` function
- `output_rule` and `output_parts` (if not already done in step 5)

### Step 7: Convert skeleton file generators
- `output_parser` → `gen_parser` returns `TokenStream` for `parser.rs`
- `output_lexer` → `gen_lexer` returns `TokenStream` for `lexer.rs`
- Convert `skeleton/parser.rs` and `skeleton/lexer.rs` from `include_str!` string templates to `quote!` blocks

### Step 8: Finalize
- Change `RustOutput::run` to build a single `TokenStream`, format with `prettyplease`, write to file
- Remove all `output_` methods, old helpers (`Generator::pattern()`, `Generator::error()`, `syntax_error_message()`), and `Indent` trait
- Remove `BufWriter` and `Write` imports no longer needed
- Remove `src/skeleton/` directory (templates are now code, not files)
- Rename `gen_` methods to their final names if desired

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Token boundary merging (adjacent tokens fuse) | Each generator produces complete Rust fragments (full statement, full match arm). Never concatenate bare tokens. Test after each step. |
| Dynamic identifiers | Use `quote::format_ident!` for all constructed idents. |
| `parser_name` as string | Change to `Ident` type. Two constants: `SELF`, `PARSER`. |
| Generated code readability | `prettyplease::unparse()` on final output. |
| Skeleton template conversion (615 lines) | Break into logical sections, convert each independently. |
| Incomplete fragments (match patterns, skip tokens) | Use `quote!` repetition (`#(#pats)|*`) or pre-built TokenStream interpolation. |
| `expect!`/`try_expect!` macro invocations | `quote!(expect!(#token_ident, #msg_literal, #parser, diags))` — Ident and LitStr interpolate correctly into macro positions. |
