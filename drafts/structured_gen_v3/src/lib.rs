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
