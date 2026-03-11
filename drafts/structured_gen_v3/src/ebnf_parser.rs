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
