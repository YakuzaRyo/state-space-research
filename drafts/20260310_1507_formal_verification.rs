//! 形式验证集成草案 - 状态空间架构
//!
//! 本草案展示如何将形式验证工具(Clover/Dafny/Verus/Kani风格)集成到状态空间架构中，
//! 实现LLM输出的自动过滤与验证引导的代码生成。
//!
//! 核心设计:
//! 1. 形式规约作为状态约束
//! 2. 验证反馈循环机制
//! 3. 证明义务生成器
//! 4. 分层验证架构(L1-L4)

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// 第一部分: 形式规约DSL (Dafny/Creusot风格)
// ============================================================================

/// 规约表达式 - 用于表达前置/后置条件和不变量
#[derive(Debug, Clone, PartialEq)]
pub enum SpecExpr {
    /// 布尔常量
    Bool(bool),
    /// 整数常量
    Int(i64),
    /// 变量引用
    Var(String),
    /// 相等
    Eq(Box<SpecExpr>, Box<SpecExpr>),
    /// 不等
    Ne(Box<SpecExpr>, Box<SpecExpr>),
    /// 小于
    Lt(Box<SpecExpr>, Box<SpecExpr>),
    /// 小于等于
    Le(Box<SpecExpr>, Box<SpecExpr>),
    /// 大于
    Gt(Box<SpecExpr>, Box<SpecExpr>),
    /// 大于等于
    Ge(Box<SpecExpr>, Box<SpecExpr>),
    /// 逻辑与
    And(Box<SpecExpr>, Box<SpecExpr>),
    /// 逻辑或
    Or(Box<SpecExpr>, Box<SpecExpr>),
    /// 逻辑非
    Not(Box<SpecExpr>),
    /// 蕴含 (P ==> Q)
    Implies(Box<SpecExpr>, Box<SpecExpr>),
    /// 全称量词 (forall x: T :: P)
    Forall(String, Type, Box<SpecExpr>),
    /// 存在量词 (exists x: T :: P)
    Exists(String, Type, Box<SpecExpr>),
    /// 数组/向量长度
    Len(Box<SpecExpr>),
    /// 数组索引
    Index(Box<SpecExpr>, Box<SpecExpr>),
    /// 旧值引用 (用于后置条件)
    Old(Box<SpecExpr>),
    /// 加法
    Add(Box<SpecExpr>, Box<SpecExpr>),
    /// 减法
    Sub(Box<SpecExpr>, Box<SpecExpr>),
    /// 乘法
    Mul(Box<SpecExpr>, Box<SpecExpr>),
    /// 除法
    Div(Box<SpecExpr>, Box<SpecExpr>),
}

/// 类型系统
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Bool,
    Int,
    Uint(usize), // Uint(32), Uint(64)等
    Array(Box<Type>),
    Vector(Box<Type>),
    Ref(Box<Type>),
    MutRef(Box<Type>),
    Custom(String),
}

/// 函数规约 - 类似Dafny的method规格
#[derive(Debug, Clone)]
pub struct FunctionSpec {
    /// 函数名
    pub name: String,
    /// 前置条件 (requires)
    pub requires: Vec<SpecExpr>,
    /// 后置条件 (ensures)
    pub ensures: Vec<SpecExpr>,
    /// 可修改的变量/对象 (modifies)
    pub modifies: Vec<String>,
    /// 终止度量 (decreases)
    pub decreases: Option<SpecExpr>,
    /// 参数规格
    pub params: Vec<(String, Type)>,
    /// 返回值规格
    pub returns: Vec<(String, Type)>,
}

impl FunctionSpec {
    /// 创建新的函数规约
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            requires: Vec::new(),
            ensures: Vec::new(),
            modifies: Vec::new(),
            decreases: None,
            params: Vec::new(),
            returns: Vec::new(),
        }
    }

    /// 添加前置条件
    pub fn requires(mut self, expr: SpecExpr) -> Self {
        self.requires.push(expr);
        self
    }

    /// 添加后置条件
    pub fn ensures(mut self, expr: SpecExpr) -> Self {
        self.ensures.push(expr);
        self
    }

    /// 添加参数
    pub fn param(mut self, name: impl Into<String>, ty: Type) -> Self {
        self.params.push((name.into(), ty));
        self
    }

    /// 生成验证条件(VC)
    pub fn generate_vcs(&self) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();

        // 前置条件合取
        let pre = if self.requires.is_empty() {
            SpecExpr::Bool(true)
        } else {
            self.requires.iter().skip(1).fold(
                self.requires[0].clone(),
                |acc, r| SpecExpr::And(Box::new(acc), Box::new(r.clone()))
            )
        };

        // 后置条件合取
        let post = if self.ensures.is_empty() {
            SpecExpr::Bool(true)
        } else {
            self.ensures.iter().skip(1).fold(
                self.ensures[0].clone(),
                |acc, e| SpecExpr::And(Box::new(acc), Box::new(e.clone()))
            )
        };

        // 主要VC: pre ==> post
        vcs.push(VerificationCondition {
            name: format!("{}_correctness", self.name),
            expr: SpecExpr::Implies(Box::new(pre), Box::new(post)),
            kind: VCKind::Postcondition,
        });

        vcs
    }
}

/// 验证条件
#[derive(Debug, Clone)]
pub struct VerificationCondition {
    pub name: String,
    pub expr: SpecExpr,
    pub kind: VCKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VCKind {
    Precondition,
    Postcondition,
    Invariant,
    Termination,
    Safety,
}

// ============================================================================
// 第二部分: 精化类型系统 (Flux风格)
// ============================================================================

/// 精化类型 - 基础类型 + 精化谓词
#[derive(Debug, Clone)]
pub struct RefinementType {
    pub base: Type,
    pub binder: String, // 绑定变量名, 通常是"v"
    pub predicate: SpecExpr,
}

impl RefinementType {
    /// 创建非负整数类型 {v: Int | v >= 0}
    pub fn nat() -> Self {
        Self {
            base: Type::Int,
            binder: "v".to_string(),
            predicate: SpecExpr::Ge(
                Box::new(SpecExpr::Var("v".to_string())),
                Box::new(SpecExpr::Int(0))
            ),
        }
    }

    /// 创建正整数类型 {v: Int | v > 0}
    pub fn pos() -> Self {
        Self {
            base: Type::Int,
            binder: "v".to_string(),
            predicate: SpecExpr::Gt(
                Box::new(SpecExpr::Var("v".to_string())),
                Box::new(SpecExpr::Int(0))
            ),
        }
    }

    /// 创建范围受限整数 {v: Int | lo <= v < hi}
    pub fn range(lo: i64, hi: i64) -> Self {
        let v = SpecExpr::Var("v".to_string());
        Self {
            base: Type::Int,
            binder: "v".to_string(),
            predicate: SpecExpr::And(
                Box::new(SpecExpr::Ge(Box::new(v.clone()), Box::new(SpecExpr::Int(lo)))),
                Box::new(SpecExpr::Lt(Box::new(v), Box::new(SpecExpr::Int(hi))))
            ),
        }
    }

    /// 创建非空向量 {v: Vec<T> | len(v) > 0}
    pub fn non_empty_vec(elem_ty: Type) -> Self {
        Self {
            base: Type::Vector(Box::new(elem_ty)),
            binder: "v".to_string(),
            predicate: SpecExpr::Gt(
                Box::new(SpecExpr::Len(Box::new(SpecExpr::Var("v".to_string())))),
                Box::new(SpecExpr::Int(0))
            ),
        }
    }
}

/// 类型环境 - 用于精化类型检查
#[derive(Debug, Default)]
pub struct TypeEnv {
    bindings: HashMap<String, RefinementType>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, name: String, ty: RefinementType) {
        self.bindings.insert(name, ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&RefinementType> {
        self.bindings.get(name)
    }
}

// ============================================================================
// 第三部分: 验证反馈循环 (Clover风格)
// ============================================================================

/// 验证结果
#[derive(Debug, Clone)]
pub enum VerificationResult {
    /// 验证通过
    Verified,
    /// 验证失败,附带反例
    Failed(Counterexample),
    /// 超时
    Timeout,
    /// 未知/错误
    Unknown(String),
}

/// 反例 - 用于指导LLM修复
#[derive(Debug, Clone)]
pub struct Counterexample {
    /// 反例类型
    pub kind: CounterexampleKind,
    /// 具体输入值
    pub inputs: HashMap<String, ConcreteValue>,
    /// 失败路径描述
    pub trace: Vec<String>,
    /// 建议修复提示
    pub suggestion: String,
}

#[derive(Debug, Clone)]
pub enum CounterexampleKind {
    PreconditionViolation,
    PostconditionViolation,
    InvariantViolation,
    Overflow,
    DivisionByZero,
    OutOfBounds,
}

/// 具体值 - 反例中的值
#[derive(Debug, Clone)]
pub enum ConcreteValue {
    Bool(bool),
    Int(i64),
    Array(Vec<ConcreteValue>),
}

/// 验证反馈循环
pub struct VerificationFeedbackLoop {
    /// 最大迭代次数
    max_iterations: usize,
    /// 当前迭代
    current_iteration: usize,
    /// 验证历史
    history: Vec<VerificationAttempt>,
}

#[derive(Debug, Clone)]
pub struct VerificationAttempt {
    pub iteration: usize,
    pub code: String,
    pub result: VerificationResult,
    pub timestamp: std::time::SystemTime,
}

impl VerificationFeedbackLoop {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            max_iterations,
            current_iteration: 0,
            history: Vec::new(),
        }
    }

    /// 执行一次验证迭代
    pub fn iterate(&mut self, code: &str, spec: &FunctionSpec) -> VerificationResult {
        // 1. 生成验证条件
        let vcs = spec.generate_vcs();

        // 2. 模拟验证 (实际应调用SMT求解器)
        let result = self.mock_verify(code, &vcs);

        // 3. 记录历史
        self.history.push(VerificationAttempt {
            iteration: self.current_iteration,
            code: code.to_string(),
            result: result.clone(),
            timestamp: std::time::SystemTime::now(),
        });

        self.current_iteration += 1;
        result
    }

    /// 模拟验证 (实际实现应调用Z3/CVC5)
    fn mock_verify(&self, _code: &str, vcs: &[VerificationCondition]) -> VerificationResult {
        // 简化模拟: 随机成功或失败
        // 实际应使用SMT求解器验证VCs
        VerificationResult::Verified
    }

    /// 生成反馈提示用于LLM修复
    pub fn generate_feedback_prompt(&self) -> Option<String> {
        let last = self.history.last()?;

        match &last.result {
            VerificationResult::Failed(ce) => {
                let mut prompt = format!(
                    "验证失败 (迭代 {}):\n\n",
                    last.iteration
                );

                prompt.push_str(&format!("错误类型: {:?}\n", ce.kind));
                prompt.push_str("反例输入:\n");

                for (name, value) in &ce.inputs {
                    prompt.push_str(&format!("  {} = {:?}\n", name, value));
                }

                prompt.push_str(&format!("\n失败路径:\n"));
                for step in &ce.trace {
                    prompt.push_str(&format!("  - {}\n", step));
                }

                prompt.push_str(&format!("\n建议修复:\n{}\n", ce.suggestion));

                Some(prompt)
            }
            _ => None,
        }
    }

    /// 检查是否收敛
    pub fn is_converged(&self) -> bool {
        if let Some(last) = self.history.last() {
            matches!(last.result, VerificationResult::Verified)
        } else {
            false
        }
    }

    /// 检查是否达到最大迭代
    pub fn is_exhausted(&self) -> bool {
        self.current_iteration >= self.max_iterations
    }
}

// ============================================================================
// 第四部分: 证明义务生成器 (PO Generator)
// ============================================================================

/// 证明义务生成器 - 从状态空间约束生成验证条件
pub struct ProofObligationGenerator;

impl ProofObligationGenerator {
    /// 为状态转移生成证明义务
    pub fn generate_state_transition_po(
        pre_state: &StateConstraint,
        action: &Action,
        post_state: &StateConstraint,
    ) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();

        // VC1: 前置状态满足前置条件
        vcs.push(VerificationCondition {
            name: "state_precondition".to_string(),
            expr: pre_state.to_spec(),
            kind: VCKind::Precondition,
        });

        // VC2: 后置状态满足后置条件
        vcs.push(VerificationCondition {
            name: "state_postcondition".to_string(),
            expr: post_state.to_spec(),
            kind: VCKind::Postcondition,
        });

        // VC3: 不变量保持
        if let Some(inv) = &action.invariant {
            vcs.push(VerificationCondition {
                name: "invariant_preservation".to_string(),
                expr: SpecExpr::Implies(
                    Box::new(pre_state.to_spec()),
                    Box::new(inv.clone())
                ),
                kind: VCKind::Invariant,
            });
        }

        vcs
    }

    /// 为循环生成证明义务
    pub fn generate_loop_po(
        init: &SpecExpr,
        invariant: &SpecExpr,
        condition: &SpecExpr,
        body: &SpecExpr,
        post: &SpecExpr,
    ) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();

        // VC1: 初始化建立不变量
        vcs.push(VerificationCondition {
            name: "loop_init".to_string(),
            expr: SpecExpr::Implies(Box::new(init.clone()), Box::new(invariant.clone())),
            kind: VCKind::Invariant,
        });

        // VC2: 不变量保持
        let not_cond = SpecExpr::Not(Box::new(condition.clone()));
        let preserved = SpecExpr::Implies(
            Box::new(SpecExpr::And(
                Box::new(invariant.clone()),
                Box::new(condition.clone())
            )),
            Box::new(body.clone())
        );
        vcs.push(VerificationCondition {
            name: "loop_preservation".to_string(),
            expr: preserved,
            kind: VCKind::Invariant,
        });

        // VC3: 退出时满足后置条件
        vcs.push(VerificationCondition {
            name: "loop_exit".to_string(),
            expr: SpecExpr::Implies(
                Box::new(SpecExpr::And(
                    Box::new(invariant.clone()),
                    Box::new(not_cond)
                )),
                Box::new(post.clone())
            ),
            kind: VCKind::Postcondition,
        });

        vcs
    }
}

/// 状态约束 - 状态空间中的约束
#[derive(Debug, Clone)]
pub struct StateConstraint {
    pub name: String,
    pub constraints: Vec<SpecExpr>,
}

impl StateConstraint {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constraints: Vec::new(),
        }
    }

    pub fn add(&mut self, expr: SpecExpr) {
        self.constraints.push(expr);
    }

    pub fn to_spec(&self) -> SpecExpr {
        if self.constraints.is_empty() {
            SpecExpr::Bool(true)
        } else {
            self.constraints.iter().skip(1).fold(
                self.constraints[0].clone(),
                |acc, c| SpecExpr::And(Box::new(acc), Box::new(c.clone()))
            )
        }
    }
}

/// 动作/操作
#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub invariant: Option<SpecExpr>,
    pub effects: Vec<(String, SpecExpr)>, // 变量 -> 新值表达式
}

// ============================================================================
// 第五部分: 分层验证架构 (L1-L4)
// ============================================================================

/// 验证级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceLevel {
    /// L1: 类型安全 (Rust编译器保证)
    L1_TypeSafety,
    /// L2: 内存安全 (MIRI, Kani)
    L2_MemorySafety,
    /// L3: 功能正确性 (Verus, Creusot)
    L3_FunctionalCorrectness,
    /// L4: 完整形式验证 (seL4级别)
    L4_FullVerification,
}

impl fmt::Display for AssuranceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssuranceLevel::L1_TypeSafety => write!(f, "L1-TypeSafety"),
            AssuranceLevel::L2_MemorySafety => write!(f, "L2-MemorySafety"),
            AssuranceLevel::L3_FunctionalCorrectness => write!(f, "L3-FunctionalCorrectness"),
            AssuranceLevel::L4_FullVerification => write!(f, "L4-FullVerification"),
        }
    }
}

/// 分层验证器
pub struct LayeredVerifier {
    /// 目标验证级别
    target_level: AssuranceLevel,
    /// 各级验证器
    verifiers: HashMap<AssuranceLevel, Box<dyn Verifier>>,
}

/// 验证器trait
pub trait Verifier {
    fn name(&self) -> &str;
    fn level(&self) -> AssuranceLevel;
    fn verify(&self, code: &str, spec: &FunctionSpec) -> VerificationResult;
}

impl LayeredVerifier {
    pub fn new(target: AssuranceLevel) -> Self {
        Self {
            target_level: target,
            verifiers: HashMap::new(),
        }
    }

    pub fn register(&mut self, verifier: Box<dyn Verifier>) {
        self.verifiers.insert(verifier.level(), verifier);
    }

    /// 执行分层验证
    pub fn verify(&self, code: &str, spec: &FunctionSpec) -> LayeredVerificationResult {
        let mut results = Vec::new();
        let mut current_level = AssuranceLevel::L1_TypeSafety;

        while current_level <= self.target_level {
            if let Some(verifier) = self.verifiers.get(&current_level) {
                let result = verifier.verify(code, spec);
                let passed = matches!(result, VerificationResult::Verified);

                results.push(LevelResult {
                    level: current_level,
                    verifier: verifier.name().to_string(),
                    result,
                });

                if !passed {
                    break; // 某级失败,停止验证
                }
            }
            current_level = Self::next_level(current_level);
        }

        LayeredVerificationResult {
            target_level: self.target_level,
            reached_level: results.last().map(|r| r.level).unwrap_or(AssuranceLevel::L1_TypeSafety),
            results,
        }
    }

    fn next_level(current: AssuranceLevel) -> AssuranceLevel {
        match current {
            AssuranceLevel::L1_TypeSafety => AssuranceLevel::L2_MemorySafety,
            AssuranceLevel::L2_MemorySafety => AssuranceLevel::L3_FunctionalCorrectness,
            AssuranceLevel::L3_FunctionalCorrectness => AssuranceLevel::L4_FullVerification,
            AssuranceLevel::L4_FullVerification => AssuranceLevel::L4_FullVerification,
        }
    }
}

#[derive(Debug)]
pub struct LayeredVerificationResult {
    pub target_level: AssuranceLevel,
    pub reached_level: AssuranceLevel,
    pub results: Vec<LevelResult>,
}

#[derive(Debug)]
pub struct LevelResult {
    pub level: AssuranceLevel,
    pub verifier: String,
    pub result: VerificationResult,
}

// ============================================================================
// 第六部分: 测试用例
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试1: 基本函数规约 (Dafny风格)
    #[test]
    fn test_basic_function_spec() {
        // 定义 abs 函数的规约
        let spec = FunctionSpec::new("abs")
            .param("x", Type::Int)
            .returns(vec![("y".to_string(), Type::Int)])
            .requires(SpecExpr::Bool(true)) // 无前置条件
            .ensures(SpecExpr::Ge(
                Box::new(SpecExpr::Var("y".to_string())),
                Box::new(SpecExpr::Int(0))
            )) // y >= 0
            .ensures(SpecExpr::Or(
                Box::new(SpecExpr::Eq(
                    Box::new(SpecExpr::Var("y".to_string())),
                    Box::new(SpecExpr::Var("x".to_string()))
                )),
                Box::new(SpecExpr::Eq(
                    Box::new(SpecExpr::Var("y".to_string())),
                    Box::new(SpecExpr::Sub(
                        Box::new(SpecExpr::Int(0)),
                        Box::new(SpecExpr::Var("x".to_string()))
                    ))
                ))
            )); // y == x || y == -x

        let vcs = spec.generate_vcs();
        assert_eq!(vcs.len(), 1);
        assert_eq!(vcs[0].kind, VCKind::Postcondition);

        println!("Generated VC: {:?}", vcs[0]);
    }

    /// 测试2: 精化类型
    #[test]
    fn test_refinement_types() {
        let nat = RefinementType::nat();
        assert_eq!(nat.base, Type::Int);

        let range = RefinementType::range(0, 100);
        println!("Range type: {:?}", range);

        let non_empty = RefinementType::non_empty_vec(Type::Int);
        println!("Non-empty vec type: {:?}", non_empty);
    }

    /// 测试3: 类型环境
    #[test]
    fn test_type_env() {
        let mut env = TypeEnv::new();
        env.bind("x".to_string(), RefinementType::nat());
        env.bind("arr".to_string(), RefinementType::non_empty_vec(Type::Int));

        assert!(env.lookup("x").is_some());
        assert!(env.lookup("y").is_none());
    }

    /// 测试4: 验证反馈循环
    #[test]
    fn test_verification_feedback_loop() {
        let mut loop_ = VerificationFeedbackLoop::new(5);

        let spec = FunctionSpec::new("test")
            .requires(SpecExpr::Bool(true))
            .ensures(SpecExpr::Bool(true));

        let result = loop_.iterate("fn test() {}", &spec);

        assert!(matches!(result, VerificationResult::Verified));
        assert!(loop_.is_converged());
    }

    /// 测试5: 证明义务生成
    #[test]
    fn test_po_generator() {
        let pre = StateConstraint::new("pre")
            .tap(|s| s.add(SpecExpr::Ge(
                Box::new(SpecExpr::Var("x".to_string())),
                Box::new(SpecExpr::Int(0))
            )));

        let post = StateConstraint::new("post")
            .tap(|s| s.add(SpecExpr::Gt(
                Box::new(SpecExpr::Var("y".to_string())),
                Box::new(SpecExpr::Int(0))
            )));

        let action = Action {
            name: "increment".to_string(),
            invariant: Some(SpecExpr::Bool(true)),
            effects: vec![],
        };

        let vcs = ProofObligationGenerator::generate_state_transition_po(&pre, &action, &post);
        assert_eq!(vcs.len(), 3);
    }

    /// 测试6: 循环证明义务
    #[test]
    fn test_loop_po() {
        let init = SpecExpr::Eq(
            Box::new(SpecExpr::Var("i".to_string())),
            Box::new(SpecExpr::Int(0))
        );

        let invariant = SpecExpr::Ge(
            Box::new(SpecExpr::Var("i".to_string())),
            Box::new(SpecExpr::Int(0))
        );

        let condition = SpecExpr::Lt(
            Box::new(SpecExpr::Var("i".to_string())),
            Box::new(SpecExpr::Var("n".to_string()))
        );

        let body = SpecExpr::Ge(
            Box::new(SpecExpr::Add(
                Box::new(SpecExpr::Var("i".to_string())),
                Box::new(SpecExpr::Int(1))
            )),
            Box::new(SpecExpr::Int(0))
        );

        let post = SpecExpr::Ge(
            Box::new(SpecExpr::Var("i".to_string())),
            Box::new(SpecExpr::Int(0))
        );

        let vcs = ProofObligationGenerator::generate_loop_po(
            &init, &invariant, &condition, &body, &post
        );

        assert_eq!(vcs.len(), 3);
        assert_eq!(vcs[0].kind, VCKind::Invariant); // init
        assert_eq!(vcs[1].kind, VCKind::Invariant); // preservation
        assert_eq!(vcs[2].kind, VCKind::Postcondition); // exit
    }

    /// 测试7: 分层验证
    #[test]
    fn test_layered_verification() {
        let verifier = LayeredVerifier::new(AssuranceLevel::L3_FunctionalCorrectness);
        // 注意: 这里只是测试结构,实际验证器需要注册
        println!("Target level: {}", verifier.target_level);
    }

    /// 测试8: 复杂规约 (二分查找)
    #[test]
    fn test_binary_search_spec() {
        let sorted = SpecExpr::Forall(
            "i".to_string(),
            Type::Int,
            Box::new(SpecExpr::Forall(
                "j".to_string(),
                Type::Int,
                Box::new(SpecExpr::Implies(
                    Box::new(SpecExpr::And(
                        Box::new(SpecExpr::Ge(Box::new(SpecExpr::Var("i".to_string())), Box::new(SpecExpr::Int(0)))),
                        Box::new(SpecExpr::Lt(
                            Box::new(SpecExpr::Var("i".to_string())),
                            Box::new(SpecExpr::Var("j".to_string()))
                        ))
                    )),
                    Box::new(SpecExpr::Le(
                        Box::new(SpecExpr::Index(
                            Box::new(SpecExpr::Var("arr".to_string())),
                            Box::new(SpecExpr::Var("i".to_string()))
                        )),
                        Box::new(SpecExpr::Index(
                            Box::new(SpecExpr::Var("arr".to_string())),
                            Box::new(SpecExpr::Var("j".to_string()))
                        ))
                    ))
                ))
            ))
        );

        let spec = FunctionSpec::new("binary_search")
            .param("arr", Type::Array(Box::new(Type::Int)))
            .param("key", Type::Int)
            .returns(vec![("index".to_string(), Type::Int)])
            .requires(sorted)
            .ensures(SpecExpr::Implies(
                Box::new(SpecExpr::Ge(Box::new(SpecExpr::Var("index".to_string())), Box::new(SpecExpr::Int(0)))),
                Box::new(SpecExpr::Eq(
                    Box::new(SpecExpr::Index(
                        Box::new(SpecExpr::Var("arr".to_string())),
                        Box::new(SpecExpr::Var("index".to_string()))
                    )),
                    Box::new(SpecExpr::Var("key".to_string()))
                ))
            ));

        let vcs = spec.generate_vcs();
        assert_eq!(vcs.len(), 1);
        println!("Binary search VC generated successfully");
    }
}

// 辅助trait用于测试
trait Tap: Sized {
    fn tap<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }
}

impl Tap for StateConstraint {}

// ============================================================================
// 第八部分: 使用示例
// ============================================================================

/// 完整使用示例
pub fn example_usage() {
    // 1. 定义函数规约
    let spec = FunctionSpec::new("safe_divide")
        .param("a", Type::Int)
        .param("b", Type::Int)
        .returns(vec![("result".to_string(), Type::Int)])
        .requires(SpecExpr::Ne(
            Box::new(SpecExpr::Var("b".to_string())),
            Box::new(SpecExpr::Int(0))
        )) // b != 0
        .ensures(SpecExpr::Eq(
            Box::new(SpecExpr::Mul(
                Box::new(SpecExpr::Var("result".to_string())),
                Box::new(SpecExpr::Var("b".to_string()))
            )),
            Box::new(SpecExpr::Var("a".to_string()))
        )); // result * b == a

    // 2. 生成验证条件
    let vcs = spec.generate_vcs();
    println!("Generated {} verification conditions", vcs.len());

    // 3. 使用精化类型
    let pos = RefinementType::pos();
    println!("Positive integer type defined");

    // 4. 创建验证反馈循环
    let mut feedback_loop = VerificationFeedbackLoop::new(10);

    // 5. 模拟验证迭代
    let code = r#"
        fn safe_divide(a: i64, b: i64) -> i64 {
            assert!(b != 0);
            a / b
        }
    "#;

    let result = feedback_loop.iterate(code, &spec);
    println!("Verification result: {:?}", result);

    // 6. 分层验证
    let layered = LayeredVerifier::new(AssuranceLevel::L3_FunctionalCorrectness);
    println!("Target assurance level: {}", layered.target_level);
}

fn main() {
    example_usage();
}
