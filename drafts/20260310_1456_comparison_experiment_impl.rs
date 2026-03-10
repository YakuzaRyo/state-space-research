//! 软约束 vs 硬边界对比实验框架实现
//! Comparison Experiment: Soft Constraints vs Hard Boundaries
//!
//! 实验目标: 量化验证状态空间架构(硬边界)相对于传统AI编码工具(软约束)的优势
//!
//! 核心假设:
//! - H1: 硬边界方法在复杂任务成功率上显著高于软约束(>20%提升)
//! - H2: 硬边界方法的编译错误率显著低于软约束
//! - H3: 开发者在硬边界工具下的任务完成时间更短

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, StudentsT};
use statrs::statistics::Statistics;

// ============================================================================
// 1. 核心类型定义
// ============================================================================

/// 实验组别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExperimentGroup {
    /// A组: 软约束 (传统AI工具模式 - Prompt驱动)
    SoftConstraint,
    /// B组: 硬边界 (状态空间架构 - 类型约束)
    HardBoundary,
    /// C组: 混合模式 (软约束+硬边界)
    Hybrid,
}

impl ExperimentGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExperimentGroup::SoftConstraint => "SoftConstraint",
            ExperimentGroup::HardBoundary => "HardBoundary",
            ExperimentGroup::Hybrid => "Hybrid",
        }
    }
}

/// 任务复杂度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskComplexity {
    Simple,     // 简单任务 (HumanEval级别)
    Medium,     // 中等复杂度 (MBPP级别)
    Complex,    // 复杂任务 (SWE-Bench级别)
    VeryComplex, // 极复杂任务 (真实生产环境)
}

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    CodeGeneration,    // 代码生成
    CodeRepair,        // 代码修复
    CodeTranslation,   // 代码翻译
    Refactoring,       // 重构
    SecurityFix,       // 安全修复
}

/// 单个任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub complexity: TaskComplexity,
    pub prompt: String,
    pub test_cases: Vec<TestCase>,
    pub expected_output_schema: Option<TypeSchema>, // 硬边界组的类型约束
    pub time_limit: Duration,
}

/// 测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
    pub is_hidden: bool,
}

/// 类型约束定义 (用于硬边界组)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchema {
    pub input_types: Vec<String>,
    pub output_type: String,
    pub constraints: Vec<String>, // 额外的类型约束
}

// ============================================================================
// 2. 实验执行引擎
// ============================================================================

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub group: ExperimentGroup,
    pub participant_id: String,
    pub start_time: Instant,
    pub end_time: Instant,
    pub duration: Duration,
    pub generated_code: String,
    pub compilation_result: CompilationResult,
    pub test_results: Vec<TestResult>,
    pub success: bool, // 是否通过所有测试
    pub attempts: u32, // 尝试次数
    pub metadata: HashMap<String, String>,
}

/// 编译结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    pub success: bool,
    pub error_count: u32,
    pub warning_count: u32,
    pub error_types: Vec<ErrorType>,
    pub compilation_time: Duration,
}

/// 错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    SyntaxError { line: u32, message: String },
    TypeError { line: u32, message: String },
    SemanticError { line: u32, message: String },
    SecurityVulnerability { severity: Severity, description: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low, Medium, High, Critical,
}

/// 单个测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_case_id: String,
    pub passed: bool,
    pub actual_output: String,
    pub execution_time: Duration,
}

/// 任务执行器 trait
pub trait TaskExecutor {
    fn execute(&self, task: &Task, group: ExperimentGroup) -> TaskResult;
}

/// 软约束执行器 (模拟传统AI工具)
pub struct SoftConstraintExecutor {
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: usize,
}

impl TaskExecutor for SoftConstraintExecutor {
    fn execute(&self, task: &Task, group: ExperimentGroup) -> TaskResult {
        // 实现: 使用LLM生成代码，依赖Prompt约束
        // 特点: 无硬性类型约束，依赖模型理解
        todo!("Implement soft constraint execution")
    }
}

/// 硬边界执行器 (状态空间架构)
pub struct HardBoundaryExecutor {
    pub model_name: String,
    pub type_system: TypeSystem,
    pub constrained_decoder: ConstrainedDecoder,
}

/// 类型系统
pub struct TypeSystem {
    pub schema_registry: HashMap<String, TypeSchema>,
}

/// 约束解码器
pub struct ConstrainedDecoder {
    pub prefix_automaton: PrefixAutomaton,
    pub type_inhabitation_search: TypeInhabitationSearch,
}

/// 前缀自动机 (来自ETH Zurich论文)
pub struct PrefixAutomaton;

/// 类型 inhabitation 搜索
pub struct TypeInhabitationSearch;

impl TaskExecutor for HardBoundaryExecutor {
    fn execute(&self, task: &Task, group: ExperimentGroup) -> TaskResult {
        // 实现: 使用类型约束指导生成
        // 特点: 解码过程中强制执行类型约束
        todo!("Implement hard boundary execution")
    }
}

/// 混合执行器
pub struct HybridExecutor {
    pub soft_executor: SoftConstraintExecutor,
    pub hard_executor: HardBoundaryExecutor,
    pub constraint_threshold: f32, // 何时切换到硬约束
}

impl TaskExecutor for HybridExecutor {
    fn execute(&self, task: &Task, group: ExperimentGroup) -> TaskResult {
        // 实现: 先尝试软约束，失败时切换到硬约束
        todo!("Implement hybrid execution")
    }
}

// ============================================================================
// 3. 结果收集器
// ============================================================================

/// 实验结果收集器
pub struct ResultsCollector {
    pub results: Vec<TaskResult>,
    pub metrics: HashMap<ExperimentGroup, GroupMetrics>,
}

/// 组别指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupMetrics {
    pub total_tasks: u32,
    pub successful_tasks: u32,
    pub failed_tasks: u32,
    pub compilation_errors: u32,
    pub type_errors: u32,
    pub syntax_errors: u32,
    pub security_vulnerabilities: u32,
    pub total_duration: Duration,
    pub avg_duration: Duration,
    pub success_rate: f64,
    pub compilation_error_rate: f64,
}

impl ResultsCollector {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    pub fn add_result(&mut self, result: TaskResult) {
        self.results.push(result);
    }

    /// 计算各组指标
    pub fn compute_metrics(&mut self) {
        for group in [ExperimentGroup::SoftConstraint,
                      ExperimentGroup::HardBoundary,
                      ExperimentGroup::Hybrid] {
            let group_results: Vec<&TaskResult> = self.results
                .iter()
                .filter(|r| r.group == group)
                .collect();

            let total = group_results.len() as u32;
            let successful = group_results.iter().filter(|r| r.success).count() as u32;
            let compilation_errors: u32 = group_results
                .iter()
                .map(|r| if r.compilation_result.success { 0 } else { 1 })
                .sum();

            let total_duration: Duration = group_results
                .iter()
                .map(|r| r.duration)
                .fold(Duration::ZERO, |acc, d| acc + d);

            let avg_duration = if total > 0 {
                total_duration / total
            } else {
                Duration::ZERO
            };

            let metrics = GroupMetrics {
                total_tasks: total,
                successful_tasks: successful,
                failed_tasks: total - successful,
                compilation_errors,
                total_duration,
                avg_duration,
                success_rate: if total > 0 { successful as f64 / total as f64 } else { 0.0 },
                compilation_error_rate: if total > 0 { compilation_errors as f64 / total as f64 } else { 0.0 },
                ..Default::default()
            };

            self.metrics.insert(group, metrics);
        }
    }
}

// ============================================================================
// 4. 统计分析模块
// ============================================================================

/// 统计分析器
pub struct StatisticalAnalyzer;

/// 统计检验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTestResult {
    pub test_name: String,
    pub group_a: ExperimentGroup,
    pub group_b: ExperimentGroup,
    pub metric: String,
    pub mean_a: f64,
    pub mean_b: f64,
    pub std_a: f64,
    pub std_b: f64,
    pub n_a: usize,
    pub n_b: usize,
    pub t_statistic: f64,
    pub p_value: f64,
    pub cohens_d: f64,
    pub significant: bool, // p < 0.05
    pub effect_size_interpretation: EffectSizeInterpretation,
}

/// 效应量解释
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectSizeInterpretation {
    Negligible, // |d| < 0.2
    Small,      // 0.2 <= |d| < 0.5
    Medium,     // 0.5 <= |d| < 0.8
    Large,      // |d| >= 0.8
}

impl StatisticalAnalyzer {
    /// 独立样本t检验 (两组比较)
    pub fn independent_t_test(
        data_a: &[f64],
        data_b: &[f64],
        group_a: ExperimentGroup,
        group_b: ExperimentGroup,
        metric_name: &str,
    ) -> StatisticalTestResult {
        let n_a = data_a.len();
        let n_b = data_b.len();

        let mean_a = data_a.mean();
        let mean_b = data_b.mean();

        let std_a = data_a.std_dev();
        let std_b = data_b.std_dev();

        // 合并标准差 (假设方差齐性)
        let pooled_std = (((n_a - 1) as f64 * std_a.powi(2) +
                          (n_b - 1) as f64 * std_b.powi(2)) /
                         (n_a + n_b - 2) as f64)
            .sqrt();

        // t统计量
        let t_stat = (mean_a - mean_b) / (pooled_std * (1.0 / n_a as f64 + 1.0 / n_b as f64).sqrt());

        // 自由度
        let df = (n_a + n_b - 2) as f64;

        // p值 (双尾检验)
        let t_dist = StudentsT::new(0.0, 1.0, df).unwrap();
        let p_value = 2.0 * (1.0 - t_dist.cdf(t_stat.abs()));

        // Cohen's d (效应量)
        let cohens_d = (mean_a - mean_b) / pooled_std;

        let effect_interpretation = if cohens_d.abs() < 0.2 {
            EffectSizeInterpretation::Negligible
        } else if cohens_d.abs() < 0.5 {
            EffectSizeInterpretation::Small
        } else if cohens_d.abs() < 0.8 {
            EffectSizeInterpretation::Medium
        } else {
            EffectSizeInterpretation::Large
        };

        StatisticalTestResult {
            test_name: "Independent t-test".to_string(),
            group_a,
            group_b,
            metric: metric_name.to_string(),
            mean_a,
            mean_b,
            std_a,
            std_b,
            n_a,
            n_b,
            t_statistic: t_stat,
            p_value,
            cohens_d,
            significant: p_value < 0.05,
            effect_size_interpretation: effect_interpretation,
        }
    }

    /// 配对t检验 (同一参与者在不同条件下的比较)
    pub fn paired_t_test(
        paired_data: &[(f64, f64)], // (condition_a, condition_b)
        group_a: ExperimentGroup,
        group_b: ExperimentGroup,
        metric_name: &str,
    ) -> StatisticalTestResult {
        let n = paired_data.len();

        // 计算差异
        let differences: Vec<f64> = paired_data
            .iter()
            .map(|(a, b)| a - b)
            .collect();

        let mean_diff = differences.mean();
        let std_diff = differences.std_dev();

        // t统计量
        let t_stat = mean_diff / (std_diff / (n as f64).sqrt());

        // 自由度
        let df = (n - 1) as f64;

        // p值
        let t_dist = StudentsT::new(0.0, 1.0, df).unwrap();
        let p_value = 2.0 * (1.0 - t_dist.cdf(t_stat.abs()));

        // Cohen's d for paired samples
        let cohens_d = mean_diff / std_diff;

        // 计算原始均值
        let mean_a = paired_data.iter().map(|(a, _)| a).sum::<f64>() / n as f64;
        let mean_b = paired_data.iter().map(|(_, b)| b).sum::<f64>() / n as f64;

        let effect_interpretation = if cohens_d.abs() < 0.2 {
            EffectSizeInterpretation::Negligible
        } else if cohens_d.abs() < 0.5 {
            EffectSizeInterpretation::Small
        } else if cohens_d.abs() < 0.8 {
            EffectSizeInterpretation::Medium
        } else {
            EffectSizeInterpretation::Large
        };

        StatisticalTestResult {
            test_name: "Paired t-test".to_string(),
            group_a,
            group_b,
            metric: metric_name.to_string(),
            mean_a,
            mean_b,
            std_a: 0.0, // 配对检验关注差异
            std_b: 0.0,
            n_a: n,
            n_b: n,
            t_statistic: t_stat,
            p_value,
            cohens_d,
            significant: p_value < 0.05,
            effect_size_interpretation: effect_interpretation,
        }
    }

    /// ANOVA (三组及以上比较)
    pub fn one_way_anova(
        groups: &[&[f64]],
        group_labels: &[ExperimentGroup],
    ) -> AnovaResult {
        let k = groups.len(); // 组数
        let n_total: usize = groups.iter().map(|g| g.len()).sum();

        // 总均值
        let all_data: Vec<f64> = groups.iter().flat_map(|g| g.iter().copied()).collect();
        let grand_mean = all_data.mean();

        // 组间平方和 (SSB)
        let ssb: f64 = groups
            .iter()
            .map(|g| {
                let group_mean = g.mean();
                g.len() as f64 * (group_mean - grand_mean).powi(2)
            })
            .sum();

        // 组内平方和 (SSW)
        let ssw: f64 = groups
            .iter()
            .map(|g| {
                let group_mean = g.mean();
                g.iter().map(|x| (x - group_mean).powi(2)).sum::<f64>()
            })
            .sum();

        // 自由度
        let df_between = k - 1;
        let df_within = n_total - k;

        // 均方
        let msb = ssb / df_between as f64;
        let msw = ssw / df_within as f64;

        // F统计量
        let f_stat = msb / msw;

        AnovaResult {
            f_statistic: f_stat,
            p_value: 0.0, // 需要F分布计算
            df_between,
            df_within,
            ssb,
            ssw,
            msb,
            msw,
            significant: false, // 根据p值判断
        }
    }

    /// 功效分析 (Power Analysis)
    /// 计算检测给定效应量所需的样本量
    pub fn sample_size_for_power(
        effect_size: f64,      // Cohen's d
        alpha: f64,            // 显著性水平 (通常0.05)
        power: f64,            // 期望功效 (通常0.80)
    ) -> usize {
        // 简化公式: n = 2 * ((z_(1-alpha/2) + z_power) / effect_size)^2
        let z_alpha = 1.96; // 对应 alpha = 0.05 (双尾)
        let z_power = 0.84; // 对应 power = 0.80

        let n = 2.0 * ((z_alpha + z_power) / effect_size).powi(2);
        n.ceil() as usize
    }

    /// 置信区间计算
    pub fn confidence_interval(data: &[f64], confidence: f64) -> (f64, f64) {
        let n = data.len() as f64;
        let mean = data.mean();
        let std = data.std_dev();
        let se = std / n.sqrt();

        // z值对应置信水平
        let z = match confidence {
            0.90 => 1.645,
            0.95 => 1.96,
            0.99 => 2.576,
            _ => 1.96,
        };

        let margin = z * se;
        (mean - margin, mean + margin)
    }
}

/// ANOVA结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnovaResult {
    pub f_statistic: f64,
    pub p_value: f64,
    pub df_between: usize,
    pub df_within: usize,
    pub ssb: f64, // 组间平方和
    pub ssw: f64, // 组内平方和
    pub msb: f64, // 组间均方
    pub msw: f64, // 组内均方
    pub significant: bool,
}

// ============================================================================
// 5. 实验配置与运行
// ============================================================================

/// 实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub experiment_name: String,
    pub description: String,
    pub groups: Vec<ExperimentGroup>,
    pub tasks_per_group: usize,
    pub participants_per_group: usize,
    pub randomization_method: RandomizationMethod,
    pub significance_level: f64,
    pub desired_power: f64,
    pub min_effect_size: f64, // 最小可检测效应量
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RandomizationMethod {
    SimpleRandomization,
    BlockRandomization { block_size: usize },
    StratifiedRandomization { strata: Vec<String> },
}

/// 实验运行器
pub struct ExperimentRunner {
    pub config: ExperimentConfig,
    pub tasks: Vec<Task>,
    pub collectors: HashMap<ExperimentGroup, ResultsCollector>,
}

impl ExperimentRunner {
    pub fn new(config: ExperimentConfig, tasks: Vec<Task>) -> Self {
        let mut collectors = HashMap::new();
        for group in &config.groups {
            collectors.insert(*group, ResultsCollector::new());
        }

        Self {
            config,
            tasks,
            collectors,
        }
    }

    /// 运行完整实验
    pub fn run_experiment(&mut self) -> ExperimentReport {
        // 1. 随机分配任务到各组
        // 2. 执行每个任务
        // 3. 收集结果
        // 4. 统计分析
        todo!("Implement full experiment execution")
    }

    /// 生成任务集 (基于HumanEval/MBPP/SWE-Bench)
    pub fn generate_task_set(&self) -> Vec<Task> {
        // 包含不同复杂度等级的任务
        vec![
            // 简单任务 (HumanEval级别)
            Task {
                id: "HE_001".to_string(),
                name: "has_close_elements".to_string(),
                description: "Check if any two numbers in list are within threshold".to_string(),
                task_type: TaskType::CodeGeneration,
                complexity: TaskComplexity::Simple,
                prompt: "...".to_string(),
                test_cases: vec![],
                expected_output_schema: Some(TypeSchema {
                    input_types: vec!["List[float]".to_string(), "float".to_string()],
                    output_type: "bool".to_string(),
                    constraints: vec![],
                }),
                time_limit: Duration::from_secs(300),
            },
            // 更多任务...
        ]
    }
}

/// 实验报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentReport {
    pub config: ExperimentConfig,
    pub summary: ExperimentSummary,
    pub group_metrics: HashMap<ExperimentGroup, GroupMetrics>,
    pub statistical_tests: Vec<StatisticalTestResult>,
    pub hypothesis_results: Vec<HypothesisResult>,
    pub raw_data: Vec<TaskResult>,
}

/// 实验摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub total_tasks: usize,
    pub total_participants: usize,
    pub experiment_duration: Duration,
    pub overall_success_rate: f64,
}

/// 假设检验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisResult {
    pub hypothesis_id: String,
    pub description: String,
    pub prediction: String,
    pub result: HypothesisOutcome,
    pub supporting_evidence: Vec<String>,
    pub statistical_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HypothesisOutcome {
    Supported,
    Rejected,
    Inconclusive,
}

// ============================================================================
// 6. 预定义任务集 (基于真实基准测试)
// ============================================================================

/// HumanEval任务集 (164个任务)
pub const HUMANEVAL_TASK_COUNT: usize = 164;

/// MBPP任务集 (974个任务)
pub const MBPP_TASK_COUNT: usize = 974;

/// SWE-Bench任务集 (2294个任务)
pub const SWE_BENCH_TASK_COUNT: usize = 2294;

/// 基于Anthropic统计方法的最佳实践
pub mod statistical_best_practices {
    //! 统计方法最佳实践 (基于Anthropic研究 "A statistical approach to model evaluations")
    //!
    //! 核心推荐:
    //! 1. 使用中心极限定理计算SEM和95%置信区间
    //! 2. 聚类标准误 (对于相关问题组)
    //! 3. 减少问题内方差 (重采样或直接使用token概率)
    //! 4. 配对差异分析 (消除问题难度方差)
    //! 5. 功效分析 (确定所需样本量)

    use super::*;

    /// 计算标准误 (SEM)
    /// SEM = σ / √n
    pub fn standard_error(data: &[f64]) -> f64 {
        let n = data.len() as f64;
        let std = data.std_dev();
        std / n.sqrt()
    }

    /// 计算95%置信区间
    /// CI = M ± 1.96 * SEM
    pub fn confidence_interval_95(data: &[f64]) -> (f64, f64) {
        let mean = data.mean();
        let sem = standard_error(data);
        let margin = 1.96 * sem;
        (mean - margin, mean + margin)
    }

    /// 聚类标准误计算
    /// 适用于相关问题组 (如SWE-Bench的同一仓库多个问题)
    pub fn clustered_standard_error(
        cluster_means: &[f64],
        cluster_sizes: &[usize],
    ) -> f64 {
        let n_clusters = cluster_means.len() as f64;
        let overall_mean = cluster_means.mean();

        // 组间方差
        let between_variance: f64 = cluster_means
            .iter()
            .zip(cluster_sizes.iter())
            .map(|(mean, size)| {
                let diff = mean - overall_mean;
                diff.powi(2) * (*size as f64)
            })
            .sum();

        (between_variance / n_clusters).sqrt()
    }

    /// 配对差异检验
    /// 用于比较两个模型在同一组问题上的表现
    pub fn paired_difference_test(
        model_a_scores: &[f64],
        model_b_scores: &[f64],
    ) -> PairedTestResult {
        assert_eq!(model_a_scores.len(), model_b_scores.len());

        let differences: Vec<f64> = model_a_scores
            .iter()
            .zip(model_b_scores.iter())
            .map(|(a, b)| a - b)
            .collect();

        let mean_diff = differences.mean();
        let std_diff = differences.std_dev();
        let n = differences.len() as f64;

        // t统计量
        let t_stat = mean_diff / (std_diff / n.sqrt());

        // 效应量 (Cohen's d for paired samples)
        let cohens_d = mean_diff / std_diff;

        // Pearson相关系数
        let correlation = pearson_correlation(model_a_scores, model_b_scores);

        PairedTestResult {
            mean_difference: mean_diff,
            t_statistic: t_stat,
            cohens_d,
            correlation,
            n: n as usize,
        }
    }

    /// Pearson相关系数
    fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
        let n = x.len() as f64;
        let mean_x = x.mean();
        let mean_y = y.mean();

        let numerator: f64 = x.iter()
            .zip(y.iter())
            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
            .sum();

        let sum_sq_x: f64 = x.iter().map(|xi| (xi - mean_x).powi(2)).sum();
        let sum_sq_y: f64 = y.iter().map(|yi| (yi - mean_y).powi(2)).sum();

        numerator / (sum_sq_x.sqrt() * sum_sq_y.sqrt())
    }

    /// 功效分析: 计算所需样本量
    /// 基于期望效应量、显著性水平和功效
    pub fn power_analysis_sample_size(
        effect_size: f64,  // Cohen's d
        alpha: f64,        // 显著性水平 (通常0.05)
        power: f64,        // 期望功效 (通常0.80)
    ) -> usize {
        // z值
        let z_alpha = match alpha {
            0.10 => 1.645,
            0.05 => 1.96,
            0.01 => 2.576,
            _ => 1.96,
        };

        let z_power = match power {
            0.80 => 0.84,
            0.90 => 1.28,
            0.95 => 1.645,
            _ => 0.84,
        };

        // 每组所需样本量
        let n = 2.0 * ((z_alpha + z_power) / effect_size).powi(2);
        n.ceil() as usize
    }

    /// 配对检验结果
    #[derive(Debug, Clone)]
    pub struct PairedTestResult {
        pub mean_difference: f64,
        pub t_statistic: f64,
        pub cohens_d: f64,
        pub correlation: f64,
        pub n: usize,
    }

    /// 多重比较校正: Bonferroni方法
    pub fn bonferroni_correction(alpha: f64, num_comparisons: usize) -> f64 {
        alpha / num_comparisons as f64
    }

    /// 效应量解释
    pub fn interpret_effect_size(cohens_d: f64) -> &'static str {
        let abs_d = cohens_d.abs();
        if abs_d < 0.2 {
            "可忽略 (Negligible)"
        } else if abs_d < 0.5 {
            "小效应 (Small)"
        } else if abs_d < 0.8 {
            "中等效应 (Medium)"
        } else {
            "大效应 (Large)"
        }
    }
}

/// XGrammar风格约束解码实现参考
pub mod xgrammar_integration {
    //! XGrammar集成参考
    //!
    //! XGrammar核心概念:
    //! 1. 上下文无关文法 (CFG) 用于结构化生成
    //! 2. 词汇表分区: 上下文无关token vs 上下文相关token
    //! 3. 持久化栈结构加速运行时检查
    //! 4. 与LLM推理引擎协同设计

    use super::*;

    /// 上下文无关文法规则
    #[derive(Debug, Clone)]
    pub struct GrammarRule {
        pub non_terminal: String,
        pub production: Vec<String>,
    }

    /// 类型约束文法 (用于代码生成)
    pub struct TypeConstrainedGrammar {
        pub rules: Vec<GrammarRule>,
        pub start_symbol: String,
        pub type_constraints: TypeSchema,
    }

    /// 词汇表分区结果
    pub struct VocabularyPartition {
        /// 上下文无关token (可预检查)
        pub context_independent: Vec<usize>,
        /// 上下文相关token (需运行时检查)
        pub context_dependent: Vec<usize>,
    }

    impl TypeConstrainedGrammar {
        /// 分区词汇表 (XGrammar核心优化)
        pub fn partition_vocabulary(&self, vocabulary: &[String]) -> VocabularyPartition {
            let mut context_independent = Vec::new();
            let mut context_dependent = Vec::new();

            for (idx, token) in vocabulary.iter().enumerate() {
                if self.is_context_independent(token) {
                    context_independent.push(idx);
                } else {
                    context_dependent.push(idx);
                }
            }

            VocabularyPartition {
                context_independent,
                context_dependent,
            }
        }

        /// 判断token是否上下文无关
        fn is_context_independent(&self, token: &str) -> bool {
            // 关键字、基本类型通常是上下文无关的
            let context_independent_tokens = [
                "def", "class", "if", "else", "for", "while", "return",
                "int", "str", "bool", "float", "None", "True", "False",
            ];
            context_independent_tokens.contains(&token)
        }
    }

    /// 约束解码引擎接口
    pub trait ConstrainedDecoder {
        /// 根据当前状态和文法，获取允许的下一个token
        fn get_allowed_tokens(&self, current_state: &DecoderState) -> Vec<usize>;

        /// 更新解码器状态
        fn update_state(&mut self, token: usize);

        /// 检查是否完成
        fn is_complete(&self) -> bool;
    }

    /// 解码器状态
    #[derive(Debug, Clone)]
    pub struct DecoderState {
        pub stack: Vec<String>,
        pub generated_tokens: Vec<usize>,
        pub current_type: Option<String>,
    }
}

/// Praetorian风格确定性编排参考
pub mod praetorian_orchestration {
    //! Praetorian确定性编排架构参考
    //!
    //! 核心原则:
    //! 1. Thin Agent (<150行) + Fat Platform
    //! 2. 8层防御深度
    //! 3. 三层循环系统
    //! 4. 工具限制边界 (物理约束)

    use super::*;

    /// Agent定义 (Thin Agent规范)
    #[derive(Debug, Clone)]
    pub struct Agent {
        pub id: String,
        pub name: String,
        pub description: String,
        pub line_count: usize,  // 必须 < 150
        pub allowed_tools: Vec<Tool>,
        pub forbidden_tools: Vec<Tool>,
    }

    impl Agent {
        /// 验证Agent符合Thin Agent规范
        pub fn validate(&self) -> Result<(), String> {
            if self.line_count > 150 {
                return Err(format!(
                    "Agent {} exceeds 150 line limit: {}",
                    self.name, self.line_count
                ));
            }
            Ok(())
        }
    }

    /// 工具定义
    #[derive(Debug, Clone, PartialEq)]
    pub enum Tool {
        Task,       // 委派任务
        TodoWrite,  // 写待办
        Read,       // 读取文件
        Edit,       // 编辑文件
        Write,      // 写入文件
        Bash,       // 执行命令
    }

    /// 角色类型 (工具限制边界)
    #[derive(Debug, Clone)]
    pub enum Role {
        /// Orchestrator: 有Task/TodoWrite/Read，无Edit/Write
        Orchestrator,
        /// Worker: 有Edit/Write/Bash，无Task
        Worker,
    }

    impl Role {
        /// 获取允许的工具
        pub fn allowed_tools(&self) -> Vec<Tool> {
            match self {
                Role::Orchestrator => vec![
                    Tool::Task,
                    Tool::TodoWrite,
                    Tool::Read,
                ],
                Role::Worker => vec![
                    Tool::Edit,
                    Tool::Write,
                    Tool::Bash,
                    Tool::Read,
                ],
            }
        }

        /// 检查是否有权限使用工具
        pub fn can_use(&self, tool: &Tool) -> bool {
            self.allowed_tools().contains(tool)
        }
    }

    /// Hook类型 (8层防御)
    #[derive(Debug, Clone)]
    pub enum Hook {
        /// L4: UserPromptSubmit - 提示注入
        UserPromptSubmit,
        /// L5: PreToolUse - 动作前阻断
        PreToolUse,
        /// L6: PostToolUse - 结果验证
        PostToolUse,
        /// L7: SubagentStop - 退出拦截
        SubagentStop,
        /// L8: Stop - 最终质量门
        Stop,
    }

    /// 编排状态机 (16阶段模板)
    #[derive(Debug, Clone, PartialEq)]
    pub enum OrchestrationPhase {
        Setup,                // 1. 设置
        Triage,               // 2. 分类
        CodebaseDiscovery,    // 3. 代码库发现
        SkillDiscovery,       // 4. 技能发现
        Complexity,           // 5. 复杂度评估
        Brainstorming,        // 6. 头脑风暴
        ArchitectingPlan,     // 7. 架构规划
        Implementation,       // 8. 实现
        DesignVerification,   // 9. 设计验证
        DomainCompliance,     // 10. 领域合规
        CodeQuality,          // 11. 代码质量
        TestPlanning,         // 12. 测试规划
        Testing,              // 13. 测试
        CoverageVerification, // 14. 覆盖率验证
        TestQuality,          // 15. 测试质量
        Completion,           // 16. 完成
    }

    /// 工作流编排器
    pub struct WorkflowOrchestrator {
        pub current_phase: OrchestrationPhase,
        pub manifest: Manifest,
        pub context_usage: f64,  // 0.0 - 1.0
    }

    /// 工作流清单 (持久化状态)
    #[derive(Debug, Clone)]
    pub struct Manifest {
        pub feature_id: String,
        pub current_phase: OrchestrationPhase,
        pub active_agents: Vec<String>,
        pub validation_status: ValidationStatus,
    }

    #[derive(Debug, Clone)]
    pub enum ValidationStatus {
        Pending,
        InProgress,
        Passed,
        Failed,
    }

    impl WorkflowOrchestrator {
        /// 检查上下文压缩门限
        pub fn check_compaction_gate(&self) -> CompactionDecision {
            match self.context_usage {
                x if x < 0.75 => CompactionDecision::Proceed,
                x if x < 0.85 => CompactionDecision::Warning,
                _ => CompactionDecision::HardBlock,
            }
        }

        /// 智能阶段跳过
        pub fn should_skip_phase(&self, work_type: WorkType) -> bool {
            use WorkType::*;
            use OrchestrationPhase::*;

            match (work_type, &self.current_phase) {
                (BugFix, Complexity) |
                (BugFix, Brainstorming) |
                (BugFix, ArchitectingPlan) |
                (BugFix, DesignVerification) |
                (BugFix, TestPlanning) => true,
                (Small, Complexity) |
                (Small, Brainstorming) |
                (Small, ArchitectingPlan) |
                (Small, DesignVerification) => true,
                _ => false,
            }
        }
    }

    /// 压缩决策
    #[derive(Debug, Clone)]
    pub enum CompactionDecision {
        Proceed,
        Warning,
        HardBlock,
    }

    /// 工作类型
    #[derive(Debug, Clone)]
    pub enum WorkType {
        BugFix,      // Bug修复
        Small,       // 小改动 (<100行)
        Medium,      // 中等改动
        Large,       // 大改动 (新子系统)
    }

    /// 反馈循环状态 (运行时状态)
    #[derive(Debug, Clone)]
    pub struct FeedbackLoopState {
        pub dirty_bit: bool,           // 代码是否被修改
        pub tests_passed: bool,        // 测试是否通过
        pub review_passed: bool,       // 审查是否通过
        pub iteration_count: u32,      // 迭代次数
    }

    impl FeedbackLoopState {
        /// 检查是否可以退出
        pub fn can_exit(&self) -> bool {
            if !self.dirty_bit {
                return true;  // 没有修改，可以退出
            }
            // 有修改时，必须通过测试和审查
            self.tests_passed && self.review_passed
        }
    }
}

/// 创建分层任务集
pub fn create_stratified_task_set() -> Vec<Task> {
    let mut tasks = Vec::new();

    // 简单任务 (40%)
    for i in 0..40 {
        tasks.push(create_simple_task(i));
    }

    // 中等任务 (35%)
    for i in 0..35 {
        tasks.push(create_medium_task(i));
    }

    // 复杂任务 (20%)
    for i in 0..20 {
        tasks.push(create_complex_task(i));
    }

    // 极复杂任务 (5%)
    for i in 0..5 {
        tasks.push(create_very_complex_task(i));
    }

    tasks
}

fn create_simple_task(id: usize) -> Task {
    Task {
        id: format!("SIMPLE_{}", id),
        name: format!("Simple Task {}", id),
        description: "Entry-level programming problem".to_string(),
        task_type: TaskType::CodeGeneration,
        complexity: TaskComplexity::Simple,
        prompt: String::new(),
        test_cases: vec![],
        expected_output_schema: None,
        time_limit: Duration::from_secs(300),
    }
}

fn create_medium_task(id: usize) -> Task {
    Task {
        id: format!("MEDIUM_{}", id),
        name: format!("Medium Task {}", id),
        description: "Intermediate programming problem".to_string(),
        task_type: TaskType::CodeGeneration,
        complexity: TaskComplexity::Medium,
        prompt: String::new(),
        test_cases: vec![],
        expected_output_schema: None,
        time_limit: Duration::from_secs(600),
    }
}

fn create_complex_task(id: usize) -> Task {
    Task {
        id: format!("COMPLEX_{}", id),
        name: format!("Complex Task {}", id),
        description: "Complex software engineering task".to_string(),
        task_type: TaskType::CodeRepair,
        complexity: TaskComplexity::Complex,
        prompt: String::new(),
        test_cases: vec![],
        expected_output_schema: None,
        time_limit: Duration::from_secs(1800),
    }
}

fn create_very_complex_task(id: usize) -> Task {
    Task {
        id: format!("VERY_COMPLEX_{}", id),
        name: format!("Very Complex Task {}", id),
        description: "Production-level software engineering task".to_string(),
        task_type: TaskType::CodeRepair,
        complexity: TaskComplexity::VeryComplex,
        prompt: String::new(),
        test_cases: vec![],
        expected_output_schema: None,
        time_limit: Duration::from_secs(3600),
    }
}

// ============================================================================
// 7. 主函数与示例
// ============================================================================

fn main() {
    println!("软约束 vs 硬边界对比实验框架");
    println!("Soft Constraints vs Hard Boundaries Comparison Experiment");
    println!();

    // 实验配置
    let config = ExperimentConfig {
        experiment_name: "Soft vs Hard Boundary Experiment".to_string(),
        description: "Quantitative comparison of soft constraint vs hard boundary approaches".to_string(),
        groups: vec![
            ExperimentGroup::SoftConstraint,
            ExperimentGroup::HardBoundary,
            ExperimentGroup::Hybrid,
        ],
        tasks_per_group: 100,
        participants_per_group: 30,
        randomization_method: RandomizationMethod::BlockRandomization { block_size: 6 },
        significance_level: 0.05,
        desired_power: 0.80,
        min_effect_size: 0.5, // 中等效应量
    };

    // 计算所需样本量
    let required_n = StatisticalAnalyzer::sample_size_for_power(0.5, 0.05, 0.80);
    println!("Required sample size per group for medium effect size: {}", required_n);

    // 创建任务集
    let tasks = create_stratified_task_set();
    println!("Total tasks: {}", tasks.len());

    // 任务复杂度分布
    let simple_count = tasks.iter().filter(|t| matches!(t.complexity, TaskComplexity::Simple)).count();
    let medium_count = tasks.iter().filter(|t| matches!(t.complexity, TaskComplexity::Medium)).count();
    let complex_count = tasks.iter().filter(|t| matches!(t.complexity, TaskComplexity::Complex)).count();
    let very_complex_count = tasks.iter().filter(|t| matches!(t.complexity, TaskComplexity::VeryComplex)).count();

    println!("Task distribution:");
    println!("  Simple: {} ({}%)", simple_count, simple_count * 100 / tasks.len());
    println!("  Medium: {} ({}%)", medium_count, medium_count * 100 / tasks.len());
    println!("  Complex: {} ({}%)", complex_count, complex_count * 100 / tasks.len());
    println!("  Very Complex: {} ({}%)", very_complex_count, very_complex_count * 100 / tasks.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistical_analyzer() {
        let data_a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let data_b = vec![2.0, 3.0, 4.0, 5.0, 6.0];

        let result = StatisticalAnalyzer::independent_t_test(
            &data_a,
            &data_b,
            ExperimentGroup::SoftConstraint,
            ExperimentGroup::HardBoundary,
            "test_metric",
        );

        assert!(result.n_a == 5);
        assert!(result.n_b == 5);
    }

    #[test]
    fn test_sample_size_calculation() {
        let n = StatisticalAnalyzer::sample_size_for_power(0.5, 0.05, 0.80);
        assert!(n > 0);
        println!("Required sample size: {}", n);
    }
}
