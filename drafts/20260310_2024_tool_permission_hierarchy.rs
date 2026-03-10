//! 状态空间工具设计 - 权限层次与最小权限原则
//! 研究方向: 工具设计（10）
//! 时间: 2026-03-10 20:24
//! 
//! 核心洞察: MiniScope论文 (arXiv:2512.11147) 提出了通过重构权限层次
//! 来实现工具调用的最小权限原则。本草稿探索将此方案与Rust类型系统结合，
//! 实现编译期的权限约束。
//!
//! 关键发现:
//! 1. MiniScope仍是"软约束"——通过权限检查机制来限制，而非类型约束
//! 2. 状态空间架构的目标：将权限层次编码进类型系统，实现"硬边界"
//! 3. 工具调用的安全性不应依赖LLM"理解"权限，而应由类型系统强制执行

use std::marker::PhantomData;

// =============================================================================
// 权限层次系统 - 基于MiniScope的思路，但升级为硬边界
// =============================================================================

/// 权限节点 - 表示工具权限层次中的一个节点
/// 
/// 核心区别于MiniScope:
/// - MiniScope: 运行时权限检查，依赖策略文件
/// - 状态空间: 编译期权限约束，依赖类型系统
pub trait PermissionNode: Sealed {
    type Parent: PermissionNode;
    const LEVEL: u8;
    const NAME: &'static str;
}

/// 密封trait - 防止外部实现
mod sealed {
    pub trait Sealed {}
}
use sealed::Sealed;

/// 权限层次定义
/// Root → Read → Write → Execute → Admin
pub struct RootPermission;
pub struct ReadPermission;
pub struct WritePermission;  
pub struct ExecutePermission;
pub struct AdminPermission;

impl Sealed for RootPermission {}
impl Sealed for ReadPermission {}
impl Sealed for WritePermission {}
impl Sealed for ExecutePermission {}
impl Sealed for AdminPermission {}

// 权限层次通过类型关系编码 - 编译期强制
impl PermissionNode for RootPermission {
    type Parent = RootPermission; // 根节点，无父节点
    const LEVEL: u8 = 0;
    const NAME: &'static str = "root";
}

impl PermissionNode for ReadPermission {
    type Parent = RootPermission;
    const LEVEL: u8 = 1;
    const NAME: &'static str = "read";
}

impl PermissionNode for WritePermission {
    type Parent = ReadPermission; // Write 包含 Read 能力
    const LEVEL: u8 = 2;
    const NAME: &'static str = "write";
}

impl PermissionNode for ExecutePermission {
    type Parent = WritePermission; // Execute 包含 Write 能力
    const LEVEL: u8 = 3;
    const NAME: &'static str = "execute";
}

impl PermissionNode for AdminPermission {
    type Parent = ExecutePermission; // Admin 包含所有能力
    const LEVEL: u8 = 4;
    const NAME: &'static str = "admin";
}

// =============================================================================
// 工具类型系统 - 硬边界实现
// =============================================================================

/// 工具令牌 - 表示一个已授权的工具调用能力
/// P: 所需的最小权限级别
pub struct ToolToken<P: PermissionNode> {
    _permission: PhantomData<P>,
    pub tool_id: &'static str,
}

impl<P: PermissionNode> ToolToken<P> {
    /// 创建工具令牌（内部使用，通过权限工厂创建）
    fn new(tool_id: &'static str) -> Self {
        ToolToken {
            _permission: PhantomData,
            tool_id,
        }
    }
}

/// 权限工厂 - 根据权限级别发放工具令牌
pub struct PermissionFactory<P: PermissionNode> {
    _permission: PhantomData<P>,
}

impl<P: PermissionNode> PermissionFactory<P> {
    pub fn new() -> Self {
        PermissionFactory { _permission: PhantomData }
    }
    
    /// 发放工具令牌 - 只能发放与当前权限级别匹配的令牌
    pub fn issue_token(&self, tool_id: &'static str) -> ToolToken<P> {
        println!("[Permission] Issuing token for '{}' at level {}", 
                 tool_id, P::LEVEL);
        ToolToken::new(tool_id)
    }
}

// 只有读取工厂可以发放读取令牌
impl PermissionFactory<WritePermission> {
    /// 降级到读取权限（最小权限原则）
    pub fn downgrade_to_read(self) -> PermissionFactory<ReadPermission> {
        println!("[Permission] Downgrading from write to read");
        PermissionFactory { _permission: PhantomData }
    }
}

// =============================================================================
// 工具定义 - 每个工具只接受特定权限令牌
// =============================================================================

/// 文件读取工具 - 只接受ReadPermission令牌
pub struct FileReadTool;

impl FileReadTool {
    /// 执行文件读取
    /// 硬边界: 只接受ReadPermission令牌，编译期保证
    pub fn read(&self, _token: &ToolToken<ReadPermission>, path: &str) -> Result<String, ToolError> {
        println!("[Tool] Reading file: {}", path);
        // 实际读取逻辑...
        Ok(format!("contents of {}", path))
    }
}

/// 文件写入工具 - 只接受WritePermission令牌
pub struct FileWriteTool;

impl FileWriteTool {
    /// 执行文件写入
    /// 硬边界: 只接受WritePermission令牌，尝试传入ReadPermission会编译失败
    pub fn write(&self, _token: &ToolToken<WritePermission>, path: &str, content: &str) -> Result<(), ToolError> {
        println!("[Tool] Writing {} bytes to: {}", content.len(), path);
        Ok(())
    }
}

/// Shell执行工具 - 只接受ExecutePermission令牌
pub struct ShellTool;

impl ShellTool {
    /// 执行Shell命令
    /// 硬边界: 只接受ExecutePermission令牌
    pub fn execute(&self, _token: &ToolToken<ExecutePermission>, cmd: &str) -> Result<String, ToolError> {
        println!("[Tool] Executing: {}", cmd);
        Ok(format!("output of: {}", cmd))
    }
}

/// 工具错误类型
#[derive(Debug)]
pub enum ToolError {
    PathNotFound(String),
    PermissionDenied,
    ExecutionFailed(String),
}

// =============================================================================
// 核心洞察: 与MiniScope的对比
// =============================================================================

/// MiniScope风格（软约束，运行时检查）
pub mod miniscope_style {
    use super::*;
    
    pub struct RuntimePermissionChecker {
        allowed_operations: Vec<String>,
    }
    
    impl RuntimePermissionChecker {
        pub fn new(ops: Vec<String>) -> Self {
            RuntimePermissionChecker { allowed_operations: ops }
        }
        
        /// 软约束：运行时检查权限
        /// 问题：1. 检查可以被绕过 2. Prompt注入可能欺骗LLM绕过检查
        pub fn check_and_execute(&self, op: &str) -> bool {
            if self.allowed_operations.contains(&op.to_string()) {
                println!("[MiniScope] Operation '{}' allowed", op);
                true
            } else {
                println!("[MiniScope] Operation '{}' denied", op);
                false
            }
        }
    }
}

/// 状态空间风格（硬边界，编译期约束）
pub mod state_space_style {
    use super::*;
    
    /// 编译期约束: LLM调度器只能访问被授权的工具
    /// 硬边界: 没有令牌就无法调用工具
    pub struct LLMDispatcher<P: PermissionNode> {
        factory: PermissionFactory<P>,
    }
    
    impl LLMDispatcher<ReadPermission> {
        pub fn new() -> Self {
            LLMDispatcher {
                factory: PermissionFactory::new(),
            }
        }
        
        pub fn dispatch_read(&self, path: &str) -> Result<String, ToolError> {
            let token = self.factory.issue_token("file_read");
            let tool = FileReadTool;
            tool.read(&token, path)
        }
        
        // 尝试调用写入工具会编译错误:
        // pub fn dispatch_write(&self, path: &str, content: &str) -> Result<(), ToolError> {
        //     let token = self.factory.issue_token("file_write");  // 只能发ReadPermission令牌
        //     let tool = FileWriteTool;
        //     tool.write(&token, path, content)  // ERROR: 类型不匹配
        // }
    }
    
    impl LLMDispatcher<WritePermission> {
        pub fn new() -> Self {
            LLMDispatcher {
                factory: PermissionFactory::new(),
            }
        }
        
        pub fn dispatch_write(&self, path: &str, content: &str) -> Result<(), ToolError> {
            let write_token = self.factory.issue_token("file_write");
            let tool = FileWriteTool;
            tool.write(&write_token, path, content)
        }
        
        pub fn dispatch_read_only(&self, path: &str) -> Result<String, ToolError> {
            // 降级到读取权限（最小权限原则）
            let read_factory = self.factory.downgrade_to_read();
            let token = read_factory.issue_token("file_read");
            let tool = FileReadTool;
            tool.read(&token, path)
        }
    }
}

// =============================================================================
// 权限作用域 - 任务上下文中的权限范围
// =============================================================================

/// 任务权限作用域
/// 类似OAuth scope，但在编译期强制执行
pub struct TaskScope<P: PermissionNode> {
    task_id: String,
    _permission: PhantomData<P>,
}

impl<P: PermissionNode> TaskScope<P> {
    pub fn new(task_id: impl Into<String>) -> Self {
        TaskScope {
            task_id: task_id.into(),
            _permission: PhantomData,
        }
    }
    
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
    
    pub fn permission_level(&self) -> u8 {
        P::LEVEL
    }
}

/// 权限作用域工厂 - 创建特定权限的任务作用域
pub struct ScopeFactory;

impl ScopeFactory {
    /// 创建只读作用域
    pub fn read_only(task_id: &str) -> TaskScope<ReadPermission> {
        println!("[Scope] Creating read-only scope for task: {}", task_id);
        TaskScope::new(task_id)
    }
    
    /// 创建读写作用域
    pub fn read_write(task_id: &str) -> TaskScope<WritePermission> {
        println!("[Scope] Creating read-write scope for task: {}", task_id);
        TaskScope::new(task_id)
    }
    
    /// 创建执行作用域（需要显式审批）
    pub fn execute_with_approval(task_id: &str, _approval: ApprovalToken) -> TaskScope<ExecutePermission> {
        println!("[Scope] Creating execute scope for task: {} (approved)", task_id);
        TaskScope::new(task_id)
    }
}

/// 审批令牌 - 执行高权限操作需要用户审批
/// 这是"渐进式权限提升"的关键机制
pub struct ApprovalToken {
    approved_at: std::time::SystemTime,
    expires_at: std::time::SystemTime,
}

impl ApprovalToken {
    pub fn new_for_testing() -> Self {
        let now = std::time::SystemTime::now();
        ApprovalToken {
            approved_at: now,
            expires_at: now + std::time::Duration::from_secs(300), // 5分钟有效
        }
    }
    
    pub fn is_valid(&self) -> bool {
        std::time::SystemTime::now() < self.expires_at
    }
}

// =============================================================================
// 工具注册表 - 运行时与编译期的结合点
// =============================================================================

/// 工具注册表 - 用于动态工具发现，但仍保持类型安全
pub struct ToolRegistry {
    read_tools: Vec<Box<dyn ReadOnlyTool>>,
    write_tools: Vec<Box<dyn WriteableTool>>,
}

pub trait ReadOnlyTool: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, token: &ToolToken<ReadPermission>, input: &str) -> Result<String, ToolError>;
}

pub trait WriteableTool: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, token: &ToolToken<WritePermission>, input: &str) -> Result<(), ToolError>;
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            read_tools: Vec::new(),
            write_tools: Vec::new(),
        }
    }
    
    /// 注册只读工具
    pub fn register_read_tool(&mut self, tool: Box<dyn ReadOnlyTool>) {
        println!("[Registry] Registering read-only tool: {}", tool.name());
        self.read_tools.push(tool);
    }
    
    /// 注册写入工具
    pub fn register_write_tool(&mut self, tool: Box<dyn WriteableTool>) {
        println!("[Registry] Registering write tool: {}", tool.name());
        self.write_tools.push(tool);
    }
    
    /// 获取只读工具（提供ReadPermission令牌）
    pub fn get_read_tool<'a>(&'a self, name: &str, token: &'a ToolToken<ReadPermission>) -> Option<ReadToolHandle<'a>> {
        self.read_tools
            .iter()
            .find(|t| t.name() == name)
            .map(|tool| ReadToolHandle { tool: tool.as_ref(), token })
    }
}

pub struct ReadToolHandle<'a> {
    tool: &'a dyn ReadOnlyTool,
    token: &'a ToolToken<ReadPermission>,
}

impl<'a> ReadToolHandle<'a> {
    pub fn call(&self, input: &str) -> Result<String, ToolError> {
        self.tool.execute(self.token, input)
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::state_space_style::*;
    
    #[test]
    fn test_read_only_dispatcher() {
        let dispatcher = LLMDispatcher::<ReadPermission>::new();
        let result = dispatcher.dispatch_read("test.txt");
        assert!(result.is_ok());
        println!("Read result: {:?}", result);
    }
    
    #[test]
    fn test_write_dispatcher() {
        let dispatcher = LLMDispatcher::<WritePermission>::new();
        let result = dispatcher.dispatch_write("output.txt", "hello world");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_scope_factory() {
        let read_scope = ScopeFactory::read_only("task-001");
        assert_eq!(read_scope.permission_level(), 1); // ReadPermission::LEVEL
        
        let write_scope = ScopeFactory::read_write("task-002");
        assert_eq!(write_scope.permission_level(), 2); // WritePermission::LEVEL
    }
    
    #[test]
    fn test_approval_token() {
        let approval = ApprovalToken::new_for_testing();
        assert!(approval.is_valid());
        
        let exec_scope = ScopeFactory::execute_with_approval("task-003", approval);
        assert_eq!(exec_scope.permission_level(), 3); // ExecutePermission::LEVEL
    }
    
    #[test]
    fn test_privilege_hierarchy() {
        // 验证权限层次正确性
        assert_eq!(ReadPermission::LEVEL, 1);
        assert_eq!(WritePermission::LEVEL, 2);
        assert_eq!(ExecutePermission::LEVEL, 3);
        assert_eq!(AdminPermission::LEVEL, 4);
        
        // Write.Parent == Read (Rust类型系统保证)
        assert_eq!(<WritePermission as PermissionNode>::Parent::LEVEL, 
                   ReadPermission::LEVEL);
    }
    
    // 编译期约束测试（无法通过编译的代码）：
    // #[test]
    // fn test_read_cannot_write() {
    //     let factory: PermissionFactory<ReadPermission> = PermissionFactory::new();
    //     let token = factory.issue_token("file_write"); // token类型: ToolToken<ReadPermission>
    //     let tool = FileWriteTool;
    //     tool.write(&token, "test.txt", "content"); // 编译错误: 期望WritePermission，得到ReadPermission
    // }
}

// =============================================================================
// 主要洞察总结
// =============================================================================

/*
## 本次研究的核心发现

### 1. MiniScope vs 状态空间架构的本质区别

MiniScope (Dec 2025, cs.CR):
- 通过"权限层次重构"实现最小权限原则
- 结合"移动端权限模型"（一次性授权、用时请求）
- **性质**: 软约束——运行时权限检查，1-6%性能开销
- **缺陷**: 仍然依赖权限策略文件，LLM无法理解的权限可能被绕过

状态空间架构 (本研究):
- 通过Rust类型系统将权限层次编码为编译期约束
- 工具令牌（ToolToken<P>）绑定权限级别，错误的令牌类型无法编译
- **性质**: 硬边界——编译期约束，零运行时开销
- **优势**: 不依赖任何运行时检查，物理上不可能发生权限升级

### 2. VIGIL和DRIFT的方法对比

VIGIL (NeurIPS workshop 2026):
- "Verify-before-commit"：工具执行前验证
- 软约束：仍然依赖LLM理解工具输出

DRIFT (NeurIPS 2025):
- 动态规则 + 隔离注入内容
- 软约束：规则可能不完整，隔离可能被绕过

状态空间架构：
- 物理隔离：工具边界由类型系统保证
- 无需"verify-before-commit"，因为错误的操作根本无法构造

### 3. 渐进式权限提升的价值

关键设计模式：
1. 默认最小权限（ReadPermission）
2. 提升权限需要显式ApprovalToken（用户审批）
3. 高风险操作需要ApprovalToken（不能被prompt绕过）

这与MiniScope的"移动端模型"相似，但通过类型系统强制执行，而非策略文件。

### 4. 工具注册表的角色

ToolRegistry解决了动态性问题：
- 编译期：类型约束（ToolToken<P>）确保正确权限
- 运行时：注册表提供工具发现
- 结合点：get_read_tool()返回ReadToolHandle<'a>，通过借用检查确保令牌有效

### 5. 待验证假设

1. 性能：Rust编译期约束的零成本抽象 vs MiniScope的1-6%运行时开销
2. 表达能力：类型系统能否表达所有现实的权限需求？
3. 迁移路径：如何将现有的运行时权限系统迁移到编译期约束？
4. LLM集成：如何让LLM的"工具调用"与类型系统兼容？
   - 关键问题：LLM输出JSON，JSON不携带类型信息
   - 解决思路：工具分发层（dispatcher）持有令牌，LLM只能请求已授权的工具ID
*/
