use lelwel_l::generate_syntax_tree;
use lelwel_testing::check;

#[test]
fn incomplete() {
    check!(generate_syntax_tree, "incomplete", "l");
}

#[test]
fn missing_arrow() {
    check!(generate_syntax_tree, "missing_arrow", "l");
}