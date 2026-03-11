// 深度研究：无法产生错误的工具设计
// 研究方向: 10_tool_design
// 时间: 2026-03-11

//! # 无法产生错误的工具设计
//!
//! 本代码展示了如何通过Rust类型系统使工具"无法产生错误"。
//! 核心思想：让非法状态在编译期就不可表示。

use std::path::PathBuf;
use std::fmt;

// ============================================================================
// 假设1: Newtype模式 - 区分语义不同的同类型值
// ============================================================================

/// 用户ID - 使用Newtype模式防止与订单ID混淆
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UserId(u64);

/// 订单ID - 即使底层类型相同，也无法与用户ID互换使用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OrderId(u64);

/// 产品ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ProductId(u64);

impl UserId {
    fn new(id: u64) -> Self {
        // 可以在构造函数中添加验证逻辑
        assert!(id > 0, "UserId must be positive");
        Self(id)
    }

    fn value(&self) -> u64 {
        self.0
    }
}

impl OrderId {
    fn new(id: u64) -> Self {
        assert!(id > 0, "OrderId must be positive");
        Self(id)
    }

    fn value(&self) -> u64 {
        self.0
    }
}

/// 演示Newtype模式如何防止错误
fn process_order(user_id: UserId, order_id: OrderId) {
    println!("Processing order {} for user {}", order_id.value(), user_id.value());
}

// 以下代码在编译期就会报错，防止将订单ID误作用户ID：
// process_order(OrderId(1), UserId(2)); // 编译错误！

// ============================================================================
// 假设2: Typestate模式 - 用类型系统建模状态机
// ============================================================================

/// 文件状态：未打开
struct Closed;
/// 文件状态：已打开可读
struct OpenForRead;
/// 文件状态：已打开可写
struct OpenForWrite;

/// 类型安全的文件句柄
///
/// State是一个泛型参数，表示当前文件的编译期状态。
/// 只有特定状态下才能调用特定方法。
struct TypedFile<State> {
    path: PathBuf,
    _state: std::marker::PhantomData<State>,
}

impl TypedFile<Closed> {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            _state: std::marker::PhantomData,
        }
    }

    /// 打开文件用于读取 - 状态从Closed转换为OpenForRead
    fn open_for_read(self) -> Result<TypedFile<OpenForRead>, std::io::Error> {
        // 实际打开文件的逻辑...
        println!("Opening {:?} for reading", self.path);
        Ok(TypedFile {
            path: self.path,
            _state: std::marker::PhantomData,
        })
    }

    /// 打开文件用于写入 - 状态从Closed转换为OpenForWrite
    fn open_for_write(self) -> Result<TypedFile<OpenForWrite>, std::io::Error> {
        println!("Opening {:?} for writing", self.path);
        Ok(TypedFile {
            path: self.path,
            _state: std::marker::PhantomData,
        })
    }
}

impl TypedFile<OpenForRead> {
    /// 只有在OpenForRead状态下才能读取
    fn read(&self) -> Vec<u8> {
        println!("Reading from {:?}", self.path);
        vec![] // 模拟读取
    }

    /// 关闭文件 - 返回Closed状态
    fn close(self) -> TypedFile<Closed> {
        println!("Closing file");
        TypedFile {
            path: self.path,
            _state: std::marker::PhantomData,
        }
    }
}

impl TypedFile<OpenForWrite> {
    /// 只有在OpenForWrite状态下才能写入
    fn write(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        println!("Writing {} bytes to {:?}", data.len(), self.path);
        Ok(())
    }

    /// 关闭文件
    fn close(self) -> TypedFile<Closed> {
        println!("Closing file");
        TypedFile {
            path: self.path,
            _state: std::marker::PhantomData,
        }
    }
}

// 以下代码在编译期就会报错：
// let file = TypedFile::<Closed>::new("test.txt");
// file.read(); // 编译错误！Closed状态没有read方法

// ============================================================================
// 假设3: 使用枚举而非布尔值 - 显式表示所有状态
// ============================================================================

/// 使用枚举而非bool表示日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// 使用枚举而非bool表示输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Plain,
    Json,
    Yaml,
    Table,
}

/// 类型安全的配置结构
#[derive(Debug, Clone)]
struct ToolConfig {
    log_level: LogLevel,
    output_format: OutputFormat,
    dry_run: bool,
    max_retries: u32,
}

impl ToolConfig {
    fn new() -> Self {
        Self {
            log_level: LogLevel::Info,
            output_format: OutputFormat::Plain,
            dry_run: false,
            max_retries: 3,
        }
    }

    fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    fn with_output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

// ============================================================================
// 假设4: 边界验证 - 在入口点立即验证所有输入
// ============================================================================

/// 验证后的非空字符串
#[derive(Debug, Clone)]
struct NonEmptyString(String);

impl NonEmptyString {
    fn new(s: String) -> Result<Self, ValidationError> {
        if s.trim().is_empty() {
            Err(ValidationError::new("String cannot be empty"))
        } else {
            Ok(Self(s))
        }
    }

    fn value(&self) -> &str {
        &self.0
    }
}

/// 验证后的正整数
#[derive(Debug, Clone, Copy)]
struct PositiveInt(u32);

impl PositiveInt {
    fn new(n: u32) -> Result<Self, ValidationError> {
        if n == 0 {
            Err(ValidationError::new("Value must be positive"))
        } else {
            Ok(Self(n))
        }
    }

    fn value(&self) -> u32 {
        self.0
    }
}

/// 验证错误
#[derive(Debug, Clone)]
struct ValidationError {
    message: String,
}

impl ValidationError {
    fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error: {}", self.message)
    }
}

impl std::error::Error for ValidationError {}

/// 命令参数 - 所有字段都是已验证的类型
#[derive(Debug, Clone)]
struct CreateUserCommand {
    username: NonEmptyString,
    email: NonEmptyString,
    age: PositiveInt,
}

impl CreateUserCommand {
    /// 构造函数执行所有验证
    fn new(
        username: String,
        email: String,
        age: u32,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            username: NonEmptyString::new(username)?,
            email: NonEmptyString::new(email)?,
            age: PositiveInt::new(age)?,
        })
    }

    /// 执行业务逻辑 - 无需再次验证输入
    fn execute(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Creating user: {}", self.username.value());
        println!("Email: {}", self.email.value());
        println!("Age: {}", self.age.value());
        // 这里可以安全地使用所有字段，因为它们已经过验证
        Ok(())
    }
}

// ============================================================================
// 假设5: 类型安全的构建器模式
// ============================================================================

/// 未初始化状态标记
struct Uninitialized;
/// 已初始化状态标记
struct Initialized;

/// 类型安全的HTTP请求构建器
///
/// 使用泛型参数确保只有设置了所有必需字段后才能构建
struct HttpRequestBuilder<UrlState, MethodState> {
    url: Option<String>,
    method: Option<String>,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    _url_state: std::marker::PhantomData<UrlState>,
    _method_state: std::marker::PhantomData<MethodState>,
}

impl HttpRequestBuilder<Uninitialized, Uninitialized> {
    fn new() -> Self {
        Self {
            url: None,
            method: None,
            headers: Vec::new(),
            body: None,
            _url_state: std::marker::PhantomData,
            _method_state: std::marker::PhantomData,
        }
    }
}

impl<MethodState> HttpRequestBuilder<Uninitialized, MethodState> {
    /// 设置URL - 将UrlState从Uninitialized转换为Initialized
    fn url(self, url: impl Into<String>) -> HttpRequestBuilder<Initialized, MethodState> {
        HttpRequestBuilder {
            url: Some(url.into()),
            method: self.method,
            headers: self.headers,
            body: self.body,
            _url_state: std::marker::PhantomData,
            _method_state: std::marker::PhantomData,
        }
    }
}

impl<UrlState> HttpRequestBuilder<UrlState, Uninitialized> {
    /// 设置方法 - 将MethodState从Uninitialized转换为Initialized
    fn method(self, method: impl Into<String>) -> HttpRequestBuilder<UrlState, Initialized> {
        HttpRequestBuilder {
            url: self.url,
            method: Some(method.into()),
            headers: self.headers,
            body: self.body,
            _url_state: std::marker::PhantomData,
            _method_state: std::marker::PhantomData,
        }
    }
}

impl<UrlState, MethodState> HttpRequestBuilder<UrlState, MethodState> {
    fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// 只有UrlState和MethodState都是Initialized时才能构建
impl HttpRequestBuilder<Initialized, Initialized> {
    fn build(self) -> HttpRequest {
        HttpRequest {
            url: self.url.unwrap(),
            method: self.method.unwrap(),
            headers: self.headers,
            body: self.body,
        }
    }
}

#[derive(Debug, Clone)]
struct HttpRequest {
    url: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

// 以下代码在编译期就会报错：
// let request = HttpRequestBuilder::new().build(); // 编译错误！缺少url和method
// let request = HttpRequestBuilder::new().url("https://example.com").build(); // 编译错误！缺少method

// ============================================================================
// 假设6: 错误类型作为类型系统的一部分
// ============================================================================

/// 定义所有可能的错误类型
#[derive(Debug, Clone)]
enum ToolError {
    Validation(ValidationError),
    Io(String),
    Network(String),
    NotFound { resource: String, id: String },
    PermissionDenied { resource: String, action: String },
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::Validation(e) => write!(f, "{}", e),
            ToolError::Io(msg) => write!(f, "IO error: {}", msg),
            ToolError::Network(msg) => write!(f, "Network error: {}", msg),
            ToolError::NotFound { resource, id } => {
                write!(f, "{} with id '{}' not found", resource, id)
            }
            ToolError::PermissionDenied { resource, action } => {
                write!(f, "Permission denied: cannot {} {}", action, resource)
            }
        }
    }
}

impl std::error::Error for ToolError {}

impl From<ValidationError> for ToolError {
    fn from(e: ValidationError) -> Self {
        ToolError::Validation(e)
    }
}

// ============================================================================
// 主函数：演示所有假设的验证
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 无法产生错误的工具设计 - 验证演示 ===\n");

    // 验证假设1: Newtype模式
    println!("--- 假设1: Newtype模式 ---");
    let user_id = UserId::new(42);
    let order_id = OrderId::new(1001);
    process_order(user_id, order_id);
    // process_order(order_id, user_id); // 编译错误！类型不匹配
    println!();

    // 验证假设2: Typestate模式
    println!("--- 假设2: Typestate模式 ---");
    let file = TypedFile::<Closed>::new("data.txt");
    let file = file.open_for_read()?;
    let _data = file.read();
    // file.write(b"test"); // 编译错误！OpenForRead状态没有write方法
    let file = file.close();
    println!();

    // 验证假设3: 使用枚举而非布尔值
    println!("--- 假设3: 显式枚举 ---");
    let config = ToolConfig::new()
        .with_log_level(LogLevel::Debug)
        .with_output_format(OutputFormat::Json)
        .with_dry_run(true);
    println!("Config: {:?}\n", config);

    // 验证假设4: 边界验证
    println!("--- 假设4: 边界验证 ---");
    let cmd = CreateUserCommand::new(
        "alice".to_string(),
        "alice@example.com".to_string(),
        25,
    )?;
    cmd.execute()?;

    // 验证错误处理
    match CreateUserCommand::new("".to_string(), "test@example.com".to_string(), 25) {
        Err(e) => println!("Expected validation error: {}\n", e),
        Ok(_) => println!("Unexpected success\n"),
    }

    // 验证假设5: 类型安全构建器
    println!("--- 假设5: 类型安全构建器 ---");
    let request = HttpRequestBuilder::new()
        .url("https://api.example.com/users")
        .method("GET")
        .header("Authorization", "Bearer token123")
        .header("Content-Type", "application/json")
        .build();
    println!("Request: {:?}\n", request);

    // 以下代码编译错误：
    // let incomplete = HttpRequestBuilder::new().build();

    println!("=== 所有假设验证完成 ===");
    println!("\n核心发现:");
    println!("1. Newtype模式在编译期防止ID混淆 - 零运行时开销");
    println!("2. Typestate模式确保正确的操作序列 - 编译期状态机");
    println!("3. 枚举替代布尔值使所有状态显式 - 自文档化代码");
    println!("4. 边界验证在入口点完成 - 内部逻辑无需重复检查");
    println!("5. 类型安全构建器防止不完整对象 - 编译期强制完整性");

    Ok(())
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newtype_prevents_confusion() {
        let user_id = UserId::new(1);
        let order_id = OrderId::new(2);

        // 这两个类型不能互换使用
        // 以下代码编译错误：
        // let wrong: UserId = order_id;

        assert_eq!(user_id.value(), 1);
        assert_eq!(order_id.value(), 2);
    }

    #[test]
    fn test_typestate_enforces_correct_usage() {
        let file = TypedFile::<Closed>::new("test.txt");
        let file = file.open_for_read().unwrap();
        let _data = file.read();
        let _file = file.close();

        // 无法对已关闭的文件进行操作
        // _file.read(); // 编译错误！
    }

    #[test]
    fn test_validation_at_boundary() {
        // 有效输入
        let cmd = CreateUserCommand::new(
            "alice".to_string(),
            "alice@example.com".to_string(),
            25,
        );
        assert!(cmd.is_ok());

        // 无效输入 - 空用户名
        let cmd = CreateUserCommand::new(
            "".to_string(),
            "alice@example.com".to_string(),
            25,
        );
        assert!(cmd.is_err());

        // 无效输入 - 年龄为0
        let cmd = CreateUserCommand::new(
            "alice".to_string(),
            "alice@example.com".to_string(),
            0,
        );
        assert!(cmd.is_err());
    }

    #[test]
    fn test_builder_requires_all_fields() {
        // 完整的构建器可以构建
        let request = HttpRequestBuilder::new()
            .url("https://example.com")
            .method("GET")
            .build();
        assert_eq!(request.url, "https://example.com");
        assert_eq!(request.method, "GET");

        // 以下代码编译错误：
        // let incomplete = HttpRequestBuilder::new().build();
    }
}
