//! Deterministic AI Agent Architecture: Thin Agent + Fat Platform
//!
//! This module implements the "Thin Agent / Fat Platform" architecture pattern
//! inspired by Praetorian's deterministic AI orchestration system.
//!
//! Core Philosophy:
//! - Agents are stateless, ephemeral workers (<150 lines of logic)
//! - Platform provides deterministic runtime, state management, and enforcement
//! - Skills are loaded just-in-time via Gateway pattern
//! - Hooks provide enforcement outside LLM context
//!
//! References:
//! - Praetorian Deterministic AI Orchestration
//! - State Space Models for sequential processing
//! - Actor model for agent isolation

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

// ============================================================================
// SECTION 1: Core Types and Domain Model
// ============================================================================

/// Unique identifier for agents, tasks, and skills
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(prefix: &str) -> Self {
        Self(format!("{}_{}", prefix, uuid::Uuid::new_v4()))
    }
}

/// Agent capability - what a thin agent can do
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    Read,
    Write,
    Edit,
    Bash,
    Task,      // Can spawn sub-agents (Orchestrator only)
    TodoWrite,
}

/// Agent role determines capabilities and constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// Orchestrator: Can delegate (Task), plan, coordinate
    /// Constraint: NO Edit/Write - must delegate to workers
    Orchestrator,
    /// Worker: Can execute (Edit, Write, Bash)
    /// Constraint: NO Task - cannot delegate
    Worker,
    /// Reviewer: Read-only with validation capabilities
    Reviewer,
    /// Tester: Execution and validation
    Tester,
    /// Designer: Architecture and patterns
    Designer,
}

impl AgentRole {
    /// Returns allowed capabilities for this role
    pub fn capabilities(&self) -> Vec<Capability> {
        match self {
            AgentRole::Orchestrator => vec![
                Capability::Read,
                Capability::Task,
                Capability::TodoWrite,
            ],
            AgentRole::Worker => vec![
                Capability::Read,
                Capability::Write,
                Capability::Edit,
                Capability::Bash,
            ],
            AgentRole::Reviewer => vec![
                Capability::Read,
            ],
            AgentRole::Tester => vec![
                Capability::Read,
                Capability::Bash,
            ],
            AgentRole::Designer => vec![
                Capability::Read,
                Capability::Write, // For design docs
            ],
        }
    }

    /// Check if role can perform a capability
    pub fn can(&self, cap: Capability) -> bool {
        self.capabilities().contains(&cap)
    }
}

/// Task state in the orchestration lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    InProgress,
    AwaitingReview,
    AwaitingTest,
    Completed,
    Failed,
    Blocked,      // Waiting for dependency
}

/// Phase in the 16-phase orchestration template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Discovery = 0,
    Design = 1,
    Planning = 2,
    Implementation = 3,
    SelfReview = 4,
    ExternalReview = 5,
    Testing = 6,
    Integration = 7,
    Validation = 8,
    Documentation = 9,
    Deployment = 10,
    Monitoring = 11,
    Feedback = 12,
    Optimization = 13,
    Completion = 14,
    Archive = 15,
}

impl Phase {
    /// Total number of phases
    pub const COUNT: usize = 16;

    /// Check if this phase is a compaction gate (hard block at >85% context)
    pub fn is_compaction_gate(&self) -> bool {
        matches!(self, Phase::SelfReview | Phase::ExternalReview | Phase::Validation)
    }

    /// Get next phase
    pub fn next(&self) -> Option<Phase> {
        let current = *self as usize;
        if current + 1 < Self::COUNT {
            Some(match current + 1 {
                0 => Phase::Discovery,
                1 => Phase::Design,
                2 => Phase::Planning,
                3 => Phase::Implementation,
                4 => Phase::SelfReview,
                5 => Phase::ExternalReview,
                6 => Phase::Testing,
                7 => Phase::Integration,
                8 => Phase::Validation,
                9 => Phase::Documentation,
                10 => Phase::Deployment,
                11 => Phase::Monitoring,
                12 => Phase::Feedback,
                13 => Phase::Optimization,
                14 => Phase::Completion,
                15 => Phase::Archive,
                _ => unreachable!(),
            })
        } else {
            None
        }
    }
}

// ============================================================================
// SECTION 2: Thin Agent Implementation
// ============================================================================

/// Thin Agent - Stateless, ephemeral worker (<150 lines of logic)
///
/// The agent itself contains minimal logic. All state, orchestration,
/// and enforcement is handled by the Fat Platform.
#[derive(Debug, Clone)]
pub struct ThinAgent {
    pub id: EntityId,
    pub role: AgentRole,
    pub current_task: Option<EntityId>,
    pub spawn_time: Instant,
    pub iteration_count: u32,
    pub context_usage_percent: f32,
}

/// Maximum iterations before forced stop (L1 loop enforcement)
const MAX_ITERATIONS: u32 = 10;

/// Context usage threshold for compaction gate
const CONTEXT_THRESHOLD: f32 = 0.85;

impl ThinAgent {
    /// Create a new thin agent with specified role
    pub fn new(role: AgentRole) -> Self {
        Self {
            id: EntityId::new("agent"),
            role,
            current_task: None,
            spawn_time: Instant::now(),
            iteration_count: 0,
            context_usage_percent: 0.0,
        }
    }

    /// Check if agent can perform an action (role-based + runtime enforcement)
    pub fn can_perform(&self, capability: Capability) -> Result<(), AgentError> {
        // Role-based check
        if !self.role.can(capability) {
            return Err(AgentError::Unauthorized {
                agent_id: self.id.clone(),
                capability,
                role: self.role,
            });
        }

        // L1: Iteration limit enforcement
        if self.iteration_count >= MAX_ITERATIONS {
            return Err(AgentError::IterationLimitExceeded {
                agent_id: self.id.clone(),
                limit: MAX_ITERATIONS,
            });
        }

        // Compaction gate check
        if self.context_usage_percent > CONTEXT_THRESHOLD {
            return Err(AgentError::ContextCompactionRequired {
                agent_id: self.id.clone(),
                usage: self.context_usage_percent,
            });
        }

        Ok(())
    }

    /// Execute one iteration (called by platform)
    pub fn iterate(&mut self) -> Result<(), AgentError> {
        self.iteration_count += 1;

        if self.iteration_count > MAX_ITERATIONS {
            return Err(AgentError::IterationLimitExceeded {
                agent_id: self.id.clone(),
                limit: MAX_ITERATIONS,
            });
        }

        Ok(())
    }

    /// Update context usage (called by platform monitoring)
    pub fn update_context_usage(&mut self, percent: f32) {
        self.context_usage_percent = percent;
    }

    /// Get agent statistics
    pub fn stats(&self) -> AgentStats {
        AgentStats {
            lifetime: self.spawn_time.elapsed(),
            iterations: self.iteration_count,
            context_usage: self.context_usage_percent,
        }
    }
}

/// Agent runtime statistics
#[derive(Debug, Clone)]
pub struct AgentStats {
    pub lifetime: Duration,
    pub iterations: u32,
    pub context_usage: f32,
}

/// Agent error types
#[derive(Debug, Clone)]
pub enum AgentError {
    Unauthorized {
        agent_id: EntityId,
        capability: Capability,
        role: AgentRole,
    },
    IterationLimitExceeded {
        agent_id: EntityId,
        limit: u32,
    },
    ContextCompactionRequired {
        agent_id: EntityId,
        usage: f32,
    },
    HookBlocked {
        agent_id: EntityId,
        hook_name: String,
        reason: String,
    },
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Unauthorized { agent_id, capability, role } => {
                write!(f, "Agent {:?} with role {:?} cannot perform {:?}",
                    agent_id, role, capability)
            }
            AgentError::IterationLimitExceeded { agent_id, limit } => {
                write!(f, "Agent {:?} exceeded iteration limit of {}", agent_id, limit)
            }
            AgentError::ContextCompactionRequired { agent_id, usage } => {
                write!(f, "Agent {:?} requires context compaction (usage: {:.1}%)",
                    agent_id, usage * 100.0)
            }
            AgentError::HookBlocked { agent_id, hook_name, reason } => {
                write!(f, "Hook '{}' blocked agent {:?}: {}", hook_name, agent_id, reason)
            }
        }
    }
}

impl std::error::Error for AgentError {}

// ============================================================================
// SECTION 3: Skill System (Two-Tier Architecture)
// ============================================================================

/// Skill tier - Core (BIOS) or Library (Hard Drive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTier {
    /// Core skills - always available, registered as tools
    Core,
    /// Library skills - loaded on-demand via Gateway
    Library,
}

/// A skill that agents can invoke
#[derive(Debug, Clone)]
pub struct Skill {
    pub id: EntityId,
    pub name: String,
    pub tier: SkillTier,
    pub description: String,
    pub content: String,      // Skill implementation/instructions
    pub token_cost: usize,    // Estimated token cost when loaded
}

impl Skill {
    /// Create a new core skill
    pub fn core(name: &str, description: &str, content: &str) -> Self {
        Self {
            id: EntityId::new("skill"),
            name: name.to_string(),
            tier: SkillTier::Core,
            description: description.to_string(),
            content: content.to_string(),
            token_cost: content.len() / 4, // Rough estimate
        }
    }

    /// Create a new library skill
    pub fn library(name: &str, description: &str, content: &str) -> Self {
        Self {
            id: EntityId::new("skill"),
            name: name.to_string(),
            tier: SkillTier::Library,
            description: description.to_string(),
            content: content.to_string(),
            token_cost: content.len() / 4,
        }
    }
}

/// Gateway for intent-based skill loading
///
/// The Gateway pattern enables Just-in-Time context loading.
/// Instead of loading all library skills upfront, the gateway
/// routes to specific skills based on intent detection.
#[derive(Debug, Clone)]
pub struct Gateway {
    pub id: EntityId,
    pub name: String,
    pub intent_patterns: Vec<String>,
    pub target_skills: Vec<EntityId>,
}

impl Gateway {
    /// Create a new gateway
    pub fn new(name: &str) -> Self {
        Self {
            id: EntityId::new("gateway"),
            name: name.to_string(),
            intent_patterns: Vec::new(),
            target_skills: Vec::new(),
        }
    }

    /// Add an intent pattern and target skill
    pub fn route(mut self, pattern: &str, skill_id: EntityId) -> Self {
        self.intent_patterns.push(pattern.to_string());
        self.target_skills.push(skill_id);
        self
    }

    /// Detect intent from input and return matching skill IDs
    pub fn detect_intent(&self, input: &str) -> Vec<EntityId> {
        self.intent_patterns
            .iter()
            .enumerate()
            .filter(|(_, pattern)| input.contains(pattern))
            .map(|(idx, _)| self.target_skills[idx].clone())
            .collect()
    }
}

/// Skill registry managing Core and Library skills
#[derive(Debug, Clone)]
pub struct SkillRegistry {
    core_skills: HashMap<EntityId, Skill>,
    library_skills: HashMap<EntityId, Skill>,
    gateways: HashMap<EntityId, Gateway>,
    name_to_id: HashMap<String, EntityId>,
}

impl SkillRegistry {
    /// Create empty skill registry
    pub fn new() -> Self {
        Self {
            core_skills: HashMap::new(),
            library_skills: HashMap::new(),
            gateways: HashMap::new(),
            name_to_id: HashMap::new(),
        }
    }

    /// Register a core skill
    pub fn register_core(&mut self, skill: Skill) {
        self.name_to_id.insert(skill.name.clone(), skill.id.clone());
        self.core_skills.insert(skill.id.clone(), skill);
    }

    /// Register a library skill
    pub fn register_library(&mut self, skill: Skill) {
        self.name_to_id.insert(skill.name.clone(), skill.id.clone());
        self.library_skills.insert(skill.id.clone(), skill);
    }

    /// Register a gateway
    pub fn register_gateway(&mut self, gateway: Gateway) {
        self.gateways.insert(gateway.id.clone(), gateway);
    }

    /// Get a skill by ID
    pub fn get(&self, id: &EntityId) -> Option<&Skill> {
        self.core_skills.get(id)
            .or_else(|| self.library_skills.get(id))
    }

    /// Get a skill by name
    pub fn get_by_name(&self, name: &str) -> Option<&Skill> {
        self.name_to_id.get(name)
            .and_then(|id| self.get(id))
    }

    /// Load skills via gateway intent detection
    pub fn load_via_gateway(&self, gateway_name: &str, input: &str) -> Vec<&Skill> {
        self.gateways.values()
            .find(|g| g.name == gateway_name)
            .map(|g| g.detect_intent(input))
            .unwrap_or_default()
            .iter()
            .filter_map(|id| self.get(id))
            .collect()
    }

    /// Get all core skills
    pub fn core_skills(&self) -> Vec<&Skill> {
        self.core_skills.values().collect()
    }

    /// Calculate total token cost of core skills
    pub fn core_token_cost(&self) -> usize {
        self.core_skills.values().map(|s| s.token_cost).sum()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SECTION 4: Fat Platform - Orchestration and State Management
// ============================================================================

/// Persistent task state (survives session restarts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    pub task_id: EntityId,
    pub phase: Phase,
    pub state: TaskState,
    pub assigned_agents: Vec<EntityId>,
    pub validation_status: ValidationStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatus {
    pub implementation_complete: bool,
    pub review_passed: bool,
    pub tests_passed: bool,
    pub dirty_bit: bool,  // Set on Edit/Write, cleared on test pass
}

impl ValidationStatus {
    pub fn new() -> Self {
        Self {
            implementation_complete: false,
            review_passed: false,
            tests_passed: false,
            dirty_bit: false,
        }
    }
}

impl Default for ValidationStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Ephemeral runtime state (cleared on restart)
#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub feedback_loops: HashMap<EntityId, FeedbackLoop>,
    pub dirty_bits: HashMap<EntityId, bool>,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            feedback_loops: HashMap::new(),
            dirty_bits: HashMap::new(),
        }
    }

    /// Set dirty bit for an agent
    pub fn set_dirty(&mut self, agent_id: &EntityId) {
        self.dirty_bits.insert(agent_id.clone(), true);
    }

    /// Clear dirty bit for an agent
    pub fn clear_dirty(&mut self, agent_id: &EntityId) {
        self.dirty_bits.insert(agent_id.clone(), false);
    }

    /// Check if agent has dirty bit set
    pub fn is_dirty(&self, agent_id: &EntityId) -> bool {
        self.dirty_bits.get(agent_id).copied().unwrap_or(false)
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Feedback loop tracking for L2 enforcement
#[derive(Debug, Clone)]
pub struct FeedbackLoop {
    pub task_id: EntityId,
    pub domain: String,           // e.g., "backend", "frontend"
    pub phases_completed: Vec<Phase>,
    pub awaiting_validation: bool,
}

/// The Fat Platform - Central orchestration and state management
///
/// This is the "Fat" part of "Thin Agent / Fat Platform".
/// It handles all the complexity that traditional agents would
/// need to manage internally.
pub struct FatPlatform {
    /// Skill registry (Core + Library)
    skill_registry: SkillRegistry,

    /// Active agents
    agents: Mutex<HashMap<EntityId, ThinAgent>>,

    /// Task manifests (persistent state)
    task_manifests: Mutex<HashMap<EntityId, TaskManifest>>,

    /// Runtime state (ephemeral)
    runtime_state: Mutex<RuntimeState>,

    /// Registered hooks
    hooks: Vec<Box<dyn Hook>>,
}

impl FatPlatform {
    /// Create a new fat platform
    pub fn new() -> Self {
        Self {
            skill_registry: SkillRegistry::new(),
            agents: Mutex::new(HashMap::new()),
            task_manifests: Mutex::new(HashMap::new()),
            runtime_state: Mutex::new(RuntimeState::new()),
            hooks: Vec::new(),
        }
    }

    /// Initialize with default skills and gateways
    pub fn initialize_default(&mut self) {
        // Register core skills
        self.skill_registry.register_core(Skill::core(
            "gateway-frontend",
            "Routes to frontend-related skills",
            "Intent patterns: React, Vue, Angular, CSS, HTML, DOM"
        ));

        self.skill_registry.register_core(Skill::core(
            "gateway-backend",
            "Routes to backend-related skills",
            "Intent patterns: API, database, SQL, REST, GraphQL, server"
        ));

        self.skill_registry.register_core(Skill::core(
            "gateway-devops",
            "Routes to DevOps-related skills",
            "Intent patterns: Docker, Kubernetes, CI/CD, deploy, infrastructure"
        ));

        // Register library skills
        self.skill_registry.register_library(Skill::library(
            "react-patterns",
            "React component patterns and best practices",
            "Detailed React patterns..."
        ));

        self.skill_registry.register_library(Skill::library(
            "rust-api-design",
            "Rust API design patterns",
            "Detailed Rust API patterns..."
        ));
    }

    /// Spawn a new thin agent
    pub fn spawn_agent(&self, role: AgentRole) -> EntityId {
        let agent = ThinAgent::new(role);
        let id = agent.id.clone();

        let mut agents = self.agents.lock().unwrap();
        agents.insert(id.clone(), agent);

        id
    }

    /// Get agent by ID
    pub fn get_agent(&self, id: &EntityId) -> Option<ThinAgent> {
        let agents = self.agents.lock().unwrap();
        agents.get(id).cloned()
    }

    /// Execute agent action with full hook enforcement
    pub fn execute_action(
        &self,
        agent_id: &EntityId,
        capability: Capability,
        context: &ActionContext,
    ) -> Result<ActionResult, AgentError> {
        // Pre-execution hooks
        for hook in &self.hooks {
            if let Err(e) = hook.pre_execute(agent_id, capability, context) {
                return Err(AgentError::HookBlocked {
                    agent_id: agent_id.clone(),
                    hook_name: hook.name().to_string(),
                    reason: e,
                });
            }
        }

        // Get and validate agent
        let mut agents = self.agents.lock().unwrap();
        let agent = agents.get_mut(agent_id)
            .ok_or_else(|| AgentError::HookBlocked {
                agent_id: agent_id.clone(),
                hook_name: "agent-existence".to_string(),
                reason: "Agent not found".to_string(),
            })?;

        // Role and runtime validation
        agent.can_perform(capability)?;

        // Set dirty bit on write operations
        if matches!(capability, Capability::Write | Capability::Edit) {
            let mut runtime = self.runtime_state.lock().unwrap();
            runtime.set_dirty(agent_id);
        }

        // Increment iteration
        agent.iterate()?;

        // Post-execution hooks
        for hook in &self.hooks {
            hook.post_execute(agent_id, capability, context);
        }

        Ok(ActionResult::Success)
    }

    /// Attempt to stop an agent (L2/L3 enforcement)
    pub fn request_stop(&self, agent_id: &EntityId) -> Result<(), AgentError> {
        let runtime = self.runtime_state.lock().unwrap();

        // L2: Block if dirty bit set (tests not passed)
        if runtime.is_dirty(agent_id) {
            return Err(AgentError::HookBlocked {
                agent_id: agent_id.clone(),
                hook_name: "feedback-loop-stop".to_string(),
                reason: "Dirty bit set - tests must pass before stopping".to_string(),
            });
        }

        Ok(())
    }

    /// Mark tests as passed for an agent
    pub fn mark_tests_passed(&self, agent_id: &EntityId) {
        let mut runtime = self.runtime_state.lock().unwrap();
        runtime.clear_dirty(agent_id);
    }

    /// Create a new task with manifest
    pub fn create_task(&self, task_id: EntityId) -> TaskManifest {
        let manifest = TaskManifest {
            task_id: task_id.clone(),
            phase: Phase::Discovery,
            state: TaskState::Pending,
            assigned_agents: Vec::new(),
            validation_status: ValidationStatus::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut manifests = self.task_manifests.lock().unwrap();
        manifests.insert(task_id, manifest.clone());

        manifest
    }

    /// Advance task to next phase
    pub fn advance_phase(&self, task_id: &EntityId) -> Result<Phase, AgentError> {
        let mut manifests = self.task_manifests.lock().unwrap();

        if let Some(manifest) = manifests.get_mut(task_id) {
            if let Some(next) = manifest.phase.next() {
                // Check compaction gate
                if next.is_compaction_gate() {
                    // Would check context usage here
                }

                manifest.phase = next;
                manifest.updated_at = chrono::Utc::now().to_rfc3339();
                Ok(next)
            } else {
                Err(AgentError::HookBlocked {
                    agent_id: task_id.clone(),
                    hook_name: "phase-advance".to_string(),
                    reason: "Already at final phase".to_string(),
                })
            }
        } else {
            Err(AgentError::HookBlocked {
                agent_id: task_id.clone(),
                hook_name: "phase-advance".to_string(),
                reason: "Task not found".to_string(),
            })
        }
    }

    /// Register a hook
    pub fn register_hook(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    /// Get skill registry
    pub fn skills(&self) -> &SkillRegistry {
        &self.skill_registry
    }

    /// Get mutable skill registry
    pub fn skills_mut(&mut self) -> &mut SkillRegistry {
        &mut self.skill_registry
    }
}

impl Default for FatPlatform {
    fn default() -> Self {
        Self::new()
    }
}

/// Action context for hook evaluation
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub task_id: Option<EntityId>,
    pub target_file: Option<String>,
    pub content_hash: Option<String>,
}

impl ActionContext {
    pub fn new() -> Self {
        Self {
            task_id: None,
            target_file: None,
            content_hash: None,
        }
    }
}

impl Default for ActionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Action result
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success,
    Blocked(String),
    RequiresReview,
}

/// Hook trait for deterministic enforcement
///
/// Hooks operate outside the LLM context and provide
/// enforcement that skills cannot (skills only provide guidance).
pub trait Hook: Send + Sync {
    /// Hook name for identification
    fn name(&self) -> &str;

    /// Called before action execution
    /// Return Err to block the action
    fn pre_execute(
        &self,
        agent_id: &EntityId,
        capability: Capability,
        context: &ActionContext,
    ) -> Result<(), String>;

    /// Called after action execution
    fn post_execute(
        &self,
        agent_id: &EntityId,
        capability: Capability,
        context: &ActionContext,
    );
}

// ============================================================================
// SECTION 5: Deterministic Hooks Implementation
// ============================================================================

/// L1 Hook: Iteration limit enforcement
pub struct IterationLimitHook {
    max_iterations: u32,
}

impl IterationLimitHook {
    pub fn new(max: u32) -> Self {
        Self { max_iterations: max }
    }
}

impl Hook for IterationLimitHook {
    fn name(&self) -> &str {
        "iteration-limit"
    }

    fn pre_execute(
        &self,
        _agent_id: &EntityId,
        _capability: Capability,
        _context: &ActionContext,
    ) -> Result<(), String> {
        // Actual check is done in agent.iterate()
        Ok(())
    }

    fn post_execute(&self, _agent_id: &EntityId, _capability: Capability, _context: &ActionContext) {}
}

/// L2 Hook: Output location enforcement
pub struct OutputLocationHook;

impl Hook for OutputLocationHook {
    fn name(&self) -> &str {
        "output-location"
    }

    fn pre_execute(
        &self,
        _agent_id: &EntityId,
        capability: Capability,
        context: &ActionContext,
    ) -> Result<(), String> {
        if matches!(capability, Capability::Write | Capability::Edit) {
            if let Some(ref file) = context.target_file {
                // Enforce outputs go to .claude/.output/
                if !file.starts_with(".claude/.output/") && !file.starts_with("drafts/") {
                    return Err(format!(
                        "Outputs must be written to .claude/.output/ or drafts/, not {}",
                        file
                    ));
                }
            }
        }
        Ok(())
    }

    fn post_execute(&self, _agent_id: &EntityId, _capability: Capability, _context: &ActionContext) {}
}

/// L3 Hook: Quality gate enforcement
pub struct QualityGateHook;

impl Hook for QualityGateHook {
    fn name(&self) -> &str {
        "quality-gate"
    }

    fn pre_execute(
        &self,
        _agent_id: &EntityId,
        capability: Capability,
        _context: &ActionContext,
    ) -> Result<(), String> {
        // Block Write/Edit during certain phases without review
        if matches!(capability, Capability::Write | Capability::Edit) {
            // Would check task phase here
        }
        Ok(())
    }

    fn post_execute(&self, _agent_id: &EntityId, _capability: Capability, _context: &ActionContext) {}
}

// ============================================================================
// SECTION 6: State Space Model Integration
// ============================================================================

/// State Space Model for sequential agent state tracking
///
/// Based on control theory: x(k+1) = Ax(k) + Bu(k)
/// where x is state, u is input, y is output
#[derive(Debug, Clone)]
pub struct StateSpaceModel {
    /// State dimension
    pub state_dim: usize,
    /// Input dimension
    pub input_dim: usize,
    /// Output dimension
    pub output_dim: usize,
    /// State transition matrix A
    pub a: Vec<Vec<f64>>,
    /// Input matrix B
    pub b: Vec<Vec<f64>>,
    /// Output matrix C
    pub c: Vec<Vec<f64>>,
    /// Feedthrough matrix D
    pub d: Vec<Vec<f64>>,
    /// Current state x
    pub state: Vec<f64>,
}

impl StateSpaceModel {
    /// Create a new SSM with given dimensions
    pub fn new(state_dim: usize, input_dim: usize, output_dim: usize) -> Self {
        Self {
            state_dim,
            input_dim,
            output_dim,
            a: vec![vec![0.0; state_dim]; state_dim],
            b: vec![vec![0.0; state_dim]; input_dim],
            c: vec![vec![0.0; output_dim]; state_dim],
            d: vec![vec![0.0; output_dim]; input_dim],
            state: vec![0.0; state_dim],
        }
    }

    /// Step the model forward: x(k+1) = Ax(k) + Bu(k)
    pub fn step(&mut self, input: &[f64]) -> Vec<f64> {
        assert_eq!(input.len(), self.input_dim);

        // Compute next state: x_next = A*x + B*u
        let mut next_state = vec![0.0; self.state_dim];

        // A*x term
        for i in 0..self.state_dim {
            for j in 0..self.state_dim {
                next_state[i] += self.a[i][j] * self.state[j];
            }
        }

        // B*u term
        for i in 0..self.state_dim {
            for j in 0..self.input_dim {
                next_state[i] += self.b[j][i] * input[j];
            }
        }

        self.state = next_state;

        // Compute output: y = C*x + D*u
        let mut output = vec![0.0; self.output_dim];

        // C*x term
        for i in 0..self.output_dim {
            for j in 0..self.state_dim {
                output[i] += self.c[j][i] * self.state[j];
            }
        }

        // D*u term
        for i in 0..self.output_dim {
            for j in 0..self.input_dim {
                output[i] += self.d[j][i] * input[j];
            }
        }

        output
    }
}

/// Agent state tracker using SSM
///
/// Tracks agent behavior patterns over time to detect
/// anomalies or infinite loops
pub struct AgentStateTracker {
    model: StateSpaceModel,
    history: Vec<Vec<f64>>,
    max_history: usize,
}

impl AgentStateTracker {
    /// Create a new tracker
    ///
    /// Input features: [iteration_count, context_usage, error_rate, action_type_encoded]
    /// Output: [anomaly_score, loop_probability]
    pub fn new() -> Self {
        Self {
            model: StateSpaceModel::new(4, 4, 2),
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Record an agent action
    pub fn record_action(&mut self, iteration: f64, context: f64, errors: f64, action: f64) {
        let input = vec![iteration, context, errors, action];
        let output = self.model.step(&input);

        self.history.push(output);

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Detect if agent is in a loop
    pub fn detect_loop(&self) -> bool {
        if self.history.len() < 10 {
            return false;
        }

        // Simple loop detection: check if recent outputs are similar
        let recent = &self.history[self.history.len()-10..];
        let first = &recent[0];

        recent.iter().all(|h| {
            h.iter().zip(first.iter())
                .all(|(a, b)| (a - b).abs() < 0.01)
        })
    }

    /// Get anomaly score (0.0 = normal, 1.0 = anomalous)
    pub fn anomaly_score(&self) -> f64 {
        self.history.last()
            .map(|h| h[0])
            .unwrap_or(0.0)
    }
}

impl Default for AgentStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SECTION 7: Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_capabilities() {
        let orchestrator = AgentRole::Orchestrator;
        assert!(orchestrator.can(Capability::Task));
        assert!(orchestrator.can(Capability::Read));
        assert!(!orchestrator.can(Capability::Write)); // Cannot write
        assert!(!orchestrator.can(Capability::Edit));  // Cannot edit

        let worker = AgentRole::Worker;
        assert!(worker.can(Capability::Write));
        assert!(worker.can(Capability::Edit));
        assert!(!worker.can(Capability::Task)); // Cannot delegate
    }

    #[test]
    fn test_thin_agent_iteration_limit() {
        let mut agent = ThinAgent::new(AgentRole::Worker);

        // Should succeed for first 10 iterations
        for _ in 0..MAX_ITERATIONS {
            assert!(agent.iterate().is_ok());
        }

        // 11th iteration should fail
        assert!(agent.iterate().is_err());
    }

    #[test]
    fn test_thin_agent_context_compaction() {
        let mut agent = ThinAgent::new(AgentRole::Worker);

        // Normal context usage
        agent.update_context_usage(0.5);
        assert!(agent.can_perform(Capability::Read).is_ok());

        // Above threshold
        agent.update_context_usage(0.9);
        assert!(agent.can_perform(Capability::Read).is_err());
    }

    #[test]
    fn test_phase_transitions() {
        assert_eq!(Phase::Discovery as usize, 0);
        assert_eq!(Phase::Archive as usize, 15);

        let phases: Vec<_> = std::iter::successors(Some(Phase::Discovery), |p| p.next())
            .collect();
        assert_eq!(phases.len(), 16);

        assert!(Phase::SelfReview.is_compaction_gate());
        assert!(!Phase::Discovery.is_compaction_gate());
    }

    #[test]
    fn test_skill_registry() {
        let mut registry = SkillRegistry::new();

        let core_skill = Skill::core("test-core", "Test", "Content");
        let lib_skill = Skill::library("test-lib", "Test", "Content");

        registry.register_core(core_skill.clone());
        registry.register_library(lib_skill.clone());

        assert!(registry.get(&core_skill.id).is_some());
        assert!(registry.get(&lib_skill.id).is_some());
        assert!(registry.get_by_name("test-core").is_some());
    }

    #[test]
    fn test_gateway_intent_detection() {
        let skill_id = EntityId::new("skill");
        let gateway = Gateway::new("test-gateway")
            .route("react", skill_id.clone())
            .route("frontend", skill_id.clone());

        let matches = gateway.detect_intent("I need help with react components");
        assert_eq!(matches.len(), 1);

        let no_match = gateway.detect_intent("I need help with python");
        assert_eq!(no_match.len(), 0);
    }

    #[test]
    fn test_output_location_hook() {
        let hook = OutputLocationHook;
        let mut context = ActionContext::new();

        // Valid location
        context.target_file = Some(".claude/.output/test.rs".to_string());
        assert!(hook.pre_execute(&EntityId::new("agent"), Capability::Write, &context).is_ok());

        // Invalid location
        context.target_file = Some("/etc/passwd".to_string());
        assert!(hook.pre_execute(&EntityId::new("agent"), Capability::Write, &context).is_err());
    }

    #[test]
    fn test_state_space_model() {
        let mut ssm = StateSpaceModel::new(2, 1, 1);

        // Set simple dynamics
        ssm.a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        ssm.b = vec![vec![1.0], vec![0.0]];
        ssm.c = vec![vec![1.0], vec![0.0]];
        ssm.d = vec![vec![0.0]];

        let output1 = ssm.step(&[1.0]);
        let output2 = ssm.step(&[1.0]);

        assert_eq!(output1.len(), 1);
        assert_eq!(output2.len(), 1);
    }

    #[test]
    fn test_agent_state_tracker() {
        let mut tracker = AgentStateTracker::new();

        // Record some actions
        for i in 0..5 {
            tracker.record_action(i as f64, 0.5, 0.0, 1.0);
        }

        // Not enough history for loop detection
        assert!(!tracker.detect_loop());

        // Record more actions
        for _ in 0..10 {
            tracker.record_action(5.0, 0.5, 0.0, 1.0);
        }

        // Should detect loop now
        assert!(tracker.detect_loop());
    }

    #[test]
    fn test_fat_platform_spawn() {
        let platform = FatPlatform::new();
        let agent_id = platform.spawn_agent(AgentRole::Worker);

        let agent = platform.get_agent(&agent_id);
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().role, AgentRole::Worker);
    }

    #[test]
    fn test_fat_platform_action_execution() {
        let platform = FatPlatform::new();
        let agent_id = platform.spawn_agent(AgentRole::Worker);
        let context = ActionContext::new();

        // Worker can read
        assert!(platform.execute_action(&agent_id, Capability::Read, &context).is_ok());

        // Worker cannot delegate
        assert!(platform.execute_action(&agent_id, Capability::Task, &context).is_err());
    }

    #[test]
    fn test_fat_platform_dirty_bit() {
        let platform = FatPlatform::new();
        let agent_id = platform.spawn_agent(AgentRole::Worker);

        let mut context = ActionContext::new();
        context.target_file = Some(".claude/.output/test.rs".to_string());

        // Initially not dirty
        assert!(platform.request_stop(&agent_id).is_ok());

        // Perform write operation
        platform.execute_action(&agent_id, Capability::Write, &context).unwrap();

        // Now dirty - stop should be blocked
        assert!(platform.request_stop(&agent_id).is_err());

        // Mark tests passed
        platform.mark_tests_passed(&agent_id);

        // Now can stop
        assert!(platform.request_stop(&agent_id).is_ok());
    }

    #[test]
    fn test_task_phase_advance() {
        let platform = FatPlatform::new();
        let task_id = EntityId::new("task");

        platform.create_task(task_id.clone());

        // Advance through phases
        let phase1 = platform.advance_phase(&task_id).unwrap();
        assert_eq!(phase1, Phase::Design);

        let phase2 = platform.advance_phase(&task_id).unwrap();
        assert_eq!(phase2, Phase::Planning);
    }
}
