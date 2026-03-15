//! 状态转换模块
//!
//! 定义状态之间的转换

use crate::state::State;

/// 转换结果
#[derive(Debug, Clone)]
pub enum TransitionResult<T, E> {
    /// 转换成功
    Success(T),
    /// 转换失败
    Failure(E),
    /// 转换被拒绝（权限不足等）
    Rejected(String),
}

impl<T, E> TransitionResult<T, E> {
    pub fn is_success(&self) -> bool {
        matches!(self, TransitionResult::Success(_))
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, TransitionResult::Failure(_))
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self, TransitionResult::Rejected(_))
    }

    pub fn unwrap(self) -> T where E: std::fmt::Debug {
        match self {
            TransitionResult::Success(t) => t,
            TransitionResult::Failure(e) => panic!("unwrap failed: {:?}", e),
            TransitionResult::Rejected(s) => panic!("unwrap failed: rejected: {}", s),
        }
    }
}

/// 状态转换 trait
pub trait Transition<S: State> {
    /// 执行转换
    fn apply(&self, state: &S) -> TransitionResult<S, String>;

    /// 转换是否可逆
    fn reversible(&self) -> bool;
}

/// 转换链 - 按顺序执行多个转换
pub struct TransitionChain<S: State> {
    transitions: Vec<Box<dyn Transition<S>>>,
}

impl<S: State> TransitionChain<S> {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
        }
    }

    pub fn add<T: Transition<S> + 'static>(mut self, t: T) -> Self {
        self.transitions.push(Box::new(t));
        self
    }

    pub fn apply(&self, state: &S) -> TransitionResult<S, String> {
        let mut current = state.clone();
        for t in &self.transitions {
            match t.apply(&current) {
                TransitionResult::Success(s) => current = s,
                r @ TransitionResult::Failure(_) => return r,
                r @ TransitionResult::Rejected(_) => return r,
            }
        }
        TransitionResult::Success(current)
    }
}

impl<S: State> Default for TransitionChain<S> {
    fn default() -> Self {
        Self::new()
    }
}
