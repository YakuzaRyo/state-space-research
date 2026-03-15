//! 状态空间框架核心模块
//!
//! # 设计原则
//!
//! 1. **硬边界**: 通过类型系统强制约束，物理上不可能绕过
//! 2. **编译期验证**: 所有约束在编译期检查
//! 3. **纯函数**: 相同的输入总是产生相同的输出
//! 4. **失败快速**: 错误在最早可能点被捕获
//!
//! # 核心概念
//!
//! - `State`: 状态 - 表示系统的当前状态
//! - `Transition`: 状态转换 - 状态之间的转换
//! - `Tool`: 工具 - 执行特定操作的函数
//! - `Boundary`: 边界 - 工具操作的允许范围

pub mod state;
pub mod transition;
pub mod tool;
pub mod boundary;

pub use state::{State, StateError};
pub use transition::{Transition, TransitionResult};
pub use tool::{Tool, ToolInput, ToolOutput};
pub use boundary::{Boundary, Permission};

/// 框架版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
