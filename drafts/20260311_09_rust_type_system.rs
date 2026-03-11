//! Rust类型系统实现状态空间
//!
//! 核心概念：使用Typestate模式将状态编码为类型
//! 状态转换在编译期验证，无效转换会导致编译错误

use std::marker::PhantomData;

// ============================================================================
// 第一部分：基础Typestate模式 - 交通灯状态机
// ============================================================================

/// 红灯状态
pub struct Red;

/// 绿灯状态
pub struct Green;

/// 黄灯状态
pub struct Yellow;

/// 交通灯状态机
/// State类型参数编码当前状态
pub struct TrafficLight<State> {
    _state: PhantomData<State>,
}

// 为每个状态实现特定行为
impl TrafficLight<Red> {
    /// 创建新的红灯状态交通灯
    pub fn new() -> TrafficLight<Red> {
        TrafficLight {
            _state: PhantomData,
        }
    }

    /// 红灯 -> 绿灯（唯一合法转换）
    pub fn turn_green(self) -> TrafficLight<Green> {
        println!("Red -> Green");
        TrafficLight {
            _state: PhantomData,
        }
    }

    /// 红灯持续时间
    pub fn duration_secs(&self) -> u32 {
        30
    }
}

impl TrafficLight<Green> {
    /// 绿灯 -> 黄灯（唯一合法转换）
    pub fn turn_yellow(self) -> TrafficLight<Yellow> {
        println!("Green -> Yellow");
        TrafficLight {
            _state: PhantomData,
        }
    }

    /// 绿灯持续时间
    pub fn duration_secs(&self) -> u32 {
        45
    }

    /// 允许通行
    pub fn allow_pass(&self) {
        println!("Traffic can pass");
    }
}

impl TrafficLight<Yellow> {
    /// 黄灯 -> 红灯（唯一合法转换）
    pub fn turn_red(self) -> TrafficLight<Red> {
        println!("Yellow -> Red");
        TrafficLight {
            _state: PhantomData,
        }
    }

    /// 黄灯持续时间
    pub fn duration_secs(&self) -> u32 {
        5
    }

    /// 警告准备停车
    pub fn warn_stop(&self) {
        println!("Prepare to stop!");
    }
}

// ============================================================================
// 第二部分：带数据的Typestate - 文件状态机
// ============================================================================

/// 文件已打开状态
pub struct Open {
    path: String,
    content: String,
}

/// 文件已关闭状态
pub struct Closed {
    path: String,
}

/// 文件状态机
pub struct File<State> {
    state: State,
}

impl File<Closed> {
    /// 创建（关闭的）文件
    pub fn new(path: impl Into<String>) -> File<Closed> {
        File {
            state: Closed {
                path: path.into(),
            },
        }
    }

    /// 打开文件
    pub fn open(self) -> File<Open> {
        println!("Opening file: {}", self.state.path);
        File {
            state: Open {
                path: self.state.path,
                content: String::new(),
            },
        }
    }

    /// 获取路径（关闭状态也可访问）
    pub fn path(&self) -> &str {
        &self.state.path
    }
}

impl File<Open> {
    /// 读取内容
    pub fn read(&self) -> &str {
        &self.state.content
    }

    /// 写入内容
    pub fn write(&mut self, content: impl Into<String>) {
        self.state.content = content.into();
        println!("Written to file");
    }

    /// 关闭文件
    pub fn close(self) -> File<Closed> {
        println!("Closing file: {}", self.state.path);
        File {
            state: Closed {
                path: self.state.path,
            },
        }
    }

    /// 获取路径
    pub fn path(&self) -> &str {
        &self.state.path
    }
}

// ============================================================================
// 第三部分：复杂状态空间 - 订单状态机
// ============================================================================

/// 待支付
pub struct Pending;

/// 已支付
pub struct Paid {
    amount: f64,
}

/// 已发货
pub struct Shipped {
    amount: f64,
    tracking_id: String,
}

/// 已完成
pub struct Completed {
    amount: f64,
    tracking_id: String,
    delivered_at: String,
}

/// 已取消
pub struct Cancelled {
    reason: String,
}

/// 订单状态机
pub struct Order<State> {
    id: u64,
    state: State,
}

impl Order<Pending> {
    pub fn new(id: u64) -> Order<Pending> {
        Order { id, state: Pending }
    }

    /// 支付
    pub fn pay(self, amount: f64) -> Order<Paid> {
        println!("Order {} paid: ${}", self.id, amount);
        Order {
            id: self.id,
            state: Paid { amount },
        }
    }

    /// 取消
    pub fn cancel(self, reason: impl Into<String>) -> Order<Cancelled> {
        println!("Order {} cancelled: {}", self.id, reason.into());
        Order {
            id: self.id,
            state: Cancelled {
                reason: reason.into(),
            },
        }
    }
}

impl Order<Paid> {
    /// 发货
    pub fn ship(self, tracking_id: impl Into<String>) -> Order<Shipped> {
        let tracking_id = tracking_id.into();
        println!("Order {} shipped with tracking: {}", self.id, tracking_id);
        Order {
            id: self.id,
            state: Shipped {
                amount: self.state.amount,
                tracking_id,
            },
        }
    }

    /// 退款（从Paid返回Pending的特殊转换）
    pub fn refund(self) -> Order<Pending> {
        println!("Order {} refunded", self.id);
        Order {
            id: self.id,
            state: Pending,
        }
    }

    pub fn amount(&self) -> f64 {
        self.state.amount
    }
}

impl Order<Shipped> {
    /// 完成订单
    pub fn complete(self, delivered_at: impl Into<String>) -> Order<Completed> {
        println!("Order {} completed", self.id);
        Order {
            id: self.id,
            state: Completed {
                amount: self.state.amount,
                tracking_id: self.state.tracking_id,
                delivered_at: delivered_at.into(),
            },
        }
    }

    pub fn tracking_id(&self) -> &str {
        &self.state.tracking_id
    }
}

impl Order<Completed> {
    pub fn summary(&self) -> String {
        format!(
            "Order {}: ${} delivered at {}",
            self.id, self.state.amount, self.state.delivered_at
        )
    }
}

impl Order<Cancelled> {
    pub fn reason(&self) -> &str {
        &self.state.reason
    }
}

// ============================================================================
// 第四部分：状态转换图验证（编译期）
// ============================================================================

/// 状态转换标记trait
/// 用于在类型层面声明允许的状态转换
pub trait StateTransition<From, To> {
    fn validate() -> bool;
}

/// 使用宏定义允许的状态转换
#[macro_export]
macro_rules! allow_transition {
    ($from:ty => $to:ty) => {
        impl StateTransition<$from, $to> for () {
            fn validate() -> bool {
                true
            }
        }
    };
}

// 定义交通灯的有效转换
allow_transition!(Red => Green);
allow_transition!(Green => Yellow);
allow_transition!(Yellow => Red);

// ============================================================================
// 第五部分：泛型状态空间 - 状态组合
// ============================================================================

/// 连接状态
pub struct Connected;

/// 断开状态
pub struct Disconnected;

/// 认证状态
pub struct Authenticated;

/// 未认证状态
pub struct Unauthenticated;

/// 组合状态：连接 + 认证
pub struct Connection<ConnState, AuthState> {
    _conn: PhantomData<ConnState>,
    _auth: PhantomData<AuthState>,
}

impl Connection<Disconnected, Unauthenticated> {
    pub fn new() -> Connection<Disconnected, Unauthenticated> {
        Connection {
            _conn: PhantomData,
            _auth: PhantomData,
        }
    }

    pub fn connect(self) -> Connection<Connected, Unauthenticated> {
        println!("Connected");
        Connection {
            _conn: PhantomData,
            _auth: PhantomData,
        }
    }
}

impl Connection<Connected, Unauthenticated> {
    pub fn authenticate(self, token: &str) -> Connection<Connected, Authenticated> {
        println!("Authenticated with token: {}", token);
        Connection {
            _conn: PhantomData,
            _auth: PhantomData,
        }
    }

    pub fn disconnect(self) -> Connection<Disconnected, Unauthenticated> {
        println!("Disconnected");
        Connection {
            _conn: PhantomData,
            _auth: PhantomData,
        }
    }
}

impl Connection<Connected, Authenticated> {
    pub fn send_data(&self, data: &str) {
        println!("Sending data: {}", data);
    }

    pub fn logout(self) -> Connection<Connected, Unauthenticated> {
        println!("Logged out");
        Connection {
            _conn: PhantomData,
            _auth: PhantomData,
        }
    }

    pub fn disconnect(self) -> Connection<Disconnected, Unauthenticated> {
        println!("Disconnected");
        Connection {
            _conn: PhantomData,
            _auth: PhantomData,
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_light_cycle() {
        let red = TrafficLight::new();
        assert_eq!(red.duration_secs(), 30);

        let green = red.turn_green();
        assert_eq!(green.duration_secs(), 45);
        green.allow_pass();

        let yellow = green.turn_yellow();
        assert_eq!(yellow.duration_secs(), 5);
        yellow.warn_stop();

        let _red = yellow.turn_red();
    }

    #[test]
    fn test_file_state_machine() {
        let file = File::new("test.txt");
        assert_eq!(file.path(), "test.txt");

        let mut file = file.open();
        file.write("Hello, World!");
        assert_eq!(file.read(), "Hello, World!");

        let file = file.close();
        assert_eq!(file.path(), "test.txt");
    }

    #[test]
    fn test_order_lifecycle() {
        // 正常流程
        let order = Order::new(1);
        let order = order.pay(100.0);
        assert_eq!(order.amount(), 100.0);

        let order = order.ship("TRACK123");
        assert_eq!(order.tracking_id(), "TRACK123");

        let order = order.complete("2024-01-01");
        assert!(order.summary().contains("$100"));

        // 取消流程
        let order2 = Order::new(2);
        let order2 = order2.cancel("Out of stock");
        assert_eq!(order2.reason(), "Out of stock");

        // 退款流程
        let order3 = Order::new(3);
        let order3 = order3.pay(50.0);
        let _order3 = order3.refund();
    }

    #[test]
    fn test_connection_states() {
        let conn = Connection::new();
        let conn = conn.connect();
        let conn = conn.authenticate("token123");
        conn.send_data("Hello");
        let conn = conn.logout();
        let conn = conn.disconnect();
        let _conn = conn.connect();
    }

    #[test]
    fn test_state_transition_validation() {
        // 验证定义的转换
        assert!(<() as StateTransition<Red, Green>>::validate());
        assert!(<() as StateTransition<Green, Yellow>>::validate());
        assert!(<() as StateTransition<Yellow, Red>>::validate());
    }
}

// ============================================================================
// 主函数示例
// ============================================================================

fn main() {
    println!("=== Rust Type System State Space Demo ===\n");

    // 交通灯示例
    println!("--- Traffic Light ---");
    let red = TrafficLight::new();
    println!("Red light duration: {}s", red.duration_secs());
    let green = red.turn_green();
    green.allow_pass();
    let yellow = green.turn_yellow();
    yellow.warn_stop();
    let _red = yellow.turn_red();

    println!();

    // 文件示例
    println!("--- File State Machine ---");
    let file = File::new("document.txt");
    let mut file = file.open();
    file.write("Important data");
    println!("Content: {}", file.read());
    let _file = file.close();

    println!();

    // 订单示例
    println!("--- Order Lifecycle ---");
    let order = Order::new(1001);
    let order = order.pay(299.99);
    let order = order.ship("SF123456789");
    let order = order.complete("2024-03-11");
    println!("{}", order.summary());

    println!();

    // 连接示例
    println!("--- Connection States ---");
    let conn = Connection::new();
    let conn = conn.connect();
    let conn = conn.authenticate("secret_token");
    conn.send_data("Hello, Server!");
    let conn = conn.logout();
    let _conn = conn.disconnect();

    println!("\n=== All demonstrations completed successfully ===");
}
