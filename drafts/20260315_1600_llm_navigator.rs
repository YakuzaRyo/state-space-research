//! LLM导航器实现：代码生成任务中的启发式搜索
//! 方向: 08_llm_as_navigator
//! 时间: 2026-03-15 16:00
//! 核心: LLM作为启发式函数在A*搜索中的应用

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

// ============================================================================
// 第一部分：状态空间定义
// ============================================================================

/// 代码状态 - 类型系统约束的有效状态
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CodeState {
    /// AST节点序列
    pub ast_nodes: Vec<AstNode>,
    /// 当前类型上下文
    pub type_context: TypeContext,
    /// 状态有效性标记
    pub is_valid: bool,
}

/// AST节点
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum AstNode {
    Function(String),           // 函数名
    Variable(String, Type),    // 变量名: 类型
    Expression(Expression),
    Statement(Statement),
}

/// 表达式
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    BinaryOp(Box<Expression>, Op, Box<Expression>),
    FunctionCall(String, Vec<Expression>),
}

/// 字面量
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Literal {
    Int(i64),
    String(String),
    Bool(bool),
}

/// 二元运算符
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Op {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
}

/// 语句
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Statement {
    Assign(String, Expression),
    Return(Expression),
    If(Expression, Vec<Statement>, Vec<Statement>),
    While(Expression, Vec<Statement>),
}

/// 类型
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Type {
    Int, String, Bool, Void,
    Function(Box<Type>, Vec<Type>),
}

/// 类型上下文 - 追踪可用类型
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeContext {
    variables: HashMap<String, Type>,
    functions: HashMap<String, (Vec<Type>, Type)>,
}

impl Hash for TypeContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // 使用简单的方式：基于变量和函数数量的哈希
        self.variables.len().hash(state);
        self.functions.len().hash(state);
    }
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_var(&mut self, name: String, ty: Type) {
        self.variables.insert(name, ty);
    }

    pub fn add_function(&mut self, name: String, params: Vec<Type>, ret: Type) {
        self.functions.insert(name, (params, ret));
    }

    pub fn get_var_type(&self, name: &str) -> Option<Type> {
        self.variables.get(name).cloned()
    }

    pub fn get_function_signature(&self, name: &str) -> Option<(Vec<Type>, Type)> {
        self.functions.get(name).cloned()
    }
}

// ============================================================================
// 第二部分：动作空间定义
// ============================================================================

/// 代码编辑动作 - 类型系统约束下的合法操作
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum EditAction {
    AddFunction(String, Vec<(String, Type)>, Type, Vec<Statement>),
    AddVariable(String, Type, Option<Expression>),
    UpdateVariable(String, Expression),
    AddStatement(Statement),
    RefactorRename { old_name: String, new_name: String },
    AddImport(String),
}

/// 动作代价
impl EditAction {
    pub fn cost(&self) -> f64 {
        match self {
            EditAction::AddFunction(..) => 10.0,
            EditAction::AddVariable(..) => 2.0,
            EditAction::UpdateVariable(..) => 1.0,
            EditAction::AddStatement(..) => 1.0,
            EditAction::RefactorRename { .. } => 5.0,
            EditAction::AddImport(..) => 3.0,
        }
    }
}

/// 动作生成器 - 根据当前状态生成可用动作
pub struct ActionGenerator;

impl ActionGenerator {
    /// 生成当前状态下的合法动作
    pub fn generate_actions(state: &CodeState) -> Vec<EditAction> {
        let mut actions = Vec::new();

        // 添加变量
        for ty in &[Type::Int, Type::String, Type::Bool] {
            let name = format!("var_{}", state.ast_nodes.len());
            actions.push(EditAction::AddVariable(name, ty.clone(), None));
        }

        // 添加基本语句
        actions.push(EditAction::AddStatement(Statement::Return(
            Expression::Literal(Literal::Int(0))
        )));
        actions.push(EditAction::AddStatement(Statement::Assign(
            "x".to_string(),
            Expression::Literal(Literal::Int(0))
        )));

        // 添加函数（需要更多上下文）
        if state.type_context.functions.is_empty() {
            actions.push(EditAction::AddFunction(
                "main".to_string(),
                vec![],
                Type::Void,
                vec![]
            ));
        }

        actions
    }
}

// ============================================================================
// 第三部分：状态转移函数
// ============================================================================

/// 状态转移函数 - 确定性执行
pub struct StateTransition;

impl StateTransition {
    /// 应用动作到状态，返回新状态
    pub fn apply(state: &CodeState, action: &EditAction) -> CodeState {
        let mut new_state = state.clone();

        match action {
            EditAction::AddFunction(name, params, ret, body) => {
                new_state.ast_nodes.push(AstNode::Function(name.clone()));
                let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                new_state.type_context.add_function(name.clone(), param_types, ret.clone());
            }

            EditAction::AddVariable(name, ty, init) => {
                new_state.ast_nodes.push(AstNode::Variable(name.clone(), ty.clone()));
                new_state.type_context.add_var(name.clone(), ty.clone());
                if let Some(expr) = init {
                    new_state.ast_nodes.push(AstNode::Statement(
                        Statement::Assign(name.clone(), expr.clone())
                    ));
                }
            }

            EditAction::UpdateVariable(name, expr) => {
                new_state.ast_nodes.push(AstNode::Statement(
                    Statement::Assign(name.clone(), expr.clone())
                ));
            }

            EditAction::AddStatement(stmt) => {
                new_state.ast_nodes.push(AstNode::Statement(stmt.clone()));
            }

            EditAction::RefactorRename { old_name, new_name } => {
                // 重命名逻辑
                if let Some(ty) = new_state.type_context.get_var_type(old_name) {
                    new_state.type_context.variables.remove(old_name);
                    new_state.type_context.variables.insert(new_name.clone(), ty);
                }
            }

            EditAction::AddImport(module) => {
                // 导入逻辑
                println!("Import: {}", module);
            }
        }

        // 验证状态有效性
        new_state.is_valid = Self::validate(&new_state);

        new_state
    }

    /// 验证状态有效性
    fn validate(state: &CodeState) -> bool {
        // 基本验证：检查类型一致性
        // 这里简化处理，实际需要更复杂的类型检查
        true
    }
}

// ============================================================================
// 第四部分：LLM启发式函数接口
// ============================================================================

/// LLM启发式函数 trait
pub trait LLMHeuristic: Send + Sync {
    /// 评估状态到目标的接近程度
    fn evaluate(&self, state: &CodeState, target: &Goal) -> f64;

    /// 评估候选动作的优先级
    fn score_action(&self, state: &CodeState, action: &EditAction, target: &Goal) -> f64;
}

/// 目标定义
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Goal {
    pub description: String,
    pub required_types: Vec<Type>,
    pub required_functions: Vec<String>,
}

impl Goal {
    pub fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
            required_types: vec![],
            required_functions: vec![],
        }
    }

    pub fn with_type(mut self, ty: Type) -> Self {
        self.required_types.push(ty);
        self
    }

    pub fn with_function(mut self, name: &str) -> Self {
        self.required_functions.push(name.to_string());
        self
    }

    /// 检查状态是否满足目标
    pub fn is_satisfied(&self, state: &CodeState) -> bool {
        // 检查必需的函数
        for func in &self.required_functions {
            if !state.type_context.functions.contains_key(func) {
                return false;
            }
        }

        // 检查必需的变量类型
        for ty in &self.required_types {
            if !state.ast_nodes.iter().any(|n| match n {
                AstNode::Variable(_, t) => t == ty,
                _ => false,
            }) {
                return false;
            }
        }

        true
    }
}

/// 示例：基于规则的启发式函数
pub struct RuleBasedHeuristic;

impl RuleBasedHeuristic {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RuleBasedHeuristic {
    fn default() -> Self {
        Self::new()
    }
}

impl LLMHeuristic for RuleBasedHeuristic {
    fn evaluate(&self, state: &CodeState, target: &Goal) -> f64 {
        // 简化的启发式评估：基于目标匹配度
        let mut score = 0.0;

        // 函数匹配
        for func in &target.required_functions {
            if state.type_context.functions.contains_key(func) {
                score += 10.0;
            }
        }

        // 类型匹配
        for ty in &target.required_types {
            if state.ast_nodes.iter().any(|n| match n {
                AstNode::Variable(_, t) => t == ty,
                _ => false,
            }) {
                score += 5.0;
            }
        }

        // 状态复杂度（越复杂代价越高）
        score -= state.ast_nodes.len() as f64 * 0.1;

        score
    }

    fn score_action(&self, state: &CodeState, action: &EditAction, target: &Goal) -> f64 {
        // 评估动作的预期价值
        let mut score = self.evaluate(state, target);

        // 动作代价
        score -= action.cost() * 0.5;

        // 动作特定加成
        match action {
            EditAction::AddFunction(name, _, _, _) => {
                if target.required_functions.contains(name) {
                    score += 20.0;
                }
            }
            EditAction::AddVariable(_, ty, _) => {
                if target.required_types.contains(ty) {
                    score += 10.0;
                }
            }
            _ => {}
        }

        score
    }
}

// ============================================================================
// 第五部分：A*搜索实现
// ============================================================================

/// A*搜索节点
#[derive(Clone, Debug)]
struct AStarNode {
    state: CodeState,
    g_cost: f64,  // 已付代价
    f_cost: f64,  // 总代价 f = g + h
    parent: Option<Box<AStarNode>>,
    action: Option<EditAction>,
}

impl Eq for AStarNode {}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.f_cost.partial_cmp(&self.f_cost)
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

/// LLM导航器 - A*搜索驱动的代码生成
pub struct LLMNavigator<H: LLMHeuristic> {
    heuristic: H,
    max_iterations: usize,
}

impl<H: LLMHeuristic> LLMNavigator<H> {
    pub fn new(heuristic: H) -> Self {
        Self {
            heuristic,
            max_iterations: 1000,
        }
    }

    pub fn navigate(&self, initial: CodeState, target: Goal) -> Option<Vec<EditAction>> {
        // A*搜索
        let mut frontier: BinaryHeap<AStarNode> = BinaryHeap::new();
        let mut visited: HashMap<CodeState, f64> = HashMap::new();

        // 初始化
        let h_cost = self.heuristic.evaluate(&initial, &target);
        frontier.push(AStarNode {
            state: initial.clone(),
            g_cost: 0.0,
            f_cost: h_cost,
            parent: None,
            action: None,
        });

        // 搜索循环
        for _ in 0..self.max_iterations {
            let current = match frontier.pop() {
                Some(node) => node,
                None => return None,  // 无解
            };

            // 检查目标
            if target.is_satisfied(&current.state) {
                return Some(Self::reconstruct_path(&current));
            }

            // 跳过已访问状态
            if let Some(&best_g) = visited.get(&current.state) {
                if current.g_cost > best_g {
                    continue;
                }
            }
            visited.insert(current.state.clone(), current.g_cost);

            // 生成并评估动作
            let actions = ActionGenerator::generate_actions(&current.state);
            for action in actions {
                // LLM评估动作分数
                let action_score = self.heuristic.score_action(
                    &current.state,
                    &action,
                    &target
                );

                // 应用动作
                let next_state = StateTransition::apply(&current.state, &action);

                // 跳过无效状态
                if !next_state.is_valid {
                    continue;
                }

                // 计算新代价
                let g = current.g_cost + action.cost();
                let h = self.heuristic.evaluate(&next_state, &target);
                let f = g + h;

                // 添加到前沿
                frontier.push(AStarNode {
                    state: next_state,
                    g_cost: g,
                    f_cost: f,
                    parent: Some(Box::new(current.clone())),
                    action: Some(action),
                });
            }
        }

        None  // 达到最大迭代次数
    }

    fn reconstruct_path(node: &AStarNode) -> Vec<EditAction> {
        let mut path = Vec::new();
        let mut current = Some(node);

        while let Some(n) = current {
            if let Some(action) = &n.action {
                path.push(action.clone());
            }
            current = n.parent.as_ref().map(|p| p.as_ref());
        }

        path.reverse();
        path
    }
}

// ============================================================================
// 第六部分：MCTS变体（简化版）
// ============================================================================

/// MCTS节点
#[derive(Clone, Debug)]
struct MCTSNode {
    state: CodeState,
    visits: usize,
    value: f64,
    children: HashMap<EditAction, MCTSNode>,
    parent: Option<Box<MCTSNode>>,
}

impl MCTSNode {
    pub fn new(state: CodeState) -> Self {
        Self {
            state,
            visits: 0,
            value: 0.0,
            children: HashMap::new(),
            parent: None,
        }
    }

    pub fn ucb1(&self, parent_visits: usize) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        self.value / self.visits as f64 +
            (2.0 * (parent_visits as f64).ln() / self.visits as f64).sqrt()
    }
}

/// MCTS导航器
pub struct MCTSNavigator<H: LLMHeuristic> {
    heuristic: H,
    max_iterations: usize,
}

impl<H: LLMHeuristic> MCTSNavigator<H> {
    pub fn new(heuristic: H) -> Self {
        Self {
            heuristic,
            max_iterations: 500,
        }
    }

    pub fn navigate(&self, initial: CodeState, target: Goal) -> Option<Vec<EditAction>> {
        let mut root = MCTSNode::new(initial);

        for _ in 0..self.max_iterations {
            // 选择
            let mut current = &mut root;
            while !current.children.is_empty() {
                let best_action = current.children.iter()
                    .max_by_key(|(_, n)| n.ucb1(current.visits) as i64)
                    .map(|(a, _)| a.clone())
                    .unwrap();

                current = current.children.get_mut(&best_action).unwrap();
            }

            // 扩展
            let actions = ActionGenerator::generate_actions(&current.state);
            for action in actions {
                let next_state = StateTransition::apply(&current.state, &action);
                if next_state.is_valid {
                    let mut child = MCTSNode::new(next_state);
                    child.parent = Some(Box::new(current.clone()));
                    current.children.insert(action, child);
                }
            }

            // 模拟（使用启发式评估）
            // 简化：只评估一次
            if let Some((_, child)) = current.children.iter_mut().next() {
                let value = self.heuristic.evaluate(&child.state, &target);
                child.visits += 1;
                child.value = value;
            }
        }

        // 选择最佳路径
        // 简化：返回空路径
        Some(vec![])
    }
}

// ============================================================================
// 主函数示例
// ============================================================================

fn main() {
    println!("LLM Navigator - A* Search with Heuristic Function");
    println!("==================================================\n");

    // 创建启发式函数
    let heuristic = RuleBasedHeuristic::new();

    // 创建导航器
    let navigator = LLMNavigator::new(heuristic);

    // 初始状态
    let initial = CodeState {
        ast_nodes: vec![],
        type_context: TypeContext::new(),
        is_valid: true,
    };

    // 目标：生成一个包含main函数的代码
    let target = Goal::new("Generate a main function")
        .with_function("main")
        .with_type(Type::Int);

    println!("Initial state: {:?}", initial);
    println!("Target: {:?}\n", target);

    // 执行导航
    match navigator.navigate(initial, target) {
        Some(path) => {
            println!("Found path with {} actions:", path.len());
            for (i, action) in path.iter().enumerate() {
                println!("  {}. {:?}", i + 1, action);
            }
        }
        None => {
            println!("No solution found within iteration limit");
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_context() {
        let mut ctx = TypeContext::new();
        ctx.add_var("x".to_string(), Type::Int);
        ctx.add_function("add".to_string(), vec![Type::Int, Type::Int], Type::Int);

        assert_eq!(ctx.get_var_type("x"), Some(Type::Int));
        assert_eq!(ctx.get_function_signature("add"), Some((vec![Type::Int, Type::Int], Type::Int)));
    }

    #[test]
    fn test_goal_satisfaction() {
        let goal = Goal::new("test")
            .with_function("main")
            .with_type(Type::Int);

        let mut state = CodeState {
            ast_nodes: vec![AstNode::Variable("x".to_string(), Type::Int)],
            type_context: {
                let mut ctx = TypeContext::new();
                ctx.add_function("main".to_string(), vec![], Type::Void);
                ctx
            },
            is_valid: true,
        };

        assert!(goal.is_satisfied(&state));
    }

    #[test]
    fn test_astar_navigation() {
        let heuristic = RuleBasedHeuristic::new();
        let navigator = LLMNavigator::new(heuristic);

        let initial = CodeState {
            ast_nodes: vec![],
            type_context: TypeContext::new(),
            is_valid: true,
        };

        let target = Goal::new("simple function")
            .with_function("main");

        let path = navigator.navigate(initial, target);
        println!("Path: {:?}", path);
    }
}
