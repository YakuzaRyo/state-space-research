//! 形式验证研究 v3 - 轻量级契约宏实现
//!
//! 研究方向: 06_formal_verification
//! 核心问题: 形式验证如何过滤LLM输出?
//!
//! 本实现展示如何在Rust中实现轻量级契约验证框架
//! 包括前置条件、后置条件和不变量的宏实现

use std::fmt::Debug;

// ============================================================================
// 第一部分: 基础契约宏实现
// ============================================================================

/// 前置条件宏 - 在函数开始时检查
///
/// # 示例
/// ```
/// requires!(x > 0, "x must be positive");
/// ```
#[macro_export]
macro_rules! requires {
    ($condition:expr, $msg:expr) => {
        if !$condition {
            panic!("Precondition failed: {} - Condition: {}", $msg, stringify!($condition));
        }
    };
    ($condition:expr) => {
        if !$condition {
            panic!("Precondition failed: {}", stringify!($condition));
        }
    };
}

/// 后置条件宏 - 在函数结束时检查返回值
///
/// # 示例
/// ```
/// let result = {
///     let __result = compute();
///     ensures!(__result >= 0, "result must be non-negative");
///     __result
/// };
/// ```
#[macro_export]
macro_rules! ensures {
    ($condition:expr, $msg:expr) => {
        if !$condition {
            panic!("Postcondition failed: {} - Condition: {}", $msg, stringify!($condition));
        }
    };
    ($condition:expr) => {
        if !$condition {
            panic!("Postcondition failed: {}", stringify!($condition));
        }
    };
}

/// 不变量宏 - 在循环或状态转换中检查
#[macro_export]
macro_rules! invariant {
    ($condition:expr, $msg:expr) => {
        if !$condition {
            panic!("Invariant violated: {} - Condition: {}", $msg, stringify!($condition));
        }
    };
    ($condition:expr) => {
        if !$condition {
            panic!("Invariant violated: {}", stringify!($condition));
        }
    };
}

// ============================================================================
// 第二部分: 契约包装器类型
// ============================================================================

/// 带有契约验证的函数包装器
///
/// 这个类型允许将任意函数包装为带有前置/后置条件验证的版本
pub struct ContractFn<T, R> {
    name: &'static str,
    f: fn(T) -> R,
}

impl<T: Debug, R: Debug> ContractFn<T, R> {
    pub fn new(name: &'static str, f: fn(T) -> R) -> Self {
        Self { name, f }
    }

    /// 执行带有前置/后置条件验证的函数
    pub fn call_with_contracts(
        &self,
        input: T,
        precondition: fn(&T) -> bool,
        postcondition: fn(&R) -> bool,
    ) -> R {
        // 检查前置条件
        if !precondition(&input) {
            panic!(
                "Precondition failed for function '{}' with input: {:?}",
                self.name, input
            );
        }

        // 执行函数
        let result = (self.f)(input);

        // 检查后置条件
        if !postcondition(&result) {
            panic!(
                "Postcondition failed for function '{}' with result: {:?}",
                self.name, result
            );
        }

        result
    }
}

// ============================================================================
// 第三部分: 契约属性宏模拟 (使用函数包装)
// ============================================================================

/// 二分查找函数 - 带有契约验证的示例
///
/// # 前置条件
/// - 数组必须是有序的
/// - 数组长度必须在合理范围内
///
/// # 后置条件
/// - 如果返回Some(index)，则arr[index] == key
/// - 如果返回None，则key不在数组中
pub fn binary_search(arr: &[i32], key: i32) -> Option<usize> {
    // 前置条件检查
    requires!(!arr.is_empty(), "array must not be empty");
    requires!(
        arr.windows(2).all(|w| w[0] <= w[1]),
        "array must be sorted"
    );

    let mut left = 0;
    let mut right = arr.len();

    while left < right {
        // 循环不变量
        invariant!(left <= right, "left must not exceed right");
        invariant!(
            left == 0 || arr[left - 1] < key,
            "left boundary invariant"
        );
        invariant!(
            right == arr.len() || arr[right] > key,
            "right boundary invariant"
        );

        let mid = left + (right - left) / 2;

        if arr[mid] == key {
            // 后置条件: 找到时arr[index] == key
            ensures!(arr[mid] == key);
            return Some(mid);
        } else if arr[mid] < key {
            left = mid + 1;
        } else {
            right = mid;
        }
    }

    // 后置条件: 未找到时key不在数组中
    // (在运行时无法完全验证，需要形式验证工具)
    None
}

/// 安全的整数除法 - 带有契约验证
///
/// # 前置条件
/// - divisor != 0 (防止除零错误)
///
/// # 后置条件
/// - result * divisor + remainder == dividend
pub fn safe_divide(dividend: i32, divisor: i32) -> (i32, i32) {
    // 前置条件
    requires!(divisor != 0, "division by zero");

    // 防止溢出检查 (i32::MIN / -1 会溢出)
    requires!(
        !(dividend == i32::MIN && divisor == -1),
        "division overflow"
    );

    let quotient = dividend / divisor;
    let remainder = dividend % divisor;

    // 后置条件验证
    ensures!(
        quotient * divisor + remainder == dividend,
        "division correctness"
    );
    ensures!(
        remainder.abs() < divisor.abs(),
        "remainder must be less than divisor"
    );

    (quotient, remainder)
}

/// 向量安全索引访问
///
/// # 前置条件
/// - index < vec.len()
///
/// # 后置条件
/// - 返回的值等于vec[index]
pub fn safe_get<T: Copy + Debug + PartialEq>(vec: &[T], index: usize) -> T {
    // 前置条件
    requires!(index < vec.len(), "index out of bounds");

    let result = vec[index];

    // 后置条件 (对于Copy类型)
    ensures!(vec[index] == result, "returned value matches vec[index]");

    result
}

// ============================================================================
// 第四部分: LLM输出验证器
// ============================================================================

/// LLM生成代码的验证结果
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// 通过所有验证
    Passed,
    /// 前置条件违反
    PreconditionViolation { condition: String, context: String },
    /// 后置条件违反
    PostconditionViolation { condition: String, actual: String },
    /// 不变量违反
    InvariantViolation { invariant: String, state: String },
    /// 运行时错误
    RuntimeError { error: String },
}

/// LLM输出验证器
///
/// 用于验证LLM生成的代码输出是否符合预期契约
pub struct LlmOutputValidator;

impl LlmOutputValidator {
    /// 验证LLM生成的数值是否在合理范围内
    pub fn validate_numeric_range(
        value: i32,
        min: i32,
        max: i32,
        context: &str,
    ) -> VerificationResult {
        if value < min || value > max {
            return VerificationResult::PostconditionViolation {
                condition: format!("value in range [{}, {}]", min, max),
                actual: format!("{} = {}", context, value),
            };
        }
        VerificationResult::Passed
    }

    /// 验证LLM生成的数组是否满足不变量
    pub fn validate_array_invariants<T: Ord + Debug>(
        arr: &[T],
        should_be_sorted: bool,
    ) -> VerificationResult {
        if should_be_sorted {
            let is_sorted = arr.windows(2).all(|w| w[0] <= w[1]);
            if !is_sorted {
                return VerificationResult::InvariantViolation {
                    invariant: "array must be sorted".to_string(),
                    state: format!("{:?}", arr),
                };
            }
        }
        VerificationResult::Passed
    }

    /// 验证LLM生成的字符串是否非空且长度合理
    pub fn validate_string(output: &str, max_len: usize) -> VerificationResult {
        if output.is_empty() {
            return VerificationResult::PostconditionViolation {
                condition: "string must not be empty".to_string(),
                actual: "empty string".to_string(),
            };
        }
        if output.len() > max_len {
            return VerificationResult::PostconditionViolation {
                condition: format!("string length <= {}", max_len),
                actual: format!("length = {}", output.len()),
            };
        }
        VerificationResult::Passed
    }
}

// ============================================================================
// 第五部分: 测试验证
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search_success() {
        let arr = vec![1, 3, 5, 7, 9];
        assert_eq!(binary_search(&arr, 5), Some(2));
        assert_eq!(binary_search(&arr, 1), Some(0));
        assert_eq!(binary_search(&arr, 9), Some(4));
    }

    #[test]
    fn test_binary_search_not_found() {
        let arr = vec![1, 3, 5, 7, 9];
        assert_eq!(binary_search(&arr, 4), None);
        assert_eq!(binary_search(&arr, 10), None);
    }

    #[test]
    #[should_panic(expected = "array must be sorted")]
    fn test_binary_search_unsorted_panic() {
        let arr = vec![5, 3, 1, 7, 9];
        binary_search(&arr, 5);
    }

    #[test]
    fn test_safe_divide_success() {
        assert_eq!(safe_divide(10, 3), (3, 1));
        assert_eq!(safe_divide(7, 2), (3, 1));
        assert_eq!(safe_divide(-10, 3), (-3, -1));
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_safe_divide_by_zero() {
        safe_divide(10, 0);
    }

    #[test]
    fn test_safe_get_success() {
        let vec = vec![10, 20, 30, 40, 50];
        assert_eq!(safe_get(&vec, 0), 10);
        assert_eq!(safe_get(&vec, 4), 50);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_safe_get_out_of_bounds() {
        let vec = vec![10, 20, 30];
        safe_get(&vec, 5);
    }

    #[test]
    fn test_llm_validator_numeric() {
        assert_eq!(
            LlmOutputValidator::validate_numeric_range(50, 0, 100, "score"),
            VerificationResult::Passed
        );

        let result = LlmOutputValidator::validate_numeric_range(150, 0, 100, "score");
        assert!(matches!(result, VerificationResult::PostconditionViolation { .. }));
    }

    #[test]
    fn test_llm_validator_array() {
        let sorted = vec![1, 2, 3, 4, 5];
        assert_eq!(
            LlmOutputValidator::validate_array_invariants(&sorted, true),
            VerificationResult::Passed
        );

        let unsorted = vec![3, 1, 4, 2, 5];
        let result = LlmOutputValidator::validate_array_invariants(&unsorted, true);
        assert!(matches!(result, VerificationResult::InvariantViolation { .. }));
    }

    #[test]
    fn test_llm_validator_string() {
        assert_eq!(
            LlmOutputValidator::validate_string("hello", 100),
            VerificationResult::Passed
        );

        let result = LlmOutputValidator::validate_string("", 100);
        assert!(matches!(result, VerificationResult::PostconditionViolation { .. }));
    }
}

// ============================================================================
// 第六部分: 主函数演示
// ============================================================================

fn main() {
    println!("=== 形式验证契约宏演示 ===\n");

    // 演示1: 二分查找
    println!("1. 二分查找契约验证:");
    let sorted_arr = vec![1, 3, 5, 7, 9, 11, 13];
    match binary_search(&sorted_arr, 7) {
        Some(idx) => println!("   Found 7 at index {}", idx),
        None => println!("   7 not found"),
    }

    // 演示2: 安全除法
    println!("\n2. 安全除法契约验证:");
    let (q, r) = safe_divide(17, 5);
    println!("   17 / 5 = {} remainder {}", q, r);

    // 演示3: LLM输出验证
    println!("\n3. LLM输出验证器:");

    // 验证数值范围
    let score = 85;
    match LlmOutputValidator::validate_numeric_range(score, 0, 100, "confidence") {
        VerificationResult::Passed => println!("   Score {} is valid", score),
        _ => println!("   Score {} is invalid", score),
    }

    // 验证数组不变量
    let llm_output = vec![10, 20, 30, 40];
    match LlmOutputValidator::validate_array_invariants(&llm_output, true) {
        VerificationResult::Passed => println!("   Array satisfies sorted invariant"),
        _ => println!("   Array violates sorted invariant"),
    }

    // 验证字符串
    match LlmOutputValidator::validate_string("LLM generated output", 1000) {
        VerificationResult::Passed => println!("   String output is valid"),
        _ => println!("   String output is invalid"),
    }

    println!("\n=== 所有契约验证通过 ===");
}
