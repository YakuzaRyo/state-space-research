//! LLM作为导航器：A*搜索与启发式函数实现
//! 方向: llm_as_navigator
//! 时间: 2026-03-10 16:15
//! 核心: 展示LLM如何在L2 Pattern层作为启发式函数指导搜索

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::marker::PhantomData;

// =============================================================================
// 第一部分: 状态空间定义 (L3 Typestate层)
// =============================================================================

/// 状态种类标记
pub trait StateKind {}
pub struct Initial;
pub struct Expanded;
pub struct Goal;
impl StateKind for Initial {}
impl StateKind for Expanded {}
impl StateKind for Goal {}

/// 状态空间中的节点
/// 使用Typestate确保状态转换顺序
pub struct State<S: StateKind> {
    id: StateId,
    context: StateContext,
    _marker: PhantomData<S>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId(pub usize);

#[derive(Clone, Debug)]
pub struct StateContext {
    pub description: String,
    pub depth: usize,
    pub parent: Option<StateId>,
}

impl State<Initial> {
    pub fn new(id: usize, description: impl Into<String>) -> Self {
        State {
            id: StateId(id),
            context: StateContext {
                description: description.into(),
                depth: 0,
                parent: None,
            },
            _marker: PhantomData,
        }
    }

    pub fn expand(self, action: &str) -> State<Expanded> {
        State {
            id: self.id,
            context: StateContext {
                description: format!("{} -> {}", self.context.description, action),
                depth: self.context.depth + 1,
                parent: Some(self.id),
            },
            _marker: PhantomData,
        }
    }
}

impl State<Expanded> {
    pub fn check_goal(&self, goal_condition: impl Fn(&str) -> bool) -> bool {
        goal_condition(&self.context.description)
    }

    pub fn to_goal(self) -> State<Goal> {
        State {
            id: self.id,
            context: self.context,
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// 第二部分: LLM启发式函数接口 (L2 Pattern层)
// =============================================================================

/// LLM启发式函数trait
/// 模拟LLM评估状态的"希望值"
pub trait LLMHeuristic {
    /// 评估单个状态：返回启发式值 h(n)
    /// 值越小表示越接近目标
    fn evaluate(&self, state: &StateContext) -> f64;

    /// 比较两个状态：返回更有希望的状态
    fn compare(&self, s1: &StateContext, s2: &StateContext) -> Ordering {
        let h1 = self.evaluate(s1);
        let h2 = self.evaluate(s2);
        h1.partial_cmp(&h2).unwrap_or(Ordering::Equal)
    }

    /// 批量评估：优化API调用
    fn evaluate_batch(&self, states: &[&StateContext]) -> Vec<f64> {
        states.iter().map(|s| self.evaluate(s)).collect()
    }
}

/// 模拟LLM启发式实现
/// 实际应用中这里会调用LLM API
pub struct SimulatedLLMHeuristic {
    /// 关键词匹配表：关键词 -> 启发式权重
    keyword_weights: HashMap<String, f64>,
}

impl SimulatedLLMHeuristic {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        // 模拟LLM对目标关键词的识别
        weights.insert("goal".to_string(), 0.0);
        weights.insert("success".to_string(), 0.1);
        weights.insert("complete".to_string(), 0.2);
        weights.insert("progress".to_string(), 0.5);
        weights.insert("start".to_string(), 1.0);
        SimulatedLLMHeuristic {
            keyword_weights: weights,
        }
    }

    /// 模拟LLM推理过程
    fn llm_reason(&self, description: &str) -> f64 {
        let desc_lower = description.to_lowercase();
        let mut score = 1.0; // 默认最高代价

        // 模拟LLM的关键词匹配和推理
        for (keyword, weight) in &self.keyword_weights {
            if desc_lower.contains(keyword) {
                score = score.min(*weight);
            }
        }

        // 模拟LLM的上下文理解：路径越短越好
        let path_length = desc_lower.matches("->").count();
        score + (path_length as f64 * 0.1)
    }
}

impl LLMHeuristic for SimulatedLLMHeuristic {
    fn evaluate(&self, state: &StateContext) -> f64 {
        self.llm_reason(&state.description)
    }
}

/// 投票LLM启发式：多次采样取平均
/// 利用LLM的自我一致性
pub struct VotingLLMHeuristic<H: LLMHeuristic> {
    base_heuristic: H,
    num_samples: usize,
}

impl<H: LLMHeuristic> VotingLLMHeuristic<H> {
    pub fn new(base: H, samples: usize) -> Self {
        VotingLLMHeuristic {
            base_heuristic: base,
            num_samples: samples,
        }
    }
}

impl<H: LLMHeuristic> LLMHeuristic for VotingLLMHeuristic<H> {
    fn evaluate(&self, state: &StateContext) -> f64 {
        // 模拟多次LLM调用取平均
        let sum: f64 = (0..self.num_samples)
            .map(|_| self.base_heuristic.evaluate(state))
            .sum();
        sum / self.num_samples as f64
    }
}

// =============================================================================
// 第三部分: A*搜索算法 + LLM启发式
// =============================================================================

/// A*搜索节点
#[derive(Debug)]
pub struct AStarNode {
    state_id: StateId,
    g_cost: f64,           // 实际代价：从起点到当前状态
    h_cost: f64,           // 启发式估计：LLM评估
    f_cost: f64,           // f = g + h
    parent: Option<StateId>,
}

impl AStarNode {
    fn new(state_id: StateId, g: f64, h: f64, parent: Option<StateId>) -> Self {
        AStarNode {
            state_id,
            g_cost: g,
            h_cost: h,
            f_cost: g + h,
            parent,
        }
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // 最小堆：f_cost小的优先
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.state_id == other.state_id
    }
}

impl Eq for AStarNode {}

/// A*搜索器，使用LLM作为启发式函数
pub struct LLMStarSearch<H: LLMHeuristic> {
    heuristic: H,
    open_set: BinaryHeap<AStarNode>,
    closed_set: HashSet<StateId>,
    g_scores: HashMap<StateId, f64>,
    came_from: HashMap<StateId, StateId>,
    states: HashMap<StateId, StateContext>,
}

impl<H: LLMHeuristic> LLMStarSearch<H> {
    pub fn new(heuristic: H) -> Self {
        LLMStarSearch {
            heuristic,
            open_set: BinaryHeap::new(),
            closed_set: HashSet::new(),
            g_scores: HashMap::new(),
            came_from: HashMap::new(),
            states: HashMap::new(),
        }
    }

    /// 执行A*搜索
    pub fn search(
        &mut self,
        initial: State<Initial>,
        goal_check: impl Fn(&str) -> bool,
        expand_fn: impl Fn(&StateContext) -> Vec<String>,
    ) -> Option<Vec<StateId>> {
        let initial_id = initial.id;
        let initial_ctx = initial.context.clone();

        // 初始化
        let h0 = self.heuristic.evaluate(&initial_ctx);
        self.open_set.push(AStarNode::new(initial_id, 0.0, h0, None));
        self.g_scores.insert(initial_id, 0.0);
        self.states.insert(initial_id, initial_ctx);

        while let Some(current) = self.open_set.pop() {
            let current_id = current.state_id;

            // 检查是否到达目标
            if let Some(ctx) = self.states.get(&current_id) {
                if goal_check(&ctx.description) {
                    return Some(self.reconstruct_path(current_id));
                }
            }

            if self.closed_set.contains(&current_id) {
                continue;
            }
            self.closed_set.insert(current_id);

            // 扩展邻居
            if let Some(ctx) = self.states.get(&current_id) {
                let actions = expand_fn(ctx);
                for action in actions {
                    let new_ctx = StateContext {
                        description: format!("{} -> {}", ctx.description, action),
                        depth: ctx.depth + 1,
                        parent: Some(current_id),
                    };

                    let neighbor_id = StateId(self.states.len());
                    let tentative_g = current.g_cost + 1.0; // 假设单位代价

                    if tentative_g < *self.g_scores.get(&neighbor_id).unwrap_or(&f64::INFINITY) {
                        self.came_from.insert(neighbor_id, current_id);
                        self.g_scores.insert(neighbor_id, tentative_g);
                        let h = self.heuristic.evaluate(&new_ctx);
                        self.open_set.push(AStarNode::new(
                            neighbor_id,
                            tentative_g,
                            h,
                            Some(current_id),
                        ));
                        self.states.insert(neighbor_id, new_ctx);
                    }
                }
            }
        }

        None // 未找到路径
    }

    fn reconstruct_path(&self, mut current: StateId) -> Vec<StateId> {
        let mut path = vec![current];
        while let Some(&parent) = self.came_from.get(&current) {
            path.push(parent);
            current = parent;
        }
        path.reverse();
        path
    }
}

// =============================================================================
// 第四部分: LLM导航器与L2 Pattern层的集成
// =============================================================================

/// 与Pattern库集成的LLM导航器
/// 在L2 Pattern层使用LLM启发式选择设计模式
pub struct PatternNavigator<H: LLMHeuristic> {
    heuristic: H,
    available_patterns: Vec<&'static str>,
}

impl<H: LLMHeuristic> PatternNavigator<H> {
    pub fn new(heuristic: H) -> Self {
        PatternNavigator {
            heuristic,
            available_patterns: vec![
                "Builder",
                "Factory",
                "Strategy",
                "Observer",
                "Adapter",
            ],
        }
    }

    /// LLM导航器选择最佳模式
    /// 返回启发式值最优的模式
    pub fn navigate(&self, context: &str) -> &'static str {
        let mut best_pattern = self.available_patterns[0];
        let mut best_score = f64::INFINITY;

        for &pattern in &self.available_patterns {
            // 创建模拟状态上下文
            let state_ctx = StateContext {
                description: format!("Use {} pattern for {}", pattern, context),
                depth: 0,
                parent: None,
            };

            let score = self.heuristic.evaluate(&state_ctx);
            if score < best_score {
                best_score = score;
                best_pattern = pattern;
            }
        }

        best_pattern
    }

    /// 批量导航：一次性评估所有候选
    pub fn navigate_batch(&self, contexts: &[String]) -> Vec<&'static str> {
        contexts
            .iter()
            .map(|ctx| self.navigate(ctx))
            .collect()
    }
}

// =============================================================================
// 第五部分: Tree of Thoughts (ToT) 实现
// =============================================================================

/// ToT中的"Thought"节点
#[derive(Clone, Debug)]
pub struct Thought {
    content: String,
    value: f64,          // LLM评估的值
    visits: usize,       // 访问次数（用于MCTS）
}

/// Tree of Thoughts搜索
pub struct TreeOfThoughts<H: LLMHeuristic> {
    heuristic: H,
    thoughts: HashMap<StateId, Vec<Thought>>,
}

impl<H: LLMHeuristic> TreeOfThoughts<H> {
    pub fn new(heuristic: H) -> Self {
        TreeOfThoughts {
            heuristic,
            thoughts: HashMap::new(),
        }
    }

    /// BFS搜索：每层保留top-k个thought
    pub fn bfs_search(
        &mut self,
        initial: &str,
        generate_thoughts: impl Fn(&str) -> Vec<String>,
        evaluate_thought: impl Fn(&str) -> f64,
        k: usize,
        max_depth: usize,
    ) -> Option<String> {
        let mut current_level = vec![initial.to_string()];

        for depth in 0..max_depth {
            let mut candidates = Vec::new();

            // 生成所有候选thoughts
            for thought in &current_level {
                let new_thoughts = generate_thoughts(thought);
                for new_thought in new_thoughts {
                    let value = evaluate_thought(&new_thought);
                    candidates.push((new_thought, value));
                }
            }

            // 按LLM评估值排序，保留top-k
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
            candidates.truncate(k);

            // 检查是否到达目标
            for (thought, value) in &candidates {
                if *value < 0.1 {
                    // 接近目标
                    return Some(thought.clone());
                }
            }

            current_level = candidates.into_iter().map(|(t, _)| t).collect();
            println!("Depth {}: {} candidates", depth, current_level.len());
        }

        current_level.first().cloned()
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_heuristic() {
        let heuristic = SimulatedLLMHeuristic::new();

        let start_ctx = StateContext {
            description: "start state".to_string(),
            depth: 0,
            parent: None,
        };

        let goal_ctx = StateContext {
            description: "reach goal".to_string(),
            depth: 0,
            parent: None,
        };

        // 目标状态应该有更低的启发式值
        let h_start = heuristic.evaluate(&start_ctx);
        let h_goal = heuristic.evaluate(&goal_ctx);

        assert!(h_goal < h_start, "Goal should have lower heuristic value");
    }

    #[test]
    fn test_voting_heuristic() {
        let base = SimulatedLLMHeuristic::new();
        let voting = VotingLLMHeuristic::new(base, 3);

        let ctx = StateContext {
            description: "test state".to_string(),
            depth: 0,
            parent: None,
        };

        let score = voting.evaluate(&ctx);
        assert!(score >= 0.0 && score <= 2.0);
    }

    #[test]
    fn test_astar_search() {
        let heuristic = SimulatedLLMHeuristic::new();
        let mut search = LLMStarSearch::new(heuristic);

        let initial = State::<Initial>::new(0, "start");

        let goal_check = |desc: &str| desc.contains("goal");

        let expand = |ctx: &StateContext| {
            if ctx.depth < 3 {
                vec!["step1".to_string(), "step2".to_string()]
            } else {
                vec!["goal".to_string()]
            }
        };

        let result = search.search(initial, goal_check, expand);
        assert!(result.is_some(), "Should find a path to goal");
    }

    #[test]
    fn test_pattern_navigator() {
        let heuristic = SimulatedLLMHeuristic::new();
        let navigator = PatternNavigator::new(heuristic);

        // 测试导航器选择
        let pattern = navigator.navigate("building complex object");
        println!("Selected pattern: {}", pattern);

        // Builder模式应该适合"building"场景
        assert!(navigator.available_patterns.contains(&pattern));
    }

    #[test]
    fn test_tot_bfs() {
        let heuristic = SimulatedLLMHeuristic::new();
        let mut tot = TreeOfThoughts::new(heuristic);

        let generate = |thought: &str| {
            vec![
                format!("{} -> option A", thought),
                format!("{} -> option B", thought),
                format!("{} -> goal reached", thought),
            ]
        };

        let evaluate = |thought: &str| {
            if thought.contains("goal") {
                0.0
            } else {
                1.0
            }
        };

        let result = tot.bfs_search("start", generate, evaluate, 2, 5);
        assert!(result.is_some());
        assert!(result.unwrap().contains("goal"));
    }

    #[test]
    fn test_state_typestate() {
        // 正确流程: Initial -> Expanded -> Goal
        let initial = State::<Initial>::new(0, "start");
        let expanded = initial.expand("action1");

        // 检查是否到达目标
        let is_goal = expanded.check_goal(|desc| desc.contains("goal"));
        assert!(!is_goal);

        // 编译错误: 无法从Initial直接检查目标
        // let initial = State::<Initial>::new(0, "start");
        // initial.check_goal(|_| true); // ERROR!
    }
}

// =============================================================================
// 架构注释
// =============================================================================

/*
 * LLM导航器在状态空间架构中的角色:
 *
 * 1. **L2 Pattern层的启发式搜索**
 *    - LLM作为启发式函数评估Pattern选择
 *    - A*搜索确保路径有效性
 *    - Typestate保证状态转换的正确性
 *
 * 2. **与传统LLM生成的区别**
 *    ┌─────────────────┬──────────────────┬──────────────────┐
 *    │     维度        │   LLM生成        │   LLM导航        │
 *    ├─────────────────┼──────────────────┼──────────────────┤
 *    │ 正确性保证      │ 概率性           │ 可验证、可回溯   │
 *    │ 错误处理        │ 累积无法纠正     │ 局部可回退       │
 *    │ 解释性          │ 黑盒             │ 白盒(搜索轨迹)   │
 *    │ 计算成本        │ 低(单次)         │ 高(多次评估)     │
 *    │ 适用场景        │ 简单确定性任务   │ 复杂规划任务     │
 *    └─────────────────┴──────────────────┴──────────────────┘
 *
 * 3. **关键算法组件**
 *    - LLMHeuristic: 启发式函数接口
 *    - VotingLLMHeuristic: 自我一致性投票
 *    - LLMStarSearch: A* + LLM启发式
 *    - TreeOfThoughts: BFS/DFS搜索思维树
 *
 * 4. **理论问题**
 *    - LLM启发式的可采纳性：实践中使用相对排序而非绝对值
 *    - 复杂度权衡：搜索深度 vs 分支因子 vs 评估精度
 *    - 早停优化：找到解后立即停止，不必探索完整树
 *
 * 5. **与六层渐进式边界的结合**
 *    - L3 Typestate: 状态转换编译期保证
 *    - L2 Pattern: LLM在受限空间中选择
 *    - L1 Semantic: 类型安全的状态表示
 *    - L0 Syntax: 搜索轨迹的可验证编码
 */
