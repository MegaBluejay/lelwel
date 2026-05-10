use lelwel_lua::generate_syntax_tree;
use lelwel_testing::check;

#[test]
fn incomplete() {
    check!(generate_syntax_tree, "incomplete", "lua");
}