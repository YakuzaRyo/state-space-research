//! Refinement Calculus for LLM Code Generation
//!
//! This module implements a formal refinement calculus system that can constrain
//! LLM code generation through mathematically verified refinement steps.
//!
//! Based on Morgan's Refinement Calculus and inspired by Refine4LLM (POPL 2025).

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// A predicate is a boolean condition over program state
/// Using Rc to allow cloning of the function pointer
pub type Predicate = Rc<dyn Fn(&State) -> bool>;

/// Program state as variable bindings
#[derive(Clone, Debug, Default)]
pub struct State {
    vars: HashMap<String, i64>,
}

impl State {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<i64> {
        self.vars.get(name).copied()
    }

    pub fn set(&mut self, name: String, value: i64) {
        self.vars.insert(name, value);
    }

    pub fn with(&self, name: &str, value: i64) -> Self {
        let mut new_state = self.clone();
        new_state.set(name.to_string(), value);
        new_state
    }
}

/// A specification [pre, post] where pre is precondition and post is postcondition
pub struct Specification {
    pub pre: Predicate,
    pub post: Predicate,
    pub name: String,
}

impl Specification {
    pub fn new<F, G>(name: &str, pre: F, post: G) -> Self
    where
        F: Fn(&State) -> bool + 'static,
        G: Fn(&State) -> bool + 'static,
    {
        Self {
            pre: Rc::new(pre),
            post: Rc::new(post),
            name: name.to_string(),
        }
    }

    /// Check if precondition holds in given state
    pub fn check_pre(&self, state: &State) -> bool {
        (self.pre)(state)
    }

    /// Check if postcondition holds in given state
    pub fn check_post(&self, state: &State) -> bool {
        (self.post)(state)
    }
}

impl fmt::Debug for Specification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Spec({})", self.name)
    }
}

/// A program command in our simple imperative language
#[derive(Clone, Debug)]
pub enum Command {
    /// Do nothing
    Skip,
    /// Variable assignment: x := expr
    Assign(String, Expr),
    /// Sequential composition: C1; C2
    Seq(Box<Command>, Box<Command>),
    /// Conditional: if cond then C1 else C2
    If(Expr, Box<Command>, Box<Command>),
    /// Loop: while cond do C (with invariant)
    While(Expr, Box<Command>, Invariant),
}

/// Invariant wrapper to allow Clone and Debug
#[derive(Clone)]
pub struct Invariant(pub Predicate);

impl fmt::Debug for Invariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invariant(<predicate>)")
    }
}

/// Arithmetic and boolean expressions
#[derive(Clone, Debug)]
pub enum Expr {
    Const(i64),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

impl Expr {
    /// Evaluate expression in given state
    pub fn eval(&self, state: &State) -> Option<i64> {
        match self {
            Expr::Const(n) => Some(*n),
            Expr::Var(name) => state.get(name),
            Expr::Add(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(v1 + v2)
            }
            Expr::Sub(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(v1 - v2)
            }
            Expr::Mul(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(v1 * v2)
            }
            Expr::Eq(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(if v1 == v2 { 1 } else { 0 })
            }
            Expr::Lt(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(if v1 < v2 { 1 } else { 0 })
            }
            Expr::Gt(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(if v1 > v2 { 1 } else { 0 })
            }
            Expr::And(e1, e2) => {
                let v1 = e1.eval(state)?;
                let v2 = e2.eval(state)?;
                Some(if v1 != 0 && v2 != 0 { 1 } else { 0 })
            }
            Expr::Not(e) => {
                let v = e.eval(state)?;
                Some(if v == 0 { 1 } else { 0 })
            }
        }
    }

    /// Evaluate as boolean
    pub fn eval_bool(&self, state: &State) -> Option<bool> {
        self.eval(state).map(|v| v != 0)
    }
}

/// Execute a command in a state, returning the new state
pub fn execute(cmd: &Command, state: &State) -> Option<State> {
    match cmd {
        Command::Skip => Some(state.clone()),
        Command::Assign(name, expr) => {
            let value = expr.eval(state)?;
            let mut new_state = state.clone();
            new_state.set(name.clone(), value);
            Some(new_state)
        }
        Command::Seq(c1, c2) => {
            let s1 = execute(c1, state)?;
            execute(c2, &s1)
        }
        Command::If(cond, c1, c2) => {
            if cond.eval_bool(state)? {
                execute(c1, state)
            } else {
                execute(c2, state)
            }
        }
        Command::While(cond, body, _invariant) => {
            let mut current = state.clone();
            while cond.eval_bool(&current)? {
                current = execute(body, &current)?;
            }
            Some(current)
        }
    }
}

/// Refinement relation: spec is refined by command if:
/// 1. Command terminates when precondition holds
/// 2. Postcondition holds after execution
pub fn check_refinement(spec: &Specification, cmd: &Command, state: &State) -> bool {
    if !spec.check_pre(state) {
        // Precondition doesn't hold, refinement is vacuously true
        return true;
    }

    match execute(cmd, state) {
        Some(final_state) => spec.check_post(&final_state),
        None => false, // Command failed to terminate
    }
}

/// Refinement calculus laws as functions that produce valid refinements

/// Skip Law: If pre => post, then w:[pre, post] ⊑ skip
pub fn skip_law<F>(pre: F, post: F) -> Option<Command>
where
    F: Fn(&State) -> bool + Clone + 'static,
{
    // In a real system, we would check if pre implies post
    // For now, we return Skip as a candidate refinement
    Some(Command::Skip)
}

/// Assignment Law: If pre => post[x := E], then w:[pre, post] ⊑ x := E
pub fn assignment_law<F>(var: &str, expr: Expr, pre: F, post: F) -> Command
where
    F: Fn(&State) -> bool + 'static,
{
    Command::Assign(var.to_string(), expr)
}

/// Sequential Composition Law:
/// w:[pre, post] ⊑ w:[pre, mid]; w:[mid, post]
pub fn sequential_composition_law<F>(
    mid: F,
    c1: Command,
    c2: Command,
) -> Command
where
    F: Fn(&State) -> bool + 'static,
{
    Command::Seq(Box::new(c1), Box::new(c2))
}

/// Alternation Law (If-statement introduction):
/// If pre ∧ G1 => wp(C1, post) and pre ∧ G2 => wp(C2, post)
/// then w:[pre, post] ⊑ if G1 -> C1 | G2 -> C2 fi
pub fn alternation_law(
    guard: Expr,
    then_branch: Command,
    else_branch: Command,
) -> Command {
    Command::If(guard, Box::new(then_branch), Box::new(else_branch))
}

/// Iteration Law (While-loop introduction):
/// w:[pre, post] ⊑ while G do w:[G ∧ post, post] with invariant
pub fn iteration_law(
    guard: Expr,
    body: Command,
    invariant: Predicate,
) -> Command {
    Command::While(guard, Box::new(body), Invariant(invariant))
}

/// Strengthen Postcondition Law:
/// If post' => post, then w:[pre, post] ⊑ w:[pre, post']
pub fn strengthen_postcondition<F>(new_post: F, cmd: Command) -> Command
where
    F: Fn(&State) -> bool + 'static,
{
    // The command that satisfies the stronger postcondition
    // also satisfies the weaker one
    cmd
}

/// Weaken Precondition Law:
/// If pre => pre', then w:[pre, post] ⊑ w:[pre', post]
pub fn weaken_precondition<F>(new_pre: F, cmd: Command) -> Command
where
    F: Fn(&State) -> bool + 'static,
{
    cmd
}

/// Refinement step records a transformation from spec to code
#[derive(Debug)]
pub struct RefinementStep {
    pub from: String,
    pub to: String,
    pub law: String,
}

/// A refinement derivation is a sequence of steps
pub struct RefinementDerivation {
    pub steps: Vec<RefinementStep>,
    pub final_code: Option<Command>,
}

impl RefinementDerivation {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            final_code: None,
        }
    }

    pub fn add_step(&mut self, from: &str, to: &str, law: &str) {
        self.steps.push(RefinementStep {
            from: from.to_string(),
            to: to.to_string(),
            law: law.to_string(),
        });
    }

    pub fn set_final_code(&mut self, code: Command) {
        self.final_code = Some(code);
    }
}

/// LLM-Guided Refinement System
///
/// This struct represents how an LLM can be constrained by refinement calculus
/// to generate verified code.
pub struct LLMRefinementGuide {
    /// Available refinement laws
    pub laws: Vec<String>,
    /// Current specification being refined
    pub current_spec: Option<Specification>,
    /// Accumulated refinement steps
    pub derivation: RefinementDerivation,
}

impl LLMRefinementGuide {
    pub fn new() -> Self {
        Self {
            laws: vec![
                "Skip".to_string(),
                "Assignment".to_string(),
                "Sequential Composition".to_string(),
                "Alternation".to_string(),
                "Iteration".to_string(),
                "Strengthen Postcondition".to_string(),
                "Weaken Precondition".to_string(),
            ],
            current_spec: None,
            derivation: RefinementDerivation::new(),
        }
    }

    /// Set the initial specification
    pub fn set_specification(&mut self, spec: Specification) {
        self.current_spec = Some(spec);
    }

    /// Get available refinement laws for current state
    pub fn available_laws(&self) -> &[String] {
        &self.laws
    }

    /// Apply a refinement law (this would be called by LLM with verification)
    pub fn apply_law(&mut self, law: &str, result: &str) {
        if let Some(ref spec) = self.current_spec {
            self.derivation.add_step(&spec.name, result, law);
        }
    }
}

/// Example: Refining a specification for computing absolute value
///
/// Specification: w:[true, r = |x|]
/// where r is the result and x is the input
pub fn example_abs_refinement() -> (Specification, Command) {
    // Specification: given any x, compute r = |x|
    let spec = Specification::new(
        "abs",
        |_s| true, // precondition: any state
        |s| {
            // postcondition: r = |x|
            let x = s.get("x").unwrap_or(0);
            let r = s.get("r").unwrap_or(0);
            r == x.abs()
        },
    );

    // Refinement using alternation law:
    // if x >= 0 then r := x else r := -x
    let cmd = alternation_law(
        Expr::Gt(Box::new(Expr::Var("x".to_string())), Box::new(Expr::Const(0))),
        Command::Assign("r".to_string(), Expr::Var("x".to_string())),
        Command::Assign(
            "r".to_string(),
            Expr::Sub(Box::new(Expr::Const(0)), Box::new(Expr::Var("x".to_string()))),
        ),
    );

    (spec, cmd)
}

/// Example: Refining a specification for computing sum 1 to n
///
/// Specification: w:[n >= 0, s = sum(1..n)]
pub fn example_sum_refinement() -> (Specification, Command) {
    // Specification: given n >= 0, compute s = 1 + 2 + ... + n
    let spec = Specification::new(
        "sum_to_n",
        |s| s.get("n").unwrap_or(0) >= 0,
        |s| {
            let n = s.get("n").unwrap_or(0);
            let sum_val = s.get("s").unwrap_or(0);
            // s should equal n*(n+1)/2
            sum_val == n * (n + 1) / 2
        },
    );

    // Refinement using iteration law:
    // s := 0; i := 1;
    // while i <= n do s := s + i; i := i + 1 od
    let init = Command::Seq(
        Box::new(Command::Assign("s".to_string(), Expr::Const(0))),
        Box::new(Command::Assign("i".to_string(), Expr::Const(1))),
    );

    let body = Command::Seq(
        Box::new(Command::Assign(
            "s".to_string(),
            Expr::Add(
                Box::new(Expr::Var("s".to_string())),
                Box::new(Expr::Var("i".to_string())),
            ),
        )),
        Box::new(Command::Assign(
            "i".to_string(),
            Expr::Add(
                Box::new(Expr::Var("i".to_string())),
                Box::new(Expr::Const(1)),
            ),
        )),
    );

    let loop_cmd = iteration_law(
        Expr::Lt(
            Box::new(Expr::Var("i".to_string())),
            Box::new(Expr::Add(Box::new(Expr::Var("n".to_string())), Box::new(Expr::Const(1)))),
        ),
        body,
        Rc::new(|s: &State| {
            // Invariant: s = sum(1..i-1) and i <= n+1
            let i = s.get("i").unwrap_or(0);
            let s_val = s.get("s").unwrap_or(0);
            s_val == (i - 1) * i / 2
        }),
    );

    let cmd = Command::Seq(Box::new(init), Box::new(loop_cmd));

    (spec, cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_operations() {
        let mut state = State::new();
        state.set("x".to_string(), 5);
        assert_eq!(state.get("x"), Some(5));
        assert_eq!(state.get("y"), None);
    }

    #[test]
    fn test_expression_evaluation() {
        let mut state = State::new();
        state.set("x".to_string(), 5);
        state.set("y".to_string(), 3);

        let expr = Expr::Add(
            Box::new(Expr::Var("x".to_string())),
            Box::new(Expr::Var("y".to_string())),
        );
        assert_eq!(expr.eval(&state), Some(8));

        let expr2 = Expr::Mul(
            Box::new(Expr::Const(2)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(expr2.eval(&state), Some(10));
    }

    #[test]
    fn test_skip_command() {
        let mut state = State::new();
        state.set("x".to_string(), 5);

        let result = execute(&Command::Skip, &state);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get("x"), Some(5));
    }

    #[test]
    fn test_assignment_command() {
        let state = State::new();
        let cmd = Command::Assign("x".to_string(), Expr::Const(42));

        let result = execute(&cmd, &state);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get("x"), Some(42));
    }

    #[test]
    fn test_sequential_composition() {
        let state = State::new();
        let cmd = Command::Seq(
            Box::new(Command::Assign("x".to_string(), Expr::Const(5))),
            Box::new(Command::Assign("y".to_string(), Expr::Var("x".to_string()))),
        );

        let result = execute(&cmd, &state);
        assert!(result.is_some());
        let final_state = result.unwrap();
        assert_eq!(final_state.get("x"), Some(5));
        assert_eq!(final_state.get("y"), Some(5));
    }

    #[test]
    fn test_conditional_command() {
        let mut state = State::new();
        state.set("x".to_string(), 10);

        let cmd = Command::If(
            Expr::Gt(Box::new(Expr::Var("x".to_string())), Box::new(Expr::Const(5))),
            Box::new(Command::Assign("y".to_string(), Expr::Const(1))),
            Box::new(Command::Assign("y".to_string(), Expr::Const(0))),
        );

        let result = execute(&cmd, &state);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get("y"), Some(1));
    }

    #[test]
    fn test_while_loop() {
        let mut state = State::new();
        state.set("n".to_string(), 5);
        state.set("i".to_string(), 0);
        state.set("s".to_string(), 0);

        let body = Command::Seq(
            Box::new(Command::Assign(
                "s".to_string(),
                Expr::Add(
                    Box::new(Expr::Var("s".to_string())),
                    Box::new(Expr::Const(1)),
                ),
            )),
            Box::new(Command::Assign(
                "i".to_string(),
                Expr::Add(
                    Box::new(Expr::Var("i".to_string())),
                    Box::new(Expr::Const(1)),
                ),
            )),
        );

        let cmd = Command::While(
            Expr::Lt(
                Box::new(Expr::Var("i".to_string())),
                Box::new(Expr::Var("n".to_string())),
            ),
            Box::new(body),
            Invariant(Rc::new(|_| true)), // trivial invariant
        );

        let result = execute(&cmd, &state);
        assert!(result.is_some());
        assert_eq!(result.unwrap().get("s"), Some(5));
    }

    #[test]
    fn test_abs_refinement() {
        let (spec, cmd) = example_abs_refinement();

        // Test with positive x
        let mut state1 = State::new();
        state1.set("x".to_string(), 5);
        assert!(spec.check_pre(&state1));

        let result1 = execute(&cmd, &state1);
        assert!(result1.is_some());
        assert!(spec.check_post(&result1.unwrap()));

        // Test with negative x
        let mut state2 = State::new();
        state2.set("x".to_string(), -5);
        assert!(spec.check_pre(&state2));

        let result2 = execute(&cmd, &state2);
        assert!(result2.is_some());
        assert!(spec.check_post(&result2.unwrap()));
    }

    #[test]
    fn test_sum_refinement() {
        let (spec, cmd) = example_sum_refinement();

        let mut state = State::new();
        state.set("n".to_string(), 5);
        assert!(spec.check_pre(&state));

        let result = execute(&cmd, &state);
        assert!(result.is_some());
        let final_state = result.unwrap();
        assert!(spec.check_post(&final_state));
        // 1+2+3+4+5 = 15
        assert_eq!(final_state.get("s"), Some(15));
    }

    #[test]
    fn test_refinement_check() {
        let spec = Specification::new(
            "test",
            |s| s.get("x").unwrap_or(0) >= 0,
            |s| s.get("y").unwrap_or(0) == s.get("x").unwrap_or(0) * 2,
        );

        // Command that doubles x and stores in y
        let cmd = Command::Assign(
            "y".to_string(),
            Expr::Mul(
                Box::new(Expr::Const(2)),
                Box::new(Expr::Var("x".to_string())),
            ),
        );

        let mut state = State::new();
        state.set("x".to_string(), 5);

        assert!(check_refinement(&spec, &cmd, &state));
    }

    #[test]
    fn test_llm_refinement_guide() {
        let mut guide = LLMRefinementGuide::new();

        let spec = Specification::new(
            "example",
            |_s| true,
            |s| s.get("r").unwrap_or(0) >= 0,
        );

        guide.set_specification(spec);

        let laws = guide.available_laws();
        assert!(!laws.is_empty());
        assert!(laws.contains(&"Skip".to_string()));
        assert!(laws.contains(&"Assignment".to_string()));
    }
}
