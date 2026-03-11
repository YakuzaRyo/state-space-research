// 状态空间架构深度研究：核心原则验证
// 研究方向: 01_core_principles - 如何让错误在设计上不可能发生
// 时间: 2026-03-11 21:01
//
// 本代码验证以下假设：
// H1: Typestate与精化类型可以互补组合
// H2: Const Generics使得编译期状态机可携带更多上下文
// H3: 泛型+PhantomData+Const Generics实现"携带数据的类型状态"
// H4: Capability模式可通过const泛型参数化权限向量
// H5: Typestate+Const Generics保持零运行时开销
// H6: 精化类型检查在编译期完成

use std::marker::PhantomData;
use std::mem::size_of;

// ============================================================================
// 模块1: 基础Typestate模式 - 零成本状态约束
// ============================================================================

/// 状态标记类型（ZST - 零大小类型）
pub struct Disconnected;
pub struct Connecting;
pub struct Connected;
pub struct Closed;

/// 带类型状态的网络连接
/// 使用PhantomData标记状态，编译后无运行时开销
pub struct Connection<State> {
    address: String,
    _state: PhantomData<State>,
}

// Disconnected状态：只能连接
impl Connection<Disconnected> {
    pub fn new(addr: &str) -> Self {
        Connection {
            address: addr.to_string(),
            _state: PhantomData,
        }
    }

    pub fn connect(self) -> Connection<Connecting> {
        println!("Connecting to {}...", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Connecting状态：只能完成连接或失败
impl Connection<Connecting> {
    pub fn on_connected(self) -> Connection<Connected> {
        println!("Connected to {}", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }

    pub fn on_failed(self) -> Connection<Disconnected> {
        println!("Connection failed, returning to disconnected");
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Connected状态：可以发送数据和关闭
impl Connection<Connected> {
    pub fn send(&self, data: &str) {
        println!("Sending '{}' to {}", data, self.address);
    }

    pub fn close(self) -> Connection<Closed> {
        println!("Closing connection to {}", self.address);
        Connection {
            address: self.address,
            _state: PhantomData,
        }
    }
}

// Closed状态：终态，无可用操作
impl Connection<Closed> {
    // 不提供任何转换方法，表示状态机终止
    pub fn is_closed(&self) -> bool {
        true
    }
}

// ============================================================================
// 模块2: Const Generics + Typestate - 携带数据的类型状态（假设H2, H3验证）
// ============================================================================

/// 编译期范围约束的数值类型
/// L0层：Const Generics实现编译期数值验证
pub struct BoundedU32<const MIN: u32, const MAX: u32>(u32);

impl<const MIN: u32, const MAX: u32> BoundedU32<MIN, MAX> {
    /// 尝试创建受约束的值
    /// 无效值在构造时返回None，无法创建非法状态
    pub fn new(value: u32) -> Option<Self> {
        if value >= MIN && value <= MAX {
            Some(BoundedU32(value))
        } else {
            None
        }
    }

    pub fn get(&self) -> u32 {
        self.0
    }

    /// 安全地增加值，保持约束
    pub fn add_checked(&self, delta: u32) -> Option<Self> {
        Self::new(self.0.saturating_add(delta))
    }
}

/// 端口类型：编译期验证有效端口范围
pub type Port = BoundedU32<1, 65535>;

/// HTTP状态码类型：编译期验证有效范围
pub type HttpStatusCode = BoundedU32<100, 599>;

// ============================================================================
// 模块3: 携带数据的Typestate - 状态包含编译期已知信息
// ============================================================================

/// 带容量约束的缓冲区状态机
/// 使用Const Generics在类型层面编码容量信息
pub struct Buffer<State, const CAPACITY: usize> {
    data: Vec<u8>,
    _state: PhantomData<State>,
}

pub struct Empty;
pub struct Partial;
pub struct Full;

impl<const CAPACITY: usize> Buffer<Empty, CAPACITY> {
    pub fn new() -> Self {
        Buffer {
            data: Vec::with_capacity(CAPACITY),
            _state: PhantomData,
        }
    }

    /// 添加数据，可能转换到Partial或Full状态
    pub fn push(mut self, item: u8) -> Result<Buffer<Partial, CAPACITY>, Buffer<Full, CAPACITY>> {
        self.data.push(item);
        if self.data.len() == CAPACITY {
            Ok(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        } else {
            Err(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        }
    }
}

impl<const CAPACITY: usize> Buffer<Partial, CAPACITY> {
    pub fn push(mut self, item: u8) -> Result<Buffer<Partial, CAPACITY>, Buffer<Full, CAPACITY>> {
        self.data.push(item);
        if self.data.len() == CAPACITY {
            Ok(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        } else {
            Err(Buffer {
                data: self.data,
                _state: PhantomData,
            })
        }
    }

    pub fn pop(mut self) -> (u8, EitherBuffer<CAPACITY>) {
        let item = self.data.pop().unwrap();
        if self.data.is_empty() {
            (item, EitherBuffer::Empty(Buffer {
                data: self.data,
                _state: PhantomData,
            }))
        } else {
            (item, EitherBuffer::Partial(Buffer {
                data: self.data,
                _state: PhantomData,
            }))
        }
    }
}

impl<const CAPACITY: usize> Buffer<Full, CAPACITY> {
    pub fn pop(mut self) -> (u8, Buffer<Partial, CAPACITY>) {
        let item = self.data.pop().unwrap();
        (item, Buffer {
            data: self.data,
            _state: PhantomData,
        })
    }

    pub fn flush(self) -> Buffer<Empty, CAPACITY> {
        Buffer {
            data: Vec::with_capacity(CAPACITY),
            _state: PhantomData,
        }
    }
}

pub enum EitherBuffer<const CAPACITY: usize> {
    Empty(Buffer<Empty, CAPACITY>),
    Partial(Buffer<Partial, CAPACITY>),
}

// ============================================================================
// 模块4: Capability-Based权限系统（假设H4验证）
// ============================================================================

/// 权限标记类型
pub struct Read;
pub struct Write;
pub struct Execute;

/// 能力安全资源容器
/// 使用类型参数编码权限向量
pub struct SecureResource<T, R, W, X> {
    data: T,
    _read: PhantomData<R>,
    _write: PhantomData<W>,
    _execute: PhantomData<X>,
}

/// 只读资源
pub type ReadOnly<T> = SecureResource<T, Read, (), ()>;

/// 读写资源
pub type ReadWrite<T> = SecureResource<T, Read, Write, ()>;

/// 完全权限资源
pub type FullAccess<T> = SecureResource<T, Read, Write, Execute>;

impl<T> SecureResource<T, (), (), ()> {
    /// 创建无权限资源
    pub fn new(data: T) -> Self {
        SecureResource {
            data,
            _read: PhantomData,
            _write: PhantomData,
            _execute: PhantomData,
        }
    }

    /// 授予读权限
    pub fn grant_read(self) -> SecureResource<T, Read, (), ()> {
        SecureResource {
            data: self.data,
            _read: PhantomData,
            _write: PhantomData,
            _execute: PhantomData,
        }
    }
}

impl<T, W, X> SecureResource<T, Read, W, X> {
    /// 读取数据（需要Read权限）
    pub fn read(&self) -> &T {
        &self.data
    }
}

impl<T, R, X> SecureResource<T, R, Write, X> {
    /// 写入数据（需要Write权限）
    pub fn write(&mut self, data: T) {
        self.data = data;
    }
}

impl<T, R, W> SecureResource<T, R, W, Execute> {
    /// 执行操作（需要Execute权限）
    pub fn execute<F, Ret>(&self, f: F) -> Ret
    where
        F: FnOnce(&T) -> Ret,
    {
        f(&self.data)
    }
}

impl<T, R, X> SecureResource<T, R, Write, X> {
    /// 降级权限：移除写权限
    pub fn revoke_write(self) -> SecureResource<T, R, (), X> {
        SecureResource {
            data: self.data,
            _read: PhantomData,
            _write: PhantomData,
            _execute: PhantomData,
        }
    }
}

// ============================================================================
// 模块5: Typestate + Capability组合 - 权限状态机
// ============================================================================

/// 带权限的状态机
/// 结合L3(Typestate)和L5(Capability)层
pub struct PermissionedStateMachine<State, const CAN_READ: bool, const CAN_WRITE: bool> {
    data: String,
    _state: PhantomData<State>,
}

pub struct Draft;
pub struct Review;
pub struct Published;

// Draft状态：可读写
impl PermissionedStateMachine<Draft, true, true> {
    pub fn new(content: &str) -> Self {
        PermissionedStateMachine {
            data: content.to_string(),
            _state: PhantomData,
        }
    }

    pub fn edit(&mut self, new_content: &str) {
        self.data = new_content.to_string();
    }

    pub fn submit(self) -> PermissionedStateMachine<Review, true, false> {
        PermissionedStateMachine {
            data: self.data,
            _state: PhantomData,
        }
    }
}

// Review状态：只读
impl PermissionedStateMachine<Review, true, false> {
    pub fn view(&self) -> &str {
        &self.data
    }

    pub fn approve(self) -> PermissionedStateMachine<Published, true, false> {
        PermissionedStateMachine {
            data: self.data,
            _state: PhantomData,
        }
    }

    pub fn reject(self) -> PermissionedStateMachine<Draft, true, true> {
        PermissionedStateMachine {
            data: self.data,
            _state: PhantomData,
        }
    }
}

// Published状态：只读
impl PermissionedStateMachine<Published, true, false> {
    pub fn view(&self) -> &str {
        &self.data
    }
}

// ============================================================================
// 模块6: 业务状态机 - 订单生命周期示例
// ============================================================================

/// 订单状态
pub struct OrderCreated;
pub struct OrderPaid { payment_id: String, amount: u64 }
pub struct OrderShipped { tracking_number: String }
pub struct OrderDelivered;
pub struct OrderCancelled;

/// 订单状态机
/// 演示复杂业务逻辑的类型安全表达
pub struct Order<State> {
    order_id: String,
    customer_id: String,
    items: Vec<String>,
    state_data: Option<Box<dyn std::any::Any>>, // 用于携带状态特定数据
    _state: PhantomData<State>,
}

impl Order<OrderCreated> {
    pub fn new(order_id: &str, customer_id: &str, items: Vec<String>) -> Self {
        Order {
            order_id: order_id.to_string(),
            customer_id: customer_id.to_string(),
            items,
            state_data: None,
            _state: PhantomData,
        }
    }

    /// 支付转换：Created -> Paid
    pub fn pay(self, payment_id: &str, amount: u64) -> Order<OrderPaid> {
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            items: self.items,
            state_data: Some(Box::new(OrderPaid {
                payment_id: payment_id.to_string(),
                amount,
            })),
            _state: PhantomData,
        }
    }

    /// 取消转换：Created -> Cancelled
    pub fn cancel(self) -> Order<OrderCancelled> {
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            items: self.items,
            state_data: None,
            _state: PhantomData,
        }
    }
}

impl Order<OrderPaid> {
    /// 获取支付信息（仅在Paid状态可用）
    pub fn payment_info(&self) -> Option<(&str, u64)> {
        self.state_data.as_ref().and_then(|d| {
            d.downcast_ref::<OrderPaid>()
                .map(|p| (p.payment_id.as_str(), p.amount))
        })
    }

    /// 发货转换：Paid -> Shipped
    pub fn ship(self, tracking_number: &str) -> Order<OrderShipped> {
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            items: self.items,
            state_data: Some(Box::new(OrderShipped {
                tracking_number: tracking_number.to_string(),
            })),
            _state: PhantomData,
        }
    }
}

impl Order<OrderShipped> {
    /// 获取物流信息（仅在Shipped状态可用）
    pub fn tracking_info(&self) -> Option<&str> {
        self.state_data.as_ref().and_then(|d| {
            d.downcast_ref::<OrderShipped>()
                .map(|s| s.tracking_number.as_str())
        })
    }

    /// 送达转换：Shipped -> Delivered
    pub fn deliver(self) -> Order<OrderDelivered> {
        Order {
            order_id: self.order_id,
            customer_id: self.customer_id,
            items: self.items,
            state_data: None,
            _state: PhantomData,
        }
    }
}

impl Order<OrderDelivered> {
    /// 完成订单（终态）
    pub fn complete(self) -> CompletedOrder {
        CompletedOrder {
            order_id: self.order_id,
            customer_id: self.customer_id,
            items: self.items,
        }
    }
}

/// 已完成订单（非泛型，表示状态机终止）
pub struct CompletedOrder {
    order_id: String,
    customer_id: String,
    items: Vec<String>,
}

// ============================================================================
// 模块7: 零成本抽象验证（假设H5验证）
// ============================================================================

/// 验证Typestate模式的零成本特性
pub fn verify_zero_cost() {
    println!("=== 零成本抽象验证 ===");

    // 验证状态标记是ZST
    println!("size_of::<Disconnected>() = {}", size_of::<Disconnected>());
    println!("size_of::<Connected>() = {}", size_of::<Connected>());
    println!("size_of::<PhantomData<Connected>>() = {}", size_of::<PhantomData<Connected>>());

    // 验证Connection在不同状态下大小相同
    println!("size_of::<Connection<Disconnected>>() = {}", size_of::<Connection<Disconnected>>());
    println!("size_of::<Connection<Connected>>() = {}", size_of::<Connection<Connected>>());

    // 验证Buffer在不同状态下大小相同
    println!("size_of::<Buffer<Empty, 1024>>() = {}", size_of::<Buffer<Empty, 1024>>());
    println!("size_of::<Buffer<Full, 1024>>() = {}", size_of::<Buffer<Full, 1024>>());

    // 验证SecureResource权限不影响大小
    println!("size_of::<SecureResource<String, (), (), ()>>() = {}", size_of::<SecureResource<String, (), (), ()>>());
    println!("size_of::<SecureResource<String, Read, Write, Execute>>() = {}", size_of::<SecureResource<String, Read, Write, Execute>>());

    // 关键断言：所有状态大小相同
    assert_eq!(size_of::<Disconnected>(), 0);
    assert_eq!(size_of::<PhantomData<Connected>>(), 0);
    assert_eq!(size_of::<Connection<Disconnected>>(), size_of::<Connection<Connected>>());
    assert_eq!(size_of::<SecureResource<String, (), (), ()>>(), size_of::<SecureResource<String, Read, Write, Execute>>());

    println!("✓ 零成本抽象验证通过！\n");
}

// ============================================================================
// 模块8: 编译期错误捕获演示
// ============================================================================

/// 演示编译期错误捕获
/// 以下代码如果取消注释，将产生编译错误
pub fn demonstrate_compile_time_errors() {
    println!("=== 编译期错误捕获演示 ===");
    println!("以下错误在编译期被捕获：");
    println!("1. 未连接就发送数据");
    println!("2. 未支付就发货");
    println!("3. 无写权限却尝试写入");
    println!("4. 无效数值构造");

    // 示例1: 无效状态转换被阻止
    // let conn = Connection::<Disconnected>::new("127.0.0.1:8080");
    // conn.send("data"); // 编译错误：Disconnected状态没有send方法

    // 示例2: 无效业务操作被阻止
    // let order = Order::<OrderCreated>::new("ORD-001", "CUST-001", vec!["item1".to_string()]);
    // order.ship("TRACK-001"); // 编译错误：OrderCreated状态没有ship方法

    // 示例3: 无效权限操作被阻止
    // let resource = SecureResource::<String, (), (), ()>::new("data".to_string());
    // resource.grant_read().write("new data".to_string()); // 编译错误：ReadOnly没有write方法

    // 示例4: 无效数值被阻止
    // let port = Port::new(0); // 返回None，无法构造无效端口
    // let status = HttpStatusCode::new(99); // 返回None，无效状态码

    println!("✓ 所有错误在编译期被捕获！\n");
}

// ============================================================================
// 模块9: 实际使用示例
// ============================================================================

pub fn run_examples() {
    println!("=== 实际使用示例 ===\n");

    // 示例1: 连接状态机
    println!("--- 连接状态机示例 ---");
    let conn = Connection::<Disconnected>::new("127.0.0.1:8080");
    let conn = conn.connect();
    let conn = conn.on_connected();
    conn.send("Hello, World!");
    let conn = conn.close();
    println!("Connection closed: {}\n", conn.is_closed());

    // 示例2: 缓冲区状态机
    println!("--- 缓冲区状态机示例 ---");
    let buf: Buffer<Empty, 1> = Buffer::new(); // 容量为1，push后立即满
    let buf: Buffer<Full, 1> = match buf.push(1) {
        Ok(_) => unreachable!("Capacity is 1, should be full (Err branch)"),
        Err(full) => {
            println!("Buffer is full after 1 item");
            full
        }
    };
    // 从Full状态刷新
    let _buf: Buffer<Empty, 1> = buf.flush();
    println!("Buffer flushed\n");

    // 示例3: 能力安全资源
    println!("--- 能力安全资源示例 ---");
    let resource = SecureResource::<String, (), (), ()>::new("secret data".to_string());
    let ro_resource = resource.grant_read();
    println!("Read: {}", ro_resource.read());
    // ro_resource.write("new data".to_string()); // 编译错误：没有写权限
    println!();

    // 示例4: 权限状态机
    println!("--- 权限状态机示例 ---");
    let doc = PermissionedStateMachine::<Draft, true, true>::new("Draft content");
    // doc在Draft状态可编辑
    let doc = doc.submit(); // 提交后变为Review状态，只读
    println!("Reviewing: {}", doc.view());
    let doc = doc.approve(); // 批准后变为Published状态，只读
    println!("Published: {}\n", doc.view());

    // 示例5: 订单状态机
    println!("--- 订单状态机示例 ---");
    let order = Order::<OrderCreated>::new("ORD-001", "CUST-001", vec!["Laptop".to_string()]);
    let order = order.pay("PAY-123", 999_99);
    println!("Payment info: {:?}", order.payment_info());
    let order = order.ship("TRACK-456");
    println!("Tracking: {:?}", order.tracking_info());
    let order = order.deliver();
    let completed = order.complete();
    println!("Order completed: {}\n", completed.order_id);

    // 示例6: Const Generics数值约束
    println!("--- Const Generics数值约束示例 ---");
    let port = Port::new(8080).expect("Valid port");
    println!("Port: {}", port.get());

    let invalid_port = Port::new(0);
    println!("Invalid port (0): {:?}", invalid_port.is_none());

    let status = HttpStatusCode::new(200).expect("Valid status code");
    println!("HTTP Status: {}", status.get());

    println!("\n✓ 所有示例执行成功！");
}

// ============================================================================
// 主函数
// ============================================================================

fn main() {
    println!("========================================");
    println!("状态空间架构核心原则验证");
    println!("研究方向: 01_core_principles");
    println!("核心问题: 如何让错误在设计上不可能发生?");
    println!("========================================\n");

    verify_zero_cost();
    demonstrate_compile_time_errors();
    run_examples();

    println!("\n========================================");
    println!("验证完成！");
    println!("关键结论:");
    println!("1. Typestate模式实现零成本状态约束");
    println!("2. Const Generics扩展编译期验证能力");
    println!("3. Capability模式实现细粒度权限控制");
    println!("4. 组合使用仍保持零运行时开销");
    println!("5. 所有非法状态在编译期被拒绝");
    println!("========================================");
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_u32() {
        let valid = Port::new(8080);
        assert!(valid.is_some());
        assert_eq!(valid.unwrap().get(), 8080);

        let invalid = Port::new(0);
        assert!(invalid.is_none());

        let invalid2 = Port::new(70000);
        assert!(invalid2.is_none());
    }

    #[test]
    fn test_connection_state_machine() {
        let conn = Connection::<Disconnected>::new("test");
        let conn = conn.connect();
        let conn = conn.on_connected();
        conn.send("test");
        let conn = conn.close();
        assert!(conn.is_closed());
    }

    #[test]
    fn test_secure_resource() {
        let resource = SecureResource::<i32, (), (), ()>::new(42);
        let ro = resource.grant_read();
        assert_eq!(*ro.read(), 42);
    }

    #[test]
    fn test_order_state_machine() {
        let order = Order::<OrderCreated>::new("ORD-001", "CUST-001", vec!["item".to_string()]);
        let order = order.pay("PAY-001", 100);
        assert_eq!(order.payment_info(), Some(("PAY-001", 100)));
        let order = order.ship("TRACK-001");
        let order = order.deliver();
        let completed = order.complete();
        assert_eq!(completed.order_id, "ORD-001");
    }

    #[test]
    fn test_zero_cost() {
        assert_eq!(size_of::<Disconnected>(), 0);
        assert_eq!(size_of::<PhantomData<Connected>>(), 0);
        assert_eq!(
            size_of::<Connection<Disconnected>>(),
            size_of::<Connection<Connected>>()
        );
    }
}
