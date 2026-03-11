//! 结构化生成v3 - XGrammar 2核心机制验证
//!
//! 本实现验证以下核心假设：
//! 1. EBNF约束如何保证输出格式正确
//! 2. 如何用Rust实现高效的语法约束解析器
//! 3. 约束检查的开销有多大
//! 4. 适用于哪些LLM应用场景

use std::collections::{HashMap, HashSet};
use std::fmt;

// =============================================================================
// 模块定义
// =============================================================================

pub mod token_mask;
pub mod ebnf_parser;
pub mod pda_engine;
pub mod json_validator;

use token_mask::{DynamicBitset, TokenMaskCache};
use ebnf_parser::{EbnfGrammar, GrammarRule};
use pda_engine::{PDAState, PushdownAutomaton};
use json_validator::JsonSchemaValidator;

// =============================================================================
// 核心类型定义
// =============================================================================

/// Token ID类型
pub type TokenId = u32;

/// 语法错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum GrammarError {
    InvalidToken(TokenId),
    InvalidSyntax(String),
    SchemaViolation(String),
    UnexpectedEof,
}

impl fmt::Display for GrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrammarError::InvalidToken(id) => write!(f, "Invalid token: {}", id),
            GrammarError::InvalidSyntax(msg) => write!(f, "Syntax error: {}", msg),
            GrammarError::SchemaViolation(msg) => write!(f, "Schema violation: {}", msg),
            GrammarError::UnexpectedEof => write!(f, "Unexpected end of input"),
        }
    }
}

impl std::error::Error for GrammarError {}

// =============================================================================
// 主函数 - 验证测试
// =============================================================================

fn main() {
    println!("========================================");
    println!("结构化生成v3 - XGrammar核心验证");
    println!("========================================\n");

    // 测试1: DynamicBitset性能
    test_dynamic_bitset();

    // 测试2: EBNF解析
    test_ebnf_parser();

    // 测试3: PDA引擎
    test_pda_engine();

    // 测试4: JSON Schema验证
    test_json_schema_validator();

    // 测试5: Token Mask Cache
    test_token_mask_cache();

    // 测试6: 端到端约束生成
    test_end_to_end_constraint();

    println!("\n========================================");
    println!("所有测试通过!");
    println!("========================================");
}

// =============================================================================
// 测试函数
// =============================================================================

fn test_dynamic_bitset() {
    println!("[Test 1] DynamicBitset性能测试");
    println!("----------------------------------------");

    // 模拟128K词汇表
    let vocab_size = 128_000;
    let mut bitset = DynamicBitset::new(vocab_size);

    // 设置一些token
    bitset.set(0, true);      // <|begin_of_text|>
    bitset.set(1, true);      // <|end_of_text|>
    bitset.set(100, true);    // "name"
    bitset.set(101, true);    // "value"
    bitset.set(1000, true);   // "{"
    bitset.set(1001, true);   // "}"

    println!("  词汇表大小: {}", vocab_size);
    println!("  Bitset内存占用: {} bytes", bitset.memory_usage());
    println!("  bool[]内存占用: {} bytes", vocab_size * std::mem::size_of::<bool>());
    println!("  压缩率: {:.1}x",
        (vocab_size * std::mem::size_of::<bool>()) as f64 / bitset.memory_usage() as f64);

    // 验证操作
    assert!(bitset.get(0));
    assert!(bitset.get(100));
    assert!(!bitset.get(999));

    // AND操作测试
    let mut bitset2 = DynamicBitset::new(vocab_size);
    bitset2.set(0, true);
    bitset2.set(100, true);

    let result = bitset.and(&bitset2);
    assert!(result.get(0));
    assert!(result.get(100));
    assert!(!result.get(1));  // 只在bitset中

    println!("  AND操作测试: 通过");
    println!();
}

fn test_ebnf_parser() {
    println!("[Test 2] EBNF解析器测试");
    println!("----------------------------------------");

    // 定义简单的JSON语法
    let json_grammar = r#"
        root ::= object
        object ::= "{" pair_list "}"
        pair_list ::= pair ("," pair)*
        pair ::= string ":" value
        value ::= object | array | string | number | "true" | "false" | "null"
        array ::= "[" value_list "]"
        value_list ::= value ("," value)*
        string ::= "\"" char* "\""
        char ::= [^"\\]
        number ::= "-"? [0-9]+ ("." [0-9]+)?
    "#;

    let grammar = EbnfGrammar::parse(json_grammar).expect("Failed to parse grammar");

    println!("  语法规则数: {}", grammar.rules.len());
    for (name, rule) in &grammar.rules {
        println!("    {} -> {:?}", name, rule);
    }

    // 验证特定规则
    assert!(grammar.rules.contains_key("object"));
    assert!(grammar.rules.contains_key("value"));

    println!("  EBNF解析: 通过");
    println!();
}

fn test_pda_engine() {
    println!("[Test 3] PDA引擎测试");
    println!("----------------------------------------");

    // 创建一个简单的括号匹配PDA
    let mut pda = PushdownAutomaton::new();

    // 状态: 0=开始, 1=期望(, 2=期望), 3=接受
    pda.add_state(0, PDAState::Initial);
    pda.add_state(1, PDAState::Intermediate);
    pda.add_state(2, PDAState::Intermediate);
    pda.add_state(3, PDAState::Accepting);

    // 转移: 读入 '(' 压栈
    pda.add_transition(0, '(', 1, Some('('));
    pda.add_transition(1, '(', 1, Some('('));

    // 转移: 读入 ')' 弹栈
    pda.add_transition(1, ')', 2, None);  // 弹栈匹配
    pda.add_transition(2, ')', 2, None);

    // 空栈转移到接受状态
    pda.add_empty_transition(2, 3);

    // 测试有效输入: "(()())"
    let test_input = "(()())";
    let result = pda.validate(test_input.chars());
    println!("  输入: {}", test_input);
    println!("  验证结果: {:?}", result);
    assert!(result.is_ok(), "Expected valid parentheses");

    // 测试无效输入: "(()"
    let invalid_input = "(()";
    let result = pda.validate(invalid_input.chars());
    println!("  输入: {}", invalid_input);
    println!("  验证结果: {:?}", result);
    assert!(result.is_err(), "Expected invalid parentheses");

    println!("  PDA引擎: 通过");
    println!();
}

fn test_json_schema_validator() {
    println!("[Test 4] JSON Schema验证器测试");
    println!("----------------------------------------");

    // 定义Person schema
    let schema = r#"{
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer", "minimum": 0},
            "email": {"type": "string", "format": "email"}
        },
        "required": ["name", "age"]
    }"#;

    let validator = JsonSchemaValidator::from_schema(schema).expect("Invalid schema");

    // 有效数据
    let valid_json = r#"{"name": "Alice", "age": 30, "email": "alice@example.com"}"#;
    let result = validator.validate(valid_json);
    println!("  有效JSON验证: {:?}", result);
    assert!(result.is_ok());

    // 无效数据: 缺少必填字段
    let invalid_json = r#"{"name": "Bob"}"#;
    let result = validator.validate(invalid_json);
    println!("  缺少字段验证: {:?}", result);
    assert!(result.is_err());

    // 无效数据: 类型错误
    let type_error_json = r#"{"name": "Charlie", "age": "thirty"}"#;
    let result = validator.validate(type_error_json);
    println!("  类型错误验证: {:?}", result);
    assert!(result.is_err());

    println!("  JSON Schema验证: 通过");
    println!();
}

fn test_token_mask_cache() {
    println!("[Test 5] Token Mask Cache测试");
    println!("----------------------------------------");

    let vocab_size = 128_000;
    let mut cache = TokenMaskCache::new(vocab_size);

    // 模拟不同PDA状态的token mask
    let state_0_mask = {
        let mut mask = DynamicBitset::new(vocab_size);
        mask.set(1000, true);  // {
        mask.set(1002, true);  // [
        mask.set(2000, true);  // "
        mask
    };

    let state_1_mask = {
        let mut mask = DynamicBitset::new(vocab_size);
        mask.set(1001, true);  // }
        mask.set(1003, true);  // ]
        mask.set(2001, true);  // :
        mask.set(2002, true);  // ,
        mask
    };

    // 插入缓存
    cache.insert(0, state_0_mask.clone());
    cache.insert(1, state_1_mask.clone());

    // 查询缓存
    let retrieved = cache.get(0).expect("Cache miss");
    assert!(retrieved.get(1000));
    assert!(retrieved.get(1002));

    println!("  缓存状态数: {}", cache.len());
    println!("  缓存命中率: {:.1}%", cache.hit_rate() * 100.0);
    println!("  内存占用: {} KB", cache.memory_usage() / 1024);

    println!("  Token Mask Cache: 通过");
    println!();
}

fn test_end_to_end_constraint() {
    println!("[Test 6] 端到端约束生成测试");
    println!("----------------------------------------");

    // 创建一个简化的约束生成器
    let constraint_gen = ConstraintGenerator::new();

    // 定义JSON对象约束
    let json_schema = r#"{"type": "object"}"#;
    let grammar = constraint_gen.compile_schema(json_schema).expect("Compilation failed");

    // 模拟token序列生成
    let tokens = vec![1000, 2000, 2001, 2000, 1001];  // { "name" : "value" }

    // 逐步验证每个token
    let mut pda = PushdownAutomaton::from_grammar(&grammar);

    for (i, &token) in tokens.iter().enumerate() {
        let allowed = pda.get_allowed_tokens();

        if !allowed.get(token as usize) {
            panic!("Token {} not allowed at step {}", token, i);
        }

        pda.consume(token).expect("Failed to consume token");
        println!("  Step {}: token {} accepted", i, token);
    }

    // 验证最终状态
    assert!(pda.is_accepting(), "PDA should be in accepting state");

    println!("  端到端约束生成: 通过");
    println!();
}

// =============================================================================
// 约束生成器
// =============================================================================

pub struct ConstraintGenerator;

impl ConstraintGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn compile_schema(&self, schema: &str) -> Result<EbnfGrammar, GrammarError> {
        // 简化实现: 将JSON Schema转换为EBNF
        let grammar_str = r#"
            root ::= object
            object ::= "{" pair_list "}"
            pair_list ::= pair ("," pair)* | ""
            pair ::= string ":" value
            value ::= object | array | string | number | "true" | "false" | "null"
            array ::= "[" value_list "]"
            value_list ::= value ("," value)* | ""
            string ::= "\"" char* "\""
            char ::= [^"\\]
            number ::= "-"? [0-9]+ ("." [0-9]+)?
        "#;

        EbnfGrammar::parse(grammar_str)
    }
}

impl Default for ConstraintGenerator {
    fn default() -> Self {
        Self::new()
    }
}
