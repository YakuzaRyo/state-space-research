// 状态空间架构 - 无缺陷工具设计原型
// 文件: drafts/20260310_1310_tool_design.rs
// 核心理念: 纯函数 + 硬边界 = 无缺陷工具

use std::marker::PhantomData;

// ============================================================
// 第一部分: 状态空间基础类型
// ============================================================

/// 状态空间中的有效状态标记
/// T 表示状态的具体类型
pub struct StateSpace<T> {
    _phantom: PhantomData<T>,
}

/// 状态转换的结果
#[derive(Debug, Clone)]
pub enum TransitionResult<S> {
    Success(S),           // 转换成功，携带新状态
    InvalidTransition,    // 无效转换（软约束 - 仍可能被绕过）
    BoundaryViolation,    // 边界违规（硬边界 - 物理上不可能）
}

/// 纯函数式工具 trait
/// 核心原则：相同的输入总是产生相同的输出
pub trait PureTool<Input, Output, State> {
    fn execute(&self, input: Input, state: &State) -> TransitionResult<Output>;
}

// ============================================================
// 第二部分: 文件操作工具 - 展示硬边界设计
// ============================================================

/// 文件操作的可能模式 - 用类型系统强制约束
#[derive(Debug, Clone)]
pub enum FileMode {
    ReadOnly,      // 只读模式
    WriteNew,      // 写入新文件（不能覆盖）
    AppendOnly,    // 追加模式
}

/// 文件操作请求 - 泛型约束确保类型安全
pub struct FileRequest<M: FileModeMarker> {
    pub path: String,
    pub content: Option<String>,
    pub _mode: PhantomData<M>,
}

/// 文件模式标记 trait - 编译期类型安全
pub trait FileModeMarker {}
pub struct ReadOnlyMarker;
pub struct WriteNewMarker;
pub struct AppendMarker;

impl FileModeMarker for ReadOnlyMarker {}
impl FileModeMarker for WriteNewMarker {}
impl FileModeMarker for AppendMarker {}

/// 文件读取工具 - 只读模式
/// 硬边界设计：无论如何调用，都只能读取，无法写入
pub struct FileReadTool;

impl FileReadTool {
    pub fn new() -> Self {
        FileReadTool
    }

    /// 执行读取 - 纯函数
    /// Input: FileRequest<ReadOnlyMarker>
    /// Output: Result<String, FileError>
    pub fn execute(
        &self, 
        request: FileRequest<ReadOnlyMarker>
    ) -> TransitionResult<String> {
        // 边界检查在编译期完成
        // 运行时只需执行确定性操作
        TransitionResult::Success(format!("Content of: {}", request.path))
    }
}

/// 文件写入工具 - 只写新文件
/// 硬边界设计：无法覆盖已有文件
pub struct FileWriteNewTool;

impl FileWriteNewTool {
    pub fn new() -> Self {
        FileWriteNewTool
    }

    /// 执行写入 - 纯函数
    /// 硬边界：编译期保证不会传入覆盖模式
    pub fn execute(
        &self,
        request: FileRequest<WriteNewMarker>
    ) -> TransitionResult<String> {
        // 检查文件是否存在（如果需要）
        // 执行写入操作
        TransitionResult::Success(format!("Written to: {}", request.path))
    }
}

// ============================================================
// 第三部分: API边界设计 - 防止"prompt注入"
// ============================================================

/// API边界控制器
/// 核心思想：LLM只能通过受控API与系统交互
pub struct APIBoundary<InnerState> {
    inner: InnerState,
    // 硬边界：不可逾越的API限制
}

impl<InnerState> APIBoundary<InnerState> {
    /// 创建带边界的API控制器
    pub fn new(inner: InnerState) -> Self {
        APIBoundary { inner }
    }

    /// 受限的工具调用接口
    /// 硬边界：只暴露安全的工具子集
    pub fn call_tool<T, R>(
        &self,
        tool: &T,
        input: R
    ) -> TransitionResult<String>
    where
        T: PureTool<R, String, InnerState>,
    {
        // 边界检查：验证工具是否在白名单中
        // 硬边界：不在白名单中的工具根本无法被调用
        tool.execute(input, &self.inner)
    }
}

/// 工具白名单 - 编译期确定
/// 软约束（错误示例）: "LLM应该只使用白名单中的工具"
/// 硬边界（正确示例）: "LLM只能访问白名单中的工具，API物理上不提供其他入口"
struct ToolWhitelist;
impl ToolWhitelist {
    const ALLOWED: &'static [&'static str] = &[
        "file_read",
        "file_write_new",
        "search",
        "grep",
    ];

    pub fn is_allowed(tool_name: &str) -> bool {
        Self::ALLOWED.contains(&tool_name)
    }
}

// ============================================================
// 第四部分: 命令行工具设计模式
// ============================================================

/// CLI命令解析结果
#[derive(Debug, Clone)]
pub enum CliCommand {
    Read(String),
    Write(String, String),
    Search(String),
    Unknown,
}

/// CLI解析器 - 确定性解析，无二义性
pub struct CliParser;

impl CliParser {
    /// 解析命令 - 纯函数
    /// 相同的输入字符串总是产生相同的解析结果
    pub fn parse(input: &str) -> CliCommand {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        
        match parts.first() {
            Some(&"read") if parts.len() == 2 => {
                CliCommand::Read(parts[1].to_string())
            },
            Some(&"write") if parts.len() >= 3 => {
                CliCommand::Write(parts[1].to_string(), parts[2..].join(" "))
            },
            Some(&"search") if parts.len() == 2 => {
                CliCommand::Search(parts[1].to_string())
            },
            _ => CliCommand::Unknown,
        }
    }
}

// ============================================================
// 第五部分: 渐进式披露设计
// ============================================================

/// 工具复杂度级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityLevel {
    Basic,      // 基础操作
    Intermediate, // 中级操作
    Advanced,   // 高级操作
}

/// 根据用户级别披露不同工具
pub struct ToolDiscloser;

impl ToolDiscloser {
    /// 获取当前级别可用的工具列表
    pub fn available_tools(level: ComplexityLevel) -> Vec<&'static str> {
        match level {
            ComplexityLevel::Basic => vec![
                "read",
                "list",
                "search",
            ],
            ComplexityLevel::Intermediate => vec![
                "read",
                "list", 
                "search",
                "write_new",
                "edit",
            ],
            ComplexityLevel::Advanced => vec![
                "read",
                "list",
                "search",
                "write_new",
                "edit",
                "delete",
                "execute",
            ],
        }
    }

    /// 检查工具是否对当前级别可见
    pub fn is_visible(tool: &str, level: ComplexityLevel) -> bool {
        Self::available_tools(level).contains(&tool)
    }
}

// ============================================================
// 测试用例
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_read_tool() {
        let tool = FileReadTool::new();
        let request = FileRequest::<ReadOnlyMarker> {
            path: "test.txt".to_string(),
            content: None,
            _mode: PhantomData,
        };
        
        match tool.execute(request) {
            TransitionResult::Success(msg) => {
                assert!(msg.contains("test.txt"));
            },
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_cli_parser() {
        assert_eq!(
            CliParser::parse("read file.txt"),
            CliCommand::Read("file.txt".to_string())
        );
        
        assert_eq!(
            CliParser::parse("write file.txt hello"),
            CliCommand::Write("file.txt".to_string(), "hello".to_string())
        );
    }

    #[test]
    fn test_tool_discloser() {
        let basic_tools = ToolDiscloser::available_tools(ComplexityLevel::Basic);
        assert!(basic_tools.contains(&"read"));
        assert!(!basic_tools.contains(&"delete"));
        
        let advanced_tools = ToolDiscloser::available_tools(ComplexityLevel::Advanced);
        assert!(advanced_tools.contains(&"delete"));
    }
}
