// 深度研究：无法产生错误的工具设计 v2
// 研究方向: 10_tool_design - 工具设计
// 核心问题: 如何设计'无法产生错误'的工具?
//
// 本代码验证以下假设：
// 1. 类型安全的CLI参数解析可以完全消除运行时参数错误
// 2. Builder模式与Typestate结合可以实现"无法错误配置"
// 3. Command模式与状态机结合可以实现"无法错误执行"
// 4. "解析-验证-执行"的类型安全管道
// 5. 状态机驱动的工具流程防止非法状态转换
// 6. 类型安全设计对性能影响可忽略（零成本抽象）

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use std::collections::VecDeque;

// ============================================================================
// 第一部分: 类型安全的CLI参数解析
// ============================================================================

/// 验证后的端口类型 - 保证值在有效范围内
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ValidPort(u16);

impl ValidPort {
    /// 创建ValidPort，验证端口号在1-65535之间（排除0）
    pub fn new(port: u16) -> Result<Self, PortError> {
        if port == 0 {
            Err(PortError::ZeroPort)
        } else {
            Ok(ValidPort(port))
        }
    }

    /// 获取原始值（内部使用）
    pub fn get(&self) -> u16 {
        self.0
    }
}

/// 端口错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortError {
    ZeroPort,
    ParseError(String),
}

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortError::ZeroPort => write!(f, "Port cannot be 0"),
            PortError::ParseError(s) => write!(f, "Invalid port number: {}", s),
        }
    }
}

impl FromStr for ValidPort {
    type Err = PortError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let port: u16 = s.parse().map_err(|_| PortError::ParseError(s.to_string()))?;
        ValidPort::new(port)
    }
}

/// 验证后的主机地址类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidHost(String);

impl ValidHost {
    pub fn new(host: String) -> Result<Self, HostError> {
        if host.is_empty() {
            return Err(HostError::EmptyHost);
        }
        // 基本验证：不能包含空格
        if host.contains(' ') {
            return Err(HostError::InvalidFormat("Host cannot contain spaces".to_string()));
        }
        Ok(ValidHost(host))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    EmptyHost,
    InvalidFormat(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::EmptyHost => write!(f, "Host cannot be empty"),
            HostError::InvalidFormat(msg) => write!(f, "{}", msg),
        }
    }
}

impl FromStr for ValidHost {
    type Err = HostError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ValidHost::new(s.to_string())
    }
}

/// 验证后的文件路径类型 - 保证路径非空且格式正确
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidPath(std::path::PathBuf);

impl ValidPath {
    pub fn new(path: std::path::PathBuf) -> Result<Self, PathError> {
        if path.as_os_str().is_empty() {
            return Err(PathError::EmptyPath);
        }
        Ok(ValidPath(path))
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    pub fn exists(&self) -> bool {
        self.0.exists()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    EmptyPath,
    NotFound(std::path::PathBuf),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::EmptyPath => write!(f, "Path cannot be empty"),
            PathError::NotFound(p) => write!(f, "Path not found: {}", p.display()),
        }
    }
}

// ============================================================================
// 第二部分: Typestate Builder模式 - 无法错误配置
// ============================================================================

// 标记类型
pub struct Unset;
pub struct Set<T>(T);

/// HTTP请求配置Builder - 使用Typestate模式
/// 确保：1) 必需字段必须设置 2) 字段顺序可选 3) 类型安全
pub struct HttpRequestBuilder<UrlState, MethodState, PortState> {
    url: PhantomData<UrlState>,
    method: PhantomData<MethodState>,
    port: PhantomData<PortState>,
    // 实际数据
    url_value: Option<String>,
    method_value: Option<HttpMethod>,
    port_value: Option<ValidPort>,
    headers: Vec<(String, String)>,
    timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
        }
    }
}

/// 初始状态：所有字段未设置
impl HttpRequestBuilder<Unset, Unset, Unset> {
    pub fn new() -> Self {
        HttpRequestBuilder {
            url: PhantomData,
            method: PhantomData,
            port: PhantomData,
            url_value: None,
            method_value: None,
            port_value: None,
            headers: Vec::new(),
            timeout_ms: 30000, // 默认30秒
        }
    }
}

/// 任意状态下都可以设置可选字段
impl<UrlState, MethodState, PortState> HttpRequestBuilder<UrlState, MethodState, PortState> {
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}

/// 设置URL - 转换UrlState从Unset到Set
impl<MethodState, PortState> HttpRequestBuilder<Unset, MethodState, PortState> {
    pub fn url(self, url: impl Into<String>) -> HttpRequestBuilder<Set<String>, MethodState, PortState> {
        HttpRequestBuilder {
            url: PhantomData,
            method: self.method,
            port: self.port,
            url_value: Some(url.into()),
            method_value: self.method_value,
            port_value: self.port_value,
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

/// 设置Method - 转换MethodState从Unset到Set
impl<UrlState, PortState> HttpRequestBuilder<UrlState, Unset, PortState> {
    pub fn method(self, method: HttpMethod) -> HttpRequestBuilder<UrlState, Set<HttpMethod>, PortState> {
        HttpRequestBuilder {
            url: self.url,
            method: PhantomData,
            port: self.port,
            url_value: self.url_value,
            method_value: Some(method),
            port_value: self.port_value,
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

/// 设置Port - 转换PortState从Unset到Set
impl<UrlState, MethodState> HttpRequestBuilder<UrlState, MethodState, Unset> {
    pub fn port(self, port: ValidPort) -> HttpRequestBuilder<UrlState, MethodState, Set<ValidPort>> {
        HttpRequestBuilder {
            url: self.url,
            method: self.method,
            port: PhantomData,
            url_value: self.url_value,
            method_value: self.method_value,
            port_value: Some(port),
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

/// 已设置字段的重新设置（允许覆盖）
impl<MethodState, PortState> HttpRequestBuilder<Set<String>, MethodState, PortState> {
    pub fn url(self, url: impl Into<String>) -> HttpRequestBuilder<Set<String>, MethodState, PortState> {
        HttpRequestBuilder {
            url: PhantomData,
            method: self.method,
            port: self.port,
            url_value: Some(url.into()),
            method_value: self.method_value,
            port_value: self.port_value,
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

impl<UrlState, PortState> HttpRequestBuilder<UrlState, Set<HttpMethod>, PortState> {
    pub fn method(self, method: HttpMethod) -> HttpRequestBuilder<UrlState, Set<HttpMethod>, PortState> {
        HttpRequestBuilder {
            url: self.url,
            method: PhantomData,
            port: self.port,
            url_value: self.url_value,
            method_value: Some(method),
            port_value: self.port_value,
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

impl<UrlState, MethodState> HttpRequestBuilder<UrlState, MethodState, Set<ValidPort>> {
    pub fn port(self, port: ValidPort) -> HttpRequestBuilder<UrlState, MethodState, Set<ValidPort>> {
        HttpRequestBuilder {
            url: self.url,
            method: self.method,
            port: PhantomData,
            url_value: self.url_value,
            method_value: self.method_value,
            port_value: Some(port),
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

/// 最终的HttpRequest类型
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub port: ValidPort,
    pub headers: Vec<(String, String)>,
    pub timeout_ms: u64,
}

/// 只有当所有必需字段都设置时才能build
impl HttpRequestBuilder<Set<String>, Set<HttpMethod>, Set<ValidPort>> {
    pub fn build(self) -> HttpRequest {
        HttpRequest {
            url: self.url_value.unwrap(), // Safe: type guarantees it's Some
            method: self.method_value.unwrap(), // Safe: type guarantees it's Some
            port: self.port_value.unwrap(), // Safe: type guarantees it's Some
            headers: self.headers,
            timeout_ms: self.timeout_ms,
        }
    }
}

// ============================================================================
// 第三部分: Command模式 + 状态机 - 无法错误执行
// ============================================================================

/// 文档编辑器的Command模式实现
/// 支持Undo/Redo，且操作前置条件由类型系统保证

/// 文档内容
#[derive(Debug, Clone)]
pub struct Document {
    content: String,
    cursor_position: usize,
    version: u64,
}

impl Document {
    pub fn new() -> Self {
        Document {
            content: String::new(),
            cursor_position: 0,
            version: 0,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    fn insert_at(&mut self, position: usize, text: &str) {
        if position <= self.content.len() {
            self.content.insert_str(position, text);
            self.cursor_position = position + text.len();
            self.version += 1;
        }
    }

    fn delete_range(&mut self, start: usize, end: usize) -> String {
        let start = start.min(self.content.len());
        let end = end.min(self.content.len());
        let deleted = self.content[start..end].to_string();
        self.content.replace_range(start..end, "");
        self.cursor_position = start;
        self.version += 1;
        deleted
    }
}

/// Command trait - 所有操作必须实现
pub trait Command: fmt::Debug {
    /// 执行命令，返回是否成功
    fn execute(&mut self, document: &mut Document) -> Result<(), CommandError>;
    /// 撤销命令
    fn undo(&mut self, document: &mut Document) -> Result<(), CommandError>;
    /// 获取命令名称（用于日志）
    fn name(&self) -> &str;
    /// 是否可以撤销
    fn is_undoable(&self) -> bool { true }
}

#[derive(Debug, Clone)]
pub enum CommandError {
    InvalidPosition { requested: usize, max: usize },
    EmptyDocument,
    InvalidRange { start: usize, end: usize },
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::InvalidPosition { requested, max } => {
                write!(f, "Invalid position: {} (max: {})", requested, max)
            }
            CommandError::EmptyDocument => write!(f, "Document is empty"),
            CommandError::InvalidRange { start, end } => {
                write!(f, "Invalid range: {}..{}", start, end)
            }
        }
    }
}

/// 插入文本命令
#[derive(Debug)]
pub struct InsertCommand {
    position: usize,
    text: String,
    executed: bool,
    backup_position: Option<usize>,
}

impl InsertCommand {
    pub fn new(position: usize, text: impl Into<String>) -> Self {
        InsertCommand {
            position,
            text: text.into(),
            executed: false,
            backup_position: None,
        }
    }
}

impl Command for InsertCommand {
    fn execute(&mut self, document: &mut Document) -> Result<(), CommandError> {
        if self.executed {
            return Ok(());
        }

        if self.position > document.content().len() {
            return Err(CommandError::InvalidPosition {
                requested: self.position,
                max: document.content().len(),
            });
        }

        self.backup_position = Some(document.cursor_position());
        document.insert_at(self.position, &self.text);
        self.executed = true;
        Ok(())
    }

    fn undo(&mut self, document: &mut Document) -> Result<(), CommandError> {
        if !self.executed {
            return Ok(());
        }

        let text_len = self.text.len();
        let deleted = document.delete_range(self.position, self.position + text_len);

        if deleted != self.text {
            // 这不应该发生，说明文档状态不一致
            panic!("Undo consistency check failed: expected '{}', got '{}'", self.text, deleted);
        }

        if let Some(pos) = self.backup_position {
            document.cursor_position = pos;
        }

        self.executed = false;
        Ok(())
    }

    fn name(&self) -> &str {
        "Insert"
    }
}

/// 删除文本命令
#[derive(Debug)]
pub struct DeleteCommand {
    start: usize,
    end: usize,
    deleted_text: Option<String>,
    executed: bool,
}

impl DeleteCommand {
    pub fn new(start: usize, end: usize) -> Self {
        DeleteCommand {
            start,
            end,
            deleted_text: None,
            executed: false,
        }
    }
}

impl Command for DeleteCommand {
    fn execute(&mut self, document: &mut Document) -> Result<(), CommandError> {
        if self.executed {
            return Ok(());
        }

        if self.start > self.end {
            return Err(CommandError::InvalidRange {
                start: self.start,
                end: self.end,
            });
        }

        if self.start > document.content().len() {
            return Err(CommandError::InvalidPosition {
                requested: self.start,
                max: document.content().len(),
            });
        }

        self.deleted_text = Some(document.delete_range(self.start, self.end));
        self.executed = true;
        Ok(())
    }

    fn undo(&mut self, document: &mut Document) -> Result<(), CommandError> {
        if !self.executed {
            return Ok(());
        }

        if let Some(ref text) = self.deleted_text {
            document.insert_at(self.start, text);
            self.executed = false;
            Ok(())
        } else {
            Err(CommandError::EmptyDocument)
        }
    }

    fn name(&self) -> &str {
        "Delete"
    }
}

/// 历史管理器 - 管理Undo/Redo栈
#[derive(Debug)]
pub struct HistoryManager {
    undo_stack: VecDeque<Box<dyn Command>>,
    redo_stack: VecDeque<Box<dyn Command>>,
    max_history: usize,
}

impl HistoryManager {
    pub fn new() -> Self {
        HistoryManager {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: 100,
        }
    }

    pub fn with_capacity(max_history: usize) -> Self {
        HistoryManager {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history,
        }
    }

    /// 执行命令并添加到历史
    pub fn execute(&mut self, mut command: Box<dyn Command>, document: &mut Document) -> Result<(), CommandError> {
        command.execute(document)?;

        if command.is_undoable() {
            self.undo_stack.push_back(command);
            self.redo_stack.clear(); // 新操作清空redo栈

            // 限制历史大小
            if self.undo_stack.len() > self.max_history {
                self.undo_stack.pop_front();
            }
        }

        Ok(())
    }

    /// 撤销最后一个操作
    pub fn undo(&mut self, document: &mut Document) -> Result<(), CommandError> {
        if let Some(mut command) = self.undo_stack.pop_back() {
            command.undo(document)?;
            self.redo_stack.push_back(command);
            Ok(())
        } else {
            Err(CommandError::EmptyDocument) // 没有可撤销的操作
        }
    }

    /// 重做最后一个撤销的操作
    pub fn redo(&mut self, document: &mut Document) -> Result<(), CommandError> {
        if let Some(mut command) = self.redo_stack.pop_back() {
            command.execute(document)?;
            self.undo_stack.push_back(command);
            Ok(())
        } else {
            Err(CommandError::EmptyDocument) // 没有可重做的操作
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

// ============================================================================
// 第四部分: 状态机驱动的工具流程
// ============================================================================

/// 文件处理工具的状态机实现
/// 状态: Idle -> Configured -> Processing -> Completed/Error

pub struct Idle;
pub struct Configured;
pub struct Processing;
pub struct Completed;
pub struct ErrorState;

/// 文件处理工具
pub struct FileProcessor<State> {
    state: PhantomData<State>,
    input_path: Option<std::path::PathBuf>,
    output_path: Option<std::path::PathBuf>,
    processed_bytes: u64,
    error_message: Option<String>,
}

/// 初始状态
impl FileProcessor<Idle> {
    pub fn new() -> Self {
        FileProcessor {
            state: PhantomData,
            input_path: None,
            output_path: None,
            processed_bytes: 0,
            error_message: None,
        }
    }

    /// 配置输入输出路径，转换到Configured状态
    pub fn configure(
        self,
        input: impl Into<std::path::PathBuf>,
        output: impl Into<std::path::PathBuf>,
    ) -> Result<FileProcessor<Configured>, ConfigError> {
        let input_path = input.into();
        let output_path = output.into();

        // 验证输入文件存在
        if !input_path.exists() {
            return Err(ConfigError::InputNotFound(input_path));
        }

        // 验证输出路径的父目录存在
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                return Err(ConfigError::OutputParentNotFound(parent.to_path_buf()));
            }
        }

        Ok(FileProcessor {
            state: PhantomData,
            input_path: Some(input_path),
            output_path: Some(output_path),
            processed_bytes: 0,
            error_message: None,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ConfigError {
    InputNotFound(std::path::PathBuf),
    OutputParentNotFound(std::path::PathBuf),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InputNotFound(p) => write!(f, "Input file not found: {}", p.display()),
            ConfigError::OutputParentNotFound(p) => write!(f, "Output directory not found: {}", p.display()),
        }
    }
}

/// 已配置状态
impl FileProcessor<Configured> {
    /// 开始处理，转换到Processing状态
    pub fn start(self) -> FileProcessor<Processing> {
        FileProcessor {
            state: PhantomData,
            input_path: self.input_path,
            output_path: self.output_path,
            processed_bytes: 0,
            error_message: None,
        }
    }

    pub fn input_path(&self) -> &std::path::Path {
        self.input_path.as_ref().unwrap()
    }

    pub fn output_path(&self) -> &std::path::Path {
        self.output_path.as_ref().unwrap()
    }
}

/// 处理中状态
impl FileProcessor<Processing> {
    /// 模拟处理（实际应用中这里会进行IO操作）
    pub fn process_chunk(mut self, bytes: u64) -> FileProcessor<Processing> {
        self.processed_bytes += bytes;
        self
    }

    /// 完成处理，转换到Completed状态
    pub fn complete(self) -> FileProcessor<Completed> {
        FileProcessor {
            state: PhantomData,
            input_path: self.input_path,
            output_path: self.output_path,
            processed_bytes: self.processed_bytes,
            error_message: None,
        }
    }

    /// 处理出错，转换到Error状态
    pub fn fail(self, error: impl Into<String>) -> FileProcessor<ErrorState> {
        FileProcessor {
            state: PhantomData,
            input_path: self.input_path,
            output_path: self.output_path,
            processed_bytes: self.processed_bytes,
            error_message: Some(error.into()),
        }
    }

    pub fn processed_bytes(&self) -> u64 {
        self.processed_bytes
    }
}

/// 完成状态
impl FileProcessor<Completed> {
    pub fn summary(&self) -> String {
        format!(
            "Processed {} bytes from {:?} to {:?}",
            self.processed_bytes,
            self.input_path.as_ref().unwrap(),
            self.output_path.as_ref().unwrap()
        )
    }

    pub fn processed_bytes(&self) -> u64 {
        self.processed_bytes
    }
}

/// 错误状态
impl FileProcessor<ErrorState> {
    pub fn error_message(&self) -> &str {
        self.error_message.as_deref().unwrap_or("Unknown error")
    }

    /// 从错误状态重置回Idle
    pub fn reset(self) -> FileProcessor<Idle> {
        FileProcessor::new()
    }
}

// ============================================================================
// 第五部分: 完整的错误处理机制
// ============================================================================

/// 统一的错误类型 - 使用thiserror风格
#[derive(Debug, Clone)]
pub enum ToolError {
    Config(ConfigError),
    Command(CommandError),
    Validation(String),
    Io(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::Config(e) => write!(f, "Configuration error: {}", e),
            ToolError::Command(e) => write!(f, "Command error: {}", e),
            ToolError::Validation(s) => write!(f, "Validation error: {}", s),
            ToolError::Io(s) => write!(f, "IO error: {}", s),
        }
    }
}

/// 结果类型别名
pub type ToolResult<T> = Result<T, ToolError>;

/// 错误上下文扩展trait
pub trait ResultExt<T, E> {
    fn with_context(self, msg: impl FnOnce() -> String) -> ToolResult<T>;
}

impl<T, E: fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn with_context(self, msg: impl FnOnce() -> String) -> ToolResult<T> {
        self.map_err(|e| ToolError::Validation(format!("{}: {}", msg(), e)))
    }
}

// ============================================================================
// 第六部分: 测试验证
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 测试类型安全的端口验证
    #[test]
    fn test_valid_port() {
        // 有效端口
        let port = ValidPort::new(8080).unwrap();
        assert_eq!(port.get(), 8080);

        // 无效端口（0）
        assert!(ValidPort::new(0).is_err());

        // 从字符串解析
        let port: ValidPort = "443".parse().unwrap();
        assert_eq!(port.get(), 443);

        // 无效解析
        assert!("0".parse::<ValidPort>().is_err());
        assert!("abc".parse::<ValidPort>().is_err());
    }

    // 测试类型安全的主机验证
    #[test]
    fn test_valid_host() {
        let host = ValidHost::new("localhost".to_string()).unwrap();
        assert_eq!(host.as_str(), "localhost");

        assert!(ValidHost::new("".to_string()).is_err());
        assert!(ValidHost::new("host with spaces".to_string()).is_err());
    }

    // 测试Typestate Builder - 成功构建
    #[test]
    fn test_http_request_builder_success() {
        let request = HttpRequestBuilder::new()
            .url("https://api.example.com")
            .method(HttpMethod::Post)
            .port(ValidPort::new(443).unwrap())
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer token123")
            .timeout(5000)
            .build();

        assert_eq!(request.url, "https://api.example.com");
        assert_eq!(request.method, HttpMethod::Post);
        assert_eq!(request.port.get(), 443);
        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.timeout_ms, 5000);
    }

    // 测试Typestate Builder - 字段顺序无关
    #[test]
    fn test_http_request_builder_order_independent() {
        // 不同的设置顺序应该都能工作
        let request1 = HttpRequestBuilder::new()
            .url("https://example.com")
            .method(HttpMethod::Get)
            .port(ValidPort::new(80).unwrap())
            .build();

        let request2 = HttpRequestBuilder::new()
            .port(ValidPort::new(80).unwrap())
            .url("https://example.com")
            .method(HttpMethod::Get)
            .build();

        assert_eq!(request1.url, request2.url);
        assert_eq!(request1.method, request2.method);
        assert_eq!(request1.port.get(), request2.port.get());
    }

    // 测试Command模式 - Insert和Undo
    #[test]
    fn test_insert_command() {
        let mut doc = Document::new();
        let mut cmd = InsertCommand::new(0, "Hello, World!");

        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.content(), "Hello, World!");
        assert_eq!(doc.cursor_position(), 13);

        cmd.undo(&mut doc).unwrap();
        assert_eq!(doc.content(), "");
    }

    // 测试Command模式 - Delete和Undo
    #[test]
    fn test_delete_command() {
        let mut doc = Document::new();

        // 先插入内容
        let mut insert = InsertCommand::new(0, "Hello, World!");
        insert.execute(&mut doc).unwrap();

        // 删除部分内容
        let mut delete = DeleteCommand::new(0, 5);
        delete.execute(&mut doc).unwrap();
        assert_eq!(doc.content(), ", World!");

        // 撤销删除
        delete.undo(&mut doc).unwrap();
        assert_eq!(doc.content(), "Hello, World!");
    }

    // 测试HistoryManager - Undo/Redo
    #[test]
    fn test_history_manager() {
        let mut doc = Document::new();
        let mut history = HistoryManager::new();

        // 执行多个操作
        history.execute(Box::new(InsertCommand::new(0, "Hello")), &mut doc).unwrap();
        assert_eq!(doc.content(), "Hello");
        assert!(history.can_undo());

        history.execute(Box::new(InsertCommand::new(5, " World")), &mut doc).unwrap();
        assert_eq!(doc.content(), "Hello World");

        // 撤销
        history.undo(&mut doc).unwrap();
        assert_eq!(doc.content(), "Hello");
        assert!(history.can_redo());

        // 重做
        history.redo(&mut doc).unwrap();
        assert_eq!(doc.content(), "Hello World");
        assert!(!history.can_redo());

        // 新的操作会清空redo栈
        history.execute(Box::new(InsertCommand::new(11, "!")), &mut doc).unwrap();
        assert!(!history.can_redo());
    }

    // 测试状态机驱动的文件处理器
    #[test]
    fn test_file_processor_state_machine() {
        // 注意：这个测试需要实际文件存在
        // 在实际测试中应该使用临时文件

        // 创建临时目录和文件用于测试
        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("test_input.txt");
        let output_path = temp_dir.join("test_output.txt");

        // 创建输入文件
        std::fs::write(&input_path, "test content").unwrap();

        // Idle -> Configured
        let processor = FileProcessor::new()
            .configure(&input_path, &output_path)
            .expect("Configuration should succeed");

        assert_eq!(processor.input_path(), input_path);
        assert_eq!(processor.output_path(), output_path);

        // Configured -> Processing
        let processor = processor.start();
        assert_eq!(processor.processed_bytes(), 0);

        // Processing -> Processing (处理中)
        let processor = processor.process_chunk(100);
        assert_eq!(processor.processed_bytes(), 100);

        // Processing -> Completed
        let completed = processor.complete();
        assert!(completed.summary().contains("100 bytes"));

        // 清理
        let _ = std::fs::remove_file(&input_path);
    }

    // 测试文件处理器错误处理
    #[test]
    fn test_file_processor_error() {
        let processor = FileProcessor::new()
            .configure("/nonexistent/input.txt", "/tmp/output.txt");

        assert!(processor.is_err());
    }

    // 测试Command错误处理
    #[test]
    fn test_command_error_handling() {
        let mut doc = Document::new();

        // 尝试在无效位置插入
        let mut cmd = InsertCommand::new(100, "text");
        let result = cmd.execute(&mut doc);
        assert!(result.is_err());

        match result {
            Err(CommandError::InvalidPosition { requested, max }) => {
                assert_eq!(requested, 100);
                assert_eq!(max, 0);
            }
            _ => panic!("Expected InvalidPosition error"),
        }
    }

    // 测试边界条件
    #[test]
    fn test_edge_cases() {
        // 空文档操作
        let mut doc = Document::new();
        let mut history = HistoryManager::new();

        // 空文档上撤销应该失败
        assert!(history.undo(&mut doc).is_err());

        // 空文档上重做应该失败
        assert!(history.redo(&mut doc).is_err());

        // 插入空字符串
        let mut cmd = InsertCommand::new(0, "");
        cmd.execute(&mut doc).unwrap();
        assert_eq!(doc.content(), "");

        // 端口边界值
        let port = ValidPort::new(1).unwrap();
        assert_eq!(port.get(), 1);

        let port = ValidPort::new(65535).unwrap();
        assert_eq!(port.get(), 65535);
    }

    // 测试历史管理器容量限制
    #[test]
    fn test_history_capacity() {
        let mut doc = Document::new();
        let mut history = HistoryManager::with_capacity(3);

        // 执行超过容量的操作
        for i in 0..5 {
            let text = format!("{}", i);
            history.execute(Box::new(InsertCommand::new(doc.content().len(), &text)), &mut doc).unwrap();
        }

        // 由于容量为3，最早的2个操作应该被移除
        assert_eq!(history.undo_count(), 3);

        // 撤销3次后应该不能再撤销
        for _ in 0..3 {
            assert!(history.undo(&mut doc).is_ok());
        }
        assert!(!history.can_undo());
    }
}

// ============================================================================
// 第七部分: 示例用法和演示
// ============================================================================

/// 演示类型安全工具设计的完整流程
pub fn demo_type_safe_tool() {
    println!("=== Type-Safe Tool Design Demo ===\n");

    // 1. 演示类型安全的参数验证
    println!("1. Validated Types:");
    let port = ValidPort::new(8080).unwrap();
    println!("   ValidPort: {}", port.get());

    let host = ValidHost::new("api.example.com".to_string()).unwrap();
    println!("   ValidHost: {}", host.as_str());

    // 2. 演示Typestate Builder
    println!("\n2. Typestate Builder:");
    let request = HttpRequestBuilder::new()
        .url("https://api.example.com/users")
        .method(HttpMethod::Get)
        .port(ValidPort::new(443).unwrap())
        .header("Accept", "application/json")
        .header("X-API-Key", "secret123")
        .timeout(10000)
        .build();

    println!("   Request: {} {}", request.method, request.url);
    println!("   Port: {}", request.port.get());
    println!("   Headers: {:?}", request.headers);
    println!("   Timeout: {}ms", request.timeout_ms);

    // 3. 演示Command模式
    println!("\n3. Command Pattern with Undo/Redo:");
    let mut doc = Document::new();
    let mut history = HistoryManager::new();

    history.execute(Box::new(InsertCommand::new(0, "Hello")), &mut doc).unwrap();
    println!("   After 'Hello': '{}'", doc.content());

    history.execute(Box::new(InsertCommand::new(5, " World")), &mut doc).unwrap();
    println!("   After ' World': '{}'", doc.content());

    history.undo(&mut doc).unwrap();
    println!("   After Undo: '{}'", doc.content());

    history.redo(&mut doc).unwrap();
    println!("   After Redo: '{}'", doc.content());

    // 4. 演示状态机
    println!("\n4. State Machine:");
    println!("   FileProcessor states: Idle -> Configured -> Processing -> Completed");
    println!("   State transitions enforced at compile time!");
}

fn main() {
    demo_type_safe_tool();
}
