//! 状态空间架构核心原则研究
//!
//! 核心问题: 如何让错误在设计上不可能发生?
//!
//! 研究假设:
//! 1. 技术假设: 状态空间架构通过类型系统消除运行时错误
//! 2. 实现假设: Rust的Typestate模式可实现"Make illegal states unrepresentable"
//! 3. 性能假设: 类型安全是零成本的(通过PhantomData)
//! 4. 适用性假设: 适用于状态机、协议验证、资源管理

use std::marker::PhantomData;

// ============================================================================
// 第一部分: 基础Typestate模式 - 文件状态机
// ============================================================================

/// 文件状态标记
mod file_states {
    /// 文件已创建但未打开
    pub struct Created;
    /// 文件已打开可读
    pub struct OpenForRead;
    /// 文件已打开可写
    pub struct OpenForWrite;
    /// 文件已关闭
    pub struct Closed;
}

use file_states::*;

/// 泛型文件结构，状态由类型参数编码
///
/// 关键设计: 状态不是运行时值，而是编译期类型
/// 这确保了无效状态转换在编译时被拒绝
pub struct File<State> {
    path: String,
    // PhantomData<State> 是零成本抽象
    // 它在编译时存在，运行时不占用内存
    _state: PhantomData<State>,
}

// 只有Created状态可以创建新文件
impl File<Created> {
    pub fn new(path: &str) -> Self {
        println!("[File] Created: {}", path);
        Self {
            path: path.to_string(),
            _state: PhantomData,
        }
    }

    /// 转换为只读打开状态
    /// 注意: self被消耗，旧状态不可用
    pub fn open_for_read(self) -> File<OpenForRead> {
        println!("[File] Opening for read: {}", self.path);
        File {
            path: self.path,
            _state: PhantomData,
        }
    }

    /// 转换为写入打开状态
    pub fn open_for_write(self) -> File<OpenForWrite> {
        println!("[File] Opening for write: {}", self.path);
        File {
            path: self.path,
            _state: PhantomData,
        }
    }
}

// 只有OpenForRead状态可以读取
impl File<OpenForRead> {
    pub fn read(&self) -> String {
        println!("[File] Reading from: {}", self.path);
        format!("Content of {}", self.path)
    }

    pub fn close(self) -> File<Closed> {
        println!("[File] Closing (was read mode): {}", self.path);
        File {
            path: self.path,
            _state: PhantomData,
        }
    }
}

// 只有OpenForWrite状态可以写入
impl File<OpenForWrite> {
    pub fn write(&mut self, content: &str) {
        println!("[File] Writing '{}' to: {}", content, self.path);
    }

    pub fn close(self) -> File<Closed> {
        println!("[File] Closing (was write mode): {}", self.path);
        File {
            path: self.path,
            _state: PhantomData,
        }
    }
}

// Closed状态只能被丢弃，不能进行任何操作
impl File<Closed> {
    /// 获取文件路径用于记录
    pub fn path(&self) -> &str {
        &self.path
    }
}

// ============================================================================
// 第二部分: 高级模式 - 带状态转换验证的连接状态机
// ============================================================================

/// 连接状态
mod connection_states {
    /// 初始状态
    pub struct Disconnected;
    /// 连接中
    pub struct Connecting { attempt: u32 }
    /// 已连接
    pub struct Connected { session_id: u64 }
    /// 连接失败
    pub struct Failed { reason: String }
    /// 连接已关闭
    pub struct Closed;
}

use connection_states::*;

/// 连接配置
pub struct ConnectionConfig {
    host: String,
    port: u16,
    max_retries: u32,
}

/// 连接状态机
///
/// 设计要点:
/// 1. 每个状态可以携带不同的数据
/// 2. 状态转换是类型安全的
/// 3. 无效操作在编译时被阻止
pub struct Connection<State> {
    config: ConnectionConfig,
    state_data: State,
}

impl Connection<Disconnected> {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state_data: Disconnected,
        }
    }

    /// 开始连接，进入Connecting状态
    pub fn connect(self) -> Connection<Connecting> {
        println!("[Connection] Starting connection to {}:{}",
                 self.config.host, self.config.port);
        Connection {
            config: self.config,
            state_data: Connecting { attempt: 1 },
        }
    }
}

impl Connection<Connecting> {
    /// 获取当前尝试次数
    pub fn attempt(&self) -> u32 {
        self.state_data.attempt
    }

    /// 模拟连接成功
    pub fn on_connected(self, session_id: u64) -> Connection<Connected> {
        println!("[Connection] Connected with session {}", session_id);
        Connection {
            config: self.config,
            state_data: Connected { session_id },
        }
    }

    /// 模拟连接失败，可以选择重试或失败
    pub fn on_failed(self, reason: String) -> Result<Connection<Connecting>, Connection<Failed>> {
        let attempt = self.state_data.attempt;
        if attempt < self.config.max_retries {
            println!("[Connection] Retry {}/{} after: {}",
                     attempt, self.config.max_retries, reason);
            Ok(Connection {
                config: self.config,
                state_data: Connecting { attempt: attempt + 1 },
            })
        } else {
            println!("[Connection] Max retries reached, failing");
            Err(Connection {
                config: self.config,
                state_data: Failed { reason },
            })
        }
    }
}

impl Connection<Connected> {
    /// 获取会话ID
    pub fn session_id(&self) -> u64 {
        self.state_data.session_id
    }

    /// 发送数据（只能在Connected状态）
    pub fn send(&self, data: &[u8]) {
        println!("[Connection] Sending {} bytes on session {}",
                 data.len(), self.session_id());
    }

    /// 接收数据（只能在Connected状态）
    pub fn receive(&self) -> Vec<u8> {
        println!("[Connection] Receiving data on session {}",
                 self.session_id());
        vec![1, 2, 3] // 模拟数据
    }

    /// 关闭连接
    pub fn disconnect(self) -> Connection<Closed> {
        println!("[Connection] Disconnecting session {}",
                 self.state_data.session_id);
        Connection {
            config: self.config,
            state_data: Closed,
        }
    }
}

impl Connection<Failed> {
    /// 获取失败原因
    pub fn reason(&self) -> &str {
        &self.state_data.reason
    }

    /// 可以重新尝试连接
    pub fn retry(self) -> Connection<Connecting> {
        println!("[Connection] Retrying after failure: {}",
                 self.state_data.reason);
        Connection {
            config: self.config,
            state_data: Connecting { attempt: 1 },
        }
    }
}

// ============================================================================
// 第三部分: 零成本抽象验证 - 协议状态机
// ============================================================================

/// 协议状态 - 模拟HTTP请求/响应协议
mod protocol_states {
    /// 初始状态
    pub struct Init;
    /// 请求已构建
    pub struct RequestBuilt;
    /// 请求已发送
    pub struct RequestSent;
    /// 响应已接收
    pub struct ResponseReceived;
    /// 协议完成
    pub struct Complete;
}

use protocol_states::*;

/// HTTP方法
#[derive(Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

/// 协议状态机
///
/// 这个例子展示了如何强制正确的协议顺序:
/// Init -> RequestBuilt -> RequestSent -> ResponseReceived -> Complete
pub struct Protocol<State> {
    method: Method,
    url: String,
    _state: PhantomData<State>,
}

impl Protocol<Init> {
    pub fn new() -> Self {
        Self {
            method: Method::Get,
            url: String::new(),
            _state: PhantomData,
        }
    }

    pub fn with_method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    /// 构建请求，进入RequestBuilt状态
    pub fn build(self) -> Protocol<RequestBuilt> {
        println!("[Protocol] Building {:?} request to {}",
                 self.method, self.url);
        Protocol {
            method: self.method,
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Protocol<RequestBuilt> {
    /// 发送请求，进入RequestSent状态
    pub fn send(self) -> Protocol<RequestSent> {
        println!("[Protocol] Sending {:?} request to {}",
                 self.method, self.url);
        Protocol {
            method: self.method,
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Protocol<RequestSent> {
    /// 接收响应，进入ResponseReceived状态
    pub fn receive(self) -> Protocol<ResponseReceived> {
        println!("[Protocol] Received response for {:?} {}",
                 self.method, self.url);
        Protocol {
            method: self.method,
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Protocol<ResponseReceived> {
    /// 处理响应
    pub fn process(&self) -> String {
        format!("Processed {:?} response from {}", self.method, self.url)
    }

    /// 完成协议
    pub fn complete(self) -> Protocol<Complete> {
        println!("[Protocol] Completed");
        Protocol {
            method: self.method,
            url: self.url,
            _state: PhantomData,
        }
    }
}

impl Protocol<Complete> {
    pub fn is_complete(&self) -> bool {
        true
    }
}

// ============================================================================
// 第四部分: 编译期错误捕获演示
// ============================================================================

/// 这个模块包含故意错误的代码，用于演示编译期错误捕获
/// 取消注释以查看编译错误
#[cfg(feature = "compile_errors")]
mod compile_error_demo {
    use super::*;

    fn invalid_transitions() {
        // 错误1: 不能在Created状态读取
        // let file = File::new("test.txt");
        // file.read(); // 编译错误: File<Created>没有read方法

        // 错误2: 不能在OpenForRead状态写入
        // let file = File::new("test.txt").open_for_read();
        // file.write("data"); // 编译错误: File<OpenForRead>没有write方法

        // 错误3: 不能跳过状态
        // let file = File::new("test.txt");
        // file.close(); // 编译错误: File<Created>没有close方法

        // 错误4: 不能在Disconnected状态发送数据
        // let conn = Connection::new(ConnectionConfig { ... });
        // conn.send(b"data"); // 编译错误: Connection<Disconnected>没有send方法
    }
}

// ============================================================================
// 主函数: 演示正确的状态转换
// ============================================================================

fn main() {
    println!("=== 状态空间架构核心原则演示 ===\n");

    // 演示1: 文件状态机
    println!("--- 文件状态机演示 ---");
    let file = File::new("document.txt")
        .open_for_write();
    // file.read(); // 编译错误! 不能在写入模式读取
    let mut file = file;
    file.write("Hello, Typestate!");
    let file = file.close();
    println!("File closed: {}\n", file.path());

    // 演示2: 连接状态机
    println!("--- 连接状态机演示 ---");
    let config = ConnectionConfig {
        host: "example.com".to_string(),
        port: 443,
        max_retries: 3,
    };

    let conn = Connection::new(config)
        .connect()  // Disconnected -> Connecting
        .on_connected(12345); // Connecting -> Connected

    conn.send(b"Hello");
    let response = conn.receive();
    println!("Received: {:?}", response);
    let conn = conn.disconnect(); // Connected -> Closed
    // conn.send(b"data"); // 编译错误! 已关闭的连接不能发送
    println!("Connection closed\n");

    // 演示3: 协议状态机（Builder模式变体）
    println!("--- 协议状态机演示 ---");
    let protocol = Protocol::new()
        .with_method(Method::Post)
        .with_url("https://api.example.com/data")
        .build()      // Init -> RequestBuilt
        .send()       // RequestBuilt -> RequestSent
        .receive()    // RequestSent -> ResponseReceived
        .complete();  // ResponseReceived -> Complete

    println!("Protocol complete: {}\n", protocol.is_complete());

    println!("=== 所有状态转换成功完成 ===");
    println!("\n核心结论:");
    println!("1. 无效状态转换在编译时被捕获");
    println!("2. PhantomData实现零成本抽象");
    println!("3. 类型系统即文档，状态转换自描述");
    println!("4. 运行时无需状态检查，提升性能");
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_state_transitions() {
        let file = File::<Created>::new("test.txt");
        let file = file.open_for_read();
        let _content = file.read();
        let _file = file.close();
    }

    #[test]
    fn test_connection_success_path() {
        let config = ConnectionConfig {
            host: "test.com".to_string(),
            port: 80,
            max_retries: 2,
        };

        let conn = Connection::new(config)
            .connect()
            .on_connected(999);

        assert_eq!(conn.session_id(), 999);
        conn.send(b"test");
        let _ = conn.disconnect();
    }

    #[test]
    fn test_protocol_flow() {
        let protocol = Protocol::new()
            .with_method(Method::Get)
            .with_url("/test")
            .build()
            .send()
            .receive()
            .complete();

        assert!(protocol.is_complete());
    }

    /// 验证PhantomData的大小为0
    #[test]
    fn test_zero_cost() {
        use std::mem::size_of;

        // File<Created>应该只包含path字段的大小
        let file_size = size_of::<File<Created>>();
        let string_size = size_of::<String>();

        assert_eq!(file_size, string_size,
                   "PhantomData应该是零大小的");
    }
}
