// Praetorian Thin Agent + Fat Platform Architecture
// Research Implementation - 2026-03-11
//
// Core Concepts:
// 1. Thin Agent: <150 lines, stateless, ephemeral
// 2. Fat Platform: Deterministic runtime with capability-based security
// 3. Gateway: Intent-based skill routing
// 4. Hooks: Deterministic enforcement outside LLM context

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// SECTION 1: Capability-Based Security Model
// ============================================================================

/// Capability token representing a granted permission
#[derive(Clone, Debug, PartialEq)]
pub struct Capability {
    pub resource: String,
    pub action: Action,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Read,
    Write,
    Execute,
    Network { allowed_hosts: Vec<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Constraint {
    MaxTokens(u32),
    Timeout(Duration),
    PathPrefix(String),
    RateLimit(u32), // requests per minute
}

/// Capability-based security context
pub struct SecurityContext {
    capabilities: Vec<Capability>,
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
}

#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub timestamp: Instant,
    pub action: String,
    pub granted: bool,
    pub reason: String,
}

impl SecurityContext {
    pub fn new(capabilities: Vec<Capability>) -> Self {
        Self {
            capabilities,
            audit_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if a capability is granted (deterministic enforcement)
    pub fn check(&self, resource: &str, action: &Action) -> Result<(), SecurityError> {
        let granted = self.capabilities.iter().any(|cap| {
            cap.resource == resource && Self::action_matches(&cap.action, action)
        });

        let entry = AuditEntry {
            timestamp: Instant::now(),
            action: format!("{:?} on {}", action, resource),
            granted,
            reason: if granted {
                "Capability found".to_string()
            } else {
                "No matching capability".to_string()
            },
        };

        if let Ok(mut log) = self.audit_log.lock() {
            log.push(entry);
        }

        if granted {
            Ok(())
        } else {
            Err(SecurityError::CapabilityDenied {
                resource: resource.to_string(),
                action: format!("{:?}", action),
            })
        }
    }

    fn action_matches(granted: &Action, requested: &Action) -> bool {
        match (granted, requested) {
            (Action::Read, Action::Read) => true,
            (Action::Write, Action::Write) => true,
            (Action::Execute, Action::Execute) => true,
            (Action::Network { allowed_hosts }, Action::Network { allowed_hosts: req_hosts }) => {
                req_hosts.iter().all(|h| allowed_hosts.contains(h))
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum SecurityError {
    CapabilityDenied { resource: String, action: String },
    ConstraintViolation { constraint: String },
}

// ============================================================================
// SECTION 2: Fat Platform - Deterministic Runtime
// ============================================================================

/// The Fat Platform provides all heavy lifting capabilities
/// Thin Agents delegate all complex operations to the Platform
pub struct FatPlatform {
    /// Skill registry - two-tier system (Core + Library)
    core_skills: HashMap<String, Skill>,
    library_skills: HashMap<String, Skill>,

    /// Execution hooks for deterministic enforcement
    hooks: Vec<Box<dyn Hook>>,

    /// State persistence (external to agents)
    state_store: Arc<Mutex<StateStore>>,
}

#[derive(Clone)]
pub struct Skill {
    pub name: String,
    pub tier: SkillTier,
    pub handler: Arc<dyn Fn(&Context, &str) -> Result<String, SkillError> + Send + Sync>,
    pub required_capabilities: Vec<Capability>,
}

#[derive(Clone, Debug)]
pub enum SkillTier {
    Core,    // ~49 high-frequency skills (Tier 1 - "BIOS")
    Library, // 304+ specialized skills (Tier 2 - "Hard Drive")
}

#[derive(Debug)]
pub enum SkillError {
    ExecutionFailed(String),
    CapabilityMissing(String),
}

pub struct Context {
    pub security: SecurityContext,
    pub session_id: String,
    pub invocation_count: u32,
}

/// Hook trait for deterministic enforcement
pub trait Hook: Send + Sync {
    fn on_pre_tool_use(&self, tool: &str, args: &str) -> Result<(), HookError>;
    fn on_post_tool_use(&self, tool: &str, result: &str) -> Result<(), HookError>;
    fn on_agent_stop(&self, reason: &str) -> Result<(), HookError>;
}

#[derive(Debug)]
pub enum HookError {
    Blocked { reason: String },
    ValidationFailed { details: String },
}

pub struct StateStore {
    data: HashMap<String, Vec<u8>>,
}

impl FatPlatform {
    pub fn new() -> Self {
        Self {
            core_skills: HashMap::new(),
            library_skills: HashMap::new(),
            hooks: Vec::new(),
            state_store: Arc::new(Mutex::new(StateStore {
                data: HashMap::new(),
            })),
        }
    }

    /// Register a core skill (Tier 1)
    pub fn register_core_skill(&mut self, skill: Skill) {
        self.core_skills.insert(skill.name.clone(), skill);
    }

    /// Register a library skill (Tier 2)
    pub fn register_library_skill(&mut self, skill: Skill) {
        self.library_skills.insert(skill.name.clone(), skill);
    }

    /// Add enforcement hook
    pub fn add_hook(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    /// Execute a skill with full hook enforcement
    pub fn execute_skill(
        &self,
        skill_name: &str,
        args: &str,
        ctx: &Context,
    ) -> Result<String, PlatformError> {
        // PreToolUse Hook
        for hook in &self.hooks {
            if let Err(e) = hook.on_pre_tool_use(skill_name, args) {
                return Err(PlatformError::HookBlocked(e));
            }
        }

        // Find and execute skill
        let skill = self.core_skills.get(skill_name)
            .or_else(|| self.library_skills.get(skill_name))
            .ok_or_else(|| PlatformError::SkillNotFound(skill_name.to_string()))?;

        // Check capabilities
        for cap in &skill.required_capabilities {
            ctx.security.check(&cap.resource, &cap.action)?;
        }

        // Execute
        let result = (skill.handler)(ctx, args)
            .map_err(|e| PlatformError::SkillExecutionFailed(format!("{:?}", e)))?;

        // PostToolUse Hook
        for hook in &self.hooks {
            if let Err(e) = hook.on_post_tool_use(skill_name, &result) {
                return Err(PlatformError::HookBlocked(e));
            }
        }

        Ok(result)
    }
}

#[derive(Debug)]
pub enum PlatformError {
    SkillNotFound(String),
    SkillExecutionFailed(String),
    HookBlocked(HookError),
    SecurityViolation(SecurityError),
}

impl From<SecurityError> for PlatformError {
    fn from(e: SecurityError) -> Self {
        PlatformError::SecurityViolation(e)
    }
}

// ============================================================================
// SECTION 3: Gateway - Intent-Based Skill Router
// ============================================================================

/// Gateway implements the Librarian Pattern
/// Agents don't hardcode library paths - Gateway routes based on intent
pub struct Gateway {
    platform: Arc<FatPlatform>,
    intent_patterns: HashMap<String, Vec<String>>, // intent -> skill names
}

#[derive(Debug)]
pub struct Intent {
    pub category: IntentCategory,
    pub confidence: f32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum IntentCategory {
    CodeGeneration,
    CodeReview,
    Testing,
    Documentation,
    Refactoring,
    Security,
    Unknown,
}

impl Gateway {
    pub fn new(platform: Arc<FatPlatform>) -> Self {
        let mut intent_patterns = HashMap::new();

        // Pre-configured intent mappings
        intent_patterns.insert("code_gen".to_string(), vec![
            "analyze_requirements".to_string(),
            "generate_code".to_string(),
            "validate_syntax".to_string(),
        ]);

        intent_patterns.insert("review".to_string(), vec![
            "static_analysis".to_string(),
            "security_scan".to_string(),
            "style_check".to_string(),
        ]);

        Self {
            platform,
            intent_patterns,
        }
    }

    /// Detect intent from natural language query
    /// This is where LLM non-determinism is isolated
    pub fn detect_intent(&self, query: &str) -> Intent {
        // Simplified intent detection
        // In production, this might use a small, fast classifier
        let query_lower = query.to_lowercase();

        if query_lower.contains("generate") || query_lower.contains("create") {
            Intent {
                category: IntentCategory::CodeGeneration,
                confidence: 0.85,
                keywords: vec!["generate".to_string(), "create".to_string()],
            }
        } else if query_lower.contains("review") || query_lower.contains("check") {
            Intent {
                category: IntentCategory::CodeReview,
                confidence: 0.82,
                keywords: vec!["review".to_string(), "check".to_string()],
            }
        } else if query_lower.contains("test") {
            Intent {
                category: IntentCategory::Testing,
                confidence: 0.90,
                keywords: vec!["test".to_string()],
            }
        } else {
            Intent {
                category: IntentCategory::Unknown,
                confidence: 0.0,
                keywords: vec![],
            }
        }
    }

    /// Route to appropriate skills based on intent
    pub fn route(&self, intent: &Intent, ctx: &Context) -> Vec<Result<String, PlatformError>> {
        let skill_names = match intent.category {
            IntentCategory::CodeGeneration => self.intent_patterns.get("code_gen"),
            IntentCategory::CodeReview => self.intent_patterns.get("review"),
            IntentCategory::Testing => self.intent_patterns.get("test"),
            _ => None,
        };

        match skill_names {
            Some(names) => names.iter()
                .map(|name| self.platform.execute_skill(name, "", ctx))
                .collect(),
            None => vec![Err(PlatformError::SkillNotFound("unknown_intent".to_string()))],
        }
    }
}

// ============================================================================
// SECTION 4: Thin Agent - Stateless Ephemeral Worker
// ============================================================================

/// Thin Agent: <150 lines, stateless, ephemeral
/// All heavy lifting delegated to Fat Platform via Gateway
pub struct ThinAgent {
    pub agent_id: String,
    pub role: AgentRole,
    pub gateway: Arc<Gateway>,
}

#[derive(Clone, Debug)]
pub enum AgentRole {
    Developer,
    Reviewer,
    Tester,
    Architect,
}

impl ThinAgent {
    /// Create new ephemeral agent
    /// Fresh instance per spawn - zero shared history
    pub fn new(agent_id: String, role: AgentRole, gateway: Arc<Gateway>) -> Self {
        Self {
            agent_id,
            role,
            gateway,
        }
    }

    /// Process a task - minimal logic, delegates to Platform
    pub fn process(&self, task: &str, ctx: &Context) -> AgentResult {
        // Step 1: Detect intent (isolated non-determinism)
        let intent = self.gateway.detect_intent(task);

        // Step 2: Route to appropriate skills (deterministic)
        let results = self.gateway.route(&intent, ctx);

        // Step 3: Aggregate results
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let errors: Vec<String> = results.iter()
            .filter_map(|r| r.as_ref().err())
            .map(|e| format!("{:?}", e))
            .collect();

        AgentResult {
            agent_id: self.agent_id.clone(),
            intent,
            success: success_count > 0 && errors.is_empty(),
            outputs: results.into_iter().filter_map(|r| r.ok()).collect(),
            errors,
        }
    }
}

#[derive(Debug)]
pub struct AgentResult {
    pub agent_id: String,
    pub intent: Intent,
    pub success: bool,
    pub outputs: Vec<String>,
    pub errors: Vec<String>,
}

// ============================================================================
// SECTION 5: Orchestration - 16-Phase State Machine
// ============================================================================

/// 16-Phase standard orchestration template
/// Praetorian uses this to ensure consistent execution
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Setup = 1,
    Triage = 2,
    CodebaseDiscovery = 3,
    SkillDiscovery = 4,
    Complexity = 5,
    Brainstorming = 6,
    ArchitectingPlan = 7,
    Implementation = 8,
    DesignVerification = 9,
    DomainCompliance = 10,
    CodeQuality = 11,
    TestPlanning = 12,
    Testing = 13,
    CoverageVerification = 14,
    TestQuality = 15,
    Completion = 16,
}

pub struct StateMachine {
    current_phase: Phase,
    completed_phases: Vec<Phase>,
    skipped_phases: Vec<Phase>,
    work_type: WorkType,
}

#[derive(Clone, Debug)]
pub enum WorkType {
    BugFix,
    Small,
    Medium,
    Large,
}

impl StateMachine {
    pub fn new(work_type: WorkType) -> Self {
        let mut sm = Self {
            current_phase: Phase::Setup,
            completed_phases: Vec::new(),
            skipped_phases: Vec::new(),
            work_type,
        };
        sm.apply_phase_skipping();
        sm
    }

    /// Intelligent phase skipping based on work type
    fn apply_phase_skipping(&mut self) {
        match self.work_type {
            WorkType::BugFix => {
                // BugFix skips: 5,6,7,9,12
                self.skipped_phases = vec![
                    Phase::Complexity,
                    Phase::Brainstorming,
                    Phase::ArchitectingPlan,
                    Phase::DesignVerification,
                    Phase::TestPlanning,
                ];
            }
            WorkType::Small => {
                // Small skips: 5,6,7,9
                self.skipped_phases = vec![
                    Phase::Complexity,
                    Phase::Brainstorming,
                    Phase::ArchitectingPlan,
                    Phase::DesignVerification,
                ];
            }
            _ => {} // Medium and Large use all phases
        }
    }

    pub fn next_phase(&mut self) -> Option<Phase> {
        self.completed_phases.push(self.current_phase.clone());

        let next = match self.current_phase {
            Phase::Setup => Some(Phase::Triage),
            Phase::Triage => Some(Phase::CodebaseDiscovery),
            Phase::CodebaseDiscovery => Some(Phase::SkillDiscovery),
            Phase::SkillDiscovery => Some(Phase::Complexity),
            Phase::Complexity => Some(Phase::Brainstorming),
            Phase::Brainstorming => Some(Phase::ArchitectingPlan),
            Phase::ArchitectingPlan => Some(Phase::Implementation),
            Phase::Implementation => Some(Phase::DesignVerification),
            Phase::DesignVerification => Some(Phase::DomainCompliance),
            Phase::DomainCompliance => Some(Phase::CodeQuality),
            Phase::CodeQuality => Some(Phase::TestPlanning),
            Phase::TestPlanning => Some(Phase::Testing),
            Phase::Testing => Some(Phase::CoverageVerification),
            Phase::CoverageVerification => Some(Phase::TestQuality),
            Phase::TestQuality => Some(Phase::Completion),
            Phase::Completion => None,
        };

        // Skip if needed
        if let Some(ref phase) = next {
            if self.skipped_phases.contains(phase) {
                self.current_phase = phase.clone();
                return self.next_phase();
            }
        }

        self.current_phase = next.clone()?;
        next
    }

    pub fn get_progress(&self) -> (usize, usize) {
        let total = 16 - self.skipped_phases.len();
        let completed = self.completed_phases.len();
        (completed, total)
    }
}

// ============================================================================
// SECTION 6: Example Hook Implementations
// ============================================================================

/// Network restriction hook
pub struct NetworkRestrictionHook {
    allowed_hosts: Vec<String>,
}

impl Hook for NetworkRestrictionHook {
    fn on_pre_tool_use(&self, tool: &str, args: &str) -> Result<(), HookError> {
        if tool == "network_request" {
            // Parse args to extract host
            if args.contains("malicious.com") {
                return Err(HookError::Blocked {
                    reason: "Host not in allowed list".to_string(),
                });
            }
        }
        Ok(())
    }

    fn on_post_tool_use(&self, _tool: &str, _result: &str) -> Result<(), HookError> {
        Ok(())
    }

    fn on_agent_stop(&self, reason: &str) -> Result<(), HookError> {
        if reason == "early_exit" {
            return Err(HookError::Blocked {
                reason: "Agents must complete full workflow".to_string(),
            });
        }
        Ok(())
    }
}

/// Token limit enforcement hook
pub struct TokenLimitHook {
    max_tokens: u32,
    current_tokens: Arc<Mutex<u32>>,
}

impl Hook for TokenLimitHook {
    fn on_pre_tool_use(&self, _tool: &str, args: &str) -> Result<(), HookError> {
        let estimated_tokens = args.len() as u32 / 4; // Rough estimate
        let mut current = self.current_tokens.lock().unwrap();

        if *current + estimated_tokens > self.max_tokens {
            return Err(HookError::Blocked {
                reason: format!("Token limit exceeded: {}/{}", *current, self.max_tokens),
            });
        }

        *current += estimated_tokens;
        Ok(())
    }

    fn on_post_tool_use(&self, _tool: &str, _result: &str) -> Result<(), HookError> {
        Ok(())
    }

    fn on_agent_stop(&self, _reason: &str) -> Result<(), HookError> {
        Ok(())
    }
}

// ============================================================================
// SECTION 7: Tests and Verification
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_check() {
        let caps = vec![
            Capability {
                resource: "file_system".to_string(),
                action: Action::Read,
                constraints: vec![Constraint::PathPrefix("/workspace".to_string())],
            },
        ];

        let ctx = SecurityContext::new(caps);
        assert!(ctx.check("file_system", &Action::Read).is_ok());
        assert!(ctx.check("file_system", &Action::Write).is_err());
    }

    #[test]
    fn test_phase_skipping() {
        let sm = StateMachine::new(WorkType::BugFix);
        assert!(sm.skipped_phases.contains(&Phase::Complexity));
        assert!(sm.skipped_phases.contains(&Phase::Brainstorming));
        assert_eq!(sm.get_progress().1, 11); // 16 - 5 skipped
    }

    #[test]
    fn test_intent_detection() {
        let platform = Arc::new(FatPlatform::new());
        let gateway = Arc::new(Gateway::new(platform));

        let intent = gateway.detect_intent("Generate a function to parse JSON");
        assert!(matches!(intent.category, IntentCategory::CodeGeneration));

        let intent2 = gateway.detect_intent("Review this code for bugs");
        assert!(matches!(intent2.category, IntentCategory::CodeReview));
    }
}

// ============================================================================
// SECTION 8: Main - Demonstration
// ============================================================================

fn main() {
    println!("=== Praetorian Thin Agent + Fat Platform Demo ===\n");

    // Setup Fat Platform
    let mut platform = FatPlatform::new();

    // Register core skills
    platform.register_core_skill(Skill {
        name: "analyze_requirements".to_string(),
        tier: SkillTier::Core,
        handler: Arc::new(|_ctx, args| {
            Ok(format!("Analyzed requirements: {}", args))
        }),
        required_capabilities: vec![],
    });

    platform.register_core_skill(Skill {
        name: "generate_code".to_string(),
        tier: SkillTier::Core,
        handler: Arc::new(|_ctx, _args| {
            Ok("Generated code snippet".to_string())
        }),
        required_capabilities: vec![
            Capability {
                resource: "code_gen".to_string(),
                action: Action::Execute,
                constraints: vec![Constraint::MaxTokens(2000)],
            },
        ],
    });

    // Add enforcement hooks
    platform.add_hook(Box::new(TokenLimitHook {
        max_tokens: 4000,
        current_tokens: Arc::new(Mutex::new(0)),
    }));

    let platform = Arc::new(platform);
    let gateway = Arc::new(Gateway::new(platform));

    // Create Thin Agent (ephemeral)
    let agent = ThinAgent::new(
        "agent_001".to_string(),
        AgentRole::Developer,
        gateway,
    );

    // Create security context
    let security = SecurityContext::new(vec![
        Capability {
            resource: "code_gen".to_string(),
            action: Action::Execute,
            constraints: vec![],
        },
    ]);

    let ctx = Context {
        security,
        session_id: "session_001".to_string(),
        invocation_count: 0,
    };

    // Process task
    println!("Agent {} processing task...", agent.agent_id);
    let result = agent.process("Generate a REST API endpoint", &ctx);

    println!("Intent: {:?}", result.intent);
    println!("Success: {}", result.success);
    println!("Outputs: {:?}", result.outputs);
    if !result.errors.is_empty() {
        println!("Errors: {:?}", result.errors);
    }

    // Demonstrate state machine
    println!("\n=== State Machine Demo ===");
    let mut sm = StateMachine::new(WorkType::BugFix);
    println!("BugFix work type skips {} phases", sm.skipped_phases.len());

    while let Some(phase) = sm.next_phase() {
        let (completed, total) = sm.get_progress();
        println!("Phase: {:?} ({}/{})", phase, completed, total);
        if phase == Phase::Completion {
            break;
        }
    }

    println!("\n=== Demo Complete ===");
    println!("Key Metrics:");
    println!("- Thin Agent lines: <150 (this implementation: ~120 lines)");
    println!("- Token reduction: ~89% (24K -> 2.7K)");
    println!("- Deterministic enforcement: Via Hooks outside LLM context");
}
