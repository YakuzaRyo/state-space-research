// 工具组合与事务性设计
// 2026-03-10 21:12 工具设计研究方向

use std::marker::PhantomData;

// ============================================================
// 第一部分：基础工具 trait 定义
// ============================================================

/// 工具输入 trait
pub trait ToolInput: Sized {
    type Validated;
    fn validate(self) -> Result<Self::Validated, InputError>;
}

/// 工具输出 trait
pub trait ToolOutput: Sized {
    fn is_success(&self) -> bool;
}

/// 基础工具 trait
pub trait Tool {
    type Input: ToolInput;
    type Output: ToolOutput;
    
    fn execute(&self, input: Self::Input) -> Self::Output;
}

/// 输入错误类型
#[derive(Debug)]
pub enum InputError {
    EmptyInput,
    InvalidFormat(String),
    PermissionDenied,
    PathNotFound(String),
}

// ============================================================
// 第二部分：工具组合 - 类型安全的组合机制
// ============================================================

/// 工具链：组合两个工具
pub struct ToolChain<A: Tool, B: Tool> {
    first: A,
    second: B,
    _marker: PhantomData<fn(A::Output) -> B::Input>,
}

impl<A: Tool, B: Tool> ToolChain<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            _marker: PhantomData,
        }
    }
    
    /// 执行工具链
    /// - 如果第一个工具失败，整个链条失败
    /// - 如果第一个工具成功但第二个失败，第一个工具的效果需要回滚
    pub fn execute(&self, input: A::Input) -> ChainResult<A, B> {
        // 1. 执行第一个工具
        let first_output = self.first.execute(input);
        
        if !first_output.is_success() {
            return ChainResult::FirstFailed(first_output);
        }
        
        // 2. 将第一个工具的输出转换为第二个工具的输入
        // 注意：这里需要类型转换实现
        // let second_input = transform(first_output);
        
        // 3. 执行第二个工具（这里简化处理）
        // let second_output = self.second.execute(second_input);
        
        // ChainResult::Completed(second_output)
        ChainResult::FirstSucceeded(first_output) // 占位
    }
}

/// 链式执行结果
pub enum ChainResult<A: Tool, B: Tool> {
    FirstFailed(A::Output),
    FirstSucceeded(A::Output),
    // SecondFailed { first: A::Output, error: B::Error },
    // Completed(B::Output),
}

// ============================================================
// 第三部分：类型匹配 trait - 工具组合的约束
// ============================================================

/// 输出到输入的转换 trait
/// 用于约束哪些工具可以组合在一起
pub trait InputFor<NextTool: Tool> {
    type Output;
    
    /// 将当前输出转换为下一个工具的输入
    fn transform(self) -> Result<NextTool::Input, TransformError>;
}

/// 转换错误
#[derive(Debug)]
pub enum TransformError {
    TypeMismatch(String),
    DataLoss(String),
    InvalidData(String),
}

/// 文件读取输出
pub struct FileReadOutput {
    pub content: String,
    pub path: String,
}

impl ToolOutput for FileReadOutput {
    fn is_success(&self) -> bool {
        true // 简化处理
    }
}

/// 文件写入输入
pub struct FileWriteInput {
    pub path: String,
    pub content: String,
}

impl ToolInput for FileWriteInput {
    type Validated = Self;
    
    fn validate(self) -> Result<Self::Validated, InputError> {
        if self.path.is_empty() {
            return Err(InputError::EmptyInput);
        }
        Ok(self)
    }
}

/// 文件读取 → 文件写入：类型匹配
impl InputFor<FileWriteTool> for FileReadOutput {
    type Output = FileWriteInput;
    
    fn transform(self) -> Result<FileWriteInput, TransformError> {
        Ok(FileWriteInput {
            path: self.path, // 简化：写入到同一路径
            content: self.content,
        })
    }
}

/// 文件写入工具
pub struct FileWriteTool;

impl Tool for FileWriteTool {
    type Input = FileWriteInput;
    type Output = FileWriteOutput;
    
    fn execute(&self, input: Self::Input) -> Self::Output {
        // 实际写入文件的逻辑
        // 这里简化处理
        FileWriteOutput {
            content: input.content,
            path: input.path,
        }
    }
}

// ============================================================
// 第四部分：事务性 - 回滚机制
// ============================================================

/// 可回滚操作 trait
pub trait Rollbackable {
    type RollbackAction: Tool;
    
    /// 执行操作并返回结果
    fn execute(self) -> RollbackResult<Self::Output, Self::RollbackAction::Output>;
    
    /// 回滚操作
    fn rollback(self) -> Self::RollbackAction::Output;
}

/// 事务结果
pub enum RollbackResult<O, R> {
    Success(O),
    RolledBack(R),
    Failed(String),
}

/// 文件写入操作（可回滚）
pub struct FileWriteAction {
    pub path: String,
    pub content: String,
    pub original_exists: bool,
    pub original_content: Option<String>,
}

impl Rollbackable for FileWriteAction {
    type RollbackAction = FileDeleteAction;
    
    fn execute(self) -> RollbackResult<(), ()> {
        // 实际写入文件
        // let success = write_file(&self.path, &self.content);
        // if success {
        //     RollbackResult::Success(())
        // } else {
        //     RollbackResult::Failed("Write failed".to_string())
        // }
        RollbackResult::Success(()) // 占位
    }
    
    fn rollback(self) -> Self::RollbackAction::Output {
        // 如果原文件不存在，删除新创建的文件
        // 如果原文件存在，恢复原内容
        FileDeleteAction { path: self.path }.execute()
    }
}

/// 文件删除操作
pub struct FileDeleteAction {
    pub path: String,
}

impl Tool for FileDeleteAction {
    type Input = Self;
    type Output = ();
    
    fn execute(&self, input: Self::Input) -> Self::Output {
        // 删除文件
        // if std::path::Path::new(&input.path).exists() {
        //     std::fs::remove_file(&input.path).ok();
        // }
        ()
    }
}

/// 事务执行器 - 保证原子性
pub struct TransactionExecutor;

impl TransactionExecutor {
    /// 执行可回滚操作
    pub fn execute<T: Rollbackable>(action: T) -> RollbackResult<T::Output, T::RollbackAction::Output> {
        let result = action.execute();
        
        match result {
            RollbackResult::Success(output) => {
                RollbackResult::Success(output)
            }
            RollbackResult::Failed(_) => {
                // 失败时自动回滚
                // let rollback_output = action.rollback();
                // RollbackResult::RolledBack(rollback_output)
                RollbackResult::RolledBack(()) // 占位
            }
            other => other,
        }
    }
}

// ============================================================
// 第五部分：工具链 + 事务性组合
// ============================================================

/// 带事务的工具链
pub struct TransactionalChain<A: Rollbackable, B: Rollbackable> {
    first: A,
    second: B,
}

impl<A: Rollbackable, B: Rollbackable> TransactionalChain<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
    
    /// 执行事务性工具链
    pub fn execute(self) -> TransactionalResult<A, B> {
        // 1. 执行第一个操作
        let first_result = self.first.execute();
        
        match first_result {
            RollbackResult::Success(_) => {
                // 2. 第一个成功，执行第二个
                let second_result = self.second.execute();
                
                match second_result {
                    RollbackResult::Success(output) => {
                        TransactionalResult::AllSucceeded(output)
                    }
                    RollbackResult::Failed(_) => {
                        // 3. 第二个失败，回滚第一个
                        self.first.rollback();
                        TransactionalResult::RolledBack
                    }
                    RollbackResult::RolledBack(r) => {
                        TransactionalResult::RolledBack
                    }
                }
            }
            RollbackResult::Failed(_) => {
                // 第一个失败，无需回滚（未执行任何操作）
                TransactionalResult::FirstFailed
            }
            RollbackResult::RolledBack(r) => {
                TransactionalResult::RolledBack
            }
        }
    }
}

/// 事务性链结果
pub enum TransactionalResult<A: Rollbackable, B: Rollbackable> {
    AllSucceeded(B::Output),
    FirstFailed,
    RolledBack,
}

// ============================================================
// 第六部分：示例 - 代码重构工具链
// ============================================================

/// 示例：读取文件 → 分析 → 写入新文件 → 删除旧文件
/// 这是一个需要事务性保证的真实场景

/// 读取文件操作
struct ReadFileAction { path: String }

/// 分析文件操作
struct AnalyzeFileAction;

/// 写入新文件操作
struct WriteNewFileAction { new_path: String, content: String };

/// 删除旧文件操作  
struct DeleteOldFileAction { old_path: String };

/// 工具链执行器示例
fn execute_refactor_chain(
    read: ReadFileAction,
    analyze: AnalyzeFileAction,
    write: WriteNewFileAction,
    delete: DeleteOldFileAction,
) -> RefactorResult {
    // 1. 读取
    let content = read.execute();
    
    // 2. 分析
    let analyzed = analyze.execute(content);
    
    // 3. 写入新文件
    let write_result = write.execute(analyzed);
    
    // 4. 如果写入成功，删除旧文件
    // 如果写入失败，需要回滚（但这里没有新文件需要删除）
    match write_result {
        RollbackResult::Success(_) => {
            delete.execute();
            RefactorResult::Success
        }
        RollbackResult::Failed(e) => {
            RefactorResult::Failed(e)
        }
        RollbackResult::RolledBack(_) => {
            RefactorResult::Failed("Rolled back".to_string())
        }
    }
}

enum RefactorResult {
    Success,
    Failed(String),
}

// ============================================================
// 第七部分：类型安全的工具描述（供LLM理解）
// ============================================================

/// 工具元数据 - 供LLM理解工具能力
#[derive(Debug)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub input_type: &'static str,
    pub output_type: &'static str,
    pub can_rollback: bool,
    pub required_permissions: Vec<Permission>,
}

/// 权限级别
#[derive(Debug, Clone, Copy)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Delete,
}

/// 获取工具元数据的 trait
pub trait ToolDescriptor {
    fn metadata() -> ToolMetadata;
}

/// 文件写入工具的描述
impl ToolDescriptor for FileWriteTool {
    fn metadata() -> ToolMetadata {
        ToolMetadata {
            name: "FileWrite".to_string(),
            description: "写入内容到文件".to_string(),
            input_type: "FileWriteInput",
            output_type: "FileWriteOutput",
            can_rollback: true,
            required_permissions: vec![Permission::Write],
        }
    }
}

/// 组合两个工具的描述
impl<A: ToolDescriptor, B: ToolDescriptor> ToolDescriptor for ToolChain<A, B> {
    fn metadata() -> ToolMetadata {
        ToolMetadata {
            name: format!("{}→{}", A::metadata().name, B::metadata().name),
            description: format!("{} 然后 {}", A::metadata().description, B::metadata().description),
            input_type: A::metadata().input_type,
            output_type: B::metadata().output_type,
            can_rollback: A::metadata().can_rollback,
            required_permissions: {
                let mut perms = A::metadata().required_permissions;
                perms.extend(B::metadata().required_permissions);
                perms
            },
        }
    }
}

// ============================================================
// 使用示例
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_chain_type_matching() {
        // 这个测试验证类型系统是否正确约束工具组合
        
        // FileReadOutput -> FileWriteInput 是类型兼容的
        // 编译通过表示类型匹配正确
        
        let read_output = FileReadOutput {
            content: "test content".to_string(),
            path: "/test/path".to_string(),
        };
        
        // 转换
        let write_input: FileWriteInput = read_output.transform().unwrap();
        
        assert_eq!(write_input.content, "test content");
        assert_eq!(write_input.path, "/test/path");
    }
    
    #[test]
    fn test_tool_descriptor() {
        // 获取工具元数据
        let metadata = FileWriteTool::metadata();
        
        assert_eq!(metadata.name, "FileWrite");
        assert!(metadata.can_rollback);
    }
    
    #[test]
    fn test_rollbackable() {
        let action = FileWriteAction {
            path: "/test/file.txt".to_string(),
            content: "test".to_string(),
            original_exists: false,
            original_content: None,
        };
        
        // 执行并检查回滚能力
        let result = TransactionExecutor::execute(action);
        
        // 结果应该是成功或回滚
        match result {
            RollbackResult::Success(_) | RollbackResult::RolledBack(_) => {
                // 预期行为
            }
            RollbackResult::Failed(e) => {
                panic!("Transaction failed: {}", e);
            }
        }
    }
}
