// ============================================================================
// 状态空间Agent工程架构 - MVP实现框架
// State Space Agent Engineering Architecture - MVP Implementation Framework
//
// 研究方向: 12_engineering_roadmap - 工程路径：从理论到实现
// 创建时间: 2026-03-10
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use anyhow::{Context, Result};
use tracing::{info, warn, error, instrument};
use schemars::JsonSchema;

// ============================================================================
// 1. 核心类型系统 - Core Type System
// ============================================================================

/// 状态空间中的唯一标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub uuid::Uuid);

impl StateId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for StateId {
    fn default() -> Self {
        Self::new()
    }
}

/// 状态空间中的状态定义
/// 每个状态包含：类型、元数据、验证规则
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct State {
    pub id: StateId,
    pub state_type: StateType,
    pub metadata: StateMetadata,
    pub validation_rules: Vec<ValidationRule>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub version: u64,
}

/// 状态类型枚举 - 定义系统中所有可能的状态类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateType {
    /// 初始状态
    Initial,
    /// 处理中状态
    Processing { step: String },
    /// 等待外部输入
    WaitingForInput { required_fields: Vec<String> },
    /// 验证状态
    Validating { criteria: Vec<String> },
    /// 完成状态
    Completed { result: CompletionResult },
    /// 错误状态
    Error { code: String, message: String },
    /// 终止状态
    Terminal,
}

/// 完成结果类型
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompletionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub metrics: ExecutionMetrics,
}

/// 执行指标
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExecutionMetrics {
    pub steps_taken: u32,
    pub tokens_consumed: u64,
    pub latency_ms: u64,
    pub cache_hits: u32,
}

/// 状态元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct StateMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Priority,
    pub custom_data: HashMap<String, serde_json::Value>,
}

/// 优先级枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// 验证规则
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationRule {
    pub name: String,
    pub rule_type: RuleType,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    RequiredField { field: String },
    TypeCheck { field: String, expected_type: String },
    RangeCheck { field: String, min: Option<f64>, max: Option<f64> },
    PatternMatch { field: String, regex: String },
    Custom { expression: String },
}

// ============================================================================
// 2. 错误处理系统 - Error Handling System
// ============================================================================

/// 状态空间Agent错误类型
#[derive(Error, Debug, Clone)]
pub enum AgentError {
    #[error("State transition failed: {from} -> {to}, reason: {reason}")]
    TransitionFailed { from: StateId, to: StateId, reason: String },

    #[error("Validation failed for state {state_id}: {errors:?}")]
    ValidationFailed { state_id: StateId, errors: Vec<String> },

    #[error("State not found: {0}")]
    StateNotFound(StateId),

    #[error("Invalid state type: expected {expected}, got {actual}")]
    InvalidStateType { expected: String, actual: String },

    #[error("Execution timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Max iterations exceeded: {max}")]
    MaxIterationsExceeded { max: u32 },

    #[error("LLM provider error: {0}")]
    LlmProviderError(String),

    #[error("Circuit breaker open for provider: {0}")]
    CircuitBreakerOpen(String),

    #[error("Cache error: {0}")]
    CacheError(String),
}

/// 错误严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

impl AgentError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            AgentError::ValidationFailed { .. } => ErrorSeverity::Warning,
            AgentError::StateNotFound(_) => ErrorSeverity::Error,
            AgentError::TransitionFailed { .. } => ErrorSeverity::Error,
            AgentError::Timeout { .. } => ErrorSeverity::Error,
            AgentError::MaxIterationsExceeded { .. } => ErrorSeverity::Critical,
            AgentError::LlmProviderError(_) => ErrorSeverity::Error,
            AgentError::CircuitBreakerOpen(_) => ErrorSeverity::Error,
            AgentError::CacheError(_) => ErrorSeverity::Warning,
            AgentError::InvalidStateType { .. } => ErrorSeverity::Error,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self,
            AgentError::LlmProviderError(_) |
            AgentError::Timeout { .. } |
            AgentError::CacheError(_)
        )
    }
}

// ============================================================================
// 3. 状态机核心 - State Machine Core
// ============================================================================

/// 状态转换定义
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub trigger: TransitionTrigger,
    pub condition: Option<TransitionCondition>,
    pub action: Option<TransitionAction>,
}

/// 转换触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionTrigger {
    /// 自动触发
    Auto,
    /// LLM决策触发
    LlmDecision { prompt: String },
    /// 用户输入触发
    UserInput { field: String },
    /// 外部事件触发
    ExternalEvent { event_type: String },
    /// 定时触发
    Scheduled { delay_ms: u64 },
    /// 条件触发
    Condition { expression: String },
}

/// 转换条件
#[derive(Debug, Clone)]
pub struct TransitionCondition {
    pub predicate: Arc<dyn Fn(&State) -> bool + Send + Sync>,
}

/// 转换动作
#[derive(Debug, Clone)]
pub struct TransitionAction {
    pub handler: Arc<dyn Fn(&mut State) -> Result<()> + Send + Sync>,
}

/// 状态机引擎
pub struct StateMachine {
    states: Arc<RwLock<HashMap<StateId, State>>>,
    transitions: Arc<RwLock<Vec<Transition>>>,
    current_state: Arc<RwLock<StateId>>,
    history: Arc<RwLock<Vec<StateId>>>,
    max_history: usize,
}

impl StateMachine {
    pub fn new(initial_state: State) -> Self {
        let initial_id = initial_state.id;
        let mut states = HashMap::new();
        states.insert(initial_id, initial_state);

        Self {
            states: Arc::new(RwLock::new(states)),
            transitions: Arc::new(RwLock::new(Vec::new())),
            current_state: Arc::new(RwLock::new(initial_id)),
            history: Arc::new(RwLock::new(vec![initial_id])),
            max_history: 1000,
        }
    }

    /// 添加状态
    pub async fn add_state(&self, state: State) -> Result<()> {
        let mut states = self.states.write().await;
        states.insert(state.id, state);
        Ok(())
    }

    /// 添加转换
    pub async fn add_transition(&self, transition: Transition) -> Result<()> {
        let mut transitions = self.transitions.write().await;
        transitions.push(transition);
        Ok(())
    }

    /// 获取当前状态
    pub async fn current_state(&self) -> Result<State> {
        let current_id = *self.current_state.read().await;
        let states = self.states.read().await;
        states.get(&current_id)
            .cloned()
            .context("Current state not found")
    }

    /// 执行状态转换
    #[instrument(skip(self))]
    pub async fn transition(&self, target_id: StateId) -> Result<State> {
        let current_id = *self.current_state.read().await;

        // 验证转换是否允许
        let transitions = self.transitions.read().await;
        let valid_transition = transitions.iter().any(|t| {
            t.from == current_id && t.to == target_id
        });

        if !valid_transition {
            return Err(AgentError::TransitionFailed {
                from: current_id,
                to: target_id,
                reason: "No valid transition defined".to_string(),
            }.into());
        }

        // 执行转换
        let mut current = self.current_state.write().await;
        *current = target_id;

        // 记录历史
        let mut history = self.history.write().await;
        history.push(target_id);
        if history.len() > self.max_history {
            history.remove(0);
        }

        // 返回新状态
        let states = self.states.read().await;
        states.get(&target_id)
            .cloned()
            .context("Target state not found")
    }

    /// 获取状态历史
    pub async fn get_history(&self) -> Vec<StateId> {
        self.history.read().await.clone()
    }

    /// 回滚到上一个状态
    pub async fn rollback(&self) -> Result<State> {
        let mut history = self.history.write().await;
        if history.len() < 2 {
            return Err(anyhow::anyhow!("Cannot rollback: no previous state"));
        }

        history.pop(); // 移除当前状态
        let previous_id = *history.last().unwrap();

        let mut current = self.current_state.write().await;
        *current = previous_id;

        let states = self.states.read().await;
        states.get(&previous_id)
            .cloned()
            .context("Previous state not found")
    }
}

// ============================================================================
// 4. Actor系统 - Actor System
// ============================================================================

/// Actor消息类型
#[derive(Debug, Clone)]
pub enum ActorMessage {
    /// 执行状态转换
    Transition { target: StateId, respond_to: mpsc::Sender<Result<State>> },
    /// 获取当前状态
    GetState { respond_to: mpsc::Sender<Result<State>> },
    /// 添加新状态
    AddState { state: State, respond_to: mpsc::Sender<Result<()>> },
    /// 验证状态
    Validate { state_id: StateId, respond_to: mpsc::Sender<Result<bool>> },
    /// 停止Actor
    Stop,
}

/// 状态空间Actor
pub struct StateSpaceActor {
    state_machine: Arc<StateMachine>,
    receiver: mpsc::Receiver<ActorMessage>,
    validator: Arc<StateValidator>,
}

impl StateSpaceActor {
    pub fn new(
        state_machine: Arc<StateMachine>,
        receiver: mpsc::Receiver<ActorMessage>,
        validator: Arc<StateValidator>,
    ) -> Self {
        Self {
            state_machine,
            receiver,
            validator,
        }
    }

    pub async fn run(mut self) {
        info!("StateSpaceActor started");

        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ActorMessage::Transition { target, respond_to } => {
                    let result = self.state_machine.transition(target).await;
                    let _ = respond_to.send(result).await;
                }
                ActorMessage::GetState { respond_to } => {
                    let result = self.state_machine.current_state().await;
                    let _ = respond_to.send(result).await;
                }
                ActorMessage::AddState { state, respond_to } => {
                    let result = self.state_machine.add_state(state).await;
                    let _ = respond_to.send(result).await;
                }
                ActorMessage::Validate { state_id, respond_to } => {
                    let result = self.validate_state(state_id).await;
                    let _ = respond_to.send(result).await;
                }
                ActorMessage::Stop => {
                    info!("StateSpaceActor received stop signal");
                    break;
                }
            }
        }

        info!("StateSpaceActor stopped");
    }

    async fn validate_state(&self, state_id: StateId) -> Result<bool> {
        let states = self.state_machine.states.read().await;
        if let Some(state) = states.get(&state_id) {
            self.validator.validate(state).await
        } else {
            Err(AgentError::StateNotFound(state_id).into())
        }
    }
}

/// Actor句柄 - 用于与Actor通信
#[derive(Clone)]
pub struct StateSpaceHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl StateSpaceHandle {
    pub fn new(sender: mpsc::Sender<ActorMessage>) -> Self {
        Self { sender }
    }

    pub async fn transition(&self, target: StateId) -> Result<State> {
        let (tx, mut rx) = mpsc::channel(1);
        let msg = ActorMessage::Transition { target, respond_to: tx };
        self.sender.send(msg).await?;
        rx.recv().await.context("Actor response channel closed")?
    }

    pub async fn get_state(&self) -> Result<State> {
        let (tx, mut rx) = mpsc::channel(1);
        let msg = ActorMessage::GetState { respond_to: tx };
        self.sender.send(msg).await?;
        rx.recv().await.context("Actor response channel closed")?
    }

    pub async fn stop(&self) -> Result<()> {
        self.sender.send(ActorMessage::Stop).await?;
        Ok(())
    }
}

// ============================================================================
// 5. 验证系统 - Validation System
// ============================================================================

/// 状态验证器
pub struct StateValidator {
    rules: Vec<Arc<dyn ValidationRuleTrait + Send + Sync>>,
}

impl StateValidator {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: Arc<dyn ValidationRuleTrait + Send + Sync>) {
        self.rules.push(rule);
    }

    pub async fn validate(&self, state: &State) -> Result<bool> {
        for rule in &self.rules {
            if !rule.validate(state).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// 验证规则trait
#[async_trait::async_trait]
pub trait ValidationRuleTrait: std::fmt::Debug {
    async fn validate(&self, state: &State) -> Result<bool>;
    fn name(&self) -> &str;
}

/// 必需字段验证规则
#[derive(Debug)]
pub struct RequiredFieldRule {
    pub field: String,
}

#[async_trait::async_trait]
impl ValidationRuleTrait for RequiredFieldRule {
    async fn validate(&self, state: &State) -> Result<bool> {
        // 实现字段存在性检查
        Ok(true)
    }

    fn name(&self) -> &str {
        "required_field"
    }
}

// ============================================================================
// 6. LLM集成层 - LLM Integration Layer
// ============================================================================

/// LLM提供者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub name: String,
    pub api_endpoint: String,
    pub api_key: String,
    pub model: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub temperature: f32,
}

/// LLM客户端trait
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, prompt: &str, schema: Option<&serde_json::Value>) -> Result<String>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// 结构化输出请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredRequest<T: JsonSchema> {
    pub prompt: String,
    pub schema: serde_json::Value,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<T: JsonSchema> StructuredRequest<T> {
    pub fn new(prompt: String) -> Self {
        let schema = schemars::schema_for!(T);
        Self {
            prompt,
            schema: serde_json::to_value(schema).unwrap(),
            _phantom: std::marker::PhantomData,
        }
    }
}

/// 断路器模式实现
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout_ms: u64,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    last_failure_time: Arc<Mutex<Option<std::time::Instant>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            reset_timeout_ms,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn call<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Open => {
                let last_failure = *self.last_failure_time.lock().await;
                if let Some(last) = last_failure {
                    let elapsed = last.elapsed().as_millis() as u64;
                    if elapsed > self.reset_timeout_ms {
                        *state = CircuitState::HalfOpen;
                    } else {
                        return Err(AgentError::CircuitBreakerOpen("LLM provider".to_string()).into());
                    }
                }
            }
            CircuitState::HalfOpen => {
                // 允许一个请求通过测试
            }
            CircuitState::Closed => {}
        }

        drop(state);

        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    async fn on_success(&self) {
        let mut state = self.state.lock().await;
        let mut count = self.failure_count.lock().await;
        *state = CircuitState::Closed;
        *count = 0;
    }

    async fn on_failure(&self) {
        let mut count = self.failure_count.lock().await;
        *count += 1;

        if *count >= self.failure_threshold {
            let mut state = self.state.lock().await;
            let mut last_time = self.last_failure_time.lock().await;
            *state = CircuitState::Open;
            *last_time = Some(std::time::Instant::now());
        }
    }
}

// ============================================================================
// 7. 缓存系统 - Caching System
// ============================================================================

/// 语义缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub embedding: Vec<f32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

/// 语义缓存
pub struct SemanticCache {
    entries: Arc<RwLock<Vec<CacheEntry>>>,
    similarity_threshold: f32,
}

impl SemanticCache {
    pub fn new(similarity_threshold: f32) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            similarity_threshold,
        }
    }

    /// 计算余弦相似度
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (norm_a * norm_b)
    }

    /// 获取缓存条目
    pub async fn get(&self, embedding: &[f32]) -> Option<CacheEntry> {
        let entries = self.entries.read().await;

        entries.iter()
            .filter(|e| {
                let age = chrono::Utc::now().signed_duration_since(e.created_at);
                age.num_seconds() < e.ttl_seconds as i64
            })
            .max_by(|a, b| {
                let sim_a = Self::cosine_similarity(embedding, &a.embedding);
                let sim_b = Self::cosine_similarity(embedding, &b.embedding);
                sim_a.partial_cmp(&sim_b).unwrap()
            })
            .filter(|e| {
                let similarity = Self::cosine_similarity(embedding, &e.embedding);
                similarity >= self.similarity_threshold
            })
            .cloned()
    }

    /// 设置缓存条目
    pub async fn set(&self, entry: CacheEntry) {
        let mut entries = self.entries.write().await;

        // 清理过期条目
        let now = chrono::Utc::now();
        entries.retain(|e| {
            let age = now.signed_duration_since(e.created_at);
            age.num_seconds() < e.ttl_seconds as i64
        });

        entries.push(entry);
    }
}

// ============================================================================
// 8. 内存管理 - Memory Management
// ============================================================================

/// 分层内存系统
pub struct LayeredMemory {
    /// 短期记忆 - 当前会话
    short_term: Arc<RwLock<Vec<MemoryEntry>>>,
    /// 中期记忆 - 向量存储
    medium_term: Arc<dyn VectorStore>,
    /// 长期记忆 - 持久化存储
    long_term: Arc<dyn PersistentStore>,
}

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub memory_type: MemoryType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub importance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Episodic,   // 事件记忆
    Semantic,   // 语义记忆
    Procedural, // 程序记忆
}

/// 向量存储trait
#[async_trait::async_trait]
pub trait VectorStore: Send + Sync {
    async fn store(&self, entry: &MemoryEntry) -> Result<()>;
    async fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<MemoryEntry>>;
    async fn delete(&self, id: &str) -> Result<()>;
}

/// 持久化存储trait
#[async_trait::async_trait]
pub trait PersistentStore: Send + Sync {
    async fn save(&self, entry: &MemoryEntry) -> Result<()>;
    async fn load(&self, id: &str) -> Result<Option<MemoryEntry>>;
    async fn query(&self, filters: &HashMap<String, String>) -> Result<Vec<MemoryEntry>>;
}

// ============================================================================
// 9. 可观测性 - Observability
// ============================================================================

/// 追踪上下文
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    pub fn new() -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: None,
        }
    }

    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: Some(self.span_id.clone()),
        }
    }
}

/// 指标收集器
pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, u64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn increment_counter(&self, name: &str, value: u64) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += value;
    }

    pub async fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        histograms.entry(name.to_string()).or_insert_with(Vec::new).push(value);
    }

    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }
}

// ============================================================================
// 10. 构建器模式 - Builder Pattern
// ============================================================================

/// Agent配置构建器
pub struct AgentConfigBuilder {
    max_iterations: Option<u32>,
    timeout_ms: Option<u64>,
    enable_caching: Option<bool>,
    cache_ttl_seconds: Option<u64>,
    llm_config: Option<LlmProviderConfig>,
    validation_enabled: Option<bool>,
}

impl AgentConfigBuilder {
    pub fn new() -> Self {
        Self {
            max_iterations: None,
            timeout_ms: None,
            enable_caching: None,
            cache_ttl_seconds: None,
            llm_config: None,
            validation_enabled: None,
        }
    }

    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = Some(max);
        self
    }

    pub fn with_timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = Some(timeout);
        self
    }

    pub fn with_caching(mut self, enabled: bool, ttl_seconds: u64) -> Self {
        self.enable_caching = Some(enabled);
        self.cache_ttl_seconds = Some(ttl_seconds);
        self
    }

    pub fn with_llm_config(mut self, config: LlmProviderConfig) -> Self {
        self.llm_config = Some(config);
        self
    }

    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validation_enabled = Some(enabled);
        self
    }

    pub fn build(self) -> Result<AgentConfig> {
        Ok(AgentConfig {
            max_iterations: self.max_iterations.unwrap_or(100),
            timeout_ms: self.timeout_ms.unwrap_or(30000),
            enable_caching: self.enable_caching.unwrap_or(true),
            cache_ttl_seconds: self.cache_ttl_seconds.unwrap_or(3600),
            llm_config: self.llm_config.context("LLM config is required")?,
            validation_enabled: self.validation_enabled.unwrap_or(true),
        })
    }
}

/// Agent配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub timeout_ms: u64,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub llm_config: LlmProviderConfig,
    pub validation_enabled: bool,
}

// ============================================================================
// 11. 主Agent结构 - Main Agent Structure
// ============================================================================

/// 状态空间Agent
pub struct StateSpaceAgent {
    config: AgentConfig,
    state_machine: Arc<StateMachine>,
    actor_handle: StateSpaceHandle,
    cache: Option<SemanticCache>,
    circuit_breaker: CircuitBreaker,
    metrics: MetricsCollector,
    memory: Option<LayeredMemory>,
}

impl StateSpaceAgent {
    pub async fn new(config: AgentConfig) -> Result<(Self, JoinHandle<()>)> {
        // 创建初始状态
        let initial_state = State {
            id: StateId::new(),
            state_type: StateType::Initial,
            metadata: StateMetadata::default(),
            validation_rules: Vec::new(),
            created_at: chrono::Utc::now(),
            version: 1,
        };

        // 创建状态机
        let state_machine = Arc::new(StateMachine::new(initial_state));

        // 创建Actor
        let (tx, rx) = mpsc::channel(100);
        let validator = Arc::new(StateValidator::new());
        let actor = StateSpaceActor::new(
            Arc::clone(&state_machine),
            rx,
            validator,
        );

        let actor_handle = StateSpaceHandle::new(tx);
        let actor_task = tokio::spawn(actor.run());

        // 创建缓存
        let cache = if config.enable_caching {
            Some(SemanticCache::new(0.85))
        } else {
            None
        };

        let agent = Self {
            config,
            state_machine,
            actor_handle,
            cache,
            circuit_breaker: CircuitBreaker::new(5, 60000),
            metrics: MetricsCollector::new(),
            memory: None,
        };

        Ok((agent, actor_task))
    }

    /// 执行Agent直到完成或达到限制
    #[instrument(skip(self))]
    pub async fn execute(&self, initial_prompt: &str) -> Result<State> {
        info!("Starting agent execution");

        let start_time = std::time::Instant::now();
        let mut iterations = 0u32;

        loop {
            // 检查迭代限制
            if iterations >= self.config.max_iterations {
                return Err(AgentError::MaxIterationsExceeded {
                    max: self.config.max_iterations,
                }.into());
            }

            // 检查超时
            if start_time.elapsed().as_millis() as u64 > self.config.timeout_ms {
                return Err(AgentError::Timeout {
                    duration_ms: self.config.timeout_ms,
                }.into());
            }

            // 获取当前状态
            let current_state = self.actor_handle.get_state().await?;

            // 检查是否到达终止状态
            if matches!(current_state.state_type, StateType::Terminal | StateType::Completed { .. }) {
                info!("Agent reached terminal state");
                return Ok(current_state);
            }

            // 执行状态转换决策
            let next_state_id = self.decide_next_state(&current_state, initial_prompt).await?;

            // 执行转换
            let _ = self.actor_handle.transition(next_state_id).await?;

            iterations += 1;
            self.metrics.increment_counter("agent_iterations", 1).await;
        }
    }

    /// 决定下一个状态（核心决策逻辑）
    async fn decide_next_state(&self, current: &State, prompt: &str) -> Result<StateId> {
        // 这里集成LLM决策逻辑
        // 简化实现：返回一个新的处理状态
        let next_state = State {
            id: StateId::new(),
            state_type: StateType::Processing { step: "decision".to_string() },
            metadata: StateMetadata::default(),
            validation_rules: Vec::new(),
            created_at: chrono::Utc::now(),
            version: current.version + 1,
        };

        let next_id = next_state.id;
        self.actor_handle.add_state(next_state).await?;

        Ok(next_id)
    }

    /// 停止Agent
    pub async fn stop(&self) -> Result<()> {
        self.actor_handle.stop().await?;
        Ok(())
    }
}

// ============================================================================
// 12. 测试示例 - Test Examples
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_machine_basic() {
        let initial = State {
            id: StateId::new(),
            state_type: StateType::Initial,
            metadata: StateMetadata::default(),
            validation_rules: Vec::new(),
            created_at: chrono::Utc::now(),
            version: 1,
        };

        let sm = StateMachine::new(initial.clone());
        let current = sm.current_state().await.unwrap();

        assert_eq!(current.id, initial.id);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(2, 1000);

        // 前两次调用失败应该打开断路器
        let result1 = cb.call(|| async { Err::<(), _>(anyhow::anyhow!("error")) }).await;
        assert!(result1.is_err());

        let result2 = cb.call(|| async { Err::<(), _>(anyhow::anyhow!("error")) }).await;
        assert!(result2.is_err());

        // 第三次应该直接返回断路器打开错误
        let result3 = cb.call(|| async { Ok::<(), _>(()) }).await;
        assert!(result3.is_err());
        assert!(result3.unwrap_err().to_string().contains("Circuit breaker"));
    }

    #[test]
    fn test_builder_pattern() {
        let config = AgentConfigBuilder::new()
            .with_max_iterations(50)
            .with_timeout_ms(60000)
            .with_caching(true, 7200)
            .with_llm_config(LlmProviderConfig {
                name: "test".to_string(),
                api_endpoint: "http://test".to_string(),
                api_key: "test".to_string(),
                model: "gpt-4".to_string(),
                timeout_ms: 30000,
                max_retries: 3,
                temperature: 0.0,
            })
            .with_validation(true)
            .build();

        assert!(config.is_ok());
        let cfg = config.unwrap();
        assert_eq!(cfg.max_iterations, 50);
        assert_eq!(cfg.timeout_ms, 60000);
    }
}

// ============================================================================
// 使用示例 - Usage Example
// ============================================================================

/*
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 构建配置
    let config = AgentConfigBuilder::new()
        .with_max_iterations(100)
        .with_timeout_ms(60000)
        .with_caching(true, 3600)
        .with_llm_config(LlmProviderConfig {
            name: "openai".to_string(),
            api_endpoint: "https://api.openai.com/v1".to_string(),
            api_key: std::env::var("OPENAI_API_KEY")?,
            model: "gpt-4".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
            temperature: 0.0,
        })
        .with_validation(true)
        .build()?;

    // 创建Agent
    let (agent, actor_task) = StateSpaceAgent::new(config).await?;

    // 执行Agent
    let result = agent.execute("Process this request").await?;
    println!("Final state: {:?}", result);

    // 停止Agent
    agent.stop().await?;
    actor_task.await?;

    Ok(())
}
*/
