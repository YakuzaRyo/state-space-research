//! Thin Agent + Fat Platform Architecture
//!
//! This crate implements a deterministic state-space architecture where:
//! - Thin Agents: Stateless, ephemeral workers that execute specific tasks
//! - Fat Platform: Handles orchestration, memory, hooks, and skill management
//!
//! ## Core Design Principles
//!
//! 1. **Determinism**: All agent behavior is deterministic given the same inputs
//! 2. **Stateless Agents**: Agents are pure functions (input -> output)
//! 3. **Platform-Managed State**: All state lives in the platform
//! 4. **Message-Passing**: All communication via explicit message types
//! 5. **Skill System**: Two-tier skill loading (Core BIOS + Library)

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for agents, tasks, and messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(pub Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

/// Timestamp for events and state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    pub fn now() -> Self {
        Self(Utc::now())
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

// ============================================================================
// Message System
// ============================================================================

/// All communication between agents and platform happens through Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Id,
    pub source: Id,
    pub destination: Destination,
    pub payload: Payload,
    pub timestamp: Timestamp,
}

impl Message {
    pub fn new(source: Id, destination: Destination, payload: Payload) -> Self {
        Self {
            id: Id::new(),
            source,
            destination,
            payload,
            timestamp: Timestamp::now(),
        }
    }
}

/// Destination can be a specific agent or broadcast
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Destination {
    Agent(Id),
    Platform,
    Broadcast,
}

/// Payload types for different operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Payload {
    // Task lifecycle
    TaskRequest(TaskSpec),
    TaskResponse(TaskResult),
    TaskCancel(Id),

    // Skill system
    SkillLoad(String),
    SkillLoaded(Skill),
    SkillUnload(String),

    // State operations
    StateGet(String),
    StateSet { key: String, value: serde_json::Value },
    StateResponse(Option<serde_json::Value>),

    // Hook enforcement
    HookTrigger(HookType),
    HookResult(HookOutcome),

    // Agent lifecycle
    AgentSpawn(AgentConfig),
    AgentSpawned(Id),
    AgentTerminate(Id),
    AgentTerminated(Id),

    // Heartbeat
    Heartbeat,
    HeartbeatAck,
}

/// Task specification sent to agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: Id,
    pub task_type: String,
    pub input: serde_json::Value,
    pub required_skills: Vec<String>,
    pub timeout_ms: u64,
}

/// Task result returned by agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Id,
    pub status: TaskStatus,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: u64,
    pub tokens_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Success,
    Failure(String),
    Cancelled,
    Timeout,
}

// ============================================================================
// Skill System
// ============================================================================

/// Two-tier skill system: Core (BIOS) + Library
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub tier: SkillTier,
    pub description: String,
    pub capabilities: Vec<String>,
    pub prompt_template: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTier {
    /// Core BIOS skills always available (49 skills)
    Core,
    /// Library skills loaded on-demand (304+ skills)
    Library,
}

/// Skill registry manages available skills
pub struct SkillRegistry {
    core_skills: HashMap<String, Skill>,
    library_skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            core_skills: HashMap::new(),
            library_skills: HashMap::new(),
        }
    }

    pub fn register_core(&mut self, skill: Skill) {
        self.core_skills.insert(skill.name.clone(), skill);
    }

    pub fn register_library(&mut self, skill: Skill) {
        self.library_skills.insert(skill.name.clone(), skill);
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.core_skills.get(name)
            .or_else(|| self.library_skills.get(name))
    }

    pub fn load_for_task(&self, required: &[String]) -> Vec<&Skill> {
        required.iter()
            .filter_map(|name| self.get(name))
            .collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hook System
// ============================================================================

/// Hook types for enforcement outside LLM context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookType {
    SecurityCheck,
    ValidationCheck,
    ComplianceCheck,
    RateLimitCheck,
    BudgetCheck,
}

/// Hook execution outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookOutcome {
    Allowed,
    Blocked { reason: String },
    Modified { original: serde_json::Value, modified: serde_json::Value },
}

/// Hook trait for custom enforcement logic
#[async_trait]
pub trait Hook: Send + Sync {
    fn hook_type(&self) -> HookType;
    async fn execute(&self, context: &HookContext) -> HookOutcome;
}

/// Context provided to hooks during execution
#[derive(Debug, Clone)]
pub struct HookContext {
    pub agent_id: Id,
    pub task_id: Option<Id>,
    pub operation: String,
    pub data: serde_json::Value,
    pub timestamp: Timestamp,
}

// ============================================================================
// Agent Configuration
// ============================================================================

/// Configuration for spawning a new agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: String,
    pub max_tokens: u64,
    pub timeout_ms: u64,
    pub allowed_skills: Vec<String>,
    pub hooks: Vec<HookType>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_type: "default".to_string(),
            max_tokens: 2700,
            timeout_ms: 30000,
            allowed_skills: vec![],
            hooks: vec![
                HookType::SecurityCheck,
                HookType::ValidationCheck,
            ],
        }
    }
}

// ============================================================================
// Core Traits
// ============================================================================

/// The StateMachine trait - core abstraction for deterministic behavior
///
/// Inspired by deterministic simulation testing patterns:
/// - All state changes happen through receive()
/// - Time is passed in, not accessed directly
/// - No external dependencies except through messages
#[async_trait]
pub trait StateMachine: Send + Sync {
    /// Process an incoming message, optionally return responses
    async fn receive(&mut self, message: Message) -> Result<Vec<(Message, Destination)>, AgentError>;

    /// Periodic tick for time-based operations
    async fn tick(&mut self, current_time: Timestamp) -> Result<Vec<(Message, Destination)>, AgentError>;

    /// Get current state for debugging/observability
    fn state(&self) -> serde_json::Value;
}

/// ThinAgent trait - minimal interface for task execution
#[async_trait]
pub trait ThinAgent: StateMachine {
    /// Agent type identifier
    fn agent_type(&self) -> &str;

    /// Unique identifier
    fn id(&self) -> Id;

    /// Execute a task - pure function, no side effects
    async fn execute(&self, task: &TaskSpec, skills: &[&Skill]) -> TaskResult;
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum AgentError {
    #[error("Agent not found: {0:?}")]
    AgentNotFound(Id),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Task timeout: {0:?}")]
    TaskTimeout(Id),

    #[error("Hook blocked: {0}")]
    HookBlocked(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Platform error: {0}")]
    PlatformError(String),
}

// ============================================================================
// Platform Core
// ============================================================================

/// The Fat Platform - manages all agents, state, and orchestration
pub struct Platform {
    id: Id,
    skill_registry: Arc<SkillRegistry>,
    hooks: Vec<Box<dyn Hook>>,
    state_store: HashMap<String, serde_json::Value>,
}

impl Platform {
    pub fn new(skill_registry: Arc<SkillRegistry>) -> Self {
        Self {
            id: Id::new(),
            skill_registry,
            hooks: vec![],
            state_store: HashMap::new(),
        }
    }

    pub fn register_hook(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    pub async fn execute_hooks(
        &self,
        hook_type: HookType,
        context: &HookContext,
    ) -> Vec<HookOutcome> {
        let mut outcomes = vec![];
        for hook in &self.hooks {
            if hook.hook_type() == hook_type {
                outcomes.push(hook.execute(context).await);
            }
        }
        outcomes
    }

    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state_store.get(key)
    }

    pub fn set_state(&mut self, key: String, value: serde_json::Value) {
        self.state_store.insert(key, value);
    }

    pub fn id(&self) -> Id {
        self.id
    }
}

// ============================================================================
// Example Implementation: Simple Task Agent
// ============================================================================

/// A minimal thin agent implementation (<150 lines of logic)
pub struct SimpleTaskAgent {
    id: Id,
    config: AgentConfig,
    current_task: Option<Id>,
}

impl SimpleTaskAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            id: Id::new(),
            config,
            current_task: None,
        }
    }
}

#[async_trait]
impl StateMachine for SimpleTaskAgent {
    async fn receive(&mut self, message: Message) -> Result<Vec<(Message, Destination)>, AgentError> {
        match message.payload {
            Payload::TaskRequest(task_spec) => {
                self.current_task = Some(task_spec.id);

                // In real implementation, this would execute the task
                let result = TaskResult {
                    task_id: task_spec.id,
                    status: TaskStatus::Success,
                    output: Some(serde_json::json!({"executed": true})),
                    execution_time_ms: 100,
                    tokens_used: 150,
                };

                self.current_task = None;

                Ok(vec![(
                    Message::new(
                        self.id,
                        Destination::Platform,
                        Payload::TaskResponse(result),
                    ),
                    Destination::Platform,
                )])
            }
            Payload::TaskCancel(task_id) => {
                if self.current_task == Some(task_id) {
                    self.current_task = None;
                }
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }

    async fn tick(&mut self, _current_time: Timestamp) -> Result<Vec<(Message, Destination)>, AgentError> {
        // Simple agents don't have time-based operations
        Ok(vec![])
    }

    fn state(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "agent_type": self.config.agent_type,
            "current_task": self.current_task,
        })
    }
}

#[async_trait]
impl ThinAgent for SimpleTaskAgent {
    fn agent_type(&self) -> &str {
        &self.config.agent_type
    }

    fn id(&self) -> Id {
        self.id
    }

    async fn execute(&self, task: &TaskSpec, _skills: &[&Skill]) -> TaskResult {
        // Pure function: input -> output
        TaskResult {
            task_id: task.id,
            status: TaskStatus::Success,
            output: Some(task.input.clone()),
            execution_time_ms: 0,
            tokens_used: 0,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generation() {
        let id1 = Id::new();
        let id2 = Id::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_skill_registry() {
        let mut registry = SkillRegistry::new();

        let skill = Skill {
            name: "code_review".to_string(),
            tier: SkillTier::Core,
            description: "Review code for issues".to_string(),
            capabilities: vec!["analyze".to_string()],
            prompt_template: "Review this code: {{input}}".to_string(),
        };

        registry.register_core(skill);
        assert!(registry.get("code_review").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_message_creation() {
        let agent_id = Id::new();
        let platform_id = Id::new();

        let msg = Message::new(
            agent_id,
            Destination::Agent(platform_id),
            Payload::Heartbeat,
        );

        assert_eq!(msg.source, agent_id);
        match msg.payload {
            Payload::Heartbeat => {},
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn test_platform_state() {
        let registry = Arc::new(SkillRegistry::new());
        let mut platform = Platform::new(registry);

        platform.set_state("key".to_string(), serde_json::json!("value"));
        assert_eq!(
            platform.get_state("key"),
            Some(&serde_json::json!("value"))
        );
    }

    #[tokio::test]
    async fn test_simple_agent_state_machine() {
        let config = AgentConfig::default();
        let mut agent = SimpleTaskAgent::new(config);

        let task_spec = TaskSpec {
            id: Id::new(),
            task_type: "test".to_string(),
            input: serde_json::json!({"data": "test"}),
            required_skills: vec![],
            timeout_ms: 1000,
        };

        let msg = Message::new(
            Id::new(),
            Destination::Agent(agent.id()),
            Payload::TaskRequest(task_spec),
        );

        let responses = agent.receive(msg).await.unwrap();
        assert_eq!(responses.len(), 1);

        match &responses[0].0.payload {
            Payload::TaskResponse(result) => {
                assert!(matches!(result.status, TaskStatus::Success));
            }
            _ => panic!("Expected TaskResponse"),
        }
    }

    #[tokio::test]
    async fn test_agent_pure_execution() {
        let config = AgentConfig::default();
        let agent = SimpleTaskAgent::new(config);

        let task = TaskSpec {
            id: Id::new(),
            task_type: "echo".to_string(),
            input: serde_json::json!({"message": "hello"}),
            required_skills: vec![],
            timeout_ms: 1000,
        };

        let result = agent.execute(&task, &[]).await;
        assert!(matches!(result.status, TaskStatus::Success));
        assert_eq!(result.output, Some(task.input));
    }
}
