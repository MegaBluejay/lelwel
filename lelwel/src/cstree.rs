use cstree::build::{Checkpoint, GreenNodeBuilder, NodeCache};
use cstree::green::GreenNode;
use cstree::interning::TokenInterner;
use cstree::Syntax;

use crate::{CstBuilder, Span};

impl<S: Syntax> CstBuilder for GreenNodeBuilder<'static, 'static, S> {
    type Token = S;
    type Rule = S;
    type Mark = Checkpoint;
    type Checkpoint = Checkpoint;
    type Output = (GreenNode, Option<NodeCache<'static, TokenInterner>>);

    fn new(_spans: Vec<Span>) -> Self {
        GreenNodeBuilder::new()
    }

    fn token(&mut self, kind: S, text: &str) {
        self.token(kind, text);
    }

    fn start_rule(&mut self) -> Checkpoint {
        self.checkpoint()
    }

    fn end_rule(&mut self, mark: Checkpoint, rule: S) {
        self.start_node_at(mark, rule);
        self.finish_node();
    }

    fn mark(&self) -> Checkpoint {
        self.checkpoint()
    }

    fn start_rule_before(&mut self, mark: Checkpoint) -> Checkpoint {
        mark
    }

    fn checkpoint(&self) -> Checkpoint {
        self.checkpoint()
    }

    fn revert_to(&mut self, cp: Checkpoint) {
        self.revert_to(cp);
    }

    fn finish(self) -> (GreenNode, Option<NodeCache<'static, TokenInterner>>) {
        self.finish()
    }
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
        fn into_raw(self) -> RawSyntaxKind {
            RawSyntaxKind(self as u32)
        }
        fn static_text(self) -> Option<&'static str> {
            None
        }
    }

    impl crate::TokenType for TestKind {
        fn is_skip(&self) -> bool {
            false
        }
        fn is_eof(&self) -> bool {
            false
        }
        fn eof() -> Self {
            TestKind::Root
        }
    }

    impl crate::RuleType for TestKind {
        fn error() -> Self {
            TestKind::Root
        }
    }

    #[test]
    fn build_simple_tree() {
        let mut b = GreenNodeBuilder::<TestKind>::new();
        let m = b.start_rule();
        b.token(TestKind::TokenA, "hello");
        b.token(TestKind::TokenB, "world");
        b.end_rule(m, TestKind::Root);
        let (tree, _cache) = b.finish();
        assert_eq!(tree.children().count(), 2);
    }

    #[test]
    fn build_nested_tree() {
        let mut b = GreenNodeBuilder::<TestKind>::new();
        let root = b.start_rule();
        let inner = b.start_rule();
        b.token(TestKind::TokenA, "a");
        b.end_rule(inner, TestKind::Rule);
        b.token(TestKind::TokenB, "b");
        b.end_rule(root, TestKind::Root);
        let (tree, _cache) = b.finish();
        assert_eq!(tree.children().count(), 2);
    }

    #[test]
    fn revert_checkpoint() {
        let mut b = GreenNodeBuilder::<TestKind>::new();
        let root = b.start_rule();
        let cp = b.checkpoint();
        b.token(TestKind::TokenA, "discard");
        b.revert_to(cp);
        b.token(TestKind::TokenB, "keep");
        b.end_rule(root, TestKind::Root);
        let (tree, _cache) = b.finish();
        assert_eq!(tree.children().count(), 1);
    }

    #[test]
    fn finish_returns_cache() {
        let mut b = GreenNodeBuilder::<TestKind>::new();
        let m = b.start_rule();
        b.token(TestKind::TokenA, "hello");
        b.end_rule(m, TestKind::Root);
        let (_tree, cache) = b.finish();
        assert!(cache.is_some());
        let interner = cache.unwrap().into_interner();
        assert!(interner.is_some());
    }
}