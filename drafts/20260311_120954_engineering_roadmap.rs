//! State Space Agent Architecture - Production Engineering Roadmap
//!
//! This module implements a production-ready state space agent system based on 2025 research:
//! - Multi-Agent Mamba (MAM) - Selective State Space Models for multi-agent systems
//! - BrainChip TENN - Edge-optimized SSM with <0.5W power consumption
//! - Deterministic Simulation Testing (DST) patterns from Polar Signals
//! - Actor Model + State Machine hybrid architecture
//!
//! References:
//! - Multi-Agent Mamba (MAM) - AAMAS 2025
//! - BrainChip TENN - 1B parameter SSM under 0.5W
//! - Deterministic Simulation Testing in Rust - Polar Signals 2025
//! - ADK-Rust - Production AI Agent Framework
//!
//! Architecture: Thin Agent + Fat Platform with SSM-based memory

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// Core State Space Model (S4/S5/Mamba) Implementation
// ============================================================================

/// Selective State Space Model with input-dependent dynamics
/// Based on Mamba architecture: h'(t) = A(t)*h(t) + B(t)*x(t)
/// where A, B are input-dependent (selective)
#[derive(Debug, Clone)]
pub struct SelectiveStateSpaceModel {
    /// Base state transition matrix (n_states x n_states) - HiPPO initialized
    pub a_base: Vec<Vec<f64>>,
    /// Input projection for selective A (n_states x n_inputs)
    pub a_proj: Vec<Vec<f64>>,
    /// Base input matrix (n_states x n_inputs)
    pub b_base: Vec<Vec<f64>>,
    /// Input projection for selective B (n_states x n_inputs)
    pub b_proj: Vec<Vec<f64>>,
    /// Output matrix (n_outputs x n_states)
    pub c: Vec<Vec<f64>>,
    /// Feedthrough matrix (n_outputs x n_inputs)
    pub d: Vec<Vec<f64>>,
    /// State vector
    pub state: Vec<f64>,
    /// Discretization step size
    pub delta: f64,
    /// Dimensions
    pub n_states: usize,
    pub n_inputs: usize,
    pub n_outputs: usize,
}

impl SelectiveStateSpaceModel {
    /// Create new SSM with HiPPO-inspired initialization for long-range memory
    pub fn new(n_states: usize, n_inputs: usize, n_outputs: usize) -> Self {
        // HiPPO initialization: A[i,j] = (2i+1)^(1/2) * (2j+1)^(1/2) * integral
        let mut a_base = vec![vec![0.0; n_states]; n_states];

        for i in 0..n_states {
            for j in 0..n_states {
                if i > j {
                    // Lower triangular: (2i+1)^(1/2) * (2j+1)^(1/2)
                    a_base[i][j] = ((2.0 * i as f64 + 1.0) * (2.0 * j as f64 + 1.0)).sqrt();
                } else if i == j {
                    // Diagonal: -(i + 0.5)
                    a_base[i][j] = -(i as f64 + 0.5);
                }
                // Upper triangular remains 0 (causal structure)
            }
        }

        // Input coupling with decreasing importance
        let mut b_base = vec![vec![0.0; n_inputs]; n_states];
        for i in 0..n_states {
            for j in 0..n_inputs {
                b_base[i][j] = ((2.0 * i as f64 + 1.0)).sqrt() / (j as f64 + 1.0);
            }
        }

        // Projection matrices for selectivity
        let a_proj = vec![vec![0.01; n_inputs]; n_states];
        let b_proj = vec![vec![0.01; n_inputs]; n_states];

        // Output projection
        let c = vec![vec![1.0 / n_states as f64; n_states]; n_outputs];
        let d = vec![vec![0.0; n_inputs]; n_outputs];

        Self {
            a_base,
            a_proj,
            b_base,
            b_proj,
            c,
            d,
            state: vec![0.0; n_states],
            delta: 0.001,
            n_states,
            n_inputs,
            n_outputs,
        }
    }

    /// Compute selective parameters based on input
    /// A(t) = A_base + proj_A(x_t)
    /// B(t) = B_base + proj_B(x_t)
    fn compute_selective_params(&self, input: &[f64]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        // Simplified: linear projection of input to parameter adjustments
        let mut a_selective = self.a_base.clone();
        let mut b_selective = self.b_base.clone();

        // Add input-dependent adjustments
        for i in 0..self.n_states {
            for j in 0..self.n_states {
                if j < input.len() && j < self.n_inputs {
                    a_selective[i][j] += self.a_proj[i][j % self.n_inputs] * input[j % input.len()];
                }
            }
            for j in 0..self.n_inputs {
                if j < input.len() {
                    b_selective[i][j] += self.b_proj[i][j] * input[j];
                }
            }
        }

        (a_selective, b_selective)
    }

    /// Discretize using Zero-Order Hold with selective parameters
    pub fn discretize(&self, a: &[Vec<f64>], b: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        // Simplified ZOH: A_discrete = I + delta * A
        let mut a_disc = vec![vec![0.0; self.n_states]; self.n_states];
        let mut b_disc = vec![vec![0.0; self.n_inputs]; self.n_states];

        for i in 0..self.n_states {
            for j in 0..self.n_states {
                a_disc[i][j] = if i == j { 1.0 } else { 0.0 } + self.delta * a[i][j];
            }
            for j in 0..self.n_inputs {
                b_disc[i][j] = self.delta * b[i][j];
            }
        }

        (a_disc, b_disc)
    }

    /// Forward step with selective state space dynamics
    pub fn step(&mut self, input: &[f64]) -> Vec<f64> {
        // Compute selective parameters
        let (a_sel, b_sel) = self.compute_selective_params(input);

        // Discretize
        let (a_disc, b_disc) = self.discretize(&a_sel, &b_sel);

        // Update state: h = A_disc * h + B_disc * x
        let mut new_state = vec![0.0; self.n_states];
        for i in 0..self.n_states {
            for j in 0..self.n_states {
                new_state[i] += a_disc[i][j] * self.state[j];
            }
            for j in 0..self.n_inputs.min(input.len()) {
                new_state[i] += b_disc[i][j] * input[j];
            }
        }
        self.state = new_state;

        // Compute output: y = C * h + D * x
        let mut output = vec![0.0; self.n_outputs];
        for i in 0..self.n_outputs {
            for j in 0..self.n_states {
                output[i] += self.c[i][j] * self.state[j];
            }
            for j in 0..self.n_inputs.min(input.len()) {
                output[i] += self.d[i][j] * input[j];
            }
        }

        output
    }

    /// Process sequence with selective scanning
    pub fn forward_sequence(&mut self, inputs: &[Vec<f64>]) -> Vec<Vec<f64>> {
        inputs.iter().map(|input| self.step(input)).collect()
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.state = vec![0.0; self.n_states];
    }

    /// Get current state summary
    pub fn state_summary(&self) -> StateSummary {
        let sum: f64 = self.state.iter().sum();
        let sum_sq: f64 = self.state.iter().map(|x| x * x).sum();
        StateSummary {
            mean: sum / self.n_states as f64,
            variance: sum_sq / self.n_states as f64 - (sum / self.n_states as f64).powi(2),
            max_activation: self.state.iter().map(|x| x.abs()).fold(0.0, f64::max),
            sparsity: self.state.iter().filter(|&&x| x.abs() < 0.01).count() as f64 / self.n_states as f64,
        }
    }
}

/// State summary for observability
#[derive(Debug, Clone, Copy)]
pub struct StateSummary {
    pub mean: f64,
    pub variance: f64,
    pub max_activation: f64,
    pub sparsity: f64,
}

// ============================================================================
// Actor Model + State Machine Hybrid Architecture
// ============================================================================

/// Message types for deterministic agent communication
#[derive(Debug, Clone, PartialEq)]
pub enum AgentMessage {
    /// Observation from environment (vector embedding)
    Observation(Vec<f64>),
    /// Action request with parameters
    Action { action_type: String, params: HashMap<String, f64> },
    /// Query for information
    Query { query_id: String, content: String },
    /// Response to query
    Response { query_id: String, data: Vec<f64> },
    /// State update notification
    StateUpdate(AgentState),
    /// Control signal
    Control(ControlSignal),
    /// Cross-agent message (for multi-agent coordination)
    CrossAgent { from: String, to: String, payload: Box<AgentMessage> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlSignal {
    Start,
    Pause,
    Resume,
    Stop,
    Reset,
    Checkpoint,
}

/// Agent states for type-safe state machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentState {
    Initialized,
    Idle,
    Perceiving,
    Reasoning,
    Planning,
    Acting,
    WaitingForResponse,
    Error,
    Terminated,
}

/// Destination for message routing
#[derive(Debug, Clone, PartialEq)]
pub enum Destination {
    Agent(String),
    Broadcast,
    Environment,
    Platform,
    SelfRef,
}

/// Core trait for deterministic state machine agents
/// Based on DST (Deterministic Simulation Testing) patterns
pub trait StateMachineAgent: Send + Sync {
    /// Process message, return outgoing messages
    fn receive(&mut self, msg: AgentMessage, timestamp: Instant) -> Vec<(AgentMessage, Destination)>;

    /// Periodic tick for time-based operations
    fn tick(&mut self, current_time: Instant) -> Vec<(AgentMessage, Destination)>;

    /// Get current state
    fn state(&self) -> AgentState;

    /// Get agent ID
    fn id(&self) -> &str;

    /// Reset to initial state
    fn reset(&mut self);
}

// ============================================================================
// SSM-Based Agent Implementation
// ============================================================================

/// Production-ready State Space Agent
/// Combines Selective SSM with ReAct loop and Actor model
pub struct SsmAgent {
    /// Unique identifier
    id: String,
    /// Current state machine state
    current_state: AgentState,
    /// Selective State Space Model for memory
    ssm: SelectiveStateSpaceModel,
    /// Working memory buffer (circular)
    working_memory: VecDeque<Vec<f64>>,
    /// Configuration
    config: AgentConfig,
    /// Message queue for async processing
    message_queue: VecDeque<(AgentMessage, Instant)>,
    /// Last activity timestamp
    last_activity: Instant,
    /// Creation timestamp
    created_at: Instant,
    /// Pending queries (query_id -> timestamp)
    pending_queries: HashMap<String, Instant>,
    /// Action history for learning
    action_history: VecDeque<ActionRecord>,
}

#[derive(Debug, Clone)]
struct ActionRecord {
    timestamp: Instant,
    action: String,
    observation: Vec<f64>,
    outcome: f64, // Reward signal
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub state_dim: usize,
    pub input_dim: usize,
    pub output_dim: usize,
    pub working_memory_size: usize,
    pub operation_timeout: Duration,
    pub max_pending_queries: usize,
    pub action_history_size: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            state_dim: 64,
            input_dim: 16,
            output_dim: 8,
            working_memory_size: 100,
            operation_timeout: Duration::from_secs(30),
            max_pending_queries: 10,
            action_history_size: 1000,
        }
    }
}

impl SsmAgent {
    pub fn new(id: String, config: AgentConfig) -> Self {
        let ssm = SelectiveStateSpaceModel::new(config.state_dim, config.input_dim, config.output_dim);
        let now = Instant::now();

        Self {
            id,
            current_state: AgentState::Initialized,
            ssm,
            working_memory: VecDeque::with_capacity(config.working_memory_size),
            config,
            message_queue: VecDeque::new(),
            last_activity: now,
            created_at: now,
            pending_queries: HashMap::new(),
            action_history: VecDeque::with_capacity(1000),
        }
    }

    /// ReAct loop: Perceive -> Reason -> Act
    fn react_loop(&mut self, observation: Vec<f64>, timestamp: Instant) -> Vec<(AgentMessage, Destination)> {
        let mut responses = Vec::new();

        // 1. PERCEIVE: Process through SSM
        self.current_state = AgentState::Perceiving;
        let processed = self.ssm.step(&observation);

        // Update working memory (circular buffer)
        self.working_memory.push_back(processed.clone());
        if self.working_memory.len() > self.config.working_memory_size {
            self.working_memory.pop_front();
        }

        // 2. REASON: Use SSM state for decision making
        self.current_state = AgentState::Reasoning;
        let action = self.decide_action(&processed, &observation);

        // 3. PLAN: Simple planning based on action type
        self.current_state = AgentState::Planning;
        let plan = self.create_plan(&action);

        // 4. ACT: Generate action messages
        self.current_state = AgentState::Acting;
        for planned_action in plan {
            responses.push((
                AgentMessage::Action {
                    action_type: planned_action,
                    params: self.extract_params(&processed),
                },
                Destination::Environment,
            ));
        }

        // Record action
        self.action_history.push_back(ActionRecord {
            timestamp,
            action: action.clone(),
            observation: observation.clone(),
            outcome: 0.0, // Will be updated later
        });
        if self.action_history.len() > self.config.action_history_size {
            self.action_history.pop_front();
        }

        self.current_state = AgentState::Idle;
        self.last_activity = timestamp;

        responses
    }

    /// Decide action based on SSM state and observation
    fn decide_action(&self, state: &[f64], observation: &[f64]) -> String {
        // Compute statistics
        let state_sum: f64 = state.iter().sum();
        let state_avg = state_sum / state.len() as f64;

        let obs_sum: f64 = observation.iter().sum();
        let obs_avg = obs_sum / observation.len().max(1) as f64;

        // Simple decision logic (can be replaced with learned policy)
        if obs_avg > 0.8 {
            "urgent_execute".to_string()
        } else if obs_avg > 0.5 {
            "normal_execute".to_string()
        } else if state_avg > 0.3 {
            "exploit".to_string()
        } else {
            "explore".to_string()
        }
    }

    /// Create action plan
    fn create_plan(&self, action: &str) -> Vec<String> {
        match action {
            "urgent_execute" => vec!["validate".to_string(), "execute".to_string(), "notify".to_string()],
            "normal_execute" => vec!["validate".to_string(), "execute".to_string()],
            "exploit" => vec!["retrieve_memory".to_string(), "apply_strategy".to_string()],
            "explore" => vec!["gather_info".to_string(), "experiment".to_string()],
            _ => vec![action.to_string()],
        }
    }

    /// Extract parameters from state
    fn extract_params(&self, state: &[f64]) -> HashMap<String, f64> {
        let mut params = HashMap::new();
        if !state.is_empty() {
            params.insert("confidence".to_string(), state[0].abs().min(1.0));
        }
        if state.len() > 1 {
            params.insert("urgency".to_string(), state[1].abs().min(1.0));
        }
        params
    }

    /// Handle query with SSM state as context
    fn handle_query(&mut self, query_id: String, content: String) -> Vec<(AgentMessage, Destination)> {
        // Store pending query
        if self.pending_queries.len() >= self.config.max_pending_queries {
            // Remove oldest
            if let Some(oldest) = self.pending_queries.iter().min_by_key(|(_, t)| *t) {
                let oldest_id = oldest.0.clone();
                self.pending_queries.remove(&oldest_id);
            }
        }
        self.pending_queries.insert(query_id.clone(), Instant::now());

        // Respond with SSM state summary
        let summary = self.ssm.state_summary();
        let response_data = vec![
            summary.mean,
            summary.variance,
            summary.max_activation,
            summary.sparsity,
        ];

        vec![(
            AgentMessage::Response {
                query_id,
                data: response_data,
            },
            Destination::Environment,
        )]
    }

    /// Get metrics for observability
    pub fn metrics(&self) -> AgentMetrics {
        AgentMetrics {
            id: self.id.clone(),
            state: self.current_state,
            uptime: Instant::now().duration_since(self.created_at),
            ssm_summary: self.ssm.state_summary(),
            working_memory_size: self.working_memory.len(),
            pending_queries: self.pending_queries.len(),
            action_history_size: self.action_history.len(),
        }
    }
}

/// Agent metrics for monitoring
#[derive(Debug, Clone)]
pub struct AgentMetrics {
    pub id: String,
    pub state: AgentState,
    pub uptime: Duration,
    pub ssm_summary: StateSummary,
    pub working_memory_size: usize,
    pub pending_queries: usize,
    pub action_history_size: usize,
}

impl StateMachineAgent for SsmAgent {
    fn receive(&mut self, msg: AgentMessage, timestamp: Instant) -> Vec<(AgentMessage, Destination)> {
        match msg {
            AgentMessage::Observation(data) => {
                self.react_loop(data, timestamp)
            }
            AgentMessage::Query { query_id, content } => {
                self.handle_query(query_id, content)
            }
            AgentMessage::Response { query_id, data } => {
                // Handle response to our query
                self.pending_queries.remove(&query_id);
                // Process response data
                self.working_memory.push_back(data);
                if self.working_memory.len() > self.config.working_memory_size {
                    self.working_memory.pop_front();
                }
                vec![]
            }
            AgentMessage::Control(ControlSignal::Reset) => {
                self.reset();
                vec![]
            }
            AgentMessage::Control(ControlSignal::Stop) => {
                self.current_state = AgentState::Terminated;
                vec![]
            }
            AgentMessage::Control(ControlSignal::Pause) => {
                self.current_state = AgentState::WaitingForResponse;
                vec![]
            }
            AgentMessage::Control(ControlSignal::Resume) => {
                self.current_state = AgentState::Idle;
                vec![]
            }
            AgentMessage::CrossAgent { from, to, payload } => {
                // Handle cross-agent communication
                if to == self.id {
                    self.receive(*payload, timestamp)
                } else {
                    // Forward to intended recipient
                    vec![(AgentMessage::CrossAgent { from, to, payload }, Destination::Platform)]
                }
            }
            _ => vec![],
        }
    }

    fn tick(&mut self, current_time: Instant) -> Vec<(AgentMessage, Destination)> {
        // Check for timeout
        if current_time.duration_since(self.last_activity) > self.config.operation_timeout {
            if self.current_state != AgentState::WaitingForResponse {
                self.current_state = AgentState::Idle;
            }
        }

        // Process queued messages
        if let Some((msg, _)) = self.message_queue.pop_front() {
            return self.receive(msg, current_time);
        }

        // Check for stale pending queries
        let stale_queries: Vec<String> = self.pending_queries
            .iter()
            .filter(|(_, t)| current_time.duration_since(**t) > self.config.operation_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for query_id in stale_queries {
            self.pending_queries.remove(&query_id);
        }

        vec![]
    }

    fn state(&self) -> AgentState {
        self.current_state
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn reset(&mut self) {
        self.ssm.reset();
        self.working_memory.clear();
        self.current_state = AgentState::Initialized;
        self.pending_queries.clear();
        self.action_history.clear();
    }
}

// ============================================================================
// Multi-Agent Orchestrator (MAM-style)
// ============================================================================

/// Multi-Agent Mamba-style orchestrator
/// Manages multiple SSM-based agents with cross-attention-like coordination
pub struct MultiAgentOrchestrator {
    /// Registered agents
    agents: HashMap<String, Arc<Mutex<SsmAgent>>>,
    /// Message bus for routing
    message_bus: VecDeque<(AgentMessage, Destination, Instant)>,
    /// Configuration
    config: OrchestratorConfig,
    /// Global state for cross-agent coordination
    shared_context: SharedContext,
    /// Start time
    start_time: Instant,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_agents: usize,
    pub message_retention: Duration,
    pub enable_cross_agent: bool,
    pub deterministic: bool,
    pub tick_interval_ms: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_agents: 10,
            message_retention: Duration::from_secs(60),
            enable_cross_agent: true,
            deterministic: false,
            tick_interval_ms: 100,
        }
    }
}

/// Shared context for cross-agent coordination
#[derive(Debug, Clone)]
struct SharedContext {
    /// Global state vector (aggregated from all agents)
    global_state: Vec<f64>,
    /// Agent contributions to global state
    agent_contributions: HashMap<String, Vec<f64>>,
}

impl SharedContext {
    fn new(dim: usize) -> Self {
        Self {
            global_state: vec![0.0; dim],
            agent_contributions: HashMap::new(),
        }
    }

    fn update_agent_contribution(&mut self, agent_id: &str, contribution: Vec<f64>) {
        self.agent_contributions.insert(agent_id.to_string(), contribution);
        // Recompute global state as average
        if !self.agent_contributions.is_empty() {
            let n = self.agent_contributions.len();
            for i in 0..self.global_state.len() {
                let sum: f64 = self.agent_contributions.values()
                    .map(|v| v.get(i).copied().unwrap_or(0.0))
                    .sum();
                self.global_state[i] = sum / n as f64;
            }
        }
    }
}

impl MultiAgentOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            agents: HashMap::new(),
            message_bus: VecDeque::new(),
            config,
            shared_context: SharedContext::new(64),
            start_time: Instant::now(),
        }
    }

    /// Register a new agent
    pub fn register_agent(&mut self, agent: SsmAgent) -> Result<(), String> {
        if self.agents.len() >= self.config.max_agents {
            return Err(format!("Maximum agent limit ({}) reached", self.config.max_agents));
        }

        let id = agent.id().to_string();
        self.agents.insert(id, Arc::new(Mutex::new(agent)));
        Ok(())
    }

    /// Send message to specific agent
    pub fn send_to_agent(&mut self, agent_id: &str, msg: AgentMessage, timestamp: Instant) -> Result<(), String> {
        let agent_arc = self.agents.get(agent_id)
            .ok_or_else(|| format!("Agent {} not found", agent_id))?
            .clone();

        let mut agent = agent_arc.lock().map_err(|e| e.to_string())?;
        let responses = agent.receive(msg, timestamp);

        // Route responses
        drop(agent); // Release lock before routing
        for (resp_msg, dest) in responses {
            self.route_message(resp_msg, dest, timestamp);
        }

        Ok(())
    }

    /// Broadcast message to all agents
    pub fn broadcast(&mut self, msg: AgentMessage, timestamp: Instant) {
        let agent_ids: Vec<String> = self.agents.keys().cloned().collect();

        for agent_id in agent_ids {
            let _ = self.send_to_agent(&agent_id, msg.clone(), timestamp);
        }
    }

    /// Route message to destination
    fn route_message(&mut self, msg: AgentMessage, dest: Destination, timestamp: Instant) {
        match dest {
            Destination::Agent(id) => {
                let _ = self.send_to_agent(&id, msg, timestamp);
            }
            Destination::Broadcast => {
                self.broadcast(msg, timestamp);
            }
            Destination::Environment => {
                // Log or handle environment messages
                println!("[Environment] {:?}", msg);
            }
            Destination::Platform => {
                // Add to message bus for platform processing
                self.message_bus.push_back((msg, dest, timestamp));
            }
            Destination::SelfRef => {
                // Internal routing - already handled
            }
        }
    }

    /// Tick all agents (deterministic simulation step)
    pub fn tick(&mut self, current_time: Instant) {
        // Collect all agent responses first
        let mut all_responses: Vec<(AgentMessage, Destination)> = Vec::new();

        for (_, agent_arc) in &self.agents {
            if let Ok(mut agent) = agent_arc.lock() {
                let responses = agent.tick(current_time);
                all_responses.extend(responses);

                // Update shared context with agent state
                let metrics = agent.metrics();
                let contribution = vec![
                    metrics.ssm_summary.mean,
                    metrics.ssm_summary.variance,
                    metrics.ssm_summary.max_activation,
                ];
                self.shared_context.update_agent_contribution(&agent.id(), contribution);
            }
        }

        // Route all responses
        for (msg, dest) in all_responses {
            self.route_message(msg, dest, current_time);
        }

        // Clean up old messages
        self.cleanup_messages(current_time);
    }

    /// Clean up old messages
    fn cleanup_messages(&mut self, current_time: Instant) {
        while let Some((_, _, timestamp)) = self.message_bus.front() {
            if current_time.duration_since(*timestamp) > self.config.message_retention {
                self.message_bus.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get orchestrator metrics
    pub fn metrics(&self) -> OrchestratorMetrics {
        OrchestratorMetrics {
            agent_count: self.agents.len(),
            message_bus_size: self.message_bus.len(),
            uptime: Instant::now().duration_since(self.start_time),
            shared_context_summary: self.shared_context.global_state.iter().sum::<f64>() / self.shared_context.global_state.len().max(1) as f64,
        }
    }

    /// Get agent metrics
    pub fn agent_metrics(&self) -> Vec<AgentMetrics> {
        self.agents.values()
            .filter_map(|agent_arc| {
                agent_arc.lock().ok().map(|agent| agent.metrics())
            })
            .collect()
    }
}

/// Orchestrator metrics
#[derive(Debug, Clone)]
pub struct OrchestratorMetrics {
    pub agent_count: usize,
    pub message_bus_size: usize,
    pub uptime: Duration,
    pub shared_context_summary: f64,
}

// ============================================================================
// Production System
// ============================================================================

/// Production-ready State Space Agent System
/// Thin Agent + Fat Platform architecture
pub struct StateSpaceAgentPlatform {
    /// Multi-agent orchestrator
    orchestrator: MultiAgentOrchestrator,
    /// System state
    system_state: SystemState,
    /// Configuration
    config: PlatformConfig,
    /// Start time
    start_time: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemState {
    Initializing,
    Running,
    Paused,
    ShuttingDown,
    Error,
}

#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub default_agent_config: AgentConfig,
    pub orchestrator_config: OrchestratorConfig,
    pub auto_initialize: bool,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            default_agent_config: AgentConfig::default(),
            orchestrator_config: OrchestratorConfig::default(),
            auto_initialize: true,
        }
    }
}

impl StateSpaceAgentPlatform {
    pub fn new(config: PlatformConfig) -> Self {
        let orchestrator = MultiAgentOrchestrator::new(config.orchestrator_config.clone());

        Self {
            orchestrator,
            system_state: SystemState::Initializing,
            config,
            start_time: Instant::now(),
        }
    }

    /// Initialize with default agent topology
    pub fn initialize(&mut self) -> Result<(), String> {
        // Create specialized agents with different SSM configurations

        // 1. Perception Agent: Small, fast SSM for sensory processing
        let perception_config = AgentConfig {
            state_dim: 32,
            input_dim: 16,
            output_dim: 8,
            working_memory_size: 50,
            ..Default::default()
        };
        let perception_agent = SsmAgent::new("perception".to_string(), perception_config);
        self.orchestrator.register_agent(perception_agent)?;

        // 2. Reasoning Agent: Large SSM for complex reasoning
        let reasoning_config = AgentConfig {
            state_dim: 256,
            input_dim: 32,
            output_dim: 16,
            working_memory_size: 200,
            ..Default::default()
        };
        let reasoning_agent = SsmAgent::new("reasoning".to_string(), reasoning_config);
        self.orchestrator.register_agent(reasoning_agent)?;

        // 3. Action Agent: Medium SSM for action selection
        let action_config = AgentConfig {
            state_dim: 64,
            input_dim: 16,
            output_dim: 32,
            working_memory_size: 100,
            ..Default::default()
        };
        let action_agent = SsmAgent::new("action".to_string(), action_config);
        self.orchestrator.register_agent(action_agent)?;

        // 4. Coordination Agent: Manages cross-agent communication
        let coord_config = AgentConfig {
            state_dim: 128,
            input_dim: 64,
            output_dim: 8,
            working_memory_size: 150,
            ..Default::default()
        };
        let coord_agent = SsmAgent::new("coordinator".to_string(), coord_config);
        self.orchestrator.register_agent(coord_agent)?;

        self.system_state = SystemState::Running;
        Ok(())
    }

    /// Process observation through the system
    pub fn process(&mut self, observation: Vec<f64>) -> Result<(), String> {
        if self.system_state != SystemState::Running {
            return Err("System not running".to_string());
        }

        let timestamp = Instant::now();

        // Send to perception agent
        self.orchestrator.send_to_agent(
            "perception",
            AgentMessage::Observation(observation),
            timestamp,
        )?;

        // Tick to process
        self.orchestrator.tick(timestamp);

        Ok(())
    }

    /// Run simulation tick
    pub fn tick(&mut self) {
        self.orchestrator.tick(Instant::now());
    }

    /// Get system metrics
    pub fn metrics(&self) -> PlatformMetrics {
        let orch_metrics = self.orchestrator.metrics();

        PlatformMetrics {
            system_state: self.system_state,
            uptime: Instant::now().duration_since(self.start_time),
            orchestrator: orch_metrics,
            agents: self.orchestrator.agent_metrics(),
        }
    }

    /// Shutdown the system
    pub fn shutdown(&mut self) {
        self.system_state = SystemState::ShuttingDown;

        // Send stop signal to all agents
        let timestamp = Instant::now();
        self.orchestrator.broadcast(
            AgentMessage::Control(ControlSignal::Stop),
            timestamp,
        );
    }
}

/// Platform metrics
#[derive(Debug, Clone)]
pub struct PlatformMetrics {
    pub system_state: SystemState,
    pub uptime: Duration,
    pub orchestrator: OrchestratorMetrics,
    pub agents: Vec<AgentMetrics>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selective_ssm_creation() {
        let ssm = SelectiveStateSpaceModel::new(4, 2, 1);
        assert_eq!(ssm.n_states, 4);
        assert_eq!(ssm.n_inputs, 2);
        assert_eq!(ssm.n_outputs, 1);
        assert_eq!(ssm.state.len(), 4);
    }

    #[test]
    fn test_selective_ssm_step() {
        let mut ssm = SelectiveStateSpaceModel::new(4, 2, 1);
        let input = vec![1.0, 0.5];
        let output = ssm.step(&input);

        assert_eq!(output.len(), 1);
        assert!(ssm.state.iter().any(|&x| x != 0.0));
    }

    #[test]
    fn test_ssm_state_summary() {
        let mut ssm = SelectiveStateSpaceModel::new(4, 2, 1);
        ssm.step(&[1.0, 0.5]);

        let summary = ssm.state_summary();
        assert!(summary.max_activation >= 0.0);
        assert!(summary.sparsity >= 0.0 && summary.sparsity <= 1.0);
    }

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig::default();
        let agent = SsmAgent::new("test".to_string(), config);

        assert_eq!(agent.id(), "test");
        assert!(matches!(agent.state(), AgentState::Initialized));
    }

    #[test]
    fn test_agent_observation() {
        let config = AgentConfig {
            state_dim: 8,
            input_dim: 4,
            output_dim: 2,
            ..Default::default()
        };
        let mut agent = SsmAgent::new("test".to_string(), config);

        let observation = vec![0.1, 0.2, 0.3, 0.4];
        let responses = agent.receive(AgentMessage::Observation(observation), Instant::now());

        assert!(!responses.is_empty());
    }

    #[test]
    fn test_agent_query() {
        let mut agent = SsmAgent::new("test".to_string(), AgentConfig::default());

        let responses = agent.receive(
            AgentMessage::Query {
                query_id: "q1".to_string(),
                content: "state_summary".to_string(),
            },
            Instant::now(),
        );

        assert_eq!(responses.len(), 1);
        assert!(matches!(responses[0].0, AgentMessage::Response { .. }));
    }

    #[test]
    fn test_agent_reset() {
        let mut agent = SsmAgent::new("test".to_string(), AgentConfig::default());

        agent.receive(AgentMessage::Observation(vec![1.0, 2.0, 3.0, 4.0]), Instant::now());
        agent.reset();

        assert!(matches!(agent.state(), AgentState::Initialized));
    }

    #[test]
    fn test_orchestrator_registration() {
        let config = OrchestratorConfig::default();
        let mut orchestrator = MultiAgentOrchestrator::new(config);

        let agent = SsmAgent::new("test".to_string(), AgentConfig::default());
        assert!(orchestrator.register_agent(agent).is_ok());
        assert_eq!(orchestrator.metrics().agent_count, 1);
    }

    #[test]
    fn test_orchestrator_max_agents() {
        let config = OrchestratorConfig {
            max_agents: 2,
            ..Default::default()
        };
        let mut orchestrator = MultiAgentOrchestrator::new(config);

        orchestrator.register_agent(SsmAgent::new("a1".to_string(), AgentConfig::default())).unwrap();
        orchestrator.register_agent(SsmAgent::new("a2".to_string(), AgentConfig::default())).unwrap();

        let result = orchestrator.register_agent(SsmAgent::new("a3".to_string(), AgentConfig::default()));
        assert!(result.is_err());
    }

    #[test]
    fn test_platform_initialization() {
        let config = PlatformConfig::default();
        let mut platform = StateSpaceAgentPlatform::new(config);

        assert!(platform.initialize().is_ok());
        assert_eq!(platform.metrics().orchestrator.agent_count, 4);
        assert!(matches!(platform.metrics().system_state, SystemState::Running));
    }

    #[test]
    fn test_platform_processing() {
        let config = PlatformConfig::default();
        let mut platform = StateSpaceAgentPlatform::new(config);
        platform.initialize().unwrap();

        let observation = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        assert!(platform.process(observation).is_ok());
    }

    #[test]
    fn test_cross_agent_message() {
        let mut agent = SsmAgent::new("agent1".to_string(), AgentConfig::default());

        let responses = agent.receive(
            AgentMessage::CrossAgent {
                from: "agent2".to_string(),
                to: "agent1".to_string(),
                payload: Box::new(AgentMessage::Observation(vec![1.0, 2.0])),
            },
            Instant::now(),
        );

        // Should process the wrapped observation
        assert!(!responses.is_empty());
    }

    #[test]
    fn test_agent_metrics() {
        let mut agent = SsmAgent::new("test".to_string(), AgentConfig::default());

        // Process some observations
        for i in 0..5 {
            agent.receive(AgentMessage::Observation(vec![i as f64 * 0.1; 16]), Instant::now());
        }

        let metrics = agent.metrics();
        assert_eq!(metrics.id, "test");
        assert!(metrics.working_memory_size > 0);
    }

    #[test]
    fn test_shared_context() {
        let mut context = SharedContext::new(4);

        context.update_agent_contribution("agent1", vec![1.0, 2.0, 3.0, 4.0]);
        context.update_agent_contribution("agent2", vec![2.0, 4.0, 6.0, 8.0]);

        // Global state should be average
        assert!((context.global_state[0] - 1.5).abs() < 0.01);
        assert!((context.global_state[1] - 3.0).abs() < 0.01);
    }
}
