//! 分层架构深度研究 v2
//! Syntax → Semantic → Pattern → Domain 四层转换实现
//!
//! 本实现探索：
//! 1. 类型安全的层间接口
//! 2. 渐进式边界检查
//! 3. 零成本抽象
//! 4. 增量处理支持

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

// ============================================================================
// Layer 0: 核心基础设施 - 层间通信协议
// ============================================================================

/// 层标识符，用于编译期类型检查
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerId {
    Syntax,
    Semantic,
    Pattern,
    Domain,
}

/// 层间转换结果
pub type LayerResult<T> = Result<T, LayerError>;

/// 层错误类型
#[derive(Debug, Clone)]
pub enum LayerError {
    SyntaxError(String),
    TypeError(String),
    PatternError(String),
    DomainError(String),
    BoundaryViolation { from: LayerId, to: LayerId, reason: String },
}

impl fmt::Display for LayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerError::SyntaxError(msg) => write!(f, "Syntax Error: {}", msg),
            LayerError::TypeError(msg) => write!(f, "Type Error: {}", msg),
            LayerError::PatternError(msg) => write!(f, "Pattern Error: {}", msg),
            LayerError::DomainError(msg) => write!(f, "Domain Error: {}", msg),
            LayerError::BoundaryViolation { from, to, reason } => {
                write!(f, "Boundary Violation ( {:?} -> {:?} ): {}", from, to, reason)
            }
        }
    }
}

impl std::error::Error for LayerError {}

/// 核心Layer trait - 定义层的基本契约
///
/// 设计原则：
/// 1. 每一层有明确的输入/输出类型
/// 2. 层间转换是显式的、可追踪的
/// 3. 支持增量处理
pub trait Layer {
    /// 层标识
    const ID: LayerId;

    /// 输入类型（来自上层或源代码）
    type Input;

    /// 输出类型（传递给下层）
    type Output;

    /// 层上下文，存储层特定的状态
    type Context;

    /// 执行层转换
    fn transform(input: Self::Input, ctx: &mut Self::Context) -> LayerResult<Self::Output>;
}

/// 层间边界检查器
///
/// 实现渐进类型边界检查的思想：
/// - 在层间插入运行时检查（如果需要）
/// - 提供静态类型保证（编译期）
pub struct LayerBoundary<From: Layer, To: Layer> {
    _phantom: PhantomData<(From, To)>,
    check_runtime: bool,
}

impl<From: Layer, To: Layer> LayerBoundary<From, To> {
    pub fn new(check_runtime: bool) -> Self {
        Self {
            _phantom: PhantomData,
            check_runtime,
        }
    }

    /// 验证层间转换的合法性
    pub fn validate(&self, output: &From::Output) -> LayerResult<()> {
        // 这里可以插入运行时检查逻辑
        if self.check_runtime {
            // 运行时边界检查
            self.runtime_check(output)?;
        }
        Ok(())
    }

    fn runtime_check(&self, _output: &From::Output) -> LayerResult<()> {
        // 具体检查逻辑由实现决定
        Ok(())
    }
}

// ============================================================================
// Layer 1: Syntax Layer - 语法解析层
// ============================================================================

/// 源代码位置信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            line: 0,
            column: 0,
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line,
            column: self.column,
        }
    }
}

/// 词法单元类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // 关键字
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Struct,
    Enum,
    Impl,
    Trait,
    Type,

    // 标识符和字面量
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // 运算符
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,

    // 分隔符
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Colon,
    Comma,
    Dot,
    Arrow,
    FatArrow,

    // 特殊
    Eof,
    Error(String),
}

/// 词法单元
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// 语法树节点基类
#[derive(Debug, Clone)]
pub enum AstNode {
    // 程序结构
    Program(Vec<AstNode>),
    Function(FunctionDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Impl(ImplBlock),
    Trait(TraitDecl),

    // 语句
    Let(LetStmt),
    ExprStmt(Box<Expr>),
    Return(Option<Box<Expr>>),

    // 表达式
    Expr(Expr),
}

/// 函数声明
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}

/// 参数
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub mutable: bool,
    pub span: Span,
}

/// 结构体声明
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// 字段
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

/// 枚举声明
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// 枚举变体
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

/// 实现块
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub target: String,
    pub trait_name: Option<String>,
    pub methods: Vec<FunctionDecl>,
    pub span: Span,
}

/// Trait声明
#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub methods: Vec<TraitMethod>,
    pub span: Span,
}

/// Trait方法签名
#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

/// Let语句
#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub mutable: bool,
    pub value: Box<Expr>,
    pub span: Span,
}

/// 代码块
#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<AstNode>,
    pub span: Span,
}

/// 表达式
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    FieldAccess(FieldAccessExpr),
    Index(IndexExpr),
    If(IfExpr),
    While(WhileExpr),
    For(ForExpr),
    Block(Block),
    Assignment(AssignmentExpr),
}

/// 字面量
#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

/// 二元表达式
#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
    pub span: Span,
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

/// 一元表达式
#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
    pub span: Span,
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// 调用表达式
#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// 字段访问
#[derive(Debug, Clone)]
pub struct FieldAccessExpr {
    pub object: Box<Expr>,
    pub field: String,
    pub span: Span,
}

/// 索引表达式
#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub span: Span,
}

/// If表达式
#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_branch: Block,
    pub else_branch: Option<Block>,
    pub span: Span,
}

/// While表达式
#[derive(Debug, Clone)]
pub struct WhileExpr {
    pub condition: Box<Expr>,
    pub body: Block,
    pub span: Span,
}

/// For表达式
#[derive(Debug, Clone)]
pub struct ForExpr {
    pub variable: String,
    pub iterable: Box<Expr>,
    pub body: Block,
    pub span: Span,
}

/// 赋值表达式
#[derive(Debug, Clone)]
pub struct AssignmentExpr {
    pub target: Box<Expr>,
    pub value: Box<Expr>,
    pub span: Span,
}

/// 类型表达式
#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String),
    Generic(String, Vec<TypeExpr>),
    Function(Vec<TypeExpr>, Box<Option<TypeExpr>>),
    Reference(Box<TypeExpr>),
    MutableReference(Box<TypeExpr>),
    Array(Box<TypeExpr>, usize),
    Tuple(Vec<TypeExpr>),
    Unit,
}

/// 语法层上下文
pub struct SyntaxContext {
    pub source: String,
    pub tokens: Vec<Token>,
    pub errors: Vec<LayerError>,
}

impl SyntaxContext {
    pub fn new(source: String) -> Self {
        Self {
            source,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// 词法分析器
pub struct Lexer {
    source: String,
    chars: Vec<char>,
    position: usize,
    current_span: Span,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        let chars: Vec<char> = source.chars().collect();
        Self {
            source: source.clone(),
            chars,
            position: 0,
            current_span: Span::new(0, 0),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }

            let start = self.position;

            let token = match self.advance() {
                '(' => Token::new(TokenKind::LParen, Span::new(start, self.position)),
                ')' => Token::new(TokenKind::RParen, Span::new(start, self.position)),
                '{' => Token::new(TokenKind::LBrace, Span::new(start, self.position)),
                '}' => Token::new(TokenKind::RBrace, Span::new(start, self.position)),
                '[' => Token::new(TokenKind::LBracket, Span::new(start, self.position)),
                ']' => Token::new(TokenKind::RBracket, Span::new(start, self.position)),
                ';' => Token::new(TokenKind::Semicolon, Span::new(start, self.position)),
                ':' => {
                    if self.match_char(':') {
                        Token::new(TokenKind::Colon, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Colon, Span::new(start, self.position))
                    }
                }
                ',' => Token::new(TokenKind::Comma, Span::new(start, self.position)),
                '.' => Token::new(TokenKind::Dot, Span::new(start, self.position)),
                '+' => Token::new(TokenKind::Plus, Span::new(start, self.position)),
                '-' => {
                    if self.match_char('>') {
                        Token::new(TokenKind::Arrow, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Minus, Span::new(start, self.position))
                    }
                }
                '*' => Token::new(TokenKind::Star, Span::new(start, self.position)),
                '/' => {
                    if self.match_char('/') {
                        self.skip_comment();
                        continue;
                    } else {
                        Token::new(TokenKind::Slash, Span::new(start, self.position))
                    }
                }
                '%' => Token::new(TokenKind::Percent, Span::new(start, self.position)),
                '=' => {
                    if self.match_char('=') {
                        Token::new(TokenKind::EqualEqual, Span::new(start, self.position))
                    } else if self.match_char('>') {
                        Token::new(TokenKind::FatArrow, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Equal, Span::new(start, self.position))
                    }
                }
                '!' => {
                    if self.match_char('=') {
                        Token::new(TokenKind::NotEqual, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Not, Span::new(start, self.position))
                    }
                }
                '<' => {
                    if self.match_char('=') {
                        Token::new(TokenKind::LessEqual, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Less, Span::new(start, self.position))
                    }
                }
                '>' => {
                    if self.match_char('=') {
                        Token::new(TokenKind::GreaterEqual, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Greater, Span::new(start, self.position))
                    }
                }
                '&' => {
                    if self.match_char('&') {
                        Token::new(TokenKind::And, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Error("Unexpected character".to_string()), Span::new(start, self.position))
                    }
                }
                '|' => {
                    if self.match_char('|') {
                        Token::new(TokenKind::Or, Span::new(start, self.position))
                    } else {
                        Token::new(TokenKind::Error("Unexpected character".to_string()), Span::new(start, self.position))
                    }
                }
                '"' => self.string(start),
                c if c.is_ascii_digit() => self.number(start, c),
                c if c.is_alphabetic() || c == '_' => self.identifier(start, c),
                _ => Token::new(TokenKind::Error("Unexpected character".to_string()), Span::new(start, self.position)),
            };

            tokens.push(token);
        }

        tokens.push(Token::new(TokenKind::Eof, Span::new(self.position, self.position)));
        tokens
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.chars.len()
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.position];
        self.position += 1;
        c
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.chars[self.position]
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.chars[self.position] != expected {
            false
        } else {
            self.position += 1;
            true
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.peek().is_whitespace() {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn string(&mut self, start: usize) -> Token {
        let mut value = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            value.push(self.advance());
        }

        if self.is_at_end() {
            Token::new(TokenKind::Error("Unterminated string".to_string()), Span::new(start, self.position))
        } else {
            self.advance(); // consume closing "
            Token::new(TokenKind::String(value), Span::new(start, self.position))
        }
    }

    fn number(&mut self, start: usize, first: char) -> Token {
        let mut value = first.to_string();
        let mut is_float = false;

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            if self.peek() == '.' {
                is_float = true;
            }
            value.push(self.advance());
        }

        if is_float {
            match value.parse::<f64>() {
                Ok(f) => Token::new(TokenKind::Float(f), Span::new(start, self.position)),
                Err(_) => Token::new(TokenKind::Error("Invalid float".to_string()), Span::new(start, self.position)),
            }
        } else {
            match value.parse::<i64>() {
                Ok(i) => Token::new(TokenKind::Integer(i), Span::new(start, self.position)),
                Err(_) => Token::new(TokenKind::Error("Invalid integer".to_string()), Span::new(start, self.position)),
            }
        }
    }

    fn identifier(&mut self, start: usize, first: char) -> Token {
        let mut value = first.to_string();

        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            value.push(self.advance());
        }

        let kind = match value.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "trait" => TokenKind::Trait,
            "type" => TokenKind::Type,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            _ => TokenKind::Identifier(value),
        };

        Token::new(kind, Span::new(start, self.position))
    }
}

/// 语法分析器
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn parse(&mut self) -> LayerResult<AstNode> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(stmt) = self.declaration()? {
                statements.push(stmt);
            }
        }

        Ok(AstNode::Program(statements))
    }

    fn declaration(&mut self) -> LayerResult<Option<AstNode>> {
        if self.match_token(&[TokenKind::Fn]) {
            Ok(Some(self.function()?))
        } else if self.match_token(&[TokenKind::Struct]) {
            Ok(Some(self.struct_decl()?))
        } else if self.match_token(&[TokenKind::Enum]) {
            Ok(Some(self.enum_decl()?))
        } else if self.match_token(&[TokenKind::Impl]) {
            Ok(Some(self.impl_block()?))
        } else if self.match_token(&[TokenKind::Trait]) {
            Ok(Some(self.trait_decl()?))
        } else {
            Ok(self.statement()?.map(|s| s))
        }
    }

    fn function(&mut self) -> LayerResult<AstNode> {
        let start = self.previous().span;
        let name = self.consume_identifier("Expected function name")?;

        self.consume(TokenKind::LParen, "Expected '(' after function name")?;
        let params = self.parameters()?;
        self.consume(TokenKind::RParen, "Expected ')' after parameters")?;

        let return_type = if self.match_token(&[TokenKind::Arrow]) {
            Some(self.type_expr()?)
        } else {
            None
        };

        let body = self.block()?;
        let span = start.merge(&body.span);

        Ok(AstNode::Function(FunctionDecl {
            name,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parameters(&mut self) -> LayerResult<Vec<Param>> {
        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                let param_start = self.peek().span;
                let mutable = self.match_token(&[TokenKind::Mut]);
                let name = self.consume_identifier("Expected parameter name")?;
                self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                let ty = self.type_expr()?;
                let span = param_start.merge(&self.previous().span);

                params.push(Param {
                    name,
                    ty,
                    mutable,
                    span,
                });

                if !self.match_token(&[TokenKind::Comma]) {
                    break;
                }
            }
        }

        Ok(params)
    }

    fn struct_decl(&mut self) -> LayerResult<AstNode> {
        let start = self.previous().span;
        let name = self.consume_identifier("Expected struct name")?;
        self.consume(TokenKind::LBrace, "Expected '{' after struct name")?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let field_start = self.peek().span;
            let field_name = self.consume_identifier("Expected field name")?;
            self.consume(TokenKind::Colon, "Expected ':' after field name")?;
            let ty = self.type_expr()?;
            let span = field_start.merge(&self.previous().span);

            fields.push(Field {
                name: field_name,
                ty,
                span,
            });

            if !self.match_token(&[TokenKind::Comma]) {
                break;
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' after struct fields")?;
        let span = start.merge(&self.previous().span);

        Ok(AstNode::Struct(StructDecl { name, fields, span }))
    }

    fn enum_decl(&mut self) -> LayerResult<AstNode> {
        let start = self.previous().span;
        let name = self.consume_identifier("Expected enum name")?;
        self.consume(TokenKind::LBrace, "Expected '{' after enum name")?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let variant_start = self.peek().span;
            let variant_name = self.consume_identifier("Expected variant name")?;

            let mut fields = Vec::new();
            if self.match_token(&[TokenKind::LParen]) {
                loop {
                    fields.push(self.type_expr()?);
                    if !self.match_token(&[TokenKind::Comma]) {
                        break;
                    }
                }
                self.consume(TokenKind::RParen, "Expected ')' after variant fields")?;
            }

            let span = variant_start.merge(&self.previous().span);
            variants.push(EnumVariant { name: variant_name, fields, span });

            if !self.match_token(&[TokenKind::Comma]) {
                break;
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' after enum variants")?;
        let span = start.merge(&self.previous().span);

        Ok(AstNode::Enum(EnumDecl { name, variants, span }))
    }

    fn impl_block(&mut self) -> LayerResult<AstNode> {
        let start = self.previous().span;
        let target = self.consume_identifier("Expected impl target")?;

        let trait_name = if self.match_token(&[TokenKind::For]) {
            Some(self.consume_identifier("Expected trait name")?)
        } else {
            None
        };

        self.consume(TokenKind::LBrace, "Expected '{' after impl")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if let AstNode::Function(func) = self.function()? {
                methods.push(func);
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' after impl block")?;
        let span = start.merge(&self.previous().span);

        Ok(AstNode::Impl(ImplBlock { target, trait_name, methods, span }))
    }

    fn trait_decl(&mut self) -> LayerResult<AstNode> {
        let start = self.previous().span;
        let name = self.consume_identifier("Expected trait name")?;
        self.consume(TokenKind::LBrace, "Expected '{' after trait name")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let method_start = self.peek().span;
            self.consume(TokenKind::Fn, "Expected 'fn' in trait method")?;
            let method_name = self.consume_identifier("Expected method name")?;

            self.consume(TokenKind::LParen, "Expected '(' after method name")?;
            let params = self.parameters()?;
            self.consume(TokenKind::RParen, "Expected ')' after parameters")?;

            let return_type = if self.match_token(&[TokenKind::Arrow]) {
                Some(self.type_expr()?)
            } else {
                None
            };

            self.consume(TokenKind::Semicolon, "Expected ';' after trait method")?;
            let span = method_start.merge(&self.previous().span);

            methods.push(TraitMethod {
                name: method_name,
                params,
                return_type,
                span,
            });
        }

        self.consume(TokenKind::RBrace, "Expected '}' after trait body")?;
        let span = start.merge(&self.previous().span);

        Ok(AstNode::Trait(TraitDecl { name, methods, span }))
    }

    fn statement(&mut self) -> LayerResult<Option<AstNode>> {
        if self.match_token(&[TokenKind::Let]) {
            Ok(Some(self.let_statement()?))
        } else if self.match_token(&[TokenKind::Return]) {
            Ok(Some(self.return_statement()?))
        } else {
            let expr = self.expression()?;
            if self.match_token(&[TokenKind::Semicolon]) {
                Ok(Some(AstNode::ExprStmt(Box::new(expr))))
            } else if self.is_at_end() || self.check(&TokenKind::RBrace) {
                Ok(Some(AstNode::ExprStmt(Box::new(expr))))
            } else {
                Err(LayerError::SyntaxError("Expected ';' after expression".to_string()))
            }
        }
    }

    fn let_statement(&mut self) -> LayerResult<AstNode> {
        let start = self.previous().span;
        let mutable = self.match_token(&[TokenKind::Mut]);
        let name = self.consume_identifier("Expected variable name")?;

        let ty = if self.match_token(&[TokenKind::Colon]) {
            Some(self.type_expr()?)
        } else {
            None
        };

        self.consume(TokenKind::Equal, "Expected '=' after variable name")?;
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::Semicolon, "Expected ';' after let statement")?;
        let span = start.merge(&self.previous().span);

        Ok(AstNode::Let(LetStmt { name, ty, mutable, value, span }))
    }

    fn return_statement(&mut self) -> LayerResult<AstNode> {
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.expression()?))
        };
        self.consume(TokenKind::Semicolon, "Expected ';' after return")?;

        Ok(AstNode::Return(value))
    }

    fn block(&mut self) -> LayerResult<Block> {
        let start = self.consume(TokenKind::LBrace, "Expected '{'")?.span;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if let Some(stmt) = self.declaration()? {
                statements.push(stmt);
            }
        }

        self.consume(TokenKind::RBrace, "Expected '}' after block")?;
        let span = start.merge(&self.previous().span);

        Ok(Block { statements, span })
    }

    fn expression(&mut self) -> LayerResult<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> LayerResult<Expr> {
        let expr = self.or()?;

        if self.match_token(&[TokenKind::Equal]) {
            let value = Box::new(self.assignment()?);
            let span = expr.get_span().merge(&value.get_span());
            return Ok(Expr::Assignment(AssignmentExpr {
                target: Box::new(expr),
                value,
                span,
            }));
        }

        Ok(expr)
    }

    fn or(&mut self) -> LayerResult<Expr> {
        let mut left = self.and()?;

        while self.match_token(&[TokenKind::Or]) {
            let right = Box::new(self.and()?);
            let span = left.get_span().merge(&right.get_span());
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::Or,
                right,
                span,
            });
        }

        Ok(left)
    }

    fn and(&mut self) -> LayerResult<Expr> {
        let mut left = self.equality()?;

        while self.match_token(&[TokenKind::And]) {
            let right = Box::new(self.equality()?);
            let span = left.get_span().merge(&right.get_span());
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op: BinaryOp::And,
                right,
                span,
            });
        }

        Ok(left)
    }

    fn equality(&mut self) -> LayerResult<Expr> {
        let mut left = self.comparison()?;

        while let Some(op) = self.match_equality_op() {
            let right = Box::new(self.comparison()?);
            let span = left.get_span().merge(&right.get_span());
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right,
                span,
            });
        }

        Ok(left)
    }

    fn match_equality_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&[TokenKind::EqualEqual]) {
            Some(BinaryOp::Eq)
        } else if self.match_token(&[TokenKind::NotEqual]) {
            Some(BinaryOp::Ne)
        } else {
            None
        }
    }

    fn comparison(&mut self) -> LayerResult<Expr> {
        let mut left = self.term()?;

        while let Some(op) = self.match_comparison_op() {
            let right = Box::new(self.term()?);
            let span = left.get_span().merge(&right.get_span());
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right,
                span,
            });
        }

        Ok(left)
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&[TokenKind::Less]) {
            Some(BinaryOp::Lt)
        } else if self.match_token(&[TokenKind::Greater]) {
            Some(BinaryOp::Gt)
        } else if self.match_token(&[TokenKind::LessEqual]) {
            Some(BinaryOp::Le)
        } else if self.match_token(&[TokenKind::GreaterEqual]) {
            Some(BinaryOp::Ge)
        } else {
            None
        }
    }

    fn term(&mut self) -> LayerResult<Expr> {
        let mut left = self.factor()?;

        while let Some(op) = self.match_term_op() {
            let right = Box::new(self.factor()?);
            let span = left.get_span().merge(&right.get_span());
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right,
                span,
            });
        }

        Ok(left)
    }

    fn match_term_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&[TokenKind::Plus]) {
            Some(BinaryOp::Add)
        } else if self.match_token(&[TokenKind::Minus]) {
            Some(BinaryOp::Sub)
        } else {
            None
        }
    }

    fn factor(&mut self) -> LayerResult<Expr> {
        let mut left = self.unary()?;

        while let Some(op) = self.match_factor_op() {
            let right = Box::new(self.unary()?);
            let span = left.get_span().merge(&right.get_span());
            left = Expr::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right,
                span,
            });
        }

        Ok(left)
    }

    fn match_factor_op(&mut self) -> Option<BinaryOp> {
        if self.match_token(&[TokenKind::Star]) {
            Some(BinaryOp::Mul)
        } else if self.match_token(&[TokenKind::Slash]) {
            Some(BinaryOp::Div)
        } else if self.match_token(&[TokenKind::Percent]) {
            Some(BinaryOp::Mod)
        } else {
            None
        }
    }

    fn unary(&mut self) -> LayerResult<Expr> {
        if self.match_token(&[TokenKind::Minus]) {
            let expr = Box::new(self.unary()?);
            let span = self.previous().span.merge(&expr.get_span());
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr,
                span,
            }));
        }

        if self.match_token(&[TokenKind::Not]) {
            let expr = Box::new(self.unary()?);
            let span = self.previous().span.merge(&expr.get_span());
            return Ok(Expr::Unary(UnaryExpr {
                op: UnaryOp::Not,
                expr,
                span,
            }));
        }

        self.call()
    }

    fn call(&mut self) -> LayerResult<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&[TokenKind::LParen]) {
                let args = self.arguments()?;
                self.consume(TokenKind::RParen, "Expected ')' after arguments")?;
                let span = expr.get_span().merge(&self.previous().span);
                expr = Expr::Call(CallExpr {
                    callee: Box::new(expr),
                    args,
                    span,
                });
            } else if self.match_token(&[TokenKind::Dot]) {
                let field = self.consume_identifier("Expected field name")?;
                let span = expr.get_span().merge(&self.previous().span);
                expr = Expr::FieldAccess(FieldAccessExpr {
                    object: Box::new(expr),
                    field,
                    span,
                });
            } else if self.match_token(&[TokenKind::LBracket]) {
                let index = Box::new(self.expression()?);
                self.consume(TokenKind::RBracket, "Expected ']' after index")?;
                let span = expr.get_span().merge(&self.previous().span);
                expr = Expr::Index(IndexExpr {
                    object: Box::new(expr),
                    index,
                    span,
                });
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> LayerResult<Expr> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::Integer(i) => {
                self.advance();
                Ok(Expr::Literal(Literal::Integer(*i)))
            }
            TokenKind::Float(f) => {
                self.advance();
                Ok(Expr::Literal(Literal::Float(*f)))
            }
            TokenKind::String(s) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s.clone())))
            }
            TokenKind::Bool(b) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(*b)))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Expr::Identifier(name.clone()))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(TokenKind::RParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::If => self.if_expression(),
            TokenKind::While => self.while_expression(),
            TokenKind::For => self.for_expression(),
            TokenKind::LBrace => {
                let block = self.block()?;
                Ok(Expr::Block(block))
            }
            _ => Err(LayerError::SyntaxError(format!("Unexpected token: {:?}", token.kind))),
        }
    }

    fn if_expression(&mut self) -> LayerResult<Expr> {
        let start = self.previous().span;
        let condition = Box::new(self.expression()?);
        let then_branch = self.block()?;

        let else_branch = if self.match_token(&[TokenKind::Else]) {
            if self.match_token(&[TokenKind::If]) {
                let else_if = self.if_expression()?;
                Some(Block {
                    statements: vec![AstNode::ExprStmt(Box::new(else_if))],
                    span: self.previous().span,
                })
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };

        let span = start.merge(&self.previous().span);
        Ok(Expr::If(IfExpr { condition, then_branch, else_branch, span }))
    }

    fn while_expression(&mut self) -> LayerResult<Expr> {
        let start = self.previous().span;
        let condition = Box::new(self.expression()?);
        let body = self.block()?;
        let span = start.merge(&body.span);

        Ok(Expr::While(WhileExpr { condition, body, span }))
    }

    fn for_expression(&mut self) -> LayerResult<Expr> {
        let start = self.previous().span;
        let variable = self.consume_identifier("Expected loop variable")?;
        self.consume(TokenKind::In, "Expected 'in' after loop variable")?;
        let iterable = Box::new(self.expression()?);
        let body = self.block()?;
        let span = start.merge(&body.span);

        Ok(Expr::For(ForExpr { variable, iterable, body, span }))
    }

    fn arguments(&mut self) -> LayerResult<Vec<Expr>> {
        let mut args = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.expression()?);
                if !self.match_token(&[TokenKind::Comma]) {
                    break;
                }
            }
        }

        Ok(args)
    }

    fn type_expr(&mut self) -> LayerResult<TypeExpr> {
        let name = self.consume_identifier("Expected type name")?;

        if self.match_token(&[TokenKind::Less]) {
            // Generic type
            let mut params = Vec::new();
            loop {
                params.push(self.type_expr()?);
                if !self.match_token(&[TokenKind::Comma]) {
                    break;
                }
            }
            self.consume(TokenKind::Greater, "Expected '>' after generic parameters")?;
            Ok(TypeExpr::Generic(name, params))
        } else {
            Ok(TypeExpr::Named(name))
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.position - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.previous()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
        }
    }

    fn match_token(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.check(kind) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> LayerResult<&Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(LayerError::SyntaxError(message.to_string()))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> LayerResult<String> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(LayerError::SyntaxError(message.to_string()))
        }
    }
}

impl Expr {
    fn get_span(&self) -> Span {
        match self {
            Expr::Literal(_) => Span::default(),
            Expr::Identifier(_) => Span::default(),
            Expr::Binary(b) => b.span,
            Expr::Unary(u) => u.span,
            Expr::Call(c) => c.span,
            Expr::FieldAccess(f) => f.span,
            Expr::Index(i) => i.span,
            Expr::If(i) => i.span,
            Expr::While(w) => w.span,
            Expr::For(f) => f.span,
            Expr::Block(b) => b.span,
            Expr::Assignment(a) => a.span,
        }
    }
}

/// Syntax Layer 实现
pub struct SyntaxLayer;

impl Layer for SyntaxLayer {
    const ID: LayerId = LayerId::Syntax;
    type Input = String;  // 源代码
    type Output = AstNode; // AST
    type Context = SyntaxContext;

    fn transform(input: Self::Input, ctx: &mut Self::Context) -> LayerResult<Self::Output> {
        ctx.source = input;

        // 词法分析
        let mut lexer = Lexer::new(ctx.source.clone());
        ctx.tokens = lexer.tokenize();

        // 语法分析
        let mut parser = Parser::new(ctx.tokens.clone());
        parser.parse()
    }
}

// ============================================================================
// Layer 2: Semantic Layer - 语义分析层
// ============================================================================

/// 类型定义
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,
    Bool,
    Int,
    Float,
    String,
    Function(Vec<Type>, Box<Type>),
    Struct(String, Vec<(String, Type)>),
    Enum(String, Vec<String>),
    Generic(String, Vec<Type>),
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    Array(Box<Type>, usize),
    Tuple(Vec<Type>),
    Unknown,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Unit => write!(f, "()"),
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "string"),
            Type::Function(params, ret) => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Struct(name, _) => write!(f, "{}", name),
            Type::Enum(name, _) => write!(f, "{}", name),
            Type::Generic(name, params) => {
                write!(f, "{}<", name)?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ">")
            }
            Type::Reference(t) => write!(f, "&{}", t),
            Type::MutableReference(t) => write!(f, "&mut {}", t),
            Type::Array(t, n) => write!(f, "[{}; {}]", t, n),
            Type::Tuple(ts) => {
                write!(f, "(")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

/// 作用域中的符号
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub kind: SymbolKind,
}

/// 符号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Parameter,
}

/// 作用域
#[derive(Debug, Default)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: Option<usize>,
}

/// 符号表
#[derive(Debug, Default)]
pub struct SymbolTable {
    pub scopes: Vec<Scope>,
    pub current_scope: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        let global_scope = Scope {
            symbols: HashMap::new(),
            parent: None,
        };
        Self {
            scopes: vec![global_scope],
            current_scope: 0,
        }
    }

    pub fn enter_scope(&mut self) {
        let new_scope = Scope {
            symbols: HashMap::new(),
            parent: Some(self.current_scope),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }

    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    pub fn define(&mut self, symbol: Symbol) {
        self.scopes[self.current_scope].symbols.insert(symbol.name.clone(), symbol);
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut scope_idx = self.current_scope;
        loop {
            if let Some(symbol) = self.scopes[scope_idx].symbols.get(name) {
                return Some(symbol);
            }
            if let Some(parent) = self.scopes[scope_idx].parent {
                scope_idx = parent;
            } else {
                return None;
            }
        }
    }

    pub fn lookup_current(&self, name: &str) -> Option<&Symbol> {
        self.scopes[self.current_scope].symbols.get(name)
    }
}

/// 语义分析上下文
pub struct SemanticContext {
    pub symbol_table: SymbolTable,
    pub errors: Vec<LayerError>,
    pub current_function_return_type: Option<Type>,
}

impl SemanticContext {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            errors: Vec::new(),
            current_function_return_type: None,
        }
    }
}

/// 类型检查器
pub struct TypeChecker;

impl TypeChecker {
    pub fn check(ast: &AstNode, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        let mut checker = SemanticAnalyzer;
        checker.analyze(ast, ctx)
    }
}

/// 类型化AST节点
#[derive(Debug, Clone)]
pub enum TypedAst {
    Program(Vec<TypedAst>),
    Function(TypedFunctionDecl),
    Struct(TypedStructDecl),
    Enum(TypedEnumDecl),
    Let(TypedLetStmt),
    Return(Option<Box<TypedExpr>>),
    Expr(TypedExpr),
}

/// 类型化函数声明
#[derive(Debug, Clone)]
pub struct TypedFunctionDecl {
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_type: Type,
    pub body: TypedBlock,
    pub span: Span,
}

/// 类型化参数
#[derive(Debug, Clone)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub span: Span,
}

/// 类型化结构体
#[derive(Debug, Clone)]
pub struct TypedStructDecl {
    pub name: String,
    pub fields: Vec<TypedField>,
    pub span: Span,
}

/// 类型化字段
#[derive(Debug, Clone)]
pub struct TypedField {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// 类型化枚举
#[derive(Debug, Clone)]
pub struct TypedEnumDecl {
    pub name: String,
    pub variants: Vec<TypedEnumVariant>,
    pub span: Span,
}

/// 类型化枚举变体
#[derive(Debug, Clone)]
pub struct TypedEnumVariant {
    pub name: String,
    pub field_types: Vec<Type>,
    pub span: Span,
}

/// 类型化Let语句
#[derive(Debug, Clone)]
pub struct TypedLetStmt {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub value: Box<TypedExpr>,
    pub span: Span,
}

/// 类型化代码块
#[derive(Debug, Clone)]
pub struct TypedBlock {
    pub statements: Vec<TypedAst>,
    pub return_type: Type,
    pub span: Span,
}

/// 类型化表达式
#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
    pub span: Span,
}

/// 类型化表达式种类
#[derive(Debug, Clone)]
pub enum TypedExprKind {
    Literal(Literal),
    Identifier(String),
    Binary(BinaryOp, Box<TypedExpr>, Box<TypedExpr>),
    Unary(UnaryOp, Box<TypedExpr>),
    Call(Box<TypedExpr>, Vec<TypedExpr>),
    FieldAccess(Box<TypedExpr>, String),
    Index(Box<TypedExpr>, Box<TypedExpr>),
    If(Box<TypedExpr>, TypedBlock, Option<TypedBlock>),
    While(Box<TypedExpr>, TypedBlock),
    For(String, Box<TypedExpr>, TypedBlock),
    Block(TypedBlock),
    Assignment(Box<TypedExpr>, Box<TypedExpr>),
}

/// 语义分析器
pub struct SemanticAnalyzer;

impl SemanticAnalyzer {
    fn analyze(&mut self, ast: &AstNode, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        match ast {
            AstNode::Program(stmts) => {
                let mut typed_stmts = Vec::new();
                for stmt in stmts {
                    typed_stmts.push(self.analyze(stmt, ctx)?);
                }
                Ok(TypedAst::Program(typed_stmts))
            }
            AstNode::Function(func) => self.analyze_function(func, ctx),
            AstNode::Struct(s) => self.analyze_struct(s, ctx),
            AstNode::Enum(e) => self.analyze_enum(e, ctx),
            AstNode::Impl(_) => Err(LayerError::TypeError("Impl blocks not yet supported".to_string())),
            AstNode::Trait(_) => Err(LayerError::TypeError("Traits not yet supported".to_string())),
            AstNode::Let(let_stmt) => self.analyze_let(let_stmt, ctx),
            AstNode::Return(expr) => self.analyze_return(expr.as_ref(), ctx),
            AstNode::ExprStmt(expr) => {
                let typed_expr = self.analyze_expr(expr, ctx)?;
                Ok(TypedAst::Expr(typed_expr))
            }
            AstNode::Expr(expr) => {
                let typed_expr = self.analyze_expr(expr, ctx)?;
                Ok(TypedAst::Expr(typed_expr))
            }
        }
    }

    fn analyze_function(&mut self, func: &FunctionDecl, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        // 收集参数类型
        let mut param_types = Vec::new();
        let mut typed_params = Vec::new();

        for param in &func.params {
            let ty = self.resolve_type(&param.ty)?;
            param_types.push(ty.clone());
            typed_params.push(TypedParam {
                name: param.name.clone(),
                ty: ty.clone(),
                mutable: param.mutable,
                span: param.span,
            });
        }

        // 解析返回类型
        let return_type = match &func.return_type {
            Some(t) => self.resolve_type(t)?,
            None => Type::Unit,
        };

        // 注册函数到符号表
        let func_type = Type::Function(param_types.clone(), Box::new(return_type.clone()));
        ctx.symbol_table.define(Symbol {
            name: func.name.clone(),
            ty: func_type,
            mutable: false,
            kind: SymbolKind::Function,
        });

        // 分析函数体
        ctx.symbol_table.enter_scope();
        ctx.current_function_return_type = Some(return_type.clone());

        // 注册参数到作用域
        for param in &typed_params {
            ctx.symbol_table.define(Symbol {
                name: param.name.clone(),
                ty: param.ty.clone(),
                mutable: param.mutable,
                kind: SymbolKind::Parameter,
            });
        }

        let body = self.analyze_block(&func.body, ctx)?;
        ctx.symbol_table.exit_scope();
        ctx.current_function_return_type = None;

        // 检查返回类型
        if body.return_type != return_type {
            return Err(LayerError::TypeError(format!(
                "Function '{}' return type mismatch: expected {}, got {}",
                func.name, return_type, body.return_type
            )));
        }

        Ok(TypedAst::Function(TypedFunctionDecl {
            name: func.name.clone(),
            params: typed_params,
            return_type,
            body,
            span: func.span,
        }))
    }

    fn analyze_struct(&mut self, s: &StructDecl, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        let mut typed_fields = Vec::new();
        let mut field_types = Vec::new();

        for field in &s.fields {
            let ty = self.resolve_type(&field.ty)?;
            field_types.push((field.name.clone(), ty.clone()));
            typed_fields.push(TypedField {
                name: field.name.clone(),
                ty,
                span: field.span,
            });
        }

        // 注册结构体类型
        let struct_type = Type::Struct(s.name.clone(), field_types);
        ctx.symbol_table.define(Symbol {
            name: s.name.clone(),
            ty: struct_type,
            mutable: false,
            kind: SymbolKind::Struct,
        });

        Ok(TypedAst::Struct(TypedStructDecl {
            name: s.name.clone(),
            fields: typed_fields,
            span: s.span,
        }))
    }

    fn analyze_enum(&mut self, e: &EnumDecl, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        let mut typed_variants = Vec::new();
        let mut variant_names = Vec::new();

        for variant in &e.variants {
            let mut field_types = Vec::new();
            for field in &variant.fields {
                field_types.push(self.resolve_type(field)?);
            }
            variant_names.push(variant.name.clone());
            typed_variants.push(TypedEnumVariant {
                name: variant.name.clone(),
                field_types,
                span: variant.span,
            });
        }

        // 注册枚举类型
        let enum_type = Type::Enum(e.name.clone(), variant_names);
        ctx.symbol_table.define(Symbol {
            name: e.name.clone(),
            ty: enum_type,
            mutable: false,
            kind: SymbolKind::Enum,
        });

        Ok(TypedAst::Enum(TypedEnumDecl {
            name: e.name.clone(),
            variants: typed_variants,
            span: e.span,
        }))
    }

    fn analyze_let(&mut self, let_stmt: &LetStmt, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        let value = self.analyze_expr(&let_stmt.value, ctx)?;

        let ty = match &let_stmt.ty {
            Some(type_expr) => {
                let declared_ty = self.resolve_type(type_expr)?;
                if declared_ty != value.ty {
                    return Err(LayerError::TypeError(format!(
                        "Type mismatch in let binding: expected {}, got {}",
                        declared_ty, value.ty
                    )));
                }
                declared_ty
            }
            None => value.ty.clone(),
        };

        // 注册变量
        ctx.symbol_table.define(Symbol {
            name: let_stmt.name.clone(),
            ty: ty.clone(),
            mutable: let_stmt.mutable,
            kind: SymbolKind::Variable,
        });

        Ok(TypedAst::Let(TypedLetStmt {
            name: let_stmt.name.clone(),
            ty,
            mutable: let_stmt.mutable,
            value: Box::new(value),
            span: let_stmt.span,
        }))
    }

    fn analyze_return(&mut self, expr: Option<&Box<Expr>>, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        let expected_type = ctx.current_function_return_type.clone()
            .ok_or_else(|| LayerError::TypeError("Return outside of function".to_string()))?;

        let typed_expr = match expr {
            Some(e) => {
                let typed = self.analyze_expr(e, ctx)?;
                if typed.ty != expected_type {
                    return Err(LayerError::TypeError(format!(
                        "Return type mismatch: expected {}, got {}",
                        expected_type, typed.ty
                    )));
                }
                Some(Box::new(typed))
            }
            None => {
                if expected_type != Type::Unit {
                    return Err(LayerError::TypeError(format!(
                        "Return type mismatch: expected {}, got ()",
                        expected_type
                    )));
                }
                None
            }
        };

        Ok(TypedAst::Return(typed_expr))
    }

    fn analyze_block(&mut self, block: &Block, ctx: &mut SemanticContext) -> LayerResult<TypedBlock> {
        ctx.symbol_table.enter_scope();

        let mut typed_stmts = Vec::new();
        let mut return_type = Type::Unit;

        for (i, stmt) in block.statements.iter().enumerate() {
            let typed = self.analyze(stmt, ctx)?;

            // 最后一个表达式语句决定块的返回类型
            if i == block.statements.len() - 1 {
                if let TypedAst::Expr(expr) = &typed {
                    return_type = expr.ty.clone();
                }
            }

            typed_stmts.push(typed);
        }

        ctx.symbol_table.exit_scope();

        Ok(TypedBlock {
            statements: typed_stmts,
            return_type,
            span: block.span,
        })
    }

    fn analyze_expr(&mut self, expr: &Expr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        match expr {
            Expr::Literal(lit) => self.analyze_literal(lit),
            Expr::Identifier(name) => self.analyze_identifier(name, ctx),
            Expr::Binary(bin) => self.analyze_binary(bin, ctx),
            Expr::Unary(unary) => self.analyze_unary(unary, ctx),
            Expr::Call(call) => self.analyze_call(call, ctx),
            Expr::FieldAccess(access) => self.analyze_field_access(access, ctx),
            Expr::Index(index) => self.analyze_index(index, ctx),
            Expr::If(if_expr) => self.analyze_if(if_expr, ctx),
            Expr::While(while_expr) => self.analyze_while(while_expr, ctx),
            Expr::For(for_expr) => self.analyze_for(for_expr, ctx),
            Expr::Block(block) => {
                let typed_block = self.analyze_block(block, ctx)?;
                Ok(TypedExpr {
                    kind: TypedExprKind::Block(typed_block.clone()),
                    ty: typed_block.return_type.clone(),
                    span: block.span,
                })
            }
            Expr::Assignment(assign) => self.analyze_assignment(assign, ctx),
        }
    }

    fn analyze_literal(&mut self, lit: &Literal) -> LayerResult<TypedExpr> {
        let (ty, span) = match lit {
            Literal::Integer(_) => (Type::Int, Span::default()),
            Literal::Float(_) => (Type::Float, Span::default()),
            Literal::String(_) => (Type::String, Span::default()),
            Literal::Bool(_) => (Type::Bool, Span::default()),
        };

        Ok(TypedExpr {
            kind: TypedExprKind::Literal(lit.clone()),
            ty,
            span,
        })
    }

    fn analyze_identifier(&mut self, name: &str, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        match ctx.symbol_table.lookup(name) {
            Some(symbol) => Ok(TypedExpr {
                kind: TypedExprKind::Identifier(name.to_string()),
                ty: symbol.ty.clone(),
                span: Span::default(),
            }),
            None => Err(LayerError::TypeError(format!("Undefined identifier: {}", name))),
        }
    }

    fn analyze_binary(&mut self, bin: &BinaryExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let left = self.analyze_expr(&bin.left, ctx)?;
        let right = self.analyze_expr(&bin.right, ctx)?;

        let ty = match bin.op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                // 算术运算
                if left.ty != right.ty {
                    return Err(LayerError::TypeError(format!(
                        "Type mismatch in binary expression: {} vs {}",
                        left.ty, right.ty
                    )));
                }
                match (&left.ty, &right.ty) {
                    (Type::Int, Type::Int) => Type::Int,
                    (Type::Float, Type::Float) => Type::Float,
                    _ => return Err(LayerError::TypeError(format!(
                        "Invalid types for arithmetic: {} and {}",
                        left.ty, right.ty
                    ))),
                }
            }
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
                // 比较运算
                if left.ty != right.ty {
                    return Err(LayerError::TypeError(format!(
                        "Type mismatch in comparison: {} vs {}",
                        left.ty, right.ty
                    )));
                }
                Type::Bool
            }
            BinaryOp::And | BinaryOp::Or => {
                // 逻辑运算
                if left.ty != Type::Bool || right.ty != Type::Bool {
                    return Err(LayerError::TypeError(
                        "Logical operators require boolean operands".to_string()
                    ));
                }
                Type::Bool
            }
        };

        Ok(TypedExpr {
            kind: TypedExprKind::Binary(bin.op, Box::new(left), Box::new(right)),
            ty,
            span: bin.span,
        })
    }

    fn analyze_unary(&mut self, unary: &UnaryExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let expr = self.analyze_expr(&unary.expr, ctx)?;

        let ty = match unary.op {
            UnaryOp::Neg => {
                if expr.ty != Type::Int && expr.ty != Type::Float {
                    return Err(LayerError::TypeError(
                        "Negation requires numeric operand".to_string()
                    ));
                }
                expr.ty.clone()
            }
            UnaryOp::Not => {
                if expr.ty != Type::Bool {
                    return Err(LayerError::TypeError(
                        "Not operator requires boolean operand".to_string()
                    ));
                }
                Type::Bool
            }
        };

        Ok(TypedExpr {
            kind: TypedExprKind::Unary(unary.op, Box::new(expr)),
            ty,
            span: unary.span,
        })
    }

    fn analyze_call(&mut self, call: &CallExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let callee = self.analyze_expr(&call.callee, ctx)?;

        let (param_types, return_type) = match &callee.ty {
            Type::Function(params, ret) => (params.clone(), (**ret).clone()),
            _ => return Err(LayerError::TypeError(format!(
                "Cannot call non-function type: {}", callee.ty
            ))),
        };

        // 检查参数数量
        if call.args.len() != param_types.len() {
            return Err(LayerError::TypeError(format!(
                "Argument count mismatch: expected {}, got {}",
                param_types.len(), call.args.len()
            )));
        }

        // 检查参数类型
        let mut typed_args = Vec::new();
        for (i, arg) in call.args.iter().enumerate() {
            let typed_arg = self.analyze_expr(arg, ctx)?;
            if typed_arg.ty != param_types[i] {
                return Err(LayerError::TypeError(format!(
                    "Argument {} type mismatch: expected {}, got {}",
                    i, param_types[i], typed_arg.ty
                )));
            }
            typed_args.push(typed_arg);
        }

        Ok(TypedExpr {
            kind: TypedExprKind::Call(Box::new(callee), typed_args),
            ty: return_type,
            span: call.span,
        })
    }

    fn analyze_field_access(&mut self, access: &FieldAccessExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let object = self.analyze_expr(&access.object, ctx)?;

        let field_ty = match &object.ty {
            Type::Struct(name, fields) => {
                fields.iter()
                    .find(|(f, _)| f == &access.field)
                    .map(|(_, t)| t.clone())
                    .ok_or_else(|| LayerError::TypeError(format!(
                        "Field '{}' not found in struct '{}'", access.field, name
                    )))?
            }
            _ => return Err(LayerError::TypeError(format!(
                "Cannot access field on type: {}", object.ty
            ))),
        };

        Ok(TypedExpr {
            kind: TypedExprKind::FieldAccess(Box::new(object), access.field.clone()),
            ty: field_ty,
            span: access.span,
        })
    }

    fn analyze_index(&mut self, index: &IndexExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let object = self.analyze_expr(&index.object, ctx)?;
        let index_expr = self.analyze_expr(&index.index, ctx)?;

        // 检查索引类型
        if index_expr.ty != Type::Int {
            return Err(LayerError::TypeError(
                "Array index must be integer".to_string()
            ));
        }

        let element_ty = match &object.ty {
            Type::Array(elem, _) => (**elem).clone(),
            _ => return Err(LayerError::TypeError(format!(
                "Cannot index type: {}", object.ty
            ))),
        };

        Ok(TypedExpr {
            kind: TypedExprKind::Index(Box::new(object), Box::new(index_expr)),
            ty: element_ty,
            span: index.span,
        })
    }

    fn analyze_if(&mut self, if_expr: &IfExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let condition = self.analyze_expr(&if_expr.condition, ctx)?;

        if condition.ty != Type::Bool {
            return Err(LayerError::TypeError(
                "If condition must be boolean".to_string()
            ));
        }

        let then_branch = self.analyze_block(&if_expr.then_branch, ctx)?;

        let (else_branch, result_ty) = match &if_expr.else_branch {
            Some(else_block) => {
                let else_branch = self.analyze_block(else_block, ctx)?;
                if then_branch.return_type != else_branch.return_type {
                    return Err(LayerError::TypeError(format!(
                        "If branches have different types: {} vs {}",
                        then_branch.return_type, else_branch.return_type
                    )));
                }
                (Some(else_branch), then_branch.return_type.clone())
            }
            None => {
                if then_branch.return_type != Type::Unit {
                    return Err(LayerError::TypeError(
                        "If without else must return unit".to_string()
                    ));
                }
                (None, Type::Unit)
            }
        };

        Ok(TypedExpr {
            kind: TypedExprKind::If(Box::new(condition), then_branch, else_branch),
            ty: result_ty,
            span: if_expr.span,
        })
    }

    fn analyze_while(&mut self, while_expr: &WhileExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let condition = self.analyze_expr(&while_expr.condition, ctx)?;

        if condition.ty != Type::Bool {
            return Err(LayerError::TypeError(
                "While condition must be boolean".to_string()
            ));
        }

        let body = self.analyze_block(&while_expr.body, ctx)?;

        Ok(TypedExpr {
            kind: TypedExprKind::While(Box::new(condition), body),
            ty: Type::Unit,
            span: while_expr.span,
        })
    }

    fn analyze_for(&mut self, for_expr: &ForExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let iterable = self.analyze_expr(&for_expr.iterable, ctx)?;

        // 简化处理：假设iterable是数组
        let element_ty = match &iterable.ty {
            Type::Array(elem, _) => (**elem).clone(),
            _ => return Err(LayerError::TypeError(format!(
                "Cannot iterate over type: {}", iterable.ty
            ))),
        };

        ctx.symbol_table.enter_scope();
        ctx.symbol_table.define(Symbol {
            name: for_expr.variable.clone(),
            ty: element_ty,
            mutable: false,
            kind: SymbolKind::Variable,
        });

        let body = self.analyze_block(&for_expr.body, ctx)?;
        ctx.symbol_table.exit_scope();

        Ok(TypedExpr {
            kind: TypedExprKind::For(for_expr.variable.clone(), Box::new(iterable), body),
            ty: Type::Unit,
            span: for_expr.span,
        })
    }

    fn analyze_assignment(&mut self, assign: &AssignmentExpr, ctx: &mut SemanticContext) -> LayerResult<TypedExpr> {
        let target = self.analyze_expr(&assign.target, ctx)?;
        let value = self.analyze_expr(&assign.value, ctx)?;

        // 检查目标是否可变
        match &assign.target {
            Expr::Identifier(name) => {
                if let Some(symbol) = ctx.symbol_table.lookup(name) {
                    if !symbol.mutable {
                        return Err(LayerError::TypeError(format!(
                            "Cannot assign to immutable variable '{}'", name
                        )));
                    }
                }
            }
            _ => {}
        }

        if target.ty != value.ty {
            return Err(LayerError::TypeError(format!(
                "Assignment type mismatch: {} vs {}",
                target.ty, value.ty
            )));
        }

        Ok(TypedExpr {
            kind: TypedExprKind::Assignment(Box::new(target), Box::new(value)),
            ty: Type::Unit,
            span: assign.span,
        })
    }

    fn resolve_type(&self, type_expr: &TypeExpr) -> LayerResult<Type> {
        match type_expr {
            TypeExpr::Named(name) => match name.as_str() {
                "unit" | "()" => Ok(Type::Unit),
                "bool" => Ok(Type::Bool),
                "int" | "i32" | "i64" => Ok(Type::Int),
                "float" | "f32" | "f64" => Ok(Type::Float),
                "string" | "str" => Ok(Type::String),
                _ => Ok(Type::Named(name.clone())),
            }
            TypeExpr::Generic(name, params) => {
                let resolved_params: LayerResult<Vec<_>> = params.iter()
                    .map(|p| self.resolve_type(p))
                    .collect();
                Ok(Type::Generic(name.clone(), resolved_params?))
            }
            TypeExpr::Function(params, ret) => {
                let param_types: LayerResult<Vec<_>> = params.iter()
                    .map(|p| self.resolve_type(p))
                    .collect();
                let ret_type = match ret.as_ref() {
                    Some(t) => self.resolve_type(t)?,
                    None => Type::Unit,
                };
                Ok(Type::Function(param_types?, Box::new(ret_type)))
            }
            TypeExpr::Reference(t) => Ok(Type::Reference(Box::new(self.resolve_type(t)?))),
            TypeExpr::MutableReference(t) => Ok(Type::MutableReference(Box::new(self.resolve_type(t)?))),
            TypeExpr::Array(t, n) => Ok(Type::Array(Box::new(self.resolve_type(t)?), *n)),
            TypeExpr::Tuple(ts) => {
                let types: LayerResult<Vec<_>> = ts.iter()
                    .map(|t| self.resolve_type(t))
                    .collect();
                Ok(Type::Tuple(types?))
            }
            TypeExpr::Unit => Ok(Type::Unit),
        }
    }
}

/// Semantic Layer 实现
pub struct SemanticLayer;

impl Layer for SemanticLayer {
    const ID: LayerId = LayerId::Semantic;
    type Input = AstNode;  // 来自Syntax层的AST
    type Output = TypedAst; // 类型化AST
    type Context = SemanticContext;

    fn transform(input: Self::Input, ctx: &mut Self::Context) -> LayerResult<Self::Output> {
        TypeChecker::check(&input, ctx)
    }
}

// ============================================================================
// Layer 3: Pattern Layer - 优化和转换层
// ============================================================================

/// 优化上下文
pub struct PatternContext {
    pub optimizations_applied: Vec<String>,
    pub enable_constant_folding: bool,
    pub enable_dead_code_elimination: bool,
    pub enable_inline_expansion: bool,
}

impl PatternContext {
    pub fn new() -> Self {
        Self {
            optimizations_applied: Vec::new(),
            enable_constant_folding: true,
            enable_dead_code_elimination: true,
            enable_inline_expansion: true,
        }
    }
}

/// 优化后的AST
#[derive(Debug, Clone)]
pub enum OptimizedAst {
    Program(Vec<OptimizedAst>),
    Function(OptimizedFunctionDecl),
    Let(OptimizedLetStmt),
    Return(Option<Box<OptimizedExpr>>),
    Expr(OptimizedExpr),
}

/// 优化后的函数
#[derive(Debug, Clone)]
pub struct OptimizedFunctionDecl {
    pub name: String,
    pub params: Vec<TypedParam>,
    pub return_type: Type,
    pub body: OptimizedBlock,
    pub is_inlined: bool,
    pub span: Span,
}

/// 优化后的Let语句
#[derive(Debug, Clone)]
pub struct OptimizedLetStmt {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub value: Box<OptimizedExpr>,
    pub is_folded: bool,
    pub span: Span,
}

/// 优化后的代码块
#[derive(Debug, Clone)]
pub struct OptimizedBlock {
    pub statements: Vec<OptimizedAst>,
    pub return_type: Type,
    pub has_dead_code: bool,
    pub span: Span,
}

/// 优化后的表达式
#[derive(Debug, Clone)]
pub struct OptimizedExpr {
    pub kind: OptimizedExprKind,
    pub ty: Type,
    pub is_constant: bool,
    pub constant_value: Option<Literal>,
    pub span: Span,
}

/// 优化后的表达式种类
#[derive(Debug, Clone)]
pub enum OptimizedExprKind {
    Literal(Literal),
    Identifier(String),
    Binary(BinaryOp, Box<OptimizedExpr>, Box<OptimizedExpr>),
    Unary(UnaryOp, Box<OptimizedExpr>),
    Call(Box<OptimizedExpr>, Vec<OptimizedExpr>),
    FieldAccess(Box<OptimizedExpr>, String),
    Index(Box<OptimizedExpr>, Box<OptimizedExpr>),
    If(Box<OptimizedExpr>, OptimizedBlock, Option<OptimizedBlock>),
    While(Box<OptimizedExpr>, OptimizedBlock),
    For(String, Box<OptimizedExpr>, OptimizedBlock),
    Block(OptimizedBlock),
    Assignment(Box<OptimizedExpr>, Box<OptimizedExpr>),
    // 新增：优化后的模式
    ConstantFolded(Literal),  // 常量折叠结果
    InlinedCall(String, Vec<OptimizedExpr>),  // 内联展开
}

/// 优化器
pub struct Optimizer;

impl Optimizer {
    pub fn optimize(ast: &TypedAst, ctx: &mut PatternContext) -> LayerResult<OptimizedAst> {
        let mut optimizer = PatternOptimizer;
        optimizer.optimize(ast, ctx)
    }
}

/// 模式优化器
pub struct PatternOptimizer;

impl PatternOptimizer {
    fn optimize(&mut self, ast: &TypedAst, ctx: &mut PatternContext) -> LayerResult<OptimizedAst> {
        match ast {
            TypedAst::Program(stmts) => {
                let mut optimized = Vec::new();
                for stmt in stmts {
                    optimized.push(self.optimize(stmt, ctx)?);
                }
                Ok(OptimizedAst::Program(optimized))
            }
            TypedAst::Function(func) => self.optimize_function(func, ctx),
            TypedAst::Struct(_) => Err(LayerError::PatternError("Structs should be resolved".to_string())),
            TypedAst::Enum(_) => Err(LayerError::PatternError("Enums should be resolved".to_string())),
            TypedAst::Let(let_stmt) => self.optimize_let(let_stmt, ctx),
            TypedAst::Return(expr) => self.optimize_return(expr.as_ref(), ctx),
            TypedAst::Expr(expr) => {
                let opt_expr = self.optimize_expr(expr, ctx)?;
                Ok(OptimizedAst::Expr(opt_expr))
            }
        }
    }

    fn optimize_function(&mut self, func: &TypedFunctionDecl, ctx: &mut PatternContext) -> LayerResult<OptimizedAst> {
        let body = self.optimize_block(&func.body, ctx)?;

        // 决定是否内联（简单启发式：小函数内联）
        let is_inlined = ctx.enable_inline_expansion &&
            body.statements.len() <= 3 &&
            func.params.is_empty();

        if is_inlined {
            ctx.optimizations_applied.push(format!("Inlined function: {}", func.name));
        }

        Ok(OptimizedAst::Function(OptimizedFunctionDecl {
            name: func.name.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            body,
            is_inlined,
            span: func.span,
        }))
    }

    fn optimize_block(&mut self, block: &TypedBlock, ctx: &mut PatternContext) -> LayerResult<OptimizedBlock> {
        let mut statements = Vec::new();
        let mut has_dead_code = false;

        for (i, stmt) in block.statements.iter().enumerate() {
            // 死代码检测：return之后的代码
            if ctx.enable_dead_code_elimination && has_dead_code {
                ctx.optimizations_applied.push(format!("Eliminated dead code at statement {}", i));
                continue;
            }

            if let TypedAst::Return(_) = stmt {
                has_dead_code = true;
            }

            statements.push(self.optimize(stmt, ctx)?);
        }

        Ok(OptimizedBlock {
            statements,
            return_type: block.return_type.clone(),
            has_dead_code,
            span: block.span,
        })
    }

    fn optimize_let(&mut self, let_stmt: &TypedLetStmt, ctx: &mut PatternContext) -> LayerResult<OptimizedAst> {
        let value = self.optimize_expr(&let_stmt.value, ctx)?;

        // 常量折叠检测
        let is_folded = ctx.enable_constant_folding && value.is_constant;
        if is_folded {
            ctx.optimizations_applied.push(format!(
                "Constant folded variable: {} = {:?}",
                let_stmt.name, value.constant_value
            ));
        }

        Ok(OptimizedAst::Let(OptimizedLetStmt {
            name: let_stmt.name.clone(),
            ty: let_stmt.ty.clone(),
            mutable: let_stmt.mutable,
            value: Box::new(value),
            is_folded,
            span: let_stmt.span,
        }))
    }

    fn optimize_return(&mut self, expr: Option<&Box<TypedExpr>>, ctx: &mut PatternContext) -> LayerResult<OptimizedAst> {
        let opt_expr = match expr {
            Some(e) => Some(Box::new(self.optimize_expr(e, ctx)?)),
            None => None,
        };
        Ok(OptimizedAst::Return(opt_expr))
    }

    fn optimize_expr(&mut self, expr: &TypedExpr, ctx: &mut PatternContext) -> LayerResult<OptimizedExpr> {
        let (kind, is_constant, constant_value) = match &expr.kind {
            TypedExprKind::Literal(lit) => {
                (OptimizedExprKind::Literal(lit.clone()), true, Some(lit.clone()))
            }
            TypedExprKind::Identifier(name) => {
                (OptimizedExprKind::Identifier(name.clone()), false, None)
            }
            TypedExprKind::Binary(op, left, right) => {
                let opt_left = self.optimize_expr(left, ctx)?;
                let opt_right = self.optimize_expr(right, ctx)?;

                // 常量折叠
                if ctx.enable_constant_folding && opt_left.is_constant && opt_right.is_constant {
                    if let (Some(l), Some(r)) = (&opt_left.constant_value, &opt_right.constant_value) {
                        let folded = self.fold_constant(*op, l, r)?;
                        ctx.optimizations_applied.push(format!("Folded constant: {:?} {:?} {:?}", l, op, r));
                        return Ok(OptimizedExpr {
                            kind: OptimizedExprKind::ConstantFolded(folded.clone()),
                            ty: expr.ty.clone(),
                            is_constant: true,
                            constant_value: Some(folded),
                            span: expr.span,
                        });
                    }
                }

                (OptimizedExprKind::Binary(*op, Box::new(opt_left), Box::new(opt_right)), false, None)
            }
            TypedExprKind::Unary(op, expr) => {
                let opt_expr = self.optimize_expr(expr, ctx)?;

                // 常量折叠
                if ctx.enable_constant_folding && opt_expr.is_constant {
                    if let Some(v) = &opt_expr.constant_value {
                        let folded = self.fold_unary(*op, v)?;
                        return Ok(OptimizedExpr {
                            kind: OptimizedExprKind::ConstantFolded(folded.clone()),
                            ty: expr.ty.clone(),
                            is_constant: true,
                            constant_value: Some(folded),
                            span: expr.span,
                        });
                    }
                }

                (OptimizedExprKind::Unary(*op, Box::new(opt_expr)), false, None)
            }
            TypedExprKind::Call(callee, args) => {
                let opt_callee = self.optimize_expr(callee, ctx)?;
                let opt_args: LayerResult<Vec<_>> = args.iter()
                    .map(|a| self.optimize_expr(a, ctx))
                    .collect();
                (OptimizedExprKind::Call(Box::new(opt_callee), opt_args?), false, None)
            }
            TypedExprKind::FieldAccess(object, field) => {
                let opt_object = self.optimize_expr(object, ctx)?;
                (OptimizedExprKind::FieldAccess(Box::new(opt_object), field.clone()), false, None)
            }
            TypedExprKind::Index(object, index) => {
                let opt_object = self.optimize_expr(object, ctx)?;
                let opt_index = self.optimize_expr(index, ctx)?;
                (OptimizedExprKind::Index(Box::new(opt_object), Box::new(opt_index)), false, None)
            }
            TypedExprKind::If(cond, then_branch, else_branch) => {
                let opt_cond = self.optimize_expr(cond, ctx)?;
                let opt_then = self.optimize_block(then_branch, ctx)?;
                let opt_else = match else_branch {
                    Some(b) => Some(self.optimize_block(b, ctx)?),
                    None => None,
                };

                // 常量条件优化
                if ctx.enable_constant_folding && opt_cond.is_constant {
                    if let Some(Literal::Bool(b)) = &opt_cond.constant_value {
                        ctx.optimizations_applied.push(format!("Eliminated branch based on constant condition: {}", b));
                        if *b {
                            return Ok(OptimizedExpr {
                                kind: OptimizedExprKind::Block(opt_then),
                                ty: expr.ty.clone(),
                                is_constant: false,
                                constant_value: None,
                                span: expr.span,
                            });
                        } else if let Some(else_block) = opt_else {
                            return Ok(OptimizedExpr {
                                kind: OptimizedExprKind::Block(else_block),
                                ty: expr.ty.clone(),
                                is_constant: false,
                                constant_value: None,
                                span: expr.span,
                            });
                        }
                    }
                }

                (OptimizedExprKind::If(Box::new(opt_cond), opt_then, opt_else), false, None)
            }
            TypedExprKind::While(cond, body) => {
                let opt_cond = self.optimize_expr(cond, ctx)?;
                let opt_body = self.optimize_block(body, ctx)?;

                // 检测永假循环条件
                if ctx.enable_constant_folding && opt_cond.is_constant {
                    if let Some(Literal::Bool(false)) = &opt_cond.constant_value {
                        ctx.optimizations_applied.push("Eliminated unreachable while loop".to_string());
                        return Ok(OptimizedExpr {
                            kind: OptimizedExprKind::Block(OptimizedBlock {
                                statements: vec![],
                                return_type: Type::Unit,
                                has_dead_code: false,
                                span: body.span,
                            }),
                            ty: Type::Unit,
                            is_constant: true,
                            constant_value: None,
                            span: expr.span,
                        });
                    }
                }

                (OptimizedExprKind::While(Box::new(opt_cond), opt_body), false, None)
            }
            TypedExprKind::For(var, iterable, body) => {
                let opt_iterable = self.optimize_expr(iterable, ctx)?;
                let opt_body = self.optimize_block(body, ctx)?;
                (OptimizedExprKind::For(var.clone(), Box::new(opt_iterable), opt_body), false, None)
            }
            TypedExprKind::Block(block) => {
                let opt_block = self.optimize_block(block, ctx)?;
                (OptimizedExprKind::Block(opt_block), false, None)
            }
            TypedExprKind::Assignment(target, value) => {
                let opt_target = self.optimize_expr(target, ctx)?;
                let opt_value = self.optimize_expr(value, ctx)?;
                (OptimizedExprKind::Assignment(Box::new(opt_target), Box::new(opt_value)), false, None)
            }
        };

        Ok(OptimizedExpr {
            kind,
            ty: expr.ty.clone(),
            is_constant,
            constant_value,
            span: expr.span,
        })
    }

    fn fold_constant(&self, op: BinaryOp, left: &Literal, right: &Literal) -> LayerResult<Literal> {
        match (op, left, right) {
            (BinaryOp::Add, Literal::Integer(l), Literal::Integer(r)) => Ok(Literal::Integer(l + r)),
            (BinaryOp::Sub, Literal::Integer(l), Literal::Integer(r)) => Ok(Literal::Integer(l - r)),
            (BinaryOp::Mul, Literal::Integer(l), Literal::Integer(r)) => Ok(Literal::Integer(l * r)),
            (BinaryOp::Div, Literal::Integer(l), Literal::Integer(r)) => {
                if *r == 0 {
                    Err(LayerError::PatternError("Division by zero".to_string()))
                } else {
                    Ok(Literal::Integer(l / r))
                }
            }
            (BinaryOp::Add, Literal::Float(l), Literal::Float(r)) => Ok(Literal::Float(l + r)),
            (BinaryOp::Sub, Literal::Float(l), Literal::Float(r)) => Ok(Literal::Float(l - r)),
            (BinaryOp::Mul, Literal::Float(l), Literal::Float(r)) => Ok(Literal::Float(l * r)),
            (BinaryOp::Div, Literal::Float(l), Literal::Float(r)) => Ok(Literal::Float(l / r)),
            (BinaryOp::And, Literal::Bool(l), Literal::Bool(r)) => Ok(Literal::Bool(*l && *r)),
            (BinaryOp::Or, Literal::Bool(l), Literal::Bool(r)) => Ok(Literal::Bool(*l || *r)),
            (BinaryOp::Eq, l, r) => Ok(Literal::Bool(std::mem::discriminant(l) == std::mem::discriminant(r) &&
                format!("{:?}", l) == format!("{:?}", r))),
            _ => Err(LayerError::PatternError(format!("Cannot fold {:?} {:?} {:?}", left, op, right))),
        }
    }

    fn fold_unary(&self, op: UnaryOp, expr: &Literal) -> LayerResult<Literal> {
        match (op, expr) {
            (UnaryOp::Neg, Literal::Integer(i)) => Ok(Literal::Integer(-i)),
            (UnaryOp::Neg, Literal::Float(f)) => Ok(Literal::Float(-f)),
            (UnaryOp::Not, Literal::Bool(b)) => Ok(Literal::Bool(!b)),
            _ => Err(LayerError::PatternError(format!("Cannot fold {:?} {:?}", op, expr))),
        }
    }
}

/// Pattern Layer 实现
pub struct PatternLayer;

impl Layer for PatternLayer {
    const ID: LayerId = LayerId::Pattern;
    type Input = TypedAst;  // 来自Semantic层的类型化AST
    type Output = OptimizedAst; // 优化后的AST
    type Context = PatternContext;

    fn transform(input: Self::Input, ctx: &mut Self::Context) -> LayerResult<Self::Output> {
        Optimizer::optimize(&input, ctx)
    }
}

// ============================================================================
// Layer 4: Domain Layer - 代码生成层
// ============================================================================

/// 目标平台
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlatform {
    Native,      // 本地机器码
    Wasm,        // WebAssembly
    Llvm,        // LLVM IR
    C,           // C语言
    Interpreter, // 解释执行
}

/// 代码生成上下文
pub struct DomainContext {
    pub target: TargetPlatform,
    pub output: String,
    pub indent_level: usize,
    pub symbol_table: SymbolTable,
}

impl DomainContext {
    pub fn new(target: TargetPlatform) -> Self {
        Self {
            target,
            output: String::new(),
            indent_level: 0,
            symbol_table: SymbolTable::new(),
        }
    }

    pub fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub fn writeln(&mut self, s: &str) {
        self.write(&"  ".repeat(self.indent_level));
        self.write(s);
        self.write("\n");
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
}

/// 代码生成器
pub struct CodeGenerator;

impl CodeGenerator {
    pub fn generate(ast: &OptimizedAst, ctx: &mut DomainContext) -> LayerResult<String> {
        let mut gen = DomainGenerator;
        gen.generate(ast, ctx)?;
        Ok(ctx.output.clone())
    }
}

/// 领域代码生成器
pub struct DomainGenerator;

impl DomainGenerator {
    fn generate(&mut self, ast: &OptimizedAst, ctx: &mut DomainContext) -> LayerResult<()> {
        match ctx.target {
            TargetPlatform::C => self.generate_c(ast, ctx),
            TargetPlatform::Interpreter => self.generate_interpreter(ast, ctx),
            _ => Err(LayerError::DomainError(format!("Target {:?} not yet implemented", ctx.target))),
        }
    }

    fn generate_c(&mut self, ast: &OptimizedAst, ctx: &mut DomainContext) -> LayerResult<()> {
        match ast {
            OptimizedAst::Program(stmts) => {
                ctx.writeln("#include <stdio.h>");
                ctx.writeln("#include <stdbool.h>");
                ctx.writeln("#include <stdlib.h>");
                ctx.writeln("#include <string.h>");
                ctx.writeln("");

                for stmt in stmts {
                    self.generate_c(stmt, ctx)?;
                    ctx.writeln("");
                }

                Ok(())
            }
            OptimizedAst::Function(func) => {
                let ret_type = self.c_type(&func.return_type);
                ctx.write(&format!("{} {}(", ret_type, func.name));

                for (i, param) in func.params.iter().enumerate() {
                    if i > 0 { ctx.write(", "); }
                    ctx.write(&format!("{} {}", self.c_type(&param.ty), param.name));
                }

                if func.params.is_empty() {
                    ctx.write("void");
                }

                ctx.write(") {\n");
                ctx.indent();

                self.generate_c_block(&func.body, ctx)?;

                ctx.dedent();
                ctx.writeln("}");
                Ok(())
            }
            OptimizedAst::Let(let_stmt) => {
                let ty = self.c_type(&let_stmt.ty);
                let mutable = if let_stmt.mutable { "" } else { "const " };
                ctx.write(&format!("{}{} {} = ", mutable, ty, let_stmt.name));
                self.generate_c_expr(&let_stmt.value, ctx)?;
                ctx.write(";");
                Ok(())
            }
            OptimizedAst::Return(expr) => {
                ctx.write("return");
                if let Some(e) = expr {
                    ctx.write(" ");
                    self.generate_c_expr(e, ctx)?;
                }
                ctx.write(";");
                Ok(())
            }
            OptimizedAst::Expr(expr) => {
                self.generate_c_expr(expr, ctx)?;
                ctx.write(";");
                Ok(())
            }
        }
    }

    fn generate_c_block(&mut self, block: &OptimizedBlock, ctx: &mut DomainContext) -> LayerResult<()> {
        for stmt in &block.statements {
            match stmt {
                OptimizedAst::Let(let_stmt) => {
                    let ty = self.c_type(&let_stmt.ty);
                    let mutable = if let_stmt.mutable { "" } else { "const " };
                    ctx.write(&format!("{}{} {} = ", mutable, ty, let_stmt.name));
                    self.generate_c_expr(&let_stmt.value, ctx)?;
                    ctx.writeln(";");
                }
                OptimizedAst::Return(expr) => {
                    ctx.write("return");
                    if let Some(e) = expr {
                        ctx.write(" ");
                        self.generate_c_expr(e, ctx)?;
                    }
                    ctx.writeln(";");
                }
                OptimizedAst::Expr(expr) => {
                    self.generate_c_expr(expr, ctx)?;
                    ctx.writeln(";");
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn generate_c_expr(&mut self, expr: &OptimizedExpr, ctx: &mut DomainContext) -> LayerResult<()> {
        match &expr.kind {
            OptimizedExprKind::Literal(lit) => {
                match lit {
                    Literal::Integer(i) => ctx.write(&i.to_string()),
                    Literal::Float(f) => ctx.write(&f.to_string()),
                    Literal::String(s) => ctx.write(&format!("\"{}\"", s)),
                    Literal::Bool(b) => ctx.write(if *b { "true" } else { "false" }),
                }
            }
            OptimizedExprKind::Identifier(name) => ctx.write(name),
            OptimizedExprKind::Binary(op, left, right) => {
                self.generate_c_expr(left, ctx)?;
                let op_str = match op {
                    BinaryOp::Add => " + ",
                    BinaryOp::Sub => " - ",
                    BinaryOp::Mul => " * ",
                    BinaryOp::Div => " / ",
                    BinaryOp::Mod => " % ",
                    BinaryOp::Eq => " == ",
                    BinaryOp::Ne => " != ",
                    BinaryOp::Lt => " < ",
                    BinaryOp::Gt => " > ",
                    BinaryOp::Le => " <= ",
                    BinaryOp::Ge => " >= ",
                    BinaryOp::And => " && ",
                    BinaryOp::Or => " || ",
                };
                ctx.write(op_str);
                self.generate_c_expr(right, ctx)?;
            }
            OptimizedExprKind::Unary(op, expr) => {
                match op {
                    UnaryOp::Neg => ctx.write("-"),
                    UnaryOp::Not => ctx.write("!"),
                }
                self.generate_c_expr(expr, ctx)?;
            }
            OptimizedExprKind::Call(callee, args) => {
                self.generate_c_expr(callee, ctx)?;
                ctx.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { ctx.write(", "); }
                    self.generate_c_expr(arg, ctx)?;
                }
                ctx.write(")");
            }
            OptimizedExprKind::ConstantFolded(lit) => {
                match lit {
                    Literal::Integer(i) => ctx.write(&i.to_string()),
                    Literal::Float(f) => ctx.write(&f.to_string()),
                    Literal::String(s) => ctx.write(&format!("\"{}\"", s)),
                    Literal::Bool(b) => ctx.write(if *b { "true" } else { "false" }),
                }
            }
            _ => ctx.write("/* unsupported expression */"),
        }
        Ok(())
    }

    fn c_type(&self, ty: &Type) -> String {
        match ty {
            Type::Unit => "void".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Int => "int64_t".to_string(),
            Type::Float => "double".to_string(),
            Type::String => "const char*".to_string(),
            Type::Reference(t) => format!("{}*", self.c_type(t)),
            Type::MutableReference(t) => format!("{}*", self.c_type(t)),
            _ => "/* unknown type */".to_string(),
        }
    }

    fn generate_interpreter(&mut self, ast: &OptimizedAst, ctx: &mut DomainContext) -> LayerResult<()> {
        // 解释器生成：生成字节码或AST遍历代码
        ctx.writeln("// Interpreted execution");
        ctx.writeln(&format!("// AST: {:?}", ast));
        Ok(())
    }
}

/// Domain Layer 实现
pub struct DomainLayer;

impl Layer for DomainLayer {
    const ID: LayerId = LayerId::Domain;
    type Input = OptimizedAst;  // 来自Pattern层的优化AST
    type Output = String;       // 生成的代码
    type Context = DomainContext;

    fn transform(input: Self::Input, ctx: &mut Self::Context) -> LayerResult<Self::Output> {
        CodeGenerator::generate(&input, ctx)
    }
}

// ============================================================================
// 层间类型安全接口
// ============================================================================

/// 层间转换器 - 确保类型安全的层间通信
pub struct LayerTransformer;

impl LayerTransformer {
    /// 执行完整的Syntax -> Semantic转换
    pub fn syntax_to_semantic(
        ast: AstNode,
        ctx: &mut SemanticContext,
    ) -> LayerResult<TypedAst> {
        let boundary = LayerBoundary::<SyntaxLayer, SemanticLayer>::new(true);
        boundary.validate(&ast)?;
        SemanticLayer::transform(ast, ctx)
    }

    /// 执行Semantic -> Pattern转换
    pub fn semantic_to_pattern(
        typed_ast: TypedAst,
        ctx: &mut PatternContext,
    ) -> LayerResult<OptimizedAst> {
        let boundary = LayerBoundary::<SemanticLayer, PatternLayer>::new(true);
        boundary.validate(&typed_ast)?;
        PatternLayer::transform(typed_ast, ctx)
    }

    /// 执行Pattern -> Domain转换
    pub fn pattern_to_domain(
        optimized_ast: OptimizedAst,
        ctx: &mut DomainContext,
    ) -> LayerResult<String> {
        let boundary = LayerBoundary::<PatternLayer, DomainLayer>::new(true);
        boundary.validate(&optimized_ast)?;
        DomainLayer::transform(optimized_ast, ctx)
    }
}

// ============================================================================
// 编译器管道
// ============================================================================

/// 编译器配置
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub target: TargetPlatform,
    pub enable_optimizations: bool,
    pub enable_runtime_checks: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target: TargetPlatform::C,
            enable_optimizations: true,
            enable_runtime_checks: true,
        }
    }
}

/// 编译结果
#[derive(Debug)]
pub struct CompileResult {
    pub success: bool,
    pub output: Option<String>,
    pub errors: Vec<LayerError>,
    pub optimizations_applied: Vec<String>,
}

/// 分层编译器
pub struct LayeredCompiler {
    config: CompilerConfig,
}

impl LayeredCompiler {
    pub fn new(config: CompilerConfig) -> Self {
        Self { config }
    }

    /// 编译源代码
    pub fn compile(&self, source: String) -> CompileResult {
        let mut errors = Vec::new();
        let mut optimizations_applied = Vec::new();

        // Step 1: Syntax Layer
        let mut syntax_ctx = SyntaxContext::new(source);
        let ast = match SyntaxLayer::transform(syntax_ctx.source.clone(), &mut syntax_ctx) {
            Ok(ast) => ast,
            Err(e) => {
                errors.push(e);
                return CompileResult {
                    success: false,
                    output: None,
                    errors,
                    optimizations_applied,
                };
            }
        };

        // Step 2: Semantic Layer
        let mut semantic_ctx = SemanticContext::new();
        let typed_ast = match LayerTransformer::syntax_to_semantic(ast, &mut semantic_ctx) {
            Ok(ta) => ta,
            Err(e) => {
                errors.push(e);
                errors.extend(semantic_ctx.errors);
                return CompileResult {
                    success: false,
                    output: None,
                    errors,
                    optimizations_applied,
                };
            }
        };

        // Step 3: Pattern Layer
        let mut pattern_ctx = PatternContext::new();
        if !self.config.enable_optimizations {
            pattern_ctx.enable_constant_folding = false;
            pattern_ctx.enable_dead_code_elimination = false;
            pattern_ctx.enable_inline_expansion = false;
        }

        let optimized_ast = match LayerTransformer::semantic_to_pattern(typed_ast, &mut pattern_ctx) {
            Ok(oa) => oa,
            Err(e) => {
                errors.push(e);
                return CompileResult {
                    success: false,
                    output: None,
                    errors,
                    optimizations_applied,
                };
            }
        };
        optimizations_applied = pattern_ctx.optimizations_applied;

        // Step 4: Domain Layer
        let mut domain_ctx = DomainContext::new(self.config.target);
        let output = match LayerTransformer::pattern_to_domain(optimized_ast, &mut domain_ctx) {
            Ok(code) => code,
            Err(e) => {
                errors.push(e);
                return CompileResult {
                    success: false,
                    output: None,
                    errors,
                    optimizations_applied,
                };
            }
        };

        CompileResult {
            success: true,
            output: Some(output),
            errors,
            optimizations_applied,
        }
    }
}

// ============================================================================
// 测试和示例
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let source = "let x = 42;".to_string();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 6); // let, x, =, 42, ;, EOF
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_parser() {
        let source = r#"
            fn add(a: int, b: int) -> int {
                return a + b;
            }
        "#.to_string();

        let mut syntax_ctx = SyntaxContext::new(source);
        let result = SyntaxLayer::transform(syntax_ctx.source.clone(), &mut syntax_ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_checker() {
        let source = r#"
            fn main() {
                let x: int = 42;
                let y = x + 1;
            }
        "#.to_string();

        let compiler = LayeredCompiler::new(CompilerConfig::default());
        let result = compiler.compile(source);

        assert!(result.success, "Compilation failed: {:?}", result.errors);
    }

    #[test]
    fn test_constant_folding() {
        let source = r#"
            fn main() {
                let x = 1 + 2 + 3;
            }
        "#.to_string();

        let compiler = LayeredCompiler::new(CompilerConfig::default());
        let result = compiler.compile(source);

        assert!(result.success);
        assert!(!result.optimizations_applied.is_empty(), "Expected constant folding");
    }

    #[test]
    fn test_full_pipeline() {
        let source = r#"
            fn factorial(n: int) -> int {
                if n <= 1 {
                    return 1;
                }
                return n * factorial(n - 1);
            }

            fn main() {
                let result = factorial(5);
            }
        "#.to_string();

        let compiler = LayeredCompiler::new(CompilerConfig::default());
        let result = compiler.compile(source);

        assert!(result.success, "Compilation failed: {:?}", result.errors);

        if let Some(output) = result.output {
            assert!(output.contains("#include"));
            assert!(output.contains("factorial"));
        }
    }
}

// ============================================================================
// 主函数示例
// ============================================================================

pub fn main() {
    let source = r#"
        // 计算斐波那契数列
        fn fibonacci(n: int) -> int {
            if n <= 1 {
                return n;
            }
            return fibonacci(n - 1) + fibonacci(n - 2);
        }

        fn main() {
            let n = 10;
            let result = fibonacci(n);

            // 常量折叠测试
            let folded = 1 + 2 + 3 + 4 + 5;

            // 死代码消除测试
            if false {
                let unreachable = 999;
            }

            return result;
        }
    "#.to_string();

    println!("=== Layered Compiler Demo ===\n");

    let compiler = LayeredCompiler::new(CompilerConfig::default());
    let result = compiler.compile(source);

    if result.success {
        println!("Compilation successful!\n");
        println!("Optimizations applied:");
        for opt in &result.optimizations_applied {
            println!("  - {}", opt);
        }
        println!();

        if let Some(output) = result.output {
            println!("Generated code:");
            println!("{}", output);
        }
    } else {
        println!("Compilation failed:");
        for error in &result.errors {
            println!("  Error: {}", error);
        }
    }
}
