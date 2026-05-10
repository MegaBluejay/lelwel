use lelwel_wgsl::generate_syntax_tree;
use lelwel_testing::check;

#[test]
fn incomplete() {
    check!(generate_syntax_tree, "incomplete", "wgsl");
}

#[test]
fn template() {
    check!(generate_syntax_tree, "template", "wgsl");
}

#[test]
fn incomplete_if() {
    check!(generate_syntax_tree, "incomplete_if", "wgsl");
}

#[test]
fn missing_comma() {
    check!(generate_syntax_tree, "missing_comma", "wgsl");
}

#[test]
fn missing_arrow() {
    check!(generate_syntax_tree, "missing_arrow", "wgsl");
}

#[test]
fn invalid_for() {
    check!(generate_syntax_tree, "invalid_for", "wgsl");
}

#[test]
fn incomplete_let() {
    check!(generate_syntax_tree, "incomplete_let", "wgsl");
}

#[test]
fn missing_semicolon() {
    check!(generate_syntax_tree, "missing_semicolon", "wgsl");
}