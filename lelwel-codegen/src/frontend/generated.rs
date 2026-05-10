use lelwel::{
    TokenType, RuleType, CstBuilder, ParserHooks, Parser, NodeRef, Cst, Span, CstData, err,
};
impl TokenType for Token {
    #[inline]
    fn is_skip(&self) -> bool {
        matches!(
            self, Token::Error | Token::LineComment | Token::BlockComment |
            Token::DocComment | Token::Whitespace
        )
    }
    #[inline]
    fn is_eof(&self) -> bool {
        matches!(self, Token::EOF)
    }
    #[inline]
    fn eof() -> Self {
        Token::EOF
    }
}
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Rule {
    Error,
    Action,
    Alternation,
    Assertion,
    Commit,
    Concat,
    Decl,
    File,
    Name,
    NodeCreation,
    NodeElision,
    NodeMarker,
    NodeRename,
    Optional,
    OrderedChoice,
    Paren,
    PartDecl,
    Plus,
    Postfix,
    Predicate,
    Regex,
    Return,
    RightDecl,
    RuleDecl,
    SkipDecl,
    Star,
    StartDecl,
    Symbol,
    TokenDecl,
    TokenList,
}
impl RuleType for Rule {
    #[inline]
    fn error() -> Self {
        Rule::Error
    }
}
impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rule::Action => write!(f, "action"),
            Rule::Alternation => write!(f, "alternation"),
            Rule::Assertion => write!(f, "assertion"),
            Rule::Commit => write!(f, "commit"),
            Rule::Concat => write!(f, "concat"),
            Rule::Decl => write!(f, "decl"),
            Rule::Error => write!(f, "error"),
            Rule::File => write!(f, "file"),
            Rule::Name => write!(f, "name"),
            Rule::NodeCreation => write!(f, "node_creation"),
            Rule::NodeElision => write!(f, "node_elision"),
            Rule::NodeMarker => write!(f, "node_marker"),
            Rule::NodeRename => write!(f, "node_rename"),
            Rule::Optional => write!(f, "optional"),
            Rule::OrderedChoice => write!(f, "ordered_choice"),
            Rule::Paren => write!(f, "paren"),
            Rule::PartDecl => write!(f, "part_decl"),
            Rule::Plus => write!(f, "plus"),
            Rule::Postfix => write!(f, "postfix"),
            Rule::Predicate => write!(f, "predicate"),
            Rule::Regex => write!(f, "regex"),
            Rule::Return => write!(f, "return"),
            Rule::RightDecl => write!(f, "right_decl"),
            Rule::RuleDecl => write!(f, "rule_decl"),
            Rule::SkipDecl => write!(f, "skip_decl"),
            Rule::Star => write!(f, "star"),
            Rule::StartDecl => write!(f, "start_decl"),
            Rule::Symbol => write!(f, "symbol"),
            Rule::TokenDecl => write!(f, "token_decl"),
            Rule::TokenList => write!(f, "token_list"),
        }
    }
}
#[allow(clippy::ptr_arg)]
pub trait ParserCallbacks<
    'a,
>: ParserHooks<'a, Token, Rule, Diag = Self::Diagnostic, Ctx = Self::Context> {
    type Diagnostic;
    type Context;
    ///Called at the start of the parse to generate all tokens and corresponding spans.
    fn create_tokens(
        context: &mut Self::Context,
        source: &'a str,
        diags: &mut Vec<Self::Diagnostic>,
    ) -> (Vec<Token>, Vec<Span>);
    ///Called when diagnostic is created.
    fn create_diagnostic(&self, span: Span, message: String) -> Self::Diagnostic;
    ///This predicate can be used to skip normal tokens.
    fn predicate_skip(&self, _token: Token) -> bool {
        false
    }
    ///Called when `action` node is created.
    fn create_node_action(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `alternation` node is created.
    fn create_node_alternation(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `assertion` node is created.
    fn create_node_assertion(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `commit` node is created.
    fn create_node_commit(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `concat` node is created.
    fn create_node_concat(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `decl` node is created.
    fn create_node_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `error` node is created.
    fn create_node_error(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `file` node is created.
    fn create_node_file(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `name` node is created.
    fn create_node_name(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `node_creation` node is created.
    fn create_node_node_creation(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `node_elision` node is created.
    fn create_node_node_elision(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `node_marker` node is created.
    fn create_node_node_marker(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `node_rename` node is created.
    fn create_node_node_rename(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `optional` node is created.
    fn create_node_optional(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `ordered_choice` node is created.
    fn create_node_ordered_choice(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `paren` node is created.
    fn create_node_paren(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `part_decl` node is created.
    fn create_node_part_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `plus` node is created.
    fn create_node_plus(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `postfix` node is created.
    fn create_node_postfix(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `predicate` node is created.
    fn create_node_predicate(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `regex` node is created.
    fn create_node_regex(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `return` node is created.
    fn create_node_return(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `right_decl` node is created.
    fn create_node_right_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `rule_decl` node is created.
    fn create_node_rule_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `skip_decl` node is created.
    fn create_node_skip_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `star` node is created.
    fn create_node_star(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `start_decl` node is created.
    fn create_node_start_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `symbol` node is created.
    fn create_node_symbol(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `token_decl` node is created.
    fn create_node_token_decl(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when `token_list` node is created.
    fn create_node_token_list(
        &mut self,
        _node_ref: NodeRef,
        _diags: &mut Vec<Self::Diagnostic>,
    ) {}
    ///Called when semantic predicate `?1` in rule `decl` is visited.
    fn predicate_decl_1(&self) -> bool;
}
impl<'a, B, Ctx> ParserHooks<'a, Token, Rule> for Parser<'a, B, Ctx>
where
    B: CstBuilder<Token=Token, Rule=Rule>,
    Self: ParserCallbacks<'a>,
{
    type Diag = <Self as ParserCallbacks<'a>>::Diagnostic;
    type Ctx = <Self as ParserCallbacks<'a>>::Context;
    #[inline]
    fn create_tokens_hook(
        ctx: &mut Self::Ctx,
        source: &'a str,
        diags: &mut Vec<Self::Diag>,
    ) -> (Vec<Token>, Vec<Span>) {
        Self::create_tokens(ctx, source, diags)
    }
    #[inline]
    fn create_diagnostic_hook(&self, span: Span, message: String) -> Self::Diag {
        Self::create_diagnostic(self, span, message)
    }
    #[inline]
    fn predicate_skip_hook(&self, token: Token) -> bool {
        self.predicate_skip(token)
    }
    #[inline]
    fn create_node(
        &mut self,
        rule: Rule,
        node_ref: NodeRef,
        diags: &mut Vec<Self::Diag>,
    ) {
        match rule {
            Rule::Action => self.create_node_action(node_ref, diags),
            Rule::Alternation => self.create_node_alternation(node_ref, diags),
            Rule::Assertion => self.create_node_assertion(node_ref, diags),
            Rule::Commit => self.create_node_commit(node_ref, diags),
            Rule::Concat => self.create_node_concat(node_ref, diags),
            Rule::Decl => self.create_node_decl(node_ref, diags),
            Rule::Error => self.create_node_error(node_ref, diags),
            Rule::File => self.create_node_file(node_ref, diags),
            Rule::Name => self.create_node_name(node_ref, diags),
            Rule::NodeCreation => self.create_node_node_creation(node_ref, diags),
            Rule::NodeElision => self.create_node_node_elision(node_ref, diags),
            Rule::NodeMarker => self.create_node_node_marker(node_ref, diags),
            Rule::NodeRename => self.create_node_node_rename(node_ref, diags),
            Rule::Optional => self.create_node_optional(node_ref, diags),
            Rule::OrderedChoice => self.create_node_ordered_choice(node_ref, diags),
            Rule::Paren => self.create_node_paren(node_ref, diags),
            Rule::PartDecl => self.create_node_part_decl(node_ref, diags),
            Rule::Plus => self.create_node_plus(node_ref, diags),
            Rule::Postfix => self.create_node_postfix(node_ref, diags),
            Rule::Predicate => self.create_node_predicate(node_ref, diags),
            Rule::Regex => self.create_node_regex(node_ref, diags),
            Rule::Return => self.create_node_return(node_ref, diags),
            Rule::RightDecl => self.create_node_right_decl(node_ref, diags),
            Rule::RuleDecl => self.create_node_rule_decl(node_ref, diags),
            Rule::SkipDecl => self.create_node_skip_decl(node_ref, diags),
            Rule::Star => self.create_node_star(node_ref, diags),
            Rule::StartDecl => self.create_node_start_decl(node_ref, diags),
            Rule::Symbol => self.create_node_symbol(node_ref, diags),
            Rule::TokenDecl => self.create_node_token_decl(node_ref, diags),
            Rule::TokenList => self.create_node_token_list(node_ref, diags),
        }
    }
    #[inline]
    fn create_node_error_hook(
        &mut self,
        node_ref: NodeRef,
        diags: &mut Vec<Self::Diag>,
    ) {
        self.create_node_error(node_ref, diags)
    }
    #[inline]
    fn delete_node(&mut self, _rule: Rule, _node_ref: NodeRef) {}
}
macro_rules! expect {
    ($token:ident, $msg:literal, $self:expr, $diags:expr) => {
        if let Token:: $token = $self .current { $self .advance(false, $diags); } else {
        $self .error($diags, err![$self, $msg]); }
    };
}
#[allow(unused_macros)]
macro_rules! try_expect {
    ($token:ident, $msg:literal, $self:expr, $diags:expr) => {
        if let Token:: $token = $self .current { $self .advance(false, $diags); } else {
        if $self .builder.in_ordered_choice { return None; } $self .error($diags, err![$self,
        $msg]); }
    };
}
pub trait Rules<'a>: ParserCallbacks<'a> + Sized {
    fn parse(self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule>;
    fn rule_file(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_start_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_right_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_skip_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_part_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_token_list(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_token_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_rule_decl(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_regex(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_alternation(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_ordered_choice(&mut self, diags: &mut Vec<Self::Diagnostic>);
    fn rule_concat(&mut self, diags: &mut Vec<Self::Diagnostic>);
    #[allow(unused_assignments)]
    fn rule_postfix(&mut self, diags: &mut Vec<Self::Diagnostic>);
}
#[allow(clippy::while_let_loop, dead_code, unused_parens)]
impl<'a, Ctx> Rules<'a> for Parser<'a, CstData<Token, Rule>, Ctx>
where
    Self: ParserCallbacks<'a>,
    Ctx: From<<Self as ParserHooks<'a, Token, Rule>>::Ctx>,
    <Self as ParserHooks<'a, Token, Rule>>::Ctx: From<Ctx>,
{
    fn parse(mut self, diags: &mut Vec<Self::Diagnostic>) -> Cst<'a, Token, Rule> {
        self.end_of_input = Token::EOF;
        let source = self.builder.source();
        let data = self.parse_with(|parser, diags| parser.rule_file(diags), diags, Rule::File);
        Cst::new(source, data)
    }
    fn rule_file(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        loop {
            match self.current {
                Token::Id
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.rule_decl(diags);
                }
                Token::EOF => break,
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <end of file>, <identifier>, 'part', 'right', 'skip', 'start', 'token'"
                        ],
                    );
                }
            }
        }
    }
    fn rule_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        match self.current {
            Token::Token => {
                self.rule_token_list(diags);
            }
            Token::Id if self.predicate_decl_1() => {
                self.rule_rule_decl(diags);
            }
            Token::Start => {
                self.rule_start_decl(diags);
            }
            Token::Right => {
                self.rule_right_decl(diags);
            }
            Token::Skip => {
                self.rule_skip_decl(diags);
            }
            Token::Part => {
                self.rule_part_decl(diags);
            }
            Token::Id => {
                self.advance_with_error(diags, err![self, "invalid syntax"]);
            }
            _ => {
                self.error(
                    diags,
                    err![
                        self,
                        "invalid syntax, expected one of: <identifier>, 'part', 'right', 'skip', 'start', 'token'"
                    ],
                );
            }
        }
    }
    fn rule_start_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Start, "invalid syntax, expected: 'start'", self, diags);
        expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
        expect!(Semi, "invalid syntax, expected: ';'", self, diags);
        let closed = self.close(m, Rule::StartDecl, diags);
    }
    fn rule_right_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Right, "invalid syntax, expected: 'right'", self, diags);
        match self.current {
            Token::Id => {
                expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
            }
            Token::Str => {
                expect!(Str, "invalid syntax, expected: <string literal>", self, diags);
            }
            _ => {
                self.error(
                    diags,
                    err![
                        self,
                        "invalid syntax, expected one of: <identifier>, <string literal>"
                    ],
                );
            }
        }
        loop {
            match self.current {
                Token::Id | Token::Str => {
                    match self.current {
                        Token::Id => {
                            expect!(
                                Id, "invalid syntax, expected: <identifier>", self, diags
                            );
                        }
                        Token::Str => {
                            expect!(
                                Str, "invalid syntax, expected: <string literal>", self,
                                diags
                            );
                        }
                        _ => {
                            self.error(
                                diags,
                                err![
                                    self,
                                    "invalid syntax, expected one of: <identifier>, <string literal>"
                                ],
                            );
                        }
                    }
                }
                Token::Semi => break,
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <identifier>, ';', <string literal>"
                        ],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <identifier>, ';', <string literal>"
                        ],
                    );
                }
            }
        }
        expect!(Semi, "invalid syntax, expected: ';'", self, diags);
        let closed = self.close(m, Rule::RightDecl, diags);
    }
    fn rule_skip_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Skip, "invalid syntax, expected: 'skip'", self, diags);
        match self.current {
            Token::Id => {
                expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
            }
            Token::Str => {
                expect!(Str, "invalid syntax, expected: <string literal>", self, diags);
            }
            _ => {
                self.error(
                    diags,
                    err![
                        self,
                        "invalid syntax, expected one of: <identifier>, <string literal>"
                    ],
                );
            }
        }
        loop {
            match self.current {
                Token::Id | Token::Str => {
                    match self.current {
                        Token::Id => {
                            expect!(
                                Id, "invalid syntax, expected: <identifier>", self, diags
                            );
                        }
                        Token::Str => {
                            expect!(
                                Str, "invalid syntax, expected: <string literal>", self,
                                diags
                            );
                        }
                        _ => {
                            self.error(
                                diags,
                                err![
                                    self,
                                    "invalid syntax, expected one of: <identifier>, <string literal>"
                                ],
                            );
                        }
                    }
                }
                Token::Semi => break,
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <identifier>, ';', <string literal>"
                        ],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <identifier>, ';', <string literal>"
                        ],
                    );
                }
            }
        }
        expect!(Semi, "invalid syntax, expected: ';'", self, diags);
        let closed = self.close(m, Rule::SkipDecl, diags);
    }
    fn rule_part_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Part, "invalid syntax, expected: 'part'", self, diags);
        expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
        loop {
            match self.current {
                Token::Id => {
                    expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
                }
                Token::Semi => break,
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![self, "invalid syntax, expected one of: <identifier>, ';'"],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![self, "invalid syntax, expected one of: <identifier>, ';'"],
                    );
                }
            }
        }
        expect!(Semi, "invalid syntax, expected: ';'", self, diags);
        let closed = self.close(m, Rule::PartDecl, diags);
    }
    fn rule_token_list(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Token, "invalid syntax, expected: 'token'", self, diags);
        self.rule_token_decl(diags);
        loop {
            match self.current {
                Token::Id => {
                    self.rule_token_decl(diags);
                }
                Token::Semi => break,
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![self, "invalid syntax, expected one of: <identifier>, ';'"],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![self, "invalid syntax, expected one of: <identifier>, ';'"],
                    );
                }
            }
        }
        expect!(Semi, "invalid syntax, expected: ';'", self, diags);
        let closed = self.close(m, Rule::TokenList, diags);
    }
    fn rule_token_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
        loop {
            match self.current {
                Token::Equal => {
                    expect!(Equal, "invalid syntax, expected: '='", self, diags);
                    expect!(
                        Str, "invalid syntax, expected: <string literal>", self, diags
                    );
                    break;
                }
                Token::Id | Token::Semi => break,
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: '=', <identifier>, ';'"
                        ],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: '=', <identifier>, ';'"
                        ],
                    );
                }
            }
        }
        let closed = self.close(m, Rule::TokenDecl, diags);
    }
    fn rule_rule_decl(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let m = self.open(diags);
        expect!(Id, "invalid syntax, expected: <identifier>", self, diags);
        loop {
            match self.current {
                Token::Hat => {
                    expect!(Hat, "invalid syntax, expected: '^'", self, diags);
                    break;
                }
                Token::Colon => break,
                Token::EOF
                | Token::Id
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![self, "invalid syntax, expected one of: ':', '^'"],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![self, "invalid syntax, expected one of: ':', '^'"],
                    );
                }
            }
        }
        expect!(Colon, "invalid syntax, expected: ':'", self, diags);
        loop {
            match self.current {
                Token::Action
                | Token::And
                | Token::Assertion
                | Token::Hat
                | Token::Id
                | Token::LBrak
                | Token::LPar
                | Token::NodeCreation
                | Token::NodeMarker
                | Token::NodeRename
                | Token::Predicate
                | Token::Str
                | Token::Tilde => {
                    self.rule_regex(diags);
                    break;
                }
                Token::Semi => break,
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, <semantic predicate>, ';', <string literal>, '~'"
                        ],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, <semantic predicate>, ';', <string literal>, '~'"
                        ],
                    );
                }
            }
        }
        expect!(Semi, "invalid syntax, expected: ';'", self, diags);
        let closed = self.close(m, Rule::RuleDecl, diags);
    }
    fn rule_regex(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        self.rule_alternation(diags);
    }
    fn rule_alternation(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let start = self.mark(diags);
        self.rule_ordered_choice(diags);
        loop {
            match self.current {
                Token::Or => {
                    expect!(Or, "invalid syntax, expected: '|'", self, diags);
                    self.rule_ordered_choice(diags);
                    loop {
                        match self.current {
                            Token::Or => {
                                expect!(Or, "invalid syntax, expected: '|'", self, diags);
                                self.rule_ordered_choice(diags);
                            }
                            Token::RBrak | Token::RPar | Token::Semi => break,
                            Token::EOF
                            | Token::Id
                            | Token::Part
                            | Token::Right
                            | Token::Skip
                            | Token::Start
                            | Token::Token => {
                                self.error(
                                    diags,
                                    err![
                                        self, "invalid syntax, expected one of: '|', ']', ')', ';'"
                                    ],
                                );
                                break;
                            }
                            _ => {
                                self.advance_with_error(
                                    diags,
                                    err![
                                        self, "invalid syntax, expected one of: '|', ']', ')', ';'"
                                    ],
                                );
                            }
                        }
                    }
                    let open_node = self.open_before(start, diags);
                    self.close(open_node, Rule::Alternation, diags);
                    break;
                }
                Token::RBrak | Token::RPar | Token::Semi => break,
                Token::EOF
                | Token::Id
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![self, "invalid syntax, expected one of: '|', ']', ')', ';'"],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![self, "invalid syntax, expected one of: '|', ']', ')', ';'"],
                    );
                }
            }
        }
    }
    fn rule_ordered_choice(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let start = self.mark(diags);
        self.rule_concat(diags);
        loop {
            match self.current {
                Token::Slash => {
                    expect!(Slash, "invalid syntax, expected: '/'", self, diags);
                    self.rule_concat(diags);
                    loop {
                        match self.current {
                            Token::Slash => {
                                expect!(
                                    Slash, "invalid syntax, expected: '/'", self, diags
                                );
                                self.rule_concat(diags);
                            }
                            Token::Or | Token::RBrak | Token::RPar | Token::Semi => break,
                            Token::EOF
                            | Token::Id
                            | Token::Part
                            | Token::Right
                            | Token::Skip
                            | Token::Start
                            | Token::Token => {
                                self.error(
                                    diags,
                                    err![
                                        self,
                                        "invalid syntax, expected one of: '|', ']', ')', ';', '/'"
                                    ],
                                );
                                break;
                            }
                            _ => {
                                self.advance_with_error(
                                    diags,
                                    err![
                                        self,
                                        "invalid syntax, expected one of: '|', ']', ')', ';', '/'"
                                    ],
                                );
                            }
                        }
                    }
                    let open_node = self.open_before(start, diags);
                    self.close(open_node, Rule::OrderedChoice, diags);
                    break;
                }
                Token::Or | Token::RBrak | Token::RPar | Token::Semi => break,
                Token::EOF
                | Token::Id
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: '|', ']', ')', ';', '/'"
                        ],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: '|', ']', ')', ';', '/'"
                        ],
                    );
                }
            }
        }
    }
    fn rule_concat(&mut self, diags: &mut Vec<Self::Diagnostic>) {
        let start = self.mark(diags);
        self.rule_postfix(diags);
        loop {
            match self.current {
                Token::Action
                | Token::And
                | Token::Assertion
                | Token::Hat
                | Token::Id
                | Token::LBrak
                | Token::LPar
                | Token::NodeCreation
                | Token::NodeMarker
                | Token::NodeRename
                | Token::Predicate
                | Token::Str
                | Token::Tilde => {
                    self.rule_postfix(diags);
                    loop {
                        match self.current {
                            Token::Action
                            | Token::And
                            | Token::Assertion
                            | Token::Hat
                            | Token::Id
                            | Token::LBrak
                            | Token::LPar
                            | Token::NodeCreation
                            | Token::NodeMarker
                            | Token::NodeRename
                            | Token::Predicate
                            | Token::Str
                            | Token::Tilde => {
                                self.rule_postfix(diags);
                            }
                            Token::Or
                            | Token::RBrak
                            | Token::RPar
                            | Token::Semi
                            | Token::Slash => break,
                            Token::EOF
                            | Token::Part
                            | Token::Right
                            | Token::Skip
                            | Token::Start
                            | Token::Token => {
                                self.error(
                                    diags,
                                    err![
                                        self,
                                        "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, '|', <semantic predicate>, ']', ')', ';', '/', <string literal>, '~'"
                                    ],
                                );
                                break;
                            }
                            _ => {
                                self.advance_with_error(
                                    diags,
                                    err![
                                        self,
                                        "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, '|', <semantic predicate>, ']', ')', ';', '/', <string literal>, '~'"
                                    ],
                                );
                            }
                        }
                    }
                    let open_node = self.open_before(start, diags);
                    self.close(open_node, Rule::Concat, diags);
                    break;
                }
                Token::Or | Token::RBrak | Token::RPar | Token::Semi | Token::Slash => {
                    break;
                }
                Token::EOF
                | Token::Part
                | Token::Right
                | Token::Skip
                | Token::Start
                | Token::Token => {
                    self.error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, '|', <semantic predicate>, ']', ')', ';', '/', <string literal>, '~'"
                        ],
                    );
                    break;
                }
                _ => {
                    self.advance_with_error(
                        diags,
                        err![
                            self,
                            "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, '|', <semantic predicate>, ']', ')', ';', '/', <string literal>, '~'"
                        ],
                    );
                }
            }
        }
    }
    #[allow(unused_assignments)]
    fn rule_postfix(&mut self, diags: &mut Vec<Self::Diagnostic>) {
fn rec<'b, Ctx>(
            parser: &mut Parser<'b, CstData<Token, Rule>, Ctx>,
            diags: &mut Vec<
                <Parser<'b, CstData<Token, Rule>, Ctx> as ParserCallbacks<'b>>::Diagnostic,
            >,
            mut lhs: usize,
        )
        where
            Parser<'b, CstData<Token, Rule>, Ctx>: ParserCallbacks<'b>,
            Ctx: From<
                <Parser<'b, CstData<Token, Rule>, Ctx> as ParserHooks<'b, Token, Rule>>::Ctx,
            >,
            <Parser<
                'b,
                CstData<Token, Rule>,
                Ctx,
            > as ParserHooks<'b, Token, Rule>>::Ctx: From<Ctx>,
        {
            let mut node_kind = Rule::Postfix;
            match parser.current {
                Token::LPar => {
                    let m = parser.open(diags);
                    expect!(LPar, "invalid syntax, expected: '('", parser, diags);
                    loop {
                        match parser.current {
                            Token::Action
                            | Token::And
                            | Token::Assertion
                            | Token::Hat
                            | Token::Id
                            | Token::LBrak
                            | Token::LPar
                            | Token::NodeCreation
                            | Token::NodeMarker
                            | Token::NodeRename
                            | Token::Predicate
                            | Token::Str
                            | Token::Tilde => {
                                parser.rule_regex(diags);
                                break;
                            }
                            Token::RPar => break,
                            Token::EOF
                            | Token::Or
                            | Token::Part
                            | Token::Plus
                            | Token::RBrak
                            | Token::Right
                            | Token::Semi
                            | Token::Skip
                            | Token::Slash
                            | Token::Star
                            | Token::Start
                            | Token::Token => {
                                parser
                                    .error(
                                        diags,
                                        err![
                                            parser,
                                            "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, <semantic predicate>, ')', <string literal>, '~'"
                                        ],
                                    );
                                break;
                            }
                            _ => {
                                parser
                                    .advance_with_error(
                                        diags,
                                        err![
                                            parser,
                                            "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, <semantic predicate>, ')', <string literal>, '~'"
                                        ],
                                    );
                            }
                        }
                    }
                    expect!(RPar, "invalid syntax, expected: ')'", parser, diags);
                    node_kind = Rule::Paren;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::LBrak => {
                    let m = parser.open(diags);
                    expect!(LBrak, "invalid syntax, expected: '['", parser, diags);
                    parser.rule_regex(diags);
                    expect!(RBrak, "invalid syntax, expected: ']'", parser, diags);
                    node_kind = Rule::Optional;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Id => {
                    let m = parser.open(diags);
                    expect!(Id, "invalid syntax, expected: <identifier>", parser, diags);
                    node_kind = Rule::Name;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Str => {
                    let m = parser.open(diags);
                    expect!(
                        Str, "invalid syntax, expected: <string literal>", parser, diags
                    );
                    node_kind = Rule::Symbol;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Predicate => {
                    let m = parser.open(diags);
                    expect!(
                        Predicate, "invalid syntax, expected: <semantic predicate>",
                        parser, diags
                    );
                    node_kind = Rule::Predicate;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Action => {
                    let m = parser.open(diags);
                    expect!(
                        Action, "invalid syntax, expected: <semantic action>", parser,
                        diags
                    );
                    node_kind = Rule::Action;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Assertion => {
                    let m = parser.open(diags);
                    expect!(
                        Assertion, "invalid syntax, expected: <semantic assertion>",
                        parser, diags
                    );
                    node_kind = Rule::Assertion;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::NodeRename => {
                    let m = parser.open(diags);
                    expect!(
                        NodeRename, "invalid syntax, expected: <node rename>", parser,
                        diags
                    );
                    node_kind = Rule::NodeRename;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::NodeMarker => {
                    let m = parser.open(diags);
                    expect!(
                        NodeMarker, "invalid syntax, expected: <node marker>", parser,
                        diags
                    );
                    node_kind = Rule::NodeMarker;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::NodeCreation => {
                    let m = parser.open(diags);
                    expect!(
                        NodeCreation, "invalid syntax, expected: <node creation>",
                        parser, diags
                    );
                    node_kind = Rule::NodeCreation;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Hat => {
                    let m = parser.open(diags);
                    expect!(Hat, "invalid syntax, expected: '^'", parser, diags);
                    node_kind = Rule::NodeElision;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::Tilde => {
                    let m = parser.open(diags);
                    expect!(Tilde, "invalid syntax, expected: '~'", parser, diags);
                    node_kind = Rule::Commit;
                    let closed = parser.close(m, node_kind, diags);
                }
                Token::And => {
                    let m = parser.open(diags);
                    expect!(And, "invalid syntax, expected: '&'", parser, diags);
                    node_kind = Rule::Return;
                    let closed = parser.close(m, node_kind, diags);
                }
                _ => {
                    parser
                        .error(
                            diags,
                            err![
                                parser,
                                "invalid syntax, expected one of: <semantic action>, '&', <semantic assertion>, '^', <identifier>, '[', '(', <node creation>, <node marker>, <node rename>, <semantic predicate>, <string literal>, '~'"
                            ],
                        );
                }
            }
            loop {
                node_kind = Rule::Postfix;
                match parser.current {
                    Token::Star => {
                        let m = parser.open_before(lhs, diags);
                        expect!(Star, "invalid syntax, expected: '*'", parser, diags);
                        node_kind = Rule::Star;
                        let closed = parser.close(m, node_kind, diags);
                        lhs = closed;
                        continue;
                    }
                    Token::Plus => {
                        let m = parser.open_before(lhs, diags);
                        expect!(Plus, "invalid syntax, expected: '+'", parser, diags);
                        node_kind = Rule::Plus;
                        let closed = parser.close(m, node_kind, diags);
                        lhs = closed;
                        continue;
                    }
                    _ => {
                        break;
                    }
                }
            }
        }
        let lhs = self.mark(diags);
        rec(self, diags, lhs);
    }
}
