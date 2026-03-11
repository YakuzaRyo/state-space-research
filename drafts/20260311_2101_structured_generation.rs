//! Token-Level Constrained Generation Implementation
//!
//! This module implements a token-level constraint system for structured LLM generation
//! based on the XGrammar paper's key insights:
//! 1. Context-independent token caching (>99% of vocabulary)
//! 2. Context-dependent token dynamic validation (<1% of vocabulary)
//! 3. Efficient bitmask representation for O(1) mask application
//!
//! References:
//! - XGrammar: Flexible and Efficient Structured Generation Engine for LLMs (2024)
//! - llguidance: Microsoft's Rust-based structured generation library

use std::collections::{HashMap, HashSet};

/// Size of vocabulary (typical LLM vocab size ~50k-100k)
pub const VOCAB_SIZE: usize = 50000;

/// Number of bits in a u64 word
pub const BITS_PER_WORD: usize = 64;

/// Number of u64 words needed to represent the entire vocabulary
pub const BITMASK_WORDS: usize = (VOCAB_SIZE + BITS_PER_WORD - 1) / BITS_PER_WORD;

/// Token bitmask for efficient vocabulary masking
/// Uses a fixed-size array of u64 words to represent allowed tokens
#[derive(Clone, Debug, PartialEq)]
pub struct TokenBitmask {
    /// Bitmask words - each bit represents one token
    /// bit i in word j represents token (j * 64 + i)
    words: [u64; BITMASK_WORDS],
}

impl TokenBitmask {
    /// Create a new bitmask with all tokens allowed
    pub fn all_allowed() -> Self {
        let mut words = [u64::MAX; BITMASK_WORDS];
        // Mask out bits beyond vocabulary size in the last word
        let remaining_bits = VOCAB_SIZE % BITS_PER_WORD;
        if remaining_bits != 0 {
            words[BITMASK_WORDS - 1] = (1u64 << remaining_bits) - 1;
        }
        Self { words }
    }

    /// Create a new bitmask with all tokens disallowed
    pub fn all_disallowed() -> Self {
        Self { words: [0u64; BITMASK_WORDS] }
    }

    /// Check if a token is allowed
    pub fn is_allowed(&self, token_id: u32) -> bool {
        let token_id = token_id as usize;
        if token_id >= VOCAB_SIZE {
            return false;
        }
        let word_idx = token_id / BITS_PER_WORD;
        let bit_idx = token_id % BITS_PER_WORD;
        (self.words[word_idx] >> bit_idx) & 1 == 1
    }

    /// Allow a specific token
    pub fn allow_token(&mut self, token_id: u32) {
        let token_id = token_id as usize;
        if token_id >= VOCAB_SIZE {
            return;
        }
        let word_idx = token_id / BITS_PER_WORD;
        let bit_idx = token_id % BITS_PER_WORD;
        self.words[word_idx] |= 1u64 << bit_idx;
    }

    /// Disallow a specific token
    pub fn disallow_token(&mut self, token_id: u32) {
        let token_id = token_id as usize;
        if token_id >= VOCAB_SIZE {
            return;
        }
        let word_idx = token_id / BITS_PER_WORD;
        let bit_idx = token_id % BITS_PER_WORD;
        self.words[word_idx] &= !(1u64 << bit_idx);
    }

    /// Apply this mask to logits (in-place)
    /// Sets disallowed token logits to negative infinity
    pub fn apply_to_logits(&self, logits: &mut [f32]) {
        for (token_id, logit) in logits.iter_mut().enumerate() {
            if token_id >= VOCAB_SIZE {
                *logit = f32::NEG_INFINITY;
                continue;
            }
            let word_idx = token_id / BITS_PER_WORD;
            let bit_idx = token_id % BITS_PER_WORD;
            if (self.words[word_idx] >> bit_idx) & 1 == 0 {
                *logit = f32::NEG_INFINITY;
            }
        }
    }

    /// Compute intersection with another bitmask (AND operation)
    pub fn intersect(&mut self, other: &TokenBitmask) {
        for (word, other_word) in self.words.iter_mut().zip(other.words.iter()) {
            *word &= *other_word;
        }
    }

    /// Count number of allowed tokens
    pub fn count_allowed(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }
}

/// Represents a position in the Pushdown Automaton (PDA)
/// Used for context-independent token cache indexing
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct PdaPosition {
    /// State ID in the finite state automaton
    pub state_id: u32,
    /// Rule ID currently being expanded
    pub rule_id: u32,
    /// Position within the rule
    pub position: u16,
}

/// Context-independent token cache
/// Maps PDA positions to precomputed token bitmasks
pub struct ContextIndependentCache {
    /// Cache storage: PdaPosition -> TokenBitmask
    cache: HashMap<PdaPosition, TokenBitmask>,
}

impl ContextIndependentCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Get cached bitmask for a PDA position
    pub fn get(&self, pos: &PdaPosition) -> Option<&TokenBitmask> {
        self.cache.get(pos)
    }

    /// Insert a precomputed bitmask for a PDA position
    pub fn insert(&mut self, pos: PdaPosition, bitmask: TokenBitmask) {
        self.cache.insert(pos, bitmask);
    }

    /// Check if position has cached mask
    pub fn contains(&self, pos: &PdaPosition) -> bool {
        self.cache.contains_key(pos)
    }
}

/// Stack element for Pushdown Automaton
#[derive(Clone, Debug)]
pub struct StackElement {
    /// Return state when this rule completes
    pub return_state: u32,
    /// Rule ID to continue after completion
    pub return_rule: u32,
    /// Position within return rule
    pub return_position: u16,
}

/// Pushdown Automaton execution stack
pub struct PdaStack {
    /// Stack storage
    stack: Vec<StackElement>,
}

impl PdaStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Push a new frame onto the stack
    pub fn push(&mut self, element: StackElement) {
        self.stack.push(element);
    }

    /// Pop the top frame from the stack
    pub fn pop(&mut self) -> Option<StackElement> {
        self.stack.pop()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get stack depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Peek at the top element without removing
    pub fn peek(&self) -> Option<&StackElement> {
        self.stack.last()
    }
}

/// Token classification: context-independent vs context-dependent
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenClass {
    /// Token validity depends only on PDA position
    ContextIndependent,
    /// Token validity requires stack inspection
    ContextDependent,
}

/// Token classifier that categorizes vocabulary
pub struct TokenClassifier {
    /// Map from token ID to classification
    classifications: Vec<TokenClass>,
    /// Set of context-dependent token IDs
    context_dependent_tokens: HashSet<u32>,
}

impl TokenClassifier {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            classifications: vec![TokenClass::ContextIndependent; vocab_size],
            context_dependent_tokens: HashSet::new(),
        }
    }

    /// Mark a token as context-dependent
    pub fn mark_context_dependent(&mut self, token_id: u32) {
        if (token_id as usize) < self.classifications.len() {
            self.classifications[token_id as usize] = TokenClass::ContextDependent;
            self.context_dependent_tokens.insert(token_id);
        }
    }

    /// Get token classification
    pub fn classify(&self, token_id: u32) -> TokenClass {
        self.classifications.get(token_id as usize).copied()
            .unwrap_or(TokenClass::ContextDependent)
    }

    /// Check if token is context-dependent
    pub fn is_context_dependent(&self, token_id: u32) -> bool {
        self.context_dependent_tokens.contains(&token_id)
    }

    /// Get all context-dependent tokens
    pub fn context_dependent_tokens(&self) -> &HashSet<u32> {
        &self.context_dependent_tokens
    }

    /// Estimate proportion of context-dependent tokens
    pub fn context_dependent_ratio(&self) -> f64 {
        self.context_dependent_tokens.len() as f64 / self.classifications.len() as f64
    }
}

/// Grammar rule for context-free grammar
#[derive(Clone, Debug)]
pub struct GrammarRule {
    /// Rule ID
    pub id: u32,
    /// Rule name (for debugging)
    pub name: String,
    /// Sequence of symbols (token IDs or rule references)
    pub symbols: Vec<Symbol>,
}

/// Symbol in a grammar rule
#[derive(Clone, Copy, Debug)]
pub enum Symbol {
    /// Terminal token
    Terminal(u32),
    /// Reference to another rule
    RuleRef(u32),
}

/// Constraint engine for token-level structured generation
pub struct ConstraintEngine {
    /// Context-independent token cache
    ci_cache: ContextIndependentCache,
    /// Token classifier
    classifier: TokenClassifier,
    /// PDA execution stack
    stack: PdaStack,
    /// Current PDA position
    current_pos: PdaPosition,
    /// Grammar rules
    rules: Vec<GrammarRule>,
}

impl ConstraintEngine {
    pub fn new(vocab_size: usize, rules: Vec<GrammarRule>) -> Self {
        Self {
            ci_cache: ContextIndependentCache::new(),
            classifier: TokenClassifier::new(vocab_size),
            stack: PdaStack::new(),
            current_pos: PdaPosition {
                state_id: 0,
                rule_id: 0,
                position: 0,
            },
            rules,
        }
    }

    /// Compute the token mask for the next generation step
    /// This is the core algorithm implementing the XGrammar approach
    pub fn compute_mask(&mut self) -> TokenBitmask {
        // Step 1: Try to get context-independent mask from cache
        if let Some(cached_mask) = self.ci_cache.get(&self.current_pos).cloned() {
            // Step 2: Check if we need to handle context-dependent tokens
            if self.classifier.context_dependent_tokens.is_empty() {
                // Fast path: all tokens are context-independent
                return cached_mask;
            }

            // Step 3: Validate context-dependent tokens
            let mut mask = cached_mask;
            self.validate_context_dependent_tokens(&mut mask);
            return mask;
        }

        // Step 4: Cache miss - compute mask dynamically
        let mask = self.compute_mask_dynamic();

        // Step 5: Cache the result if position is cacheable
        if self.is_cacheable_position(&self.current_pos) {
            self.ci_cache.insert(self.current_pos, mask.clone());
        }

        mask
    }

    /// Validate context-dependent tokens against the current stack state
    fn validate_context_dependent_tokens(&self, mask: &mut TokenBitmask) {
        for &token_id in self.classifier.context_dependent_tokens() {
            // Check if token is valid given current stack state
            if !self.is_token_valid_with_stack(token_id) {
                mask.disallow_token(token_id);
            }
        }
    }

    /// Check if a token is valid given the current stack state
    fn is_token_valid_with_stack(&self, token_id: u32) -> bool {
        // Simplified validation: token is valid if stack depth < 100
        // In real implementation, this would simulate PDA execution
        self.stack.depth() < 100
    }

    /// Compute mask dynamically (without cache)
    fn compute_mask_dynamic(&self) -> TokenBitmask {
        let mut mask = TokenBitmask::all_disallowed();

        // Get current rule
        if let Some(rule) = self.rules.get(self.current_pos.rule_id as usize) {
            // Find valid next tokens based on current position in rule
            let pos = self.current_pos.position as usize;
            if pos < rule.symbols.len() {
                match rule.symbols[pos] {
                    Symbol::Terminal(token_id) => {
                        mask.allow_token(token_id);
                    }
                    Symbol::RuleRef(_rule_id) => {
                        // Would expand referenced rule in full implementation
                        // For now, allow all tokens as placeholder
                        mask = TokenBitmask::all_allowed();
                    }
                }
            }
        }

        mask
    }

    /// Check if a PDA position can be cached
    fn is_cacheable_position(&self, _pos: &PdaPosition) -> bool {
        // In real implementation, check if position is context-independent
        true
    }

    /// Advance the constraint state with a generated token
    pub fn advance(&mut self, token_id: u32) -> Result<(), ConstraintError> {
        // Update PDA position based on token
        self.current_pos.position += 1;

        // Handle stack operations for rule expansion/completion
        // Simplified: just track position

        Ok(())
    }

    /// Get current context-dependent token ratio
    pub fn context_dependent_ratio(&self) -> f64 {
        self.classifier.context_dependent_ratio()
    }
}

/// Error type for constraint operations
#[derive(Debug, Clone)]
pub enum ConstraintError {
    InvalidToken(u32),
    StackUnderflow,
    GrammarError(String),
}

impl std::fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstraintError::InvalidToken(id) => write!(f, "Invalid token: {}", id),
            ConstraintError::StackUnderflow => write!(f, "Stack underflow"),
            ConstraintError::GrammarError(msg) => write!(f, "Grammar error: {}", msg),
        }
    }
}

impl std::error::Error for ConstraintError {}

/// JSON Schema to grammar converter
pub struct JsonSchemaConverter;

impl JsonSchemaConverter {
    /// Convert a simple JSON schema to grammar rules
    pub fn convert(schema: &str) -> Vec<GrammarRule> {
        // Simplified implementation
        // Real implementation would parse JSON and generate CFG rules
        vec![
            GrammarRule {
                id: 0,
                name: "object".to_string(),
                symbols: vec![
                    Symbol::Terminal(1), // '{'
                    Symbol::Terminal(2), // '"'
                ],
            },
        ]
    }
}

/// Benchmark and test utilities
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bitmask_basic() {
        let mut mask = TokenBitmask::all_disallowed();
        assert!(!mask.is_allowed(100));

        mask.allow_token(100);
        assert!(mask.is_allowed(100));
        assert!(!mask.is_allowed(101));

        mask.disallow_token(100);
        assert!(!mask.is_allowed(100));
    }

    #[test]
    fn test_token_bitmask_all_allowed() {
        let mask = TokenBitmask::all_allowed();
        assert!(mask.is_allowed(0));
        assert!(mask.is_allowed(100));
        assert!(mask.is_allowed(VOCAB_SIZE as u32 - 1));
    }

    #[test]
    fn test_token_bitmask_intersection() {
        let mut mask1 = TokenBitmask::all_disallowed();
        mask1.allow_token(10);
        mask1.allow_token(20);

        let mut mask2 = TokenBitmask::all_disallowed();
        mask2.allow_token(20);
        mask2.allow_token(30);

        mask1.intersect(&mask2);

        assert!(!mask1.is_allowed(10));
        assert!(mask1.is_allowed(20));
        assert!(!mask1.is_allowed(30));
    }

    #[test]
    fn test_token_classifier() {
        let mut classifier = TokenClassifier::new(1000);

        // Initially all tokens are context-independent
        assert_eq!(classifier.classify(100), TokenClass::ContextIndependent);

        // Mark some as context-dependent
        classifier.mark_context_dependent(100);
        classifier.mark_context_dependent(200);

        assert_eq!(classifier.classify(100), TokenClass::ContextDependent);
        assert_eq!(classifier.classify(200), TokenClass::ContextDependent);
        assert_eq!(classifier.classify(300), TokenClass::ContextIndependent);

        // Check ratio
        let ratio = classifier.context_dependent_ratio();
        assert!((ratio - 0.002).abs() < 0.0001); // 2/1000 = 0.002
    }

    #[test]
    fn test_pda_stack() {
        let mut stack = PdaStack::new();
        assert!(stack.is_empty());

        stack.push(StackElement {
            return_state: 1,
            return_rule: 2,
            return_position: 3,
        });

        assert!(!stack.is_empty());
        assert_eq!(stack.depth(), 1);

        let elem = stack.pop().unwrap();
        assert_eq!(elem.return_state, 1);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_context_independent_cache() {
        let mut cache = ContextIndependentCache::new();
        let pos = PdaPosition {
            state_id: 1,
            rule_id: 2,
            position: 3,
        };

        assert!(!cache.contains(&pos));

        let mask = TokenBitmask::all_allowed();
        cache.insert(pos, mask);

        assert!(cache.contains(&pos));
        assert!(cache.get(&pos).is_some());
    }

    #[test]
    fn test_constraint_engine() {
        let rules = vec![
            GrammarRule {
                id: 0,
                name: "start".to_string(),
                symbols: vec![
                    Symbol::Terminal(1),
                    Symbol::Terminal(2),
                ],
            },
        ];

        let mut engine = ConstraintEngine::new(VOCAB_SIZE, rules);
        let mask = engine.compute_mask();

        // Should have some allowed tokens
        assert!(mask.count_allowed() > 0);
    }

    #[test]
    fn test_apply_to_logits() {
        let mut mask = TokenBitmask::all_disallowed();
        mask.allow_token(0);
        mask.allow_token(5);

        let mut logits = vec![1.0f32; 100];
        mask.apply_to_logits(&mut logits);

        assert!(logits[0].is_finite());
        assert!(!logits[1].is_finite()); // -inf
        assert!(!logits[2].is_finite());
        assert!(!logits[3].is_finite());
        assert!(!logits[4].is_finite());
        assert!(logits[5].is_finite());
    }
}

/// Performance benchmark (run with: cargo test --release -- --nocapture bench)
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_bitmask_operations() {
        let iterations = 100000;

        // Benchmark allow_token
        let start = Instant::now();
        for i in 0..iterations {
            let mut mask = TokenBitmask::all_disallowed();
            mask.allow_token((i % VOCAB_SIZE) as u32);
        }
        let elapsed = start.elapsed();
        println!("allow_token: {:?} per op", elapsed / iterations as u32);

        // Benchmark is_allowed
        let mask = TokenBitmask::all_allowed();
        let start = Instant::now();
        for i in 0..iterations {
            let _ = mask.is_allowed((i % VOCAB_SIZE) as u32);
        }
        let elapsed = start.elapsed();
        println!("is_allowed: {:?} per op", elapsed / iterations as u32);

        // Benchmark intersect
        let mask1 = TokenBitmask::all_allowed();
        let mask2 = TokenBitmask::all_allowed();
        let start = Instant::now();
        for _ in 0..iterations {
            let mut m = mask1.clone();
            m.intersect(&mask2);
        }
        let elapsed = start.elapsed();
        println!("intersect: {:?} per op", elapsed / iterations as u32);
    }

    #[test]
    fn bench_apply_to_logits() {
        let iterations = 10000;
        let mask = TokenBitmask::all_allowed();
        let mut logits = vec![1.0f32; VOCAB_SIZE];

        let start = Instant::now();
        for _ in 0..iterations {
            mask.apply_to_logits(&mut logits);
        }
        let elapsed = start.elapsed();
        println!("apply_to_logits ({} tokens): {:?} per op",
                 VOCAB_SIZE, elapsed / iterations);
    }
}

/// Main function demonstrating usage
fn main() {
    println!("Token-Level Constrained Generation Implementation");
    println!("=================================================");
    println!("Vocabulary size: {}", VOCAB_SIZE);
    println!("Bitmask words: {} ({} bits/word)", BITMASK_WORDS, BITS_PER_WORD);
    println!();

    // Create a simple grammar
    let rules = vec![
        GrammarRule {
            id: 0,
            name: "json_object".to_string(),
            symbols: vec![
                Symbol::Terminal(1), // '{'
                Symbol::RuleRef(1),  // string
                Symbol::Terminal(3), // ':'
                Symbol::RuleRef(2),  // value
                Symbol::Terminal(4), // '}'
            ],
        },
        GrammarRule {
            id: 1,
            name: "string".to_string(),
            symbols: vec![
                Symbol::Terminal(2), // '"'
                Symbol::Terminal(100), // content
                Symbol::Terminal(2), // '"'
            ],
        },
        GrammarRule {
            id: 2,
            name: "value".to_string(),
            symbols: vec![
                Symbol::Terminal(200), // number
            ],
        },
    ];

    // Create constraint engine
    let mut engine = ConstraintEngine::new(VOCAB_SIZE, rules);

    // Compute token mask
    let mask = engine.compute_mask();
    let allowed_count = mask.count_allowed();

    println!("Initial mask allows {} / {} tokens ({:.2}%)",
             allowed_count, VOCAB_SIZE,
             100.0 * allowed_count as f64 / VOCAB_SIZE as f64);

    // Demonstrate context-dependent ratio
    println!("Context-dependent token ratio: {:.4}%",
             engine.context_dependent_ratio() * 100.0);

    println!();
    println!("Key insights from XGrammar:");
    println!("- >99% of tokens are context-independent (cacheable)");
    println!("- <1% of tokens require stack inspection");
    println!("- Bitmask enables O(1) mask application");
    println!("- Precomputed masks reduce per-token latency to <40 microseconds");
}
