macro_rules! err {{
    [$self:expr, $msg:literal] => {{
        $self.create_diagnostic($self.span(), String::from($msg))
    }}
}}

#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Rule {{{0}
}}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NodeRef(pub usize);

impl NodeRef {{
    #[allow(dead_code)]
    pub const ROOT: NodeRef = NodeRef(0);
}}

#[cfg(target_pointer_width = "64")]
#[derive(Copy, Clone)]
pub struct CstIndex([u8; 6]);

#[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
#[derive(Copy, Clone)]
pub struct CstIndex(usize);

impl std::fmt::Debug for CstIndex {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {{
        usize::from(*self).fmt(f)
    }}
}}

impl From<CstIndex> for usize {{
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn from(value: CstIndex) -> Self {{
        let [b0, b1, b2, b3, b4, b5] = value.0;
        usize::from_le_bytes([b0, b1, b2, b3, b4, b5, 0, 0])
    }}
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    #[inline]
    fn from(value: CstIndex) -> Self {{
        value.0
    }}
}}
impl From<usize> for CstIndex {{
    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn from(value: usize) -> Self {{
        let [b0, b1, b2, b3, b4, b5, b6, b7] = value.to_le_bytes();
        debug_assert!(b6 == 0 && b7 == 0);
        Self([b0, b1, b2, b3, b4, b5])
    }}
    #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
    #[inline]
    fn from(value: usize) -> Self {{
        Self(value)
    }}
}}

/// Type of a node in the CST.
///
/// The nodes for rules contain the offset to their last child node.
/// The nodes for tokens contain an index to their span.
///
/// On 64 bit platforms offsets and indices are stored as 48 bit integers.
/// This allows the `Node` type to be 8 bytes in size as long as the `Rule`
/// and `Token` enums are one byte in size.
#[derive(Debug, Copy, Clone)]
pub enum Node {{
    Rule(Rule, CstIndex),
    Token(Token, CstIndex),
}}

#[derive(Clone, Copy)]
struct MarkOpened(usize);
#[derive(Clone, Copy)]
struct MarkClosed(usize);
#[derive(Clone)]
struct MarkTruncation {{
    node_count: usize,
    token_count: usize,
    non_skip_len: usize,
    open: usize,
    starts_len: usize,
}}

/// An iterator for child nodes of a CST node.
#[derive(Default)]
pub struct CstChildren<'a> {{
    iter: std::slice::Iter<'a, Node>,
    offset: usize,
}}
impl Iterator for CstChildren<'_> {{
    type Item = NodeRef;

    fn next(&mut self) -> Option<Self::Item> {{
        let offset = self.offset;
        self.offset += 1;
        if let Some(node) = self.iter.next() {{
            if let Node::Rule(_, end_offset) = node {{
                let end_offset = usize::from(*end_offset);
                if end_offset > 0 {{
                    self.iter.nth(end_offset.saturating_sub(1));
                    self.offset += end_offset;
                }}
            }}
            Some(NodeRef(offset))
        }} else {{
            None
        }}
    }}
}}

pub type Span = core::ops::Range<usize>;

#[derive(Debug)]
pub struct CstData {{
    spans: Vec<Span>,
    nodes: Vec<Node>,
    token_count: usize,
    non_skip_len: usize,
    open: usize,
    starts: Vec<usize>,
}}
#[allow(dead_code)]
impl CstData {{
    fn new(spans: Vec<Span>) -> Self {{
        let nodes = Vec::with_capacity(spans.len() * 2);
        Self {{
            spans,
            nodes,
            token_count: 0,
            non_skip_len: 0,
            open: 0,
            starts: vec![],
        }}
    }}
    fn flush_open(&mut self) {{
        if self.open == 0 {{
            return;
        }}
        for _ in 0..self.open {{
            self.starts.push(self.nodes.len());
            self.nodes.push(Node::Rule(Rule::Error, 0.into()));
        }}
        self.open = 0;
        self.non_skip_len = self.nodes.len();
    }}
    fn open(&mut self) -> MarkOpened {{
        self.open += 1;
        if self.nodes.len() == 0 {{
            self.flush_open();
        }}
        MarkOpened(self.starts.len() + self.open)
    }}
    fn close(&mut self, mark: MarkOpened, rule: Rule) -> MarkClosed {{
        self.flush_open();
        assert_eq!(mark.0, self.starts.len());
        let start = self.starts.pop().expect("no start");
        let len = self.non_skip_len - 1;
        self.nodes[start] = Node::Rule(
            rule,
            if start > len {{
                self.non_skip_len += start - len;
                0
            }} else {{
                len - start
            }}
            .into(),
        );
        MarkClosed(start)
    }}
    fn close_root(&mut self, mark: MarkOpened, rule: Rule) -> MarkClosed {{
        self.flush_open();
        assert_eq!(mark.0, self.starts.len());
        assert_eq!(mark.0, 1);
        let start = self.starts.pop().expect("no start");
        assert_eq!(start, 0);
        self.nodes[start] = Node::Rule(rule, (self.nodes.len() - start - 1).into());
        MarkClosed(start)
    }}
    fn advance(&mut self, token: Token, skip: bool) {{
        if !skip {{
            self.flush_open();
        }}
        self.nodes.push(Node::Token(token, self.token_count.into()));
        self.token_count += 1;
        if !skip {{
            self.non_skip_len = self.nodes.len();
        }}
    }}
    fn open_before(&mut self, mark: MarkClosed, is_skipped: fn(Token) -> bool) -> MarkOpened {{
        self.flush_open();
        let from = self.starts
            .last()
            .map_or(mark.0, |&s| s + 1)
            .max(mark.0);
        let i = self.nodes[from..]
            .iter()
            .position(|node| match node {{
                Node::Rule(_, _) => true,
                Node::Token(token, _) => !is_skipped(*token),
            }})
            .map_or(self.nodes.len(), |pos| from + pos);
        self.nodes.insert(i, Node::Rule(Rule::Error, 0.into()));
        self.starts.push(i);
        self.non_skip_len += 1;
        MarkOpened(self.starts.len())
    }}
    fn mark(&self) -> MarkClosed {{
        MarkClosed(self.nodes.len())
    }}
    fn mark_truncation(&self) -> MarkTruncation {{
        MarkTruncation {{
            node_count: self.nodes.len(),
            token_count: self.token_count,
            non_skip_len: self.non_skip_len,
            open: self.open,
            starts_len: self.starts.len(),
        }}
    }}
    fn truncate(&mut self, mark: MarkTruncation) {{
        self.nodes.truncate(mark.node_count);
        self.token_count = mark.token_count;
        self.non_skip_len = mark.non_skip_len;
        self.open = mark.open;
        self.starts.truncate(mark.starts_len);
    }}
    pub fn children(&self, node_ref: NodeRef) -> CstChildren<'_> {{
        let iter = if let Node::Rule(_, end_offset) = self.nodes[node_ref.0] {{
            self.nodes[node_ref.0 + 1..node_ref.0 + usize::from(end_offset) + 1].iter()
        }} else {{
            std::slice::Iter::default()
        }};
        CstChildren {{
            iter,
            offset: node_ref.0 + 1,
        }}
    }}
    pub fn get(&self, node_ref: NodeRef) -> Node {{
        self.nodes[node_ref.0]
    }}
    pub fn span(&self, node_ref: NodeRef) -> Span {{
        fn find_token<'a>(mut iter: impl Iterator<Item = &'a Node>) -> Option<usize> {{
            iter.find_map(|node| match node {{
                Node::Rule(..) => None,
                Node::Token(_, idx) => Some(usize::from(*idx)),
            }})
        }}
        match self.nodes[node_ref.0] {{
            Node::Token(_, idx) => self.spans[usize::from(idx)].clone(),
            Node::Rule(_, end_offset) => {{
                let end = node_ref.0 + usize::from(end_offset);
                let first = find_token(self.nodes[node_ref.0 + 1..=end].iter());
                let last = find_token(self.nodes[node_ref.0 + 1..=end].iter().rev());
                if let (Some(first), Some(last)) = (first, last) {{
                    self.spans[first].start..self.spans[last].end
                }} else {{
                    let offset = find_token(self.nodes[..node_ref.0].iter().rev())
                        .map_or(0, |before| self.spans[before].end);
                    offset..offset
                }}
            }}
        }}
    }}
    pub fn match_token(&self, node_ref: NodeRef, matched_token: Token) -> Option<Span> {{
        match self.nodes[node_ref.0] {{
            Node::Token(token, idx) if token == matched_token => {{
                Some(self.spans[usize::from(idx)].clone())
            }}
            _ => None,
        }}
    }}
    pub fn match_rule(&self, node_ref: NodeRef, matched_rule: Rule) -> bool {{
        matches!(self.nodes[node_ref.0], Node::Rule(rule, _) if rule == matched_rule)
    }}
}}

/// A concrete syntax tree (CST) type.
///
/// Nodes are laid out linearly in memory.
/// Spans for tokens are directly stored in the `spans` vector.
/// Spans for rule nodes are calculated based on their contained token nodes.
///
/// # Example
/// This syntax tree
/// ```text
/// foo
///   bar
///     A
///     B
///   C
/// ```
/// will have the following `nodes` vector.
/// ```text
/// [
///    Node::Rule(Rule::Foo, 4),
///    Node::Rule(Rule::Bar, 2),
///    Node::Token(Token::A, 0),
///    Node::Token(Token::B, 1),
///    Node::Token(Token::C, 2),
/// ]
/// ```
#[derive(Debug)]
pub struct Cst<'a> {{
    source: &'a str,
    data: CstData,
}}
#[allow(dead_code)]
impl<'a> Cst<'a> {{
    pub fn source(&self) -> &'a str {{
        self.source
    }}
    pub fn into_data(self) -> CstData {{
        self.data
    }}
    /// Returns an iterator over the children of the node referenced by `node_ref`.
    pub fn children(&self, node_ref: NodeRef) -> CstChildren<'_> {{
        self.data.children(node_ref)
    }}
    /// Returns the node referenced by `node_ref`.
    pub fn get(&self, node_ref: NodeRef) -> Node {{
        self.data.get(node_ref)
    }}
    /// Returns the span for the node referenced by `node_ref`.
    ///
    /// For rules the span is calculated based on the first and last token.
    /// If there are no tokens the function returns `None`.
    pub fn span(&self, node_ref: NodeRef) -> Span {{
        self.data.span(node_ref)
    }}
    /// Returns the slice and span of the node referenced by `node_ref` if it matches `matched_token`.
    pub fn match_token(&self, node_ref: NodeRef, matched_token: Token) -> Option<(&'a str, Span)> {{
        self.data.match_token(node_ref, matched_token).map(|span| (&self.source[span.clone()], span))
    }}
    /// Checks if the node referenced by `node_ref` matches `matched_rule`.
    pub fn match_rule(&self, node_ref: NodeRef, matched_rule: Rule) -> bool {{
        self.data.match_rule(node_ref, matched_rule)
    }}
    /// Returns the source text corresponding to the span index
    pub fn span_text(&self, span_idx: CstIndex) -> &'a str {{
        &self.source[self.data.spans[usize::from(span_idx)].clone()]
    }}
}}

impl std::fmt::Display for Cst<'_> {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        const DEPTH: &str = "    ";
        fn rec(
            cst: &Cst<'_>,
            f: &mut std::fmt::Formatter<'_>,
            node_ref: NodeRef,
            indent: usize,
        ) -> std::fmt::Result {{
            match cst.get(node_ref) {{
                Node::Rule(rule, _) => {{
                    let span = cst.span(node_ref);
                    writeln!(f, "{{}}{{rule:?}} [{{span:?}}]", DEPTH.repeat(indent))?;
                    for child_node_ref in cst.children(node_ref) {{
                        rec(cst, f, child_node_ref, indent + 1)?;
                    }}
                    Ok(())
                }}
                Node::Token(token, idx) => {{
                    let span = &cst.data.spans[usize::from(idx)];
                    writeln!(
                        f,
                        "{{}}{{:?}} {{:?}} [{{:?}}]",
                        DEPTH.repeat(indent),
                        token,
                        &cst.source[span.clone()],
                        span,
                    )
                }}
            }}
        }}
        rec(self, f, NodeRef::ROOT, 0)
    }}
}}

impl std::fmt::Debug for Rule {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        match self {{{4}
        }}
    }}
}}

macro_rules! expect {{
    ($token:ident, $msg:literal, $self:expr, $diags:expr) => {{
        if let Token::$token = $self.current($diags) {{
            $self.advance(false, $diags);
        }} else {{
            $self.error($diags, err![$self, $msg]);
        }}
    }};
}}
#[allow(unused_macros)]
macro_rules! try_expect {{
    ($token:ident, $msg:literal, $self:expr, $diags:expr) => {{
        if let Token::$token = $self.current($diags) {{
            $self.advance(false, $diags);
        }} else {{
            if $self.in_ordered_choice {{
                return None;
            }}
            $self.error($diags, err![$self, $msg]);
        }}
    }};
}}

struct ParserState<S> {{
    pos: usize,
    current: Option<Token>,
    truncation_mark: MarkTruncation,
    diag_count: usize,
    state: S,
    lexed: usize,
}}
pub struct Parser<'a> {{
    cst: Cst<'a>,
    tokens: Vec<Token>,
    pos: usize,
    current: Option<Token>,
    end_of_input: Token,
    max_offset: usize,
    #[allow(dead_code)]
    context: <Self as ParserCallbacks<'a>>::Context,
    error_node: Option<MarkOpened>,
    #[allow(dead_code)]
    in_ordered_choice: bool,
    error_since_advance: bool,
    state: <Self as ParserCallbacks<'a>>::State,
}}
#[allow(clippy::while_let_loop, dead_code, unused_parens)]
impl<'a> Parser<'a> {{
    fn active_error(&self) -> bool {{
        self.error_node.is_some() || self.error_since_advance
    }}
    fn error(
        &mut self,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>,
        diag: <Self as ParserCallbacks<'a>>::Diagnostic
    ) {{
        if self.active_error() {{
            return;
        }}
        self.error_since_advance = true;
        diags.push(diag);
    }}
    fn token(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> Option<Token> {{
        if self.pos < self.tokens.len() {{
            return Some(self.tokens[self.pos])
        }}

        let (token, span) = self.lex(diags)?;

        self.tokens.push(token);
        self.cst.data.spans.push(span);

        Some(token)
    }}
    fn current(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> Token {{
        if let Some(token) = self.current {{
            return token;
        }}

        loop {{
            match self.token(diags) {{
                Some(token @ (Token::Error{1})) => {{
                    self.pos += 1;
                    self.cst.data.advance(token, true);
                }}
                Some(token) if self.predicate_skip(token) => {{
                    self.pos += 1;
                    self.cst.data.advance(token, true);
                }}
                Some(token) => {{
                    self.current = Some(token);
                    return token;
                }}
                None => {{
                    self.current = Some(self.end_of_input);
                    return self.end_of_input;
                }}
            }}
        }}
    }}
    fn advance(&mut self, error: bool, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) {{
        if !error {{
            self.close_error_node(diags);
            self.error_since_advance = false;
        }}
        let token = self.current.take().expect("advance before current");
        self.cst.data.advance(token, false);
        self.pos += 1;
    }}
    fn is_skipped(token: Token) -> bool {{
        matches!(token, Token::Error{1})
    }}
    fn advance_with_error(
        &mut self,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>,
        diag: <Self as ParserCallbacks<'a>>::Diagnostic
    ) {{
        self.error(diags, diag);
        if self.error_node.is_none() {{
            self.error_node = Some(self.cst.data.open());
        }}
        self.advance(true, diags);
    }}
    fn close_error_node(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) {{
        if let Some(error_node) = self.error_node {{
            let closed = self.cst.data.close(error_node, Rule::Error);
            self.create_node_error(NodeRef(closed.0), diags);
            self.error_node = None;
        }}
    }}
    fn open(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> MarkOpened {{
        self.close_error_node(diags);
        self.cst.data.open()
    }}
    fn open_before(&mut self, mark: MarkClosed, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> MarkOpened {{
        self.close_error_node(diags);
        self.cst.data.open_before(mark, Self::is_skipped)
    }}
    fn close(&mut self, mark: MarkOpened, rule: Rule, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> MarkClosed {{
        self.close_error_node(diags);
        self.cst.data.close(mark, rule)
    }}
    fn close_root(&mut self, mark: MarkOpened, rule: Rule, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> MarkClosed {{
        self.close_error_node(diags);
        self.cst.data.close_root(mark, rule)
    }}
    fn mark(&mut self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> MarkClosed {{
        self.close_error_node(diags);
        self.cst.data.mark()
    }}
    fn span(&self) -> Span {{
        self.cst.data.spans
            .get(self.pos)
            .map_or(self.max_offset..self.max_offset, |span| span.clone())
    }}
    fn get_state(&self, diags: &[<Self as ParserCallbacks<'a>>::Diagnostic]) -> ParserState<<Self as ParserCallbacks<'a>>::State> {{
        ParserState {{
            pos: self.pos,
            current: self.current,
            truncation_mark: self.cst.data.mark_truncation(),
            diag_count: diags.len(),
            state: self.state.clone(),
            lexed: self.tokens.len(),
        }}
    }}
    fn set_state(
        &mut self,
        state: &ParserState<<Self as ParserCallbacks<'a>>::State>,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>
    ) {{
        self.pos = state.pos;
        self.current = state.current;
        diags.truncate(state.diag_count);
        for i in state.truncation_mark.node_count..self.cst.data.nodes.len() {{
            if let Node::Rule(rule, _) = self.cst.data.nodes[i] {{
                self.delete_node(rule, NodeRef(i));
            }}
        }}
        self.cst.data.truncate(state.truncation_mark.clone());
        self.state = state.state.clone();
        self.tokens.truncate(state.lexed);
        self.cst.data.spans.truncate(state.lexed);
    }}
    fn create_node(
        &mut self,
        rule: Rule,
        node_ref: NodeRef,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>
    ) {{
        match rule {{{5}
        }}
    }}
    fn delete_node(&mut self, _rule: Rule, _node_ref: NodeRef) {{
        {6}
    }}
    pub fn new_with_context(
        source: &'a str,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>,
        mut context: <Self as ParserCallbacks<'a>>::Context,
        state: <Self as ParserCallbacks<'a>>::State,
    ) -> Parser<'a> {{
        let (tokens, spans) = Self::create_tokens(&mut context, source, diags);
        let max_offset = source.len();
        Self {{
            current: None,
            end_of_input: Token::EOF,
            cst: Cst {{ data: CstData::new(spans), source }},
            tokens,
            pos: 0,
            max_offset,
            context,
            error_node: None,
            in_ordered_choice: false,
            error_since_advance: false,
            state,
        }}
    }}
    pub fn new(
        source: &'a str,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>,
    ) -> Parser<'a>
    where
        for<'trivial_bound> <Self as ParserCallbacks<'a>>::Context: Default,
        for<'trivial_bound> <Self as ParserCallbacks<'a>>::State: Default,
    {{
        #[allow(clippy::unit_arg)]
        Self::new_with_context(source, diags, <Self as ParserCallbacks<'a>>::Context::default(), <Self as ParserCallbacks<'a>>::State::default())
    }}
    fn parse_rule<RuleParser: Fn(&mut Self, &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>)>(
        mut self,
        rule: RuleParser,
        diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>,
        root: Rule,
    ) -> Cst<'a> {{
        let m = self.open(diags);

        rule(&mut self, diags);

        self.close_error_node(diags);
        if self.current(diags) != self.end_of_input {{
            self.error(diags, err![self, "invalid syntax, expected: <end of file>"]);
            let error_tree = self.open(diags);

            while let token = self.current(diags) && token != self.end_of_input {{
                self.advance(false, diags);
            }}

            let closed = self.cst.data.close(error_tree, Rule::Error);
            self.create_node_error(NodeRef(closed.0), diags);
        }}

        let closed = self.cst.data.close_root(m, root);
        self.create_node(root, NodeRef(closed.0), diags);
        self.cst
    }}
    /// Returns the CST for a parse of the start rule
    pub fn parse(self, diags: &mut Vec<<Self as ParserCallbacks<'a>>::Diagnostic>) -> Cst<'a> {{
        self.parse_rule(|parser, diags| parser.rule_{2}(diags), diags, Rule::{3})
    }}
