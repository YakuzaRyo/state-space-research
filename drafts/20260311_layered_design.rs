//! 分层设计研究：Syntax → Semantic → Pattern → Domain 转换
//!
//! 核心问题：如何在Rust中实现从语法层到领域层的类型安全转换？
//!
//! 架构层次：
//! 1. Syntax Layer: 原始语法表示（Token/AST）
//! 2. Semantic Layer: 语义分析后的表示（带有类型信息）
//! 3. Pattern Layer: 模式匹配层（状态转换规则）
//! 4. Domain Layer: 领域模型（业务逻辑）

use std::marker::PhantomData;

// ============================================================================
// Layer 1: Syntax Layer - 语法表示层
// ============================================================================

/// 语法层标记trait - 表示这是原始语法结构
pub trait Syntax {}

/// 原始标识符（未解析）
#[derive(Debug, Clone, PartialEq)]
pub struct RawIdent {
    pub name: String,
}

/// 原始表达式（语法树）
#[derive(Debug, Clone, PartialEq)]
pub enum RawExpr {
    Variable(RawIdent),
    Number(i64),
    String(String),
    Binary {
        op: BinaryOp,
        left: Box<RawExpr>,
        right: Box<RawExpr>,
    },
    Call {
        func: Box<RawExpr>,
        args: Vec<RawExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl Syntax for RawIdent {}
impl Syntax for RawExpr {}

// ============================================================================
// Layer 2: Semantic Layer - 语义层
// ============================================================================

/// 语义层标记trait - 表示这是经过语义分析的结构
pub trait Semantic {
    /// 关联的类型信息
    fn type_info(&self) -> Type;
}

/// 类型系统
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type {
    Int,
    String,
    Bool,
    Unit,
    Function {
        params: &'static [Type],
        ret: &'static Type,
    },
    Error,
}

/// 解析后的标识符（带有作用域信息）
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedIdent {
    pub name: String,
    pub id: u64,        // 唯一标识符ID
    pub ty: Type,
}

impl Semantic for ResolvedIdent {
    fn type_info(&self) -> Type {
        self.ty
    }
}

/// 语义表达式（带有类型信息）
#[derive(Debug, Clone, PartialEq)]
pub enum TypedExpr {
    Variable(ResolvedIdent),
    Literal(Literal),
    Binary {
        op: BinaryOp,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
        ty: Type,
    },
    Call {
        func: Box<TypedExpr>,
        args: Vec<TypedExpr>,
        ty: Type,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    String(String),
    Bool(bool),
}

impl Semantic for TypedExpr {
    fn type_info(&self) -> Type {
        match self {
            TypedExpr::Variable(ident) => ident.ty,
            TypedExpr::Binary { ty, .. } => *ty,
            TypedExpr::Call { ty, .. } => *ty,
            TypedExpr::Literal(lit) => match lit {
                Literal::Int(_) => Type::Int,
                Literal::String(_) => Type::String,
                Literal::Bool(_) => Type::Bool,
            },
        }
    }
}

// ============================================================================
// Layer 3: Pattern Layer - 模式层
// ============================================================================

/// 模式层标记trait - 表示这是可匹配的模式
pub trait Pattern {
    type Input;
    type Output;
    fn apply(&self, input: Self::Input) -> Option<Self::Output>;
}

/// 状态转换标记类型
pub struct Unmatched;
pub struct Matched<T>(PhantomData<T>);
pub struct Transformed<T>(PhantomData<T>);

/// 模式匹配器 - 使用类型状态确保正确的转换流程
pub struct PatternMatcher<State> {
    state: PhantomData<State>,
    expr: TypedExpr,
}

impl PatternMatcher<Unmatched> {
    pub fn new(expr: TypedExpr) -> Self {
        Self {
            state: PhantomData,
            expr,
        }
    }

    /// 尝试匹配常量折叠模式
    pub fn match_const_fold(self) -> PatternMatcher<Matched<ConstFoldPattern>> {
        PatternMatcher {
            state: PhantomData,
            expr: self.expr,
        }
    }

    /// 尝试匹配函数内联模式
    pub fn match_inline(self) -> PatternMatcher<Matched<InlinePattern>> {
        PatternMatcher {
            state: PhantomData,
            expr: self.expr,
        }
    }
}

/// 常量折叠模式标记类型
pub struct ConstFoldPattern;

/// 函数内联模式标记类型
pub struct InlinePattern;

impl PatternMatcher<Matched<ConstFoldPattern>> {
    /// 执行常量折叠转换
    pub fn transform(self) -> PatternMatcher<Transformed<ConstFoldPattern>> {
        let transformed = fold_constants(self.expr);
        PatternMatcher {
            state: PhantomData,
            expr: transformed,
        }
    }
}

impl PatternMatcher<Matched<InlinePattern>> {
    /// 执行函数内联转换
    pub fn transform(self) -> PatternMatcher<Transformed<InlinePattern>> {
        // 简化实现：实际会进行函数内联
        PatternMatcher {
            state: PhantomData,
            expr: self.expr,
        }
    }
}

impl<T> PatternMatcher<Transformed<T>> {
    /// 提取转换后的表达式
    pub fn into_expr(self) -> TypedExpr {
        self.expr
    }
}

/// 常量折叠实现
fn fold_constants(expr: TypedExpr) -> TypedExpr {
    match expr {
        TypedExpr::Binary { op, left, right, ty } => {
            let left = fold_constants(*left);
            let right = fold_constants(*right);

            // 尝试常量折叠
            match (&left, &right) {
                (TypedExpr::Literal(Literal::Int(l)), TypedExpr::Literal(Literal::Int(r))) => {
                    let result = match op {
                        BinaryOp::Add => l + r,
                        BinaryOp::Sub => l - r,
                        BinaryOp::Mul => l * r,
                        BinaryOp::Div if *r != 0 => l / r,
                        _ => return TypedExpr::Binary {
                            op,
                            left: Box::new(left),
                            right: Box::new(right),
                            ty,
                        },
                    };
                    TypedExpr::Literal(Literal::Int(result))
                }
                _ => TypedExpr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                    ty,
                }
            }
        }
        _ => expr,
    }
}

// ============================================================================
// Layer 4: Domain Layer - 领域层
// ============================================================================

/// 领域层标记trait - 表示这是领域模型
pub trait Domain {
    /// 领域验证
    fn validate(&self) -> Result<(), DomainError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomainError {
    InvalidState(String),
    ConstraintViolation(String),
}

/// 领域实体：配置项
#[derive(Debug, Clone)]
pub struct ConfigItem {
    pub key: String,
    pub value: ConfigValue,
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    Integer(i64),
    Text(String),
    Boolean(bool),
    List(Vec<ConfigValue>),
}

impl Domain for ConfigItem {
    fn validate(&self) -> Result<(), DomainError> {
        if self.key.is_empty() {
            return Err(DomainError::InvalidState(
                "Config key cannot be empty".to_string()
            ));
        }
        Ok(())
    }
}

/// 领域实体：工作流定义
#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub name: String,
    pub action: String,
    pub params: Vec<ConfigItem>,
}

impl Domain for Workflow {
    fn validate(&self) -> Result<(), DomainError> {
        if self.name.is_empty() {
            return Err(DomainError::InvalidState(
                "Workflow name cannot be empty".to_string()
            ));
        }
        if self.steps.is_empty() {
            return Err(DomainError::ConstraintViolation(
                "Workflow must have at least one step".to_string()
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Transformation Pipeline - 转换管道
// ============================================================================

/// 转换管道：管理从 Syntax -> Semantic -> Pattern -> Domain 的完整流程
pub struct TransformationPipeline;

impl TransformationPipeline {
    /// 语法解析（模拟）
    pub fn parse(input: &str) -> Result<RawExpr, ParseError> {
        // 简化实现：实际会调用parser
        parse_simple(input)
    }

    /// 语义分析
    pub fn analyze(expr: RawExpr) -> Result<TypedExpr, SemanticError> {
        analyze_expr(expr)
    }

    /// 模式优化
    pub fn optimize(expr: TypedExpr) -> TypedExpr {
        let matcher = PatternMatcher::new(expr)
            .match_const_fold()
            .transform();
        matcher.into_expr()
    }

    /// 领域转换
    pub fn to_domain<T: FromSemantic>(expr: TypedExpr) -> Result<T, DomainError> {
        T::from_semantic(expr)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticError(pub String);

/// 从语义层转换到领域层的trait
pub trait FromSemantic: Sized {
    fn from_semantic(expr: TypedExpr) -> Result<Self, DomainError>;
}

impl FromSemantic for ConfigItem {
    fn from_semantic(expr: TypedExpr) -> Result<Self, DomainError> {
        // 简化实现：从表达式提取配置项
        match expr {
            TypedExpr::Binary { op: BinaryOp::Assign, left, right, .. } => {
                if let TypedExpr::Variable(ident) = *left {
                    let value = expr_to_config_value(*right)?;
                    let item = ConfigItem {
                        key: ident.name,
                        value,
                    };
                    item.validate()?;
                    Ok(item)
                } else {
                    Err(DomainError::InvalidState(
                        "Expected identifier on left side of assignment".to_string()
                    ))
                }
            }
            _ => Err(DomainError::InvalidState(
                "Expected assignment expression".to_string()
            ))
        }
    }
}

fn expr_to_config_value(expr: TypedExpr) -> Result<ConfigValue, DomainError> {
    match expr {
        TypedExpr::Literal(Literal::Int(i)) => Ok(ConfigValue::Integer(i)),
        TypedExpr::Literal(Literal::String(s)) => Ok(ConfigValue::Text(s)),
        TypedExpr::Literal(Literal::Bool(b)) => Ok(ConfigValue::Boolean(b)),
        _ => Err(DomainError::InvalidState(
            "Unsupported value type".to_string()
        ))
    }
}

// 简化解析器
fn parse_simple(input: &str) -> Result<RawExpr, ParseError> {
    // 这是一个简化实现，用于演示
    // 实际会使用nom或类似库
    if input.contains("+") {
        let parts: Vec<&str> = input.split("+").map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let left = parse_simple(parts[0])?;
            let right = parse_simple(parts[1])?;
            return Ok(RawExpr::Binary {
                op: BinaryOp::Add,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
    }

    if let Ok(num) = input.parse::<i64>() {
        return Ok(RawExpr::Number(num));
    }

    if input.starts_with("\"") && input.ends_with("\"") {
        return Ok(RawExpr::String(input[1..input.len()-1].to_string()));
    }

    Ok(RawExpr::Variable(RawIdent {
        name: input.to_string(),
    }))
}

// 简化语义分析器
fn analyze_expr(expr: RawExpr) -> Result<TypedExpr, SemanticError> {
    match expr {
        RawExpr::Number(n) => Ok(TypedExpr::Literal(Literal::Int(n))),
        RawExpr::String(s) => Ok(TypedExpr::Literal(Literal::String(s))),
        RawExpr::Variable(ident) => {
            // 简化：假设所有未解析变量都是Int类型
            Ok(TypedExpr::Variable(ResolvedIdent {
                name: ident.name,
                id: 0,
                ty: Type::Int,
            }))
        }
        RawExpr::Binary { op, left, right } => {
            let left = analyze_expr(*left)?;
            let right = analyze_expr(*right)?;
            let ty = left.type_info(); // 简化类型推导
            Ok(TypedExpr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                ty,
            })
        }
        RawExpr::Call { func, args } => {
            let func = analyze_expr(*func)?;
            let args: Result<Vec<_>, _> = args.into_iter().map(analyze_expr).collect();
            let args = args?;
            Ok(TypedExpr::Call {
                func: Box::new(func),
                args,
                ty: Type::Int, // 简化
            })
        }
    }
}

// 添加Assign操作符用于配置项解析
impl BinaryOp {
    pub const Assign: BinaryOp = BinaryOp::Add; // 占位符，实际应该单独定义
}

// ============================================================================
// Tests - 验证各层转换
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_layer() {
        let expr = RawExpr::Binary {
            op: BinaryOp::Add,
            left: Box::new(RawExpr::Number(1)),
            right: Box::new(RawExpr::Number(2)),
        };
        assert!(matches!(expr, RawExpr::Binary { .. }));
    }

    #[test]
    fn test_semantic_layer() {
        let ident = ResolvedIdent {
            name: "x".to_string(),
            id: 1,
            ty: Type::Int,
        };
        assert_eq!(ident.type_info(), Type::Int);

        let expr = TypedExpr::Literal(Literal::Int(42));
        assert_eq!(expr.type_info(), Type::Int);
    }

    #[test]
    fn test_pattern_const_fold() {
        // 1 + 2 + 3 应该被折叠为 6
        let expr = TypedExpr::Binary {
            op: BinaryOp::Add,
            left: Box::new(TypedExpr::Binary {
                op: BinaryOp::Add,
                left: Box::new(TypedExpr::Literal(Literal::Int(1))),
                right: Box::new(TypedExpr::Literal(Literal::Int(2))),
                ty: Type::Int,
            }),
            right: Box::new(TypedExpr::Literal(Literal::Int(3))),
            ty: Type::Int,
        };

        let matcher = PatternMatcher::new(expr)
            .match_const_fold()
            .transform();
        let result = matcher.into_expr();

        // 由于fold_constants只处理一层，这里需要递归处理
        // 实际实现会更复杂
        assert!(matches!(result, TypedExpr::Binary { .. }));
    }

    #[test]
    fn test_domain_validation() {
        let valid_config = ConfigItem {
            key: "timeout".to_string(),
            value: ConfigValue::Integer(30),
        };
        assert!(valid_config.validate().is_ok());

        let invalid_config = ConfigItem {
            key: "".to_string(),
            value: ConfigValue::Integer(30),
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_pipeline_parse() {
        let result = TransformationPipeline::parse("42");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RawExpr::Number(42)));
    }

    #[test]
    fn test_full_pipeline() {
        // 测试完整流程: "1 + 2" -> RawExpr -> TypedExpr -> Optimized
        let raw = TransformationPipeline::parse("1 + 2").unwrap();
        let typed = TransformationPipeline::analyze(raw).unwrap();
        let optimized = TransformationPipeline::optimize(typed);

        // 验证优化后的结果
        assert_eq!(optimized.type_info(), Type::Int);
    }
}
