use core::fmt;

use cstree::syntax::ResolvedNode;
use cstree::traversal::WalkEvent;
use cstree::util::NodeOrToken;
use cstree::Syntax;

/// Wrapper that formats a [`ResolvedNode`] tree in the same format as [`Cst`](lelwel::Cst).
pub struct CstreeDisplay<'a, S: Syntax + fmt::Debug> {
    node: &'a ResolvedNode<S>,
}

impl<'a, S: Syntax + fmt::Debug> fmt::Display for CstreeDisplay<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut depth = 0;
        for event in self.node.preorder_with_tokens() {
            match event {
                WalkEvent::Enter(element) => {
                    let indent = "    ".repeat(depth);
                    match element {
                        NodeOrToken::Node(node) => {
                            let range = node.text_range();
                            writeln!(
                                f,
                                "{}{:?} [{}..{}]",
                                indent,
                                node.kind(),
                                u32::from(range.start()),
                                u32::from(range.end()),
                            )?;
                            depth += 1;
                        }
                        NodeOrToken::Token(token) => {
                            let range = token.text_range();
                            let text = token.text();
                            writeln!(
                                f,
                                "{}{:?} {:?} [{}..{}]",
                                indent,
                                token.kind(),
                                text,
                                u32::from(range.start()),
                                u32::from(range.end()),
                            )?;
                        }
                    }
                }
                WalkEvent::Leave(element) => {
                    if matches!(element, NodeOrToken::Node(_)) {
                        depth -= 1;
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'a, S: Syntax + fmt::Debug> CstreeDisplay<'a, S> {
    pub fn new(node: &'a ResolvedNode<S>) -> Self {
        CstreeDisplay { node }
    }
}

/// Assert that parsing `$file.$ext` with `$parse` produces the expected tree and diagnostics.
///
/// Loads `data/$file.$ext`, `data/$file.tree`, and `data/$file.diag` via `include_str!`.
/// `$parse` should be `fn(&str) -> Vec<String>` returning `[tree_string, diag_string]`.
#[macro_export]
macro_rules! check {
    ($parse:expr, $file:literal, $ext:literal) => {
        let source = &include_str!(concat!("data/", $file, ".", $ext)).replace('\r', "");
        let expected_tree = include_str!(concat!("data/", $file, ".tree")).replace('\r', "");
        let expected_diag = include_str!(concat!("data/", $file, ".diag")).replace('\r', "");
        let res = $parse(source);
        assert_eq!(format!("{}", res[0]), expected_tree);
        assert_eq!(format!("{}", res[1]), expected_diag);
    };
}