//! 核心原则研究：如何让错误在设计上不可能发生
//! 研究方向: 01_core_principles
//! 时间: 2026-03-11 22:05
//!
//! 本代码验证以下假设：
//! 1. 技术假设: Typestate模式与精化类型(Refinement Types)结合可实现更强的编译期保证
//! 2. 实现假设: Rust的所有权系统+PhantomData可实现零成本状态机
//! 3. 性能假设: 编译期约束不增加运行时开销
//! 4. 适用性假设: 适用于协议验证、资源管理、业务状态机

use std::marker::PhantomData;
use std::mem::size_of;

// ============================================================================
// 第一部分: 基础Typestate模式 - 连接状态机
// ============================================================================

/// 状态标记类型 - 零大小类型(ZST)
pub struct Disconnected;
pub struct Connecting;
pub struct Connected;
pub struct Closed;

/// 带类型状态的连接
/// State泛型参数在编译期编码连接状态
pub struct Connection<State> {
    address: String,
    _state: PhantomData<State>,
}

// Disconnected状态下的操作
impl Connection<Disconnected> {
    pub fn new(address: &str) -> Self {
        Connection {
            address: address.to_string(),
            _state: PhantomData,
        }
    }

    /// 状态转换: Disconnected -> Connecting
    /// 消耗self防止重复连接
    pub fn connect(self) -> Connection<Connecting> {
        println!("Connecting to {}...", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Connecting状态下的操作
impl Connection<Connecting> {
    /// 状态转换: Connecting -> Connected
    pub fn establish(self) -> Connection<Connected> {
        println!("Connected to {}", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }

    /// 连接失败处理
    pub fn fail(self) -> Connection<Disconnected> {
        println!("Connection failed, returning to disconnected state");
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Connected状态下的操作
impl Connection<Connected> {
    /// 只能在Connected状态下发送数据
    pub fn send(&self, data: &str) {
        println!("Sending '{}' to {}", data, self.address);
    }

    /// 只能在Connected状态下接收数据
    pub fn receive(&self) -> String {
        format!("Data from {}", self.address)
    }

    /// 状态转换: Connected -> Closed
    pub fn close(self) -> Connection<Closed> {
        println!("Closing connection to {}", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Closed状态 - 终态，不提供任何转换方法
impl Connection<Closed> {
    /// 只能获取地址信息，无法重新打开
    pub fn address(&self) -> &str {
        &self.address
    }
}

// ============================================================================
// 第二部分: 精化类型模拟 - 编译期数值约束
// ============================================================================

/// 编译期范围约束类型
/// MIN和MAX是const泛型参数，在编译期确定
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedU32<const MIN: u32, const MAX: u32>(u32);

impl<const MIN: u32, const MAX: u32> BoundedU32<MIN, MAX> {
    /// 尝试构造，失败时返回None
    /// 无效值在类型层面无法构造
    pub fn new(value: u32) -> Option<Self> {
        if value >= MIN && value <= MAX {
            Some(BoundedU32(value))
        } else {
            None
        }
    }

    /// 获取内部值
    pub fn get(&self) -> u32 {
        self.0
    }

    /// 安全地更新值，保持约束
    pub fn update(&mut self, new_value: u32) -> Result<(), ()> {
        if new_value >= MIN && new_value <= MAX {
            self.0 = new_value;
            Ok(())
        } else {
            Err(())
        }
    }
}

// 常用端口类型
pub type Port = BoundedU32<1, 65535>;

// 常用HTTP状态码类型
pub type HttpStatusCode = BoundedU32<100, 599>;

// ============================================================================
// 第三部分: 携带数据的Typestate - Buffer状态机
// ============================================================================

pub struct Empty;
pub struct HasData;
pub struct Full;

/// 带编译期容量约束的Buffer
/// State: 当前状态
/// CAPACITY: 编译期确定的容量
pub struct Buffer<State, const CAPACITY: usize> {
    data: Vec<u8>,
    _state: PhantomData<State>,
}

impl<const CAPACITY: usize> Buffer<Empty, CAPACITY> {
    pub fn new() -> Self {
        Buffer {
            data: Vec::with_capacity(CAPACITY),
            _state: PhantomData,
        }
    }

    /// 添加数据，状态可能变为HasData或Full
    pub fn push(mut self, item: u8) -> Result<Buffer<HasData, CAPACITY>, Buffer<Full, CAPACITY>> {
        self.data.push(item);
        if self.data.len() == CAPACITY {
            Ok(Buffer {
                data: self.data,
                _state: PhantomData,
            }.into_full())
        } else {
            Ok(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        }
    }

    fn into_full(self) -> Buffer<Full, CAPACITY> {
        Buffer {
            data: self.data,
            _state: PhantomData,
        }
    }
}

impl<const CAPACITY: usize> Buffer<HasData, CAPACITY> {
    /// 继续添加数据
    pub fn push(mut self, item: u8) -> Result<Buffer<HasData, CAPACITY>, Buffer<Full, CAPACITY>> {
        self.data.push(item);
        if self.data.len() == CAPACITY {
            Err(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        } else {
            Ok(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        }
    }

    /// 消费数据
    pub fn pop(mut self) -> (u8, Buffer<Empty, CAPACITY>) {
        let item = self.data.remove(0);
        if self.data.is_empty() {
            (item, Buffer {
                data: self.data,
                _state: PhantomData,
            })
        } else {
            (item, Buffer {
                data: self.data,
                _state: PhantomData,
            }.into_has_data())
        }
    }

    fn into_has_data(self) -> Buffer<Empty, CAPACITY> {
        Buffer {
            data: self.data,
            _state: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<const CAPACITY: usize> Buffer<Full, CAPACITY> {
    /// 只能消费数据
    pub fn pop(mut self) -> (u8, Buffer<HasData, CAPACITY>) {
        let item = self.data.remove(0);
        (item, Buffer {
            data: self.data,
            _state: PhantomData,
        })
    }

    pub fn is_full(&self) -> bool {
        true
    }
}

// ============================================================================
// 第四部分: Capability-Based权限控制
// ============================================================================

pub struct Read;
pub struct Write;
pub struct Execute;

/// 带权限标记的资源
/// R, W, X泛型参数表示读/写/执行权限
pub struct SecureResource<T, R, W, X> {
    data: T,
    _read: PhantomData<R>,
    _write: PhantomData<W>,
    _execute: PhantomData<X>,
}

// 只读资源
impl<T> SecureResource<T, Read, (), ()> {
    pub fn new_readonly(data: T) -> Self {
        SecureResource {
            data,
            _read: PhantomData,
            _write: PhantomData,
            _execute: PhantomData,
        }
    }

    pub fn read(&self) -> &T {
        &self.data
    }
}

// 读写资源
impl<T> SecureResource<T, Read, Write, ()> {
    pub fn new_readwrite(data: T) -> Self {
        SecureResource {
            data,
            _read: PhantomData,
            _write: PhantomData,
            _execute: PhantomData,
        }
    }

    pub fn read(&self) -> &T {
        &self.data
    }

    pub fn write(&mut self, data: T) {
        self.data = data;
    }
}

// 完全权限
impl<T> SecureResource<T, Read, Write, Execute> {
    pub fn new_full(data: T) -> Self {
        SecureResource {
            data,
            _read: PhantomData,
            _write: PhantomData,
            _execute: PhantomData,
        }
    }

    pub fn read(&self) -> &T {
        &self.data
    }

    pub fn write(&mut self, data: T) {
        self.data = data;
    }

    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        f(&self.data)
    }
}

// ============================================================================
// 第五部分: 业务状态机 - 订单生命周期
// ============================================================================

pub struct OrderCreated;
pub struct OrderPaid {
    amount: u64,
}
pub struct OrderShipped {
    tracking_id: String,
}
pub struct OrderDelivered;
pub struct OrderCompleted;

/// 订单状态机
pub struct Order<State> {
    order_id: String,
    customer_id: String,
    state_data: State,
}

impl Order<OrderCreated> {
    pub fn new(order_id: &str, customer_id: &str) -> Self {
        Order {
            order_id: order_id.to_string(),
            customer_id: customer_id.to_string(),
            state_data: OrderCreated,
        }
    }

    /// Created -> Paid
    pub fn pay(self, amount: u64) -> Order<OrderPaid> {
        println!("Order {} paid with amount {}", self.order_id, amount);
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            state_data: OrderPaid { amount },
        }
    }

    pub fn cancel(self) {
        println!("Order {} cancelled", self.order_id);
    }
}

impl Order<OrderPaid> {
    pub fn get_amount(&self) -> u64 {
        self.state_data.amount
    }

    /// Paid -> Shipped
    pub fn ship(self, tracking_id: &str) -> Order<OrderShipped> {
        println!("Order {} shipped with tracking {}", self.order_id, tracking_id);
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            state_data: OrderShipped {
                tracking_id: tracking_id.to_string(),
            },
        }
    }

    /// Paid -> Cancelled (退款)
    pub fn refund(self) -> Order<OrderCreated> {
        println!("Order {} refunded", self.order_id);
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            state_data: OrderCreated,
        }
    }
}

impl Order<OrderShipped> {
    pub fn get_tracking(&self) -> &str {
        &self.state_data.tracking_id
    }

    /// Shipped -> Delivered
    pub fn deliver(self) -> Order<OrderDelivered> {
        println!("Order {} delivered", self.order_id);
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            state_data: OrderDelivered,
        }
    }
}

impl Order<OrderDelivered> {
    /// Delivered -> Completed
    pub fn complete(self) -> Order<OrderCompleted> {
        println!("Order {} completed", self.order_id);
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            state_data: OrderCompleted,
        }
    }

    /// Delivered -> Returned
    pub fn return_item(self) -> Order<OrderCreated> {
        println!("Order {} returned", self.order_id);
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            state_data: OrderCreated,
        }
    }
}

impl Order<OrderCompleted> {
    pub fn archive(&self) {
        println!("Order {} archived", self.order_id);
    }
}

// ============================================================================
// 第六部分: 零成本抽象验证
// ============================================================================

/// 验证所有状态标记都是零大小类型
pub fn verify_zero_cost() {
    println!("=== Zero-Cost Abstraction Verification ===");

    // 状态标记是ZST
    assert_eq!(size_of::<Disconnected>(), 0);
    assert_eq!(size_of::<Connected>(), 0);
    assert_eq!(size_of::<PhantomData<Connected>>(), 0);

    // Connection在任意状态下大小相同
    let size_disconnected = size_of::<Connection<Disconnected>>();
    let size_connected = size_of::<Connection<Connected>>();
    assert_eq!(size_disconnected, size_connected);
    println!("Connection<Disconnected> size: {}", size_disconnected);
    println!("Connection<Connected> size: {}", size_connected);

    // SecureResource权限不影响大小
    let size_readonly = size_of::<SecureResource<String, Read, (), ()>>();
    let size_full = size_of::<SecureResource<String, Read, Write, Execute>>();
    assert_eq!(size_readonly, size_full);
    println!("SecureResource<Read> size: {}", size_readonly);
    println!("SecureResource<Read,Write,Execute> size: {}", size_full);

    // Buffer容量不影响运行时大小（仅Vec大小）
    let size_empty_10 = size_of::<Buffer<Empty, 10>>();
    let size_empty_100 = size_of::<Buffer<Empty, 100>>();
    assert_eq!(size_empty_10, size_empty_100);
    println!("Buffer<Empty, 10> size: {}", size_empty_10);
    println!("Buffer<Empty, 100> size: {}", size_empty_100);

    println!("All zero-cost assertions passed!");
}

// ============================================================================
// 第七部分: 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_lifecycle() {
        let conn = Connection::<Disconnected>::new("127.0.0.1:8080");
        let conn = conn.connect();
        let conn = conn.establish();
        conn.send("Hello");
        let _closed = conn.close();
    }

    #[test]
    fn test_bounded_u32() {
        let port = Port::new(8080).expect("Valid port");
        assert_eq!(port.get(), 8080);

        let invalid = Port::new(0);
        assert!(invalid.is_none());

        let invalid = Port::new(70000);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_buffer_state_machine() {
        let buffer: Buffer<Empty, 3> = Buffer::new();
        let buffer = buffer.push(1).unwrap();
        let buffer = buffer.push(2).unwrap();
        let buffer = buffer.push(3).expect_err("Should be full");

        let (item, buffer) = buffer.pop();
        assert_eq!(item, 1);
        let (item, buffer) = buffer.pop();
        assert_eq!(item, 2);
        let (_item, _buffer) = buffer.pop();
    }

    #[test]
    fn test_secure_resource() {
        let readonly = SecureResource::<String, Read, (), ()>::new_readonly("secret".to_string());
        assert_eq!(readonly.read(), "secret");

        let mut readwrite = SecureResource::<String, Read, Write, ()>::new_readwrite("data".to_string());
        readwrite.write("new data".to_string());
        assert_eq!(readwrite.read(), "new data");

        let full = SecureResource::<i32, Read, Write, Execute>::new_full(42);
        let result = full.execute(|x| x * 2);
        assert_eq!(result, 84);
    }

    #[test]
    fn test_order_lifecycle() {
        let order = Order::<OrderCreated>::new("ORD-001", "CUST-001");
        let order = order.pay(100);
        assert_eq!(order.get_amount(), 100);
        let order = order.ship("TRACK-123");
        assert_eq!(order.get_tracking(), "TRACK-123");
        let order = order.deliver();
        let order = order.complete();
        order.archive();
    }

    #[test]
    fn test_zero_cost() {
        verify_zero_cost();
    }
}

// ============================================================================
// 第八部分: 主函数演示
// ============================================================================

fn main() {
    println!("=== Core Principles: Making Illegal States Unrepresentable ===\n");

    // 演示连接状态机
    println!("1. Connection State Machine:");
    let conn = Connection::<Disconnected>::new("127.0.0.1:8080");
    let conn = conn.connect();
    let conn = conn.establish();
    conn.send("Hello, World!");
    let closed = conn.close();
    println!("Connection closed: {}\n", closed.address());

    // 演示精化类型
    println!("2. Refinement Types (BoundedU32):");
    if let Some(port) = Port::new(8080) {
        println!("Valid port: {}", port.get());
    }
    if Port::new(0).is_none() {
        println!("Port 0 is invalid (correctly rejected)");
    }
    if Port::new(70000).is_none() {
        println!("Port 70000 is invalid (correctly rejected)\n");
    }

    // 演示Buffer状态机
    println!("3. Buffer State Machine (Capacity = 2):");
    let buffer: Buffer<Empty, 2> = Buffer::new();
    let buffer = buffer.push(1).unwrap();
    println!("Pushed 1, current len: {}", buffer.len());
    let buffer = buffer.push(2).expect_err("Should be full");
    println!("Buffer is full");
    let (item, buffer) = buffer.pop();
    println!("Popped: {}, buffer len: {}\n", item, buffer.len());

    // 演示Capability权限
    println!("4. Capability-Based Security:");
    let readonly = SecureResource::<String, Read, (), ()>::new_readonly("secret data".to_string());
    println!("Readonly resource: {}", readonly.read());

    let mut readwrite = SecureResource::<String, Read, Write, ()>::new_readwrite("initial".to_string());
    readwrite.write("modified".to_string());
    println!("ReadWrite resource: {}", readwrite.read());

    let full = SecureResource::<i32, Read, Write, Execute>::new_full(42);
    let result = full.execute(|x| x * 2);
    println!("Execute result: {}\n", result);

    // 演示订单状态机
    println!("5. Order Lifecycle State Machine:");
    let order = Order::<OrderCreated>::new("ORD-2024-001", "CUST-123");
    let order = order.pay(199);
    println!("Order paid: ${}", order.get_amount());
    let order = order.ship("TRACK-ABC-123");
    println!("Order shipped: {}", order.get_tracking());
    let order = order.deliver();
    let order = order.complete();
    order.archive();
    println!("Order lifecycle completed\n");

    // 零成本验证
    verify_zero_cost();

    println!("\n=== All demonstrations completed successfully! ===");
}
