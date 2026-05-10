use core::fmt;

use crate::parser::{CstBuilder, RuleType, TokenType};
use crate::types::*;

#[derive(Debug, Copy, Clone)]
pub enum Node<T, R> {
    Rule(R, CstIndex),
    Token(T, CstIndex),
}

#[derive(Default)]
pub struct CstChildren<'a, T, R> {
    iter: core::slice::Iter<'a, Node<T, R>>,
    offset: usize,
}

impl<'a, T, R> Iterator for CstChildren<'a, T, R> {
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.offset;
        self.offset += 1;
        if let Some(node) = self.iter.next() {
            if let Node::Rule(_, end_offset) = node {
                let end_offset = usize::from(*end_offset);
                if end_offset > 0 {
                    self.iter.nth(end_offset.saturating_sub(1));
                    self.offset += end_offset;
                }
            }
            Some(NodeRef(offset))
        } else {
            None
        }
    }
}

impl<T: Clone, R: Clone> Clone for CstData<T, R> {
    fn clone(&self) -> Self {
        Self { spans: self.spans.clone(), nodes: self.nodes.clone(), token_count: self.token_count, non_skip_len: self.non_skip_len }
    }
}

#[derive(Debug)]
pub struct CstData<T, R> {
    pub(crate) spans: Vec<Span>,
    pub nodes: Vec<Node<T, R>>,
    pub(crate) token_count: usize,
    non_skip_len: usize,
}

impl<T, R> CstData<T, R>
where
    R: RuleType,
{
    pub fn children(&self, node_ref: NodeRef) -> CstChildren<'_, T, R> {
        let iter = if let Node::Rule(_, end_offset) = self.nodes[node_ref.0] {
            self.nodes[node_ref.0 + 1..node_ref.0 + usize::from(end_offset) + 1].iter()
        } else {
            core::slice::Iter::default()
        };
        CstChildren {
            iter,
            offset: node_ref.0 + 1,
        }
    }
    pub fn get(&self, node_ref: NodeRef) -> Node<T, R>
    where
        T: Clone,
        R: Clone,
    {
        self.nodes[node_ref.0].clone()
    }
    pub fn span(&self, node_ref: NodeRef) -> Span {
        fn find_token<'b, T: 'b, R: 'b>(
            mut iter: impl Iterator<Item = &'b Node<T, R>>,
        ) -> Option<usize> {
            iter.find_map(|node| match node {
                Node::Rule(..) => None,
                Node::Token(_, idx) => Some(usize::from(*idx)),
            })
        }
        match self.nodes[node_ref.0] {
            Node::Token(_, idx) => self.spans[usize::from(idx)].clone(),
            Node::Rule(_, end_offset) => {
                let end = node_ref.0 + usize::from(end_offset);
                let first = find_token(self.nodes[node_ref.0 + 1..=end].iter());
                let last = find_token(self.nodes[node_ref.0 + 1..=end].iter().rev());
                if let (Some(first), Some(last)) = (first, last) {
                    self.spans[first].start..self.spans[last].end
                } else {
                    let offset = find_token(self.nodes[..node_ref.0].iter().rev())
                        .map_or(0, |before| self.spans[before].end);
                    offset..offset
                }
            }
        }
    }
    pub fn match_token(&self, node_ref: NodeRef, matched_token: T) -> Option<Span>
    where
        T: PartialEq,
    {
        match &self.nodes[node_ref.0] {
            Node::Token(token, idx) if *token == matched_token => {
                Some(self.spans[usize::from(*idx)].clone())
            }
            _ => None,
        }
    }
    pub fn match_rule(&self, node_ref: NodeRef, matched_rule: R) -> bool
    where
        R: PartialEq,
    {
        matches!(&self.nodes[node_ref.0], Node::Rule(rule, _) if rule == &matched_rule)
    }
}

impl<T: TokenType, R: RuleType> CstBuilder for CstData<T, R> {
    type Token = T;
    type Rule = R;
    type Mark = usize;
    type Checkpoint = MarkTruncation;
    type Output = CstData<T, R>;

    fn new(spans: Vec<Span>) -> Self {
        Self { spans, nodes: Vec::new(), token_count: 0, non_skip_len: 0 }
    }

    fn token(&mut self, kind: T, _text: &str) {
        self.nodes.push(Node::Token(kind, self.token_count.into()));
        self.token_count += 1;
        self.non_skip_len = self.nodes.len();
    }

    fn token_skip(&mut self, kind: T, _text: &str) {
        self.nodes.push(Node::Token(kind, self.token_count.into()));
        self.token_count += 1;
    }

    fn start_rule(&mut self) -> usize {
        let pos = self.nodes.len();
        self.nodes.push(Node::Rule(R::error(), 0.into()));
        self.non_skip_len = self.nodes.len();
        pos
    }

fn end_rule(&mut self, mark: usize, rule: R) {
        let len = self.non_skip_len - 1;
        self.nodes[mark] = Node::Rule(
            rule,
            (if mark >= len { 0 } else { len - mark }).into(),
        );
    }

    fn end_rule_root(&mut self, mark: usize, rule: R) {
        self.non_skip_len = self.nodes.len();
        self.end_rule(mark, rule);
    }

    fn mark(&self) -> usize { self.nodes.len() }

    fn start_rule_before(&mut self, mark: usize) -> usize {
        self.nodes.insert(mark, Node::Rule(R::error(), 0.into()));
        self.non_skip_len += 1;
        mark
    }

    fn checkpoint(&self) -> MarkTruncation {
        MarkTruncation { node_count: self.nodes.len(), token_count: self.token_count }
    }

    fn revert_to(&mut self, cp: MarkTruncation) {
        self.nodes.truncate(cp.node_count);
        self.token_count = cp.token_count;
        self.non_skip_len = self.nodes.len();
    }

    fn iterate_removed(&self, checkpoint: MarkTruncation, f: &mut dyn FnMut(R, NodeRef)) {
        for i in checkpoint.node_count..self.nodes.len() {
            if let Node::Rule(rule, _) = &self.nodes[i] {
                f(*rule, NodeRef(i));
            }
        }
    }

    fn node_ref(&self, mark: usize) -> Option<NodeRef> { Some(NodeRef(mark)) }

fn finish(self) -> CstData<T, R> { self }
}

#[derive(Debug)]
pub struct Cst<'a, T, R> {
    pub(crate) source: &'a str,
    pub data: CstData<T, R>,
}

#[allow(dead_code)]
impl<'a, T, R> Cst<'a, T, R>
where
    R: RuleType,
{
    pub fn new(source: &'a str, data: CstData<T, R>) -> Self {
        Cst { source, data }
    }
    pub fn source(&self) -> &'a str {
        self.source
    }
    pub fn into_data(self) -> CstData<T, R> {
        self.data
    }
    pub fn children(&self, node_ref: NodeRef) -> CstChildren<'_, T, R> {
        self.data.children(node_ref)
    }
    pub fn get(&self, node_ref: NodeRef) -> Node<T, R>
    where
        T: Clone,
        R: Clone,
    {
        self.data.get(node_ref)
    }
    pub fn span(&self, node_ref: NodeRef) -> Span {
        self.data.span(node_ref)
    }
    pub fn match_token(&self, node_ref: NodeRef, matched_token: T) -> Option<(&'a str, Span)>
    where
        T: PartialEq,
    {
        self.data
            .match_token(node_ref, matched_token)
            .map(|span| (&self.source[span.clone()], span))
    }
    pub fn match_rule(&self, node_ref: NodeRef, matched_rule: R) -> bool
    where
        R: PartialEq,
    {
        self.data.match_rule(node_ref, matched_rule)
    }
    pub fn span_text(&self, span_idx: CstIndex) -> &'a str {
        &self.source[self.data.spans[usize::from(span_idx)].clone()]
    }
}

impl<T: fmt::Debug + Clone, R: RuleType> fmt::Display for Cst<'_, T, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const DEPTH: &str = "    ";
        fn rec<T: fmt::Debug + Clone, R: RuleType>(
            cst: &Cst<'_, T, R>,
            f: &mut fmt::Formatter<'_>,
            node_ref: NodeRef,
            indent: usize,
        ) -> fmt::Result {
            match cst.get(node_ref) {
                Node::Rule(rule, _) => {
                    let span = cst.span(node_ref);
                    writeln!(f, "{}{rule:?} [{span:?}]", DEPTH.repeat(indent))?;
                    for child_node_ref in cst.children(node_ref) {
                        rec(cst, f, child_node_ref, indent + 1)?;
                    }
                    Ok(())
                }
                Node::Token(token, idx) => {
                    let span = &cst.data.spans[usize::from(idx)];
                    writeln!(
                        f,
                        "{}{:?} {:?} [{:?}]",
                        DEPTH.repeat(indent),
                        token,
                        &cst.source[span.clone()],
                        span,
                    )
                }
            }
        }
        rec(self, f, NodeRef::ROOT, 0)
    }
}
