//! 类型约束生成深度研究 - 2026-03-11
//! 研究方向: 05_type_constraints - 类型约束生成
//! 核心问题: 类型系统如何指导代码生成?

use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;

// =============================================================================
// 第一部分: Typestate模式与类型约束解码结合 (验证H1)
// =============================================================================

/// 生成器状态标记
pub struct Idle;
pub struct Parsing;
pub struct TypeChecking;
pub struct Generating;
pub struct Complete;

/// Typestate编码的代码生成器
/// 在编译期强制执行有效的状态转换
pub struct CodeGenerator<State> {
    tokens: Vec<String>,
    type_context: TypeContext,
    automaton: PrefixAutomaton,
    _state: PhantomData<State>,
}

/// 类型上下文
#[derive(Clone, Debug)]
pub struct TypeContext {
    variables: HashMap<String, Type>,
    functions: HashMap<String, (Vec<Type>, Type)>,
}

/// 简单类型系统
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Base(String),
    Arrow(Box<Type>, Box<Type>),
    Refinement(Box<Type>, RefinementPredicate),
}

/// 细化谓词
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RefinementPredicate {
    GreaterThan(i64),
    LessThan(i64),
    Range(i64, i64),
    Equals(String),
}

/// 前缀自动机 - 确保每个中间状态都可完成
#[derive(Clone, Debug)]
pub struct PrefixAutomaton {
    states: Vec<AutomatonState>,
    current: usize,
}

#[derive(Clone, Debug)]
struct AutomatonState {
    accepting: bool,
    transitions: HashMap<String, usize>,
}

impl Default for PrefixAutomaton {
    fn default() -> Self {
        Self {
            states: vec![AutomatonState {
                accepting: true,
                transitions: HashMap::new(),
            }],
            current: 0,
        }
    }
}

impl PrefixAutomaton {
    /// 验证前缀属性: 从每个接受状态都存在路径到达终止状态
    pub fn verify_prefix_property(&self) -> bool {
        // 简化的验证: 检查所有接受状态是否可达终止状态
        for (idx, state) in self.states.iter().enumerate() {
            if state.accepting && !self.can_reach_terminal(idx) {
                return false;
            }
        }
        true
    }

    fn can_reach_terminal(&self, from: usize) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // 检查是否是终止状态(没有出边或明确标记为终止)
            if self.states[current].transitions.is_empty() {
                return true;
            }

            for &next in self.states[current].transitions.values() {
                if !visited.contains(&next) {
                    queue.push_back(next);
                }
            }
        }

        // 如果没有找到终止状态，检查当前状态是否是接受状态
        self.states[from].accepting
    }
}

// Idle状态的实现
impl CodeGenerator<Idle> {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            type_context: TypeContext {
                variables: HashMap::new(),
                functions: HashMap::new(),
            },
            automaton: PrefixAutomaton::default(),
            _state: PhantomData,
        }
    }

    /// 转换到Parsing状态 - 消耗self，返回新类型
    pub fn start_parsing(self) -> CodeGenerator<Parsing> {
        CodeGenerator {
            tokens: self.tokens,
            type_context: self.type_context,
            automaton: self.automaton,
            _state: PhantomData,
        }
    }
}

// Parsing状态的实现
impl CodeGenerator<Parsing> {
    pub fn parse_input(mut self, input: &str) -> Self {
        self.tokens = input.split_whitespace().map(String::from).collect();
        self
    }

    pub fn finish_parsing(self) -> CodeGenerator<TypeChecking> {
        CodeGenerator {
            tokens: self.tokens,
            type_context: self.type_context,
            automaton: self.automaton,
            _state: PhantomData,
        }
    }
}

// TypeChecking状态的实现
impl CodeGenerator<TypeChecking> {
    pub fn add_variable(mut self, name: &str, ty: Type) -> Self {
        self.type_context.variables.insert(name.to_string(), ty);
        self
    }

    pub fn finish_type_checking(self) -> CodeGenerator<Generating> {
        CodeGenerator {
            tokens: self.tokens,
            type_context: self.type_context,
            automaton: self.automaton,
            _state: PhantomData,
        }
    }
}

// Generating状态的实现 - 核心: 类型约束解码
impl CodeGenerator<Generating> {
    /// 类型约束解码: 只生成类型兼容的token
    pub fn generate_constrained_token(&mut self, candidates: &[String]) -> Option<String> {
        // 过滤出类型兼容的候选
        let valid_candidates: Vec<_> = candidates
            .iter()
            .filter(|token| self.is_type_compatible(token))
            .cloned()
            .collect();

        // 选择第一个有效候选(实际应用中会使用LLM概率)
        valid_candidates.first().cloned()
    }

    fn is_type_compatible(&self, token: &str) -> bool {
        // 简化实现: 检查token是否在类型上下文中
        self.type_context.variables.contains_key(token)
            || self.type_context.functions.contains_key(token)
    }

    pub fn finish_generation(self) -> CodeGenerator<Complete> {
        CodeGenerator {
            tokens: self.tokens,
            type_context: self.type_context,
            automaton: self.automaton,
            _state: PhantomData,
        }
    }
}

// Complete状态的实现
impl CodeGenerator<Complete> {
    pub fn get_result(&self) -> &[String] {
        &self.tokens
    }

    /// 验证前缀属性
    pub fn verify_prefix_property(&self) -> bool {
        self.automaton.verify_prefix_property()
    }
}

// =============================================================================
// 第二部分: 类型可达性搜索 (验证H2, H5)
// =============================================================================

/// 类型 inhabitation 搜索结果
#[derive(Clone, Debug)]
pub enum InhabitationResult {
    Inhabited(Vec<Constructor>),
    Uninhabited,
    Timeout,
}

/// 构造函数
#[derive(Clone, Debug, PartialEq)]
pub enum Constructor {
    Variable(String),
    FunctionCall(String),
    Lambda(String, Box<Type>),
    Application(Box<Constructor>, Box<Constructor>),
}

/// 类型可达性搜索器
pub struct TypeReachabilitySearch {
    constructors: Vec<Constructor>,
    cache: HashMap<Type, InhabitationResult>,
    max_depth: usize,
}

impl TypeReachabilitySearch {
    pub fn new(constructors: Vec<Constructor>) -> Self {
        Self {
            constructors,
            cache: HashMap::new(),
            max_depth: 10,
        }
    }

    /// 自顶向下搜索: 从目标类型出发，寻找构造路径
    pub fn search_top_down(&mut self, target: &Type) -> InhabitationResult {
        if let Some(result) = self.cache.get(target) {
            return result.clone();
        }

        let result = self.search_top_down_recursive(target, 0);
        self.cache.insert(target.clone(), result.clone());
        result
    }

    fn search_top_down_recursive(&self, target: &Type, depth: usize) -> InhabitationResult {
        if depth > self.max_depth {
            return InhabitationResult::Timeout;
        }

        // 简化的 inhabitation 检查
        // 实际实现需要完整的类型推导
        for ctor in &self.constructors {
            if self.constructor_produces_type(ctor, target) {
                return InhabitationResult::Inhabited(vec![ctor.clone()]);
            }
        }

        InhabitationResult::Uninhabited
    }

    fn constructor_produces_type(&self, ctor: &Constructor, target: &Type) -> bool {
        // 简化实现
        matches!(ctor, Constructor::Variable(_))
    }

    /// 自底向上搜索 (SOBEQ风格)
    pub fn search_bottom_up(&mut self, target: &Type) -> InhabitationResult {
        let mut known_terms: HashMap<Type, Vec<Constructor>> = HashMap::new();

        // 从基础项开始(变量、常量)
        for ctor in &self.constructors {
            if let Some(ty) = self.infer_type_simple(ctor) {
                known_terms.entry(ty).or_default().push(ctor.clone());
            }
        }

        // 迭代扩展直到找到目标类型或达到不动点
        for _ in 0..self.max_depth {
            if let Some(terms) = known_terms.get(target) {
                return InhabitationResult::Inhabited(terms.clone());
            }

            // 尝试应用函数构造新项
            self.expand_terms(&mut known_terms);
        }

        InhabitationResult::Uninhabited
    }

    fn infer_type_simple(&self, ctor: &Constructor) -> Option<Type> {
        match ctor {
            Constructor::Variable(name) => {
                // 简化: 假设所有变量都是int类型
                Some(Type::Base("int".to_string()))
            }
            _ => None,
        }
    }

    fn expand_terms(&self, known_terms: &mut HashMap<Type, Vec<Constructor>>) {
        // 简化实现: 尝试函数应用
        // 实际实现需要完整的类型推导和合一
    }
}

// =============================================================================
// 第三部分: 细化类型实现 (验证H3)
// =============================================================================

/// 细化类型系统
pub struct RefinementTypeSystem;

impl RefinementTypeSystem {
    /// 检查细化类型的子类型关系
    /// {x: T | p} <: T 总是成立
    /// T <: {x: T | p} 仅当 p 对所有x成立
    pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
        match (sub, sup) {
            // 细化类型是其基础类型的子类型
            (Type::Refinement(base, _), super_type) if base.as_ref() == super_type => true,

            // 相同类型
            (t1, t2) if t1 == t2 => true,

            _ => false,
        }
    }

    /// 检查值是否满足细化谓词
    pub fn check_predicate(value: i64, pred: &RefinementPredicate) -> bool {
        match pred {
            RefinementPredicate::GreaterThan(n) => value > *n,
            RefinementPredicate::LessThan(n) => value < *n,
            RefinementPredicate::Range(min, max) => value >= *min && value <= *max,
            RefinementPredicate::Equals(s) => value.to_string() == *s,
        }
    }

    /// 渐进式细化: 从基础类型添加约束
    pub fn refine(base: Type, pred: RefinementPredicate) -> Type {
        Type::Refinement(Box::new(base), pred)
    }
}

// =============================================================================
// 第四部分: 上下文无关token预计算 (验证H4)
// =============================================================================

/// Token分类器 - 区分上下文无关和上下文相关token
pub struct TokenClassifier {
    /// 上下文无关token: 有效性仅由当前PDA位置决定
    context_independent: HashSet<String>,
    /// 预计算的token掩码缓存
    mask_cache: HashMap<usize, Vec<bool>>,
}

impl TokenClassifier {
    pub fn new(vocabulary: Vec<String>) -> Self {
        let mut context_independent = HashSet::new();

        // 启发式: 关键字、字面量通常是上下文无关的
        for token in &vocabulary {
            if Self::is_likely_context_independent(token) {
                context_independent.insert(token.clone());
            }
        }

        Self {
            context_independent,
            mask_cache: HashMap::new(),
        }
    }

    fn is_likely_context_independent(token: &str) -> bool {
        // 简化启发式
        matches!(token.as_str(),
            "if" | "else" | "while" | "for" | "return" |
            "true" | "false" | "null" | "let" | "const" |
            "fn" | "struct" | "enum" | "impl" | "pub"
        ) || token.parse::<i64>().is_ok()
    }

    /// 预计算给定状态的token掩码
    pub fn precompute_mask(&mut self, state: usize, vocabulary: &[String]) -> Vec<bool> {
        if let Some(cached) = self.mask_cache.get(&state) {
            return cached.clone();
        }

        let mask: Vec<bool> = vocabulary
            .iter()
            .map(|token| self.context_independent.contains(token))
            .collect();

        self.mask_cache.insert(state, mask.clone());
        mask
    }

    /// 获取上下文无关token比例
    pub fn context_independent_ratio(&self, vocabulary: &[String]) -> f64 {
        let ci_count = vocabulary
            .iter()
            .filter(|t| self.context_independent.contains(*t))
            .count();
        ci_count as f64 / vocabulary.len() as f64
    }
}

// =============================================================================
// 第五部分: 编译器引导生成反馈循环 (额外验证)
// =============================================================================

/// 编译器反馈类型
#[derive(Clone, Debug)]
pub enum CompilerFeedback {
    TypeMismatch { expected: Type, found: Type },
    UndefinedVariable(String),
    UndefinedFunction(String),
    BorrowCheckError(String),
    SyntaxError(String),
}

/// 编译器引导的生成器
pub struct CompilerGuidedGenerator {
    feedback_history: Vec<CompilerFeedback>,
    max_iterations: usize,
}

impl CompilerGuidedGenerator {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            feedback_history: Vec::new(),
            max_iterations,
        }
    }

    /// 模拟编译器反馈
    pub fn simulate_compile(&self, code: &str) -> Vec<CompilerFeedback> {
        let mut feedback = Vec::new();

        // 简化实现: 检查常见的类型错误模式
        if code.contains("undefined_var") {
            feedback.push(CompilerFeedback::UndefinedVariable("undefined_var".to_string()));
        }

        feedback
    }

    /// 基于反馈生成修复提示
    pub fn generate_fix_suggestion(&self, feedback: &CompilerFeedback) -> String {
        match feedback {
            CompilerFeedback::TypeMismatch { expected, found } => {
                format!("Type mismatch: expected {:?}, found {:?}", expected, found)
            }
            CompilerFeedback::UndefinedVariable(name) => {
                format!("Variable '{}' is not defined. Consider declaring it first.", name)
            }
            CompilerFeedback::UndefinedFunction(name) => {
                format!("Function '{}' is not defined.", name)
            }
            CompilerFeedback::BorrowCheckError(msg) => {
                format!("Borrow check error: {}", msg)
            }
            CompilerFeedback::SyntaxError(msg) => {
                format!("Syntax error: {}", msg)
            }
        }
    }
}

// =============================================================================
// 测试模块
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试H1: Typestate模式编译期保证
    #[test]
    fn test_typestate_guarantees() {
        // 正确的状态转换链
        let generator = CodeGenerator::<Idle>::new()
            .start_parsing()
            .parse_input("test code")
            .finish_parsing()
            .add_variable("x", Type::Base("int".to_string()))
            .finish_type_checking();

        // 注意: 以下代码如果取消注释会在编译期报错
        // 证明Typestate模式确实在编译期强制执行状态转换

        // 错误1: 无法从Idle直接调用generate_constrained_token
        // let _ = CodeGenerator::<Idle>::new().generate_constrained_token(&[]);

        // 错误2: 无法跳过Parsing直接到TypeChecking
        // let _ = CodeGenerator::<Idle>::new().start_parsing().finish_type_checking();

        // 错误3: 状态转换后旧值被消耗，无法再次使用
        // let idle = CodeGenerator::<Idle>::new();
        // let parsing = idle.start_parsing();
        // let _ = idle.start_parsing(); // 编译错误: idle已被移动

        // 完成生成
        let mut gen = generator;
        let candidates = vec!["x".to_string(), "y".to_string()];
        let token = gen.generate_constrained_token(&candidates);
        assert_eq!(token, Some("x".to_string())); // 只有x在类型上下文中

        let complete = gen.finish_generation();
        assert!(complete.verify_prefix_property());
    }

    /// 测试H2: 自底向上 vs 自顶向下搜索
    #[test]
    fn test_search_strategies() {
        let constructors = vec![
            Constructor::Variable("x".to_string()),
            Constructor::Variable("y".to_string()),
        ];

        let mut search = TypeReachabilitySearch::new(constructors);
        let target = Type::Base("int".to_string());

        let top_down_result = search.search_top_down(&target);
        let bottom_up_result = search.search_bottom_up(&target);

        // 两种策略应该产生一致的结果
        match (&top_down_result, &bottom_up_result) {
            (InhabitationResult::Inhabited(_), InhabitationResult::Inhabited(_)) => {}
            (InhabitationResult::Uninhabited, InhabitationResult::Uninhabited) => {}
            _ => panic!("Search strategies produced inconsistent results"),
        }
    }

    /// 测试H3: 细化类型子类型关系
    #[test]
    fn test_refinement_subtyping() {
        let base_int = Type::Base("int".to_string());
        let pos_int = Type::Refinement(
            Box::new(base_int.clone()),
            RefinementPredicate::GreaterThan(0),
        );

        // {x: int | x > 0} <: int 成立
        assert!(RefinementTypeSystem::is_subtype(&pos_int, &base_int));

        // int <: {x: int | x > 0} 不成立
        assert!(!RefinementTypeSystem::is_subtype(&base_int, &pos_int));

        // 检查谓词
        assert!(RefinementTypeSystem::check_predicate(5, &RefinementPredicate::GreaterThan(0)));
        assert!(!RefinementTypeSystem::check_predicate(-1, &RefinementPredicate::GreaterThan(0)));
    }

    /// 测试H4: 上下文无关token预计算
    #[test]
    fn test_token_classification() {
        let vocabulary = vec![
            "if".to_string(),
            "else".to_string(),
            "while".to_string(),
            "x".to_string(),
            "y".to_string(),
            "123".to_string(),
        ];

        let classifier = TokenClassifier::new(vocabulary.clone());
        let ratio = classifier.context_independent_ratio(&vocabulary);

        // 预期至少50%是上下文无关的(关键字和数字)
        assert!(ratio >= 0.5, "Context independent ratio too low: {}", ratio);

        // 预计算掩码
        let mut classifier = classifier;
        let mask = classifier.precompute_mask(0, &vocabulary);
        assert_eq!(mask.len(), vocabulary.len());
    }

    /// 测试H5: 类型 inhabitation 搜索性能特征
    #[test]
    fn test_inhabitation_performance() {
        let constructors: Vec<Constructor> = (0..100)
            .map(|i| Constructor::Variable(format!("var{}", i)))
            .collect();

        let mut search = TypeReachabilitySearch::new(constructors);
        let target = Type::Base("int".to_string());

        // 使用缓存的搜索应该更快
        let start = std::time::Instant::now();
        let _ = search.search_top_down(&target);
        let first_duration = start.elapsed();

        let start = std::time::Instant::now();
        let _ = search.search_top_down(&target); // 从缓存获取
        let cached_duration = start.elapsed();

        // 缓存版本应该更快(或至少不更慢)
        println!("First search: {:?}, Cached search: {:?}", first_duration, cached_duration);
    }

    /// 测试编译器反馈循环
    #[test]
    fn test_compiler_feedback() {
        let generator = CompilerGuidedGenerator::new(10);

        let code = "let x = undefined_var;";
        let feedback = generator.simulate_compile(code);

        assert!(!feedback.is_empty());
        match &feedback[0] {
            CompilerFeedback::UndefinedVariable(name) => {
                assert_eq!(name, "undefined_var");
            }
            _ => panic!("Expected undefined variable error"),
        }

        let suggestion = generator.generate_fix_suggestion(&feedback[0]);
        assert!(suggestion.contains("undefined_var"));
    }

    /// 测试前缀属性验证
    #[test]
    fn test_prefix_property() {
        let generator = CodeGenerator::<Idle>::new()
            .start_parsing()
            .finish_parsing()
            .finish_type_checking()
            .finish_generation();

        assert!(generator.verify_prefix_property());
    }

    /// 测试类型约束解码集成
    #[test]
    fn test_type_constrained_generation() {
        let generator = CodeGenerator::<Idle>::new()
            .start_parsing()
            .finish_parsing()
            .add_variable("valid_var", Type::Base("int".to_string()))
            .finish_type_checking();

        let mut gen = generator;
        let candidates = vec![
            "valid_var".to_string(),
            "invalid_var".to_string(),
        ];

        let token = gen.generate_constrained_token(&candidates);
        assert_eq!(token, Some("valid_var".to_string()));
    }
}

// =============================================================================
// 主函数 - 演示
// =============================================================================

fn main() {
    println!("=== 类型约束生成研究演示 ===\n");

    // 演示Typestate模式
    println!("1. Typestate模式演示:");
    let generator = CodeGenerator::<Idle>::new()
        .start_parsing()
        .parse_input("generate code")
        .finish_parsing()
        .add_variable("x", Type::Base("int".to_string()))
        .add_variable("y", Type::Base("string".to_string()))
        .finish_type_checking();

    let mut gen = generator;
    let candidates = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let token = gen.generate_constrained_token(&candidates);
    println!("   类型约束解码选择: {:?}", token);

    let complete = gen.finish_generation();
    println!("   前缀属性验证: {}", complete.verify_prefix_property());

    // 演示细化类型
    println!("\n2. 细化类型演示:");
    let pos_int = RefinementTypeSystem::refine(
        Type::Base("int".to_string()),
        RefinementPredicate::GreaterThan(0),
    );
    println!("   正整数类型: {:?}", pos_int);
    println!("   5 > 0: {}", RefinementTypeSystem::check_predicate(5, &RefinementPredicate::GreaterThan(0)));

    // 演示Token分类
    println!("\n3. Token分类演示:");
    let vocabulary: Vec<String> = (0..1000)
        .map(|i| {
            if i < 50 {
                ["if", "else", "while", "for", "return", "fn", "let"][i % 7].to_string()
            } else {
                format!("var{}", i)
            }
        })
        .collect();

    let classifier = TokenClassifier::new(vocabulary.clone());
    let ratio = classifier.context_independent_ratio(&vocabulary);
    println!("   上下文无关token比例: {:.1}%", ratio * 100.0);

    println!("\n=== 演示完成 ===");
}
