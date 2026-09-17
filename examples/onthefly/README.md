# lelwel-onthefly

An example that demonstrates on-the-fly lexing: instead of tokenizing the whole
source upfront, the lexer runs only when the parser needs the next token.

The three pieces that make this work are

- `ParserCallbacks::create_tokens` returns empty vectors,
- the lexer state is stored in `ParserCallbacks::State` ([`LexState`](src/lexer.rs)),
- `ParserCallbacks::lex` produces the next token on demand and advances the state.

The state just needs a `Default` implementation to be usable with
`Parser::new`; a custom initial state can be passed through
`Parser::new_with_context`.

Because the lexer state lives in the parser, it is snapshotted and restored
together with the token stream when an ordered choice backtracks: a failed
branch is re-lexed from the exact same position for the next branch.

The grammar is a small expression language. Statements are an ordered choice
whose branches all start with an `Id`, so a failed branch has to backtrack
and re-lex:

- `decl` parses `x = y;`
- `op_stmt` parses `x = 1;`
- `expr_stmt` parses an expression followed by `;`

`x = 1;` is parsed by `op_stmt` only after `decl` fails at the third token
(`1` is not an `Id`), so the tokens from there on are re-lexed.

Expressions exercise the repetition and recursion constructs, and every test
pins the expected set the parser passes to `lex` at each point:

- left recursion in `expr`, where `a + b - c` groups as `(a + b) - c`
- right recursion in `term`, where `a * b * c` groups as `a * (b * c)`
- alternation in `factor` over `Num`, `Id`, and `(expr)`
- ordered choice in `stmt` and in the left recursive branches of `expr`
- optionals for the call argument list and the `*` tail of `term`
- star repetition in `prog: stmt*` and in `args: expr (Comma expr)*`

The tests in [`tests/test.rs`](tests/test.rs) assert the exact CST including
byte spans, which only match if the lexer state was reset correctly. They
also assert the **sequence of `lex` calls** itself: [`LexLog`](src/parser.rs)
lives in the `Context`, which unlike `State` is *not* restored on rollback,
and records every lexer invocation. Rolled-back branches show up in the log,
so the tests directly prove that a failed branch is re-lexed from the same
offset instead of reusing cached tokens. Each logged call also carries the
expected set the parser passed down to `lex`, so the tests pin the predict
and follow sets the generator emits for every token.