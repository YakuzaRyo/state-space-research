//! 状态空间架构核心原则验证代码
//! 研究方向: 01_core_principles - 如何让错误在设计上不可能发生
//!
//! 核心假设:
//! 1. 技术假设: Typestate模式将运行时错误转化为编译时错误
//! 2. 实现假设: Rust类型系统通过PhantomData和泛型实现零成本状态空间
//! 3. 性能假设: 类型驱动设计在编译期完成状态检查，运行时零开销
//! 4. 适用性假设: 适用于协议实现、资源管理、工作流引擎等状态敏感场景

use std::marker::PhantomData;

// ============================================================================
// 第一部分: 基础Typestate模式 - 连接状态机
// ============================================================================

/// 未连接状态标记
pub struct Disconnected;
/// 连接中状态标记
pub struct Connecting;
/// 已连接状态标记
pub struct Connected;
/// 已关闭状态标记
pub struct Closed;

/// 泛型连接状态机
/// State类型参数在编译期确定当前状态
pub struct Connection<State> {
    address: String,
    _state: PhantomData<State>,
}

// 仅在Disconnected状态下可创建连接
impl Connection<Disconnected> {
    pub fn new(address: impl Into<String>) -> Self {
        Connection {
            address: address.into(),
            _state: PhantomData,
        }
    }

    /// 开始连接，状态从Disconnected转移到Connecting
    pub fn connect(self) -> Connection<Connecting> {
        println!("Connecting to {}...", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// 仅在Connecting状态下可完成连接
impl Connection<Connecting> {
    /// 连接成功，状态从Connecting转移到Connected
    pub fn handshake_success(self) -> Connection<Connected> {
        println!("Connection established!");
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }

    /// 连接失败，状态从Connecting转移回Disconnected
    pub fn handshake_failed(self) -> Connection<Disconnected> {
        println!("Connection failed, returning to disconnected state.");
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// 仅在Connected状态下可发送数据
impl Connection<Connected> {
    pub fn send(&self, data: &str) {
        println!("Sending data: {}", data);
    }

    pub fn receive(&self) -> String {
        "response data".to_string()
    }

    /// 关闭连接，状态从Connected转移到Closed
    pub fn close(self) -> Connection<Closed> {
        println!("Connection closed.");
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Closed状态 - 无法再操作
impl Connection<Closed> {
    pub fn address(&self) -> &str {
        &self.address
    }
    // 不提供任何状态转换方法，Closed是终止状态
}

// ============================================================================
// 第二部分: 高级模式 - 带资源的状态机
// ============================================================================

/// 文件句柄状态
pub struct Unopened;
pub struct Opened { handle: u32 };
pub struct Reading;
pub struct Writing;

/// 文件状态机，包含实际资源
pub struct FileHandle<State> {
    path: String,
    state: State,
}

impl FileHandle<Unopened> {
    pub fn new(path: impl Into<String>) -> Self {
        FileHandle {
            path: path.into(),
            state: Unopened,
        }
    }

    pub fn open(self) -> FileHandle<Opened> {
        FileHandle {
            path: self.path,
            state: Opened { handle: 42 }, // 模拟文件句柄
        }
    }
}

impl FileHandle<Opened> {
    pub fn handle(&self) -> u32 {
        self.state.handle
    }

    pub fn read_mode(self) -> FileHandle<Reading> {
        FileHandle {
            path: self.path,
            state: Reading,
        }
    }

    pub fn write_mode(self) -> FileHandle<Writing> {
        FileHandle {
            path: self.path,
            state: Writing,
        }
    }
}

impl FileHandle<Reading> {
    pub fn read(&self, buf: &mut [u8]) -> usize {
        println!("Reading {} bytes from {}", buf.len(), self.path);
        buf.len()
    }

    pub fn close(self) -> FileHandle<Unopened> {
        FileHandle {
            path: self.path,
            state: Unopened,
        }
    }
}

impl FileHandle<Writing> {
    pub fn write(&self, data: &[u8]) -> usize {
        println!("Writing {} bytes to {}", data.len(), self.path);
        data.len()
    }

    pub fn close(self) -> FileHandle<Unopened> {
        FileHandle {
            path: self.path,
            state: Unopened,
        }
    }
}

// ============================================================================
// 第三部分: 零成本抽象验证
// ============================================================================

/// 零大小类型状态标记
pub struct Idle;
pub struct Running;
pub struct Paused;

/// 任务状态机 - 验证ZST(Zero-Sized Type)特性
pub struct Task<State> {
    id: u64,
    _state: PhantomData<State>,
}

impl Task<Idle> {
    pub fn new(id: u64) -> Self {
        Task { id, _state: PhantomData }
    }

    pub fn start(self) -> Task<Running> {
        Task { id: self.id, _state: PhantomData }
    }
}

impl Task<Running> {
    pub fn pause(self) -> Task<Paused> {
        Task { id: self.id, _state: PhantomData }
    }

    pub fn stop(self) -> Task<Idle> {
        Task { id: self.id, _state: PhantomData }
    }
}

impl Task<Paused> {
    pub fn resume(self) -> Task<Running> {
        Task { id: self.id, _state: PhantomData }
    }

    pub fn abort(self) -> Task<Idle> {
        Task { id: self.id, _state: PhantomData }
    }
}

// ============================================================================
// 第四部分: 编译时状态验证
// ============================================================================

/// 协议状态 - 模拟TLS握手
pub struct ClientHello;
pub struct ServerHello;
pub struct Encrypted;

pub struct SecureChannel<State> {
    session_id: u64,
    _state: PhantomData<State>,
}

impl SecureChannel<ClientHello> {
    pub fn initiate(session_id: u64) -> Self {
        SecureChannel { session_id, _state: PhantomData }
    }

    pub fn send_client_hello(self) -> SecureChannel<ServerHello> {
        SecureChannel { session_id: self.session_id, _state: PhantomData }
    }
}

impl SecureChannel<ServerHello> {
    pub fn receive_server_hello(self) -> SecureChannel<Encrypted> {
        SecureChannel { session_id: self.session_id, _state: PhantomData }
    }
}

impl SecureChannel<Encrypted> {
    pub fn send_encrypted(&self, data: &[u8]) -> Vec<u8> {
        // 加密数据
        data.to_vec()
    }

    pub fn receive_encrypted(&self, data: &[u8]) -> Vec<u8> {
        // 解密数据
        data.to_vec()
    }
}

// ============================================================================
// 第五部分: 类型级状态空间约束
// ============================================================================

/// 使用trait约束限制状态转换
pub trait ValidState {}
impl ValidState for Idle {}
impl ValidState for Running {}
impl ValidState for Paused {}

/// 带约束的泛型结构
pub struct StateMachine<S: ValidState> {
    data: String,
    _state: PhantomData<S>,
}

// ============================================================================
// 测试与验证
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_lifecycle() {
        // 正确的状态转换流程
        let conn = Connection::new("127.0.0.1:8080");
        let conn = conn.connect();
        let conn = conn.handshake_success();
        conn.send("Hello");
        let _closed = conn.close();

        // 以下代码将无法编译 - 非法状态转换
        // let conn = Connection::new("127.0.0.1:8080");
        // conn.send("Hello"); // 错误: Disconnected状态没有send方法

        // let conn = Connection::new("127.0.0.1:8080");
        // let conn = conn.connect();
        // conn.send("Hello"); // 错误: Connecting状态没有send方法
    }

    #[test]
    fn test_file_handle() {
        let file = FileHandle::new("test.txt");
        let file = file.open();
        let file = file.read_mode();
        let mut buf = [0u8; 1024];
        let _n = file.read(&mut buf);
        let _file = file.close();
    }

    #[test]
    fn test_task_state_machine() {
        let task = Task::new(1);
        let task = task.start();
        let task = task.pause();
        let task = task.resume();
        let _task = task.stop();
    }

    #[test]
    fn test_secure_channel() {
        let channel = SecureChannel::initiate(12345);
        let channel = channel.send_client_hello();
        let channel = channel.receive_server_hello();
        let _encrypted = channel.send_encrypted(b"secret data");
    }
}

// ============================================================================
// 尺寸验证 - 证明零成本抽象
// ============================================================================

pub fn assert_zero_sized_types() {
    // 验证状态标记是零大小类型
    assert_eq!(std::mem::size_of::<Idle>(), 0);
    assert_eq!(std::mem::size_of::<Running>(), 0);
    assert_eq!(std::mem::size_of::<Paused>(), 0);

    // 验证带PhantomData的Task在任意状态下大小相同
    assert_eq!(std::mem::size_of::<Task<Idle>>(), std::mem::size_of::<u64>());
    assert_eq!(std::mem::size_of::<Task<Running>>(), std::mem::size_of::<u64>());
    assert_eq!(std::mem::size_of::<Task<Paused>>(), std::mem::size_of::<u64>());

    println!("All zero-cost abstraction assertions passed!");
}

// 主函数用于演示
fn main() {
    println!("=== 状态空间架构核心原则验证 ===\n");

    // 演示1: 连接状态机
    println!("1. 连接状态机演示:");
    let conn = Connection::new("127.0.0.1:8080");
    let conn = conn.connect();
    let conn = conn.handshake_success();
    conn.send("Hello, Typestate!");
    let _closed = conn.close();
    println!();

    // 演示2: 文件句柄
    println!("2. 文件句柄状态机演示:");
    let file = FileHandle::new("example.txt");
    let file = file.open();
    let file = file.write_mode();
    let _ = file.write(b"Test data");
    let _file = file.close();
    println!();

    // 演示3: 零成本验证
    println!("3. 零成本抽象验证:");
    assert_zero_sized_types();
    println!("Task<Idle> size: {} bytes", std::mem::size_of::<Task<Idle>>());
    println!("Task<Running> size: {} bytes", std::mem::size_of::<Task<Running>>());
    println!();

    println!("=== 所有验证通过 ===");
    println!("\n核心结论:");
    println!("- 非法状态转换在编译期被阻止");
    println!("- 状态标记使用ZST，运行时零开销");
    println!("- 类型系统成为状态机的编译期验证器");
}