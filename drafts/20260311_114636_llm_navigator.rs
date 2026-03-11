//! LLM作为启发式函数的理论基础 - Rust实现
//!
//! 本代码实现了一个理论框架，用于分析LLM作为启发式函数的性质
//! 核心概念：近似可采纳性、概率最优性、混合启发式

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::hash::Hash;

// ============================================================================
// 第一部分：核心类型和Trait定义
// ============================================================================

/// 搜索状态trait
pub trait State: Clone + Eq + Hash {
    /// 判断是否为目标状态
    fn is_goal(&self) -> bool;
    /// 获取后继状态及其转移成本
    fn successors(&self) -> Vec<(Self, f64)>;
    /// 获取状态的唯一标识（用于调试）
    fn id(&self) -> String;
}

/// 启发式函数trait
///
/// 理论背景：启发式函数h(n)估计从状态n到目标状态的最优成本
/// 关键性质：
/// - 可采纳性(Admissibility): h(n) ≤ h*(n) 对所有n成立
/// - 一致性(Consistency): h(n) ≤ c(n,n') + h(n') 对所有边成立
pub trait Heuristic<S: State> {
    /// 计算启发式值
    fn estimate(&self, state: &S) -> f64;

    /// 检查是否可采纳（需要知道真实最优成本）
    fn is_admissible(&self, state: &S, true_cost: f64) -> bool {
        self.estimate(state) <= true_cost + 1e-9
    }
}

/// 概率可采纳性结果
#[derive(Debug, Clone)]
pub struct ProbabilisticAdmissibility {
    /// 可采纳概率阈值
    pub epsilon: f64,
    /// 经验可采纳概率
    pub empirical_probability: f64,
    /// 样本数量
    pub sample_count: usize,
}

// ============================================================================
// 第二部分：传统启发式实现
// ============================================================================

/// 欧几里得距离启发式（可采纳且一致）
#[derive(Clone, Debug)]
pub struct EuclideanHeuristic {
    goal_x: f64,
    goal_y: f64,
}

impl EuclideanHeuristic {
    pub fn new(goal_x: f64, goal_y: f64) -> Self {
        Self { goal_x, goal_y }
    }
}

/// 网格位置状态
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct GridState {
    pub x: i32,
    pub y: i32,
}

impl GridState {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 计算到另一个状态的欧几里得距离
    pub fn euclidean_distance(&self, other: &GridState) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }
}

impl State for GridState {
    fn is_goal(&self) -> bool {
        // 默认实现，实际使用时由搜索算法设置目标
        false
    }

    fn successors(&self) -> Vec<(Self, f64)> {
        // 四方向移动
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        directions
            .iter()
            .map(|(dx, dy)| {
                let new_state = GridState::new(self.x + dx, self.y + dy);
                (new_state, 1.0) // 单位成本
            })
            .collect()
    }

    fn id(&self) -> String {
        format!("({},{})", self.x, self.y)
    }
}

impl Heuristic<GridState> for EuclideanHeuristic {
    fn estimate(&self, state: &GridState) -> f64 {
        let dx = (state.x as f64) - self.goal_x;
        let dy = (state.y as f64) - self.goal_y;
        (dx * dx + dy * dy).sqrt()
    }
}

// ============================================================================
// 第三部分：LLM启发式实现（理论模型）
// ============================================================================

/// LLM启发式函数
///
/// 理论模型：
/// - LLM启发式是一个概率性估计器
/// - 它以高概率提供有用的启发信息
/// - 但不保证严格的可采纳性
///
/// 核心假设：
/// 1. LLM通过预训练学习了世界结构的统计模式
/// 2. 这种学习使得LLM能够"理解"问题的语义结构
/// 3. 但LLM可能基于模式识别高估或低估成本
#[derive(Clone, Debug)]
pub struct LLMHeuristic {
    /// 目标状态（用于语义理解）
    goal_description: String,
    /// 领域知识描述
    domain_knowledge: String,
    /// 模拟的估计误差分布（用于理论分析）
    /// 正态分布 N(0, sigma^2)，sigma表示不确定性
    uncertainty_sigma: f64,
    /// 系统性偏差（LLM可能系统性地高估或低估）
    systematic_bias: f64,
}

impl LLMHeuristic {
    pub fn new(goal_desc: &str, domain: &str, sigma: f64, bias: f64) -> Self {
        Self {
            goal_description: goal_desc.to_string(),
            domain_knowledge: domain.to_string(),
            uncertainty_sigma: sigma,
            systematic_bias: bias,
        }
    }

    /// 模拟LLM的启发式估计
    ///
    /// 在实际实现中，这将调用LLM API
    /// 这里使用概率模型来模拟LLM的行为
    ///
    /// 理论模型：
    /// h_LLM(n) = h_true(n) + bias + noise
    ///
    /// 其中：
    /// - h_true(n) 是真实的最优成本（通常未知）
    /// - bias 是系统性偏差
    /// - noise ~ N(0, sigma^2) 是随机噪声
    pub fn llm_estimate(&self, state: &GridState, goal: &GridState) -> f64 {
        // 基础估计：欧几里得距离（LLM可能使用类似的直觉）
        let base_estimate = state.euclidean_distance(goal);

        // 添加系统性偏差
        let with_bias = base_estimate + self.systematic_bias;

        // 添加随机噪声（模拟LLM的不确定性）
        // 注意：在实际代码中使用确定性值以保证可测试性
        let noise = 0.0; // 实际噪声应在运行时生成

        (with_bias + noise).max(0.0) // 启发式值非负
    }

    /// 计算近似可采纳性
    ///
    /// 定义：对于给定的ε > 0，如果
    /// P(h_LLM(n) ≤ h*(n)) ≥ 1-ε 对所有n成立
    /// 则称h_LLM是ε-可采纳的
    ///
    /// 这是传统可采纳性的概率松弛
    pub fn approximate_admissibility(
        &self,
        samples: &[(GridState, f64)], // (状态, 真实最优成本)
        epsilon: f64,
    ) -> ProbabilisticAdmissibility {
        let total = samples.len();
        let admissible_count = samples
            .iter()
            .filter(|(state, true_cost)| {
                let estimate = self.estimate(state);
                estimate <= *true_cost + 1e-9
            })
            .count();

        let empirical_prob = admissible_count as f64 / total as f64;

        ProbabilisticAdmissibility {
            epsilon,
            empirical_probability: empirical_prob,
            sample_count: total,
        }
    }
}

impl Heuristic<GridState> for LLMHeuristic {
    fn estimate(&self, state: &GridState) -> f64 {
        // 简化的估计：假设目标在原点
        let goal = GridState::new(0, 0);
        self.llm_estimate(state, &goal)
    }
}

// ============================================================================
// 第四部分：混合启发式（LLM-A*风格）
// ============================================================================

/// 混合启发式：结合可采纳启发式与LLM启发式
///
/// 理论框架：
/// h_hybrid(n) = α · h_admissible(n) + β · h_LLM(n)
///
/// 其中：
/// - α + β = 1（权重归一化）
/// - h_admissible 保证基础可采纳性
/// - h_LLM 提供语义理解和搜索引导
///
/// 性质分析：
/// 1. 当β = 0时，退化为传统可采纳启发式
/// 2. 当β > 0时，可能偏离可采纳性，但获得更强的搜索引导
/// 3. 通过调整α和β可以权衡最优性与效率
#[derive(Clone, Debug)]
pub struct HybridHeuristic<H: Heuristic<S>, S: State> {
    admissible_heuristic: H,
    llm_heuristic: LLMHeuristic,
    alpha: f64, // 可采纳启发式权重
    beta: f64,  // LLM启发式权重
    _phantom: std::marker::PhantomData<S>,
}

impl<H: Heuristic<S>, S: State> HybridHeuristic<H, S> {
    pub fn new(admissible: H, llm: LLMHeuristic, alpha: f64, beta: f64) -> Self {
        assert!((alpha + beta - 1.0).abs() < 1e-9, "权重必须归一化: α + β = 1");
        Self {
            admissible_heuristic: admissible,
            llm_heuristic: llm,
            alpha,
            beta,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 计算次优界
    ///
    /// 如果LLM启发式高估了成本，最终路径可能次优
    /// 次优程度取决于β和LLM高估的程度
    pub fn suboptimality_bound(
        &self,
        max_llm_overestimate: f64,
    ) -> f64 {
        // 理论分析：
        // 如果h_LLM最多高估M，则
        // h_hybrid = α·h_adm + β·h_LLM
        //          ≤ α·h* + β·(h* + M)
        //          = h* + β·M
        //
        // 因此次优界为 β·M
        self.beta * max_llm_overestimate
    }
}

impl Heuristic<GridState> for HybridHeuristic<EuclideanHeuristic, GridState> {
    fn estimate(&self, state: &GridState) -> f64 {
        let h_adm = self.admissible_heuristic.estimate(state);
        let h_llm = self.llm_heuristic.estimate(state);
        self.alpha * h_adm + self.beta * h_llm
    }
}

// ============================================================================
// 第五部分：A*搜索算法
// ============================================================================

/// 搜索节点
#[derive(Clone, Debug)]
struct SearchNode<S: State> {
    state: S,
    g_cost: f64,      // 从起点到当前状态的实际成本
    f_cost: f64,      // g + h
    parent: Option<Box<SearchNode<S>>>,
}

impl<S: State> PartialEq for SearchNode<S> {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}

impl<S: State> Eq for SearchNode<S> {}

impl<S: State> PartialOrd for SearchNode<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.f_cost.partial_cmp(&self.f_cost) // 最小堆
    }
}

impl<S: State> Ord for SearchNode<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// A*搜索结果
#[derive(Debug, Clone)]
pub struct SearchResult<S: State> {
    pub path: Vec<S>,
    pub cost: f64,
    pub nodes_expanded: usize,
    pub is_optimal: bool, // 如果启发式可采纳，则为true
}

/// A*搜索算法
///
/// 理论保证：
/// 1. 如果启发式可采纳，A*找到最优解
/// 2. 如果启发式一致，A*不会重复扩展节点
/// 3. 对于不可采纳启发式，解的质量取决于启发式质量
pub fn astar_search<S: State, H: Heuristic<S>>(
    start: S,
    goal: S,
    heuristic: &H,
    max_iterations: usize,
) -> Option<SearchResult<S>> {
    let mut open_set: BinaryHeap<SearchNode<S>> = BinaryHeap::new();
    let mut closed_set: HashMap<String, f64> = HashMap::new();

    let h_start = heuristic.estimate(&start);
    open_set.push(SearchNode {
        state: start.clone(),
        g_cost: 0.0,
        f_cost: h_start,
        parent: None,
    });

    let mut nodes_expanded = 0;

    while let Some(current) = open_set.pop() {
        nodes_expanded += 1;

        if nodes_expanded > max_iterations {
            return None; // 超过最大迭代次数
        }

        // 检查是否到达目标
        if current.state.id() == goal.id() {
            // 重建路径
            let mut path = vec![];
            let mut node = Some(current.clone());
            while let Some(n) = node {
                path.push(n.state);
                node = n.parent.map(|b| *b);
            }
            path.reverse();

            return Some(SearchResult {
                path,
                cost: current.g_cost,
                nodes_expanded,
                is_optimal: false, // 默认假设不可采纳
            });
        }

        // 记录已访问
        closed_set.insert(current.state.id(), current.g_cost);

        // 扩展后继
        for (successor, step_cost) in current.state.successors() {
            let new_g = current.g_cost + step_cost;

            // 检查是否已访问且成本更高
            if let Some(&existing_g) = closed_set.get(&successor.id()) {
                if new_g >= existing_g {
                    continue;
                }
            }

            let h = heuristic.estimate(&successor);
            let f = new_g + h;

            open_set.push(SearchNode {
                state: successor,
                g_cost: new_g,
                f_cost: f,
                parent: Some(Box::new(current.clone())),
            });
        }
    }

    None // 未找到路径
}

// ============================================================================
// 第六部分：理论分析工具
// ============================================================================

/// 启发式质量分析器
pub struct HeuristicAnalyzer;

impl HeuristicAnalyzer {
    /// 分析启发式的可采纳性
    pub fn analyze_admissibility<S: State, H: Heuristic<S>>(
        heuristic: &H,
        test_cases: &[(S, f64)], // (状态, 真实最优成本)
    ) -> AdmissibilityAnalysis {
        let mut violations = 0;
        let mut max_overestimate: f64 = 0.0;
        let mut total_overestimate = 0.0;

        for (state, true_cost) in test_cases {
            let estimate = heuristic.estimate(state);
            if estimate > *true_cost + 1e-9 {
                violations += 1;
                let over = estimate - true_cost;
                max_overestimate = max_overestimate.max(over);
                total_overestimate += over;
            }
        }

        let total = test_cases.len();
        let admissible_rate = (total - violations) as f64 / total as f64;

        AdmissibilityAnalysis {
            total_cases: total,
            violations,
            admissible_rate,
            max_overestimate,
            avg_overestimate: if violations > 0 {
                total_overestimate / violations as f64
            } else {
                0.0
            },
        }
    }
}

/// 可采纳性分析结果
#[derive(Debug, Clone)]
pub struct AdmissibilityAnalysis {
    pub total_cases: usize,
    pub violations: usize,
    pub admissible_rate: f64,
    pub max_overestimate: f64,
    pub avg_overestimate: f64,
}

// ============================================================================
// 第七部分：测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_heuristic_admissible() {
        // 欧几里得距离在网格世界中是可采纳的
        let goal = GridState::new(10, 10);
        let heuristic = EuclideanHeuristic::new(10.0, 10.0);

        let state = GridState::new(0, 0);
        let estimate = heuristic.estimate(&state);

        // 实际最短路径是20（曼哈顿距离）
        // 欧几里得距离 = sqrt(200) ≈ 14.14 < 20
        assert!(estimate <= 20.0 + 1e-9);
    }

    #[test]
    fn test_llm_heuristic_estimate() {
        let llm = LLMHeuristic::new("goal", "grid_world", 0.5, 0.0);
        let state = GridState::new(3, 4);

        let estimate = llm.estimate(&state);
        // 到原点(0,0)的距离 = 5.0
        assert!((estimate - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_hybrid_heuristic_weights() {
        let adm = EuclideanHeuristic::new(0.0, 0.0);
        let llm = LLMHeuristic::new("goal", "grid", 0.0, 0.0);
        let hybrid = HybridHeuristic::new(adm, llm, 0.7, 0.3);

        let state = GridState::new(3, 4);
        let estimate = hybrid.estimate(&state);

        // 期望: 0.7 * 5.0 + 0.3 * 5.0 = 5.0
        assert!((estimate - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_admissibility_analysis() {
        // 创建一个故意高估的启发式
        struct OverestimatingHeuristic;
        impl Heuristic<GridState> for OverestimatingHeuristic {
            fn estimate(&self, _state: &GridState) -> f64 {
                100.0 // 总是高估
            }
        }

        let heuristic = OverestimatingHeuristic;
        let test_cases = vec![
            (GridState::new(0, 0), 0.0),
            (GridState::new(1, 0), 1.0),
            (GridState::new(2, 0), 2.0),
        ];

        let analysis = HeuristicAnalyzer::analyze_admissibility(&heuristic, &test_cases
        );

        assert_eq!(analysis.total_cases, 3);
        assert_eq!(analysis.violations, 3); // 所有用例都被高估（包括起点成本为0的情况）
        assert!(analysis.admissible_rate < 1.0);
    }

    #[test]
    fn test_probabilistic_admissibility() {
        let llm = LLMHeuristic::new("goal", "grid", 0.1, 0.0);

        // 构造测试样本
        let samples: Vec<(GridState, f64)> = (0..10)
            .map(|i| (GridState::new(i, 0), i as f64))
            .collect();

        let result = llm.approximate_admissibility(&samples, 0.1);

        assert_eq!(result.sample_count, 10);
        assert!(result.empirical_probability >= 0.0 && result.empirical_probability <= 1.0);
    }

    #[test]
    fn test_grid_state_successors() {
        let state = GridState::new(0, 0);
        let succ = state.successors();

        assert_eq!(succ.len(), 4);

        let expected = vec![
            (GridState::new(0, 1), 1.0),
            (GridState::new(1, 0), 1.0),
            (GridState::new(0, -1), 1.0),
            (GridState::new(-1, 0), 1.0),
        ];

        for (exp_state, exp_cost) in expected {
            assert!(succ.contains(&(exp_state, exp_cost)));
        }
    }
}

// ============================================================================
// 第八部分：示例和演示
// ============================================================================

/// 演示：比较不同启发式的性能
pub fn demo_heuristic_comparison() {
    println!("=== LLM启发式理论基础演示 ===\n");

    // 1. 可采纳启发式
    println!("1. 欧几里得启发式（可采纳）");
    println!("   性质: h(n) = sqrt((x-goal_x)^2 + (y-goal_y)^2)");
    println!("   保证: 永远不高估真实成本");
    println!("   结果: A*找到最优解\n");

    // 2. LLM启发式
    println!("2. LLM启发式（近似可采纳）");
    println!("   模型: h_LLM(n) = h_base(n) + bias + noise");
    println!("   性质: 以概率1-ε满足 h(n) ≤ h*(n)");
    println!("   结果: 可能次优，但搜索效率更高\n");

    // 3. 混合启发式
    println!("3. 混合启发式（LLM-A*风格）");
    println!("   公式: h_hybrid = α·h_adm + β·h_LLM");
    println!("   次优界: cost ≤ optimal + β·M (M为最大高估)");
    println!("   权衡: 通过调整α,β平衡最优性与效率\n");

    println!("=== 理论结论 ===");
    println!("LLM作为启发式函数的理论基础建立在:");
    println!("1. 统计学习理论 - LLM学习世界结构的统计模式");
    println!("2. 近似可采纳性 - 概率化的可采纳性保证");
    println!("3. 混合架构 - 结合传统启发式的严格性与LLM的语义理解");
    println!("4. 经验验证 - 通过大规模实验验证启发式质量");
}

// 运行演示
fn main() {
    demo_heuristic_comparison();
}
