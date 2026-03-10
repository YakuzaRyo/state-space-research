// Type-Constrained Code Generation - Rust Implementation
// Research Draft: Prefix Automaton & Type Reachability Search
// Date: 2026-03-10

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

// ============================================================================
// PART 1: Type System Foundation
// ============================================================================

/// Represents types in our simply-typed lambda calculus variant
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Base types (int, bool, string, etc.)
    Base(String),
    /// Function type: T1 -> T2
    Arrow(Box<Type>, Box<Type>),
    /// Type variable for polymorphism
    Var(String),
    /// Product type (tuple)
    Product(Vec<Type>),
    /// Array type
    Array(Box<Type>),
}

impl Type {
    /// Create a base type
    pub fn base(name: &str) -> Self {
        Type::Base(name.to_string())
    }

    /// Create a function type
    pub fn arrow(dom: Type, cod: Type) -> Self {
        Type::Arrow(Box::new(dom), Box::new(cod))
    }

    /// Create an array type
    pub fn array(elem: Type) -> Self {
        Type::Array(Box::new(elem))
    }

    /// Check if this type matches a pattern (for generic matching)
    pub fn matches_pattern(&self, pattern: &Type) -> bool {
        match (self, pattern) {
            // Generic array pattern matches any array
            (_, Type::Base(p)) if p == "•[]" => matches!(self, Type::Array(_)),
            // Exact match
            (a, b) => a == b,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Base(s) => write!(f, "{}", s),
            Type::Arrow(dom, cod) => write!(f, "({} -> {})", dom, cod),
            Type::Var(v) => write!(f, "'{}", v),
            Type::Product(ts) => {
                let parts: Vec<String> = ts.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", parts.join(" * "))
            }
            Type::Array(t) => write!(f, "{}[]", t),
        }
    }
}

// ============================================================================
// PART 2: Prefix Automaton for Type-Constrained Parsing
// ============================================================================

/// A state in the prefix automaton
/// The key property: from any accepting state, we can reach a final state
#[derive(Debug, Clone)]
pub struct PrefixState {
    /// Unique identifier for this state
    pub id: usize,
    /// Whether this is an accepting state (prefix property)
    pub is_accepting: bool,
    /// Whether this is a final (complete) state
    pub is_final: bool,
    /// The type context at this state
    pub type_context: TypeContext,
    /// Expected type for completion (if any)
    pub expected_type: Option<Type>,
}

/// Type context tracks variable bindings
#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    /// Variable name -> Type mapping
    pub bindings: HashMap<String, Type>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn extend(&self, var: String, ty: Type) -> Self {
        let mut new = self.clone();
        new.bindings.insert(var, ty);
        new
    }

    pub fn lookup(&self, var: &str) -> Option<&Type> {
        self.bindings.get(var)
    }
}

/// Prefix Automaton for type-constrained code generation
/// Key insight: Every prefix of a valid program must be completable to a valid program
pub struct PrefixAutomaton {
    /// All states in the automaton
    pub states: HashMap<usize, PrefixState>,
    /// Transition function: (state_id, token) -> Vec<next_state_ids>
    pub transitions: HashMap<(usize, Token), Vec<usize>>,
    /// Initial state
    pub initial_state: usize,
    /// Next state ID counter
    next_state_id: usize,
}

/// Tokens in our simplified language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    /// Identifier
    Ident(String),
    /// Literal value
    Literal(Literal),
    /// Keyword
    Keyword(Keyword),
    /// Operator
    Op(Operator),
    /// Punctuation
    Punct(char),
    /// End of input
    EOF,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    Int(i64),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    Let,
    Fn,
    If,
    Then,
    Else,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Gt,
    Assign,
    Apply, // Function application
}

impl PrefixAutomaton {
    pub fn new() -> Self {
        let mut automaton = Self {
            states: HashMap::new(),
            transitions: HashMap::new(),
            initial_state: 0,
            next_state_id: 1,
        };

        // Create initial state
        let initial = PrefixState {
            id: 0,
            is_accepting: true,
            is_final: false,
            type_context: TypeContext::new(),
            expected_type: None,
        };
        automaton.states.insert(0, initial);

        automaton
    }

    /// Add a new state and return its ID
    pub fn add_state(&mut self, is_accepting: bool, is_final: bool, context: TypeContext, expected: Option<Type>) -> usize {
        let id = self.next_state_id;
        self.next_state_id += 1;

        let state = PrefixState {
            id,
            is_accepting,
            is_final,
            type_context: context,
            expected_type: expected,
        };
        self.states.insert(id, state);
        id
    }

    /// Add a transition
    pub fn add_transition(&mut self, from: usize, token: Token, to: usize) {
        self.transitions
            .entry((from, token))
            .or_default()
            .push(to);
    }

    /// Parse a single token and return possible next states
    pub fn parse_token(&self, state_id: usize, token: &Token) -> Vec<usize> {
        self.transitions
            .get(&(state_id, token.clone()))
            .cloned()
            .unwrap_or_default()
    }

    /// Check if a state can reach a final state (prefix property verification)
    pub fn can_reach_final(&self, state_id: usize) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(state_id);

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(state) = self.states.get(&current) {
                if state.is_final {
                    return true;
                }

                // Add all reachable states
                for ((from, _), tos) in &self.transitions {
                    if *from == current {
                        for to in tos {
                            if !visited.contains(to) {
                                queue.push_back(*to);
                            }
                        }
                    }
                }
            }
        }

        false
    }
}

// ============================================================================
// PART 3: Type Inhabitation / Reachability Search
// ============================================================================

/// Type reachability graph node
#[derive(Debug, Clone)]
pub struct TypeNode {
    pub ty: Type,
    /// How we can construct this type (constructor applications)
    pub constructors: Vec<Constructor>,
}

/// A constructor that can produce a value of a given type
#[derive(Debug, Clone)]
pub struct Constructor {
    pub name: String,
    /// Argument types needed
    pub arg_types: Vec<Type>,
    /// Result type
    pub result_type: Type,
}

/// Type reachability search - determines if a type can be inhabited
pub struct TypeReachabilitySearch {
    /// Available constructors (variables, functions, literals)
    pub constructors: Vec<Constructor>,
    /// Cache for reachability results
    cache: HashMap<Type, bool>,
}

impl TypeReachabilitySearch {
    pub fn new(constructors: Vec<Constructor>) -> Self {
        Self {
            constructors,
            cache: HashMap::new(),
        }
    }

    /// Check if a type is reachable (inhabited) from the current context
    /// This is the core algorithm for type-constrained decoding
    pub fn is_reachable(&mut self, target: &Type) -> bool {
        // Check cache first
        if let Some(result) = self.cache.get(target) {
            return *result;
        }

        // BFS to find a path to inhabit the type
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with all constructors that match the target type
        for ctor in &self.constructors {
            if Self::types_match(&ctor.result_type, target) {
                // Check if all args are reachable
                if self.are_all_args_reachable(&ctor.arg_types) {
                    self.cache.insert(target.clone(), true);
                    return true;
                }
                // Add args to queue for further exploration
                for arg in &ctor.arg_types {
                    if !visited.contains(arg) {
                        visited.insert(arg.clone());
                        queue.push_back(arg.clone());
                    }
                }
            }
        }

        // BFS through the type space
        while let Some(current) = queue.pop_front() {
            if Self::types_match(&current, target) {
                self.cache.insert(target.clone(), true);
                return true;
            }

            for ctor in &self.constructors {
                if Self::types_match(&ctor.result_type, &current) {
                    for arg in &ctor.arg_types {
                        if !visited.contains(arg) {
                            visited.insert(arg.clone());
                            queue.push_back(arg.clone());
                        }
                    }
                }
            }
        }

        self.cache.insert(target.clone(), false);
        false
    }

    /// Check if all argument types are reachable
    fn are_all_args_reachable(&mut self, args: &[Type]) -> bool {
        args.iter().all(|arg| self.is_reachable(arg))
    }

    /// Type matching with support for generic patterns
    fn types_match(a: &Type, b: &Type) -> bool {
        match (a, b) {
            // Generic array pattern
            (_, Type::Base(p)) if p == "•[]" => matches!(a, Type::Array(_)),
            (Type::Base(p), _) if p == "•[]" => matches!(b, Type::Array(_)),
            // Exact equality
            _ => a == b,
        }
    }

    /// Find all constructors that can produce the target type
    pub fn find_constructors(&self, target: &Type) -> Vec<&Constructor> {
        self.constructors
            .iter()
            .filter(|c| Self::types_match(&c.result_type, target))
            .collect()
    }
}

// ============================================================================
// PART 4: Integration - Type-Constrained Decoder
// ============================================================================

/// Integrates prefix automaton with type reachability for constrained decoding
pub struct TypeConstrainedDecoder {
    pub automaton: PrefixAutomaton,
    pub type_search: TypeReachabilitySearch,
    /// Current state stack for parsing
    state_stack: Vec<usize>,
}

impl TypeConstrainedDecoder {
    pub fn new(automaton: PrefixAutomaton, type_search: TypeReachabilitySearch) -> Self {
        let initial = automaton.initial_state;
        Self {
            automaton,
            type_search,
            state_stack: vec![initial],
        }
    }

    /// Get valid next tokens at the current state
    /// This is the key function for constraining LLM decoding
    pub fn valid_next_tokens(&mut self) -> Vec<Token> {
        let current_state = *self.state_stack.last().unwrap_or(&0);

        let mut valid_tokens = Vec::new();

        // Get all possible transitions from current state
        for ((from, token), tos) in &self.automaton.transitions {
            if *from == current_state {
                // Check if any target state is accepting
                let has_accepting = tos.iter().any(|to| {
                    self.automaton
                        .states
                        .get(to)
                        .map(|s| s.is_accepting)
                        .unwrap_or(false)
                });

                if has_accepting {
                    // Additional type check: can we complete from here?
                    if self.can_complete_from_state(*tos.first().unwrap()) {
                        valid_tokens.push(token.clone());
                    }
                }
            }
        }

        valid_tokens
    }

    /// Check if we can complete a valid program from the given state
    fn can_complete_from_state(&mut self, state_id: usize) -> bool {
        if let Some(state) = self.automaton.states.get(&state_id) {
            // If there's an expected type, check reachability
            if let Some(expected) = &state.expected_type {
                return self.type_search.is_reachable(expected);
            }
            return true;
        }
        false
    }

    /// Step the decoder with a token
    pub fn step(&mut self, token: &Token) -> Result<(), DecodeError> {
        let current = *self.state_stack.last().unwrap_or(&0);
        let next_states = self.automaton.parse_token(current, token);

        if next_states.is_empty() {
            return Err(DecodeError::InvalidToken);
        }

        // Choose first accepting state
        let next = next_states
            .iter()
            .find(|&&s| {
                self.automaton
                    .states
                    .get(&s)
                    .map(|st| st.is_accepting)
                    .unwrap_or(false)
            })
            .copied()
            .unwrap_or(next_states[0]);

        self.state_stack.push(next);
        Ok(())
    }

    /// Check if we're in a final state
    pub fn is_complete(&self) -> bool {
        self.state_stack
            .last()
            .and_then(|&s| self.automaton.states.get(&s))
            .map(|s| s.is_final)
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub enum DecodeError {
    InvalidToken,
    TypeMismatch,
    IncompleteProgram,
}

// ============================================================================
// PART 5: Example Usage & Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_basic_constructors() -> Vec<Constructor> {
        vec![
            // Literals
            Constructor {
                name: "lit_int".to_string(),
                arg_types: vec![],
                result_type: Type::base("int"),
            },
            Constructor {
                name: "lit_bool".to_string(),
                arg_types: vec![],
                result_type: Type::base("bool"),
            },
            // Variables in context
            Constructor {
                name: "var_x".to_string(),
                arg_types: vec![],
                result_type: Type::base("int"),
            },
            Constructor {
                name: "var_y".to_string(),
                arg_types: vec![],
                result_type: Type::base("bool"),
            },
            // Addition: int -> int -> int
            Constructor {
                name: "add".to_string(),
                arg_types: vec![Type::base("int"), Type::base("int")],
                result_type: Type::base("int"),
            },
            // Less than: int -> int -> bool
            Constructor {
                name: "lt".to_string(),
                arg_types: vec![Type::base("int"), Type::base("int")],
                result_type: Type::base("bool"),
            },
        ]
    }

    #[test]
    fn test_type_reachability_basic() {
        let constructors = setup_basic_constructors();
        let mut search = TypeReachabilitySearch::new(constructors);

        // int should be reachable (via literal or variable)
        assert!(search.is_reachable(&Type::base("int")));

        // bool should be reachable
        assert!(search.is_reachable(&Type::base("bool")));

        // string should NOT be reachable (no constructor)
        assert!(!search.is_reachable(&Type::base("string")));
    }

    #[test]
    fn test_type_reachability_nested() {
        let constructors = setup_basic_constructors();
        let mut search = TypeReachabilitySearch::new(constructors);

        // Can we construct an int from addition?
        // add requires two ints, which are both reachable
        assert!(search.is_reachable(&Type::base("int")));

        // Can we construct a bool from comparison?
        // lt requires two ints, which are reachable
        assert!(search.is_reachable(&Type::base("bool")));
    }

    #[test]
    fn test_prefix_automaton_property() {
        let mut automaton = PrefixAutomaton::new();

        // Create a simple expression parser
        let s0 = automaton.initial_state;
        let s1 = automaton.add_state(true, false, TypeContext::new(), Some(Type::base("int")));
        let s2 = automaton.add_state(true, true, TypeContext::new(), None);

        automaton.add_transition(s0, Token::Literal(Literal::Int(0)), s1);
        automaton.add_transition(s1, Token::EOF, s2);

        // Verify prefix property: from s1 we should be able to reach final
        assert!(automaton.can_reach_final(s1));

        // s0 should also be able to reach final
        assert!(automaton.can_reach_final(s0));
    }

    #[test]
    fn test_constrained_decoder() {
        let automaton = PrefixAutomaton::new();
        let constructors = setup_basic_constructors();
        let type_search = TypeReachabilitySearch::new(constructors);

        let mut decoder = TypeConstrainedDecoder::new(automaton, type_search);

        // Initially should have valid tokens
        let valid = decoder.valid_next_tokens();
        // With empty automaton, no tokens are valid yet
        assert!(valid.is_empty());
    }
}

// ============================================================================
// PART 6: Performance Analysis & Benchmarks
// ============================================================================

/// Performance characteristics of the type-constrained decoding approach
pub struct PerformanceAnalysis;

impl PerformanceAnalysis {
    /// Theoretical complexity of type reachability search
    /// - Without cache: O(|constructors| * |type_space|)
    /// - With cache: O(1) for cached types, O(|constructors|) for new types
    pub fn type_reachability_complexity(num_constructors: usize, type_space_size: usize) -> String {
        format!(
            "O({} * {}) without cache, O({}) with cache",
            num_constructors, type_space_size, num_constructors
        )
    }

    /// Space complexity of prefix automaton
    /// - States: O(|grammar_rules|)
    /// - Transitions: O(|states| * |alphabet|)
    pub fn automaton_space_complexity(num_rules: usize, alphabet_size: usize) -> String {
        format!(
            "States: O({}), Transitions: O({} * {})",
            num_rules, num_rules, alphabet_size
        )
    }

    /// Key insight: The prefix property ensures that we never reject a valid partial program
    /// This is crucial for LLM decoding because:
    /// 1. LLMs generate token by token
    /// 2. Intermediate states must always be completable
    /// 3. Traditional parsers fail on incomplete programs
    pub fn prefix_property_importance() -> &'static str {
        "The prefix property guarantees that any partial program can be completed \
         to a valid, well-typed program. This is essential for LLM-based generation \
         because LLMs generate code incrementally, token by token. Without the prefix \
         property, we might reject valid partial programs, severely limiting the \
         LLM's ability to generate correct code."
    }
}

fn main() {
    println!("Type-Constrained Code Generation - Rust Implementation");
    println!("=======================================================");
    println!();

    // Demonstrate type system
    let int_type = Type::base("int");
    let bool_type = Type::base("bool");
    let int_to_bool = Type::arrow(Type::base("int"), Type::base("bool"));

    println!("Type Examples:");
    println!("  int: {}", int_type);
    println!("  bool: {}", bool_type);
    println!("  int -> bool: {}", int_to_bool);
    println!();

    // Demonstrate type reachability
    let constructors = vec![
        Constructor {
            name: "zero".to_string(),
            arg_types: vec![],
            result_type: Type::base("int"),
        },
        Constructor {
            name: "succ".to_string(),
            arg_types: vec![Type::base("int")],
            result_type: Type::base("int"),
        },
    ];

    let mut search = TypeReachabilitySearch::new(constructors);

    println!("Type Reachability Results:");
    println!("  int reachable: {}", search.is_reachable(&Type::base("int")));
    println!("  bool reachable: {}", search.is_reachable(&Type::base("bool")));
    println!();

    println!("Performance Characteristics:");
    println!(
        "  {}",
        PerformanceAnalysis::type_reachability_complexity(100, 50)
    );
    println!(
        "  {}",
        PerformanceAnalysis::automaton_space_complexity(1000, 50)
    );
    println!();

    println!("Key Insight:");
    println!("{}", PerformanceAnalysis::prefix_property_importance());
}
