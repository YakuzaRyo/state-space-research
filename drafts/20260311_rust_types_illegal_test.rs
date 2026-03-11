//! 非法状态转换测试 - 验证编译器阻止
//! 这个文件包含应该导致编译错误的代码
//! 用于验证Typestate模式的有效性

use std::marker::PhantomData;
use std::net::TcpStream;

// 状态标记
pub struct Disconnected;
pub struct Connected { stream: TcpStream }
pub struct Authenticated { stream: TcpStream, user_id: u64 }
pub struct Closed;

pub struct Connection<State> {
    state: State,
    _marker: PhantomData<State>,
}

impl Connection<Disconnected> {
    pub fn new() -> Self {
        Connection { state: Disconnected, _marker: PhantomData }
    }
}

impl Connection<Connected> {
    pub fn send_raw(&mut self, data: &[u8]) -> std::io::Result<usize> {
        use std::io::Write;
        self.state.stream.write(data)
    }
}

impl Connection<Authenticated> {
    pub fn send_secure_command(&mut self, command: &str) -> std::io::Result<usize> {
        let payload = format!("USER:{}:{}", 0, command);
        use std::io::Write;
        self.state.stream.write(payload.as_bytes())
    }
}

fn main() {
    // 测试1: 未连接时发送数据 - 应该编译错误
    let conn = Connection::new();
    conn.send_raw(b"test"); // ERROR: no method `send_raw` for `Connection<Disconnected>`

    // 测试2: 未认证时发送安全命令 - 应该编译错误
    // 假设我们有Connected状态的conn
    // conn.send_secure_command("GET /secret"); // ERROR: no method `send_secure_command` for `Connection<Connected>`

    // 测试3: 关闭后使用 - 应该编译错误
    // let closed: Connection<Closed> = ...;
    // closed.send_raw(b"test"); // ERROR: no method `send_raw` for `Connection<Closed>`

    println!("如果这行能打印，说明非法操作被注释掉了（正确行为）");
    println!("取消注释任何ERROR行都会导致编译失败，证明Typestate有效");
}
