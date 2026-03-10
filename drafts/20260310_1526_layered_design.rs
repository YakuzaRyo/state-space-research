//! 分层架构实现: Syntax → Semantic → Pattern → Domain
//!
//! 核心问题: 如何实现四层之间的安全转换?
//! 答案: 使用类型系统约束 + 显式转换器 + 状态空间隔离

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Layer 1: SYNTAX LAYER - 解析器组合子实现
// ============================================================================

/// 语法层错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxError {
    UnexpectedToken { expected: String, found: String },
    InvalidNumber(String),
    UnmatchedBracket(char),
    UnexpectedEOF,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntaxError::UnexpectedToken { expected, found } => {
                write!(f, "Expected '{}', found '{}'", expected, found)
            }
            SyntaxError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            SyntaxError::UnmatchedBracket(c) => write!(f, "Unmatched bracket: {}", c),
            SyntaxError::UnexpectedEOF => write!(f, "Unexpected end of input"),
        }
    }
}

impl std::error::Error for SyntaxError {}

/// 原始Token - 语法层输出
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Identifier(String),
    Operator(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Keyword(String),
    EOF,
}

/// 解析器组合子类型
pub type ParserResult<T> = Result<(T, String), SyntaxError>;

/// 解析器函数类型
pub type Parser<T> = Box<dyn Fn(&str) -> ParserResult<T>>;

/// 解析数字
pub fn number_parser() -> Parser<f64> {
    Box::new(|input: &str| {
        let input = input.trim_start();
        let mut chars = input.chars().peekable();
        let mut num_str = String::new();
        let mut has_dot = false;

        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                chars.next();
            } else if c == '.' && !has_dot {
                has_dot = true;
                num_str.push(c);
                chars.next();
            } else {
                break;
            }
        }

        if num_str.is_empty() {
            return Err(SyntaxError::InvalidNumber(input.to_string()));
        }

        let remaining: String = chars.collect();
        num_str
            .parse::<f64>()
            .map(|n| (n, remaining))
            .map_err(|_| SyntaxError::InvalidNumber(num_str))
    })
}

/// 解析标识符
pub fn identifier_parser() -> Parser<String> {
    Box::new(|input: &str| {
        let input = input.trim_start();
        let mut chars = input.chars().peekable();
        let mut ident = String::new();

        if let Some(&c) = chars.peek() {
            if c.is_ascii_alphabetic() || c == '_' {
                ident.push(c);
                chars.next();
            } else {
                return Err(SyntaxError::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: c.to_string(),
                });
            }
        }

        while let Some(&c) = chars.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
                chars.next();
            } else {
                break;
            }
        }

        let remaining: String = chars.collect();
        Ok((ident, remaining))
    })
}

/// 解析特定字符串
pub fn string_parser(expected: &'static str) -> Parser<String> {
    Box::new(move |input: &str| {
        let input = input.trim_start();
        if input.starts_with(expected) {
            let remaining = input[expected.len()..].to_string();
            Ok((expected.to_string(), remaining))
        } else {
            Err(SyntaxError::UnexpectedToken {
                expected: expected.to_string(),
                found: input.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "EOF".to_string()),
            })
        }
    })
}

/// 组合子: 或者
pub fn or<T: 'static>(p1: Parser<T>, p2: Parser<T>) -> Parser<T> {
    Box::new(move |input: &str| {
        p1(input).or_else(|_| p2(input))
    })
}

/// 组合子: 映射
pub fn map<T: 'static, U: 'static>(p: Parser<T>, f: impl Fn(T) -> U + 'static) -> Parser<U> {
    Box::new(move |input: &str| {
        p(input).map(|(v, rest)| (f(v), rest))
    })
}

/// 组合子: 序列
pub fn and<T: 'static, U: 'static>(p1: Parser<T>, p2: Parser<U>) -> Parser<(T, U)> {
    Box::new(move |input: &str| {
        let (v1, rest1) = p1(input)?;
        let (v2, rest2) = p2(&rest1)?;
        Ok(((v1, v2), rest2))
    })
}

/// 组合子: 零次或多次
pub fn many<T: 'static + Clone>(p: Parser<T>) -> Parser<Vec<T>> {
    Box::new(move |input: &str| {
        let mut results = Vec::new();
        let mut current_input = input.to_string();

        loop {
            match p(&current_input) {
                Ok((v, rest)) => {
                    results.push(v);
                    current_input = rest;
                }
                Err(_) => break,
            }
        }

        Ok((results, current_input))
    })
}

/// 语法层: 解析表达式字符串为Token序列
pub fn syntax_layer(input: &str) -> Result<Vec<Token>, SyntaxError> {
    let mut tokens = Vec::new();
    let mut remaining = input.to_string();

    while !remaining.trim().is_empty() {
        let trimmed = remaining.trim_start();

        // 尝试解析数字
        if let Ok((num, rest)) = number_parser()(&trimmed) {
            tokens.push(Token::Number(num));
            remaining = rest;
            continue;
        }

        // 尝试解析标识符/关键字
        if let Ok((ident, rest)) = identifier_parser()(&trimmed) {
            let token = match ident.as_str() {
                "let" | "fn" | "if" | "else" | "return" => Token::Keyword(ident),
                _ => Token::Identifier(ident),
            };
            tokens.push(token);
            remaining = rest;
            continue;
        }

        // 解析单字符token
        let mut chars = trimmed.chars();
        if let Some(c) = chars.next() {
            let token = match c {
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                ';' => Token::Semicolon,
                '+' | '-' | '*' | '/' | '=' => Token::Operator(c.to_string()),
                _ => return Err(SyntaxError::UnexpectedToken {
                    expected: "valid token".to_string(),
                    found: c.to_string(),
                }),
            };
            tokens.push(token);
            remaining = chars.collect();
        } else {
            break;
        }
    }

    tokens.push(Token::EOF);
    Ok(tokens)
}

// ============================================================================
// Layer 2: SEMANTIC LAYER - AST + 类型系统
// ============================================================================

/// 语义层错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    UndefinedVariable(String),
    TypeMismatch { expected: Type, found: Type },
    InvalidOperation { op: String, types: Vec<Type> },
    InvalidAST(String),
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::UndefinedVariable(name) => {
                write!(f, "Undefined variable: {}", name)
            }
            SemanticError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {:?}, found {:?}", expected, found)
            }
            SemanticError::InvalidOperation { op, types } => {
                write!(f, "Invalid operation '{}' for types {:?}", op, types)
            }
            SemanticError::InvalidAST(msg) => write!(f, "Invalid AST: {}", msg),
        }
    }
}

impl std::error::Error for SemanticError {}

/// 类型系统
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Number,
    Boolean,
    String,
    Function { params: Vec<Type>, ret: Box<Type> },
    Custom(String),
    Unknown,
}

/// AST节点 - 语义层核心数据结构
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Variable(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Let {
        name: String,
        value: Box<Expr>,
        ty: Type,
    },
    Function {
        name: String,
        params: Vec<(String, Type)>,
        body: Box<Expr>,
        ret_ty: Type,
    },
    Block(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    Boolean(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Gt,
}

/// 类型环境
pub struct TypeEnv {
    bindings: HashMap<String, Type>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Type> {
        self.bindings.get(name)
    }

    pub fn set(&mut self, name: String, ty: Type) {
        self.bindings.insert(name, ty);
    }
}

/// 语法层 -> 语义层 转换器
pub struct SyntaxToSemantic;

impl SyntaxToSemantic {
    pub fn new() -> Self {
        Self
    }

    pub fn transform(&self, tokens: Vec<Token>) -> Result<Expr, SemanticError> {
        let mut pos = 0;
        self.parse_expr(&tokens, &mut pos)
    }

    fn parse_expr(&self, tokens: &[Token], pos: &mut usize) -> Result<Expr, SemanticError> {
        self.parse_let(tokens, pos)
            .or_else(|_| self.parse_binary(tokens, pos))
            .or_else(|_| self.parse_primary(tokens, pos))
    }

    fn parse_let(&self, tokens: &[Token], pos: &mut usize) -> Result<Expr, SemanticError> {
        if matches!(&tokens.get(*pos), Some(Token::Keyword(k)) if k == "let") {
            *pos += 1;

            let name = match &tokens.get(*pos) {
                Some(Token::Identifier(n)) => {
                    *pos += 1;
                    n.clone()
                }
                _ => return Err(SemanticError::InvalidAST(
                    "Expected identifier after 'let'".to_string()
                )),
            };

            if !matches!(&tokens.get(*pos), Some(Token::Operator(op)) if op == "=") {
                return Err(SemanticError::InvalidAST(
                    "Expected '=' after identifier".to_string()
                ));
            }
            *pos += 1;

            let value = self.parse_expr(tokens, pos)?;

            if matches!(&tokens.get(*pos), Some(Token::Semicolon)) {
                *pos += 1;
            }

            Ok(Expr::Let {
                name,
                value: Box::new(value),
                ty: Type::Unknown,
            })
        } else {
            Err(SemanticError::InvalidAST("Not a let expression".to_string()))
        }
    }

    fn parse_binary(&self, tokens: &[Token], pos: &mut usize) -> Result<Expr, SemanticError> {
        let left = self.parse_primary(tokens, pos)?;

        if let Some(Token::Operator(op_str)) = tokens.get(*pos) {
            let op = match op_str.as_str() {
                "+" => BinaryOp::Add,
                "-" => BinaryOp::Sub,
                "*" => BinaryOp::Mul,
                "/" => BinaryOp::Div,
                "=" => BinaryOp::Eq,
                _ => return Ok(left),
            };
            *pos += 1;

            let right = self.parse_primary(tokens, pos)?;
            Ok(Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_primary(&self, tokens: &[Token], pos: &mut usize) -> Result<Expr, SemanticError> {
        match tokens.get(*pos) {
            Some(Token::Number(n)) => {
                *pos += 1;
                Ok(Expr::Literal(Literal::Number(*n)))
            }
            Some(Token::Identifier(name)) => {
                *pos += 1;
                Ok(Expr::Variable(name.clone()))
            }
            Some(Token::LParen) => {
                *pos += 1;
                let expr = self.parse_expr(tokens, pos)?;
                if !matches!(tokens.get(*pos), Some(Token::RParen)) {
                    return Err(SemanticError::InvalidAST("Expected ')'".to_string()));
                }
                *pos += 1;
                Ok(expr)
            }
            _ => Err(SemanticError::InvalidAST("Unexpected token in primary".to_string())),
        }
    }
}

/// 类型检查器
pub struct TypeChecker {
    env: TypeEnv,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
        }
    }

    pub fn check(&mut self, expr: &Expr) -> Result<Type, SemanticError> {
        match expr {
            Expr::Literal(lit) => Ok(self.literal_type(lit)),
            Expr::Variable(name) => {
                self.env.get(name)
                    .cloned()
                    .ok_or_else(|| SemanticError::UndefinedVariable(name.clone()))
            }
            Expr::Binary { op, left, right } => {
                let left_ty = self.check(left)?;
                let right_ty = self.check(right)?;
                self.check_binary_op(op, &left_ty, &right_ty)
            }
            Expr::Let { name, value, ty: _ } => {
                let value_ty = self.check(value)?;
                self.env.set(name.clone(), value_ty.clone());
                Ok(value_ty)
            }
            _ => Ok(Type::Unknown),
        }
    }

    fn literal_type(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Number(_) => Type::Number,
            Literal::Boolean(_) => Type::Boolean,
            Literal::String(_) => Type::String,
        }
    }

    fn check_binary_op(&self, op: &BinaryOp, left: &Type, right: &Type) -> Result<Type, SemanticError> {
        match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                if *left == Type::Number && *right == Type::Number {
                    Ok(Type::Number)
                } else {
                    Err(SemanticError::InvalidOperation {
                        op: format!("{:?}", op),
                        types: vec![left.clone(), right.clone()],
                    })
                }
            }
            BinaryOp::Eq | BinaryOp::Lt | BinaryOp::Gt => {
                if left == right {
                    Ok(Type::Boolean)
                } else {
                    Err(SemanticError::TypeMismatch {
                        expected: left.clone(),
                        found: right.clone(),
                    })
                }
            }
        }
    }
}
