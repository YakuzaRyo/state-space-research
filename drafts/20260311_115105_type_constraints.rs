//! Type-Constrained Code Generation Research
//!
//! 核心问题: 类型系统如何指导代码生成?
//!
//! 假设: Rust的类型系统可以通过以下机制指导代码生成:
//! 1. Derive宏 - 从类型结构自动生成实现代码
//! 2. Typestate模式 - 类型编码状态,编译时保证状态转换正确
//! 3. Phantom类型 - 零成本运行时抽象,编译时强制执行约束
//! 4. Const泛型 - 编译时计算和类型级约束

use std::marker::PhantomData;

// ============================================================================
// 1. Typestate模式 - 类型指导状态机代码生成
// ============================================================================

/// 未初始化状态标记
pub struct Uninitialized;
/// 已配置状态标记
pub struct Configured;
/// 运行中状态标记
pub struct Running;

/// Typestate模式: DatabaseConnection的类型参数编码其运行时状态
/// 这指导了哪些方法可用,防止非法状态转换
pub struct DatabaseConnection<State> {
    config: Option<DbConfig>,
    connection_string: Option<String>,
    _state: PhantomData<State>,
}

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
}

/// 只在Uninitialized状态下可用的方法
impl DatabaseConnection<Uninitialized> {
    pub fn new() -> Self {
        DatabaseConnection {
            config: None,
            connection_string: None,
            _state: PhantomData,
        }
    }

    /// 状态转换: Uninitialized -> Configured
    /// 消费self防止旧状态重用
    pub fn configure(self, config: DbConfig) -> DatabaseConnection<Configured> {
        let conn_str = format!(
            "postgresql://{}:{}/{}",
            config.host, config.port, config.database
        );
        DatabaseConnection {
            config: Some(config),
            connection_string: Some(conn_str),
            _state: PhantomData,
        }
    }
}

/// 只在Configured状态下可用的方法
impl DatabaseConnection<Configured> {
    pub fn get_connection_string(&self) -> &str {
        self.connection_string.as_ref().unwrap()
    }

    /// 状态转换: Configured -> Running
    pub fn connect(self) -> DatabaseConnection<Running> {
        println!("Connecting to database...");
        DatabaseConnection {
            config: self.config,
            connection_string: self.connection_string,
            _state: PhantomData,
        }
    }
}

/// 只在Running状态下可用的方法
impl DatabaseConnection<Running> {
    pub fn query(&self, sql: &str) -> Vec<String> {
        println!("Executing query: {}", sql);
        vec!["result1".to_string(), "result2".to_string()]
    }

    pub fn execute(&self, sql: &str) -> usize {
        println!("Executing: {}", sql);
        1
    }
}

// ============================================================================
// 2. Phantom类型 - 编译时单位检查(指导安全代码生成)
// ============================================================================

/// 距离单位标记
pub struct Meters;
/// 时间单位标记
pub struct Seconds;
/// 速度单位标记(由类型系统推导)
pub struct MetersPerSecond;

/// 带单位的值 - 类型系统防止单位混淆
pub struct Quantity<T, Unit> {
    value: T,
    _unit: PhantomData<Unit>,
}

impl<T: Copy> Quantity<T, Meters> {
    pub fn new_meters(value: T) -> Self {
        Quantity {
            value,
            _unit: PhantomData,
        }
    }

    pub fn value(&self) -> T {
        self.value
    }
}

impl<T: Copy> Quantity<T, Seconds> {
    pub fn new_seconds(value: T) -> Self {
        Quantity {
            value,
            _unit: PhantomData,
        }
    }

    pub fn value(&self) -> T {
        self.value
    }
}

/// 类型系统指导的速度计算 - 编译时保证单位正确
impl Quantity<f64, Meters> {
    /// 计算速度: m / s = m/s
    pub fn divide_by(self, time: Quantity<f64, Seconds>) -> Quantity<f64, MetersPerSecond> {
        Quantity {
            value: self.value / time.value,
            _unit: PhantomData,
        }
    }
}

impl Quantity<f64, MetersPerSecond> {
    pub fn value(&self) -> f64 {
        self.value
    }
}

// ============================================================================
// 3. 类型约束的Builder模式 - 强制字段初始化
// ============================================================================

/// 未设置标记
pub struct NotSet;
/// 已设置标记
pub struct Set<T>(T);

/// 类型约束的HTTP请求Builder
/// 类型参数强制必须设置某些字段才能构建
pub struct HttpRequestBuilder<Url, Method> {
    url: Url,
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl HttpRequestBuilder<NotSet, NotSet> {
    pub fn new() -> Self {
        HttpRequestBuilder {
            url: NotSet,
            method: NotSet,
            headers: Vec::new(),
            body: None,
        }
    }
}

impl<Method> HttpRequestBuilder<NotSet, Method> {
    /// 设置URL - 类型变为Set<String>
    pub fn url(self, url: impl Into<String>) -> HttpRequestBuilder<Set<String>, Method> {
        HttpRequestBuilder {
            url: Set(url.into()),
            method: self.method,
            headers: self.headers,
            body: self.body,
        }
    }
}

impl<Url> HttpRequestBuilder<Url, NotSet> {
    /// 设置Method - 类型变为Set<String>
    pub fn method(self, method: impl Into<String>) -> HttpRequestBuilder<Url, Set<String>> {
        HttpRequestBuilder {
            url: self.url,
            method: Set(method.into()),
            headers: self.headers,
            body: self.body,
        }
    }
}

impl<Url, Method> HttpRequestBuilder<Url, Method> {
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// 只有Url和Method都设置后才能build
impl HttpRequestBuilder<Set<String>, Set<String>> {
    pub fn build(self) -> HttpRequest {
        HttpRequest {
            url: self.url.0,
            method: self.method.0,
            headers: self.headers,
            body: self.body,
        }
    }
}

#[derive(Debug)]
pub struct HttpRequest {
    url: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

// ============================================================================
// 4. Const泛型 - 编译时大小约束指导代码生成
// ============================================================================

/// Const泛型固定大小缓冲区 - 编译时知道大小,指导内存布局
pub struct FixedBuffer<T, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T: Default + Copy, const N: usize> FixedBuffer<T, N> {
    pub fn new() -> Self {
        FixedBuffer {
            data: [T::default(); N],
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), &'static str> {
        if self.len >= N {
            return Err("Buffer full");
        }
        self.data[self.len] = value;
        self.len += 1;
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(&self.data[index])
        } else {
            None
        }
    }

    /// 编译时已知容量
    pub const fn capacity(&self) -> usize {
        N
    }
}

/// 类型级矩阵运算 - 维度在类型中编码
pub struct Matrix<T, const ROWS: usize, const COLS: usize> {
    data: [[T; COLS]; ROWS],
}

impl<T: Default + Copy, const ROWS: usize, const COLS: usize> Matrix<T, ROWS, COLS> {
    pub fn new() -> Self {
        Matrix {
            data: [[T::default(); COLS]; ROWS],
        }
    }

    pub fn set(&mut self, row: usize, col: usize, value: T) {
        if row < ROWS && col < COLS {
            self.data[row][col] = value;
        }
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        if row < ROWS && col < COLS {
            Some(&self.data[row][col])
        } else {
            None
        }
    }
}

/// 矩阵乘法 - 类型系统保证维度兼容
/// MxN * NxP -> MxP
impl<T: Default + Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T>, const M: usize, const N: usize, const P: usize>
    Matrix<T, M, N>
{
    pub fn multiply(&self, other: &Matrix<T, N, P>) -> Matrix<T, M, P> {
        let mut result = Matrix::new();
        for i in 0..M {
            for j in 0..P {
                let mut sum = T::default();
                for k in 0..N {
                    sum = sum + self.data[i][k] * other.data[k][j];
                }
                result.data[i][j] = sum;
            }
        }
        result
    }
}

// ============================================================================
// 5. 类型指导的验证器生成
// ============================================================================

/// 验证通过的标记类型
pub struct Validated;
/// 未验证的标记类型
pub struct Unvalidated;

/// 类型编码验证状态
pub struct Email<State = Unvalidated> {
    address: String,
    _state: PhantomData<State>,
}

impl Email<Unvalidated> {
    pub fn new(address: impl Into<String>) -> Self {
        Email {
            address: address.into(),
            _state: PhantomData,
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// 验证转换 - 只有通过验证才能进入Validated状态
    pub fn validate(self) -> Result<Email<Validated>, ValidationError> {
        if self.address.contains('@') && self.address.contains('.') {
            Ok(Email {
                address: self.address,
                _state: PhantomData,
            })
        } else {
            Err(ValidationError {
                message: "Invalid email format".to_string(),
            })
        }
    }
}

impl Email<Validated> {
    /// 只有Validated状态的Email才能发送
    pub fn send(&self, content: &str) {
        println!("Sending email to {}: {}", self.address, content);
    }
}

#[derive(Debug)]
pub struct ValidationError {
    message: String,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_database_connection() {
        // 类型状态转换流程
        let conn = DatabaseConnection::new();
        let conn = conn.configure(DbConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
        });
        assert_eq!(conn.get_connection_string(), "postgresql://localhost:5432/test");

        let conn = conn.connect();
        let results = conn.query("SELECT * FROM users");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_phantom_units() {
        let distance = Quantity::new_meters(100.0);
        let time = Quantity::new_seconds(10.0);
        let speed = distance.divide_by(time);

        // 类型系统保证: 距离/时间 = 速度
        assert!((speed.value() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_type_constrained_builder() {
        let request = HttpRequestBuilder::new()
            .url("https://api.example.com")
            .method("POST")
            .header("Content-Type", "application/json")
            .body("{}")
            .build();

        assert_eq!(request.url, "https://api.example.com");
        assert_eq!(request.method, "POST");
    }

    #[test]
    fn test_fixed_buffer() {
        let mut buf: FixedBuffer<i32, 5> = FixedBuffer::new();
        assert_eq!(buf.capacity(), 5);

        buf.push(1).unwrap();
        buf.push(2).unwrap();
        assert_eq!(buf.get(0), Some(&1));
        assert_eq!(buf.get(1), Some(&2));

        // 满后push返回错误
        buf.push(3).unwrap();
        buf.push(4).unwrap();
        buf.push(5).unwrap();
        assert!(buf.push(6).is_err());
    }

    #[test]
    fn test_matrix_multiply() {
        let mut m1: Matrix<i32, 2, 3> = Matrix::new();
        m1.set(0, 0, 1);
        m1.set(0, 1, 2);
        m1.set(0, 2, 3);
        m1.set(1, 0, 4);
        m1.set(1, 1, 5);
        m1.set(1, 2, 6);

        let mut m2: Matrix<i32, 3, 2> = Matrix::new();
        m2.set(0, 0, 7);
        m2.set(0, 1, 8);
        m2.set(1, 0, 9);
        m2.set(1, 1, 10);
        m2.set(2, 0, 11);
        m2.set(2, 1, 12);

        // 2x3 * 3x2 = 2x2
        let result: Matrix<i32, 2, 2> = m1.multiply(&m2);
        assert_eq!(result.get(0, 0), Some(&58));  // 1*7 + 2*9 + 3*11
        assert_eq!(result.get(0, 1), Some(&64));  // 1*8 + 2*10 + 3*12
        assert_eq!(result.get(1, 0), Some(&139)); // 4*7 + 5*9 + 6*11
        assert_eq!(result.get(1, 1), Some(&154)); // 4*8 + 5*10 + 6*12
    }

    #[test]
    fn test_email_validation() {
        let email = Email::new("test@example.com");
        assert_eq!(email.address(), "test@example.com");

        let validated = email.validate().unwrap();
        validated.send("Hello!");

        let invalid = Email::new("invalid-email");
        assert!(invalid.validate().is_err());
    }
}
