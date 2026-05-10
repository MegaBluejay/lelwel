use lelwel_c::generate_syntax_tree;
use lelwel_testing::check;

#[test]
fn incomplete() {
    check!(generate_syntax_tree, "incomplete", "c");
}

#[test]
fn incomplete_call() {
    check!(generate_syntax_tree, "incomplete_call", "c");
}