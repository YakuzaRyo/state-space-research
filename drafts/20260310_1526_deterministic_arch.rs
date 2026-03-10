// ============================================================================
// 确定性架构实现: Thin Agent + Fat Platform + Gateway模式 + 16阶段状态机
// 文件名: 20260310_1526_deterministic_arch.rs
// 研究方向: 04_deterministic_arch
// ============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

// =============================================================================
// 第一部分: Gateway路由器 (Gateway Pattern)
// =============================================================================

/// Gateway路由器 - 动态路由技能请求到确定性处理模块
/// 这是Fat Platform的入口点，隔离LLM非确定性
#[derive(Debug, Clone)]
pub struct GatewayRouter {
    /// 已注册的技能路由表
    routes: HashMap<String, SkillHandler>,
    /// 意图检测器
    intent_classifier: IntentClassifier,
    /// 执行统计
    metrics: Arc<Mutex<GatewayMetrics>>,
}

/// 技能处理器类型
pub type SkillHandler = Box<dyn Fn(SkillRequest) -> SkillResponse + Send + Sync>;

/// 技能请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequest {
    pub intent: String,
    pub payload: serde_json::Value,
    pub context: ExecutionContext,
}

/// 技能响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub execution_time_ms: u64,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub session_id: String,
    pub phase: ExecutionPhase,
    pub token_budget: TokenBudget,
}

/// Token预算管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub max_tokens: usize,
    pub used_tokens: usize,
}

/// Gateway指标
#[derive(Debug, Default)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub avg_latency_ms: f64,
}

impl GatewayRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            intent_classifier: IntentClassifier::new(),
            metrics: Arc::new(Mutex::new(GatewayMetrics::default())),
        }
    }

    /// 注册技能路由
    pub fn register_route<F>(&mut self, intent_pattern: &str, handler: F)
    where
        F: Fn(SkillRequest) -> SkillResponse + Send + Sync + 'static,
    {
        self.routes.insert(intent_pattern.to_string(), Box::new(handler));
    }

    /// 路由请求到对应的技能处理器
    /// 这是Gateway模式的核心 - 确定性路由，不依赖LLM决策
    pub fn route(&self, request: SkillRequest) -> Result<SkillResponse, GatewayError> {
        let start = Instant::now();

        // 1. 意图识别 (确定性规则匹配，非LLM)
        let intent = self.intent_classifier.classify(&request.intent);

        // 2. 查找路由
        let handler = self.routes
            .get(&intent)
            .ok_or(GatewayError::RouteNotFound(intent.clone()))?;

        // 3. 执行处理
        let mut response = handler(request);
        response.execution_time_ms = start.elapsed().as_millis() as u64;

        // 4. 更新指标
        self.update_metrics(&response);

        Ok(response)
    }

    fn update_metrics(&self, response: &SkillResponse) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_requests += 1;
            if response.success {
                metrics.successful_routes += 1;
            } else {
                metrics.failed_routes += 1;
            }
            // 更新平均延迟
            let alpha = 0.1; // 指数移动平均
            metrics.avg_latency_ms = metrics.avg_latency_ms * (1.0 - alpha)
                + response.execution_time_ms as f64 * alpha;
        }
    }
}

#[derive(Debug)]
pub enum GatewayError {
    RouteNotFound(String),
    ExecutionFailed(String),
}

// =============================================================================
// 第二部分: 意图分类器 (确定性意图识别)
// =============================================================================

/// 意图分类器 - 基于规则的确定性分类，避免LLM非确定性
#[derive(Debug, Clone)]
pub struct IntentClassifier {
    patterns: Vec<(String, Vec<String>)>, // (intent_name, keywords)
}

impl IntentClassifier {
    pub fn new() -> Self {
        let mut classifier = Self { patterns: Vec::new() };

        // 预定义意图模式 - 确定性匹配
        classifier.add_pattern("frontend", vec!["react", "vue", "angular", "css", "html", "dom"]);
        classifier.add_pattern("backend", vec!["api", "server", "database", "sql", "rest", "grpc"]);
        classifier.add_pattern("testing", vec!["test", "jest", "pytest", "unit", "integration", "e2e"]);
        classifier.add_pattern("security", vec!["auth", "jwt", "oauth", "encrypt", "vulnerability"]);
        classifier.add_pattern("deployment", vec!["deploy", "docker", "k8s", "kubernetes", "ci/cd"]);
        classifier.add_pattern("debugging", vec!["bug", "error", "fix", "debug", "trace", "log"]);

        classifier
    }

    fn add_pattern(&mut self, intent: &str, keywords: Vec<&str>) {
        self.patterns.push((
            intent.to_string(),
            keywords.iter().map(|s| s.to_string()).collect()
        ));
    }

    /// 确定性分类 - 基于关键词匹配，无LLM参与
    pub fn classify(&self, input: &str) -> String {
        let input_lower = input.to_lowercase();

        for (intent, keywords) in &self.patterns {
            for keyword in keywords {
                if input_lower.contains(keyword) {
                    return intent.clone();
                }
            }
        }

        "general".to_string() // 默认意图
    }
}

// =============================================================================
// 第三部分: 16阶段状态机 (16-Phase State Machine)
// =============================================================================

/// 16阶段执行状态机 - 覆盖所有执行路径的确定性工作流
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionPhase {
    Phase1_Setup,              // 工作区创建、输出目录、MANIFEST
    Phase2_Triage,             // 分类工作类型，选择执行阶段
    Phase3_CodebaseDiscovery,  // 代码库发现，探索模式
    Phase4_SkillDiscovery,     // 技能映射
    Phase5_Complexity,         // 技术评估、执行策略
    Phase6_Brainstorming,      // 设计细化（人机协作）
    Phase7_ArchitectingPlan,   // 技术设计 + 任务分解
    Phase8_Implementation,     // 代码开发
    Phase9_DesignVerification, // 验证实现符合设计
    Phase10_DomainCompliance,  // 领域特定强制模式
    Phase11_CodeQuality,       // 代码审查
    Phase12_TestPlanning,      // 测试策略
    Phase13_Testing,           // 测试实现与执行
    Phase14_CoverageVerification, // 覆盖率验证
    Phase15_TestQuality,       // 测试质量检查
    Phase16_Completion,        // 最终验证、PR、清理
}

impl ExecutionPhase {
    /// 获取阶段名称
    pub fn name(&self) -> &'static str {
        match self {
            ExecutionPhase::Phase1_Setup => "Setup",
            ExecutionPhase::Phase2_Triage => "Triage",
            ExecutionPhase::Phase3_CodebaseDiscovery => "CodebaseDiscovery",
            ExecutionPhase::Phase4_SkillDiscovery => "SkillDiscovery",
            ExecutionPhase::Phase5_Complexity => "Complexity",
            ExecutionPhase::Phase6_Brainstorming => "Brainstorming",
            ExecutionPhase::Phase7_ArchitectingPlan => "ArchitectingPlan",
            ExecutionPhase::Phase8_Implementation => "Implementation",
            ExecutionPhase::Phase9_DesignVerification => "DesignVerification",
            ExecutionPhase::Phase10_DomainCompliance => "DomainCompliance",
            ExecutionPhase::Phase11_CodeQuality => "CodeQuality",
            ExecutionPhase::Phase12_TestPlanning => "TestPlanning",
            ExecutionPhase::Phase13_Testing => "Testing",
            ExecutionPhase::Phase14_CoverageVerification => "CoverageVerification",
            ExecutionPhase::Phase15_TestQuality => "TestQuality",
            ExecutionPhase::Phase16_Completion => "Completion",
        }
    }

    /// 获取阶段序号 (1-16)
    pub fn number(&self) -> u8 {
        match self {
            ExecutionPhase::Phase1_Setup => 1,
            ExecutionPhase::Phase2_Triage => 2,
            ExecutionPhase::Phase3_CodebaseDiscovery => 3,
            ExecutionPhase::Phase4_SkillDiscovery => 4,
            ExecutionPhase::Phase5_Complexity => 5,
            ExecutionPhase::Phase6_Brainstorming => 6,
            ExecutionPhase::Phase7_ArchitectingPlan => 7,
            ExecutionPhase::Phase8_Implementation => 8,
            ExecutionPhase::Phase9_DesignVerification => 9,
            ExecutionPhase::Phase10_DomainCompliance => 10,
            ExecutionPhase::Phase11_CodeQuality => 11,
            ExecutionPhase::Phase12_TestPlanning => 12,
            ExecutionPhase::Phase13_Testing => 13,
            ExecutionPhase::Phase14_CoverageVerification => 14,
            ExecutionPhase::Phase15_TestQuality => 15,
            ExecutionPhase::Phase16_Completion => 16,
        }
    }

    /// 获取下一个阶段
    pub fn next(&self) -> Option<ExecutionPhase> {
        match self {
            ExecutionPhase::Phase1_Setup => Some(ExecutionPhase::Phase2_Triage),
            ExecutionPhase::Phase2_Triage => Some(ExecutionPhase::Phase3_CodebaseDiscovery),
            ExecutionPhase::Phase3_CodebaseDiscovery => Some(ExecutionPhase::Phase4_SkillDiscovery),
            ExecutionPhase::Phase4_SkillDiscovery => Some(ExecutionPhase::Phase5_Complexity),
            ExecutionPhase::Phase5_Complexity => Some(ExecutionPhase::Phase6_Brainstorming),
            ExecutionPhase::Phase6_Brainstorming => Some(ExecutionPhase::Phase7_ArchitectingPlan),
            ExecutionPhase::Phase7_ArchitectingPlan => Some(ExecutionPhase::Phase8_Implementation),
            ExecutionPhase::Phase8_Implementation => Some(ExecutionPhase::Phase9_DesignVerification),
            ExecutionPhase::Phase9_DesignVerification => Some(ExecutionPhase::Phase10_DomainCompliance),
            ExecutionPhase::Phase10_DomainCompliance => Some(ExecutionPhase::Phase11_CodeQuality),
            ExecutionPhase::Phase11_CodeQuality => Some(ExecutionPhase::Phase12_TestPlanning),
            ExecutionPhase::Phase12_TestPlanning => Some(ExecutionPhase::Phase13_Testing),
            ExecutionPhase::Phase13_Testing => Some(ExecutionPhase::Phase14_CoverageVerification),
            ExecutionPhase::Phase14_CoverageVerification => Some(ExecutionPhase::Phase15_TestQuality),
            ExecutionPhase::Phase15_TestQuality => Some(ExecutionPhase::Phase16_Completion),
            ExecutionPhase::Phase16_Completion => None,
        }
    }
}

/// 工作类型 - 决定哪些阶段可以跳过
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkType {
    BugFix,    // 跳过: 5, 6, 7, 9, 12
    Small,     // 跳过: 5, 6, 7, 9
    Medium,    // 执行所有阶段
    Large,     // 执行所有阶段（更严格）
}

impl WorkType {
    /// 根据工作类型获取需要执行的阶段
    pub fn get_phases(&self) -> Vec<ExecutionPhase> {
        let all_phases = vec![
            ExecutionPhase::Phase1_Setup,
            ExecutionPhase::Phase2_Triage,
            ExecutionPhase::Phase3_CodebaseDiscovery,
            ExecutionPhase::Phase4_SkillDiscovery,
            ExecutionPhase::Phase5_Complexity,
            ExecutionPhase::Phase6_Brainstorming,
            ExecutionPhase::Phase7_ArchitectingPlan,
            ExecutionPhase::Phase8_Implementation,
            ExecutionPhase::Phase9_DesignVerification,
            ExecutionPhase::Phase10_DomainCompliance,
            ExecutionPhase::Phase11_CodeQuality,
            ExecutionPhase::Phase12_TestPlanning,
            ExecutionPhase::Phase13_Testing,
            ExecutionPhase::Phase14_CoverageVerification,
            ExecutionPhase::Phase15_TestQuality,
            ExecutionPhase::Phase16_Completion,
        ];

        match self {
            WorkType::BugFix => {
                // BugFix: 跳过 5, 6, 7, 9, 12
                all_phases.into_iter()
                    .filter(|p| !matches!(p,
                        ExecutionPhase::Phase5_Complexity |
                        ExecutionPhase::Phase6_Brainstorming |
                        ExecutionPhase::Phase7_ArchitectingPlan |
                        ExecutionPhase::Phase9_DesignVerification |
                        ExecutionPhase::Phase12_TestPlanning
                    ))
                    .collect()
            }
            WorkType::Small => {
                // Small: 跳过 5, 6, 7, 9
                all_phases.into_iter()
                    .filter(|p| !matches!(p,
                        ExecutionPhase::Phase5_Complexity |
                        ExecutionPhase::Phase6_Brainstorming |
                        ExecutionPhase::Phase7_ArchitectingPlan |
                        ExecutionPhase::Phase9_DesignVerification
                    ))
                    .collect()
            }
            WorkType::Medium | WorkType::Large => all_phases,
        }
    }
}

/// 状态机执行器
pub struct StateMachineExecutor {
    current_phase: ExecutionPhase,
    work_type: WorkType,
    phases_to_execute: Vec<ExecutionPhase>,
    phase_index: usize,
    /// 上下文压缩门限检查点
    compaction_gates: Vec<ExecutionPhase>,
}

impl StateMachineExecutor {
    pub fn new(work_type: WorkType) -> Self {
        let phases = work_type.get_phases();
        Self {
            current_phase: ExecutionPhase::Phase1_Setup,
            work_type,
            phases_to_execute: phases.clone(),
            phase_index: 0,
            // 在阶段 3, 8, 13 设置压缩检查点
            compaction_gates: vec![
                ExecutionPhase::Phase3_CodebaseDiscovery,
                ExecutionPhase::Phase8_Implementation,
                ExecutionPhase::Phase13_Testing,
            ],
        }
    }

    /// 检查是否需要上下文压缩
    pub fn should_compact(&self, context_usage_percent: f64) -> bool {
        if !self.compaction_gates.contains(&self.current_phase) {
            return false;
        }

        // > 85% 硬阻塞，75-85% 警告
        context_usage_percent > 75.0
    }

    /// 执行当前阶段
    pub fn execute_current_phase<F>(&mut self, executor: F) -> PhaseResult
    where
        F: FnOnce(&ExecutionPhase) -> PhaseResult,
    {
        let result = executor(&self.current_phase);

        if result.success {
            self.advance();
        }

        result
    }

    /// 前进到下一个阶段
    fn advance(&mut self) {
        self.phase_index += 1;
        if self.phase_index < self.phases_to_execute.len() {
            self.current_phase = self.phases_to_execute[self.phase_index].clone();
        }
    }

    /// 获取当前阶段
    pub fn current_phase(&self) -> &ExecutionPhase {
        &self.current_phase
    }

    /// 是否已完成
    pub fn is_complete(&self) -> bool {
        self.phase_index >= self.phases_to_execute.len()
    }
}

/// 阶段执行结果
#[derive(Debug)]
pub struct PhaseResult {
    pub phase: ExecutionPhase,
    pub success: bool,
    pub output: serde_json::Value,
    pub artifacts: Vec<String>,
}

// =============================================================================
// 第四部分: Thin Agent框架 (<150行约束)
// =============================================================================

/// Thin Agent - 极简Agent，专注于意图识别和Gateway调用
/// 约束: <150行代码（不含注释和空行）
pub struct ThinAgent {
    agent_id: String,
    role: AgentRole,
    gateway: Arc<GatewayRouter>,
    max_iterations: u32,
}

/// Agent角色
#[derive(Debug, Clone)]
pub enum AgentRole {
    Coordinator,  // 协调者 - 有Task工具，无Edit权限
    Developer,    // 开发者 - 有Edit/Write工具，无Task权限
    Reviewer,     // 审查者 - 只读权限
    Tester,       // 测试者 - 执行测试
}

/// Agent能力（基于角色的工具权限）
#[derive(Debug)]
pub struct AgentCapabilities {
    can_delegate: bool,    // Task工具
    can_edit: bool,        // Edit/Write工具
    can_execute: bool,     // Bash工具
    read_only: bool,       // 只读模式
}

impl AgentRole {
    pub fn capabilities(&self) -> AgentCapabilities {
        match self {
            // Coordinator: 可以委派，不能编辑
            AgentRole::Coordinator => AgentCapabilities {
                can_delegate: true,
                can_edit: false,
                can_execute: false,
                read_only: true,
            },
            // Developer: 可以编辑，不能委派
            AgentRole::Developer => AgentCapabilities {
                can_delegate: false,
                can_edit: true,
                can_execute: true,
                read_only: false,
            },
            // Reviewer: 只读
            AgentRole::Reviewer => AgentCapabilities {
                can_delegate: false,
                can_edit: false,
                can_execute: false,
                read_only: true,
            },
            // Tester: 可以执行测试，有限编辑
            AgentRole::Tester => AgentCapabilities {
                can_delegate: false,
                can_edit: false,
                can_execute: true,
                read_only: false,
            },
        }
    }
}

/// Agent执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub task: String,
    pub context: HashMap<String, String>,
}

/// Agent执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub iterations: u32,
    pub completion_signal: Option<String>,
}

impl ThinAgent {
    pub fn new(agent_id: String, role: AgentRole, gateway: Arc<GatewayRouter>) -> Self {
        Self {
            agent_id,
            role,
            gateway,
            max_iterations: 10,
        }
    }

    /// 执行Agent任务 - 核心逻辑（保持简洁）
    pub fn execute(&self, request: AgentRequest) -> AgentResult {
        let caps = self.role.capabilities();
        let mut iterations = 0;

        // 1. 意图识别
        let intent = self.extract_intent(&request.task);

        // 2. 构建Gateway请求
        let skill_request = SkillRequest {
            intent: intent.clone(),
            payload: serde_json::to_value(&request).unwrap(),
            context: ExecutionContext {
                session_id: self.agent_id.clone(),
                phase: ExecutionPhase::Phase8_Implementation,
                token_budget: TokenBudget {
                    max_tokens: 2000,
                    used_tokens: 0,
                },
            },
        };

        // 3. 通过Gateway路由到确定性处理
        let response = match self.gateway.route(skill_request) {
            Ok(resp) => resp,
            Err(e) => {
                return AgentResult {
                    success: false,
                    output: format!("Gateway routing failed: {:?}", e),
                    iterations: 0,
                    completion_signal: None,
                }
            }
        };

        // 4. 生成完成信号（如果成功）
        let completion_signal = if response.success {
            Some(format!("{}_COMPLETE", intent.to_uppercase()))
        } else {
            None
        };

        AgentResult {
            success: response.success,
            output: response.data.to_string(),
            iterations,
            completion_signal,
        }
    }

    /// 提取意图 - 简单关键词匹配
    fn extract_intent(&self, task: &str) -> String {
        let classifier = IntentClassifier::new();
        classifier.classify(task)
    }

    /// 获取Agent ID
    pub fn id(&self) -> &str {
        &self.agent_id
    }

    /// 获取Agent角色
    pub fn role(&self) -> &AgentRole {
        &self.role
    }
}

// =============================================================================
// 第五部分: Fat Platform接口
// =============================================================================

/// Fat Platform - 确定性运行时，包含所有业务逻辑
pub struct FatPlatform {
    gateway: Arc<GatewayRouter>,
    state_machine: StateMachineExecutor,
    hooks: Vec<Box<dyn PlatformHook>>,
    manifest: Manifest,
}

/// Platform Hook - 在LLM上下文外强制执行
pub trait PlatformHook: Send + Sync {
    fn name(&self) -> &str;
    fn before_execution(&self, phase: &ExecutionPhase) -> HookResult;
    fn after_execution(&self, phase: &ExecutionPhase, result: &PhaseResult) -> HookResult;
}

pub type HookResult = Result<(), HookError>;

#[derive(Debug)]
pub struct HookError {
    pub hook_name: String,
    pub reason: String,
}

/// MANIFEST.yaml 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub feature_id: String,
    pub current_phase: String,
    pub active_agents: Vec<String>,
    pub validation_status: ValidationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatus {
    pub design_verified: bool,
    pub code_reviewed: bool,
    pub tests_passing: bool,
    pub coverage_met: bool,
}

impl FatPlatform {
    pub fn new(work_type: WorkType, feature_id: String) -> Self {
        let gateway = Arc::new(GatewayRouter::new());

        Self {
            gateway: gateway.clone(),
            state_machine: StateMachineExecutor::new(work_type),
            hooks: Vec::new(),
            manifest: Manifest {
                feature_id,
                current_phase: "Setup".to_string(),
                active_agents: Vec::new(),
                validation_status: ValidationStatus {
                    design_verified: false,
                    code_reviewed: false,
                    tests_passing: false,
                    coverage_met: false,
                },
            },
        }
    }

    /// 注册Hook
    pub fn register_hook<H: PlatformHook + 'static>(&mut self, hook: H) {
        self.hooks.push(Box::new(hook));
    }

    /// 执行工作流
    pub fn execute_workflow(&mut self) -> Result<WorkflowResult, PlatformError> {
        while !self.state_machine.is_complete() {
            let phase = self.state_machine.current_phase().clone();

            // 1. 执行前Hooks
            for hook in &self.hooks {
                if let Err(e) = hook.before_execution(&phase) {
                    return Err(PlatformError::HookBlocked(e.hook_name, e.reason));
                }
            }

            // 2. 执行阶段
            let result = self.execute_phase(&phase);

            // 3. 执行后Hooks
            for hook in &self.hooks {
                if let Err(e) = hook.after_execution(&phase, &result) {
                    return Err(PlatformError::HookBlocked(e.hook_name, e.reason));
                }
            }

            // 4. 更新manifest
            self.manifest.current_phase = phase.name().to_string();

            // 5. 检查是否需要上下文压缩
            let context_usage = self.estimate_context_usage();
            if self.state_machine.should_compact(context_usage) && context_usage > 85.0 {
                return Err(PlatformError::ContextCompactionRequired);
            }
        }

        Ok(WorkflowResult {
            success: true,
            completed_phases: self.state_machine.phase_index,
            final_manifest: self.manifest.clone(),
        })
    }

    fn execute_phase(&self, phase: &ExecutionPhase) -> PhaseResult {
        // 实际执行逻辑由Gateway路由到具体技能
        PhaseResult {
            phase: phase.clone(),
            success: true,
            output: serde_json::json!({"phase": phase.name()}),
            artifacts: vec![],
        }
    }

    fn estimate_context_usage(&self) -> f64 {
        // 简化的上下文使用估计
        50.0 // 假设50%使用率
    }

    /// 获取Gateway引用
    pub fn gateway(&self) -> Arc<GatewayRouter> {
        self.gateway.clone()
    }
}

/// 工作流结果
#[derive(Debug)]
pub struct WorkflowResult {
    pub success: bool,
    pub completed_phases: usize,
    pub final_manifest: Manifest,
}

#[derive(Debug)]
pub enum PlatformError {
    HookBlocked(String, String),
    ContextCompactionRequired,
    StateMachineError(String),
}

// =============================================================================
// 第六部分: 示例Hook实现
// =============================================================================

/// 反馈循环Hook - 强制代码必须经过审查和测试
pub struct FeedbackLoopHook;

impl PlatformHook for FeedbackLoopHook {
    fn name(&self) -> &str {
        "feedback_loop"
    }

    fn before_execution(&self, _phase: &ExecutionPhase) -> HookResult {
        Ok(())
    }

    fn after_execution(&self, phase: &ExecutionPhase, result: &PhaseResult) -> HookResult {
        // 在Implementation阶段后，强制设置"脏位"
        if matches!(phase, ExecutionPhase::Phase8_Implementation) && result.success {
            // 实际实现会设置dirty bit
            println!("[Hook] Implementation complete - setting dirty bit for review");
        }
        Ok(())
    }
}

/// 质量门Hook - 阻止未完成审查的代码退出
pub struct QualityGateHook;

impl PlatformHook for QualityGateHook {
    fn name(&self) -> &str {
        "quality_gate"
    }

    fn before_execution(&self, phase: &ExecutionPhase) -> HookResult {
        // 在Completion阶段前检查所有验证是否通过
        if matches!(phase, ExecutionPhase::Phase16_Completion) {
            // 实际实现会检查validation_status
            println!("[Hook] Checking quality gates before completion...");
        }
        Ok(())
    }

    fn after_execution(&self, _phase: &ExecutionPhase, _result: &PhaseResult) -> HookResult {
        Ok(())
    }
}

// =============================================================================
// 第七部分: 测试与验证
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_classifier() {
        let classifier = IntentClassifier::new();

        assert_eq!(classifier.classify("fix react component bug"), "frontend");
        assert_eq!(classifier.classify("optimize database query"), "backend");
        assert_eq!(classifier.classify("write unit tests"), "testing");
        assert_eq!(classifier.classify("unknown task"), "general");
    }

    #[test]
    fn test_phase_transitions() {
        assert_eq!(ExecutionPhase::Phase1_Setup.number(), 1);
        assert_eq!(ExecutionPhase::Phase8_Implementation.number(), 8);
        assert_eq!(ExecutionPhase::Phase16_Completion.number(), 16);

        assert!(ExecutionPhase::Phase1_Setup.next().is_some());
        assert!(ExecutionPhase::Phase16_Completion.next().is_none());
    }

    #[test]
    fn test_work_type_phases() {
        let bugfix = WorkType::BugFix;
        let phases = bugfix.get_phases();
        // BugFix应该跳过5个阶段，剩下11个
        assert_eq!(phases.len(), 11);

        let medium = WorkType::Medium;
        let phases = medium.get_phases();
        // Medium执行所有16个阶段
        assert_eq!(phases.len(), 16);
    }

    #[test]
    fn test_agent_capabilities() {
        let coord_caps = AgentRole::Coordinator.capabilities();
        assert!(coord_caps.can_delegate);
        assert!(!coord_caps.can_edit);

        let dev_caps = AgentRole::Developer.capabilities();
        assert!(!dev_caps.can_delegate);
        assert!(dev_caps.can_edit);

        let reviewer_caps = AgentRole::Reviewer.capabilities();
        assert!(reviewer_caps.read_only);
    }
}

// =============================================================================
// 第八部分: 主函数示例
// =============================================================================

fn main() {
    println!("=== Deterministic Architecture Demo ===\n");

    // 1. 创建Fat Platform
    let mut platform = FatPlatform::new(WorkType::Medium, "feature-001".to_string());

    // 2. 注册Hooks
    platform.register_hook(FeedbackLoopHook);
    platform.register_hook(QualityGateHook);

    // 3. 创建Thin Agent
    let agent = ThinAgent::new(
        "dev-001".to_string(),
        AgentRole::Developer,
        platform.gateway()
    );

    println!("Agent created: {} (role: {:?})", agent.id(), agent.role());

    // 4. 执行工作流
    println!("\nExecuting workflow...");
    match platform.execute_workflow() {
        Ok(result) => {
            println!("\nWorkflow completed successfully!");
            println!("Completed phases: {}", result.completed_phases);
            println!("Final phase: {}", result.final_manifest.current_phase);
        }
        Err(e) => {
            println!("\nWorkflow failed: {:?}", e);
        }
    }
}
