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

The grammar is a small synthetic language where all branches of the ordered
choice in `stmt` start with an `Id`:

- `decl`      — `x = y;`
- `op_stmt`   — `x = 1;`
- `expr_stmt` — `x;` or `x + 1;`

`x = 1;` is parsed by `op_stmt` only after `decl` fails at the third token
(`1` is not an `Id`), so three tokens have to be re-lexed after the rollback.

The tests in [`tests/test.rs`](tests/test.rs) assert the exact CST including
byte spans, which only match if the lexer state was reset correctly. They
also assert the **sequence of `lex` calls** itself: [`LexLog`](src/parser.rs)
lives in the `Context` — which, unlike `State`, is *not* restored on
rollback — and records every lexer invocation. Rolled-back branches show up
in the log, so the tests directly prove that a failed branch is re-lexed
from the same offset instead of reusing cached tokens.