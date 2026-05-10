use core::fmt;
use cstree::build::{Checkpoint, GreenNodeBuilder};
use cstree::green::GreenNode;
use cstree::util::NodeOrToken;
use cstree::Syntax;

use crate::{CstBuilder, Span};

/// [`CstBuilder`] implementation backed by `cstree`'s [`GreenNodeBuilder`].
pub struct CstreeBuilder<S: Syntax> {
    inner: GreenNodeBuilder<'static, 'static, S>,
}

impl<S: Syntax> CstreeBuilder<S> {
    pub fn new_empty() -> Self {
        CstreeBuilder { inner: GreenNodeBuilder::new() }
    }
}

impl<S: Syntax> CstBuilder for CstreeBuilder<S> {
    type Token = S;
    type Rule = S;
    type Mark = Checkpoint;
    type Checkpoint = Checkpoint;
    type Output = GreenNode;

    fn new(_spans: Vec<Span>) -> Self { CstreeBuilder::new_empty() }

    fn token(&mut self, kind: S, text: &str) {
        if S::static_text(kind).is_some() {
            self.inner.static_token(kind);
        } else {
            self.inner.token(kind, text);
        }
    }

    fn start_rule(&mut self) -> Checkpoint { self.inner.checkpoint() }

    fn end_rule(&mut self, mark: Checkpoint, rule: S) {
        self.inner.start_node_at(mark, rule);
        self.inner.finish_node();
    }

    fn mark(&self) -> Checkpoint { self.inner.checkpoint() }
    fn start_rule_before(&mut self, mark: Checkpoint) -> Checkpoint { mark }
    fn checkpoint(&self) -> Checkpoint { self.inner.checkpoint() }
    fn revert_to(&mut self, cp: Checkpoint) { self.inner.revert_to(cp); }
    fn finish(self) -> GreenNode { self.inner.finish().0 }
}

/// Wrapper that displays a cstree [`GreenNode`] in the same format as [`Cst`](crate::Cst).
pub struct CstreeDisplay<'a, S: Syntax + fmt::Debug> {
    node: &'a GreenNode,
    source: &'a str,
    _marker: core::marker::PhantomData<S>,
}

impl<'a, S: Syntax + fmt::Debug> fmt::Display for CstreeDisplay<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind: S = S::from_raw(self.node.kind());
        let len: usize = self.node.text_len().into();
        writeln!(f, "{kind:?} [0..{len}]")?;
        fmt_node::<S>(f, self.node, self.source, 1, 0)
    }
}

impl<'a, S: Syntax + fmt::Debug> CstreeDisplay<'a, S> {
    pub fn new(node: &'a GreenNode, source: &'a str) -> Self {
        CstreeDisplay { node, source, _marker: core::marker::PhantomData }
    }
}

fn fmt_node<S: Syntax + fmt::Debug>(
    f: &mut fmt::Formatter<'_>,
    node: &GreenNode,
    source: &str,
    indent: usize,
    offset: usize,
) -> fmt::Result {
    let mut off = offset;
    for child in node.children() {
        match child {
            NodeOrToken::Node(n) => {
                let len: usize = n.text_len().into();
                let kind: S = S::from_raw(n.kind());
                writeln!(f, "{}{kind:?} [{}..{}]", "    ".repeat(indent), off, off + len)?;
                fmt_node::<S>(f, &n, source, indent + 1, off)?;
                off += len;
            }
            NodeOrToken::Token(t) => {
                let len: usize = t.text_len().into();
                let kind: S = S::from_raw(t.kind());
                let text = &source[off..off + len];
                writeln!(f, "{}{kind:?} {:?} [{}..{}]", "    ".repeat(indent), text, off, off + len)?;
                off += len;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cstree::RawSyntaxKind;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    enum TestKind {
        Root,
        Rule,
        TokenA,
        TokenB,
    }

    impl Syntax for TestKind {
        fn from_raw(raw: RawSyntaxKind) -> Self {
            match raw.0 {
                0 => TestKind::Root,
                1 => TestKind::Rule,
                2 => TestKind::TokenA,
                3 => TestKind::TokenB,
                _ => panic!("unknown TestKind: {}", raw.0),
            }
        }
        fn into_raw(self) -> RawSyntaxKind { RawSyntaxKind(self as u32) }
        fn static_text(self) -> Option<&'static str> { None }
    }

    impl crate::TokenType for TestKind {
        fn is_skip(&self) -> bool { false }
        fn is_eof(&self) -> bool { false }
        fn eof() -> Self { TestKind::Root }
    }

    impl crate::RuleType for TestKind {
        fn error() -> Self { TestKind::Root }
    }

    #[test]
    fn build_simple_tree() {
        let mut b = CstreeBuilder::<TestKind>::new_empty();
        let m = b.start_rule();
        b.token(TestKind::TokenA, "hello");
        b.token(TestKind::TokenB, "world");
        b.end_rule(m, TestKind::Root);
        let tree = b.finish();
        assert_eq!(tree.children().count(), 2);
    }

    #[test]
    fn build_nested_tree() {
        let mut b = CstreeBuilder::<TestKind>::new_empty();
        let root = b.start_rule();
        let inner = b.start_rule();
        b.token(TestKind::TokenA, "a");
        b.end_rule(inner, TestKind::Rule);
        b.token(TestKind::TokenB, "b");
        b.end_rule(root, TestKind::Root);
        let tree = b.finish();
        assert_eq!(tree.children().count(), 2);
    }

    #[test]
    fn revert_checkpoint() {
        let mut b = CstreeBuilder::<TestKind>::new_empty();
        let root = b.start_rule();
        let cp = b.checkpoint();
        b.token(TestKind::TokenA, "discard");
        b.revert_to(cp);
        b.token(TestKind::TokenB, "keep");
        b.end_rule(root, TestKind::Root);
        let tree = b.finish();
        assert_eq!(tree.children().count(), 1);
    }

    #[test]
    fn cstree_display_format() {
        let mut b = CstreeBuilder::<TestKind>::new_empty();
        let m = b.start_rule();
        b.token(TestKind::TokenA, "hello");
        b.token(TestKind::TokenB, "world");
        b.end_rule(m, TestKind::Root);
        let tree = b.finish();
        let s = CstreeDisplay::<TestKind>::new(&tree, "helloworld").to_string();
        assert!(s.contains("Root"));
        assert!(s.contains("TokenA"));
        assert!(s.contains("TokenB"));
    }

    #[test]
    fn cross_backend_equivalence() {
        use crate::Cst;

        let source = "ab";

        // CstData tree
        let mut cd: crate::CstData<TestKind, TestKind> = crate::CstData::new(vec![0..1, 1..2]);
        let rm = cd.start_rule();
        cd.token(TestKind::TokenA, "a");
        cd.token(TestKind::TokenB, "b");
        cd.end_rule(rm, TestKind::Root);
        let cst = Cst::new(source, cd);
        let cst_out = cst.to_string();

        // Cstree tree (same structure)
        let mut cb = CstreeBuilder::<TestKind>::new_empty();
        let rm2 = cb.start_rule();
        cb.token(TestKind::TokenA, "a");
        cb.token(TestKind::TokenB, "b");
        cb.end_rule(rm2, TestKind::Root);
        let green = cb.finish();
        let cstree_out = CstreeDisplay::<TestKind>::new(&green, source).to_string();

        assert_eq!(cst_out, cstree_out, "CSTree display must match CstData display");
    }
}