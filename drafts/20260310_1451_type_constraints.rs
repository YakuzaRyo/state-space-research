//! 类型约束代码生成 - Type-Constrained Code Generation
//!
//! 本模块实现了基于类型系统的约束解码，用于指导LLM生成类型安全的代码。
//! 核心概念来自论文 "Type-Constrained Code Generation with Language Models" (PLDI 2025)
//!
//! 核心组件:
//! 1. PrefixAutomaton - 前缀自动机，确保每个中间状态都可以完成到类型安全程序
//! 2. TypeReachabilitySearch - 类型可达性搜索，确定表达式可以 inhabits 的类型
//! 3. TypeConstrainedDecoder - 类型约束解码器，集成到LLM生成流程
//! 4. JsonSchemaConverter - JSON Schema到Rust类型的转换器

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

// =============================================================================
// 1. 基础类型系统定义
// =============================================================================

/// 类型系统中的基本类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// 数值类型
    Number,
    /// 字符串类型
    String,
    /// 布尔类型
    Boolean,
    /// 函数类型: 参数类型 -> 返回类型
    Function(Vec<Type>, Box<Type>),
    /// 数组类型
    Array(Box<Type>),
    /// 对象/结构体类型
    Object(String, Vec<(String, Type)>),
    /// 类型变量（用于泛型）
    TypeVar(String),
    /// 未知/待推断类型
    Unknown,
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::String => write!(f, "string"),
            Type::Boolean => write!(f, "boolean"),
            Type::Function(params, ret) => {
                let params_str = params.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({}) => {}", params_str, ret)
            }
            Type::Array(elem) => write!(f, "{}[]", elem),
            Type::Object(name, _) => write!(f, "{}", name),
            Type::TypeVar(name) => write!(f, "'{}", name),
            Type::Unknown => write!(f, "unknown"),
        }
    }
}

impl Type {
    /// 检查类型是否匹配（支持子类型关系）
    pub fn matches(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (a, b) if a == b => true,
            // 协变数组
            (Type::Array(a), Type::Array(b)) => a.matches(b),
            // 函数子类型: 参数逆变，返回值协变
            (Type::Function(p1, r1), Type::Function(p2, r2)) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(a, b)| b.matches(a))
                    && r1.matches(r2)
            }
            _ => false,
        }
    }

    /// 获取类型的成员
    pub fn get_member(&self, name: &str) -> Option<Type> {
        match self {
            Type::String => match name {
                "length" => Some(Type::Number),
                "toString" => Some(Type::Function(vec![], Box::new(Type::String))),
                "charAt" => Some(Type::Function(vec![Type::Number], Box::new(Type::String))),
                _ => None,
            },
            Type::Number => match name {
                "toString" => Some(Type::Function(vec![], Box::new(Type::String))),
                "toFixed" => Some(Type::Function(vec![Type::Number], Box::new(Type::String))),
                "isFinite" => Some(Type::Function(vec![], Box::new(Type::Boolean))),
                _ => None,
            },
            Type::Array(elem) => match name {
                "length" => Some(Type::Number),
                "push" => Some(Type::Function(vec![*elem.clone()], Box::new(Type::Number))),
                "pop" => Some(Type::Function(vec![], elem.clone())),
                "map" => Some(Type::Function(
                    vec![Type::Function(vec![*elem.clone()], Box::new(Type::TypeVar("U".to_string())))],
                    Box::new(Type::Array(Box::new(Type::TypeVar("U".to_string())))),
                )),
                _ => None,
            },
            Type::Object(_, fields) => {
                fields.iter().find(|(n, _)| n == name).map(|(_, t)| t.clone())
            }
            _ => None,
        }
    }
}

// =============================================================================
// 2. 类型环境
// =============================================================================

/// 类型环境，存储变量名到类型的映射
#[derive(Debug, Clone, Default)]
pub struct TypeEnvironment {
    bindings: HashMap<String, Type>,
    parent: Option<Arc<TypeEnvironment>>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Arc<TypeEnvironment>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(parent),
        }
    }

    /// 添加变量绑定
    pub fn bind(&mut self, name: String, ty: Type) {
        self.bindings.insert(name, ty);
    }

    /// 查找变量类型
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.bindings.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.lookup(name))
        })
    }

    /// 获取所有绑定
    pub fn bindings(&self) -> &HashMap<String, Type> {
        &self.bindings
    }
}

// =============================================================================
// 3. 前缀自动机 (Prefix Automaton)
// =============================================================================

/// 自动机状态ID
pub type StateId = usize;

/// 前缀自动机状态
#[derive(Debug, Clone)]
pub struct AutomatonState {
    pub id: StateId,
    pub is_accepting: bool,
    /// 当前解析的表达式类型（如果有）
    pub expr_type: Option<Type>,
    /// 状态元数据
    pub metadata: StateMetadata,
}

#[derive(Debug, Clone, Default)]
pub struct StateMetadata {
    /// 当前正在解析的语法类别
    pub category: SyntaxCategory,
    /// 类型环境快照
    pub type_env: Option<TypeEnvironment>,
    /// 期望的返回类型（用于函数体）
    pub expected_return: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxCategory {
    Program,
    Statement,
    Expression,
    TypeAnnotation,
    Identifier,
    Literal,
    FunctionCall,
    MemberAccess,
    BinaryOp,
    Block,
    Unknown,
}

impl Default for SyntaxCategory {
    fn default() -> Self {
        SyntaxCategory::Unknown
    }
}

/// 前缀自动机
///
/// 核心性质（Prefix Property）: 从每个可达状态都存在一条路径到达接受状态
/// 这确保了部分生成的代码总是可以完成到类型安全的程序
pub struct PrefixAutomaton {
    states: HashMap<StateId, AutomatonState>,
    transitions: HashMap<StateId, Vec<(char, StateId)>>,
    initial_states: HashSet<StateId>,
    next_state_id: StateId,
}

impl PrefixAutomaton {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            transitions: HashMap::new(),
            initial_states: HashSet::new(),
            next_state_id: 0,
        }
    }

    /// 创建新状态
    pub fn create_state(&mut self, is_accepting: bool, metadata: StateMetadata) -> StateId {
        let id = self.next_state_id;
        self.next_state_id += 1;

        let state = AutomatonState {
            id,
            is_accepting,
            expr_type: None,
            metadata,
        };

        self.states.insert(id, state);
        self.transitions.insert(id, Vec::new());
        id
    }

    /// 添加转移
    pub fn add_transition(&mut self, from: StateId, ch: char, to: StateId) {
        if let Some(trans) = self.transitions.get_mut(&from) {
            trans.push((ch, to));
        }
    }

    /// 添加初始状态
    pub fn add_initial_state(&mut self, state_id: StateId) {
        self.initial_states.insert(state_id);
    }

    /// 获取状态
    pub fn get_state(&self, id: StateId) -> Option<&AutomatonState> {
        self.states.get(&id)
    }

    /// 获取转移
    pub fn get_transitions(&self, from: StateId) -> &[(char, StateId)] {
        self.transitions.get(&from).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 执行状态转移
    pub fn transition(&self, states: &HashSet<StateId>, ch: char) -> HashSet<StateId> {
        let mut result = HashSet::new();
        for state_id in states {
            if let Some(trans) = self.transitions.get(state_id) {
                for (c, next_id) in trans {
                    if *c == ch {
                        result.insert(*next_id);
                    }
                }
            }
        }
        result
    }

    /// 检查字符串是否为有效前缀
    pub fn is_valid_prefix(&self, input: &str) -> bool {
        let mut current_states = self.initial_states.clone();

        for ch in input.chars() {
            current_states = self.transition(&current_states, ch);
            if current_states.is_empty() {
                return false;
            }
        }

        // 前缀性质: 所有可达状态都可以到达接受状态
        current_states.iter().all(|state_id| {
            self.can_reach_accepting(*state_id)
        })
    }

    /// 检查状态是否可以到达接受状态
    fn can_reach_accepting(&self, start: StateId) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(state_id) = queue.pop_front() {
            if visited.contains(&state_id) {
                continue;
            }
            visited.insert(state_id);

            if let Some(state) = self.states.get(&state_id) {
                if state.is_accepting {
                    return true;
                }
            }

            if let Some(trans) = self.transitions.get(&state_id) {
                for (_, next_id) in trans {
                    queue.push_back(*next_id);
                }
            }
        }

        false
    }

    /// 获取当前状态下允许的下一个字符
    pub fn allowed_next_chars(&self, states: &HashSet<StateId>) -> HashSet<char> {
        let mut result = HashSet::new();
        for state_id in states {
            if let Some(trans) = self.transitions.get(state_id) {
                for (ch, _) in trans {
                    result.insert(*ch);
                }
            }
        }
        result
    }
}

// =============================================================================
// 4. 类型可达性搜索 (Type Reachability Search)
// =============================================================================

/// 类型搜索图节点
#[derive(Debug, Clone)]
struct TypeNode {
    ty: Type,
    /// 到达此节点的操作序列
    path: Vec<Operation>,
}

/// 类型操作
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// 成员访问
    MemberAccess(String),
    /// 函数调用
    FunctionCall(Vec<Type>),
    /// 二元操作
    BinaryOp(String, Box<Type>),
    /// 类型转换
    TypeCast(Box<Type>),
}

/// 类型可达性搜索器
///
/// 解决类型 inhabitation 问题：给定起始类型和目标类型，
/// 找到一系列操作将起始类型转换为目标类型
pub struct TypeReachabilitySearch {
    /// 类型图边: 类型 -> [(操作, 结果类型)]
    type_graph: HashMap<Type, Vec<(Operation, Type)>>,
    /// 最大搜索深度
    max_depth: usize,
}

impl TypeReachabilitySearch {
    pub fn new(max_depth: usize) -> Self {
        Self {
            type_graph: HashMap::new(),
            max_depth,
        }
    }

    /// 添加类型图边
    pub fn add_edge(&mut self, from: Type, op: Operation, to: Type) {
        self.type_graph.entry(from).or_default().push((op, to));
    }

    /// 从类型环境构建类型图
    pub fn build_from_env(&mut self, env: &TypeEnvironment) {
        // 为每种基本类型添加成员访问边
        for (name, ty) in env.bindings() {
            self.build_type_edges(ty.clone());
        }
    }

    /// 为特定类型构建边
    fn build_type_edges(&mut self, ty: Type) {
        match &ty {
            Type::String => {
                self.add_edge(
                    ty.clone(),
                    Operation::MemberAccess("length".to_string()),
                    Type::Number,
                );
                self.add_edge(
                    ty.clone(),
                    Operation::MemberAccess("toString".to_string()),
                    Type::Function(vec![], Box::new(Type::String)),
                );
            }
            Type::Number => {
                self.add_edge(
                    ty.clone(),
                    Operation::MemberAccess("toString".to_string()),
                    Type::Function(vec![], Box::new(Type::String)),
                );
                self.add_edge(
                    ty.clone(),
                    Operation::BinaryOp("+".to_string(), Box::new(Type::Number)),
                    Type::Number,
                );
            }
            Type::Array(elem) => {
                self.add_edge(ty.clone(), Operation::MemberAccess("length".to_string()), Type::Number);
                self.add_edge(
                    ty.clone(),
                    Operation::MemberAccess("pop".to_string()),
                    *elem.clone(),
                );
            }
            _ => {}
        }
    }

    /// 搜索从起始类型到目标类型的路径
    pub fn search(&self, start: &Type, target: &Type) -> Option<Vec<Operation>> {
        if start.matches(target) {
            return Some(vec![]);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(TypeNode {
            ty: start.clone(),
            path: vec![],
        });

        while let Some(node) = queue.pop_front() {
            if node.path.len() >= self.max_depth {
                continue;
            }

            if node.ty.matches(target) {
                return Some(node.path);
            }

            if visited.contains(&node.ty) {
                continue;
            }
            visited.insert(node.ty.clone());

            if let Some(edges) = self.type_graph.get(&node.ty) {
                for (op, next_ty) in edges {
                    let mut new_path = node.path.clone();
                    new_path.push(op.clone());
                    queue.push_back(TypeNode {
                        ty: next_ty.clone(),
                        path: new_path,
                    });
                }
            }
        }

        None
    }

    /// 检查类型是否可达
    pub fn is_reachable(&self, start: &Type, target: &Type) -> bool {
        self.search(start, target).is_some()
    }
}

// =============================================================================
// 5. 类型约束解码器
// =============================================================================

/// 解码约束
#[derive(Debug, Clone)]
pub enum DecodingConstraint {
    /// 无约束
    None,
    /// 类型约束
    TypeConstraint(Type),
    /// 语法类别约束
    SyntaxConstraint(SyntaxCategory),
    /// 复合约束
    And(Box<DecodingConstraint>, Box<DecodingConstraint>),
}

/// 类型约束解码器
///
/// 将类型约束转换为token级别的掩码，用于指导LLM生成
pub struct TypeConstrainedDecoder {
    automaton: PrefixAutomaton,
    type_search: TypeReachabilitySearch,
    type_env: TypeEnvironment,
}

impl TypeConstrainedDecoder {
    pub fn new(max_search_depth: usize) -> Self {
        Self {
            automaton: PrefixAutomaton::new(),
            type_search: TypeReachabilitySearch::new(max_search_depth),
            type_env: TypeEnvironment::new(),
        }
    }

    /// 初始化自动机
    pub fn initialize_automaton(&mut self) {
        self.build_literal_automaton();
        self.build_identifier_automaton();
        self.build_expression_automaton();
        self.build_statement_automaton();
    }

    /// 构建字面量自动机
    fn build_literal_automaton(&mut self) {
        // 数字字面量: [0-9]+
        let num_start = self.automaton.create_state(false, StateMetadata {
            category: SyntaxCategory::Literal,
            ..Default::default()
        });
        let num_accept = self.automaton.create_state(true, StateMetadata {
            category: SyntaxCategory::Literal,
            ..Default::default()
        });

        for ch in '0'..='9' {
            self.automaton.add_transition(num_start, ch, num_accept);
            self.automaton.add_transition(num_accept, ch, num_accept);
        }

        self.automaton.add_initial_state(num_start);

        // 字符串字面量: "..."
        let str_start = self.automaton.create_state(false, StateMetadata {
            category: SyntaxCategory::Literal,
            ..Default::default()
        });
        let str_content = self.automaton.create_state(false, StateMetadata {
            category: SyntaxCategory::Literal,
            ..Default::default()
        });
        let str_accept = self.automaton.create_state(true, StateMetadata {
            category: SyntaxCategory::Literal,
            ..Default::default()
        });

        self.automaton.add_transition(str_start, '"', str_content);
        // 简化：允许任何字符（除了"）
        for ch in ('a'..='z').chain('A'..='Z').chain('0'..='9') {
            self.automaton.add_transition(str_content, ch, str_content);
        }
        self.automaton.add_transition(str_content, '"', str_accept);

        self.automaton.add_initial_state(str_start);
    }

    /// 构建标识符自动机
    fn build_identifier_automaton(&mut self) {
        let start = self.automaton.create_state(false, StateMetadata {
            category: SyntaxCategory::Identifier,
            ..Default::default()
        });
        let accept = self.automaton.create_state(true, StateMetadata {
            category: SyntaxCategory::Identifier,
            ..Default::default()
        });

        // 首字符必须是字母或下划线
        for ch in ('a'..='z').chain('A'..='Z').chain(std::iter::once('_')) {
            self.automaton.add_transition(start, ch, accept);
        }

        // 后续字符可以是字母、数字或下划线
        for ch in ('a'..='z').chain('A'..='Z').chain('0'..='9').chain(std::iter::once('_')) {
            self.automaton.add_transition(accept, ch, accept);
        }

        self.automaton.add_initial_state(start);
    }

    /// 构建表达式自动机（简化版）
    fn build_expression_automaton(&mut self) {
        // 这里构建一个简化的表达式自动机
        // 实际实现需要处理更复杂的递归结构

        let expr_start = self.automaton.create_state(false, StateMetadata {
            category: SyntaxCategory::Expression,
            ..Default::default()
        });

        // 表达式可以是标识符或字面量
        // 这里我们复用之前定义的初始状态
        self.automaton.add_initial_state(expr_start);
    }

    /// 构建语句自动机
    fn build_statement_automaton(&mut self) {
        // let 语句: let x: type = expr;
        let stmt_start = self.automaton.create_state(false, StateMetadata {
            category: SyntaxCategory::Statement,
            ..Default::default()
        });

        self.automaton.add_initial_state(stmt_start);
    }

    /// 验证部分程序是否类型安全
    pub fn validate_partial(&self, partial_code: &str) -> bool {
        self.automaton.is_valid_prefix(partial_code)
    }

    /// 获取下一个允许的token模式
    pub fn get_allowed_patterns(&self, partial_code: &str) -> Vec<String> {
        // 简化实现：返回可能的完成模式
        let mut patterns = vec![];

        // 基于类型搜索添加可能的成员访问
        for (name, ty) in self.type_env.bindings() {
            if let Some(member_ty) = ty.get_member("toString") {
                patterns.push(format!("{}.toString()", name));
            }
        }

        patterns
    }

    /// 更新类型环境
    pub fn update_type_env(&mut self, env: TypeEnvironment) {
        self.type_env = env;
        self.type_search.build_from_env(&self.type_env);
    }
}

// =============================================================================
// 6. JSON Schema 到 Rust 类型转换器
// =============================================================================

/// JSON Schema 类型
#[derive(Debug, Clone)]
pub enum JsonSchema {
    Object {
        properties: HashMap<String, Box<JsonSchema>>,
        required: Vec<String>,
    },
    Array {
        items: Box<JsonSchema>,
    },
    String {
        min_length: Option<usize>,
        max_length: Option<usize>,
        pattern: Option<String>,
    },
    Number {
        minimum: Option<f64>,
        maximum: Option<f64>,
    },
    Integer,
    Boolean,
    Null,
    Ref(String),
    OneOf(Vec<JsonSchema>),
    AnyOf(Vec<JsonSchema>),
    AllOf(Vec<JsonSchema>),
}

/// JSON Schema 转换器
pub struct JsonSchemaConverter;

impl JsonSchemaConverter {
    pub fn new() -> Self {
        Self
    }

    /// 将 JSON Schema 转换为 Rust 类型
    pub fn convert(&self, schema: &JsonSchema, type_name: &str) -> Type {
        match schema {
            JsonSchema::Object { properties, .. } => {
                let fields: Vec<(String, Type)> = properties
                    .iter()
                    .map(|(name, prop_schema)| {
                        let ty = self.convert(prop_schema, &format!("{}_{}", type_name, name));
                        (name.clone(), ty)
                    })
                    .collect();
                Type::Object(type_name.to_string(), fields)
            }
            JsonSchema::Array { items } => {
                let elem_type = self.convert(items, &format!("{}_Item", type_name));
                Type::Array(Box::new(elem_type))
            }
            JsonSchema::String { .. } => Type::String,
            JsonSchema::Number { .. } | JsonSchema::Integer => Type::Number,
            JsonSchema::Boolean => Type::Boolean,
            JsonSchema::Null => Type::Unknown,
            JsonSchema::Ref(name) => Type::Object(name.clone(), vec![]),
            JsonSchema::OneOf(options) => {
                // 简化为第一个选项的类型
                options.first()
                    .map(|s| self.convert(s, type_name))
                    .unwrap_or(Type::Unknown)
            }
            JsonSchema::AnyOf(options) => {
                options.first()
                    .map(|s| self.convert(s, type_name))
                    .unwrap_or(Type::Unknown)
            }
            JsonSchema::AllOf(schemas) => {
                // 合并所有属性
                let mut all_fields = vec![];
                for (i, s) in schemas.iter().enumerate() {
                    if let Type::Object(_, fields) = self.convert(s, &format!("{}_Part{}", type_name, i)) {
                        all_fields.extend(fields);
                    }
                }
                Type::Object(type_name.to_string(), all_fields)
            }
        }
    }

    /// 生成 Rust 结构体代码
    pub fn generate_rust_struct(&self, schema: &JsonSchema, name: &str) -> String {
        let ty = self.convert(schema, name);
        self.type_to_rust_code(&ty, name)
    }

    fn type_to_rust_code(&self, ty: &Type, name: &str) -> String {
        match ty {
            Type::Object(type_name, fields) => {
                let fields_code: Vec<String> = fields
                    .iter()
                    .map(|(field_name, field_ty)| {
                        let rust_ty = self.rust_type_name(field_ty);
                        format!("    pub {}: {},", field_name, rust_ty)
                    })
                    .collect();

                format!(
                    "#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]\npub struct {} {{\n{}\n}}",
                    type_name,
                    fields_code.join("\n")
                )
            }
            _ => format!("pub type {} = {};", name, self.rust_type_name(ty)),
        }
    }

    fn rust_type_name(&self, ty: &Type) -> String {
        match ty {
            Type::Number => "f64".to_string(),
            Type::String => "String".to_string(),
            Type::Boolean => "bool".to_string(),
            Type::Array(elem) => format!("Vec<{}>", self.rust_type_name(elem)),
            Type::Object(name, _) => name.clone(),
            Type::Function(params, ret) => {
                let params_str = params.iter()
                    .map(|p| self.rust_type_name(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Box<dyn Fn({}) -> {}>", params_str, self.rust_type_name(ret))
            }
            Type::TypeVar(name) => name.clone(),
            Type::Unknown => "serde_json::Value".to_string(),
        }
    }
}

// =============================================================================
// 7. 类型安全的 LLM 输出解析器
// =============================================================================

/// 解析结果
#[derive(Debug, Clone)]
pub enum ParseResult<T> {
    Success(T),
    Partial { valid_prefix: String, remaining: String },
    Error(String),
}

/// 类型安全的输出解析器
pub struct TypeSafeParser<T> {
    decoder: TypeConstrainedDecoder,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TypeSafeParser<T> {
    pub fn new() -> Self {
        let mut decoder = TypeConstrainedDecoder::new(10);
        decoder.initialize_automaton();

        Self {
            decoder,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 增量解析
    pub fn parse_incremental(&self, partial: &str) -> ParseResult<String> {
        if self.decoder.validate_partial(partial) {
            ParseResult::Success(partial.to_string())
        } else {
            // 找到最长有效前缀
            let mut valid_len = 0;
            for i in (0..=partial.len()).rev() {
                if let Some(prefix) = partial.get(0..i) {
                    if self.decoder.validate_partial(prefix) {
                        valid_len = i;
                        break;
                    }
                }
            }

            let valid_prefix = partial[0..valid_len].to_string();
            let remaining = partial[valid_len..].to_string();

            ParseResult::Partial { valid_prefix, remaining }
        }
    }
}

// =============================================================================
// 8. 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_matching() {
        assert!(Type::Number.matches(&Type::Number));
        assert!(Type::String.matches(&Type::String));
        assert!(!Type::Number.matches(&Type::String));

        // 数组协变
        let arr_num = Type::Array(Box::new(Type::Number));
        let arr_num2 = Type::Array(Box::new(Type::Number));
        assert!(arr_num.matches(&arr_num2));

        // Unknown 匹配任何类型
        assert!(Type::Unknown.matches(&Type::Number));
        assert!(Type::Number.matches(&Type::Unknown));
    }

    #[test]
    fn test_type_environment() {
        let mut env = TypeEnvironment::new();
        env.bind("x".to_string(), Type::Number);
        env.bind("y".to_string(), Type::String);

        assert_eq!(env.lookup("x"), Some(&Type::Number));
        assert_eq!(env.lookup("y"), Some(&Type::String));
        assert_eq!(env.lookup("z"), None);
    }

    #[test]
    fn test_prefix_automaton() {
        let mut automaton = PrefixAutomaton::new();

        // 构建简单的 "abc" 自动机
        let s0 = automaton.create_state(false, StateMetadata::default());
        let s1 = automaton.create_state(false, StateMetadata::default());
        let s2 = automaton.create_state(false, StateMetadata::default());
        let s3 = automaton.create_state(true, StateMetadata::default());

        automaton.add_transition(s0, 'a', s1);
        automaton.add_transition(s1, 'b', s2);
        automaton.add_transition(s2, 'c', s3);

        automaton.add_initial_state(s0);

        // 测试前缀性质
        assert!(automaton.is_valid_prefix("a"));
        assert!(automaton.is_valid_prefix("ab"));
        assert!(automaton.is_valid_prefix("abc"));
        assert!(!automaton.is_valid_prefix("d"));
        assert!(!automaton.is_valid_prefix("abx"));
    }

    #[test]
    fn test_type_reachability() {
        let mut search = TypeReachabilitySearch::new(5);

        // 添加边: number -> toString() -> () => string
        search.add_edge(
            Type::Number,
            Operation::MemberAccess("toString".to_string()),
            Type::Function(vec![], Box::new(Type::String)),
        );

        // 搜索从 number 到 string 的路径
        let path = search.search(&Type::Number, &Type::String);
        assert!(path.is_some());

        // 直接搜索应该失败
        let direct = search.search(&Type::Number, &Type::Boolean);
        assert!(direct.is_none());
    }

    #[test]
    fn test_json_schema_converter() {
        let converter = JsonSchemaConverter::new();

        let schema = JsonSchema::Object {
            properties: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), Box::new(JsonSchema::String {
                    min_length: None,
                    max_length: None,
                    pattern: None,
                }));
                map.insert("age".to_string(), Box::new(JsonSchema::Integer));
                map
            },
            required: vec!["name".to_string(), "age".to_string()],
        };

        let rust_code = converter.generate_rust_struct(&schema, "Person");
        assert!(rust_code.contains("pub struct Person"));
        assert!(rust_code.contains("pub name: String"));
        assert!(rust_code.contains("pub age: f64"));
    }

    #[test]
    fn test_type_constrained_decoder() {
        let mut decoder = TypeConstrainedDecoder::new(10);
        decoder.initialize_automaton();

        // 测试数字字面量
        assert!(decoder.validate_partial("123"));

        // 测试标识符
        assert!(decoder.validate_partial("abc"));
        assert!(decoder.validate_partial("_test"));

        // 无效输入
        assert!(!decoder.validate_partial("123abc")); // 数字后不能直接跟字母
    }

    #[test]
    fn test_member_access() {
        let num = Type::Number;
        assert_eq!(num.get_member("toString"), Some(Type::Function(vec![], Box::new(Type::String))));
        assert_eq!(num.get_member("length"), None);

        let arr = Type::Array(Box::new(Type::Number));
        assert_eq!(arr.get_member("length"), Some(Type::Number));
        assert_eq!(arr.get_member("pop"), Some(Type::Number));
    }
}

// =============================================================================
// 9. 示例用法
// =============================================================================

/// 示例：使用类型约束解码器生成代码
pub fn example_usage() {
    // 1. 创建解码器
    let mut decoder = TypeConstrainedDecoder::new(10);
    decoder.initialize_automaton();

    // 2. 设置类型环境
    let mut env = TypeEnvironment::new();
    env.bind("num".to_string(), Type::Number);
    env.bind("str".to_string(), Type::String);
    decoder.update_type_env(env);

    // 3. 验证部分代码
    let partial = "num";
    if decoder.validate_partial(partial) {
        println!("'{}' 是有效的类型安全前缀", partial);

        // 4. 获取可能的完成
        let patterns = decoder.get_allowed_patterns(partial);
        println!("可能的完成: {:?}", patterns);
    }

    // 5. JSON Schema 转换示例
    let converter = JsonSchemaConverter::new();
    let schema = JsonSchema::Object {
        properties: {
            let mut map = HashMap::new();
            map.insert("id".to_string(), Box::new(JsonSchema::Integer));
            map.insert("title".to_string(), Box::new(JsonSchema::String {
                min_length: Some(1),
                max_length: Some(100),
                pattern: None,
            }));
            map
        },
        required: vec!["id".to_string(), "title".to_string()],
    };

    let rust_code = converter.generate_rust_struct(&schema, "Task");
    println!("\n生成的 Rust 代码:\n{}", rust_code);
}

fn main() {
    example_usage();
}
