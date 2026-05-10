use crate::{CstBuilder, Span};
use cstree::build::{Checkpoint, GreenNodeBuilder};
use cstree::green::GreenNode;
use cstree::Syntax;

/// [`CstBuilder`] implementation backed by `cstree`'s [`GreenNodeBuilder`].
///
/// Unlike [`CstData`](crate::CstData) (which uses separate token and rule types), cstree
/// requires a single [`Syntax`] type that represents both tokens and rules.
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

    fn new(_spans: Vec<Span>) -> Self {
        CstreeBuilder::new_empty()
    }

    fn token(&mut self, kind: S, text: &str) {
        if S::static_text(kind).is_some() {
            self.inner.static_token(kind);
        } else {
            self.inner.token(kind, text);
        }
    }

    fn start_rule(&mut self) -> Checkpoint {
        let cp = self.inner.checkpoint();
        // NB: we don't call start_node here; start_node_at in end_rule handles it
        cp
    }

    fn end_rule(&mut self, mark: Checkpoint, rule: S) {
        self.inner.start_node_at(mark, rule);
        self.inner.finish_node();
    }

    fn mark(&self) -> Checkpoint {
        self.inner.checkpoint()
    }

    fn start_rule_before(&mut self, mark: Checkpoint) -> Checkpoint {
        // cstree handles retroactive wrapping via start_node_at
        mark
    }

    fn checkpoint(&self) -> Checkpoint {
        self.inner.checkpoint()
    }

    fn revert_to(&mut self, cp: Checkpoint) {
        self.inner.revert_to(cp);
    }

    fn finish(self) -> GreenNode {
        self.inner.finish().0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CstBuilder;
    use cstree::RawSyntaxKind;

    /// A minimal syntax kind for testing.
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

    #[test]
    fn build_simple_tree() {
        let mut builder = CstreeBuilder::<TestKind>::new_empty();
        let m = builder.start_rule();
        builder.token(TestKind::TokenA, "hello");
        builder.token(TestKind::TokenB, "world");
        builder.end_rule(m, TestKind::Root);
        let tree = builder.finish();

        assert_eq!(tree.kind(), TestKind::Root.into_raw());
        assert_eq!(tree.children().count(), 2);
    }

    #[test]
    fn build_nested_tree() {
        let mut builder = CstreeBuilder::<TestKind>::new_empty();
        let root = builder.start_rule();
        let inner = builder.start_rule();
        builder.token(TestKind::TokenA, "a");
        builder.end_rule(inner, TestKind::Rule);
        builder.token(TestKind::TokenB, "b");
        builder.end_rule(root, TestKind::Root);

        let tree = builder.finish();
        assert_eq!(tree.kind(), TestKind::Root.into_raw());
        assert_eq!(tree.children().count(), 2);
    }

    #[test]
    fn build_with_extra_tokens() {
        let mut builder = CstreeBuilder::<TestKind>::new_empty();
        let m = builder.start_rule();
        builder.token(TestKind::TokenA, "hello");
        builder.token(TestKind::TokenA, " ");
        builder.token(TestKind::TokenB, "world");
        builder.end_rule(m, TestKind::Root);

        let tree = builder.finish();
        assert_eq!(tree.children().count(), 3);
    }

    #[test]
    fn revert_checkpoint() {
        let mut builder = CstreeBuilder::<TestKind>::new_empty();
        let root = builder.start_rule();
        let cp = builder.checkpoint();
        builder.token(TestKind::TokenA, "discard");
        builder.revert_to(cp);
        builder.token(TestKind::TokenB, "keep");
        builder.end_rule(root, TestKind::Root);

        let tree = builder.finish();
        assert_eq!(tree.children().count(), 1);
    }

    #[test]
    fn mark_and_rule_before() {
        // cstree: start_rule_before + end_rule wraps content since checkpoint.
        let mut builder = CstreeBuilder::<TestKind>::new_empty();
        let root = builder.start_rule();
        let before = builder.mark();
        builder.token(TestKind::TokenA, "a");
        let wrapped = builder.start_rule_before(before);
        builder.token(TestKind::TokenB, "b");
        builder.end_rule(wrapped, TestKind::Rule);
        builder.end_rule(root, TestKind::Root);

        let tree = builder.finish();
        assert_eq!(tree.children().count(), 1);
        // In cstree, start_rule_before wraps ALL content since the mark.
        let first_child = tree.children().next().unwrap();
        let wrapper = first_child.as_node().unwrap();
        assert_eq!(wrapper.kind(), TestKind::Rule.into_raw());
        assert_eq!(wrapper.children().count(), 2);
    }
}