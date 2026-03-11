//! 状态空间Agent生产级架构 - 第二轮深度研究
//! 研究方向: 12_engineering_roadmap - 工程路线图
//! 版本: v2.0 - 2026-03-11
//!
//! 本实现基于以下2025年行业最佳实践:
//! - MCP协议集成 (Model Context Protocol 2025-11-25)
//! - OpenTelemetry可观测性标准
//! - Circuit Breaker可靠性模式
//! - 六层渐进式边界架构

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock, Mutex};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// =============================================================================
// LAYER 1: Core State Machine Foundation
// =============================================================================

/// 状态标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub String);

impl StateId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 状态类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateType {
    /// 初始状态
    Initial,
    /// 中间处理状态
    Processing,
    /// 等待外部输入
    Waiting,
    /// 成功终态
    Success,
    /// 失败终态
    Failure,
    /// 可恢复的错误状态
    Recoverable,
}

/// 状态定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub id: StateId,
    pub state_type: StateType,
    pub metadata: HashMap<String, serde_json::Value>,
    pub entered_at: Instant,
}

impl State {
    pub fn new(id: StateId, state_type: StateType) -> Self {
        Self {
            id,
            state_type,
            metadata: HashMap::new(),
            entered_at: Instant::now(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state_type, StateType::Success | StateType::Failure)
    }

    pub fn can_recover(&self) -> bool {
        matches!(self.state_type, StateType::Recoverable)
    }
}

/// 状态转换触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionTrigger {
    /// 用户输入
    UserInput(String),
    /// LLM响应
    LlmResponse(String),
    /// 工具执行结果
    ToolResult(ToolOutput),
    /// MCP工具调用完成
    McpToolComplete { tool_name: String, result: serde_json::Value },
    /// 超时
    Timeout,
    /// 错误
    Error(String),
    /// 自定义事件
    Custom(String, serde_json::Value),
}

/// 工具输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub tool_name: String,
    pub success: bool,
    pub data: serde_json::Value,
    pub execution_time_ms: u64,
}

/// 状态转换定义
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub condition: TransitionCondition,
    pub action: Option<TransitionAction>,
}

/// 转换条件
#[derive(Debug, Clone)]
pub enum TransitionCondition {
    /// 任意触发器
    Any,
    /// 特定触发器类型
    TriggerType(String),
    /// 自定义条件函数
    Custom(Arc<dyn Fn(&TransitionTrigger) -> bool + Send + Sync>),
}

/// 转换动作
#[derive(Debug, Clone)]
pub enum TransitionAction {
    /// 记录日志
    Log(String),
    /// 发送指标
    Metric { name: String, value: f64 },
    /// 执行副作用
    SideEffect(Arc<dyn Fn() + Send + Sync>),
}

/// 状态机引擎
pub struct StateMachine {
    pub current_state: State,
    pub states: HashMap<StateId, State>,
    pub transitions: Vec<Transition>,
    pub history: VecDeque<State>,
    pub max_history: usize,
}

impl StateMachine {
    pub fn new(initial_state: State) -> Self {
        let mut states = HashMap::new();
        let state_id = initial_state.id.clone();
        states.insert(state_id.clone(), initial_state.clone());

        Self {
            current_state: initial_state,
            states,
            transitions: Vec::new(),
            history: VecDeque::new(),
            max_history: 100,
        }
    }

    pub fn add_state(&mut self, state: State) {
        self.states.insert(state.id.clone(), state);
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    /// 尝试状态转换
    pub fn try_transition(&mut self, trigger: &TransitionTrigger) -> Result<Option<StateId>, StateError> {
        let current_id = &self.current_state.id;

        // 查找匹配的转换
        for transition in &self.transitions {
            if &transition.from == current_id && self.matches_condition(&transition.condition, trigger) {
                // 保存历史
                if self.history.len() >= self.max_history {
                    self.history.pop_front();
                }
                self.history.push_back(self.current_state.clone());

                // 执行转换
                let new_state = self.states.get(&transition.to)
                    .ok_or_else(|| StateError::StateNotFound(transition.to.0.clone()))?
                    .clone();

                // 执行转换动作
                if let Some(ref action) = transition.action {
                    self.execute_action(action);
                }

                self.current_state = new_state;
                return Ok(Some(transition.to.clone()));
            }
        }

        Ok(None)
    }

    fn matches_condition(&self, condition: &TransitionCondition, trigger: &TransitionTrigger) -> bool {
        match condition {
            TransitionCondition::Any => true,
            TransitionCondition::TriggerType(t) => {
                let trigger_type = match trigger {
                    TransitionTrigger::UserInput(_) => "user_input",
                    TransitionTrigger::LlmResponse(_) => "llm_response",
                    TransitionTrigger::ToolResult(_) => "tool_result",
                    TransitionTrigger::McpToolComplete { .. } => "mcp_complete",
                    TransitionTrigger::Timeout => "timeout",
                    TransitionTrigger::Error(_) => "error",
                    TransitionTrigger::Custom(name, _) => name.as_str(),
                };
                trigger_type == t
            }
            TransitionCondition::Custom(f) => f(trigger),
        }
    }

    fn execute_action(&self, action: &TransitionAction) {
        match action {
            TransitionAction::Log(msg) => {
                tracing::info!("[StateMachine] {}", msg);
            }
            TransitionAction::Metric { name, value } => {
                tracing::info!("[StateMachine] metric: {} = {}", name, value);
            }
            TransitionAction::SideEffect(f) => f(),
        }
    }

    /// 回滚到上一个状态
    pub fn rollback(&mut self) -> Option<State> {
        self.history.pop_back().map(|prev_state| {
            self.current_state = prev_state.clone();
            prev_state
        })
    }

    /// 获取当前状态路径
    pub fn get_state_path(&self) -> Vec<StateId> {
        self.history.iter().map(|s| s.id.clone()).chain(std::iter::once(self.current_state.id.clone())).collect()
    }
}

#[derive(Error, Debug)]
pub enum StateError {
    #[error("State not found: {0}")]
    StateNotFound(String),
    #[error("Invalid transition from {0} with trigger {1}")]
    InvalidTransition(String, String),
    #[error("State machine is in terminal state")]
    TerminalState,
}

// =============================================================================
// LAYER 2: Actor System for Isolation and Concurrency
// =============================================================================

/// Actor消息类型
#[derive(Debug)]
pub enum ActorMessage {
    /// 处理状态转换
    ProcessTransition {
        trigger: TransitionTrigger,
        respond_to: oneshot::Sender<Result<StateId, ActorError>>,
    },
    /// 获取当前状态
    GetCurrentState {
        respond_to: oneshot::Sender<State>,
    },
    /// 执行MCP工具调用
    ExecuteMcpTool {
        tool_name: String,
        params: serde_json::Value,
        respond_to: oneshot::Sender<Result<ToolOutput, ActorError>>,
    },
    /// 查询状态历史
    GetHistory {
        respond_to: oneshot::Sender<Vec<State>>,
    },
    /// 优雅关闭
    Shutdown,
}

/// Actor错误类型
#[derive(Error, Debug, Clone)]
pub enum ActorError {
    #[error("State machine error: {0}")]
    StateMachineError(String),
    #[error("MCP tool execution failed: {0}")]
    McpToolError(String),
    #[error("Actor is shutting down")]
    ShuttingDown,
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 状态空间Actor
pub struct StateSpaceActor {
    id: String,
    state_machine: Arc<RwLock<StateMachine>>,
    mcp_gateway: Arc<McpGateway>,
    receiver: mpsc::UnboundedReceiver<ActorMessage>,
    metrics: Arc<MetricsCollector>,
}

impl StateSpaceActor {
    pub fn new(
        id: String,
        initial_state: State,
        mcp_gateway: Arc<McpGateway>,
        metrics: Arc<MetricsCollector>,
    ) -> (Self, mpsc::UnboundedSender<ActorMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let state_machine = Arc::new(RwLock::new(StateMachine::new(initial_state)));

        let actor = Self {
            id,
            state_machine,
            mcp_gateway,
            receiver,
            metrics,
        };

        (actor, sender)
    }

    pub async fn run(mut self) {
        tracing::info!("[Actor:{}] Starting actor loop", self.id);

        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ActorMessage::ProcessTransition { trigger, respond_to } => {
                    let start = Instant::now();
                    let result = self.handle_transition(trigger).await;
                    let elapsed = start.elapsed();

                    self.metrics.record_histogram("actor.transition_duration", elapsed.as_millis() as f64);

                    let _ = respond_to.send(result);
                }
                ActorMessage::GetCurrentState { respond_to } => {
                    let state = self.state_machine.read().await.current_state.clone();
                    let _ = respond_to.send(state);
                }
                ActorMessage::ExecuteMcpTool { tool_name, params, respond_to } => {
                    let result = self.handle_mcp_tool(tool_name, params).await;
                    let _ = respond_to.send(result);
                }
                ActorMessage::GetHistory { respond_to } => {
                    let history: Vec<State> = self.state_machine.read().await.history.iter().cloned().collect();
                    let _ = respond_to.send(history);
                }
                ActorMessage::Shutdown => {
                    tracing::info!("[Actor:{}] Received shutdown signal", self.id);
                    break;
                }
            }
        }

        tracing::info!("[Actor:{}] Actor loop ended", self.id);
    }

    async fn handle_transition(&self, trigger: TransitionTrigger) -> Result<StateId, ActorError> {
        let mut sm = self.state_machine.write().await;

        match sm.try_transition(&trigger) {
            Ok(Some(new_state_id)) => {
                tracing::debug!("[Actor:{}] Transitioned to {}", self.id, new_state_id);
                Ok(new_state_id)
            }
            Ok(None) => {
                Err(ActorError::StateMachineError("No valid transition".to_string()))
            }
            Err(e) => Err(ActorError::StateMachineError(e.to_string())),
        }
    }

    async fn handle_mcp_tool(&self, tool_name: String, params: serde_json::Value) -> Result<ToolOutput, ActorError> {
        let start = Instant::now();

        let result = self.mcp_gateway.call_tool(&tool_name, params).await
            .map_err(|e| ActorError::McpToolError(e.to_string()))?;

        let elapsed = start.elapsed();

        Ok(ToolOutput {
            tool_name,
            success: result.get("error").is_none(),
            data: result,
            execution_time_ms: elapsed.as_millis() as u64,
        })
    }
}

/// Actor池管理器
pub struct ActorPool {
    actors: RwLock<HashMap<String, mpsc::UnboundedSender<ActorMessage>>>,
    max_actors: usize,
}

impl ActorPool {
    pub fn new(max_actors: usize) -> Self {
        Self {
            actors: RwLock::new(HashMap::new()),
            max_actors,
        }
    }

    pub async fn spawn_actor(
        &self,
        actor_id: String,
        initial_state: State,
        mcp_gateway: Arc<McpGateway>,
        metrics: Arc<MetricsCollector>,
    ) -> Result<mpsc::UnboundedSender<ActorMessage>, ActorError> {
        let actors = self.actors.read().await;
        if actors.len() >= self.max_actors {
            return Err(ActorError::Internal("Max actors reached".to_string()));
        }
        drop(actors);

        let (actor, sender) = StateSpaceActor::new(
            actor_id.clone(),
            initial_state,
            mcp_gateway,
            metrics,
        );

        tokio::spawn(actor.run());

        self.actors.write().await.insert(actor_id, sender.clone());

        Ok(sender)
    }

    pub async fn get_actor(&self, actor_id: &str) -> Option<mpsc::UnboundedSender<ActorMessage>> {
        self.actors.read().await.get(actor_id).cloned()
    }
}

// =============================================================================
// LAYER 3: LLM Gateway with Circuit Breaker and Semantic Cache
// =============================================================================

/// 断路器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,      // 正常
    Open,        // 断开
    HalfOpen,    // 半开测试
}

/// 断路器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_duration: Duration,
    pub rolling_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration: Duration::from_secs(60),
            rolling_window: Duration::from_secs(300),
        }
    }
}

/// 断路器实现
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: RwLock<u32>,
    success_count: RwLock<u32>,
    last_failure_time: RwLock<Option<Instant>>,
    last_state_change: RwLock<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            success_count: RwLock::new(0),
            last_failure_time: RwLock::new(None),
            last_state_change: RwLock::new(Instant::now()),
        }
    }

    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
    {
        // 检查状态
        {
            let state = self.state.read().await;
            match *state {
                CircuitState::Open => {
                    let last_change = *self.last_state_change.read().await;
                    if last_change.elapsed() >= self.config.timeout_duration {
                        drop(state);
                        self.transition_to(CircuitState::HalfOpen).await;
                    } else {
                        return Err(CircuitBreakerError::CircuitOpen);
                    }
                }
                _ => {}
            }
        }

        // 执行操作
        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(CircuitBreakerError::OperationFailed(e.to_string()))
            }
        }
    }

    async fn on_success(&self) {
        let mut success = self.success_count.write().await;
        *success += 1;

        let state = self.state.read().await;
        if *state == CircuitState::HalfOpen && *success >= self.config.success_threshold {
            drop(state);
            self.transition_to(CircuitState::Closed).await;
            *self.failure_count.write().await = 0;
        }
    }

    async fn on_failure(&self) {
        let mut failures = self.failure_count.write().await;
        *failures += 1;
        *self.last_failure_time.write().await = Some(Instant::now());

        if *failures >= self.config.failure_threshold {
            drop(failures);
            self.transition_to(CircuitState::Open).await;
        }
    }

    async fn transition_to(&self, new_state: CircuitState) {
        let mut state = self.state.write().await;
        *state = new_state;
        *self.last_state_change.write().await = Instant::now();

        match new_state {
            CircuitState::Closed => {
                *self.failure_count.write().await = 0;
                tracing::info!("[CircuitBreaker] Transitioned to CLOSED");
            }
            CircuitState::Open => {
                *self.success_count.write().await = 0;
                tracing::warn!("[CircuitBreaker] Transitioned to OPEN");
            }
            CircuitState::HalfOpen => {
                tracing::info!("[CircuitBreaker] Transitioned to HALF_OPEN");
            }
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
}

#[derive(Error, Debug)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is OPEN")]
    CircuitOpen,
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// 语义缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub embedding: Vec<f32>,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// 语义缓存
pub struct SemanticCache {
    entries: RwLock<Vec<CacheEntry>>,
    similarity_threshold: f32,
    max_entries: usize,
}

impl SemanticCache {
    pub fn new(similarity_threshold: f32, max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            similarity_threshold,
            max_entries,
        }
    }

    /// 计算余弦相似度
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// 生成简单embedding (生产环境应使用真实embedding模型)
    fn simple_embedding(text: &str) -> Vec<f32> {
        // 简化的hash-based embedding用于演示
        let mut vec = vec![0.0f32; 128];
        for (i, byte) in text.bytes().enumerate() {
            vec[i % 128] += byte as f32 / 255.0;
        }
        // 归一化
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
        vec
    }

    pub async fn get(&self, query: &str) -> Option<serde_json::Value> {
        let query_embedding = Self::simple_embedding(query);
        let entries = self.entries.read().await;

        let mut best_match: Option<(f32, &CacheEntry)> = None;

        for entry in entries.iter() {
            if entry.is_expired() {
                continue;
            }

            let similarity = Self::cosine_similarity(&query_embedding, &entry.embedding);
            if similarity >= self.similarity_threshold {
                if best_match.is_none() || similarity > best_match.as_ref().unwrap().0 {
                    best_match = Some((similarity, entry));
                }
            }
        }

        best_match.map(|(_, entry)| entry.value.clone())
    }

    pub async fn put(&self, key: String, value: serde_json::Value, ttl: Duration) {
        let embedding = Self::simple_embedding(&key);
        let entry = CacheEntry {
            key,
            value,
            embedding,
            created_at: Instant::now(),
            ttl,
        };

        let mut entries = self.entries.write().await;

        // 清理过期条目
        entries.retain(|e| !e.is_expired());

        // 如果超过最大条目数，移除最旧的
        if entries.len() >= self.max_entries {
            entries.remove(0);
        }

        entries.push(entry);
    }

    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }
}

/// LLM客户端trait
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str, config: &LlmConfig) -> Result<LlmResponse, LlmError>;
}

/// LLM配置
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 1.0,
        }
    }
}

/// LLM响应
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub usage: TokenUsage,
    pub finish_reason: String,
}

/// Token使用统计
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// LLM Gateway - 统一入口
pub struct LlmGateway {
    client: Arc<dyn LlmClient>,
    circuit_breaker: CircuitBreaker,
    cache: SemanticCache,
    metrics: Arc<MetricsCollector>,
}

impl LlmGateway {
    pub fn new(
        client: Arc<dyn LlmClient>,
        circuit_breaker_config: CircuitBreakerConfig,
        cache: SemanticCache,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            client,
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
            cache,
            metrics,
        }
    }

    pub async fn complete(&self, prompt: &str, config: &LlmConfig) -> Result<LlmResponse, GatewayError> {
        let start = Instant::now();

        // 1. 检查缓存
        if let Some(cached) = self.cache.get(prompt).await {
            self.metrics.increment_counter("llm.cache_hit");
            if let Ok(response) = serde_json::from_value::<LlmResponse>(cached) {
                return Ok(response);
            }
        }

        self.metrics.increment_counter("llm.cache_miss");

        // 2. 断路器保护调用
        let client = self.client.clone();
        let prompt_owned = prompt.to_string();
        let config_owned = config.clone();

        let result = self.circuit_breaker.call(|| async {
            client.complete(&prompt_owned, &config_owned).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }).await;

        let elapsed = start.elapsed();
        self.metrics.record_histogram("llm.latency", elapsed.as_millis() as f64);

        match result {
            Ok(response) => {
                // 缓存结果
                if let Ok(value) = serde_json::to_value(&response) {
                    self.cache.put(prompt.to_string(), value, Duration::from_secs(3600)).await;
                }

                self.metrics.increment_counter("llm.success");
                self.metrics.record_histogram("llm.tokens", response.usage.total_tokens as f64);

                Ok(response)
            }
            Err(e) => {
                self.metrics.increment_counter("llm.failure");
                Err(GatewayError::CircuitBreaker(e))
            }
        }
    }

    pub async fn get_circuit_state(&self) -> CircuitState {
        self.circuit_breaker.get_state().await
    }
}

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Circuit breaker error: {0}")]
    CircuitBreaker(#[from] CircuitBreakerError),
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
}

// =============================================================================
// LAYER 4: Observability with OpenTelemetry
// =============================================================================

/// 指标收集器
pub struct MetricsCollector {
    counters: RwLock<HashMap<String, u64>>,
    histograms: RwLock<HashMap<String, Vec<f64>>>,
    gauges: RwLock<HashMap<String, f64>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
        }
    }

    pub async fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    pub async fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        histograms.entry(name.to_string()).or_insert_with(Vec::new).push(value);
    }

    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }

    pub async fn get_report(&self) -> MetricsReport {
        MetricsReport {
            counters: self.counters.read().await.clone(),
            histograms: self.histograms.read().await.clone(),
            gauges: self.gauges.read().await.clone(),
        }
    }
}

/// 指标报告
#[derive(Debug, Clone)]
pub struct MetricsReport {
    pub counters: HashMap<String, u64>,
    pub histograms: HashMap<String, Vec<f64>>,
    pub gauges: HashMap<String, f64>,
}

/// 追踪上下文
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: String,
    pub start_time: Instant,
    pub attributes: HashMap<String, String>,
}

impl TraceContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            operation: operation.into(),
            start_time: Instant::now(),
            attributes: HashMap::new(),
        }
    }

    pub fn child(&self, operation: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Uuid::new_v4().to_string(),
            parent_span_id: Some(self.span_id.clone()),
            operation: operation.into(),
            start_time: Instant::now(),
            attributes: HashMap::new(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// OpenTelemetry集成
pub struct OpenTelemetryIntegration {
    service_name: String,
    service_version: String,
    metrics: Arc<MetricsCollector>,
}

impl OpenTelemetryIntegration {
    pub fn new(service_name: String, service_version: String, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            service_name,
            service_version,
            metrics,
        }
    }

    pub fn create_span(&self, name: &str) -> TraceContext {
        TraceContext::new(name)
    }

    pub async fn export_traces(&self, traces: &[TraceContext]) {
        // 实际实现应导出到OTLP collector
        for trace in traces {
            tracing::info!(
                trace_id = %trace.trace_id,
                span_id = %trace.span_id,
                operation = %trace.operation,
                duration_ms = %trace.elapsed().as_millis(),
                "[OpenTelemetry] Exporting span"
            );
        }
    }
}

// =============================================================================
// LAYER 5: MCP Gateway Adapter
// =============================================================================

/// MCP工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP资源定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
}

/// MCP Gateway - 状态空间Agent与MCP协议的适配层
pub struct McpGateway {
    servers: RwLock<HashMap<String, McpServerConnection>>,
    tools: RwLock<HashMap<String, McpTool>>,
    metrics: Arc<MetricsCollector>,
}

/// MCP服务器连接
#[derive(Debug, Clone)]
pub struct McpServerConnection {
    pub server_id: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub connected_at: Instant,
}

impl McpGateway {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            tools: RwLock::new(HashMap::new()),
            metrics,
        }
    }

    /// 注册MCP服务器
    pub async fn register_server(&self, server_id: String, endpoint: String, capabilities: Vec<String>) {
        let connection = McpServerConnection {
            server_id: server_id.clone(),
            endpoint,
            capabilities,
            connected_at: Instant::now(),
        };

        self.servers.write().await.insert(server_id, connection);
        tracing::info!("[MCP] Registered server: {}", server_id);
    }

    /// 注册工具
    pub async fn register_tool(&self, tool: McpTool) {
        self.tools.write().await.insert(tool.name.clone(), tool);
    }

    /// 调用MCP工具
    pub async fn call_tool(&self, tool_name: &str, params: serde_json::Value) -> Result<serde_json::Value, McpError> {
        let start = Instant::now();

        let tools = self.tools.read().await;
        let tool = tools.get(tool_name)
            .ok_or_else(|| McpError::ToolNotFound(tool_name.to_string()))?;

        // 验证参数
        self.validate_params(&tool.input_schema, &params)?;

        // 模拟工具调用 (实际实现应通过stdio/HTTP调用MCP服务器)
        let result = self.execute_tool_call(tool_name, params).await;

        let elapsed = start.elapsed();
        self.metrics.record_histogram("mcp.tool_duration", elapsed.as_millis() as f64);

        result
    }

    fn validate_params(&self, schema: &serde_json::Value, params: &serde_json::Value) -> Result<(), McpError> {
        // 简化验证 - 实际应使用JSON Schema验证
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if params.get(field_name).is_none() {
                        return Err(McpError::InvalidParams(format!("Missing required field: {}", field_name)));
                    }
                }
            }
        }
        Ok(())
    }

    async fn execute_tool_call(&self, tool_name: &str, params: serde_json::Value) -> Result<serde_json::Value, McpError> {
        // 模拟工具执行
        tracing::info!("[MCP] Executing tool: {} with params: {:?}", tool_name, params);

        // 实际实现应根据tool_name路由到对应的MCP服务器
        Ok(serde_json::json!({
            "tool": tool_name,
            "status": "success",
            "result": format!("Executed {} successfully", tool_name),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// 列出可用工具
    pub async fn list_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.values().cloned().collect()
    }

    /// 将MCP工具转换为状态机动作
    pub fn tool_to_action(&self, tool_name: &str, params: serde_json::Value) -> TransitionTrigger {
        TransitionTrigger::McpToolComplete {
            tool_name: tool_name.to_string(),
            result: params,
        }
    }
}

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}

// =============================================================================
// LAYER 6: Configuration Management
// =============================================================================

/// Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub llm: LlmConfigSpec,
    pub circuit_breaker: CircuitBreakerConfigSpec,
    pub cache: CacheConfigSpec,
    pub mcp: McpConfigSpec,
    pub observability: ObservabilityConfigSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfigSpec {
    pub provider: String,
    pub model: String,
    pub api_key_env: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfigSpec {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfigSpec {
    pub enabled: bool,
    pub similarity_threshold: f32,
    pub max_entries: usize,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigSpec {
    pub enabled: bool,
    pub servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub endpoint: String,
    pub transport: String, // "stdio" or "http"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfigSpec {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub otlp_endpoint: Option<String>,
    pub log_level: String,
}

/// 配置管理器
pub struct ConfigManager {
    configs: RwLock<HashMap<String, AgentConfig>>,
    config_path: Option<String>,
}

impl ConfigManager {
    pub fn new(config_path: Option<String>) -> Self {
        Self {
            configs: RwLock::new(HashMap::new()),
            config_path,
        }
    }

    pub async fn load_config(&self, agent_id: &str) -> Option<AgentConfig> {
        self.configs.read().await.get(agent_id).cloned()
    }

    pub async fn save_config(&self, config: AgentConfig) {
        self.configs.write().await.insert(config.agent_id.clone(), config);
    }

    pub async fn load_from_file(&self, path: &str) -> Result<(), ConfigError> {
        let content = tokio::fs::read_to_string(path).await?;
        let configs: Vec<AgentConfig> = serde_json::from_str(&content)?;

        let mut map = self.configs.write().await;
        for config in configs {
            map.insert(config.agent_id.clone(), config);
        }

        Ok(())
    }

    pub async fn save_to_file(&self, path: &str) -> Result<(), ConfigError> {
        let configs: Vec<AgentConfig> = self.configs.read().await.values().cloned().collect();
        let content = serde_json::to_string_pretty(&configs)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// 热重载配置
    pub async fn hot_reload(&self) -> Result<(), ConfigError> {
        if let Some(ref path) = self.config_path {
            self.load_from_file(path).await?;
            tracing::info!("[ConfigManager] Hot reloaded configuration from {}", path);
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// =============================================================================
// Builder Pattern for Agent Construction
// =============================================================================

/// Agent Builder
pub struct AgentBuilder {
    config: AgentConfig,
    initial_state: Option<State>,
    custom_transitions: Vec<Transition>,
}

impl AgentBuilder {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            config: AgentConfig {
                agent_id: agent_id.into(),
                name: "Unnamed Agent".to_string(),
                description: "".to_string(),
                llm: LlmConfigSpec {
                    provider: "openai".to_string(),
                    model: "gpt-4".to_string(),
                    api_key_env: "OPENAI_API_KEY".to_string(),
                    timeout_seconds: 30,
                },
                circuit_breaker: CircuitBreakerConfigSpec {
                    failure_threshold: 5,
                    success_threshold: 3,
                    timeout_seconds: 60,
                },
                cache: CacheConfigSpec {
                    enabled: true,
                    similarity_threshold: 0.85,
                    max_entries: 1000,
                    ttl_seconds: 3600,
                },
                mcp: McpConfigSpec {
                    enabled: true,
                    servers: Vec::new(),
                },
                observability: ObservabilityConfigSpec {
                    tracing_enabled: true,
                    metrics_enabled: true,
                    otlp_endpoint: None,
                    log_level: "info".to_string(),
                },
            },
            initial_state: None,
            custom_transitions: Vec::new(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.config.description = desc.into();
        self
    }

    pub fn with_llm(mut self, model: impl Into<String>) -> Self {
        self.config.llm.model = model.into();
        self
    }

    pub fn with_initial_state(mut self, state: State) -> Self {
        self.initial_state = Some(state);
        self
    }

    pub fn with_transition(mut self, transition: Transition) -> Self {
        self.custom_transitions.push(transition);
        self
    }

    pub fn with_mcp_server(mut self, server: McpServerConfig) -> Self {
        self.config.mcp.servers.push(server);
        self
    }

    pub fn build(self) -> Result<AgentRuntime, BuilderError> {
        let initial_state = self.initial_state
            .ok_or(BuilderError::MissingInitialState)?;

        Ok(AgentRuntime {
            config: self.config,
            initial_state,
            transitions: self.custom_transitions,
        })
    }
}

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("Missing initial state")]
    MissingInitialState,
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Agent运行时
pub struct AgentRuntime {
    pub config: AgentConfig,
    pub initial_state: State,
    pub transitions: Vec<Transition>,
}

impl AgentRuntime {
    pub async fn start(
        self,
        metrics: Arc<MetricsCollector>,
    ) -> Result<mpsc::UnboundedSender<ActorMessage>, AgentError> {
        let mcp_gateway = Arc::new(McpGateway::new(metrics.clone()));

        // 注册配置的MCP服务器
        for server in &self.config.mcp.servers {
            mcp_gateway.register_server(
                server.id.clone(),
                server.endpoint.clone(),
                vec!["tools".to_string()],
            ).await;
        }

        let (actor, sender) = StateSpaceActor::new(
            self.config.agent_id.clone(),
            self.initial_state,
            mcp_gateway,
            metrics,
        );

        tokio::spawn(actor.run());

        Ok(sender)
    }
}

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Actor error: {0}")]
    Actor(#[from] ActorError),
    #[error("Configuration error: {0}")]
    Config(String),
}

// =============================================================================
// Testing Framework
// =============================================================================

/// 确定性模拟测试 (Deterministic Simulation Testing)
pub struct DeterministicTester {
    recorded_traces: Vec<StateTransitionTrace>,
}

#[derive(Debug, Clone)]
pub struct StateTransitionTrace {
    pub from_state: StateId,
    pub trigger: TransitionTrigger,
    pub to_state: StateId,
    pub timestamp: Instant,
}

impl DeterministicTester {
    pub fn new() -> Self {
        Self {
            recorded_traces: Vec::new(),
        }
    }

    /// 记录状态转换
    pub fn record_transition(&mut self, from: StateId, trigger: TransitionTrigger, to: StateId) {
        self.recorded_traces.push(StateTransitionTrace {
            from_state: from,
            trigger,
            to_state: to,
            timestamp: Instant::now(),
        });
    }

    /// 回放测试
    pub async fn replay(&self, agent: &mpsc::UnboundedSender<ActorMessage>) -> Result<bool, AgentError> {
        for trace in &self.recorded_traces {
            let (tx, rx) = oneshot::channel();

            agent.send(ActorMessage::ProcessTransition {
                trigger: trace.trigger.clone(),
                respond_to: tx,
            }).map_err(|_| AgentError::Config("Failed to send message".to_string()))?;

            let result = rx.await
                .map_err(|_| AgentError::Config("Failed to receive response".to_string()))?;

            match result {
                Ok(state_id) => {
                    if state_id != trace.to_state {
                        tracing::error!(
                            "Replay mismatch: expected {}, got {}",
                            trace.to_state, state_id
                        );
                        return Ok(false);
                    }
                }
                Err(e) => {
                    tracing::error!("Replay failed: {}", e);
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// 属性测试 - 验证状态机不变量
    pub fn verify_invariants(&self, invariants: &[Box<dyn Fn(&[StateTransitionTrace]) -> bool>]) -> Vec<String> {
        let mut failures = Vec::new();

        for (i, invariant) in invariants.iter().enumerate() {
            if !invariant(&self.recorded_traces) {
                failures.push(format!("Invariant {} failed", i));
            }
        }

        failures
    }
}

// =============================================================================
// Example Usage and Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_basic() {
        let initial = State::new(StateId::new("idle"), StateType::Initial);
        let mut sm = StateMachine::new(initial);

        let processing = State::new(StateId::new("processing"), StateType::Processing);
        sm.add_state(processing.clone());

        sm.add_transition(Transition {
            from: StateId::new("idle"),
            to: StateId::new("processing"),
            condition: TransitionCondition::Any,
            action: None,
        });

        let result = sm.try_transition(&TransitionTrigger::UserInput("start".to_string()));
        assert!(result.is_ok());
        assert_eq!(sm.current_state.id.0, "processing");
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout_duration: Duration::from_secs(60),
            rolling_window: Duration::from_secs(300),
        });

        // 初始状态为Closed
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    #[test]
    fn test_semantic_cache_similarity() {
        let cache = SemanticCache::new(0.8, 100);

        // 测试余弦相似度计算
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let c = vec![1.0, 0.0, 0.0];

        let sim_ab = SemanticCache::cosine_similarity(&a, &b);
        let sim_ac = SemanticCache::cosine_similarity(&a, &c);

        assert!(sim_ab < 0.1); // 正交向量
        assert!((sim_ac - 1.0).abs() < 0.001); // 相同向量
    }

    #[test]
    fn test_builder_pattern() {
        let runtime = AgentBuilder::new("test-agent")
            .name("Test Agent")
            .description("A test agent")
            .with_initial_state(State::new(StateId::new("start"), StateType::Initial))
            .build();

        assert!(runtime.is_ok());
    }

    #[test]
    fn test_trace_context() {
        let parent = TraceContext::new("parent_operation");
        let child = parent.child("child_operation");

        assert_eq!(parent.trace_id, child.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }
}

/// 完整示例：构建一个客服Agent
pub async fn example_customer_service_agent() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建指标收集器
    let metrics = Arc::new(MetricsCollector::new());

    // 2. 使用Builder构建Agent
    let agent = AgentBuilder::new("customer-service-001")
        .name("Customer Service Agent")
        .description("Handles customer inquiries with MCP tool integration")
        .with_llm("gpt-4")
        .with_initial_state(State::new(StateId::new("greeting"), StateType::Initial))
        .with_mcp_server(McpServerConfig {
            id: "knowledge-base".to_string(),
            endpoint: "/mcp/kb".to_string(),
            transport: "stdio".to_string(),
        })
        .with_transition(Transition {
            from: StateId::new("greeting"),
            to: StateId::new("understanding"),
            condition: TransitionCondition::TriggerType("user_input".to_string()),
            action: Some(TransitionAction::Log("User provided input".to_string())),
        })
        .with_transition(Transition {
            from: StateId::new("understanding"),
            to: StateId::new("retrieving"),
            condition: TransitionCondition::TriggerType("llm_response".to_string()),
            action: Some(TransitionAction::Metric { name: "intent_classified".to_string(), value: 1.0 }),
        })
        .build()?;

    // 3. 启动Agent
    let agent_handle = agent.start(metrics.clone()).await?;

    // 4. 发送消息
    let (tx, rx) = oneshot::channel();
    agent_handle.send(ActorMessage::ProcessTransition {
        trigger: TransitionTrigger::UserInput("I need help with my order".to_string()),
        respond_to: tx,
    })?;

    let new_state = rx.await??;
    println!("Agent transitioned to: {}", new_state);

    // 5. 获取指标报告
    let report = metrics.get_report().await;
    println!("Metrics: {:?}", report);

    Ok(())
}

// =============================================================================
// Module Re-exports
// =============================================================================

pub mod prelude {
    pub use super::{
        State, StateId, StateType,
        StateMachine, Transition, TransitionTrigger, TransitionCondition, TransitionAction,
        ActorMessage, ActorError, StateSpaceActor, ActorPool,
        CircuitBreaker, CircuitBreakerConfig, CircuitState, CircuitBreakerError,
        SemanticCache, CacheEntry,
        LlmClient, LlmConfig, LlmResponse, TokenUsage, LlmError,
        LlmGateway, GatewayError,
        MetricsCollector, MetricsReport, TraceContext, OpenTelemetryIntegration,
        McpGateway, McpTool, McpResource, McpError, McpServerConnection, McpServerConfig,
        AgentConfig, ConfigManager, ConfigError,
        AgentBuilder, AgentRuntime, BuilderError, AgentError,
        DeterministicTester, StateTransitionTrace,
        ToolOutput,
    };
}
