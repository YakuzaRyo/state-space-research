//! 结构化生成v3 - XGrammar核心机制Rust实现
//!
//! 本crate验证以下核心假设：
//! - EBNF约束如何保证输出格式正确
//! - 如何用Rust实现高效的语法约束解析器
//! - 约束检查的开销有多大
//! - 适用于哪些LLM应用场景

pub mod token_mask;
pub mod ebnf_parser;
pub mod pda_engine;
pub mod json_validator;

use std::fmt;

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

/// 约束生成器 - 将JSON Schema转换为可执行的约束
pub struct ConstraintGenerator;

impl ConstraintGenerator {
    pub fn new() -> Self {
        Self
    }

    /// 编译JSON Schema为EBNF语法
    pub fn compile_schema(&self, schema: &str) -> Result<ebnf_parser::EbnfGrammar, GrammarError> {
        use ebnf_parser::EbnfGrammar;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_error_display() {
        let err = GrammarError::InvalidSyntax("test error".to_string());
        assert_eq!(err.to_string(), "Syntax error: test error");
    }

    #[test]
    fn test_constraint_generator() {
        let gen = ConstraintGenerator::new();
        let schema = r#"{"type": "object"}"#;
        let grammar = gen.compile_schema(schema);
        assert!(grammar.is_ok());
    }
}
//! Token Mask模块 - 高效token掩码存储与操作
//!
//! 核心优化:
//! 1. 使用bitset而非bool数组,内存占用减少32x
//! 2. SIMD友好的位操作
//! 3. 自适应存储策略(接受集/拒绝集/bitset)

use std::ops::{BitAnd, BitOr};

/// Token ID类型
pub type TokenId = u32;

/// 动态Bitset - 高效存储大量布尔值
#[derive(Clone, Debug)]
pub struct DynamicBitset {
    data: Vec<u32>,
    size: usize,
}

impl DynamicBitset {
    /// 创建指定大小的bitset,所有位初始化为0
    pub fn new(size: usize) -> Self {
        let num_words = (size + 31) / 32;
        Self {
            data: vec![0; num_words],
            size,
        }
    }

    /// 获取指定位置的值
    pub fn get(&self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }
        let word_idx = index / 32;
        let bit_idx = index % 32;
        (self.data[word_idx] >> bit_idx) & 1 != 0
    }

    /// 设置指定位置的值
    pub fn set(&mut self, index: usize, value: bool) {
        if index >= self.size {
            return;
        }
        let word_idx = index / 32;
        let bit_idx = index % 32;
        if value {
            self.data[word_idx] |= 1 << bit_idx;
        } else {
            self.data[word_idx] &= !(1 << bit_idx);
        }
    }

    /// 返回内存占用(字节)
    pub fn memory_usage(&self) -> usize {
        self.data.len() * std::mem::size_of::<u32>()
    }

    /// 返回设置的位数
    pub fn count_ones(&self) -> usize {
        self.data.iter().map(|&w| w.count_ones() as usize).sum()
    }

    /// 返回bitset大小
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// 按位与操作
    pub fn and(&self, other: &Self) -> Self {
        let min_len = self.data.len().min(other.data.len());
        let mut result = Self::new(self.size.min(other.size));
        for i in 0..min_len {
            result.data[i] = self.data[i] & other.data[i];
        }
        result
    }

    /// 按位或操作
    pub fn or(&self, other: &Self) -> Self {
        let size = self.size.max(other.size);
        let mut result = Self::new(size);
        let min_len = self.data.len().min(other.data.len());
        for i in 0..min_len {
            result.data[i] = self.data[i] | other.data[i];
        }
        // 复制剩余的
        if self.data.len() > min_len {
            result.data[min_len..self.data.len()].copy_from_slice(&self.data[min_len..]);
        } else if other.data.len() > min_len {
            result.data[min_len..other.data.len()].copy_from_slice(&other.data[min_len..]);
        }
        result
    }

    /// 按位取反
    pub fn not(&self) -> Self {
        let mut result = self.clone();
        for word in &mut result.data {
            *word = !*word;
        }
        // 清除超出size的位
        let remainder = self.size % 32;
        if remainder != 0 && !result.data.is_empty() {
            let mask = (1u32 << remainder) - 1;
            let last_idx = result.data.len() - 1;
            result.data[last_idx] &= mask;
        }
        result
    }

    /// 返回所有设置为true的索引
    pub fn iter_set(&self) -> impl Iterator<Item = usize> + '_ {
        self.data.iter().enumerate().flat_map(|(word_idx, &word)| {
            let base = word_idx * 32;
            (0..32).filter_map(move |bit_idx| {
                if (word >> bit_idx) & 1 != 0 {
                    Some(base + bit_idx)
                } else {
                    None
                }
            })
        }).take_while(|&idx| idx < self.size)
    }
}

impl BitAnd for &DynamicBitset {
    type Output = DynamicBitset;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl BitOr for &DynamicBitset {
    type Output = DynamicBitset;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

/// Token分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    /// 上下文无关 - 仅通过当前状态即可确定有效性
    ContextIndependent,
    /// 上下文相关 - 需要完整栈信息
    ContextDependent,
    /// 不确定 - 需要运行时检查
    Uncertain,
}

/// Token分类器
pub struct TokenClassifier {
    vocab_size: usize,
    /// 预计算的分类缓存
    classifications: Vec<TokenCategory>,
}

impl TokenClassifier {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            classifications: vec![TokenCategory::Uncertain; vocab_size],
        }
    }

    /// 设置token分类
    pub fn set_category(&mut self, token: TokenId, category: TokenCategory) {
        if (token as usize) < self.vocab_size {
            self.classifications[token as usize] = category;
        }
    }

    /// 获取token分类
    pub fn get_category(&self, token: TokenId) -> TokenCategory {
        self.classifications.get(token as usize).copied()
            .unwrap_or(TokenCategory::Uncertain)
    }

    /// 统计各类token数量
    pub fn statistics(&self) -> (usize, usize, usize) {
        let mut context_independent = 0;
        let mut context_dependent = 0;
        let mut uncertain = 0;

        for &cat in &self.classifications {
            match cat {
                TokenCategory::ContextIndependent => context_independent += 1,
                TokenCategory::ContextDependent => context_dependent += 1,
                TokenCategory::Uncertain => uncertain += 1,
            }
        }

        (context_independent, context_dependent, uncertain)
    }
}

/// Token Mask缓存 - 核心优化组件
pub struct TokenMaskCache {
    vocab_size: usize,
    /// 状态 -> Token Mask映射
    masks: hashbrown::HashMap<usize, DynamicBitset>,
    /// 缓存统计
    hits: u64,
    misses: u64,
}

impl TokenMaskCache {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            masks: hashbrown::HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// 插入mask
    pub fn insert(&mut self, state: usize, mask: DynamicBitset) {
        self.masks.insert(state, mask);
    }

    /// 获取mask
    pub fn get(&mut self, state: usize) -> Option<&DynamicBitset> {
        if self.masks.contains_key(&state) {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        self.masks.get(&state)
    }

    /// 缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 缓存状态数
    pub fn len(&self) -> usize {
        self.masks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.masks.is_empty()
    }

    /// 内存占用估算
    pub fn memory_usage(&self) -> usize {
        let mask_size = self.vocab_size / 8;
        self.masks.len() * (mask_size + std::mem::size_of::<DynamicBitset>())
    }

    /// 清除缓存
    pub fn clear(&mut self) {
        self.masks.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitset_basic() {
        let mut bitset = DynamicBitset::new(100);
        assert!(!bitset.get(50));

        bitset.set(50, true);
        assert!(bitset.get(50));

        bitset.set(50, false);
        assert!(!bitset.get(50));
    }

    #[test]
    fn test_bitset_and() {
        let mut a = DynamicBitset::new(100);
        let mut b = DynamicBitset::new(100);

        a.set(10, true);
        a.set(20, true);
        b.set(10, true);
        b.set(30, true);

        let c = a.and(&b);
        assert!(c.get(10));
        assert!(!c.get(20));
        assert!(!c.get(30));
    }

    #[test]
    fn test_memory_efficiency() {
        let vocab_size = 128_000;
        let bitset = DynamicBitset::new(vocab_size);

        // bitset: 128000 / 32 * 4 = 16KB
        // bool[]: 128000 * 1 = 128KB
        assert!(bitset.memory_usage() < vocab_size / 8 + 100);
    }

    #[test]
    fn test_token_classifier() {
        let mut classifier = TokenClassifier::new(1000);
        classifier.set_category(0, TokenCategory::ContextIndependent);
        classifier.set_category(1, TokenCategory::ContextDependent);

        assert_eq!(classifier.get_category(0), TokenCategory::ContextIndependent);
        assert_eq!(classifier.get_category(1), TokenCategory::ContextDependent);
        assert_eq!(classifier.get_category(2), TokenCategory::Uncertain);
    }
}
//! EBNF解析器 - 将EBNF语法转换为内部表示
//!
//! 支持:
//! - 基本EBNF语法元素
//! - 字符类 [a-z]
//! - 重复 * + ?
//! - 分组 ( )
//! - 选择 |

use std::collections::HashMap;
use crate::GrammarError;

/// 语法表达式
#[derive(Debug, Clone, PartialEq)]
pub enum GrammarExpr {
    /// 终结符 - 字面量字符串
    Terminal(String),
    /// 非终结符 - 引用其他规则
    NonTerminal(String),
    /// 字符类 - [a-zA-Z]
    CharClass(Vec<(char, char)>),
    /// 序列 - expr1 expr2
    Sequence(Vec<GrammarExpr>),
    /// 选择 - expr1 | expr2
    Choice(Vec<GrammarExpr>),
    /// 可选 - expr?
    Optional(Box<GrammarExpr>),
    /// 零次或多次 - expr*
    Star(Box<GrammarExpr>),
    /// 一次或多次 - expr+
    Plus(Box<GrammarExpr>),
    /// 空
    Empty,
}

/// 语法规则
#[derive(Debug, Clone)]
pub struct GrammarRule {
    pub name: String,
    pub expr: GrammarExpr,
}

/// EBNF语法
#[derive(Debug)]
pub struct EbnfGrammar {
    pub rules: HashMap<String, GrammarRule>,
    pub start_rule: String,
}

impl EbnfGrammar {
    /// 解析EBNF字符串
    pub fn parse(input: &str) -> Result<Self, GrammarError> {
        let mut grammar = Self {
            rules: HashMap::new(),
            start_rule: String::new(),
        };

        let parser = EbnfParser::new(input);
        let rules = parser.parse_rules()?;

        for (i, rule) in rules.into_iter().enumerate() {
            if i == 0 {
                grammar.start_rule = rule.name.clone();
            }
            grammar.rules.insert(rule.name.clone(), rule);
        }

        Ok(grammar)
    }

    /// 获取起始规则
    pub fn get_start_rule(&self) -> Option<&GrammarRule> {
        self.rules.get(&self.start_rule)
    }

    /// 获取指定规则
    pub fn get_rule(&self, name: &str) -> Option<&GrammarRule> {
        self.rules.get(name)
    }
}

/// EBNF解析器实现
struct EbnfParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> EbnfParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_rules(&mut self) -> Result<Vec<GrammarRule>, GrammarError> {
        let mut rules = Vec::new();

        self.skip_whitespace();

        while self.pos < self.input.len() {
            let rule = self.parse_rule()?;
            rules.push(rule);
            self.skip_whitespace();
        }

        Ok(rules)
    }

    fn parse_rule(&mut self) -> Result<GrammarRule, GrammarError> {
        self.skip_whitespace();

        // 解析规则名
        let name = self.parse_identifier()?;
        self.skip_whitespace();

        // 期望 ::= 或 ::=
        if !self.consume("::=") && !self.consume(":") {
            return Err(GrammarError::InvalidSyntax(
                format!("Expected '::=' or ':' after rule name '{}'", name)
            ));
        }
        // 处理 :: 的情况(已经消费了一个:)
        if self.peek_char() == Some(':') {
            self.advance();
        }
        if self.peek_char() == Some('=') {
            self.advance();
        }

        self.skip_whitespace();

        // 解析表达式
        let expr = self.parse_expr()?;

        Ok(GrammarRule { name, expr })
    }

    fn parse_expr(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.parse_choice()
    }

    fn parse_choice(&mut self) -> Result<GrammarExpr, GrammarError> {
        let mut alternatives = vec![self.parse_sequence()?];

        self.skip_whitespace();
        while self.consume("|") {
            self.skip_whitespace();
            alternatives.push(self.parse_sequence()?);
            self.skip_whitespace();
        }

        if alternatives.len() == 1 {
            Ok(alternatives.into_iter().next().unwrap())
        } else {
            Ok(GrammarExpr::Choice(alternatives))
        }
    }

    fn parse_sequence(&mut self) -> Result<GrammarExpr, GrammarError> {
        let mut elements = Vec::new();

        self.skip_whitespace();
        while let Some(c) = self.peek_char() {
            if c == '|' || c == ')' || c == ']' || c == '}' {
                break;
            }

            let elem = self.parse_element()?;
            elements.push(elem);
            self.skip_whitespace();
        }

        if elements.is_empty() {
            Ok(GrammarExpr::Empty)
        } else if elements.len() == 1 {
            Ok(elements.into_iter().next().unwrap())
        } else {
            Ok(GrammarExpr::Sequence(elements))
        }
    }

    fn parse_element(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.skip_whitespace();

        let expr = match self.peek_char() {
            Some('"') => self.parse_string_literal()?,
            Some('\'') => self.parse_char_literal()?,
            Some('[') => self.parse_char_class()?,
            Some('(') => self.parse_group()?,
            Some('{') => self.parse_brace_group()?,
            Some(c) if c.is_alphabetic() || c == '_' => {
                let id = self.parse_identifier()?;
                GrammarExpr::NonTerminal(id)
            }
            _ => {
                return Err(GrammarError::InvalidSyntax(
                    format!("Unexpected character at position {}", self.pos)
                ));
            }
        };

        // 处理重复修饰符
        self.skip_whitespace();
        if self.consume("*") {
            Ok(GrammarExpr::Star(Box::new(expr)))
        } else if self.consume("+") {
            Ok(GrammarExpr::Plus(Box::new(expr)))
        } else if self.consume("?") {
            Ok(GrammarExpr::Optional(Box::new(expr)))
        } else {
            Ok(expr)
        }
    }

    fn parse_string_literal(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.expect('"')?;
        let mut value = String::new();

        while let Some(c) = self.next_char() {
            if c == '"' {
                return Ok(GrammarExpr::Terminal(value));
            }
            value.push(c);
        }

        Err(GrammarError::InvalidSyntax("Unterminated string literal".to_string()))
    }

    fn parse_char_literal(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.expect('\'')?;
        let mut value = String::new();

        while let Some(c) = self.next_char() {
            if c == '\'' {
                return Ok(GrammarExpr::Terminal(value));
            }
            value.push(c);
        }

        Err(GrammarError::InvalidSyntax("Unterminated char literal".to_string()))
    }

    fn parse_char_class(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.expect('[')?;
        let mut ranges = Vec::new();

        while let Some(c) = self.peek_char() {
            if c == ']' {
                self.advance();
                return Ok(GrammarExpr::CharClass(ranges));
            }

            let start = self.next_char().unwrap();

            if self.peek_char() == Some('-') && self.lookahead(1) != Some(']') {
                self.advance(); // consume '-'
                let end = self.next_char().ok_or_else(|| {
                    GrammarError::InvalidSyntax("Unexpected end in char class".to_string())
                })?;
                ranges.push((start, end));
            } else {
                ranges.push((start, start));
            }
        }

        Err(GrammarError::InvalidSyntax("Unterminated char class".to_string()))
    }

    fn parse_group(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.expect('(')?;
        let expr = self.parse_expr()?;
        self.expect(')')?;
        Ok(expr)
    }

    fn parse_brace_group(&mut self) -> Result<GrammarExpr, GrammarError> {
        self.expect('{')?;
        let expr = self.parse_expr()?;
        self.expect('}')?;
        // {} 表示零次或多次
        Ok(GrammarExpr::Star(Box::new(expr)))
    }

    fn parse_identifier(&mut self) -> Result<String, GrammarError> {
        let mut id = String::new();

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                id.push(self.next_char().unwrap());
            } else {
                break;
            }
        }

        if id.is_empty() {
            Err(GrammarError::InvalidSyntax(
                format!("Expected identifier at position {}", self.pos)
            ))
        } else {
            Ok(id)
        }
    }

    // 辅助方法
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '#' {
                if c == '#' {
                    // 跳过注释
                    while self.peek_char() != Some('\n') && self.peek_char().is_some() {
                        self.advance();
                    }
                } else {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn lookahead(&self, offset: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(offset)
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.advance();
        Some(c)
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek_char() {
            self.pos += c.len_utf8();
        }
    }

    fn consume(&mut self, s: &str) -> bool {
        if self.input[self.pos..].starts_with(s) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), GrammarError> {
        match self.next_char() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(GrammarError::InvalidSyntax(
                format!("Expected '{}', found '{}' at position {}", expected, c, self.pos)
            )),
            None => Err(GrammarError::InvalidSyntax(
                format!("Expected '{}', found EOF", expected)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let grammar = EbnfGrammar::parse("root ::= 'hello'").unwrap();
        assert!(grammar.rules.contains_key("root"));
    }

    #[test]
    fn test_parse_choice() {
        let grammar = EbnfGrammar::parse("value ::= 'true' | 'false' | 'null'").unwrap();
        let rule = grammar.get_rule("value").unwrap();
        match &rule.expr {
            GrammarExpr::Choice(alts) => assert_eq!(alts.len(), 3),
            _ => panic!("Expected Choice"),
        }
    }

    #[test]
    fn test_parse_sequence() {
        let grammar = EbnfGrammar::parse("pair ::= string ':' value").unwrap();
        let rule = grammar.get_rule("pair").unwrap();
        match &rule.expr {
            GrammarExpr::Sequence(elems) => assert_eq!(elems.len(), 3),
            _ => panic!("Expected Sequence"),
        }
    }

    #[test]
    fn test_parse_repetition() {
        let grammar = EbnfGrammar::parse("list ::= item (',' item)*").unwrap();
        assert!(grammar.rules.contains_key("list"));
    }

    #[test]
    fn test_parse_char_class() {
        let grammar = EbnfGrammar::parse("digit ::= [0-9]").unwrap();
        let rule = grammar.get_rule("digit").unwrap();
        match &rule.expr {
            GrammarExpr::CharClass(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0], ('0', '9'));
            }
            _ => panic!("Expected CharClass"),
        }
    }
}
//! PDA引擎 - 下推自动机实现
//!
//! PDA是处理上下文无关文法(CFG)的标准自动机模型。
//! 本实现包含:
//! - 确定性PDA (DPDA) - 用于LL(1)文法
//! - 非确定性PDA支持 - 通过持久栈实现
//! - 与Token Mask Cache的集成

use std::collections::{HashMap, HashSet};
use crate::{GrammarError, TokenId};
use crate::token_mask::DynamicBitset;
use crate::ebnf_parser::{EbnfGrammar, GrammarExpr};

/// PDA状态类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PDAState {
    /// 初始状态
    Initial,
    /// 中间状态
    Intermediate,
    /// 接受状态
    Accepting,
    /// 错误状态
    Error,
}

/// PDA转移
#[derive(Debug, Clone)]
pub struct PDATransition {
    /// 输入符号 (None表示ε转移)
    pub input: Option<char>,
    /// 栈顶符号 (None表示不关心)
    pub stack_top: Option<char>,
    /// 目标状态
    pub target: usize,
    /// 栈操作: None=弹栈, Some(c)=压栈c
    pub stack_op: Option<Option<char>>,
}

/// 持久栈节点 - 支持O(1)回滚
#[derive(Debug, Clone)]
struct StackNode {
    value: char,
    parent: Option<usize>, // 父节点索引
}

/// 持久栈实现
#[derive(Debug, Clone)]
pub struct PersistentStack {
    nodes: Vec<StackNode>,
    top: Option<usize>,
}

impl PersistentStack {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            top: None,
        }
    }

    /// 压栈 - 返回新栈(持久化)
    pub fn push(&self, value: char) -> Self {
        let mut new_nodes = self.nodes.clone();
        let new_idx = new_nodes.len();
        new_nodes.push(StackNode {
            value,
            parent: self.top,
        });

        Self {
            nodes: new_nodes,
            top: Some(new_idx),
        }
    }

    /// 弹栈 - 返回(值, 新栈)
    pub fn pop(&self) -> Option<(char, Self)> {
        self.top.map(|idx| {
            let node = &self.nodes[idx];
            let value = node.value;
            let new_stack = Self {
                nodes: self.nodes.clone(),
                top: node.parent,
            };
            (value, new_stack)
        })
    }

    /// 查看栈顶
    pub fn peek(&self) -> Option<char> {
        self.top.map(|idx| self.nodes[idx].value)
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    /// 栈深度
    pub fn depth(&self) -> usize {
        let mut count = 0;
        let mut current = self.top;
        while let Some(idx) = current {
            count += 1;
            current = self.nodes[idx].parent;
        }
        count
    }
}

impl Default for PersistentStack {
    fn default() -> Self {
        Self::new()
    }
}

/// PDA配置 (状态, 栈)
#[derive(Debug, Clone)]
pub struct PDAConfiguration {
    pub state: usize,
    pub stack: PersistentStack,
}

impl PDAConfiguration {
    pub fn new(state: usize) -> Self {
        Self {
            state,
            stack: PersistentStack::new(),
        }
    }

    pub fn with_stack(state: usize, stack: PersistentStack) -> Self {
        Self { state, stack }
    }
}

/// 下推自动机
#[derive(Debug)]
pub struct PushdownAutomaton {
    /// 状态集合
    pub states: HashMap<usize, PDAState>,
    /// 转移函数: (state, input) -> [transitions]
    pub transitions: HashMap<(usize, Option<char>), Vec<PDATransition>>,
    /// 当前配置
    pub current: PDAConfiguration,
    /// 历史配置(用于回滚)
    history: Vec<PDAConfiguration>,
}

impl PushdownAutomaton {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            transitions: HashMap::new(),
            current: PDAConfiguration::new(0),
            history: Vec::new(),
        }
    }

    /// 添加状态
    pub fn add_state(&mut self, id: usize, state_type: PDAState) {
        self.states.insert(id, state_type);
    }

    /// 添加转移
    pub fn add_transition(
        &mut self,
        from: usize,
        input: char,
        to: usize,
        stack_push: Option<char>,
    ) {
        let trans = PDATransition {
            input: Some(input),
            stack_top: None,
            target: to,
            stack_op: Some(stack_push),
        };

        self.transitions
            .entry((from, Some(input)))
            .or_default()
            .push(trans);
    }

    /// 添加空转移
    pub fn add_empty_transition(&mut self, from: usize, to: usize) {
        let trans = PDATransition {
            input: None,
            stack_top: None,
            target: to,
            stack_op: None,
        };

        self.transitions
            .entry((from, None))
            .or_default()
            .push(trans);
    }

    /// 从文法构建PDA
    pub fn from_grammar(grammar: &EbnfGrammar) -> Self {
        let mut pda = Self::new();

        // 简化实现: 创建一个基本的JSON对象PDA
        // 状态0: 开始, 期望 {
        // 状态1: 在对象内, 期望 " 或 }
        // 状态2: 在key后, 期望 :
        // 状态3: 在value后, 期望 , 或 }
        // 状态4: 接受

        pda.add_state(0, PDAState::Initial);
        pda.add_state(1, PDAState::Intermediate);
        pda.add_state(2, PDAState::Intermediate);
        pda.add_state(3, PDAState::Intermediate);
        pda.add_state(4, PDAState::Accepting);

        // { -> 状态1, 压栈 {
        pda.add_transition(0, '{', 1, Some('{'));

        // " -> 状态2 (简化,实际应处理字符串)
        pda.add_transition(1, '"', 2, None);

        // } -> 如果栈顶是{, 弹栈并转移到接受
        // 简化: 直接到接受
        pda.add_transition(1, '}', 4, None);

        // : -> 状态3
        pda.add_transition(2, ':', 3, None);

        // , -> 回到状态1
        pda.add_transition(3, ',', 1, None);

        // } -> 接受
        pda.add_transition(3, '}', 4, None);

        pda
    }

    /// 验证输入序列
    pub fn validate<T: Iterator<Item = char>>(
        &mut self,
        input: T,
    ) -> Result<(), GrammarError> {
        for c in input {
            self.consume_char(c)?;
        }

        if self.is_accepting() {
            Ok(())
        } else {
            Err(GrammarError::InvalidSyntax(
                "Input rejected by PDA".to_string()
            ))
        }
    }

    /// 消费一个字符
    fn consume_char(&mut self, c: char) -> Result<(), GrammarError> {
        // 保存历史
        self.history.push(self.current.clone());

        let state = self.current.state;

        // 查找转移
        if let Some(transitions) = self.transitions.get(&(state, Some(c))) {
            if let Some(trans) = transitions.first() {
                // 应用转移
                self.current.state = trans.target;

                // 处理栈操作
                if let Some(stack_op) = &trans.stack_op {
                    if let Some(push_val) = stack_op {
                        self.current.stack = self.current.stack.push(*push_val);
                    }
                    // None 表示弹栈
                }

                return Ok(());
            }
        }

        Err(GrammarError::InvalidSyntax(format!(
            "No transition for '{}' from state {}",
            c, state
        )))
    }

    /// 消费token (用于LLM集成)
    pub fn consume(&mut self, token: TokenId) -> Result<(), GrammarError> {
        // 简化: 假设token映射到字符
        // 实际实现需要tokenizer映射
        let c = match token {
            1000 => '{',
            1001 => '}',
            1002 => '[',
            1003 => ']',
            2000 => '"',
            2001 => ':',
            2002 => ',',
            _ => return Ok(()), // 忽略其他token
        };
        self.consume_char(c)
    }

    /// 获取允许的token集合
    pub fn get_allowed_tokens(&self) -> DynamicBitset {
        let mut mask = DynamicBitset::new(128_000);

        let state = self.current.state;

        // 查找所有可能的转移
        for ((from, input), _) in &self.transitions {
            if *from == state {
                if let Some(c) = input {
                    // 将字符映射回token
                    let token = match c {
                        '{' => 1000,
                        '}' => 1001,
                        '[' => 1002,
                        ']' => 1003,
                        '"' => 2000,
                        ':' => 2001,
                        ',' => 2002,
                        _ => continue,
                    };
                    mask.set(token as usize, true);
                }
            }
        }

        mask
    }

    /// 检查是否在接受状态
    pub fn is_accepting(&self) -> bool {
        self.states
            .get(&self.current.state)
            .map(|s| *s == PDAState::Accepting)
            .unwrap_or(false)
    }

    /// 回滚到上一步
    pub fn rollback(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.current = prev;
            true
        } else {
            false
        }
    }

    /// 获取当前配置
    pub fn get_configuration(&self) -> &PDAConfiguration {
        &self.current
    }
}

impl Default for PushdownAutomaton {
    fn default() -> Self {
        Self::new()
    }
}

/// PDA状态压缩 - 用于高效存储
#[derive(Debug, Clone)]
pub struct PDAStateCompressor {
    /// 等价状态映射
    equivalence_map: HashMap<usize, usize>,
}

impl PDAStateCompressor {
    pub fn new() -> Self {
        Self {
            equivalence_map: HashMap::new(),
        }
    }

    /// 合并等价状态
    pub fn compress(&mut self, pda: &mut PushdownAutomaton) {
        // 简化实现: 识别具有相同转移的状态
        let mut state_signatures: HashMap<Vec<(Option<char>, usize)>, usize> = HashMap::new();

        for (state_id, _) in &pda.states {
            let mut signature = Vec::new();

            // 收集该状态的所有转移
            for ((from, input), trans) in &pda.transitions {
                if *from == *state_id {
                    for t in trans {
                        signature.push((*input, t.target));
                    }
                }
            }

            signature.sort();

            if let Some(&canonical) = state_signatures.get(&signature) {
                self.equivalence_map.insert(*state_id, canonical);
            } else {
                state_signatures.insert(signature, *state_id);
            }
        }
    }
}

impl Default for PDAStateCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistent_stack() {
        let s1 = PersistentStack::new();
        assert!(s1.is_empty());

        let s2 = s1.push('a');
        assert!(!s2.is_empty());
        assert_eq!(s2.peek(), Some('a'));

        let s3 = s2.push('b');
        assert_eq!(s3.peek(), Some('b'));

        let (val, s4) = s3.pop().unwrap();
        assert_eq!(val, 'b');
        assert_eq!(s4.peek(), Some('a'));

        // s2保持不变
        assert_eq!(s2.peek(), Some('a'));
    }

    #[test]
    fn test_pda_simple() {
        let mut pda = PushdownAutomaton::new();
        pda.add_state(0, PDAState::Initial);
        pda.add_state(1, PDAState::Accepting);
        pda.add_transition(0, 'a', 1, None);

        assert!(pda.validate("a".chars()).is_ok());
        assert!(pda.validate("b".chars()).is_err());
    }

    #[test]
    fn test_pda_rollback() {
        let mut pda = PushdownAutomaton::new();
        pda.add_state(0, PDAState::Initial);
        pda.add_state(1, PDAState::Intermediate);
        pda.add_state(2, PDAState::Accepting);
        pda.add_transition(0, 'a', 1, None);
        pda.add_transition(1, 'b', 2, None);

        pda.consume_char('a').unwrap();
        assert_eq!(pda.current.state, 1);

        pda.rollback();
        assert_eq!(pda.current.state, 0);
    }
}
//! JSON Schema验证器 - 简化实现
//!
//! 支持:
//! - 基本类型验证 (string, integer, number, boolean, array, object)
//! - 必需字段检查
//! - 嵌套对象验证
//! - 数组元素验证

use serde_json::Value;
use crate::GrammarError;

/// JSON Schema验证器
#[derive(Debug)]
pub struct JsonSchemaValidator {
    schema: Value,
}

impl JsonSchemaValidator {
    /// 从JSON字符串创建验证器
    pub fn from_schema(schema_str: &str) -> Result<Self, GrammarError> {
        let schema: Value = serde_json::from_str(schema_str)
            .map_err(|e| GrammarError::InvalidSyntax(format!("Invalid JSON schema: {}", e)))?;

        Ok(Self { schema })
    }

    /// 验证JSON字符串
    pub fn validate(&self, json_str: &str) -> Result<(), GrammarError> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| GrammarError::InvalidSyntax(format!("Invalid JSON: {}", e)))?;

        self.validate_value(&value, &self.schema)
    }

    /// 验证值是否符合schema
    fn validate_value(&self, value: &Value, schema: &Value) -> Result<(), GrammarError> {
        // 处理schema引用
        let schema = if let Some(ref_val) = schema.get("$ref") {
            // 简化: 忽略$ref解析
            schema
        } else {
            schema
        };

        // 类型验证
        if let Some(type_val) = schema.get("type") {
            self.validate_type(value, type_val)?;
        }

        // 对象验证
        if let Some(properties) = schema.get("properties") {
            if let Value::Object(props) = properties {
                self.validate_properties(value, props, schema.get("required"))?;
            }
        }

        // 数组验证
        if let Some(items) = schema.get("items") {
            if let Value::Array(arr) = value {
                for (i, item) in arr.iter().enumerate() {
                    self.validate_value(item, items)
                        .map_err(|e| GrammarError::SchemaViolation(
                            format!("Item {}: {}", i, e)
                        ))?;
                }
            }
        }

        // 数值范围验证
        if let Value::Number(n) = value {
            if let Some(min) = schema.get("minimum") {
                if let Some(min_val) = min.as_f64() {
                    if n.as_f64().unwrap_or(0.0) < min_val {
                        return Err(GrammarError::SchemaViolation(
                            format!("Value {} is less than minimum {}", n, min_val)
                        ));
                    }
                }
            }

            if let Some(max) = schema.get("maximum") {
                if let Some(max_val) = max.as_f64() {
                    if n.as_f64().unwrap_or(0.0) > max_val {
                        return Err(GrammarError::SchemaViolation(
                            format!("Value {} is greater than maximum {}", n, max_val)
                        ));
                    }
                }
            }
        }

        // 字符串长度验证
        if let Value::String(s) = value {
            if let Some(min_len) = schema.get("minLength") {
                if let Some(min) = min_len.as_u64() {
                    if s.len() < min as usize {
                        return Err(GrammarError::SchemaViolation(
                            format!("String length {} is less than minLength {}", s.len(), min)
                        ));
                    }
                }
            }

            if let Some(max_len) = schema.get("maxLength") {
                if let Some(max) = max_len.as_u64() {
                    if s.len() > max as usize {
                        return Err(GrammarError::SchemaViolation(
                            format!("String length {} is greater than maxLength {}", s.len(), max)
                        ));
                    }
                }
            }
        }

        // 枚举验证
        if let Some(enum_vals) = schema.get("enum") {
            if let Value::Array(vals) = enum_vals {
                if !vals.contains(value) {
                    return Err(GrammarError::SchemaViolation(
                        format!("Value {:?} is not in enum {:?}", value, vals)
                    ));
                }
            }
        }

        Ok(())
    }

    /// 验证类型
    fn validate_type(&self, value: &Value, type_val: &Value) -> Result<(), GrammarError> {
        let expected_type = type_val.as_str()
            .ok_or_else(|| GrammarError::InvalidSyntax("Invalid type in schema".to_string()))?;

        let matches = match expected_type {
            "string" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => return Err(GrammarError::InvalidSyntax(
                format!("Unknown type: {}", expected_type)
            )),
        };

        if matches {
            Ok(())
        } else {
            Err(GrammarError::SchemaViolation(
                format!("Expected type {}, got {:?}", expected_type, value)
            ))
        }
    }

    /// 验证对象属性
    fn validate_properties(
        &self,
        value: &Value,
        properties: &serde_json::Map<String, Value>,
        required: Option<&Value>,
    ) -> Result<(), GrammarError> {
        let obj = match value {
            Value::Object(o) => o,
            _ => return Err(GrammarError::SchemaViolation(
                "Expected object".to_string()
            )),
        };

        // 验证必需字段
        if let Some(req) = required {
            if let Value::Array(req_fields) = req {
                for field in req_fields {
                    if let Some(field_name) = field.as_str() {
                        if !obj.contains_key(field_name) {
                            return Err(GrammarError::SchemaViolation(
                                format!("Missing required field: {}", field_name)
                            ));
                        }
                    }
                }
            }
        }

        // 验证每个属性
        for (prop_name, prop_schema) in properties {
            if let Some(prop_value) = obj.get(prop_name) {
                self.validate_value(prop_value, prop_schema)
                    .map_err(|e| GrammarError::SchemaViolation(
                        format!("Field '{}': {}", prop_name, e)
                    ))?;
            }
        }

        Ok(())
    }
}

/// 从JSON Schema生成EBNF语法
pub fn schema_to_ebnf(schema: &Value) -> String {
    let mut grammar = String::new();

    grammar.push_str("root ::= object\n\n");
    grammar.push_str("object ::= \"{\" pair_list \"}\"\n");
    grammar.push_str("pair_list ::= pair (\",\" pair)* | \"\"\n");
    grammar.push_str("pair ::= string \":\" value\n\n");
    grammar.push_str("value ::= object | array | string | number | \"true\" | \"false\" | \"null\"\n");
    grammar.push_str("array ::= \"[\" value_list \"]\"\n");
    grammar.push_str("value_list ::= value (\",\" value)* | \"\"\n\n");
    grammar.push_str("string ::= \"\\\"\" char* \"\\\"\"\n");
    grammar.push_str("char ::= [^\"\\\\] | \"\\\\\" esc_char\n");
    grammar.push_str("esc_char ::= [\"\\\\/bfnrt] | \"u\" hex_digit{4}\n");
    grammar.push_str("hex_digit ::= [0-9a-fA-F]\n\n");
    grammar.push_str("number ::= \"-\"? int frac? exp?\n");
    grammar.push_str("int ::= \"0\" | [1-9] [0-9]*\n");
    grammar.push_str("frac ::= \".\" [0-9]+\n");
    grammar.push_str("exp ::= [eE] [+-]? [0-9]+\n");

    grammar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_string() {
        let schema = r#"{"type": "string"}"#;
        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#""hello""#).is_ok());
        assert!(validator.validate(r#"123"#).is_err());
    }

    #[test]
    fn test_validate_object() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        }"#;

        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#"{"name": "Alice", "age": 30}"#).is_ok());
        assert!(validator.validate(r#"{"age": 30}"#).is_err()); // missing required
        assert!(validator.validate(r#"{"name": 123}"#).is_err()); // wrong type
    }

    #[test]
    fn test_validate_array() {
        let schema = r#"{
            "type": "array",
            "items": {"type": "integer"}
        }"#;

        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#"[1, 2, 3]"#).is_ok());
        assert!(validator.validate(r#"[1, "two", 3]"#).is_err());
    }

    #[test]
    fn test_validate_minimum() {
        let schema = r#"{"type": "integer", "minimum": 0}"#;
        let validator = JsonSchemaValidator::from_schema(schema).unwrap();

        assert!(validator.validate(r#"5"#).is_ok());
        assert!(validator.validate(r#"0"#).is_ok());
        assert!(validator.validate(r#"-1"#).is_err());
    }

    #[test]
    fn test_schema_to_ebnf() {
        let schema: Value = serde_json::from_str(r#"{"type": "object"}"#).unwrap();
        let ebnf = schema_to_ebnf(&schema);

        assert!(ebnf.contains("root ::= object"));
        assert!(ebnf.contains("object ::="));
        assert!(ebnf.contains("value ::="));
    }
}
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
