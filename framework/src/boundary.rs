//! 边界模块
//!
//! 定义工具操作的边界和权限

use std::fmt::Debug;

/// 权限级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    /// 只读权限
    Read,
    /// 写入权限
    Write,
    /// 执行权限
    Execute,
    /// 管理员权限
    Admin,
}

impl Permission {
    pub fn can_read(&self) -> bool {
        true // 所有权限都包含读取
    }

    pub fn can_write(&self) -> bool {
        matches!(self, Permission::Write | Permission::Execute | Permission::Admin)
    }

    pub fn can_execute(&self) -> bool {
        matches!(self, Permission::Execute | Permission::Admin)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, Permission::Admin)
    }
}

/// 边界 - 定义工具操作的允许范围
#[derive(Debug, Clone)]
pub struct Boundary {
    /// 边界名称
    name: String,
    /// 允许的操作
    allowed_operations: Vec<String>,
    /// 权限要求
    required_permission: Permission,
}

impl Boundary {
    pub fn new(name: impl Into<String>, permission: Permission) -> Self {
        Self {
            name: name.into(),
            allowed_operations: Vec::new(),
            required_permission: permission,
        }
    }

    pub fn allow(mut self, operation: impl Into<String>) -> Self {
        self.allowed_operations.push(operation.into());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn check_operation(&self, operation: &str, permission: Permission) -> Result<(), String> {
        // 检查权限级别
        if permission < self.required_permission {
            return Err(format!(
                "权限不足: 需要 {:?}, 当前 {:?}",
                self.required_permission, permission
            ));
        }

        // 检查操作是否允许
        if !self.allowed_operations.is_empty()
            && !self.allowed_operations.contains(&operation.to_string())
        {
            return Err(format!(
                "操作 '{}' 不在允许列表: {:?}",
                operation, self.allowed_operations
            ));
        }

        Ok(())
    }
}

/// 工具令牌 - 绑定工具和权限
#[derive(Debug, Clone)]
pub struct ToolToken<T> {
    tool: T,
    permission: Permission,
}

impl<T> ToolToken<T> {
    pub fn new(tool: T, permission: Permission) -> Self {
        Self { tool, permission }
    }

    pub fn permission(&self) -> Permission {
        self.permission
    }

    pub fn inner(&self) -> &T {
        &self.tool
    }
}
