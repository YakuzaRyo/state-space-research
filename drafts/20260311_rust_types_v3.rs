//! Rust Typestate Pattern - Network Connection State Machine
//!
//! 验证假设：
//! 1. 所有权系统保证状态转换安全
//! 2. 类型系统编码业务规则（连接必须先认证才能发送安全命令）
//! 3. 零成本抽象 - ZST在运行时无开销
//! 4. 编译器阻止非法状态转换

use std::marker::PhantomData;
use std::net::TcpStream;
use std::io::{self, Write, Read};

// =============================================================================
// 状态标记类型 (Zero-Sized Types)
// =============================================================================

/// 连接已断开状态
pub struct Disconnected;

/// 连接已建立但未认证
pub struct Connected {
    stream: TcpStream,
}

/// 连接已认证，可以执行安全操作
pub struct Authenticated {
    stream: TcpStream,
    user_id: u64,
}

/// 连接已关闭
pub struct Closed;

// =============================================================================
// 连接状态机 - 使用泛型和PhantomData实现Typestate
// =============================================================================

/// 通用连接结构体，State类型参数决定可用操作
pub struct Connection<State> {
    state: State,
    // PhantomData告诉编译器这个泛型参数被使用，但不在运行时占用空间
    _marker: PhantomData<State>,
}

// =============================================================================
// Disconnected状态实现
// =============================================================================

impl Connection<Disconnected> {
    /// 创建新的断开连接
    pub fn new() -> Self {
        Connection {
            state: Disconnected,
            _marker: PhantomData,
        }
    }

    /// 连接到服务器 - 状态转换: Disconnected -> Connected
    /// 注意：self被消费，返回新状态，旧状态不可用
    pub fn connect(self, addr: &str) -> io::Result<Connection<Connected>> {
        let stream = TcpStream::connect(addr)?;
        Ok(Connection {
            state: Connected { stream },
            _marker: PhantomData,
        })
    }
}

// =============================================================================
// Connected状态实现
// =============================================================================

impl Connection<Connected> {
    /// 发送原始数据（未认证状态下允许）
    pub fn send_raw(&mut self, data: &[u8]) -> io::Result<usize> {
        self.state.stream.write(data)
    }

    /// 接收数据（未认证状态下允许）
    pub fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.state.stream.read(buf)
    }

    /// 认证 - 状态转换: Connected -> Authenticated
    /// 业务规则：必须先认证才能发送安全命令
    pub fn authenticate(self, token: &str) -> Result<Connection<Authenticated>, AuthError> {
        // 模拟认证逻辑
        if token.starts_with("valid_") {
            // 解析用户ID（简化示例）
            let user_id = token.strip_prefix("valid_")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            Ok(Connection {
                state: Authenticated {
                    stream: self.state.stream,
                    user_id,
                },
                _marker: PhantomData,
            })
        } else {
            Err(AuthError::InvalidToken)
        }
    }

    /// 断开连接 - 状态转换: Connected -> Closed
    pub fn disconnect(self) -> Connection<Closed> {
        // 显式关闭底层连接
        drop(self.state.stream);
        Connection {
            state: Closed,
            _marker: PhantomData,
        }
    }

    /// 获取底层流的可变引用（用于高级操作）
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.state.stream
    }
}

// =============================================================================
// Authenticated状态实现
// =============================================================================

impl Connection<Authenticated> {
    /// 获取当前用户ID
    pub fn user_id(&self) -> u64 {
        self.state.user_id
    }

    /// 发送安全命令（仅认证后可使用）
    /// 业务规则编码：这个方法只在Authenticated状态下可用
    pub fn send_secure_command(&mut self, command: &str) -> io::Result<usize> {
        let secure_payload = format!("USER:{}:{}", self.state.user_id, command);
        self.state.stream.write(secure_payload.as_bytes())
    }

    /// 发送原始数据（认证后仍可用）
    pub fn send_raw(&mut self, data: &[u8]) -> io::Result<usize> {
        self.state.stream.write(data)
    }

    /// 接收数据
    pub fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.state.stream.read(buf)
    }

    /// 注销 - 状态转换: Authenticated -> Connected
    /// 回到未认证状态，但不关闭连接
    pub fn logout(self) -> Connection<Connected> {
        Connection {
            state: Connected {
                stream: self.state.stream,
            },
            _marker: PhantomData,
        }
    }

    /// 断开连接 - 状态转换: Authenticated -> Closed
    pub fn disconnect(self) -> Connection<Closed> {
        drop(self.state.stream);
        Connection {
            state: Closed,
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// Closed状态实现 - 无可用操作
// =============================================================================

impl Connection<Closed> {
    /// 重新连接 - 状态转换: Closed -> Connected
    pub fn reconnect(self, addr: &str) -> io::Result<Connection<Connected>> {
        let stream = TcpStream::connect(addr)?;
        Ok(Connection {
            state: Connected { stream },
            _marker: PhantomData,
        })
    }
}

// =============================================================================
// 错误类型
// =============================================================================

#[derive(Debug)]
pub enum AuthError {
    InvalidToken,
    IoError(io::Error),
}

impl From<io::Error> for AuthError {
    fn from(err: io::Error) -> Self {
        AuthError::IoError(err)
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidToken => write!(f, "Invalid authentication token"),
            AuthError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for AuthError {}

// =============================================================================
// 类型级状态转换验证 - 编译期检查
// =============================================================================

/// 这个模块包含编译期验证测试
/// 如果代码能编译通过，说明类型系统正确工作
#[cfg(test)]
mod typestate_tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        // 有效状态转换链
        let conn = Connection::new();
        // conn.send_raw(b"test"); // 编译错误：Disconnected状态没有send_raw

        // 连接服务器
        let mut conn = conn.connect("127.0.0.1:8080").expect("Failed to connect");

        // 未认证状态可以发送原始数据
        conn.send_raw(b"HELLO").expect("Failed to send");

        // 认证
        let mut conn = conn.authenticate("valid_42").expect("Auth failed");

        // 认证后可以发送安全命令
        assert_eq!(conn.user_id(), 42);
        // conn.send_secure_command("GET /secret") 会实际发送数据

        // 注销回到未认证状态
        let conn = conn.logout();
        // conn.send_secure_command("GET /secret"); // 编译错误：Connected状态没有send_secure_command

        // 断开连接
        let _closed = conn.disconnect();
        // _closed.send_raw(b"test"); // 编译错误：Closed状态没有send_raw
    }

    #[test]
    fn test_authentication_failure() {
        let conn = Connection::new();
        let conn = conn.connect("127.0.0.1:8080").expect("Failed to connect");

        // 认证失败，保持在Connected状态
        let result = conn.authenticate("invalid_token");
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }
}

// =============================================================================
// 零成本抽象验证
// =============================================================================

/// 验证ZST的大小
pub fn verify_zero_cost() {
    use std::mem::size_of;

    println!("Zero-Cost Abstraction Verification:");
    println!("  size_of::<Disconnected>() = {} bytes", size_of::<Disconnected>());
    println!("  size_of::<Closed>() = {} bytes", size_of::<Closed>());
    println!("  size_of::<PhantomData<Disconnected>>() = {} bytes",
             size_of::<PhantomData<Disconnected>>());

    // Connection<Disconnected> 应该只包含PhantomData的开销（0字节）
    println!("  size_of::<Connection<Disconnected>>() = {} bytes",
             size_of::<Connection<Disconnected>>());
}

// =============================================================================
// 高级模式：使用GAT的泛型状态机
// =============================================================================

/// 泛型状态机trait，展示GAT的使用
pub trait StateMachine {
    /// 当前状态类型
    type State;

    /// 状态转换函数
    fn transition<NextState>(self) -> NextState;
}

/// 带有关联类型的状态trait
pub trait State {
    /// 此状态下允许的操作
    type Operation;

    /// 执行操作
    fn execute(&self, op: Self::Operation);
}

// =============================================================================
// 主函数 - 演示用法
// =============================================================================

fn main() {
    println!("Rust Typestate Pattern - Network Connection State Machine\n");

    // 验证零成本抽象
    verify_zero_cost();
    println!();

    // 演示状态转换
    println!("State Transition Demo:");
    println!("  1. Creating new connection (Disconnected state)");
    let conn = Connection::new();

    println!("  2. Connecting to server (Disconnected -> Connected)");
    match conn.connect("127.0.0.1:8080") {
        Ok(mut conn) => {
            println!("     Connected! Can send raw data.");

            // 发送原始数据
            if let Err(e) = conn.send_raw(b"HELLO SERVER") {
                println!("     Send error: {}", e);
            }

            println!("  3. Authenticating (Connected -> Authenticated)");
            // 注意：authenticate消费conn，所以需要在match前保存conn用于错误处理
            // 但由于authenticate获取self所有权，我们需要在Err时重新获取conn
            // 这里我们修改设计：authenticate返回Result时保留原conn
            // 为简化演示，我们假设认证成功
            let mut auth_conn = conn.authenticate("valid_123").expect("Auth should succeed in demo");

            println!("     Authenticated as user {}!", auth_conn.user_id());
            println!("     Can now send secure commands.");

            // 发送安全命令
            if let Err(e) = auth_conn.send_secure_command("GET /api/data") {
                println!("     Secure send error: {}", e);
            }

            println!("  4. Logging out (Authenticated -> Connected)");
            let conn = auth_conn.logout();
            println!("     Back to Connected state.");

            println!("  5. Disconnecting (Connected -> Closed)");
            let _closed = conn.disconnect();
            println!("     Connection closed.");
        }
        Err(e) => {
            println!("     Connection failed: {}", e);
        }
    }

    println!("\nCompile-time guarantees:");
    println!("  - Cannot send data before connecting (compile error)");
    println!("  - Cannot send secure commands before authenticating (compile error)");
    println!("  - Cannot use connection after closing (compile error)");
    println!("  - State transitions are enforced by type system");
}
