//! 状态模块
//!
//! 定义系统状态和状态转换

use std::fmt::Debug;

/// 状态 trait - 所有状态必须实现
pub trait State: Debug + Clone + Send + Sync {
    /// 状态名称
    fn name(&self) -> &str;

    /// 状态是否有效
    fn is_valid(&self) -> bool;
}

/// 状态错误
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("状态无效: {0}")]
    InvalidState(String),

    #[error("状态转换失败: {0}")]
    TransitionFailed(String),

    #[error("状态不匹配: 期望 {expected}, 实际 {actual}")]
    StateMismatch { expected: String, actual: String },
}

/// 状态 trait 的默认实现
impl<S: Debug + Clone + Send + Sync> State for S {
    fn name(&self) -> &str {
        std::any::type_name::<S>()
    }

    fn is_valid(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestState {
        value: i32,
    }

    impl State for TestState {
        fn name(&self) -> &str {
            "TestState"
        }

        fn is_valid(&self) -> bool {
            self.value >= 0
        }
    }

    #[test]
    fn test_state_creation() {
        let state = TestState { value: 42 };
        assert!(state.is_valid());
        assert_eq!(state.name(), "TestState");
    }
}
