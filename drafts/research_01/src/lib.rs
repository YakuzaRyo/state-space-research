//! 研究方向 01: 核心原则 - 让错误在设计上不可能发生
//!
//! 核心问题: 如何利用Rust类型系统在编译期消除错误?
//!
//! 研究假设:
//! 1. Type State模式可以将运行时检查转移到编译期
//! 2. 零大小类型(ZST)可以用于标记状态而不增加运行时开销
//! 3. 私有构造函数+Builder模式可以强制正确的初始化顺序
//! 4. 泛型约束可以编码业务规则到类型系统

use std::marker::PhantomData;

// ============================================
// 假设1: Type State模式 - 将状态转换编码到类型
// ============================================

/// 未连接状态标记
pub struct Disconnected;
/// 已连接状态标记
pub struct Connected;

/// Type State模式: 数据库连接
/// 连接只能在编译期确认已连接的状态下执行查询
pub struct DatabaseConnection<State = Disconnected> {
    conn_string: String,
    _state: PhantomData<State>,
}

impl DatabaseConnection<Disconnected> {
    /// 只能通过new创建未连接状态
    pub fn new(conn_string: impl Into<String>) -> Self {
        Self {
            conn_string: conn_string.into(),
            _state: PhantomData,
        }
    }

    /// connect方法消费self，返回Connected状态
    /// 这确保了无法在已连接状态下再次连接
    pub fn connect(self) -> Result<DatabaseConnection<Connected>, ConnectionError> {
        if self.conn_string.is_empty() {
            return Err(ConnectionError::InvalidConnectionString);
        }
        // 模拟连接成功
        Ok(DatabaseConnection {
            conn_string: self.conn_string,
            _state: PhantomData,
        })
    }
}

impl DatabaseConnection<Connected> {
    /// 只能在Connected状态下执行查询
    /// 编译器会阻止对未连接连接调用query
    pub fn query(&self, sql: &str) -> Result<Vec<Row>, QueryError> {
        if sql.is_empty() {
            return Err(QueryError::EmptyQuery);
        }
        // 模拟查询
        Ok(vec![Row { data: sql.to_string() }])
    }

    /// 断开连接，状态转换回Disconnected
    pub fn disconnect(self) -> DatabaseConnection<Disconnected> {
        DatabaseConnection {
            conn_string: self.conn_string,
            _state: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    data: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionError {
    InvalidConnectionString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    EmptyQuery,
    NotConnected,
}

// ============================================
// 假设2: 编译期尺寸检查 - 防止缓冲区溢出
// ============================================

/// 使用const泛型在编译期固定缓冲区大小
#[derive(Debug, Clone)]
pub struct FixedBuffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> FixedBuffer<N> {
    /// 创建空缓冲区
    pub const fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    /// 尝试写入数据，编译期知道容量上限
    pub fn try_write(&mut self, data: &[u8]) -> Result<usize, BufferError> {
        let available = N.saturating_sub(self.len);
        if data.len() > available {
            return Err(BufferError::InsufficientSpace);
        }
        let to_write = data.len().min(available);
        self.data[self.len..self.len + to_write].copy_from_slice(&data[..to_write]);
        self.len += to_write;
        Ok(to_write)
    }

    /// 获取当前数据（安全，保证在边界内）
    pub fn data(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// 编译期确定的容量
    pub const fn capacity(&self) -> usize {
        N
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    InsufficientSpace,
}

// ============================================
// 假设3: 私有构造 + 验证后的类型包装
// ============================================

/// 经过验证的Email地址
/// 只能通过验证函数创建，确保内部email总是有效的
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEmail {
    email: String,
    // 私有字段防止外部直接构造
}

impl VerifiedEmail {
    /// 唯一构造方式：通过验证
    pub fn parse(email: impl Into<String>) -> Result<Self, EmailError> {
        let email = email.into();
        if !email.contains('@') {
            return Err(EmailError::MissingAtSymbol);
        }
        if email.starts_with('@') || email.ends_with('@') {
            return Err(EmailError::InvalidFormat);
        }
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 || parts[1].is_empty() {
            return Err(EmailError::InvalidDomain);
        }

        Ok(Self { email })
    }

    pub fn as_str(&self) -> &str {
        &self.email
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailError {
    MissingAtSymbol,
    InvalidFormat,
    InvalidDomain,
}

// ============================================
// 假设4: 线性类型模拟 - 确保资源被使用
// ============================================

/// 使用Drop确保资源被正确处理
/// 模拟线性类型：资源必须被显式消费
pub struct LinearResource<T> {
    value: Option<T>,
    name: &'static str,
}

impl<T> LinearResource<T> {
    pub fn new(value: T, name: &'static str) -> Self {
        Self {
            value: Some(value),
            name,
        }
    }

    /// 消费资源，返回内部值
    /// 调用后资源被标记为已使用
    pub fn consume(mut self) -> T {
        self.value.take().expect("Resource already consumed")
    }

    /// 检查资源是否已被消费
    pub fn is_consumed(&self) -> bool {
        self.value.is_none()
    }
}

impl<T> Drop for LinearResource<T> {
    fn drop(&mut self) {
        if self.value.is_some() {
            panic!(
                "LinearResource '{}' was dropped without being consumed! \
                 This is a resource leak.",
                self.name
            );
        }
    }
}

// ============================================
// 假设5: 状态机编码 - 非法转换在编译期被拒绝
// ============================================

/// 订单状态机
pub struct Pending;
pub struct Paid;
pub struct Shipped;
pub struct Delivered;

pub struct Order<State> {
    id: u64,
    amount: f64,
    _state: PhantomData<State>,
}

impl Order<Pending> {
    pub fn new(id: u64, amount: f64) -> Self {
        Self {
            id,
            amount,
            _state: PhantomData,
        }
    }

    /// 只有Pending状态可以支付
    pub fn pay(self) -> Order<Paid> {
        Order {
            id: self.id,
            amount: self.amount,
            _state: PhantomData,
        }
    }
}

impl Order<Paid> {
    /// 只有Paid状态可以发货
    pub fn ship(self) -> Order<Shipped> {
        Order {
            id: self.id,
            amount: self.amount,
            _state: PhantomData,
        }
    }
}

impl Order<Shipped> {
    /// 只有Shipped状态可以送达
    pub fn deliver(self) -> Order<Delivered> {
        Order {
            id: self.id,
            amount: self.amount,
            _state: PhantomData,
        }
    }
}

impl Order<Delivered> {
    /// 获取完成订单的信息
    pub fn receipt(&self) -> String {
        format!("Order {}: ${:.2} - Delivered", self.id, self.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Type State 测试 ==========
    #[test]
    fn test_type_state_connection() {
        let conn = DatabaseConnection::new("postgres://localhost/db");
        // conn.query("SELECT 1"); // 编译错误！Disconnected状态没有query方法

        let conn = conn.connect().expect("Connection failed");
        let result = conn.query("SELECT 1").expect("Query failed");
        assert_eq!(result.len(), 1);

        let conn = conn.disconnect();
        // conn.query("SELECT 1"); // 编译错误！已断开连接
    }

    #[test]
    fn test_invalid_connection_string() {
        let conn = DatabaseConnection::new("");
        assert!(matches!(conn.connect(), Err(ConnectionError::InvalidConnectionString)));
    }

    // ========== Fixed Buffer 测试 ==========
    #[test]
    fn test_fixed_buffer_capacity() {
        let mut buf = FixedBuffer::<16>::new();
        assert_eq!(buf.capacity(), 16);

        let written = buf.try_write(b"Hello, World!").expect("Write failed");
        assert_eq!(written, 13);
        assert_eq!(buf.data(), b"Hello, World!");
    }

    #[test]
    fn test_fixed_buffer_overflow_prevention() {
        let mut buf = FixedBuffer::<4>::new();
        let result = buf.try_write(b"Hello, World!");
        assert!(matches!(result, Err(BufferError::InsufficientSpace)));
    }

    // ========== Verified Email 测试 ==========
    #[test]
    fn test_valid_email() {
        let email = VerifiedEmail::parse("user@example.com").expect("Valid email");
        assert_eq!(email.as_str(), "user@example.com");
    }

    #[test]
    fn test_invalid_email_no_at() {
        assert!(matches!(
            VerifiedEmail::parse("invalid.email"),
            Err(EmailError::MissingAtSymbol)
        ));
    }

    #[test]
    fn test_invalid_email_starts_with_at() {
        assert!(matches!(
            VerifiedEmail::parse("@example.com"),
            Err(EmailError::InvalidFormat)
        ));
    }

    // ========== Linear Resource 测试 ==========
    #[test]
    fn test_linear_resource_consumed() {
        let resource = LinearResource::new(42, "test_resource");
        assert!(!resource.is_consumed());

        let value = resource.consume();
        assert_eq!(value, 42);
    }

    #[test]
    #[should_panic(expected = "was dropped without being consumed")]
    fn test_linear_resource_not_consumed_panics() {
        let _resource = LinearResource::new(42, "leaked_resource");
        // 未调用consume，drop时会panic
    }

    // ========== Order State Machine 测试 ==========
    #[test]
    fn test_order_lifecycle() {
        let order = Order::new(1, 100.0);
        let order = order.pay();
        let order = order.ship();
        let order = order.deliver();

        assert_eq!(order.receipt(), "Order 1: $100.00 - Delivered");
    }

    // 以下代码如果取消注释会导致编译错误，证明状态转换被正确限制：
    // #[test]
    // fn test_invalid_transitions() {
    //     let order = Order::new(1, 100.0);
    //     let order = order.ship(); // 编译错误！Pending没有ship方法
    // }
}
