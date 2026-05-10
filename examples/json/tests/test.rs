use lelwel_json::generate_syntax_tree;
use lelwel_testing::check;

#[test]
fn incomplete() {
    check!(generate_syntax_tree, "incomplete", "json");
}

#[test]
fn wrong_key() {
    check!(generate_syntax_tree, "wrong_key", "json");
}

#[test]
fn escape() {
    check!(generate_syntax_tree, "escape", "json");
}