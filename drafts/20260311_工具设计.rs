//! 工具设计验证 - 无法产生错误的工具设计
//!
//! 研究方向: 10_tool_design - 工具设计
//! 研究日期: 2026-03-11
//! 核心问题: 如何设计'无法产生错误'的工具?
//!
//! 本代码验证以下假设：
//! H1: Typestate模式通过将状态编码为类型，使非法状态转换在编译期被拒绝
//! H2: 使用PhantomData的零成本抽象在编译期完全擦除，无运行时开销
//! H3: "解析-验证-执行"三阶段管道可通过类型转换强制正确执行顺序
//! H4: Newtype模式可防止语义不同但底层类型相同的值被错误互换
//!
//! 设计决策说明:
//! 1. 使用泛型状态参数编码Typestate，PhantomData标记状态
//! 2. Newtype模式包装原始类型，防止语义混淆
//! 3. 解析-验证-执行三阶段通过类型转换强制执行顺序
//! 4. 所有验证在边界进行，内部代码无需重复检查

use std::marker::PhantomData;
use std::fmt;

// ============================================
// H4验证: Newtype模式防止语义混淆
// ============================================

/// 用户ID - 包装u64以防止与其他ID混淆
/// 设计决策: 即使底层类型相同(u64)，语义不同的ID应使用不同类型
/// 这防止了将UserId当作OrderId传递的错误
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(u64);

/// 订单ID - 同样包装u64，但类型不同
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(u64);

/// 产品ID - 第三种u64包装
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductId(u64);

impl UserId {
    /// 创建UserId，验证id不为0
    /// 设计决策: 构造函数进行验证，确保无效状态无法创建
    pub fn new(id: u64) -> Self {
        assert!(id != 0, "UserId cannot be zero");
        Self(id)
    }

    /// 获取底层u64值
    /// 设计决策: 提供访问器而非直接暴露字段，控制访问权限
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl OrderId {
    pub fn new(id: u64) -> Self {
        assert!(id != 0, "OrderId cannot be zero");
        Self(id)
    }
}

impl ProductId {
    pub fn new(id: u64) -> Self {
        assert!(id != 0, "ProductId cannot be zero");
        Self(id)
    }
}

/// 处理用户订单
/// 设计决策: 参数类型明确区分UserId和OrderId
/// 编译器会拒绝任何尝试交换这两个参数的操作
fn process_user_order(user_id: UserId, order_id: OrderId) -> String {
    format!("Processing order {} for user {}", order_id.0, user_id.as_u64())
}

// ============================================
// H1+H2验证: Typestate模式 - 文件处理器
// ============================================

/// 未配置状态标记类型
pub struct Unconfigured;
/// 已配置但未打开状态标记类型
pub struct Configured;
/// 已打开可读状态标记类型
pub struct OpenForRead;
/// 已打开可写状态标记类型
pub struct OpenForWrite;
/// 已关闭状态标记类型
pub struct Closed;

/// Typestate文件处理器
/// State泛型参数编码当前状态
/// PhantomData<State>确保零运行时开销 - 编译期完全擦除
///
/// 设计决策:
/// - 使用泛型参数State编码当前状态
/// - PhantomData<State>是零大小类型(ZST)，无运行时开销
/// - 每个状态的方法只在对应状态下可用
pub struct FileProcessor<State> {
    path: String,
    content: Option<String>,
    _state: PhantomData<State>,
}

// 实现状态转换
impl FileProcessor<Unconfigured> {
    /// 创建新的未配置文件处理器
    pub fn new() -> Self {
        Self {
            path: String::new(),
            content: None,
            _state: PhantomData,
        }
    }

    /// 配置路径 - 转换到Configured状态
    /// 设计决策: 消耗self，返回新状态，确保旧状态不再可用
    pub fn with_path(self, path: &str) -> FileProcessor<Configured> {
        FileProcessor {
            path: path.to_string(),
            content: None,
            _state: PhantomData,
        }
    }
}

impl FileProcessor<Configured> {
    /// 打开读取 - 转换到OpenForRead状态
    pub fn open_read(self) -> FileProcessor<OpenForRead> {
        FileProcessor {
            path: self.path,
            content: Some(String::new()),
            _state: PhantomData,
        }
    }

    /// 打开写入 - 转换到OpenForWrite状态
    pub fn open_write(self) -> FileProcessor<OpenForWrite> {
        FileProcessor {
            path: self.path,
            content: None,
            _state: PhantomData,
        }
    }
}

impl FileProcessor<OpenForRead> {
    /// 只能在OpenForRead状态下读取
    /// 设计决策: read方法只在此impl块中定义，其他状态无法调用
    pub fn read(&mut self) -> &str {
        self.content.get_or_insert_with(|| "file content".to_string())
    }

    /// 关闭 - 转换到Closed状态
    pub fn close(self) -> FileProcessor<Closed> {
        FileProcessor {
            path: self.path,
            content: None,
            _state: PhantomData,
        }
    }
}

impl FileProcessor<OpenForWrite> {
    /// 只能在OpenForWrite状态下写入
    /// 设计决策: write方法只在此impl块中定义，OpenForRead状态无法调用
    pub fn write(&mut self, content: &str) {
        println!("Writing: {}", content);
    }

    /// 关闭
    pub fn close(self) -> FileProcessor<Closed> {
        FileProcessor {
            path: self.path,
            content: None,
            _state: PhantomData,
        }
    }
}

impl FileProcessor<Closed> {
    /// 只能在Closed状态下重新配置
    pub fn reconfigure(self, path: &str) -> FileProcessor<Configured> {
        FileProcessor {
            path: path.to_string(),
            content: None,
            _state: PhantomData,
        }
    }
}

// ============================================
// H3验证: "解析-验证-执行"三阶段管道
// ============================================

/// 原始输入（未验证）
/// 设计决策: 明确标记为"原始"，提示需要验证
pub struct RawInput {
    data: String,
}

/// 已验证输入
/// 设计决策: 只能通过validate函数创建，确保验证逻辑执行
pub struct ValidatedInput {
    data: String,
    checksum: u32,
}

/// 执行结果
pub struct ExecutionResult {
    output: String,
}

/// 解析阶段 -> 验证阶段
/// 设计决策: parse函数返回RawInput，不能直接执行
pub fn parse(input: &str) -> RawInput {
    RawInput {
        data: input.to_string(),
    }
}

/// 验证阶段 -> 执行阶段
/// 只有通过验证才能调用此函数
/// 设计决策: validate函数是RawInput转换为ValidatedInput的唯一途径
pub fn validate(raw: RawInput) -> Result<ValidatedInput, ValidationError> {
    if raw.data.is_empty() {
        return Err(ValidationError::EmptyInput);
    }

    // 计算简单校验和
    let checksum = raw.data.bytes().map(|b| b as u32).sum();

    Ok(ValidatedInput {
        data: raw.data,
        checksum,
    })
}

/// 执行阶段 - 只能使用已验证输入
/// 设计决策: execute函数只接受ValidatedInput，不接受RawInput
/// 这强制了必须先验证再执行的顺序
pub fn execute(valid: ValidatedInput) -> ExecutionResult {
    // 此处无需再次验证，类型系统保证输入已验证
    ExecutionResult {
        output: format!("Processed: {} (checksum: {})", valid.data, valid.checksum),
    }
}

#[derive(Debug)]
pub enum ValidationError {
    EmptyInput,
    InvalidChecksum,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::EmptyInput => write!(f, "Input cannot be empty"),
            ValidationError::InvalidChecksum => write!(f, "Checksum validation failed"),
        }
    }
}

// ============================================
// 类型安全CLI参数验证
// ============================================

/// 验证端口号（1-65535）
/// 设计决策: 在类型层面保证端口号有效
#[derive(Debug, Clone, Copy)]
pub struct ValidPort(u16);

impl ValidPort {
    /// 尝试创建ValidPort，验证端口号不为0
    pub fn new(port: u16) -> Result<Self, String> {
        if port == 0 {
            Err("Port cannot be 0".to_string())
        } else {
            Ok(Self(port))
        }
    }

    pub fn get(&self) -> u16 {
        self.0
    }
}

/// 验证主机名
#[derive(Debug, Clone)]
pub struct ValidHost(String);

impl ValidHost {
    /// 尝试创建ValidHost，验证主机名非空且只包含合法字符
    pub fn new(host: &str) -> Result<Self, String> {
        if host.is_empty() {
            Err("Host cannot be empty".to_string())
        } else if !host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            Err("Host contains invalid characters".to_string())
        } else {
            Ok(Self(host.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 连接配置 - 只能使用验证过的端口和主机
/// 设计决策: ConnectionConfig::new只接受ValidHost和ValidPort
/// 这确保了无法使用未验证的参数创建配置
pub struct ConnectionConfig {
    host: ValidHost,
    port: ValidPort,
}

impl ConnectionConfig {
    pub fn new(host: ValidHost, port: ValidPort) -> Self {
        Self { host, port }
    }

    /// 连接 - 内部无需再次验证
    /// 设计决策: 由于构造时已经验证，此处直接使用
    pub fn connect(&self) -> String {
        format!("Connecting to {}:{}", self.host.as_str(), self.port.get())
    }
}

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newtype_prevents_mixing() {
        let user_id = UserId::new(1);
        let order_id = OrderId::new(100);

        // 此调用编译通过，因为类型正确
        let result = process_user_order(user_id, order_id);
        assert!(result.contains("1"));
        assert!(result.contains("100"));

        // 以下代码会导致编译错误：
        // process_user_order(order_id, user_id); // 错误！类型不匹配
    }

    #[test]
    fn test_typestate_file_processor() {
        // 正确的状态转换链
        let processor = FileProcessor::new()
            .with_path("/tmp/test.txt")
            .open_read();

        // processor现在处于OpenForRead状态，可以读取
        // 但不能写入！以下代码会导致编译错误：
        // processor.write("data"); // 错误！OpenForRead没有write方法

        let _closed = processor.close();
    }

    #[test]
    fn test_parse_validate_execute_pipeline() {
        let raw = parse("Hello, World!");
        let validated = validate(raw).expect("Validation should succeed");
        let result = execute(validated);

        assert!(result.output.contains("Hello, World!"));
    }

    #[test]
    fn test_validation_rejects_empty() {
        let raw = parse("");
        let result = validate(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_port() {
        assert!(ValidPort::new(0).is_err());
        assert!(ValidPort::new(80).is_ok());
        assert!(ValidPort::new(65535).is_ok());
    }

    #[test]
    fn test_valid_host() {
        assert!(ValidHost::new("").is_err());
        assert!(ValidHost::new("localhost").is_ok());
        assert!(ValidHost::new("example.com").is_ok());
    }

    #[test]
    fn test_connection_config() {
        let host = ValidHost::new("localhost").unwrap();
        let port = ValidPort::new(8080).unwrap();
        let config = ConnectionConfig::new(host, port);

        assert_eq!(config.connect(), "Connecting to localhost:8080");
    }
}
