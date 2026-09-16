use lelwel_onthefly::{Token, parse_with_log};

/// Asserts that parsing `source` with on-the-fly lexing produces exactly
/// `tree` and the diagnostics `diags`, and that the lexer produced exactly
/// the sequence of `(byte offset, token)` pairs in `calls`.
///
/// The `calls` assertion is the core of this example: it records every
/// `lex` invocation, including the ones that ordered choice rolled back.
/// A parsed statement can therefore show the same offsets twice — once for
/// a failed branch and once for the branch that re-lexed the input.
fn check(source: &str, tree: &str, diags: &str, calls: &[(usize, Token)]) {
    let (res, log) = parse_with_log(source);
    assert_eq!(&res[0], tree, "cst mismatch for {source:?}");
    assert_eq!(&res[1], diags, "diagnostics mismatch for {source:?}");
    assert_eq!(&log[..], calls, "lex sequence mismatch for {source:?}");
}

fn check_ok(source: &str, tree: &str, calls: &[(usize, Token)]) {
    check(source, tree, "", calls);
}

// The first branch of the ordered choice matches, so every token is lexed
// exactly once.
#[test]
fn basic_decl() {
    check_ok(
        "x = y;",
        "\
prog [0..6]
    decl [0..6]
        Id \"x\" [0..1]
        Whitespace \" \" [1..2]
        Eq \"=\" [2..3]
        Whitespace \" \" [3..4]
        Id \"y\" [4..5]
        Semi \";\" [5..6]
",
        &[
            (0, Token::Id),
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Id),
            (5, Token::Semi),
        ],
    );
}

// `decl` expects `Id Eq Id`, finds a `Num` as the third token and fails.
// The parser rolls back and `op_stmt` re-lexes the same input: offsets 3..6
// appear twice in the log, once for the failed `decl` and once for the
// winning `op_stmt`. If the rollback reused cached tokens instead, `lex`
// would not be called a second time and the sequence would not repeat.
#[test]
fn rollback_op_stmt() {
    check_ok(
        "x = 1;",
        "\
prog [0..6]
    op_stmt [0..6]
        Id \"x\" [0..1]
        Whitespace \" \" [1..2]
        Eq \"=\" [2..3]
        Whitespace \" \" [3..4]
        Num \"1\" [4..5]
        Semi \";\" [5..6]
",
        &[
            (0, Token::Id), // lexed during init_skip, before the choice
            // `decl` attempt, rolled back:
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Num),
            // `op_stmt` re-lexes the same input:
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Num),
            (5, Token::Semi),
        ],
    );
}

// `decl` and `op_stmt` both fail, so `expr_stmt` wins on the third try.
// The repeated `(1, Whitespace), (2, Plus)` triple proves the input was
// re-lexed for every branch.
#[test]
fn rollback_expr_stmt() {
    check_ok(
        "x + 1;",
        "\
prog [0..6]
    expr_stmt [0..6]
        Id \"x\" [0..1]
        Whitespace \" \" [1..2]
        Plus \"+\" [2..3]
        Whitespace \" \" [3..4]
        Num \"1\" [4..5]
        Semi \";\" [5..6]
",
        &[
            (0, Token::Id),
            (1, Token::Whitespace),
            (2, Token::Plus), // `decl` attempt
            (1, Token::Whitespace),
            (2, Token::Plus), // `op_stmt` attempt
            (1, Token::Whitespace),
            (2, Token::Plus), // `expr_stmt`
            (3, Token::Whitespace),
            (4, Token::Num),
            (5, Token::Semi),
        ],
    );
}

// Both failed branches fail right after the first token; the `;` at offset 1
// is lexed three times in total.
#[test]
fn rollback_chained() {
    check_ok(
        "x;",
        "\
prog [0..2]
    expr_stmt [0..2]
        Id \"x\" [0..1]
        Semi \";\" [1..2]
",
        &[
            (0, Token::Id),
            (1, Token::Semi), // `decl` attempt
            (1, Token::Semi), // `op_stmt` attempt
            (1, Token::Semi), // `expr_stmt`
        ],
    );
}

// Skipped whitespace tokens increase the length of both the failed branches
// and the re-lexed one.
#[test]
fn rollback_with_skipped_tokens() {
    check_ok(
        "  x = 1 ;  ",
        "\
prog [0..11]
    Whitespace \"  \" [0..2]
    op_stmt [2..9]
        Id \"x\" [2..3]
        Whitespace \" \" [3..4]
        Eq \"=\" [4..5]
        Whitespace \" \" [5..6]
        Num \"1\" [6..7]
        Whitespace \" \" [7..8]
        Semi \";\" [8..9]
    Whitespace \"  \" [9..11]
",
        &[
            (0, Token::Whitespace),
            (2, Token::Id),
            (3, Token::Whitespace),
            (4, Token::Eq),
            (5, Token::Whitespace),
            (6, Token::Num), // `decl` attempt
            (3, Token::Whitespace),
            (4, Token::Eq),
            (5, Token::Whitespace),
            (6, Token::Num), // `op_stmt`
            (7, Token::Whitespace),
            (8, Token::Semi),
            (9, Token::Whitespace),
        ],
    );
}

// Each statement rolls back on its own.
#[test]
fn multiple_statements() {
    check_ok(
        "x=1;x = 2;",
        "\
prog [0..10]
    op_stmt [0..4]
        Id \"x\" [0..1]
        Eq \"=\" [1..2]
        Num \"1\" [2..3]
        Semi \";\" [3..4]
    op_stmt [4..10]
        Id \"x\" [4..5]
        Whitespace \" \" [5..6]
        Eq \"=\" [6..7]
        Whitespace \" \" [7..8]
        Num \"2\" [8..9]
        Semi \";\" [9..10]
",
        &[
            (0, Token::Id),
            (1, Token::Eq),
            (2, Token::Num), // `decl` attempt
            (1, Token::Eq),
            (2, Token::Num), // `op_stmt`
            (3, Token::Semi),
            (4, Token::Id), // lookahead for the next statement
            (5, Token::Whitespace),
            (6, Token::Eq),
            (7, Token::Whitespace),
            (8, Token::Num), // second `decl` attempt
            (5, Token::Whitespace),
            (6, Token::Eq),
            (7, Token::Whitespace),
            (8, Token::Num), // second `op_stmt`
            (9, Token::Semi),
        ],
    );
}

// `foo` is lexed as a single multi-character `Id`.
#[test]
fn multi_character_identifiers() {
    check_ok(
        "foo;",
        "\
prog [0..4]
    expr_stmt [0..4]
        Id \"foo\" [0..3]
        Semi \";\" [3..4]
",
        &[
            (0, Token::Id),
            (3, Token::Semi), // `decl` attempt
            (3, Token::Semi), // `op_stmt` attempt
            (3, Token::Semi), // `expr_stmt`
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
        "\
prog [0..8]
    op_stmt [0..8]
        Id \"x\" [0..1]
        Whitespace \" \" [1..2]
        Eq \"=\" [2..3]
        Whitespace \" \" [3..4]
        Error \"@\" [4..5]
        Whitespace \" \" [5..6]
        Num \"1\" [6..7]
        Semi \";\" [7..8]
",
        "\
error: invalid character '@'
  \u{250c}\u{2500} <input>:1:5
  \u{2502}
1 \u{2502} x = @ 1;
  \u{2502}     ^

",
        &[
            (0, Token::Id),
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Error), // `decl` attempt
            (5, Token::Whitespace),
            (6, Token::Num),
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Error), // `op_stmt`
            (5, Token::Whitespace),
            (6, Token::Num),
            (7, Token::Semi),
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
        "\
prog [0..11]
    expr_stmt [0..5]
        Id \"a\" [0..1]
        Whitespace \" \" [1..2]
        error [2..5]
            Eq \"=\" [2..3]
            Whitespace \" \" [3..4]
            Num \"1\" [4..5]
    Whitespace \" \" [5..6]
    expr_stmt [6..11]
        Id \"b\" [6..7]
        Whitespace \" \" [7..8]
        Plus \"+\" [8..9]
        Whitespace \" \" [9..10]
        Num \"2\" [10..11]
",
        "\
error: invalid syntax, expected one of: '+', ';'
  \u{250c}\u{2500} <input>:1:3
  \u{2502}
1 \u{2502} a = 1 b + 2
  \u{2502}   ^

error: invalid syntax, expected: ;
  \u{250c}\u{2500} <input>:1:12
  \u{2502}
1 \u{2502} a = 1 b + 2
  \u{2502}            ^

",
        &[
            (0, Token::Id),
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Num), // `decl` attempt
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Num),
            (5, Token::Whitespace),
            (6, Token::Id), // `op_stmt` attempt, fails at `Semi`
            (1, Token::Whitespace),
            (2, Token::Eq),
            (3, Token::Whitespace),
            (4, Token::Num),
            (5, Token::Whitespace),
            (6, Token::Id), // `expr_stmt` attempt, hard error at `=`
            (7, Token::Whitespace),
            (8, Token::Plus), // second statement, `decl` attempt
            (7, Token::Whitespace),
            (8, Token::Plus), // second statement, `op_stmt` attempt
            (7, Token::Whitespace),
            (8, Token::Plus), // second statement, `expr_stmt`
            (9, Token::Whitespace),
            (10, Token::Num),
        ],
    );
}
