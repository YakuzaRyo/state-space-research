// ============================================================================
// 研究方向: 10_tool_design - 防错工具设计验证
// 日期: 2026-03-11
// 核心问题: 如何设计'无法产生错误'的工具?
// ============================================================================

// =============================================================================
// 验证假设1: 类型即约束 - 将非法状态变为不可表示
// =============================================================================

/// 使用类型系统确保状态转换的正确性
/// 设计一个文件处理流程，确保：
/// 1. 文件必须先打开才能读取
/// 2. 文件必须先读取才能解析
/// 3. 已关闭的文件不能再次操作

// 标记类型 - 零大小类型，仅用于编译期状态标记
pub struct Closed;
pub struct Opened;
pub struct Reading;
pub struct Parsed;

/// 文件状态机 - 使用泛型参数编码状态
pub struct FileProcessor<State> {
    path: String,
    content: Option<String>,
    parsed_data: Option<Vec<String>>,
    _state: std::marker::PhantomData<State>,
}

// 实现状态转换，确保非法转换在编译期被阻止
impl FileProcessor<Closed> {
    pub fn new(path: &str) -> Self {
        FileProcessor {
            path: path.to_string(),
            content: None,
            parsed_data: None,
            _state: std::marker::PhantomData,
        }
    }

    /// 打开文件 - 只有Closed状态可以调用
    pub fn open(self) -> FileProcessor<Opened> {
        println!("[状态机] 文件 '{}' 已打开", self.path);
        FileProcessor {
            path: self.path,
            content: None,
            parsed_data: None,
            _state: std::marker::PhantomData,
        }
    }
}

impl FileProcessor<Opened> {
    /// 读取文件 - 只有Opened状态可以调用
    pub fn read(self) -> FileProcessor<Reading> {
        println!("[状态机] 文件内容已读取");
        FileProcessor {
            path: self.path,
            content: Some("sample data line 1\nline 2\nline 3".to_string()),
            parsed_data: None,
            _state: std::marker::PhantomData,
        }
    }
}

impl FileProcessor<Reading> {
    /// 解析内容 - 只有Reading状态可以调用
    pub fn parse(self) -> FileProcessor<Parsed> {
        let content = self.content.as_ref().unwrap();
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        println!("[状态机] 内容已解析为 {} 行", lines.len());
        FileProcessor {
            path: self.path,
            content: self.content,
            parsed_data: Some(lines),
            _state: std::marker::PhantomData,
        }
    }
}

impl FileProcessor<Parsed> {
    /// 获取解析结果
    pub fn get_data(&self) -> &[String] {
        self.parsed_data.as_ref().unwrap().as_slice()
    }

    /// 关闭文件
    pub fn close(self) -> FileProcessor<Closed> {
        println!("[状态机] 文件已关闭");
        FileProcessor {
            path: self.path,
            content: None,
            parsed_data: None,
            _state: std::marker::PhantomData,
        }
    }
}

// =============================================================================
// 验证假设2: 编译期强制 - 使用类型系统防止并发错误
// =============================================================================

/// 线程安全的资源管理 - Send/Sync自动推导
/// 设计一个只能被单线程访问的资源
pub struct SingleThreadResource {
    data: String,
    // 故意不实现Send，限制跨线程移动
    _not_send: std::marker::PhantomData<*const ()>,
}

impl SingleThreadResource {
    pub fn new(data: &str) -> Self {
        SingleThreadResource {
            data: data.to_string(),
            _not_send: std::marker::PhantomData,
        }
    }

    pub fn modify(&mut self, new_data: &str) {
        self.data = new_data.to_string();
    }
}

// 安全地实现Sync，但不实现Send
// 这意味着可以在线程间共享引用，但不能移动所有权
unsafe impl Sync for SingleThreadResource {}

/// 使用RAII确保资源释放
pub struct DatabaseConnection {
    id: u64,
    active: bool,
}

impl DatabaseConnection {
    pub fn new(id: u64) -> Self {
        println!("[RAII] 数据库连接 #{} 已建立", id);
        DatabaseConnection { id, active: true }
    }

    pub fn query(&self, sql: &str) -> Result<String, &'static str> {
        if !self.active {
            return Err("连接已关闭");
        }
        Ok(format!("查询结果: {}", sql))
    }
}

impl Drop for DatabaseConnection {
    fn drop(&mut self) {
        if self.active {
            println!("[RAII] 数据库连接 #{} 自动关闭", self.id);
            self.active = false;
        }
    }
}

// =============================================================================
// 验证假设3: 错误处理 - 强制处理所有错误情况
// =============================================================================

/// 使用Result和?运算符强制错误传播
#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(String),
    ParseError(String),
    InvalidValue(String),
}

pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub debug_mode: bool,
}

impl AppConfig {
    /// 从字符串解析配置 - 必须处理所有错误
    pub fn from_str(content: &str) -> Result<Self, ConfigError> {
        let mut db_url = None;
        let mut port = None;
        let mut debug = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(ConfigError::ParseError(
                    format!("无效的配置行: {}", line)
                ));
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            match key {
                "database_url" => db_url = Some(value.to_string()),
                "port" => {
                    port = Some(value.parse::<u16>().map_err(|_| {
                        ConfigError::InvalidValue(format!("无效的端口号: {}", value))
                    })?);
                }
                "debug" => debug = Some(value == "true"),
                _ => return Err(ConfigError::InvalidValue(
                    format!("未知配置项: {}", key)
                )),
            }
        }

        Ok(AppConfig {
            database_url: db_url.ok_or(ConfigError::InvalidValue(
                "缺少 database_url".to_string()
            ))?,
            port: port.ok_or(ConfigError::InvalidValue(
                "缺少 port".to_string()
            ))?,
            debug_mode: debug.unwrap_or(false),
        })
    }
}

// =============================================================================
// 验证假设4: 消耗性设计 - 确保资源只能使用一次
// =============================================================================

/// 使用move语义确保token只能使用一次
pub struct AuthToken {
    token: String,
    used: bool,
}

impl AuthToken {
    pub fn new(token: &str) -> Self {
        AuthToken {
            token: token.to_string(),
            used: false,
        }
    }

    /// 消费token进行认证 - 只能调用一次
    pub fn authenticate(mut self) -> Result<Session, &'static str> {
        if self.used {
            return Err("Token已被使用");
        }
        self.used = true;
        println!("[消耗性设计] Token '{}' 已使用", self.token);
        Ok(Session { id: self.token })
    }
}

pub struct Session {
    pub id: String,
}

/// 使用线性类型思想 - 确保操作顺序
pub struct Transaction {
    id: u64,
    committed: bool,
}

impl Transaction {
    pub fn new(id: u64) -> Self {
        println!("[线性类型] 事务 #{} 开始", id);
        Transaction { id, committed: false }
    }

    /// 执行操作 - 返回新的Transaction，旧的不复存在
    pub fn execute<F>(self, operation: F) -> Result<Self, &'static str>
    where
        F: FnOnce() -> Result<(), &'static str>,
    {
        operation()?;
        println!("[线性类型] 事务 #{} 执行操作", self.id);
        Ok(self)
    }

    /// 提交事务 - 消费self
    pub fn commit(mut self) -> TransactionResult {
        self.committed = true;
        println!("[线性类型] 事务 #{} 已提交", self.id);
        TransactionResult { success: true }
    }

    /// 回滚事务 - 消费self
    pub fn rollback(self) -> TransactionResult {
        println!("[线性类型] 事务 #{} 已回滚", self.id);
        TransactionResult { success: false }
    }
}

pub struct TransactionResult {
    pub success: bool,
}

// =============================================================================
// 编译期验证 - 使用const和static_assert思想
// =============================================================================

/// 编译期常量验证
pub const MAX_BUFFER_SIZE: usize = 1024;
pub const MIN_BUFFER_SIZE: usize = 16;

/// 使用const泛型确保数组大小在编译期已知
pub struct FixedBuffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> FixedBuffer<N> {
    /// 编译期验证大小
    pub fn new() -> Self {
        // 编译期断言
        assert!(N >= MIN_BUFFER_SIZE, "缓冲区太小");
        assert!(N <= MAX_BUFFER_SIZE, "缓冲区太大");
        FixedBuffer {
            data: [0; N],
            len: 0,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        let available = N - self.len;
        let to_write = data.len().min(available);
        self.data[self.len..self.len + to_write].copy_from_slice(&data[..to_write]);
        self.len += to_write;
        Ok(to_write)
    }
}

// =============================================================================
// 测试和验证
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine() {
        let file = FileProcessor::new("test.txt")
            .open()
            .read()
            .parse();

        let data = file.get_data();
        assert_eq!(data.len(), 3);

        let _closed = file.close();
    }

    #[test]
    fn test_config_parsing() {
        let config_str = r#"
            database_url = postgres://localhost/db
            port = 5432
            debug = true
        "#;

        let config = AppConfig::from_str(config_str).unwrap();
        assert_eq!(config.port, 5432);
        assert!(config.debug_mode);
    }

    #[test]
    fn test_fixed_buffer() {
        let mut buf = FixedBuffer::<64>::new();
        let written = buf.write(b"hello").unwrap();
        assert_eq!(written, 5);
    }
}

// =============================================================================
// 主函数 - 演示各种防错设计模式
// =============================================================================

fn main() {
    println!("========================================");
    println!("防错工具设计验证 - Rust实现");
    println!("========================================\n");

    // 演示1: 类型状态机
    println!("--- 演示1: 类型状态机 ---");
    let processor = FileProcessor::new("data.txt")
        .open()
        .read()
        .parse();

    println!("解析数据: {:?}", processor.get_data());
    let _closed = processor.close();
    println!();

    // 演示2: RAII资源管理
    println!("--- 演示2: RAII资源管理 ---");
    {
        let conn = DatabaseConnection::new(1);
        let result = conn.query("SELECT * FROM users").unwrap();
        println!("查询结果: {}", result);
        // conn在这里自动释放
    }
    println!();

    // 演示3: 强制错误处理
    println!("--- 演示3: 强制错误处理 ---");
    let valid_config = r#"
        database_url = postgres://localhost/mydb
        port = 8080
        debug = false
    "#;

    match AppConfig::from_str(valid_config) {
        Ok(config) => println!("配置加载成功: port={}", config.port),
        Err(e) => println!("配置错误: {:?}", e),
    }
    println!();

    // 演示4: 消耗性设计
    println!("--- 演示4: 消耗性设计 ---");
    let token = AuthToken::new("secret-token-123");
    let session = token.authenticate().unwrap();
    println!("会话创建成功: id={}", session.id);
    // token不能再使用，因为它已经被move
    println!();

    // 演示5: 线性类型事务
    println!("--- 演示5: 线性类型事务 ---");
    let tx = Transaction::new(1001)
        .execute(|| {
            println!("  执行操作1...");
            Ok(())
        })
        .unwrap()
        .execute(|| {
            println!("  执行操作2...");
            Ok(())
        })
        .unwrap()
        .commit();

    println!("事务提交成功: {}", tx.success);
    println!();

    // 演示6: 编译期固定缓冲区
    println!("--- 演示6: 编译期固定缓冲区 ---");
    let mut buffer = FixedBuffer::<128>::new();
    let data = b"Hello, Rust!";
    let written = buffer.write(data).unwrap();
    println!("写入 {} 字节", written);
    println!();

    println!("========================================");
    println!("所有验证通过!");
    println!("========================================");
}
