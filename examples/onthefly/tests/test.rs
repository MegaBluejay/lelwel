use lelwel_onthefly::{Token, parse_with_log};
use pretty_assertions::assert_eq;

/// Every constant below is one of the sets the generated parser hands to
/// `lex`. The canonical values come from
/// `llw -v examples/onthefly/src/onthefly.llw`; the comment on each one names
/// the node whose set it is and the call site that passes it down.
///
/// These are grammar-wide sets, not the tokens that are valid at the call
/// site. `follow(factor)` for example is the union over every call site of
/// `factor`, because the semantic pass takes the fixpoint over all rule
/// references, so it also contains what follows a `factor` inside a call
/// argument list. The parser builds its `expected one of: ...` diagnostics
/// from the same sets, so the lexer and the diagnostics agree; both can name
/// a token that no valid parse allows at that point. Input `x` reports `,`
/// and `)` for that reason.

/// Follow set of `stmt`, passed down by the loop of `prog: stmt*`.
/// Canonical: follow(stmt) = {EOF, Id, LParen, Num}
const STMT_FOLLOW: &[Token] = &[Token::EOF, Token::Id, Token::LParen, Token::Num];

/// First set of `expr`, `term`, and `factor`, which is the same for all three.
/// The left recursive dispatch of `expr`, the alternation dispatch of
/// `factor`, and the mandatory operand after `(`, `,`, `+`, and `-` all pass it
/// down.
/// Canonical: first(expr) = first(term) = first(factor) = {Id, LParen, Num}
const FIRST_OPERAND: &[Token] = &[Token::Id, Token::LParen, Token::Num];

/// Predict set of the `[Star term]` optional of `term`, passed down by the
/// optional's loop. It is `Star` plus the follow set of `term`.
/// Canonical: predict([Star term]) = follow(factor)
const TERM_OPTIONAL: &[Token] = &[
    Token::Comma,
    Token::Minus,
    Token::Plus,
    Token::RParen,
    Token::Semi,
    Token::Star,
];

/// Predict set of the `[LParen [args] RParen]` optional of `factor`, passed
/// down by the optional's loop after an `Id`. It adds the `(` that starts a
/// call to the follow set of `factor`.
/// Canonical: predict([LParen [args] RParen]) = follow(factor) + LParen
const CALL_OPTIONAL: &[Token] = &[
    Token::Comma,
    Token::LParen,
    Token::Minus,
    Token::Plus,
    Token::RParen,
    Token::Semi,
    Token::Star,
];

/// Predict set of the `[args]` optional inside a call list, passed down by the
/// inner optional. It is the first set of `args` plus the `)` that closes the
/// list.
/// Canonical: predict([args]) = first(args) + RParen
const ARGS_OPTIONAL: &[Token] = &[Token::Id, Token::LParen, Token::Num, Token::RParen];
type LexCallCheck = (usize, Token, &'static [Token]);

/// Asserts that parsing `source` with on-the-fly lexing produces exactly
/// `tree` and the diagnostics `diags`, and that the lexer served exactly the
/// sequence in `calls`.
///
/// The `calls` assertion is the core of this example: it records every `lex`
/// invocation, including the ones that ordered choice rolled back. A parsed
/// statement can therefore show the same offsets twice, once for a failed
/// branch and once for the branch that re-lexed the input. Every entry also
/// carries the expected set the parser passed down, so the test pins the
/// predict and follow sets the generator emits. The constants at the top of
/// the file name the set behind each entry.
fn check(source: &str, tree: &str, diags: &str, calls: &[LexCallCheck]) {
    let (res, log) = parse_with_log(source);
    assert_eq!(tree, &res[0], "cst mismatch for {source:?}");
    assert_eq!(diags, &res[1], "diagnostics mismatch for {source:?}");
    assert_eq!(
        &calls
            .iter()
            .copied()
            .map(|(i, t, exp)| {
                let mut exp = exp.to_owned();
                exp.sort();
                (i, t, exp)
            })
            .collect::<Vec<_>>(),
        &log[..],
        "lex sequence mismatch for {source:?}"
    );
}

fn check_ok(source: &str, tree: &str, calls: &[LexCallCheck]) {
    check(source, tree, "", calls);
}

// The first branch of the ordered choice matches, so every token is lexed
// exactly once. The program loop runs before `stmt`, so the first token
// already arrives with the loop's follow set.
#[test]
fn basic_decl() {
    check_ok(
        "x = y;",
        r#"prog [0..6]
    decl [0..6]
        Id "x" [0..1]
        Whitespace " " [1..2]
        Eq "=" [2..3]
        Whitespace " " [3..4]
        Id "y" [4..5]
        Semi ";" [5..6]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Id]),
            (4, Token::Id, &[Token::Id]),
            (5, Token::Semi, &[Token::Semi]),
        ],
    );
}

// `decl` expects `Id Eq Id`, finds a `Num` as the third token and fails.
// The parser rolls back and `op_stmt` re-lexes the same input: offsets 1..6
// appear twice in the log, once for the failed `decl` and once for the
// winning `op_stmt`. If the rollback reused cached tokens instead, `lex`
// would not be called a second time and the sequence would not repeat.
#[test]
fn rollback_op_stmt() {
    check_ok(
        "x = 1;",
        r#"prog [0..6]
    op_stmt [0..6]
        Id "x" [0..1]
        Whitespace " " [1..2]
        Eq "=" [2..3]
        Whitespace " " [3..4]
        Num "1" [4..5]
        Semi ";" [5..6]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            // `decl` attempt, rolled back:
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Id]),
            (4, Token::Num, &[Token::Id]),
            // `op_stmt` re-lexes the same input:
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Num]),
            (4, Token::Num, &[Token::Num]),
            (5, Token::Semi, &[Token::Semi]),
        ],
    );
}

// `decl` and `op_stmt` both fail, so `expr_stmt` wins on the third try. Its
// `expr` rule is left recursive, so the tokens after the first operand are
// lexed with the follow set of `expr`, which lists every operator and every
// token that can end an expression.
#[test]
fn rollback_expr_stmt() {
    check_ok(
        "x + 1;",
        r#"prog [0..6]
    expr_stmt [0..6]
        expr [0..5]
            expr [0..1]
                term [0..1]
                    factor [0..1]
                        Id "x" [0..1]
            Whitespace " " [1..2]
            Plus "+" [2..3]
            Whitespace " " [3..4]
            term [4..5]
                factor [4..5]
                    Num "1" [4..5]
        Semi ";" [5..6]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Plus, &[Token::Eq]), // `decl` attempt
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Plus, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::Whitespace, CALL_OPTIONAL),
            (2, Token::Plus, CALL_OPTIONAL), // `expr_stmt`
            (3, Token::Whitespace, FIRST_OPERAND),
            (4, Token::Num, FIRST_OPERAND),
            (5, Token::Semi, TERM_OPTIONAL),
        ],
    );
}

// All three branches fail right after the first token; the `;` at offset 1
// is lexed three times in total, each time with the expected set of the
// branch that asked for it.
#[test]
fn rollback_chained() {
    check_ok(
        "x;",
        r#"prog [0..2]
    expr_stmt [0..2]
        expr [0..1]
            term [0..1]
                factor [0..1]
                    Id "x" [0..1]
        Semi ";" [1..2]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Semi, &[Token::Eq]),  // `decl` attempt
            (1, Token::Semi, &[Token::Eq]),  // `op_stmt` attempt
            (1, Token::Semi, CALL_OPTIONAL), // `expr_stmt`
        ],
    );
}

// Skipped whitespace tokens increase the length of both the failed branches
// and the re-lexed one. The leading whitespace is lexed by the program loop,
// the trailing one by the follow set of the same loop.
#[test]
fn rollback_with_skipped_tokens() {
    check_ok(
        "  x = 1 ;  ",
        r#"prog [0..11]
    Whitespace "  " [0..2]
    op_stmt [2..9]
        Id "x" [2..3]
        Whitespace " " [3..4]
        Eq "=" [4..5]
        Whitespace " " [5..6]
        Num "1" [6..7]
        Whitespace " " [7..8]
        Semi ";" [8..9]
    Whitespace "  " [9..11]
"#,
        &[
            (0, Token::Whitespace, STMT_FOLLOW),
            (2, Token::Id, STMT_FOLLOW),
            (3, Token::Whitespace, &[Token::Eq]),
            (4, Token::Eq, &[Token::Eq]),
            (5, Token::Whitespace, &[Token::Id]),
            (6, Token::Num, &[Token::Id]), // `decl` attempt
            (3, Token::Whitespace, &[Token::Eq]),
            (4, Token::Eq, &[Token::Eq]),
            (5, Token::Whitespace, &[Token::Num]),
            (6, Token::Num, &[Token::Num]), // `op_stmt`
            (7, Token::Whitespace, &[Token::Semi]),
            (8, Token::Semi, &[Token::Semi]),
            (9, Token::Whitespace, STMT_FOLLOW),
        ],
    );
}

// Each statement rolls back on its own. The `Id` starting the second
// statement is lexed by the program loop, so it carries the loop's follow
// set rather than a predict set of `stmt`.
#[test]
fn multiple_statements() {
    check_ok(
        "x=1;x = 2;",
        r#"prog [0..10]
    op_stmt [0..4]
        Id "x" [0..1]
        Eq "=" [1..2]
        Num "1" [2..3]
        Semi ";" [3..4]
    op_stmt [4..10]
        Id "x" [4..5]
        Whitespace " " [5..6]
        Eq "=" [6..7]
        Whitespace " " [7..8]
        Num "2" [8..9]
        Semi ";" [9..10]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Eq, &[Token::Eq]),
            (2, Token::Num, &[Token::Id]), // `decl` attempt
            (1, Token::Eq, &[Token::Eq]),
            (2, Token::Num, &[Token::Num]), // `op_stmt`
            (3, Token::Semi, &[Token::Semi]),
            (4, Token::Id, STMT_FOLLOW), // lookahead for the next statement
            (5, Token::Whitespace, &[Token::Eq]),
            (6, Token::Eq, &[Token::Eq]),
            (7, Token::Whitespace, &[Token::Id]),
            (8, Token::Num, &[Token::Id]), // second `decl` attempt
            (5, Token::Whitespace, &[Token::Eq]),
            (6, Token::Eq, &[Token::Eq]),
            (7, Token::Whitespace, &[Token::Num]),
            (8, Token::Num, &[Token::Num]), // second `op_stmt`
            (9, Token::Semi, &[Token::Semi]),
        ],
    );
}

// `foo` is lexed as a single multi-character `Id`.
#[test]
fn multi_character_identifiers() {
    check_ok(
        "foo;",
        r#"prog [0..4]
    expr_stmt [0..4]
        expr [0..3]
            term [0..3]
                factor [0..3]
                    Id "foo" [0..3]
        Semi ";" [3..4]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (3, Token::Semi, &[Token::Eq]),  // `decl` attempt
            (3, Token::Semi, &[Token::Eq]),  // `op_stmt` attempt
            (3, Token::Semi, CALL_OPTIONAL), // `expr_stmt`
        ],
    );
}

// An invalid character produces a `Token::Error` and a diagnostic. The
// `Error` token is lexed in both attempts, and the diagnostic emitted in the
// rolled back `decl` attempt is truncated so only one remains.
#[test]
fn invalid_character() {
    check(
        "x = @ 1;",
        r#"prog [0..8]
    op_stmt [0..8]
        Id "x" [0..1]
        Whitespace " " [1..2]
        Eq "=" [2..3]
        Whitespace " " [3..4]
        Error "@" [4..5]
        Whitespace " " [5..6]
        Num "1" [6..7]
        Semi ";" [7..8]
"#,
        "\
error: invalid character '@'
  \u{250c}\u{2500} <input>:1:5
  \u{2502}
1 \u{2502} x = @ 1;
  \u{2502}     ^

",
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Id]),
            (4, Token::Error, &[Token::Id]), // `decl` attempt
            (5, Token::Whitespace, &[Token::Id]),
            (6, Token::Num, &[Token::Id]),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Num]),
            (4, Token::Error, &[Token::Num]), // `op_stmt`
            (5, Token::Whitespace, &[Token::Num]),
            (6, Token::Num, &[Token::Num]),
            (7, Token::Semi, &[Token::Semi]),
        ],
    );
}

// All three branches fail on the first statement, the last one with a hard
// error. Note that the failed `decl` and `op_stmt` attempts each re-lex the
// same offsets, and the error recovery then parses `b + 2` by rolling back
// its own three attempts.
#[test]
fn parse_error_recovery() {
    check(
        "a = 1 b + 2",
        r#"prog [0..11]
    expr_stmt [0..3]
        expr [0..3]
            term [0..3]
                factor [0..3]
                    Id "a" [0..1]
                    Whitespace " " [1..2]
                    error [2..3]
                        Eq "=" [2..3]
    Whitespace " " [3..4]
    expr_stmt [4..5]
        expr [4..5]
            term [4..5]
                factor [4..5]
                    Num "1" [4..5]
    Whitespace " " [5..6]
    expr_stmt [6..11]
        expr [6..11]
            expr [6..7]
                term [6..7]
                    factor [6..7]
                        Id "b" [6..7]
            Whitespace " " [7..8]
            Plus "+" [8..9]
            Whitespace " " [9..10]
            term [10..11]
                factor [10..11]
                    Num "2" [10..11]
"#,
        "\
error: invalid syntax, expected one of: ',', '(', '-', '+', ')', ';', '*'
  \u{250c}\u{2500} <input>:1:3
  \u{2502}
1 \u{2502} a = 1 b + 2
  \u{2502}   ^

error: invalid syntax, expected one of: ',', '-', '+', ')', ';', '*'
  \u{250c}\u{2500} <input>:1:7
  \u{2502}
1 \u{2502} a = 1 b + 2
  \u{2502}       ^

error: invalid syntax, expected one of: ',', '-', '+', ')', ';', '*'
  \u{250c}\u{2500} <input>:1:12
  \u{2502}
1 \u{2502} a = 1 b + 2
  \u{2502}            ^

",
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Id]),
            (4, Token::Num, &[Token::Id]), // `decl` attempt
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Eq, &[Token::Eq]),
            (3, Token::Whitespace, &[Token::Num]),
            (4, Token::Num, &[Token::Num]),
            (5, Token::Whitespace, &[Token::Semi]),
            (6, Token::Id, &[Token::Semi]), // `op_stmt` attempt, fails at `Semi`
            (1, Token::Whitespace, CALL_OPTIONAL),
            (2, Token::Eq, CALL_OPTIONAL),
            (3, Token::Whitespace, CALL_OPTIONAL),
            (4, Token::Num, CALL_OPTIONAL),
            (5, Token::Whitespace, TERM_OPTIONAL),
            (6, Token::Id, TERM_OPTIONAL), // `expr_stmt` attempt, hard error at `=`
            (7, Token::Whitespace, &[Token::Eq]),
            (8, Token::Plus, &[Token::Eq]), // second statement, `decl` attempt
            (7, Token::Whitespace, &[Token::Eq]),
            (8, Token::Plus, &[Token::Eq]), // second statement, `op_stmt` attempt
            (7, Token::Whitespace, CALL_OPTIONAL),
            (8, Token::Plus, CALL_OPTIONAL), // second statement, `expr_stmt`
            (9, Token::Whitespace, FIRST_OPERAND),
            (10, Token::Num, FIRST_OPERAND),
        ],
    );
}

// Left recursion. `expr` is a Pratt parser: the first operand is parsed with
// the predict set of the rule, and the loop that keeps consuming `+` and `-`
// matches on the follow set of `expr`. The CST nests to the left, so
// `a + b - c` groups as `(a + b) - c`.
//
// The follow set of `expr` is `{Comma, Minus, Plus, RParen, Semi}` and the
// generated loop does pass it to `current`. It never reaches `lex` though:
// the loop only runs after `term`, and `term` always ends by checking its own
// optional, which leaves one token cached. The operators below are therefore
// lexed by `CALL_OPTIONAL` in the preceding `factor`, which is a superset that
// also allows `(` and `*`.
#[test]
fn left_recursion() {
    check_ok(
        "a + b - c;",
        r#"prog [0..10]
    expr_stmt [0..10]
        expr [0..9]
            expr [0..5]
                expr [0..1]
                    term [0..1]
                        factor [0..1]
                            Id "a" [0..1]
                Whitespace " " [1..2]
                Plus "+" [2..3]
                Whitespace " " [3..4]
                term [4..5]
                    factor [4..5]
                        Id "b" [4..5]
            Whitespace " " [5..6]
            Minus "-" [6..7]
            Whitespace " " [7..8]
            term [8..9]
                factor [8..9]
                    Id "c" [8..9]
        Semi ";" [9..10]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Plus, &[Token::Eq]), // `decl` attempt
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Plus, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::Whitespace, CALL_OPTIONAL),
            (2, Token::Plus, CALL_OPTIONAL),
            (3, Token::Whitespace, FIRST_OPERAND),
            (4, Token::Id, FIRST_OPERAND),
            (5, Token::Whitespace, CALL_OPTIONAL),
            (6, Token::Minus, CALL_OPTIONAL),
            (7, Token::Whitespace, FIRST_OPERAND),
            (8, Token::Id, FIRST_OPERAND),
            (9, Token::Semi, CALL_OPTIONAL),
        ],
    );
}

// Right recursion. `term` is `factor [Star term]`, so the CST nests to the
// right and `a * b * c` groups as `a * (b * c)`. The optional checks for
// `Star` with the follow set that includes the first token of its operand.
#[test]
fn right_recursion() {
    check_ok(
        "a * b * c;",
        r#"prog [0..10]
    expr_stmt [0..10]
        expr [0..9]
            term [0..9]
                factor [0..1]
                    Id "a" [0..1]
                Whitespace " " [1..2]
                Star "*" [2..3]
                Whitespace " " [3..4]
                term [4..9]
                    factor [4..5]
                        Id "b" [4..5]
                    Whitespace " " [5..6]
                    Star "*" [6..7]
                    Whitespace " " [7..8]
                    term [8..9]
                        factor [8..9]
                            Id "c" [8..9]
        Semi ";" [9..10]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Star, &[Token::Eq]), // `decl` attempt
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Star, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::Whitespace, CALL_OPTIONAL),
            (2, Token::Star, CALL_OPTIONAL),
            (3, Token::Whitespace, FIRST_OPERAND),
            (4, Token::Id, FIRST_OPERAND),
            (5, Token::Whitespace, CALL_OPTIONAL),
            (6, Token::Star, CALL_OPTIONAL),
            (7, Token::Whitespace, FIRST_OPERAND),
            (8, Token::Id, FIRST_OPERAND),
            (9, Token::Semi, CALL_OPTIONAL),
        ],
    );
}

// `*` binds tighter than `+` because the left recursion in `expr` only looks
// for `+` and `-`, while `term` consumes `*`. The token after `b` is checked
// against the follow set of `expr`, which still lists `Star` even though the
// left recursive loop never matches it.
#[test]
fn mixed_precedence() {
    check_ok(
        "a + b * c;",
        r#"prog [0..10]
    expr_stmt [0..10]
        expr [0..9]
            expr [0..1]
                term [0..1]
                    factor [0..1]
                        Id "a" [0..1]
            Whitespace " " [1..2]
            Plus "+" [2..3]
            Whitespace " " [3..4]
            term [4..9]
                factor [4..5]
                    Id "b" [4..5]
                Whitespace " " [5..6]
                Star "*" [6..7]
                Whitespace " " [7..8]
                term [8..9]
                    factor [8..9]
                        Id "c" [8..9]
        Semi ";" [9..10]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Plus, &[Token::Eq]), // `decl` attempt
            (1, Token::Whitespace, &[Token::Eq]),
            (2, Token::Plus, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::Whitespace, CALL_OPTIONAL),
            (2, Token::Plus, CALL_OPTIONAL),
            (3, Token::Whitespace, FIRST_OPERAND),
            (4, Token::Id, FIRST_OPERAND),
            (5, Token::Whitespace, CALL_OPTIONAL),
            (6, Token::Star, CALL_OPTIONAL),
            (7, Token::Whitespace, FIRST_OPERAND),
            (8, Token::Id, FIRST_OPERAND),
            (9, Token::Semi, CALL_OPTIONAL),
        ],
    );
}

// Parentheses select the second branch of the `factor` alternation and open a
// nested `expr`. Inside the parentheses `)` ends the expression, so it is
// part of the follow set the nested loop lexes with.
#[test]
fn parenthesized_operand() {
    check_ok(
        "(a + 1) * b;",
        r#"prog [0..12]
    expr_stmt [0..12]
        expr [0..11]
            term [0..11]
                factor [0..7]
                    LParen "(" [0..1]
                    expr [1..6]
                        expr [1..2]
                            term [1..2]
                                factor [1..2]
                                    Id "a" [1..2]
                        Whitespace " " [2..3]
                        Plus "+" [3..4]
                        Whitespace " " [4..5]
                        term [5..6]
                            factor [5..6]
                                Num "1" [5..6]
                    RParen ")" [6..7]
                Whitespace " " [7..8]
                Star "*" [8..9]
                Whitespace " " [9..10]
                term [10..11]
                    factor [10..11]
                        Id "b" [10..11]
        Semi ";" [11..12]
"#,
        &[
            (0, Token::LParen, STMT_FOLLOW),
            (1, Token::Id, FIRST_OPERAND),
            (2, Token::Whitespace, CALL_OPTIONAL),
            (3, Token::Plus, CALL_OPTIONAL),
            (4, Token::Whitespace, FIRST_OPERAND),
            (5, Token::Num, FIRST_OPERAND),
            (6, Token::RParen, TERM_OPTIONAL),
            (7, Token::Whitespace, TERM_OPTIONAL),
            (8, Token::Star, TERM_OPTIONAL),
            (9, Token::Whitespace, FIRST_OPERAND),
            (10, Token::Id, FIRST_OPERAND),
            (11, Token::Semi, CALL_OPTIONAL),
        ],
    );
}

// A call is an `Id` followed by the optional `(args)`. The `(` is lexed twice
// by the failed `decl` and `op_stmt` branches before `expr_stmt` reaches the
// optional, whose expected set contains its operand's first set and the
// `RParen` that follows it.
#[test]
fn call_with_arguments() {
    check_ok(
        "f(x, y);",
        r#"prog [0..8]
    expr_stmt [0..8]
        expr [0..7]
            term [0..7]
                factor [0..7]
                    Id "f" [0..1]
                    LParen "(" [1..2]
                    args [2..6]
                        expr [2..3]
                            term [2..3]
                                factor [2..3]
                                    Id "x" [2..3]
                        Comma "," [3..4]
                        Whitespace " " [4..5]
                        expr [5..6]
                            term [5..6]
                                factor [5..6]
                                    Id "y" [5..6]
                    RParen ")" [6..7]
        Semi ";" [7..8]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::LParen, &[Token::Eq]), // `decl` attempt
            (1, Token::LParen, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::LParen, CALL_OPTIONAL),
            (2, Token::Id, ARGS_OPTIONAL),
            (3, Token::Comma, CALL_OPTIONAL),
            (4, Token::Whitespace, FIRST_OPERAND),
            (5, Token::Id, FIRST_OPERAND),
            (6, Token::RParen, CALL_OPTIONAL),
            (7, Token::Semi, TERM_OPTIONAL),
        ],
    );
}

// With `f()` the optional `(args)` is taken but the inner `[args]` optional is
// skipped, so no `args` node appears in the CST. The `)` is lexed with the
// expected set of the inner optional.
#[test]
fn call_without_arguments() {
    check_ok(
        "f();",
        r#"prog [0..4]
    expr_stmt [0..4]
        expr [0..3]
            term [0..3]
                factor [0..3]
                    Id "f" [0..1]
                    LParen "(" [1..2]
                    RParen ")" [2..3]
        Semi ";" [3..4]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::LParen, &[Token::Eq]), // `decl` attempt
            (1, Token::LParen, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::LParen, CALL_OPTIONAL),
            (2, Token::RParen, ARGS_OPTIONAL),
            (3, Token::Semi, TERM_OPTIONAL),
        ],
    );
}

// A star repetition over the group `(Comma expr)`. Each `,` is lexed by the
// optional tail of the preceding `factor`, so it arrives with the follow set
// of `expr` rather than with the narrower follow set of `args`.
#[test]
fn repeated_arguments() {
    check_ok(
        "f(a, b, c);",
        r#"prog [0..11]
    expr_stmt [0..11]
        expr [0..10]
            term [0..10]
                factor [0..10]
                    Id "f" [0..1]
                    LParen "(" [1..2]
                    args [2..9]
                        expr [2..3]
                            term [2..3]
                                factor [2..3]
                                    Id "a" [2..3]
                        Comma "," [3..4]
                        Whitespace " " [4..5]
                        expr [5..6]
                            term [5..6]
                                factor [5..6]
                                    Id "b" [5..6]
                        Comma "," [6..7]
                        Whitespace " " [7..8]
                        expr [8..9]
                            term [8..9]
                                factor [8..9]
                                    Id "c" [8..9]
                    RParen ")" [9..10]
        Semi ";" [10..11]
"#,
        &[
            (0, Token::Id, STMT_FOLLOW),
            (1, Token::LParen, &[Token::Eq]), // `decl` attempt
            (1, Token::LParen, &[Token::Eq]), // `op_stmt` attempt
            (1, Token::LParen, CALL_OPTIONAL),
            (2, Token::Id, ARGS_OPTIONAL),
            (3, Token::Comma, CALL_OPTIONAL),
            (4, Token::Whitespace, FIRST_OPERAND),
            (5, Token::Id, FIRST_OPERAND),
            (6, Token::Comma, CALL_OPTIONAL),
            (7, Token::Whitespace, FIRST_OPERAND),
            (8, Token::Id, FIRST_OPERAND),
            (9, Token::RParen, CALL_OPTIONAL),
            (10, Token::Semi, TERM_OPTIONAL),
        ],
    );
}

// A statement that starts with `Num` skips the `decl` and `op_stmt` guards
// without a lex call, because the program loop has the token cached already.
// The `;` is then checked against the follow set of `expr`.
#[test]
fn number_expression() {
    check_ok(
        "1;",
        r#"prog [0..2]
    expr_stmt [0..2]
        expr [0..1]
            term [0..1]
                factor [0..1]
                    Num "1" [0..1]
        Semi ";" [1..2]
"#,
        &[
            (0, Token::Num, STMT_FOLLOW),
            (1, Token::Semi, TERM_OPTIONAL),
        ],
    );
}

// The program is a star repetition, so an empty input parses without ever
// calling `lex`.
#[test]
fn empty_program() {
    check_ok("", "prog [0..0]\n", &[]);
}
