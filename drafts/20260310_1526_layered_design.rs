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

// ============================================================================
// Layer 3: PATTERN LAYER - 设计模式识别与优化
// ============================================================================

/// 模式层错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum PatternError {
    NoMatchingPattern(String),
    InvalidTransformation(String),
    CycleDetected,
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::NoMatchingPattern(desc) => {
                write!(f, "No matching pattern for: {}", desc)
            }
            PatternError::InvalidTransformation(msg) => {
                write!(f, "Invalid transformation: {}", msg)
            }
            PatternError::CycleDetected => write!(f, "Pattern transformation cycle detected"),
        }
    }
}

impl std::error::Error for PatternError {}

/// 设计模式类型
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    ConstantFolding,
    DeadCodeElimination,
    VariableInline,
    LoopUnrolling,
    ExpressionSimplification,
    TailRecursion,
}

/// 模式匹配结果
#[derive(Debug, Clone, PartialEq)]
pub struct PatternMatch {
    pub pattern: Pattern,
    pub priority: u32,
    pub description: String,
}

/// 语义层 -> 模式层 转换器
pub struct SemanticToPattern;

impl SemanticToPattern {
    pub fn new() -> Self {
        Self
    }

    pub fn identify_patterns(&self, expr: &Expr) -> Vec<PatternMatch> {
        let mut patterns = Vec::new();
        self.collect_patterns(expr, &mut patterns);
        patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
        patterns
    }

    fn collect_patterns(&self, expr: &Expr, patterns: &mut Vec<PatternMatch>) {
        match expr {
            Expr::Binary { op, left, right } => {
                if self.is_constant_folding(left, right) {
                    patterns.push(PatternMatch {
                        pattern: Pattern::ConstantFolding,
                        priority: 100,
                        description: format!("Constant folding: {:?} {:?} {:?}", left, op, right),
                    });
                }
                if self.is_simplifiable(op, left, right) {
                    patterns.push(PatternMatch {
                        pattern: Pattern::ExpressionSimplification,
                        priority: 80,
                        description: "Expression can be simplified".to_string(),
                    });
                }
                self.collect_patterns(left, patterns);
                self.collect_patterns(right, patterns);
            }
            Expr::Let { name, value, .. } => {
                if self.is_inline_candidate(value) {
                    patterns.push(PatternMatch {
                        pattern: Pattern::VariableInline,
                        priority: 70,
                        description: format!("Variable '{}' can be inlined", name),
                    });
                }
                self.collect_patterns(value, patterns);
            }
            Expr::Call { func, args } => {
                if self.is_tail_recursive(func, args) {
                    patterns.push(PatternMatch {
                        pattern: Pattern::TailRecursion,
                        priority: 90,
                        description: "Tail recursion detected".to_string(),
                    });
                }
                for arg in args {
                    self.collect_patterns(arg, patterns);
                }
            }
            _ => {}
        }
    }

    fn is_constant_folding(&self, left: &Expr, right: &Expr) -> bool {
        matches!(left, Expr::Literal(_)) && matches!(right, Expr::Literal(_))
    }

    fn is_simplifiable(&self, op: &BinaryOp, _left: &Expr, right: &Expr) -> bool {
        match (op, right) {
            (BinaryOp::Add, Expr::Literal(Literal::Number(0.0))) => true,
            (BinaryOp::Mul, Expr::Literal(Literal::Number(1.0))) => true,
            (BinaryOp::Mul, Expr::Literal(Literal::Number(0.0))) => true,
            _ => false,
        }
    }

    fn is_inline_candidate(&self, value: &Expr) -> bool {
        matches!(value, Expr::Literal(_) | Expr::Variable(_))
    }

    fn is_tail_recursive(&self, _func: &Expr, _args: &[Expr]) -> bool {
        false
    }

    pub fn transform(&self, expr: Expr) -> Result<Expr, PatternError> {
        self.apply_patterns(expr, 10)
    }

    fn apply_patterns(&self, expr: Expr, max_iterations: u32) -> Result<Expr, PatternError> {
        let mut current = expr;
        let mut iterations = 0;

        loop {
            if iterations >= max_iterations {
                break;
            }
            let patterns = self.identify_patterns(&current);
            if patterns.is_empty() {
                break;
            }
            let new_expr = self.apply_first_pattern(current, &patterns)?;
            current = new_expr;
            iterations += 1;
        }

        Ok(current)
    }

    fn apply_first_pattern(&self, expr: Expr, patterns: &[PatternMatch]) -> Result<Expr, PatternError> {
        if patterns.is_empty() {
            return Ok(expr);
        }
        match patterns[0].pattern {
            Pattern::ConstantFolding => self.fold_constants(expr),
            Pattern::ExpressionSimplification => self.simplify_expression(expr),
            _ => Ok(expr),
        }
    }

    fn fold_constants(&self, expr: Expr) -> Result<Expr, PatternError> {
        match expr {
            Expr::Binary { op, left, right } => {
                match (op, left.as_ref(), right.as_ref()) {
                    (BinaryOp::Add, Expr::Literal(Literal::Number(a)), Expr::Literal(Literal::Number(b))) => {
                        Ok(Expr::Literal(Literal::Number(a + b)))
                    }
                    (BinaryOp::Sub, Expr::Literal(Literal::Number(a)), Expr::Literal(Literal::Number(b))) => {
                        Ok(Expr::Literal(Literal::Number(a - b)))
                    }
                    (BinaryOp::Mul, Expr::Literal(Literal::Number(a)), Expr::Literal(Literal::Number(b))) => {
                        Ok(Expr::Literal(Literal::Number(a * b)))
                    }
                    (BinaryOp::Div, Expr::Literal(Literal::Number(a)), Expr::Literal(Literal::Number(b))) => {
                        if *b != 0.0 {
                            Ok(Expr::Literal(Literal::Number(a / b)))
                        } else {
                            Err(PatternError::InvalidTransformation("Division by zero".to_string()))
                        }
                    }
                    _ => Ok(Expr::Binary { op, left, right }),
                }
            }
            _ => Ok(expr),
        }
    }

    fn simplify_expression(&self, expr: Expr) -> Result<Expr, PatternError> {
        match expr {
            Expr::Binary { op, left, right } => {
                match (op, right.as_ref()) {
                    (BinaryOp::Add, Expr::Literal(Literal::Number(0.0))) => Ok(*left),
                    (BinaryOp::Mul, Expr::Literal(Literal::Number(1.0))) => Ok(*left),
                    (BinaryOp::Mul, Expr::Literal(Literal::Number(0.0))) => {
                        Ok(Expr::Literal(Literal::Number(0.0)))
                    }
                    _ => Ok(Expr::Binary { op, left, right }),
                }
            }
            _ => Ok(expr),
        }
    }
}

// ============================================================================
// Layer 4: DOMAIN LAYER - 领域模型映射
// ============================================================================

/// 领域层错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    InvalidMapping(String),
    UnsupportedPattern(String),
    DomainConstraintViolation(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::InvalidMapping(msg) => write!(f, "Invalid mapping: {}", msg),
            DomainError::UnsupportedPattern(p) => write!(f, "Unsupported pattern: {}", p),
            DomainError::DomainConstraintViolation(msg) => {
                write!(f, "Domain constraint violated: {}", msg)
            }
        }
    }
}

impl std::error::Error for DomainError {}

/// 领域概念类型
#[derive(Debug, Clone, PartialEq)]
pub enum DomainConcept {
    Computation {
        name: String,
        inputs: Vec<String>,
        formula: Box<DomainConcept>,
    },
    DataFlow {
        source: String,
        target: String,
        transformation: Option<Box<DomainConcept>>,
    },
    BusinessRule {
        condition: Box<DomainConcept>,
        action: Box<DomainConcept>,
    },
    Entity {
        name: String,
        attributes: Vec<(String, DomainType)>,
    },
    Value {
        data: DomainValue,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomainType {
    Number,
    Text,
    Boolean,
    Date,
    Reference(String),
    List(Box<DomainType>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomainValue {
    Number(f64),
    Text(String),
    Boolean(bool),
    List(Vec<DomainValue>),
}

/// 模式层 -> 领域层 转换器
pub struct PatternToDomain;

impl PatternToDomain {
    pub fn new() -> Self {
        Self
    }

    pub fn transform(&self, expr: &Expr) -> Result<DomainConcept, DomainError> {
        match expr {
            Expr::Let { name, value, .. } => self.transform_let(name, value),
            Expr::Binary { op, left, right } => self.transform_binary(op, left, right),
            Expr::Literal(lit) => self.transform_literal(lit),
            Expr::Variable(name) => {
                Ok(DomainConcept::Value {
                    data: DomainValue::Text(name.clone()),
                })
            }
            Expr::Function { name, params, body, .. } => {
                self.transform_function(name, params, body)
            }
            _ => Err(DomainError::UnsupportedPattern(format!("{:?}", expr))),
        }
    }

    fn transform_let(&self, name: &str, value: &Expr) -> Result<DomainConcept, DomainError> {
        let formula = self.transform(value)?;
        let inputs = self.extract_variables(value);
        Ok(DomainConcept::Computation {
            name: name.to_string(),
            inputs,
            formula: Box::new(formula),
        })
    }

    fn transform_binary(&self, op: &BinaryOp, left: &Expr, right: &Expr) -> Result<DomainConcept, DomainError> {
        let left_concept = self.transform(left)?;
        let right_concept = self.transform(right)?;
        let op_name = match op {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "subtract",
            BinaryOp::Mul => "multiply",
            BinaryOp::Div => "divide",
            BinaryOp::Eq => "equals",
            BinaryOp::Lt => "less_than",
            BinaryOp::Gt => "greater_than",
        };
        Ok(DomainConcept::Computation {
            name: format!("binary_{}", op_name),
            inputs: vec![],
            formula: Box::new(DomainConcept::Value {
                data: DomainValue::Text(format!("{:?} {} {:?}", left_concept, op_name, right_concept)),
            }),
        })
    }

    fn transform_literal(&self, lit: &Literal) -> Result<DomainConcept, DomainError> {
        let value = match lit {
            Literal::Number(n) => DomainValue::Number(*n),
            Literal::Boolean(b) => DomainValue::Boolean(*b),
            Literal::String(s) => DomainValue::Text(s.clone()),
        };
        Ok(DomainConcept::Value { data: value })
    }

    fn transform_function(&self, name: &str, params: &[(String, Type)], body: &Expr) -> Result<DomainConcept, DomainError> {
        let attributes: Vec<(String, DomainType)> = params
            .iter()
            .map(|(name, ty)| (name.clone(), self.type_to_domain(ty)))
            .collect();
        let _body_concept = self.transform(body)?;
        Ok(DomainConcept::Entity {
            name: name.to_string(),
            attributes: [
                attributes,
                vec![("body".to_string(), DomainType::Text)],
            ]
            .concat(),
        })
    }

    fn type_to_domain(&self, ty: &Type) -> DomainType {
        match ty {
            Type::Number => DomainType::Number,
            Type::Boolean => DomainType::Boolean,
            Type::String => DomainType::Text,
            Type::Custom(name) => DomainType::Reference(name.clone()),
            _ => DomainType::Text,
        }
    }

    fn extract_variables(&self, expr: &Expr) -> Vec<String> {
        let mut vars = Vec::new();
        self.collect_vars(expr, &mut vars);
        vars
    }

    fn collect_vars(&self, expr: &Expr, vars: &mut Vec<String>) {
        match expr {
            Expr::Variable(name) => {
                if !vars.contains(name) {
                    vars.push(name.clone());
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_vars(left, vars);
                self.collect_vars(right, vars);
            }
            Expr::Let { value, .. } => {
                self.collect_vars(value, vars);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.collect_vars(arg, vars);
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// 层间转换器: 完整的转换管道
// ============================================================================

/// 分层架构管道
pub struct LayeredPipeline;

impl LayeredPipeline {
    pub fn new() -> Self {
        Self
    }

    pub fn process(&self, input: &str) -> Result<DomainConcept, LayeredError> {
        let tokens = syntax_layer(input).map_err(LayeredError::Syntax)?;
        let syntax_to_semantic = SyntaxToSemantic::new();
        let ast = syntax_to_semantic.transform(tokens).map_err(LayeredError::Semantic)?;
        let mut type_checker = TypeChecker::new();
        type_checker.check(&ast).map_err(LayeredError::Semantic)?;
        let semantic_to_pattern = SemanticToPattern::new();
        let optimized_ast = semantic_to_pattern.transform(ast).map_err(LayeredError::Pattern)?;
        let pattern_to_domain = PatternToDomain::new();
        let domain_model = pattern_to_domain.transform(&optimized_ast).map_err(LayeredError::Domain)?;
        Ok(domain_model)
    }
}

/// 统一错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum LayeredError {
    Syntax(SyntaxError),
    Semantic(SemanticError),
    Pattern(PatternError),
    Domain(DomainError),
}

impl fmt::Display for LayeredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayeredError::Syntax(e) => write!(f, "Syntax Error: {}", e),
            LayeredError::Semantic(e) => write!(f, "Semantic Error: {}", e),
            LayeredError::Pattern(e) => write!(f, "Pattern Error: {}", e),
            LayeredError::Domain(e) => write!(f, "Domain Error: {}", e),
        }
    }
}

impl std::error::Error for LayeredError {}

impl From<SyntaxError> for LayeredError {
    fn from(e: SyntaxError) -> Self {
        LayeredError::Syntax(e)
    }
}

impl From<SemanticError> for LayeredError {
    fn from(e: SemanticError) -> Self {
        LayeredError::Semantic(e)
    }
}

impl From<PatternError> for LayeredError {
    fn from(e: PatternError) -> Self {
        LayeredError::Pattern(e)
    }
}

impl From<DomainError> for LayeredError {
    fn from(e: DomainError) -> Self {
        LayeredError::Domain(e)
    }
}

// ============================================================================
// 状态空间约束实现
// ============================================================================

/// 每层的状态空间约束
pub trait StateSpaceConstraint {
    type Input;
    type Output;
    type Error;
    fn validate_input(&self, input: &Self::Input) -> Result<(), Self::Error>;
    fn validate_output(&self, output: &Self::Output) -> Result<(), Self::Error>;
}

/// 语法层约束: Token序列必须有效
pub struct SyntaxConstraint;

impl StateSpaceConstraint for SyntaxConstraint {
    type Input = String;
    type Output = Vec<Token>;
    type Error = SyntaxError;

    fn validate_input(&self, _input: &Self::Input) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_output(&self, output: &Self::Output) -> Result<(), Self::Error> {
        if let Some(Token::EOF) = output.last() {
            Ok(())
        } else {
            Err(SyntaxError::UnexpectedEOF)
        }
    }
}

/// 语义层约束: AST必须类型正确
pub struct SemanticConstraint;

impl StateSpaceConstraint for SemanticConstraint {
    type Input = Vec<Token>;
    type Output = Expr;
    type Error = SemanticError;

    fn validate_input(&self, _input: &Self::Input) -> Result<(), Self::Error> {
        Ok(())
    }

    fn validate_output(&self, _output: &Self::Output) -> Result<(), Self::Error> {
        Ok(())
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_layer() {
        let input = "let x = 42";
        let tokens = syntax_layer(input).unwrap();
        assert!(matches!(tokens[0], Token::Keyword(_)));
        assert!(matches!(tokens[1], Token::Identifier(_)));
        assert!(matches!(tokens[2], Token::Operator(_)));
        assert!(matches!(tokens[3], Token::Number(42.0)));
    }

    #[test]
    fn test_semantic_layer() {
        let input = "let x = 42";
        let tokens = syntax_layer(input).unwrap();
        let transformer = SyntaxToSemantic::new();
        let ast = transformer.transform(tokens).unwrap();
        assert!(matches!(ast, Expr::Let { .. }));
    }

    #[test]
    fn test_type_checker() {
        let input = "let x = 1 + 2";
        let tokens = syntax_layer(input).unwrap();
        let transformer = SyntaxToSemantic::new();
        let ast = transformer.transform(tokens).unwrap();
        let mut checker = TypeChecker::new();
        let ty = checker.check(&ast).unwrap();
        assert_eq!(ty, Type::Number);
    }

    #[test]
    fn test_pattern_layer() {
        let input = "let x = 1 + 2";
        let tokens = syntax_layer(input).unwrap();
        let transformer = SyntaxToSemantic::new();
        let ast = transformer.transform(tokens).unwrap();
        let pattern_transformer = SemanticToPattern::new();
        let patterns = pattern_transformer.identify_patterns(&ast);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_constant_folding() {
        let expr = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Literal(Literal::Number(1.0))),
            right: Box::new(Expr::Literal(Literal::Number(2.0))),
        };
        let transformer = SemanticToPattern::new();
        let optimized = transformer.transform(expr).unwrap();
        assert!(matches!(
            optimized,
            Expr::Literal(Literal::Number(3.0))
        ));
    }

    #[test]
    fn test_full_pipeline() {
        let pipeline = LayeredPipeline::new();
        let result = pipeline.process("let x = 42");
        assert!(result.is_ok());
    }
}
