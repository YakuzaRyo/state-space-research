//! 工具模块
//!
//! 定义 AI 工具的类型系统

use std::fmt::Debug;

/// 工具输入
#[derive(Debug, Clone)]
pub struct ToolInput {
    /// 输入数据
    pub data: Vec<u8>,
    /// 输入元数据
    pub metadata: std::collections::HashMap<String, String>,
}

impl ToolInput {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// 工具输出
#[derive(Debug, Clone)]
pub struct ToolOutput {
    /// 输出数据
    pub data: Vec<u8>,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
}

impl ToolOutput {
    pub fn success(data: Vec<u8>) -> Self {
        Self {
            data,
            success: true,
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            data: Vec::new(),
            success: false,
            error: Some(error.into()),
        }
    }
}

/// 工具 trait - 定义工具的接口
///
/// 工具必须是纯函数：相同的输入总是产生相同的输出
pub trait Tool: Debug + Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;

    /// 工具描述
    fn description(&self) -> &str;

    /// 执行工具
    fn execute(&self, input: ToolInput) -> ToolOutput;

    /// 工具是否只读
    fn is_readonly(&self) -> bool;
}

/// 工具执行错误
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("工具执行失败: {0}")]
    ExecutionFailed(String),

    #[error("工具不存在: {0}")]
    ToolNotFound(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),
}
