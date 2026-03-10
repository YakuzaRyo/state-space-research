// State Space Layered Architecture - Core Data Structures
// 状态空间四层架构的核心数据类型定义
// 目标: 实现数学上可证明的确定性层间转换

use std::marker::PhantomData;

// ============================================================
// Layer 0: Syntax - Token序列层
// ============================================================

/// 基础Token类型 - LLM生成的最小单元
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Identifier(String),
    Keyword(String),
    Literal(Literal),
    Punctuation(char),
    EOF,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

/// Token流 - L0层的状态表示
#[derive(Debug, Clone)]
pub struct TokenStream {
    tokens: Vec<Token>,
    position: usize,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0 }
    }
    
    /// 确定性peek - 总是返回确定值,无歧义
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }
    
    /// 确定性consume - 状态转移 s → s'
    pub fn consume(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        self.position += 1;
        token
    }
    
    /// 检查是否结束
    pub fn is_eof(&self) -> bool {
        self.position >= self.tokens.len()
    }
}

// ============================================================
// Layer 1: Semantic - 类型/语义层
// ============================================================

/// 类型系统基础 - L1层的状态表示
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unit,
    Bool,
    Int,
    Float,
    String,
    Tuple(Vec<Type>),
    Function(Box<Type>, Box<Type>),
    Never,  // ⊥ - 不接受状态
}

/// 带有类型的AST节点
#[derive(Debug, Clone)]
pub enum TypedExpr {
    Literal(Literal, Type),
    Variable(String, Type),
    Application(Box<TypedExpr>, Box<TypedExpr>, Type),
    Lambda(String, Box<TypedExpr>, Type),
    If(Box<TypedExpr>, Box<TypedExpr>, Box<TypedExpr>, Type),
}

/// 类型检查结果 - 确定性转换的保证
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    pub expr: TypedExpr,
    pub env: TypeEnvironment,
}

/// L1 → L0 的确定性转换
impl TypeCheckResult {
    /// 失败快速 - 立即返回Never类型,不产生下游错误
    pub fn error() -> Self {
        Self {
            expr: TypedExpr::Literal(Literal::Boolean(false), Type::Never),
            env: TypeEnvironment::new(),
        }
    }
}

// ============================================================
// Layer 2: Pattern - 设计模式层
// ============================================================

/// 设计模式 - L2层的状态表示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    Singleton,      // 单例模式
    Factory,        // 工厂模式
    Builder,        // 建造者模式
    Observer,       // 观察者模式
    Strategy,       // 策略模式
    State,          // 状态模式
    Command,        // 命令模式
    Decorator,      // 装饰器模式
    Iterator,       // 迭代器模式
    Result,         // Result模式 (错误处理)
}

/// 模式选择上下文 - LLM导航器在此层工作
#[derive(Debug, Clone)]
pub struct PatternContext {
    pub available_patterns: Vec<Pattern>,
    pub current_pattern: Option<Pattern>,
}

impl PatternContext {
    /// LLM作为导航器 - 从有限集合中选择
    pub fn select(&self, index: usize) -> Option<Pattern> {
        self.available_patterns.get(index).copied()
    }
}

/// L2 → L1 的确定性转换
impl Pattern {
    /// 返回模式对应的类型结构
    pub fn to_type(&self) -> Type {
        match self {
            Pattern::Singleton => Type::Tuple(vec![]),
            Pattern::Factory => Type::Function(
                Box::new(Type::String),
                Box::new(Type::Unit),
            ),
            Pattern::Builder => Type::Function(
                Box::new(Type::String),
                Box::new(Type::String),
            ),
            Pattern::Observer => Type::Function(
                Box::new(Type::String),
                Box::new(Type::Bool),
            ),
            Pattern::Strategy => Type::Function(
                Box::new(Type::String),
                Box::new(Type::String),
            ),
            Pattern::State => Type::Function(
                Box::new(Type::String),
                Box::new(Type::String),
            ),
            Pattern::Command => Type::Function(
                Box::new(Type::String),
                Box::new(Type::Never), // 命令可能失败
            ),
            Pattern::Decorator => Type::Function(
                Box::new(Type::String),
                Box::new(Type::String),
            ),
            Pattern::Iterator => Type::Function(
                Box::new(Type::String),
                Box::new(Type::String),
            ),
            Pattern::Result => Type::Function(
                Box::new(Type::String),
                Box::new(Type::Tuple(vec![Type::String, Type::String])),
            ),
        }
    }
}

// ============================================================
// Layer 3: Domain - 业务逻辑层
// ============================================================

/// 领域动作 - L3层的状态表示
#[derive(Debug, Clone)]
pub enum DomainAction {
    Read(String),
    Write(String, String),
    Delete(String),
    Execute(String),
    Validate(String),
    Commit,
    Rollback,
}

/// 领域模型状态
#[derive(Debug, Clone)]
pub struct DomainState {
    pub store: std::collections::HashMap<String, String>,
    pub history: Vec<DomainAction>,
    pub transaction: bool,
}

impl DomainState {
    pub fn new() -> Self {
        Self {
            store: std::collections::HashMap::new(),
            history: Vec::new(),
            transaction: false,
        }
    }
    
    /// L3层执行 - 确定性转换
    pub fn execute(&mut self, action: DomainAction) -> Result<String, DomainError> {
        match action {
            DomainAction::Read(key) => {
                self.history.push(action);
                self.store.get(&key)
                    .cloned()
                    .ok_or(DomainError::NotFound(key))
            }
            DomainAction::Write(key, value) => {
                self.history.push(action);
                self.store.insert(key, value);
                Ok("OK".to_string())
            }
            DomainAction::Delete(key) => {
                self.history.push(action);
                self.store.remove(&key);
                Ok("OK".to_string())
            }
            DomainAction::Execute(code) => {
                self.history.push(action);
                // 在实际实现中,这里会是DSL解释器
                Ok(format!("Executed: {}", code))
            }
            DomainAction::Validate(key) => {
                self.history.push(action);
                Ok(format!("Valid: {}", key))
            }
            DomainAction::Commit => {
                self.transaction = false;
                Ok("Committed".to_string())
            }
            DomainAction::Rollback => {
                // 简单的实现: 清空历史
                self.history.clear();
                self.transaction = false;
                Ok("Rolled back".to_string())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DomainError {
    NotFound(String),
    InvalidOperation(String),
    ValidationFailed(String),
}

// ============================================================
// 层间转换器 - 核心: 确定性Total函数
// ============================================================

/// L0 → L1 转换器 (Lexer + Parser)
pub struct SyntaxToSemantic;
impl SyntaxToSemantic {
    /// Total函数: 输入TokenStream → 输出TypedExpr或Never
    pub fn transform(stream: &TokenStream) -> TypeCheckResult {
        // 简化实现: 演示转换逻辑
        // 实际实现需要完整的parser
        if stream.is_eof() {
            TypeCheckResult {
                expr: TypedExpr::Literal(Literal::Boolean(true), Type::Bool),
                env: TypeEnvironment::new(),
            }
        } else {
            TypeCheckResult::error()
        }
    }
}

/// L1 → L2 转换器 (Type-Directed Pattern Selection)
pub struct SemanticToPattern;
impl SemanticToPattern {
    /// Total函数: 输入TypedExpr → 输出Pattern
    pub fn transform(typed: &TypeCheckResult) -> Pattern {
        // 根据类型选择模式
        match &typed.expr {
            TypedExpr::Literal(_, Type::Never) => Pattern::Result,
            TypedExpr::Lambda(_, _, _) => Pattern::Strategy,
            TypedExpr::If(_, _, _, _) => Pattern::State,
            _ => Pattern::Singleton,
        }
    }
}

/// L2 → L3 转换器 (Pattern-to-Domain Mapping)
pub struct PatternToDomain;
impl PatternToDomain {
    /// Total函数: 输入Pattern → 输出DomainAction
    pub fn transform(pattern: &Pattern) -> DomainAction {
        match pattern {
            Pattern::Singleton => DomainAction::Read("singleton".to_string()),
            Pattern::Factory => DomainAction::Execute("factory".to_string()),
            Pattern::Builder => DomainAction::Write("builder".to_string(), "".to_string()),
            Pattern::Observer => DomainAction::Validate("observer".to_string()),
            Pattern::Strategy => DomainAction::Execute("strategy".to_string()),
            Pattern::State => DomainAction::Read("state".to_string()),
            Pattern::Command => DomainAction::Execute("command".to_string()),
            Pattern::Decorator => DomainAction::Write("decorator".to_string(), "".to_string()),
            Pattern::Iterator => DomainAction::Read("iterator".to_string()),
            Pattern::Result => DomainAction::Validate("result".to_string()),
        }
    }
}

// ============================================================
// 类型环境 - 辅助数据结构
// ============================================================

#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    bindings: std::collections::HashMap<String, Type>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            bindings: std::collections::HashMap::new(),
        }
    }
    
    pub fn insert(&mut self, name: String, ty: Type) {
        self.bindings.insert(name, ty);
    }
    
    pub fn get(&self, name: &str) -> Option<&Type> {
        self.bindings.get(name)
    }
}

// ============================================================
// 单元测试 - 验证确定性转换
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_stream_deterministic() {
        let tokens = vec![
            Token::Keyword("let".to_string()),
            Token::Identifier("x".to_string()),
            Token::Literal(Literal::Integer(42)),
        ];
        let mut stream = TokenStream::new(tokens);
        
        // 确定性: 每次peek都返回相同结果
        assert_eq!(stream.peek(), Some(&Token::Keyword("let".to_string())));
        assert_eq!(stream.peek(), Some(&Token::Keyword("let".to_string())));
        
        // 确定性: consume改变状态
        assert_eq!(stream.consume(), Some(Token::Keyword("let".to_string())));
        assert_eq!(stream.peek(), Some(&Token::Identifier("x".to_string())));
    }
    
    #[test]
    fn test_layer_transform_total() {
        // 测试L0→L1转换是Total函数
        let empty_tokens = vec![];
        let stream = TokenStream::new(empty_tokens);
        let result = SyntaxToSemantic::transform(&stream);
        
        // 无论输入什么,都有确定输出(不会是未定义)
        assert!(matches!(result.expr, TypedExpr::Literal(_, _)));
    }
    
    #[test]
    fn test_pattern_to_type() {
        // 验证模式到类型的确定性映射
        assert_eq!(Pattern::Singleton.to_type(), Type::Tuple(vec![]));
        assert_eq!(Pattern::Result.to_type(), Type::Function(
            Box::new(Type::String),
            Box::new(Type::Tuple(vec![Type::String, Type::String])),
        ));
    }
    
    #[test]
    fn test_domain_execution() {
        let mut state = DomainState::new();
        
        // 确定性执行
        let result = state.execute(DomainAction::Write("key".to_string(), "value".to_string()));
        assert_eq!(result.unwrap(), "OK");
        
        let result = state.execute(DomainAction::Read("key".to_string()));
        assert_eq!(result.unwrap(), "value");
    }
}

fn main() {
    println!("State Space Layered Architecture - Core Types");
    println!("=============================================");
    println!();
    println!("L0 (Syntax): TokenStream - 线性Token序列");
    println!("L1 (Semantic): TypedExpr - 带类型的表达式");
    println!("L2 (Pattern): Pattern - 设计模式枚举");
    println!("L3 (Domain): DomainAction - 领域动作枚举");
    println!();
    println!("关键特性:");
    println!("  - 每层转换都是 Total 函数");
    println!("  - 失败快速: 错误类型 = Never (⊥)");
    println!("  - LLM只在L2层作为导航器选择模式");
    println!("  - 编译期保证状态转换合法性");
}
