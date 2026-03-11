//! 类型系统指导代码生成研究
//! 研究方向: 05_type_constraints
//! 核心问题: 类型系统如何指导代码生成?

// =============================================================================
// 第一部分: Typestate模式 - 类型指导状态机实现
// =============================================================================

/// 使用类型状态模式构建HTTP请求
/// 类型指导：只有设置了URL才能设置方法，只有设置了方法才能发送
pub mod typestate_http {
    use std::marker::PhantomData;

    // 状态标记类型（零大小类型）
    pub struct Uninitialized;
    pub struct UrlSet;
    pub struct MethodSet;
    pub struct BodySet;

    /// HTTP请求构建器，使用泛型参数编码状态
    pub struct HttpRequestBuilder<State> {
        url: Option<String>,
        method: Option<String>,
        body: Option<String>,
        _state: PhantomData<State>,
    }

    // 初始状态实现
    impl HttpRequestBuilder<Uninitialized> {
        pub fn new() -> Self {
            Self {
                url: None,
                method: None,
                body: None,
                _state: PhantomData,
            }
        }

        /// 设置URL，状态转换为UrlSet
        pub fn url(mut self, url: impl Into<String>) -> HttpRequestBuilder<UrlSet> {
            self.url = Some(url.into());
            HttpRequestBuilder {
                url: self.url,
                method: self.method,
                body: self.body,
                _state: PhantomData,
            }
        }
    }

    // URL已设置状态
    impl HttpRequestBuilder<UrlSet> {
        /// 设置HTTP方法，状态转换为MethodSet
        pub fn method(mut self, method: impl Into<String>) -> HttpRequestBuilder<MethodSet> {
            self.method = Some(method.into());
            HttpRequestBuilder {
                url: self.url,
                method: self.method,
                body: self.body,
                _state: PhantomData,
            }
        }
    }

    // 方法已设置状态
    impl HttpRequestBuilder<MethodSet> {
        /// 设置请求体，状态转换为BodySet
        pub fn body(mut self, body: impl Into<String>) -> HttpRequestBuilder<BodySet> {
            self.body = Some(body.into());
            HttpRequestBuilder {
                url: self.url,
                method: self.method,
                body: self.body,
                _state: PhantomData,
            }
        }

        /// 直接发送（无请求体）
        pub fn send(self) -> Result<HttpResponse, String> {
            self.build().send()
        }

        fn build(self) -> HttpRequest {
            HttpRequest {
                url: self.url.unwrap(),
                method: self.method.unwrap(),
                body: self.body,
            }
        }
    }

    // 请求体已设置状态
    impl HttpRequestBuilder<BodySet> {
        /// 发送请求
        pub fn send(self) -> Result<HttpResponse, String> {
            self.build().send()
        }

        fn build(self) -> HttpRequest {
            HttpRequest {
                url: self.url.unwrap(),
                method: self.method.unwrap(),
                body: self.body,
            }
        }
    }

    struct HttpRequest {
        url: String,
        method: String,
        body: Option<String>,
    }

    impl HttpRequest {
        fn send(self) -> Result<HttpResponse, String> {
            Ok(HttpResponse {
                status: 200,
                body: format!("Response from {}", self.url),
            })
        }
    }

    #[derive(Debug)]
    pub struct HttpResponse {
        pub status: u16,
        pub body: String,
    }
}

// =============================================================================
// 第二部分: Const Generics - 类型级计算指导代码生成
// =============================================================================

/// 使用const generics进行编译时大小检查和代码生成
pub mod const_generics_matrix {
    use std::ops::{Add, Mul};

    /// 固定大小矩阵，尺寸在类型中编码
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Matrix<T, const ROWS: usize, const COLS: usize> {
        data: [[T; COLS]; ROWS],
    }

    impl<T: Default + Copy, const ROWS: usize, const COLS: usize> Matrix<T, ROWS, COLS> {
        /// 创建零矩阵
        pub fn zeros() -> Self {
            Self {
                data: [[T::default(); COLS]; ROWS],
            }
        }

        /// 从数组创建矩阵
        pub fn from_array(data: [[T; COLS]; ROWS]) -> Self {
            Self { data }
        }
    }

    impl<T: Default + Copy, const ROWS: usize, const COLS: usize> Matrix<T, ROWS, COLS> {
        pub fn get(&self, row: usize, col: usize) -> Option<&T> {
            self.data.get(row)?.get(col)
        }

        pub fn transpose(self) -> Matrix<T, COLS, ROWS> {
            let mut result = Matrix::zeros();
            for i in 0..ROWS {
                for j in 0..COLS {
                    result.data[j][i] = self.data[i][j];
                }
            }
            result
        }
    }

    /// 矩阵加法：要求相同维度
    impl<T: Add<Output = T> + Copy, const ROWS: usize, const COLS: usize> Add for Matrix<T, ROWS, COLS> {
        type Output = Self;

        fn add(self, rhs: Self) -> Self::Output {
            let mut result = self;
            for i in 0..ROWS {
                for j in 0..COLS {
                    result.data[i][j] = self.data[i][j] + rhs.data[i][j];
                }
            }
            result
        }
    }

    /// 矩阵乘法：类型系统确保维度匹配 (M×N) * (N×P) = (M×P)
    impl<T, const M: usize, const N: usize, const P: usize> Mul<Matrix<T, N, P>> for Matrix<T, M, N>
    where
        T: Mul<Output = T> + Add<Output = T> + Default + Copy,
    {
        type Output = Matrix<T, M, P>;

        fn mul(self, rhs: Matrix<T, N, P>) -> Self::Output {
            let mut result = Matrix::zeros();
            for i in 0..M {
                for j in 0..P {
                    let mut sum = T::default();
                    for k in 0..N {
                        sum = sum + self.data[i][k] * rhs.data[k][j];
                    }
                    result.data[i][j] = sum;
                }
            }
            result
        }
    }
}

// =============================================================================
// 第三部分: Trait约束驱动实现
// =============================================================================

/// 使用trait bounds指导序列化代码生成
pub mod trait_driven_serialization {
    use std::fmt::Write;

    /// 可序列化为JSON的trait
    pub trait ToJson {
        fn to_json(&self) -> String;
    }

    /// 为原始类型实现
    impl ToJson for i32 {
        fn to_json(&self) -> String {
            self.to_string()
        }
    }

    impl ToJson for String {
        fn to_json(&self) -> String {
            format!("\"{}\"", self.replace('"', "\\\""))
        }
    }

    impl ToJson for bool {
        fn to_json(&self) -> String {
            self.to_string()
        }
    }

    /// 为Vec<T>实现，要求T: ToJson
    impl<T: ToJson> ToJson for Vec<T> {
        fn to_json(&self) -> String {
            let items: Vec<String> = self.iter().map(|item| item.to_json()).collect();
            format!("[{}]", items.join(", "))
        }
    }

    /// 为Option<T>实现
    impl<T: ToJson> ToJson for Option<T> {
        fn to_json(&self) -> String {
            match self {
                Some(v) => v.to_json(),
                None => "null".to_string(),
            }
        }
    }

    /// 宏：为结构体生成ToJson实现
    #[macro_export]
    macro_rules! impl_to_json {
        ($struct_name:ident { $($field:ident),* }) => {
            impl ToJson for $struct_name {
                fn to_json(&self) -> String {
                    let mut result = String::new();
                    result.push('{');
                    $(
                        write!(result, "\"{}\":{},", stringify!($field), self.$field.to_json()).unwrap();
                    )*
                    // 移除最后一个逗号
                    if result.ends_with(',') {
                        result.pop();
                    }
                    result.push('}');
                    result
                }
            }
        };
    }

    #[derive(Debug)]
    pub struct Person {
        pub name: String,
        pub age: i32,
        pub active: bool,
    }

    impl_to_json!(Person { name, age, active });
}

// =============================================================================
// 第四部分: 类型级状态机（高级Typestate）
// =============================================================================

pub mod type_level_state_machine {
    use std::marker::PhantomData;

    /// 状态trait，用于密封模式
    pub trait State {
        fn name() -> &'static str;
    }

    /// 具体状态
    pub struct Idle;
    pub struct Running;
    pub struct Paused;
    pub struct Stopped;

    impl State for Idle {
        fn name() -> &'static str {
            "Idle"
        }
    }

    impl State for Running {
        fn name() -> &'static str {
            "Running"
        }
    }

    impl State for Paused {
        fn name() -> &'static str {
            "Paused"
        }
    }

    impl State for Stopped {
        fn name() -> &'static str {
            "Stopped"
        }
    }

    /// 状态机，状态作为类型参数
    pub struct StateMachine<S: State> {
        data: String,
        _state: PhantomData<S>,
    }

    impl StateMachine<Idle> {
        pub fn new(data: impl Into<String>) -> Self {
            Self {
                data: data.into(),
                _state: PhantomData,
            }
        }

        /// Idle -> Running
        pub fn start(self) -> StateMachine<Running> {
            println!("Transition: Idle -> Running");
            StateMachine {
                data: self.data,
                _state: PhantomData,
            }
        }
    }

    impl StateMachine<Running> {
        /// Running -> Paused
        pub fn pause(self) -> StateMachine<Paused> {
            println!("Transition: Running -> Paused");
            StateMachine {
                data: self.data,
                _state: PhantomData,
            }
        }

        /// Running -> Stopped
        pub fn stop(self) -> StateMachine<Stopped> {
            println!("Transition: Running -> Stopped");
            StateMachine {
                data: self.data,
                _state: PhantomData,
            }
        }

        pub fn process(&self) -> String {
            format!("Processing: {}", self.data)
        }
    }

    impl StateMachine<Paused> {
        /// Paused -> Running
        pub fn resume(self) -> StateMachine<Running> {
            println!("Transition: Paused -> Running");
            StateMachine {
                data: self.data,
                _state: PhantomData,
            }
        }

        /// Paused -> Stopped
        pub fn stop(self) -> StateMachine<Stopped> {
            println!("Transition: Paused -> Stopped");
            StateMachine {
                data: self.data,
                _state: PhantomData,
            }
        }
    }

    impl StateMachine<Stopped> {
        pub fn finalize(self) -> String {
            format!("Finalized: {}", self.data)
        }
    }
}

// =============================================================================
// 测试模块
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_http_builder() {
        use typestate_http::HttpRequestBuilder;

        let response = HttpRequestBuilder::new()
            .url("https://api.example.com")
            .method("GET")
            .send();

        assert!(response.is_ok());
        assert_eq!(response.unwrap().status, 200);
    }

    #[test]
    fn test_typestate_http_with_body() {
        use typestate_http::HttpRequestBuilder;

        let response = HttpRequestBuilder::new()
            .url("https://api.example.com")
            .method("POST")
            .body("{\"key\": \"value\"}")
            .send();

        assert!(response.is_ok());
    }

    #[test]
    fn test_const_generics_matrix() {
        use const_generics_matrix::Matrix;

        // 2x3矩阵
        let m1 = Matrix::from_array([
            [1, 2, 3],
            [4, 5, 6],
        ]);

        // 3x2矩阵
        let m2 = Matrix::from_array([
            [7, 8],
            [9, 10],
            [11, 12],
        ]);

        // 矩阵乘法：2x3 * 3x2 = 2x2
        let result: Matrix<i32, 2, 2> = m1 * m2;

        assert_eq!(result.get(0, 0), Some(&58));  // 1*7 + 2*9 + 3*11
        assert_eq!(result.get(0, 1), Some(&64));  // 1*8 + 2*10 + 3*12
        assert_eq!(result.get(1, 0), Some(&139)); // 4*7 + 5*9 + 6*11
        assert_eq!(result.get(1, 1), Some(&154)); // 4*8 + 5*10 + 6*12
    }

    #[test]
    fn test_matrix_transpose() {
        use const_generics_matrix::Matrix;

        let m = Matrix::from_array([
            [1, 2, 3],
            [4, 5, 6],
        ]);

        let transposed: Matrix<i32, 3, 2> = m.transpose();

        assert_eq!(transposed.get(0, 0), Some(&1));
        assert_eq!(transposed.get(0, 1), Some(&4));
        assert_eq!(transposed.get(2, 1), Some(&6));
    }

    #[test]
    fn test_trait_driven_json() {
        use trait_driven_serialization::{Person, ToJson};

        let person = Person {
            name: "Alice".to_string(),
            age: 30,
            active: true,
        };

        let json = person.to_json();
        assert!(json.contains("\"name\":\"Alice\""));
        assert!(json.contains("\"age\":30"));
        assert!(json.contains("\"active\":true"));
    }

    #[test]
    fn test_json_vec_serialization() {
        use trait_driven_serialization::ToJson;

        let numbers = vec![1, 2, 3];
        assert_eq!(numbers.to_json(), "[1, 2, 3]");

        let strings = vec!["hello".to_string(), "world".to_string()];
        assert_eq!(strings.to_json(), "[\"hello\", \"world\"]");
    }

    #[test]
    fn test_type_level_state_machine() {
        use type_level_state_machine::StateMachine;

        let machine = StateMachine::new("test data");
        let running = machine.start();
        assert_eq!(running.process(), "Processing: test data");

        let paused = running.pause();
        let running_again = paused.resume();
        let stopped = running_again.stop();
        let result = stopped.finalize();

        assert_eq!(result, "Finalized: test data");
    }

    #[test]
    fn test_state_machine_alternate_path() {
        use type_level_state_machine::StateMachine;

        let machine = StateMachine::new("data");
        let running = machine.start();
        let stopped = running.stop();
        let result = stopped.finalize();

        assert_eq!(result, "Finalized: data");
    }
}

// =============================================================================
// 主函数示例
// =============================================================================
fn main() {
    println!("=== 类型系统指导代码生成研究 ===\n");

    // Typestate示例
    println!("1. Typestate HTTP Builder:");
    let response = typestate_http::HttpRequestBuilder::new()
        .url("https://api.example.com")
        .method("POST")
        .body("{\"test\": \"data\"}")
        .send();
    println!("   Response: {:?}\n", response);

    // Const Generics示例
    println!("2. Const Generics Matrix:");
    use const_generics_matrix::Matrix;
    let m1 = Matrix::from_array([[1, 2], [3, 4], [5, 6]]);
    let m2 = Matrix::from_array([[7, 8, 9], [10, 11, 12]]);
    let result: Matrix<i32, 3, 3> = m1 * m2;
    println!("   3x2 * 2x3 = 3x3 matrix multiplication successful\n");

    // Trait驱动序列化
    println!("3. Trait-Driven Serialization:");
    use trait_driven_serialization::{Person, ToJson};
    let person = Person {
        name: "Bob".to_string(),
        age: 25,
        active: false,
    };
    println!("   JSON: {}\n", person.to_json());

    // 类型级状态机
    println!("4. Type-Level State Machine:");
    use type_level_state_machine::StateMachine;
    let result = StateMachine::new("workflow data")
        .start()
        .pause()
        .resume()
        .stop()
        .finalize();
    println!("   Final result: {}\n", result);

    println!("=== 所有示例完成 ===");
}
