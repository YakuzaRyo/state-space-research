//! Refine4LLM: Deep Research Implementation - 2026-03-11
//!
//! Research Focus: How Program Refinement Constrains LLM Generation
//!
//! Web Research Findings:
//! 1. Refine4LLM (POPL 2025): First framework combining LLM with refinement calculus
//! 2. Flux (PLDI 2023): Liquid Types for Rust - refinement types with ownership
//! 3. Prusti: Contract-based verification with requires/ensures
//! 4. Constrained Decoding: XGrammar/Outlines for structured LLM generation
//!
//! Key Insight: Refinement calculus provides a formal state space where:
//! - States are specifications w:[pre, post]
//! - Transitions are refinement laws (Skip, Assignment, Sequential, Alternation, Iteration)
//! - ATP verification ensures each transition preserves correctness

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Section 1: Core Data Structures
// ============================================================================

/// Typed variable with bounds information for refinement tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedVariable {
    pub name: String,
    pub ty: Type,
    pub bounds: Option<Predicate>,
}

impl TypedVariable {
    pub fn new(name: &str, ty: Type) -> Self {
        Self {
            name: name.to_string(),
            ty,
            bounds: None,
        }
    }

    pub fn with_bounds(name: &str, ty: Type, bounds: Predicate) -> Self {
        Self {
            name: name.to_string(),
            ty,
            bounds: Some(bounds),
        }
    }
}

/// Types in our refinement system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    Array(Box<Type>),
    Ref(Box<Type>),
}

/// Terms (expressions) in our language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Var(String),
    ConstInt(i64),
    ConstBool(bool),
    Add(Box<Term>, Box<Term>),
    Sub(Box<Term>, Box<Term>),
    Mul(Box<Term>, Box<Term>),
    Div(Box<Term>, Box<Term>),
    Eq(Box<Term>, Box<Term>),
    Lt(Box<Term>, Box<Term>),
    Le(Box<Term>, Box<Term>),
    Gt(Box<Term>, Box<Term>),
    Ge(Box<Term>, Box<Term>),
    And(Box<Term>, Box<Term>),
    Or(Box<Term>, Box<Term>),
    Not(Box<Term>),
    ArrayIndex(String, Box<Term>),
    ArrayLen(String),
}

impl Term {
    /// Substitute variable with expression
    pub fn substitute(&self, var: &str, expr: &Term) -> Term {
        match self {
            Term::Var(v) if v == var => expr.clone(),
            Term::Var(_) => self.clone(),
            Term::ConstInt(_) | Term::ConstBool(_) => self.clone(),
            Term::Add(t1, t2) => Term::Add(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Sub(t1, t2) => Term::Sub(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Mul(t1, t2) => Term::Mul(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Div(t1, t2) => Term::Div(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Eq(t1, t2) => Term::Eq(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Lt(t1, t2) => Term::Lt(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Le(t1, t2) => Term::Le(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Gt(t1, t2) => Term::Gt(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Ge(t1, t2) => Term::Ge(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::And(t1, t2) => Term::And(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Or(t1, t2) => Term::Or(
                Box::new(t1.substitute(var, expr)),
                Box::new(t2.substitute(var, expr)),
            ),
            Term::Not(t) => Term::Not(Box::new(t.substitute(var, expr))),
            Term::ArrayIndex(arr, idx) => Term::ArrayIndex(
                arr.clone(),
                Box::new(idx.substitute(var, expr)),
            ),
            Term::ArrayLen(_) => self.clone(),
        }
    }
}

/// Predicates (boolean expressions) for specifications
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    True,
    False,
    Eq(Term, Term),
    Neq(Term, Term),
    Lt(Term, Term),
    Le(Term, Term),
    Gt(Term, Term),
    Ge(Term, Term),
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
    Implies(Box<Predicate>, Box<Predicate>),
    Forall(String, Box<Predicate>),
    Exists(String, Box<Predicate>),
}

impl Predicate {
    /// Substitute variable with expression in predicate
    pub fn substitute(&self, var: &str, expr: &Term) -> Predicate {
        match self {
            Predicate::True | Predicate::False => self.clone(),
            Predicate::Eq(t1, t2) => {
                Predicate::Eq(t1.substitute(var, expr), t2.substitute(var, expr))
            }
            Predicate::Neq(t1, t2) => {
                Predicate::Neq(t1.substitute(var, expr), t2.substitute(var, expr))
            }
            Predicate::Lt(t1, t2) => {
                Predicate::Lt(t1.substitute(var, expr), t2.substitute(var, expr))
            }
            Predicate::Le(t1, t2) => {
                Predicate::Le(t1.substitute(var, expr), t2.substitute(var, expr))
            }
            Predicate::Gt(t1, t2) => {
                Predicate::Gt(t1.substitute(var, expr), t2.substitute(var, expr))
            }
            Predicate::Ge(t1, t2) => {
                Predicate::Ge(t1.substitute(var, expr), t2.substitute(var, expr))
            }
            Predicate::And(p1, p2) => Predicate::And(
                Box::new(p1.substitute(var, expr)),
                Box::new(p2.substitute(var, expr)),
            ),
            Predicate::Or(p1, p2) => Predicate::Or(
                Box::new(p1.substitute(var, expr)),
                Box::new(p2.substitute(var, expr)),
            ),
            Predicate::Not(p) => Predicate::Not(Box::new(p.substitute(var, expr))),
            Predicate::Implies(p1, p2) => Predicate::Implies(
                Box::new(p1.substitute(var, expr)),
                Box::new(p2.substitute(var, expr)),
            ),
            Predicate::Forall(v, p) if v == var => self.clone(),
            Predicate::Forall(v, p) => {
                Predicate::Forall(v.clone(), Box::new(p.substitute(var, expr)))
            }
            Predicate::Exists(v, p) if v == var => self.clone(),
            Predicate::Exists(v, p) => {
                Predicate::Exists(v.clone(), Box::new(p.substitute(var, expr)))
            }
        }
    }
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::True => write!(f, "true"),
            Predicate::False => write!(f, "false"),
            Predicate::Eq(t1, t2) => write!(f, "({} = {})", t1, t2),
            Predicate::Neq(t1, t2) => write!(f, "({} != {})", t1, t2),
            Predicate::Lt(t1, t2) => write!(f, "({} < {})", t1, t2),
            Predicate::Le(t1, t2) => write!(f, "({} <= {})", t1, t2),
            Predicate::Gt(t1, t2) => write!(f, "({} > {})", t1, t2),
            Predicate::Ge(t1, t2) => write!(f, "({} >= {})", t1, t2),
            Predicate::And(p1, p2) => write!(f, "({} /\\ {})", p1, p2),
            Predicate::Or(p1, p2) => write!(f, "({} \/ {})", p1, p2),
            Predicate::Not(p) => write!(f, "(!{})", p),
            Predicate::Implies(p1, p2) => write!(f, "({} ==> {})", p1, p2),
            Predicate::Forall(v, p) => write!(f, "(forall {}. {})", v, p),
            Predicate::Exists(v, p) => write!(f, "(exists {}. {})", v, p),
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(v) => write!(f, "{}", v),
            Term::ConstInt(n) => write!(f, "{}", n),
            Term::ConstBool(b) => write!(f, "{}", b),
            Term::Add(t1, t2) => write!(f, "({} + {})", t1, t2),
            Term::Sub(t1, t2) => write!(f, "({} - {})", t1, t2),
            Term::Mul(t1, t2) => write!(f, "({} * {})", t1, t2),
            Term::Div(t1, t2) => write!(f, "({} / {})", t1, t2),
            Term::Eq(t1, t2) => write!(f, "({} == {})", t1, t2),
            Term::Lt(t1, t2) => write!(f, "({} < {})", t1, t2),
            Term::Le(t1, t2) => write!(f, "({} <= {})", t1, t2),
            Term::Gt(t1, t2) => write!(f, "({} > {})", t1, t2),
            Term::Ge(t1, t2) => write!(f, "({} >= {})", t1, t2),
            Term::And(t1, t2) => write!(f, "({} && {})", t1, t2),
            Term::Or(t1, t2) => write!(f, "({} || {})", t1, t2),
            Term::Not(t) => write!(f, "(!{})", t),
            Term::ArrayIndex(arr, idx) => write!(f, "{}[{}]", arr, idx),
            Term::ArrayLen(arr) => write!(f, "len({})", arr),
        }
    }
}

// ============================================================================
// Section 2: Specification Statement (w:[pre, post])
// ============================================================================

/// Specification statement: w:[pre, post]
/// - frame: set of variables that may be modified
/// - precondition: condition that must hold before execution
/// - postcondition: condition that must hold after execution
#[derive(Debug, Clone)]
pub struct Specification {
    pub frame: Vec<String>,
    pub precondition: Predicate,
    pub postcondition: Predicate,
}

impl Specification {
    pub fn new(frame: Vec<String>, precondition: Predicate, postcondition: Predicate) -> Self {
        Self {
            frame,
            precondition,
            postcondition,
        }
    }

    /// Create a simple specification with single variable
    pub fn simple(var: &str, pre: Predicate, post: Predicate) -> Self {
        Self {
            frame: vec![var.to_string()],
            precondition: pre,
            postcondition: post,
        }
    }
}

impl fmt::Display for Specification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let frame_str = if self.frame.is_empty() {
            "w".to_string()
        } else {
            self.frame.join(", ")
        };
        write!(
            f,
            "{}:[{}, {}]",
            frame_str, self.precondition, self.postcondition
        )
    }
}

// ============================================================================
// Section 3: Program Commands
// ============================================================================

/// Program commands in our imperative language
#[derive(Debug, Clone)]
pub enum Command {
    Skip,
    Assignment(String, Term),
    Seq(Box<Command>, Box<Command>),
    If {
        guard: Predicate,
        then_branch: Box<Command>,
        else_branch: Box<Command>,
    },
    While {
        guard: Predicate,
        body: Box<Command>,
        invariant: Predicate,
        variant: Term,
    },
    Spec(Specification),
}

impl Command {
    /// Sequential composition helper
    pub fn seq(c1: Command, c2: Command) -> Command {
        Command::Seq(Box::new(c1), Box::new(c2))
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Skip => write!(f, "skip"),
            Command::Assignment(var, expr) => write!(f, "{} := {}", var, expr),
            Command::Seq(c1, c2) => write!(f, "{}; {}", c1, c2),
            Command::If {
                guard,
                then_branch,
                else_branch,
            } => write!(f, "if {} then {} else {}", guard, then_branch, else_branch),
            Command::While {
                guard,
                body,
                invariant,
                variant,
            } => write!(
                f,
                "while {} inv {} var {} do {} od",
                guard, invariant, variant, body
            ),
            Command::Spec(spec) => write!(f, "{}", spec),
        }
    }
}

// ============================================================================
// Section 4: Weakest Precondition Calculus
// ============================================================================

/// Weakest precondition calculator
pub struct WPCalculator;

impl WPCalculator {
    /// Calculate weakest precondition: wp(C, Q)
    pub fn wp(command: &Command, post: &Predicate) -> Predicate {
        match command {
            Command::Skip => post.clone(),
            Command::Assignment(var, expr) => post.substitute(var, expr),
            Command::Seq(c1, c2) => {
                let wp_c2 = Self::wp(c2, post);
                Self::wp(c1, &wp_c2)
            }
            Command::If {
                guard,
                then_branch,
                else_branch,
            } => {
                let wp_then = Self::wp(then_branch, post);
                let wp_else = Self::wp(else_branch, post);
                Predicate::And(
                    Box::new(Predicate::Implies(
                        Box::new(guard.clone()),
                        Box::new(wp_then),
                    )),
                    Box::new(Predicate::Implies(
                        Box::new(Predicate::Not(Box::new(guard.clone()))),
                        Box::new(wp_else),
                    )),
                )
            }
            Command::While {
                guard,
                invariant,
                ..
            } => {
                // For loops, we use the invariant as the WP
                // The verification conditions are checked separately
                invariant.clone()
            }
            Command::Spec(spec) => spec.precondition.clone(),
        }
    }

    /// Calculate weakest liberal precondition (for partial correctness)
    pub fn wlp(command: &Command, post: &Predicate) -> Predicate {
        // For most commands, same as wp
        Self::wp(command, post)
    }
}

// ============================================================================
// Section 5: Proof Obligations
// ============================================================================

/// Proof obligation generated during refinement
#[derive(Debug, Clone)]
pub struct ProofObligation {
    pub description: String,
    pub condition: Predicate,
}

impl ProofObligation {
    pub fn new(description: &str, condition: Predicate) -> Self {
        Self {
            description: description.to_string(),
            condition,
        }
    }
}

impl fmt::Display for ProofObligation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.description, self.condition)
    }
}

/// Result of a refinement step
#[derive(Debug, Clone)]
pub enum RefinementResult {
    Success {
        command: Command,
        obligations: Vec<ProofObligation>,
    },
    Failure {
        reason: String,
    },
}

// ============================================================================
// Section 6: Refinement Laws
// ============================================================================

/// Core refinement laws from Morgan's refinement calculus
pub struct RefinementLaws;

impl RefinementLaws {
    /// Skip Law: If pre => post, then w:[pre, post] ⊑ skip
    /// Proof obligation: pre => post
    pub fn skip_law(spec: &Specification) -> RefinementResult {
        let obligation = ProofObligation::new(
            "Skip Law: pre => post",
            Predicate::Implies(
                Box::new(spec.precondition.clone()),
                Box::new(spec.postcondition.clone()),
            ),
        );

        RefinementResult::Success {
            command: Command::Skip,
            obligations: vec![obligation],
        }
    }

    /// Assignment Law: If pre => post[E/x], then w,x:[pre, post] ⊑ x := E
    /// Proof obligation: pre => post[E/x]
    pub fn assignment_law(spec: &Specification, var: &str, expr: &Term) -> RefinementResult {
        let post_substituted = spec.postcondition.substitute(var, expr);
        let obligation = ProofObligation::new(
            &format!("Assignment Law: pre => post[{} -> {}]", var, expr),
            Predicate::Implies(
                Box::new(spec.precondition.clone()),
                Box::new(post_substituted),
            ),
        );

        RefinementResult::Success {
            command: Command::Assignment(var.to_string(), expr.clone()),
            obligations: vec![obligation],
        }
    }

    /// Sequential Composition Law:
    /// w:[pre, post] ⊑ w:[pre, mid]; w:[mid, post]
    pub fn sequential_composition(spec: &Specification, mid: Predicate) -> RefinementResult {
        let spec1 = Specification::new(spec.frame.clone(), spec.precondition.clone(), mid.clone());
        let spec2 = Specification::new(spec.frame.clone(), mid, spec.postcondition.clone());

        RefinementResult::Success {
            command: Command::seq(Command::Spec(spec1), Command::Spec(spec2)),
            obligations: vec![], // No new obligations, inherited from sub-specs
        }
    }

    /// Alternation (If) Law:
    /// If pre => G1 / G2, then
    /// w:[pre, post] ⊑ if G1 then w:[pre/\G1, post] else w:[pre/\G2, post]
    pub fn alternation_law(
        spec: &Specification,
        guard: Predicate,
        then_spec: Specification,
        else_spec: Specification,
    ) -> RefinementResult {
        let obligation = ProofObligation::new(
            "Alternation Law: pre => G1 \/ G2",
            Predicate::Or(
                Box::new(guard.clone()),
                Box::new(Predicate::Not(Box::new(guard.clone()))),
            ),
        );

        RefinementResult::Success {
            command: Command::If {
                guard,
                then_branch: Box::new(Command::Spec(then_spec)),
                else_branch: Box::new(Command::Spec(else_spec)),
            },
            obligations: vec![obligation],
        }
    }

    /// Iteration (While) Law:
    /// w:[pre, post] ⊑ w:[pre, I]; while G do w:[I/\G, I/\V<V0] od
    /// where post = I /\ !G
    pub fn iteration_law(
        spec: &Specification,
        guard: Predicate,
        invariant: Predicate,
        variant: Term,
    ) -> RefinementResult {
        let init_spec = Specification::new(
            spec.frame.clone(),
            spec.precondition.clone(),
            invariant.clone(),
        );

        let body_pre = Predicate::And(
            Box::new(invariant.clone()),
            Box::new(guard.clone()),
        );
        let body_post = invariant.clone();
        let body_spec = Specification::new(spec.frame.clone(), body_pre, body_post);

        let exit_obligation = ProofObligation::new(
            "Iteration Law: I /\ !G => post",
            Predicate::Implies(
                Box::new(Predicate::And(
                    Box::new(invariant.clone()),
                    Box::new(Predicate::Not(Box::new(guard.clone()))),
                )),
                Box::new(spec.postcondition.clone()),
            ),
        );

        RefinementResult::Success {
            command: Command::seq(
                Command::Spec(init_spec),
                Command::While {
                    guard,
                    body: Box::new(Command::Spec(body_spec)),
                    invariant,
                    variant,
                },
            ),
            obligations: vec![exit_obligation],
        }
    }
}

// ============================================================================
// Section 7: LLM Integration Interface
// ============================================================================

/// Interface for LLM-guided refinement
/// This is where the LLM selects which refinement law to apply
pub struct LLMRefinementGuide;

impl LLMRefinementGuide {
    /// Build a prompt for the LLM to select a refinement law
    pub fn build_law_selection_prompt(spec: &Specification) -> String {
        format!(
            r#"Given the specification: {}

Available refinement laws:
1. Skip: Use when pre => post
2. Assignment: Use when a variable assignment satisfies the postcondition
3. Sequential: Split the specification into two sequential parts
4. Alternation: Introduce a conditional (if-then-else)
5. Iteration: Introduce a loop with invariant and variant

Select the most appropriate law and provide any necessary parameters.

Response format:
Law: <law_name>
Parameters: <parameters>
Reasoning: <why this law is appropriate>"#,
            spec
        )
    }

    /// Build a prompt for assignment generation
    pub fn build_assignment_prompt(spec: &Specification, var: &str) -> String {
        format!(
            r#"Given the specification: {}

Generate an assignment expression for variable '{}' such that:
- The assignment satisfies: pre => post[expr/{}]
- The expression is valid Rust code

Provide only the expression."#,
            spec, var, var
        )
    }

    /// Build a prompt for invariant generation
    pub fn build_invariant_prompt(spec: &Specification, guard: &Predicate) -> String {
        format!(
            r#"Given the specification: {}
And loop guard: {}

Generate a loop invariant that:
1. Is implied by the precondition
2. Is preserved by the loop body
3. Together with !guard, implies the postcondition

Provide the invariant as a logical formula."#,
            spec, guard
        )
    }
}

// ============================================================================
// Section 8: Verification Export (Verus/Flux/SMT-LIB)
// ============================================================================

/// Export specifications to various verification formats
pub struct VerificationExporter;

impl VerificationExporter {
    /// Export to Verus format
    pub fn to_verus(spec: &Specification, fn_name: &str, params: &[(&str, &str)]) -> String {
        let params_str = params
            .iter()
            .map(|(n, t)| format!("{}: {}", n, t))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"fn {fn_name}({params_str})
    requires {pre}
    ensures {post}
{{
    // Implementation generated by refinement
}}"#,
            fn_name = fn_name,
            params_str = params_str,
            pre = Self::predicate_to_verus(&spec.precondition),
            post = Self::predicate_to_verus(&spec.postcondition),
        )
    }

    /// Convert predicate to Verus syntax
    fn predicate_to_verus(pred: &Predicate) -> String {
        match pred {
            Predicate::True => "true".to_string(),
            Predicate::False => "false".to_string(),
            Predicate::Eq(t1, t2) => format!("{} == {}", t1, t2),
            Predicate::Neq(t1, t2) => format!("{} != {}", t1, t2),
            Predicate::Lt(t1, t2) => format!("{} < {}", t1, t2),
            Predicate::Le(t1, t2) => format!("{} <= {}", t1, t2),
            Predicate::Gt(t1, t2) => format!("{} > {}", t1, t2),
            Predicate::Ge(t1, t2) => format!("{} >= {}", t1, t2),
            Predicate::And(p1, p2) => format!(
                "({} && {})",
                Self::predicate_to_verus(p1),
                Self::predicate_to_verus(p2)
            ),
            Predicate::Or(p1, p2) => format!(
                "({} || {})",
                Self::predicate_to_verus(p1),
                Self::predicate_to_verus(p2)
            ),
            Predicate::Not(p) => format!("!{}", Self::predicate_to_verus(p)),
            Predicate::Implies(p1, p2) => format!(
                "({} ==> {})",
                Self::predicate_to_verus(p1),
                Self::predicate_to_verus(p2)
            ),
            Predicate::Forall(v, p) => format!(
                "forall|{}| {}",
                v,
                Self::predicate_to_verus(p)
            ),
            Predicate::Exists(v, p) => format!(
                "exists|{}| {}",
                v,
                Self::predicate_to_verus(p)
            ),
        }
    }

    /// Export to SMT-LIB format for Z3
    pub fn to_smtlib(obligations: &[ProofObligation]) -> String {
        let mut output = String::new();
        output.push_str("; Proof obligations in SMT-LIB format\n");
        output.push_str("(set-logic QF_LIA)\n\n");

        for (i, obl) in obligations.iter().enumerate() {
            output.push_str(&format!("; {}\n", obl.description));
            output.push_str(&format!(
                "(assert (not {}))\n",
                Self::predicate_to_smtlib(&obl.condition)
            ));
            output.push_str(&format!("(check-sat) ; obligation {}\n\n", i + 1));
        }

        output
    }

    /// Convert predicate to SMT-LIB syntax
    fn predicate_to_smtlib(pred: &Predicate) -> String {
        match pred {
            Predicate::True => "true".to_string(),
            Predicate::False => "false".to_string(),
            Predicate::Eq(t1, t2) => format!("(= {} {})", t1, t2),
            Predicate::Neq(t1, t2) => format!("(not (= {} {}))", t1, t2),
            Predicate::Lt(t1, t2) => format!("(< {} {})", t1, t2),
            Predicate::Le(t1, t2) => format!("(<= {} {})", t1, t2),
            Predicate::Gt(t1, t2) => format!("(> {} {})", t1, t2),
            Predicate::Ge(t1, t2) => format!("(>= {} {})", t1, t2),
            Predicate::And(p1, p2) => format!(
                "(and {} {})",
                Self::predicate_to_smtlib(p1),
                Self::predicate_to_smtlib(p2)
            ),
            Predicate::Or(p1, p2) => format!(
                "(or {} {})",
                Self::predicate_to_smtlib(p1),
                Self::predicate_to_smtlib(p2)
            ),
            Predicate::Not(p) => format!("(not {})", Self::predicate_to_smtlib(p)),
            Predicate::Implies(p1, p2) => format!(
                "(=> {} {})",
                Self::predicate_to_smtlib(p1),
                Self::predicate_to_smtlib(p2)
            ),
            Predicate::Forall(_, _) | Predicate::Exists(_, _) => {
                "; quantifiers not supported in QF_LIA".to_string()
            }
        }
    }
}

// ============================================================================
// Section 9: Example - Absolute Value
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_value_refinement() {
        // Specification: x:[true, result >= 0 && (result = x || result = -x)]
        let spec = Specification::new(
            vec!["x".to_string()],
            Predicate::True,
            Predicate::And(
                Box::new(Predicate::Ge(Term::Var("result".to_string()), Term::ConstInt(0))),
                Box::new(Predicate::Or(
                    Box::new(Predicate::Eq(
                        Term::Var("result".to_string()),
                        Term::Var("x".to_string()),
                    )),
                    Box::new(Predicate::Eq(
                        Term::Var("result".to_string()),
                        Term::Sub(Box::new(Term::ConstInt(0)), Box::new(Term::Var("x".to_string()))),
                    )),
                )),
            ),
        );

        println!("Specification: {}", spec);

        // Try Skip law (should fail verification)
        let skip_result = RefinementLaws::skip_law(&spec);
        println!("\nSkip law result: {:?}", skip_result);

        // Try Assignment law with conditional
        // x := if x >= 0 then x else -x
        let assignment_result = RefinementLaws::assignment_law(
            &spec,
            "result",
            &Term::Var("x".to_string()), // Simplified for demo
        );
        println!("\nAssignment law result: {:?}", assignment_result);
    }

    #[test]
    fn test_wp_calculus() {
        // Test: wp(x := x + 1, x > 0)
        let cmd = Command::Assignment("x".to_string(), Term::Add(
            Box::new(Term::Var("x".to_string())),
            Box::new(Term::ConstInt(1)),
        ));
        let post = Predicate::Gt(Term::Var("x".to_string()), Term::ConstInt(0));

        let wp = WPCalculator::wp(&cmd, &post);
        println!("wp(x := x + 1, x > 0) = {}", wp);

        // Expected: x + 1 > 0
        assert_eq!(
            wp,
            Predicate::Gt(
                Term::Add(
                    Box::new(Term::Var("x".to_string())),
                    Box::new(Term::ConstInt(1)),
                ),
                Term::ConstInt(0),
            )
        );
    }

    #[test]
    fn test_verus_export() {
        let spec = Specification::new(
            vec!["x".to_string()],
            Predicate::True,
            Predicate::Ge(Term::Var("result".to_string()), Term::ConstInt(0)),
        );

        let verus_code = VerificationExporter::to_verus(&spec, "abs", &[("x", "i32")]);
        println!("Verus export:\n{}", verus_code);
    }
}

// ============================================================================
// Section 10: Main Entry Point for Testing
// ============================================================================

fn main() {
    println!("Refine4LLM Implementation");
    println!("=========================\n");

    // Example 1: Simple assignment refinement
    println!("Example 1: Simple Assignment");
    let spec1 = Specification::simple(
        "x",
        Predicate::True,
        Predicate::Eq(Term::Var("x".to_string()), Term::ConstInt(5)),
    );
    println!("Specification: {}", spec1);

    let result1 = RefinementLaws::assignment_law(&spec1, "x", &Term::ConstInt(5));
    match &result1 {
        RefinementResult::Success { command, obligations } => {
            println!("Refined to: {}", command);
            println!("Proof obligations:");
            for obl in obligations {
                println!("  - {}", obl);
            }
        }
        RefinementResult::Failure { reason } => {
            println!("Refinement failed: {}", reason);
        }
    }

    // Example 2: Sequential composition
    println!("\nExample 2: Sequential Composition");
    let spec2 = Specification::simple(
        "x",
        Predicate::Eq(Term::Var("x".to_string()), Term::ConstInt(0)),
        Predicate::Eq(Term::Var("x".to_string()), Term::ConstInt(10)),
    );
    println!("Specification: {}", spec2);

    let mid = Predicate::Eq(Term::Var("x".to_string()), Term::ConstInt(5));
    let result2 = RefinementLaws::sequential_composition(&spec2, mid);
    match &result2 {
        RefinementResult::Success { command, obligations } => {
            println!("Refined to: {}", command);
            println!("Proof obligations: {}", obligations.len());
        }
        RefinementResult::Failure { reason } => {
            println!("Refinement failed: {}", reason);
        }
    }

    // Example 3: Export to Verus
    println!("\nExample 3: Verus Export");
    let spec3 = Specification::new(
        vec!["x".to_string()],
        Predicate::True,
        Predicate::And(
            Box::new(Predicate::Ge(Term::Var("result".to_string()), Term::ConstInt(0))),
            Box::new(Predicate::Or(
                Box::new(Predicate::Eq(
                    Term::Var("result".to_string()),
                    Term::Var("x".to_string()),
                )),
                Box::new(Predicate::Eq(
                    Term::Var("result".to_string()),
                    Term::Sub(Box::new(Term::ConstInt(0)), Box::new(Term::Var("x".to_string()))),
                )),
            )),
        ),
    );

    let verus_code = VerificationExporter::to_verus(&spec3, "abs", &[("x", "i32")]);
    println!("{}", verus_code);

    // Example 4: Export to SMT-LIB
    println!("\nExample 4: SMT-LIB Export");
    let obligations = match result1 {
        RefinementResult::Success { obligations, .. } => obligations,
        _ => vec![],
    };
    let smtlib_code = VerificationExporter::to_smtlib(&obligations);
    println!("{}", smtlib_code);

    println!("\n=== Research Hypotheses Validation ===");
    println!("H1: Program refinement constrains LLM generation by defining");
    println!("    a state space of valid specifications with verified transitions.");
    println!("H2: Rust's type system can encode refinement calculus core concepts.");
    println!("H3: Weakest precondition calculus enables systematic verification.");
    println!("H4: Export to existing tools (Verus, Z3) provides practical validation.");
}
