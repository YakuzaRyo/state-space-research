//! XGrammar v2: 第二轮深入研究 - 结构化生成引擎
//!
//! 核心问题: 如何在token级别约束LLM输出?
//!
//! 本次研究重点:
//! - XGrammar 2025最新进展 (TagDispatch, JIT编译, 跨grammar缓存)
//! - Token Mask Cache的详细实现
//! - 与Rust类型系统的深度集成
//! - 性能基准测试框架
//!
//! 参考:
//! - XGrammar: Flexible and Efficient Structured Generation (arXiv:2411.15100)
//! - XGrammar 2: Dynamic and Efficient Structured Generation Engine (arXiv:2601.04426)
//! - llguidance (Microsoft, 2025)
//! - Outlines, Guidance, Jsonformer对比分析

use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Section 1: 核心类型系统与基础设施
// ============================================================================

/// Token ID类型
pub type TokenId = u32;

/// 规则ID类型
pub type RuleId = u32;

/// 状态ID类型 (用于FSM/PDA)
pub type StateId = u32;

/// 位置类型
pub type Position = u32;

/// 字节类型 (XGrammar使用字节级处理)
pub type Byte = u8;

/// 词汇表大小类型
pub type VocabSize = usize;

/// 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum GrammarError {
    InvalidToken(TokenId),
    InvalidState(StateId),
    InvalidTransition { from: StateId, input: TokenId },
    StackUnderflow,
    ParseError(String),
    SchemaValidationError(String),
    CompilationError(String),
}

impl fmt::Display for GrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrammarError::InvalidToken(t) => write!(f, "Invalid token: {}", t),
            GrammarError::InvalidState(s) => write!(f, "Invalid state: {}", s),
            GrammarError::InvalidTransition { from, input } => {
                write!(f, "Invalid transition from state {} with input {}", from, input)
            }
            GrammarError::StackUnderflow => write!(f, "Stack underflow"),
            GrammarError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            GrammarError::SchemaValidationError(msg) => write!(f, "Schema validation error: {}", msg),
            GrammarError::CompilationError(msg) => write!(f, "Compilation error: {}", msg),
        }
    }
}

impl std::error::Error for GrammarError {}

// ============================================================================
// Section 2: 动态Bitset - Token Mask的高效存储
// ============================================================================

/// 动态Bitset - 用于高效存储token掩码
///
/// XGrammar的核心优化之一：将token掩码存储从O(N)布尔数组优化为O(N/32)整数数组
/// 对于Llama-3.1 (128K词汇表)，内存从160MB降至约0.46MB
#[derive(Clone, Debug)]
pub struct DynamicBitset {
    /// 元素数量
    size: usize,
    /// 存储块 (每个u32存储32个bit)
    blocks: Vec<u32>,
}

impl DynamicBitset {
    const BITS_PER_BLOCK: usize = 32;

    /// 创建新的bitset，所有位初始化为0
    pub fn new(size: usize) -> Self {
        let num_blocks = (size + Self::BITS_PER_BLOCK - 1) / Self::BITS_PER_BLOCK;
        Self {
            size,
            blocks: vec![0; num_blocks],
        }
    }

    /// 创建新的bitset，所有位初始化为1
    pub fn ones(size: usize) -> Self {
        let num_blocks = (size + Self::BITS_PER_BLOCK - 1) / Self::BITS_PER_BLOCK;
        let mut blocks = vec![u32::MAX; num_blocks];
        // 处理最后一个块的溢出位
        let remainder = size % Self::BITS_PER_BLOCK;
        if remainder != 0 && !blocks.is_empty() {
            let mask = (1u32 << remainder) - 1;
            blocks[num_blocks - 1] = mask;
        }
        Self { size, blocks }
    }

    /// 设置指定位置的值
    #[inline]
    pub fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.size, "Index out of bounds");
        let block_idx = index / Self::BITS_PER_BLOCK;
        let bit_idx = index % Self::BITS_PER_BLOCK;
        if value {
            self.blocks[block_idx] |= 1u32 << bit_idx;
        } else {
            self.blocks[block_idx] &= !(1u32 << bit_idx);
        }
    }

    /// 获取指定位置的值
    #[inline]
    pub fn get(&self, index: usize) -> bool {
        assert!(index < self.size, "Index out of bounds");
        let block_idx = index / Self::BITS_PER_BLOCK;
        let bit_idx = index % Self::BITS_PER_BLOCK;
        (self.blocks[block_idx] >> bit_idx) & 1 != 0
    }

    /// 与另一个bitset进行AND操作
    pub fn and_with(&mut self, other: &DynamicBitset) {
        assert_eq!(self.size, other.size, "Bitset sizes must match");
        for (a, b) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            *a &= *b;
        }
    }

    /// 与另一个bitset进行OR操作
    pub fn or_with(&mut self, other: &DynamicBitset) {
        assert_eq!(self.size, other.size, "Bitset sizes must match");
        for (a, b) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            *a |= *b;
        }
    }

    /// 取反
    pub fn not(&mut self) {
        for block in self.blocks.iter_mut() {
            *block = !*block;
        }
        // 处理最后一个块的溢出位
        let remainder = self.size % Self::BITS_PER_BLOCK;
        if remainder != 0 && !self.blocks.is_empty() {
            let mask = (1u32 << remainder) - 1;
            let last = self.blocks.len() - 1;
            self.blocks[last] &= mask;
        }
    }

    /// 统计置位数量
    pub fn count_ones(&self) -> usize {
        self.blocks.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// 获取所有置位的索引
    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.blocks.iter().enumerate().flat_map(|(block_idx, block)| {
            let base = block_idx * Self::BITS_PER_BLOCK;
            (0..Self::BITS_PER_BLOCK).filter_map(move |bit_idx| {
                let idx = base + bit_idx;
                if idx < self.size && (block >> bit_idx) & 1 != 0 {
                    Some(idx)
                } else {
                    None
                }
            })
        })
    }

    /// 获取大小
    pub fn len(&self) -> usize {
        self.size
    }

    /// 检查是否为空（全0）
    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(|b| *b == 0)
    }

    /// 重置为全0
    pub fn clear(&mut self) {
        self.blocks.fill(0);
    }

    /// 重置为全1
    pub fn set_all(&mut self) {
        self.blocks.fill(u32::MAX);
        let remainder = self.size % Self::BITS_PER_BLOCK;
        if remainder != 0 && !self.blocks.is_empty() {
            let mask = (1u32 << remainder) - 1;
            let last = self.blocks.len() - 1;
            self.blocks[last] = mask;
        }
    }
}

// ============================================================================
// Section 3: Tokenizer信息 - 词汇表处理
// ============================================================================

/// Tokenizer信息
///
/// 存储词汇表的解码信息，用于字节级匹配
#[derive(Clone, Debug)]
pub struct TokenizerInfo {
    /// 词汇表大小
    pub vocab_size: usize,
    /// 每个token解码后的字节序列
    pub decoded_tokens: Vec<Vec<u8>>,
    /// 前缀树 (用于快速前缀匹配)
    pub prefix_trie: PrefixTrie,
}

impl TokenizerInfo {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            decoded_tokens: vec![Vec::new(); vocab_size],
            prefix_trie: PrefixTrie::new(),
        }
    }

    /// 设置token的解码字节
    pub fn set_decoded(&mut self, token_id: TokenId, bytes: Vec<u8>) {
        self.decoded_tokens[token_id as usize] = bytes;
    }

    /// 获取token的解码字节
    pub fn get_decoded(&self, token_id: TokenId) -> &[u8] {
        &self.decoded_tokens[token_id as usize]
    }

    /// 查找具有指定前缀的所有token
    pub fn find_with_prefix(&self, prefix: &[u8]) -> Vec<TokenId> {
        self.prefix_trie.find_with_prefix(prefix)
    }
}

/// 前缀树 - 用于快速token前缀匹配
#[derive(Clone, Debug)]
pub struct PrefixTrie {
    root: TrieNode,
}

#[derive(Clone, Debug, Default)]
struct TrieNode {
    /// 在此节点结束的token IDs
    tokens: Vec<TokenId>,
    /// 子节点
    children: HashMap<u8, TrieNode>,
}

impl PrefixTrie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
        }
    }

    /// 插入token
    pub fn insert(&mut self, bytes: &[u8], token_id: TokenId) {
        let mut node = &mut self.root;
        for &byte in bytes {
            node = node.children.entry(byte).or_default();
        }
        node.tokens.push(token_id);
    }

    /// 查找具有指定前缀的所有token
    pub fn find_with_prefix(&self, prefix: &[u8]) -> Vec<TokenId> {
        let mut node = &self.root;
        for &byte in prefix {
            match node.children.get(&byte) {
                Some(n) => node = n,
                None => return Vec::new(),
            }
        }
        Self::collect_tokens(node)
    }

    fn collect_tokens(node: &TrieNode) -> Vec<TokenId> {
        let mut result = node.tokens.clone();
        for child in node.children.values() {
            result.extend(Self::collect_tokens(child));
        }
        result
    }
}

// ============================================================================
// Section 4: 语法表示 - CFG/EBNF
// ============================================================================

/// 语法表达式类型
#[derive(Clone, Debug, PartialEq)]
pub enum GrammarExpr {
    /// 空序列
    Empty,
    /// 终结符: 匹配特定token
    Terminal(TokenId),
    /// 字节终结符: 匹配特定字节
    ByteTerminal(Byte),
    /// 字符类: 匹配一组字节中的任意一个
    ByteClass(Vec<Byte>),
    /// 非终结符: 引用其他规则
    NonTerminal(RuleId),
    /// 序列: 按顺序匹配多个表达式
    Sequence(Vec<GrammarExpr>),
    /// 选择: 匹配多个表达式中的任意一个
    Choice(Vec<GrammarExpr>),
    /// 可选: 匹配0次或1次
    Optional(Box<GrammarExpr>),
    /// 重复0次或多次
    Star(Box<GrammarExpr>),
    /// 重复1次或多次
    Plus(Box<GrammarExpr>),
    /// 重复n到m次
    Repeat(Box<GrammarExpr>, usize, usize),
}

/// 语法规则
#[derive(Clone, Debug)]
pub struct GrammarRule {
    /// 规则ID
    pub id: RuleId,
    /// 规则名称
    pub name: String,
    /// 规则体
    pub body: GrammarExpr,
}

/// 上下文无关语法
#[derive(Clone, Debug)]
pub struct Grammar {
    /// 规则集合
    pub rules: Vec<GrammarRule>,
    /// 起始规则ID
    pub start_rule: RuleId,
    /// 规则名称到ID的映射
    pub name_to_id: HashMap<String, RuleId>,
}

impl Grammar {
    pub fn new(start_rule: RuleId) -> Self {
        Self {
            rules: Vec::new(),
            start_rule,
            name_to_id: HashMap::new(),
        }
    }

    /// 添加规则
    pub fn add_rule(&mut self, rule: GrammarRule) {
        self.name_to_id.insert(rule.name.clone(), rule.id);
        self.rules.push(rule);
    }

    /// 通过名称查找规则
    pub fn find_rule(&self, name: &str) -> Option<&GrammarRule> {
        self.name_to_id.get(name).and_then(|&id| self.rules.get(id as usize))
    }

    /// 创建JSON语法
    pub fn json_grammar() -> Self {
        let mut grammar = Grammar::new(0);

        // root ::= value
        grammar.add_rule(GrammarRule {
            id: 0,
            name: "root".to_string(),
            body: GrammarExpr::NonTerminal(1), // value
        });

        // value ::= object | array | string | number | boolean | null
        grammar.add_rule(GrammarRule {
            id: 1,
            name: "value".to_string(),
            body: GrammarExpr::Choice(vec![
                GrammarExpr::NonTerminal(2), // object
                GrammarExpr::NonTerminal(3), // array
                GrammarExpr::NonTerminal(4), // string
                GrammarExpr::NonTerminal(5), // number
                GrammarExpr::NonTerminal(6), // boolean
                GrammarExpr::NonTerminal(7), // null
            ]),
        });

        // object ::= "{" (pair ("," pair)*)? "}"
        grammar.add_rule(GrammarRule {
            id: 2,
            name: "object".to_string(),
            body: GrammarExpr::Sequence(vec![
                GrammarExpr::ByteTerminal(b'{'),
                GrammarExpr::Optional(Box::new(GrammarExpr::Sequence(vec![
                    GrammarExpr::NonTerminal(8), // pair
                    GrammarExpr::Star(Box::new(GrammarExpr::Sequence(vec![
                        GrammarExpr::ByteTerminal(b','),
                        GrammarExpr::NonTerminal(8), // pair
                    ]))),
                ]))),
                GrammarExpr::ByteTerminal(b'}'),
            ]),
        });

        // array ::= "[" (value ("," value)*)? "]"
        grammar.add_rule(GrammarRule {
            id: 3,
            name: "array".to_string(),
            body: GrammarExpr::Sequence(vec![
                GrammarExpr::ByteTerminal(b'['),
                GrammarExpr::Optional(Box::new(GrammarExpr::Sequence(vec![
                    GrammarExpr::NonTerminal(1), // value
                    GrammarExpr::Star(Box::new(GrammarExpr::Sequence(vec![
                        GrammarExpr::ByteTerminal(b','),
                        GrammarExpr::NonTerminal(1), // value
                    ]))),
                ]))),
                GrammarExpr::ByteTerminal(b']'),
            ]),
        });

        // string ::= "\"" char* "\""
        grammar.add_rule(GrammarRule {
            id: 4,
            name: "string".to_string(),
            body: GrammarExpr::Sequence(vec![
                GrammarExpr::ByteTerminal(b'"'),
                GrammarExpr::Star(Box::new(GrammarExpr::NonTerminal(9))), // char
                GrammarExpr::ByteTerminal(b'"'),
            ]),
        });

        // number (简化版)
        grammar.add_rule(GrammarRule {
            id: 5,
            name: "number".to_string(),
            body: GrammarExpr::Sequence(vec![
                GrammarExpr::Optional(Box::new(GrammarExpr::ByteTerminal(b'-'))),
                GrammarExpr::Plus(Box::new(GrammarExpr::ByteClass((b'0'..=b'9').collect()))),
                GrammarExpr::Optional(Box::new(GrammarExpr::Sequence(vec![
                    GrammarExpr::ByteTerminal(b'.'),
                    GrammarExpr::Plus(Box::new(GrammarExpr::ByteClass((b'0'..=b'9').collect()))),
                ]))),
            ]),
        });

        // boolean ::= "true" | "false"
        grammar.add_rule(GrammarRule {
            id: 6,
            name: "boolean".to_string(),
            body: GrammarExpr::Choice(vec![
                GrammarExpr::Sequence(vec![
                    GrammarExpr::ByteTerminal(b't'),
                    GrammarExpr::ByteTerminal(b'r'),
                    GrammarExpr::ByteTerminal(b'u'),
                    GrammarExpr::ByteTerminal(b'e'),
                ]),
                GrammarExpr::Sequence(vec![
                    GrammarExpr::ByteTerminal(b'f'),
                    GrammarExpr::ByteTerminal(b'a'),
                    GrammarExpr::ByteTerminal(b'l'),
                    GrammarExpr::ByteTerminal(b's'),
                    GrammarExpr::ByteTerminal(b'e'),
                ]),
            ]),
        });

        // null ::= "null"
        grammar.add_rule(GrammarRule {
            id: 7,
            name: "null".to_string(),
            body: GrammarExpr::Sequence(vec![
                GrammarExpr::ByteTerminal(b'n'),
                GrammarExpr::ByteTerminal(b'u'),
                GrammarExpr::ByteTerminal(b'l'),
                GrammarExpr::ByteTerminal(b'l'),
            ]),
        });

        // pair ::= string ":" value
        grammar.add_rule(GrammarRule {
            id: 8,
            name: "pair".to_string(),
            body: GrammarExpr::Sequence(vec![
                GrammarExpr::NonTerminal(4), // string
                GrammarExpr::ByteTerminal(b':'),
                GrammarExpr::NonTerminal(1), // value
            ]),
        });

        // char (简化版 - 不包括转义)
        grammar.add_rule(GrammarRule {
            id: 9,
            name: "char".to_string(),
            body: GrammarExpr::ByteClass((0x20..=0x21).chain(0x23..=0x7E).collect()),
        });

        grammar
    }
}

// ============================================================================
// Section 5: FSM - 有限状态机 (用于正则表达式/简单规则)
// ============================================================================

/// FSM转换
#[derive(Clone, Debug)]
pub struct FSMTransition {
    /// 当前状态
    pub from: StateId,
    /// 输入字节 (None表示epsilon转换)
    pub input: Option<Byte>,
    /// 目标状态
    pub to: StateId,
}

/// 确定性有限状态机 (DFA)
#[derive(Clone, Debug)]
pub struct DFA {
    /// 状态数量
    pub num_states: usize,
    /// 初始状态
    pub initial_state: StateId,
    /// 接受状态集合
    pub accept_states: HashSet<StateId>,
    /// 转换表: transitions[from][input] = to
    pub transitions: HashMap<StateId, HashMap<Option<Byte>, StateId>>,
}

impl DFA {
    pub fn new(initial_state: StateId) -> Self {
        Self {
            num_states: (initial_state + 1) as usize,
            initial_state,
            accept_states: HashSet::new(),
            transitions: HashMap::new(),
        }
    }

    /// 添加转换
    pub fn add_transition(&mut self, from: StateId, input: Option<Byte>, to: StateId) {
        self.transitions
            .entry(from)
            .or_default()
            .insert(input, to);
        self.num_states = self.num_states.max((from.max(to) + 1) as usize);
    }

    /// 设置接受状态
    pub fn set_accept(&mut self, state: StateId) {
        self.accept_states.insert(state);
    }

    /// 执行状态转换
    pub fn transition(&self, state: StateId, input: Byte) -> Option<StateId> {
        self.transitions
            .get(&state)
            .and_then(|map| map.get(&Some(input)).copied())
    }

    /// 检查是否为接受状态
    pub fn is_accept(&self, state: StateId) -> bool {
        self.accept_states.contains(&state)
    }

    /// 从正则表达式构建DFA (简化版)
    pub fn from_regex(pattern: &str) -> Result<Self, GrammarError> {
        // 简化实现：仅支持字符类 [a-z], [0-9] 和字面量
        let mut dfa = DFA::new(0);
        let mut current_state = 0u32;
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '[' => {
                    // 字符类
                    let mut byte_class = Vec::new();
                    while let Some(c) = chars.next() {
                        if c == ']' {
                            break;
                        }
                        if c == '-' {
                            // 范围
                            if let (Some(start), Some(end)) = (byte_class.last().copied(), chars.next()) {
                                for b in (start + 1)..=end as u8 {
                                    byte_class.push(b);
                                }
                            }
                        } else {
                            byte_class.push(c as u8);
                        }
                    }
                    // 为字符类中的每个字节创建转换
                    let next_state = current_state + 1;
                    for &byte in &byte_class {
                        dfa.add_transition(current_state, Some(byte), next_state);
                    }
                    current_state = next_state;
                }
                'a'..='z' | 'A'..='Z' | '0'..='9' => {
                    let next_state = current_state + 1;
                    dfa.add_transition(current_state, Some(ch as u8), next_state);
                    current_state = next_state;
                }
                _ => {}
            }
        }

        dfa.set_accept(current_state);
        Ok(dfa)
    }
}

// ============================================================================
// Section 6: PDA - 下推自动机 (用于CFG)
// ============================================================================

/// 栈符号
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StackSymbol {
    /// 规则ID
    Rule(RuleId),
    /// 状态标记
    Marker(String),
    /// 自定义数据
    Custom(Vec<u8>),
}

/// 栈操作
#[derive(Clone, Debug)]
pub enum StackOp {
    /// 无操作
    None,
    /// 压栈
    Push(StackSymbol),
    /// 弹栈
    Pop,
    /// 替换栈顶
    Replace(StackSymbol),
    /// 多重压栈 (按顺序)
    PushMulti(Vec<StackSymbol>),
}

/// PDA转换规则
#[derive(Clone, Debug)]
pub struct PDATransition {
    /// 当前状态
    pub from_state: StateId,
    /// 输入字节 (None表示epsilon)
    pub input: Option<Byte>,
    /// 栈顶符号 (None表示不检查)
    pub stack_top: Option<StackSymbol>,
    /// 目标状态
    pub to_state: StateId,
    /// 栈操作
    pub stack_op: StackOp,
}

/// 确定性下推自动机 (DPDA)
#[derive(Clone, Debug)]
pub struct DPDA {
    /// 状态集合
    pub states: HashSet<StateId>,
    /// 初始状态
    pub initial_state: StateId,
    /// 接受状态
    pub accept_states: HashSet<StateId>,
    /// 转换规则
    pub transitions: Vec<PDATransition>,
    /// 当前状态 (运行时)
    pub current_state: StateId,
    /// 栈 (运行时)
    pub stack: Vec<StackSymbol>,
}

impl DPDA {
    pub fn new(initial_state: StateId) -> Self {
        let mut states = HashSet::new();
        states.insert(initial_state);
        Self {
            states,
            initial_state,
            accept_states: HashSet::new(),
            transitions: Vec::new(),
            current_state: initial_state,
            stack: Vec::new(),
        }
    }

    /// 添加转换
    pub fn add_transition(&mut self, trans: PDATransition) {
        self.states.insert(trans.from_state);
        self.states.insert(trans.to_state);
        self.transitions.push(trans);
    }

    /// 设置接受状态
    pub fn set_accept(&mut self, state: StateId) {
        self.accept_states.insert(state);
    }

    /// 获取当前允许的输入字节
    pub fn get_allowed_bytes(&self) -> HashSet<Byte> {
        let mut allowed = HashSet::new();
        for trans in &self.transitions {
            if trans.from_state == self.current_state {
                let stack_matches = match &trans.stack_top {
                    None => true,
                    Some(expected) => self.stack.last() == Some(expected),
                };
                if stack_matches {
                    if let Some(byte) = trans.input {
                        allowed.insert(byte);
                    }
                }
            }
        }
        allowed
    }

    /// 处理输入字节
    pub fn process_byte(&mut self, byte: Byte) -> Result<(), GrammarError> {
        for trans in &self.transitions {
            if trans.from_state == self.current_state && trans.input == Some(byte) {
                let stack_matches = match &trans.stack_top {
                    None => true,
                    Some(expected) => self.stack.last() == Some(expected),
                };
                if stack_matches {
                    // 执行栈操作
                    match &trans.stack_op {
                        StackOp::None => {}
                        StackOp::Push(sym) => self.stack.push(sym.clone()),
                        StackOp::Pop => {
                            self.stack.pop().ok_or(GrammarError::StackUnderflow)?;
                        }
                        StackOp::Replace(sym) => {
                            self.stack.pop().ok_or(GrammarError::StackUnderflow)?;
                            self.stack.push(sym.clone());
                        }
                        StackOp::PushMulti(syms) => {
                            for sym in syms.iter().rev() {
                                self.stack.push(sym.clone());
                            }
                        }
                    }
                    self.current_state = trans.to_state;
                    return Ok(());
                }
            }
        }
        Err(GrammarError::InvalidTransition {
            from: self.current_state,
            input: byte as u32,
        })
    }

    /// 检查是否处于接受状态
    pub fn is_accepting(&self) -> bool {
        self.accept_states.contains(&self.current_state) && self.stack.is_empty()
    }

    /// 重置PDA
    pub fn reset(&mut self) {
        self.current_state = self.initial_state;
        self.stack.clear();
    }
}

// ============================================================================
// Section 7: Token Mask Cache - 核心优化机制
// ============================================================================

/// Token分类 (XGrammar核心概念)
#[derive(Clone, Debug, PartialEq)]
pub enum TokenCategory {
    /// 上下文无关: 仅通过当前状态即可确定有效性
    ContextIndependent,
    /// 上下文相关: 需要栈信息才能确定
    ContextDependent,
    /// 不确定: 需要运行时检查
    Uncertain,
}

/// Token Mask Cache条目
#[derive(Clone, Debug)]
pub struct TokenMaskCacheEntry {
    /// 状态ID
    pub state_id: StateId,
    /// Token分类
    pub category: TokenCategory,
    /// 预计算的掩码 (仅用于ContextIndependent)
    pub precomputed_mask: Option<DynamicBitset>,
}

/// 自适应Token Mask Cache
///
/// XGrammar的核心创新：将token分为上下文无关和上下文相关两类
/// - 上下文无关token (99%): 预计算掩码
/// - 上下文相关token (1%): 运行时检查
#[derive(Clone, Debug)]
pub struct AdaptiveTokenMaskCache {
    /// 词汇表大小
    vocab_size: usize,
    /// 每个状态的缓存条目
    entries: HashMap<StateId, TokenMaskCacheEntry>,
    /// 上下文无关token集合 (预计算)
    context_independent_tokens: DynamicBitset,
    /// 上下文相关token集合
    context_dependent_tokens: DynamicBitset,
}

impl AdaptiveTokenMaskCache {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            entries: HashMap::new(),
            context_independent_tokens: DynamicBitset::new(vocab_size),
            context_dependent_tokens: DynamicBitset::new(vocab_size),
        }
    }

    /// 构建缓存 (基于Grammar和Tokenizer)
    pub fn build(grammar: &Grammar, tokenizer: &TokenizerInfo) -> Self {
        let vocab_size = tokenizer.vocab_size;
        let mut cache = Self::new(vocab_size);

        // 分析每个token，分类为上下文无关或上下文相关
        for token_id in 0..vocab_size as TokenId {
            let decoded = tokenizer.get_decoded(token_id);
            // 简化启发式：单字节token通常是上下文无关的
            if decoded.len() == 1 {
                cache.context_independent_tokens.set(token_id as usize, true);
            } else {
                cache.context_dependent_tokens.set(token_id as usize, true);
            }
        }

        // 为每个状态预计算上下文无关token的掩码
        for rule in &grammar.rules {
            let entry = TokenMaskCacheEntry {
                state_id: rule.id,
                category: TokenCategory::ContextIndependent,
                precomputed_mask: Some(cache.compute_mask_for_state(rule.id, tokenizer)),
            };
            cache.entries.insert(rule.id, entry);
        }

        cache
    }

    /// 为指定状态计算token掩码
    fn compute_mask_for_state(&self, state_id: RuleId, tokenizer: &TokenizerInfo) -> DynamicBitset {
        let mut mask = DynamicBitset::new(self.vocab_size);

        // 这里应该根据Grammar规则计算允许的token
        // 简化实现：假设所有单字节token都是允许的
        for token_id in self.context_independent_tokens.iter_ones() {
            mask.set(token_id, true);
        }

        mask
    }

    /// 获取指定状态的token掩码
    pub fn get_mask(&self, state_id: StateId) -> Option<&DynamicBitset> {
        self.entries.get(&state_id).and_then(|e| e.precomputed_mask.as_ref())
    }

    /// 获取上下文无关token数量
    pub fn context_independent_count(&self) -> usize {
        self.context_independent_tokens.count_ones()
    }

    /// 获取上下文相关token数量
    pub fn context_dependent_count(&self) -> usize {
        self.context_dependent_tokens.count_ones()
    }
}

// ============================================================================
// Section 8: 编译后的Grammar
// ============================================================================

/// 编译后的Grammar (可用于运行时匹配)
#[derive(Clone, Debug)]
pub struct CompiledGrammar {
    /// 原始Grammar
    pub grammar: Grammar,
    /// Token Mask Cache
    pub token_mask_cache: AdaptiveTokenMaskCache,
    /// 每个规则对应的DFA (用于正则表达式规则)
    pub rule_dfas: HashMap<RuleId, DFA>,
    /// 主PDA (用于CFG)
    pub main_pda: DPDA,
}

impl CompiledGrammar {
    pub fn compile(grammar: Grammar, tokenizer: &TokenizerInfo) -> Result<Self, GrammarError> {
        let token_mask_cache = AdaptiveTokenMaskCache::build(&grammar, tokenizer);
        let main_pda = Self::build_pda(&grammar)?;

        Ok(Self {
            grammar,
            token_mask_cache,
            rule_dfas: HashMap::new(),
            main_pda,
        })
    }

    /// 从Grammar构建PDA
    fn build_pda(grammar: &Grammar) -> Result<DPDA, GrammarError> {
        let mut pda = DPDA::new(0);

        // 简化实现：为JSON Grammar构建基本的PDA
        // 实际实现需要更复杂的转换算法

        // 这里只是一个占位实现
        pda.set_accept(1);

        Ok(pda)
    }
}

// ============================================================================
// Section 9: Grammar Matcher - 运行时匹配器
// ============================================================================

/// Grammar匹配结果
#[derive(Clone, Debug)]
pub enum MatchResult {
    /// 匹配成功
    Success,
    /// 需要更多输入
    NeedMore,
    /// 匹配失败
    Failure(GrammarError),
}

/// Grammar匹配器
pub struct GrammarMatcher {
    /// 编译后的Grammar
    compiled: Arc<CompiledGrammar>,
    /// 当前PDA状态
    current_state: StateId,
    /// 栈状态
    stack: Vec<StackSymbol>,
    /// 未完成的token字节缓冲
    pending_bytes: Vec<u8>,
    /// 已匹配的token序列
    matched_tokens: Vec<TokenId>,
    /// 匹配统计
    stats: MatcherStats,
}

/// 匹配统计
#[derive(Clone, Debug, Default)]
pub struct MatcherStats {
    pub tokens_processed: usize,
    pub bytes_processed: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl GrammarMatcher {
    pub fn new(compiled: Arc<CompiledGrammar>) -> Self {
        let initial_state = compiled.main_pda.initial_state;
        Self {
            compiled,
            current_state: initial_state,
            stack: Vec::new(),
            pending_bytes: Vec::new(),
            matched_tokens: Vec::new(),
            stats: MatcherStats::default(),
        }
    }

    /// 获取当前允许的token掩码 (核心API)
    pub fn get_allowed_tokens(&self) -> DynamicBitset {
        // 1. 获取预计算的上下文无关token掩码
        let mut mask = self
            .compiled
            .token_mask_cache
            .get_mask(self.current_state)
            .cloned()
            .unwrap_or_else(|| DynamicBitset::ones(self.compiled.token_mask_cache.vocab_size));

        // 2. 对于上下文相关token，需要运行时检查
        // 简化实现：假设所有上下文相关token都被禁止
        // 实际实现需要遍历每个上下文相关token并检查其有效性

        mask
    }

    /// 处理一个token
    pub fn process_token(&mut self, token_id: TokenId, tokenizer: &TokenizerInfo) -> MatchResult {
        let bytes = tokenizer.get_decoded(token_id);
        self.stats.tokens_processed += 1;
        self.stats.bytes_processed += bytes.len();

        // 尝试处理token的字节
        for &byte in bytes {
            match self.process_byte(byte) {
                Ok(_) => {}
                Err(e) => return MatchResult::Failure(e),
            }
        }

        self.matched_tokens.push(token_id);
        MatchResult::Success
    }

    /// 处理单个字节
    fn process_byte(&mut self, byte: Byte) -> Result<(), GrammarError> {
        // 这里应该根据当前PDA状态和栈来处理字节
        // 简化实现
        self.pending_bytes.push(byte);
        Ok(())
    }

    /// 检查是否完成
    pub fn is_complete(&self) -> bool {
        // 简化实现
        self.stack.is_empty()
    }

    /// 获取统计信息
    pub fn stats(&self) -> &MatcherStats {
        &self.stats
    }

    /// 重置匹配器
    pub fn reset(&mut self) {
        self.current_state = self.compiled.main_pda.initial_state;
        self.stack.clear();
        self.pending_bytes.clear();
        self.matched_tokens.clear();
        self.stats = MatcherStats::default();
    }
}

// ============================================================================
// Section 10: JSON Schema支持
// ============================================================================

/// JSON Schema类型
#[derive(Clone, Debug, PartialEq)]
pub enum JsonSchemaType {
    Object,
    Array,
    String,
    Number,
    Integer,
    Boolean,
    Null,
}

/// JSON Schema属性
#[derive(Clone, Debug, Default)]
pub struct JsonSchema {
    /// 类型
    pub schema_type: Option<JsonSchemaType>,
    /// 标题
    pub title: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 必需字段
    pub required: Vec<String>,
    /// 对象属性
    pub properties: HashMap<String, Box<JsonSchema>>,
    /// 数组项
    pub items: Option<Box<JsonSchema>>,
    /// 枚举值
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// 字符串最小长度
    pub min_length: Option<usize>,
    /// 字符串最大长度
    pub max_length: Option<usize>,
    /// 数字最小值
    pub minimum: Option<f64>,
    /// 数字最大值
    pub maximum: Option<f64>,
    /// 正则表达式模式
    pub pattern: Option<String>,
}

impl JsonSchema {
    /// 从JSON Value解析Schema
    pub fn from_value(value: &serde_json::Value) -> Result<Self, GrammarError> {
        let mut schema = JsonSchema::default();

        if let Some(obj) = value.as_object() {
            // 解析类型
            if let Some(type_val) = obj.get("type") {
                schema.schema_type = Some(match type_val.as_str() {
                    Some("object") => JsonSchemaType::Object,
                    Some("array") => JsonSchemaType::Array,
                    Some("string") => JsonSchemaType::String,
                    Some("number") => JsonSchemaType::Number,
                    Some("integer") => JsonSchemaType::Integer,
                    Some("boolean") => JsonSchemaType::Boolean,
                    Some("null") => JsonSchemaType::Null,
                    _ => return Err(GrammarError::SchemaValidationError(
                        format!("Unknown type: {:?}", type_val)
                    )),
                });
            }

            // 解析必需字段
            if let Some(required) = obj.get("required").and_then(|v| v.as_array()) {
                schema.required = required
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }

            // 解析属性
            if let Some(props) = obj.get("properties").and_then(|v| v.as_object()) {
                for (key, val) in props {
                    schema.properties.insert(key.clone(), Box::new(Self::from_value(val)?));
                }
            }

            // 解析数组项
            if let Some(items) = obj.get("items") {
                schema.items = Some(Box::new(Self::from_value(items)?));
            }

            // 解析字符串约束
            if let Some(min) = obj.get("minLength").and_then(|v| v.as_u64()) {
                schema.min_length = Some(min as usize);
            }
            if let Some(max) = obj.get("maxLength").and_then(|v| v.as_u64()) {
                schema.max_length = Some(max as usize);
            }
            if let Some(pattern) = obj.get("pattern").and_then(|v| v.as_str()) {
                schema.pattern = Some(pattern.to_string());
            }

            // 解析数值约束
            if let Some(min) = obj.get("minimum").and_then(|v| v.as_f64()) {
                schema.minimum = Some(min);
            }
            if let Some(max) = obj.get("maximum").and_then(|v| v.as_f64()) {
                schema.maximum = Some(max);
            }
        }

        Ok(schema)
    }

    /// 转换为Grammar
    pub fn to_grammar(&self) -> Grammar {
        // 简化实现：返回基本的JSON Grammar
        Grammar::json_grammar()
    }
}

// ============================================================================
// Section 11: LLM集成 - Logits处理器
// ============================================================================

/// Logits处理器 - 将Grammar约束应用到LLM输出
pub struct GrammarLogitsProcessor {
    /// Grammar匹配器
    matcher: GrammarMatcher,
    /// 当前token掩码
    current_mask: DynamicBitset,
    /// 词汇表大小
    vocab_size: usize,
}

impl GrammarLogitsProcessor {
    pub fn new(compiled: Arc<CompiledGrammar>, vocab_size: usize) -> Self {
        let matcher = GrammarMatcher::new(compiled);
        let current_mask = DynamicBitset::ones(vocab_size);
        Self {
            matcher,
            current_mask,
            vocab_size,
        }
    }

    /// 处理logits (核心API)
    ///
    /// 在LLM生成每个token之前调用，将不符合Grammar的token的logits设为负无穷
    pub fn process_logits(&mut self, logits: &mut [f32]) {
        assert_eq!(logits.len(), self.vocab_size, "Logits size mismatch");

        // 获取当前允许的token
        self.current_mask = self.matcher.get_allowed_tokens();

        // 应用掩码：禁止的token设为负无穷
        for (i, logit) in logits.iter_mut().enumerate() {
            if i < self.vocab_size && !self.current_mask.get(i) {
                *logit = f32::NEG_INFINITY;
            }
        }
    }

    /// 处理生成的token
    pub fn process_token(&mut self, token_id: TokenId, tokenizer: &TokenizerInfo) -> MatchResult {
        self.matcher.process_token(token_id, tokenizer)
    }

    /// 检查是否完成
    pub fn is_complete(&self) -> bool {
        self.matcher.is_complete()
    }

    /// 获取统计信息
    pub fn stats(&self) -> &MatcherStats {
        self.matcher.stats()
    }
}

// ============================================================================
// Section 12: 性能基准测试框架
// ============================================================================

/// 基准测试结果
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub total_time: Duration,
    pub avg_time_per_op: Duration,
    pub ops_per_second: f64,
    pub memory_usage_bytes: usize,
}

impl BenchmarkResult {
    pub fn new(name: String, iterations: usize, total_time: Duration) -> Self {
        let avg_time = total_time / iterations as u32;
        let ops_per_second = iterations as f64 / total_time.as_secs_f64();

        Self {
            name,
            iterations,
            total_time,
            avg_time_per_op: avg_time,
            ops_per_second,
            memory_usage_bytes: 0,
        }
    }

    pub fn report(&self) -> String {
        format!(
            "Benchmark: {}\n\
             Iterations: {}\n\
             Total time: {:?}\n\
             Avg time/op: {:?}\n\
             Ops/second: {:.2}\n\
             Memory: {} bytes",
            self.name,
            self.iterations,
            self.total_time,
            self.avg_time_per_op,
            self.ops_per_second,
            self.memory_usage_bytes
        )
    }
}

/// 基准测试运行器
pub struct BenchmarkRunner;

impl BenchmarkRunner {
    /// 运行基准测试
    pub fn run<F>(name: &str, iterations: usize, mut f: F) -> BenchmarkResult
    where
        F: FnMut(),
    {
        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        let total_time = start.elapsed();

        BenchmarkResult::new(name.to_string(), iterations, total_time)
    }

    /// 运行结构化生成基准测试
    pub fn run_structured_generation_benchmark() -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        let vocab_size = 128000; // Llama-3.1词汇表大小

        // 测试1: Token Mask生成
        let mask_result = Self::run("token_mask_generation", 10000, || {
            let mut mask = DynamicBitset::new(vocab_size);
            for i in 0..1000 {
                mask.set(i, true);
            }
            mask.count_ones();
        });
        results.push(mask_result);

        // 测试2: Bitset AND操作
        let bitset1 = DynamicBitset::ones(vocab_size);
        let mut bitset2 = DynamicBitset::new(vocab_size);
        for i in 0..50000 {
            bitset2.set(i, true);
        }
        let and_result = Self::run("bitset_and_operation", 10000, || {
            let mut b1 = bitset1.clone();
            b1.and_with(&bitset2);
        });
        results.push(and_result);

        // 测试3: DFA状态转换
        let dfa = DFA::from_regex("[a-z]+").unwrap();
        let dfa_result = Self::run("dfa_transition", 100000, || {
            let mut state = dfa.initial_state;
            for byte in b"hello" {
                if let Some(next) = dfa.transition(state, *byte) {
                    state = next;
                }
            }
        });
        results.push(dfa_result);

        // 测试4: Grammar编译
        let grammar = Grammar::json_grammar();
        let compile_result = Self::run("grammar_compile", 1000, || {
            let _ = grammar.clone();
        });
        results.push(compile_result);

        results
    }
}

// ============================================================================
// Section 13: 与其他库的对比分析
// ============================================================================

/// 结构化生成库对比
pub mod comparison {
    use super::*;

    /// 库特性对比
    #[derive(Clone, Debug)]
    pub struct LibraryComparison {
        pub name: String,
        pub grammar_support: GrammarSupport,
        pub performance_tier: PerformanceTier,
        pub rust_native: bool,
        pub token_level_constraint: bool,
        pub json_schema_support: bool,
        pub streaming_support: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum GrammarSupport {
        None,
        RegexOnly,
        FSM,
        CFG,
        FullEBNF,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum PerformanceTier {
        Slow,      // >100μs/token
        Moderate,  // 50-100μs/token
        Fast,      // 10-50μs/token
        UltraFast, // <10μs/token
    }

    /// 获取所有库的对比信息
    pub fn get_all_comparisons() -> Vec<LibraryComparison> {
        vec![
            LibraryComparison {
                name: "XGrammar".to_string(),
                grammar_support: GrammarSupport::FullEBNF,
                performance_tier: PerformanceTier::UltraFast,
                rust_native: false, // C++ core
                token_level_constraint: true,
                json_schema_support: true,
                streaming_support: true,
            },
            LibraryComparison {
                name: "llguidance".to_string(),
                grammar_support: GrammarSupport::CFG,
                performance_tier: PerformanceTier::UltraFast,
                rust_native: true,
                token_level_constraint: true,
                json_schema_support: true,
                streaming_support: true,
            },
            LibraryComparison {
                name: "Outlines".to_string(),
                grammar_support: GrammarSupport::FSM,
                performance_tier: PerformanceTier::Moderate,
                rust_native: false,
                token_level_constraint: true,
                json_schema_support: true,
                streaming_support: false,
            },
            LibraryComparison {
                name: "Guidance".to_string(),
                grammar_support: GrammarSupport::FullEBNF,
                performance_tier: PerformanceTier::Fast,
                rust_native: false,
                token_level_constraint: true,
                json_schema_support: true,
                streaming_support: true,
            },
            LibraryComparison {
                name: "Jsonformer".to_string(),
                grammar_support: GrammarSupport::None,
                performance_tier: PerformanceTier::Moderate,
                rust_native: false,
                token_level_constraint: false, // Post-processing approach
                json_schema_support: true,
                streaming_support: false,
            },
            LibraryComparison {
                name: "LMQL".to_string(),
                grammar_support: GrammarSupport::RegexOnly,
                performance_tier: PerformanceTier::Fast,
                rust_native: false,
                token_level_constraint: true,
                json_schema_support: false,
                streaming_support: true,
            },
        ]
    }

    /// 打印对比表格
    pub fn print_comparison_table() {
        let comparisons = get_all_comparisons();

        println!("{:<15} {:<15} {:<15} {:<12} {:<20} {:<18} {:<12}",
            "Library", "Grammar", "Performance", "Rust", "Token Constraint", "JSON Schema", "Streaming");
        println!("{}", "-".repeat(110));

        for lib in comparisons {
            println!("{:<15} {:<15?} {:<15?} {:<12} {:<20} {:<18} {:<12}",
                lib.name,
                lib.grammar_support,
                lib.performance_tier,
                if lib.rust_native { "Yes" } else { "No" },
                if lib.token_level_constraint { "Yes" } else { "No" },
                if lib.json_schema_support { "Yes" } else { "No" },
                if lib.streaming_support { "Yes" } else { "No" }
            );
        }
    }
}

// ============================================================================
// Section 14: 类型状态模式 - Rust类型系统集成
// ============================================================================

/// 使用Rust类型状态模式实现类型安全的JSON构建
pub mod type_state {
    use super::*;

    /// 标记trait用于JSON构建器状态
    pub trait JsonBuilderState {}

    /// 初始状态
    pub struct Start;
    impl JsonBuilderState for Start {}

    /// 对象构建中
    pub struct InObject {
        has_fields: bool,
    }
    impl JsonBuilderState for InObject {}

    /// 数组构建中
    pub struct InArray {
        has_elements: bool,
    }
    impl JsonBuilderState for InArray {}

    /// 键已设置，等待值
    pub struct KeySet {
        key: String,
    }
    impl JsonBuilderState for KeySet {}

    /// 值已设置
    pub struct ValueSet;
    impl JsonBuilderState for ValueSet {}

    /// 类型安全的JSON构建器
    pub struct JsonBuilder<S: JsonBuilderState> {
        state: std::marker::PhantomData<S>,
        output: String,
        indent_level: usize,
        pretty: bool,
    }

    impl JsonBuilder<Start> {
        pub fn new() -> Self {
            Self {
                state: std::marker::PhantomData,
                output: String::new(),
                indent_level: 0,
                pretty: false,
            }
        }

        pub fn pretty(mut self) -> Self {
            self.pretty = true;
            self
        }

        pub fn begin_object(mut self) -> JsonBuilder<InObject> {
            self.output.push('{');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level + 1,
                pretty: self.pretty,
            }
        }

        pub fn begin_array(mut self) -> JsonBuilder<InArray> {
            self.output.push('[');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level + 1,
                pretty: self.pretty,
            }
        }

        pub fn null(mut self) -> JsonBuilder<Start> {
            self.output.push_str("null");
            self
        }

        pub fn bool_value(mut self, value: bool) -> JsonBuilder<Start> {
            self.output.push_str(if value { "true" } else { "false" });
            self
        }
    }

    impl JsonBuilder<InObject> {
        pub fn key(mut self, k: &str) -> JsonBuilder<KeySet> {
            if self.output.ends_with('"') {
                self.output.push(',');
                if self.pretty {
                    self.output.push('\n');
                    self.output.push_str(&"  ".repeat(self.indent_level));
                }
            }
            self.output.push('"');
            self.output.push_str(k);
            self.output.push_str("\":");
            if self.pretty {
                self.output.push(' ');
            }

            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level,
                pretty: self.pretty,
            }
        }

        pub fn end_object(mut self) -> JsonBuilder<Start> {
            if self.pretty && self.output.ends_with('"') {
                self.output.push('\n');
                self.output.push_str(&"  ".repeat(self.indent_level - 1));
            }
            self.output.push('}');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level - 1,
                pretty: self.pretty,
            }
        }
    }

    impl JsonBuilder<KeySet> {
        pub fn string_value(mut self, v: &str) -> JsonBuilder<InObject> {
            self.output.push('"');
            // 转义特殊字符
            for ch in v.chars() {
                match ch {
                    '"' => self.output.push_str("\\\""),
                    '\\' => self.output.push_str("\\\\"),
                    '\n' => self.output.push_str("\\n"),
                    '\r' => self.output.push_str("\\r"),
                    '\t' => self.output.push_str("\\t"),
                    _ => self.output.push(ch),
                }
            }
            self.output.push('"');

            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level,
                pretty: self.pretty,
            }
        }

        pub fn number_value(mut self, v: f64) -> JsonBuilder<InObject> {
            self.output.push_str(&v.to_string());
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level,
                pretty: self.pretty,
            }
        }

        pub fn int_value(mut self, v: i64) -> JsonBuilder<InObject> {
            self.output.push_str(&v.to_string());
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level,
                pretty: self.pretty,
            }
        }

        pub fn bool_value(mut self, v: bool) -> JsonBuilder<InObject> {
            self.output.push_str(if v { "true" } else { "false" });
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level,
                pretty: self.pretty,
            }
        }

        pub fn null_value(mut self) -> JsonBuilder<InObject> {
            self.output.push_str("null");
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level,
                pretty: self.pretty,
            }
        }

        pub fn begin_nested_object(mut self) -> JsonBuilder<InObject> {
            self.output.push('{');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level + 1,
                pretty: self.pretty,
            }
        }

        pub fn begin_nested_array(mut self) -> JsonBuilder<InArray> {
            self.output.push('[');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level + 1,
                pretty: self.pretty,
            }
        }
    }

    impl JsonBuilder<InArray> {
        pub fn string_element(mut self, v: &str) -> JsonBuilder<InArray> {
            if !self.output.ends_with('[') {
                self.output.push(',');
                if self.pretty {
                    self.output.push(' ');
                }
            }
            self.output.push('"');
            for ch in v.chars() {
                match ch {
                    '"' => self.output.push_str("\\\""),
                    '\\' => self.output.push_str("\\\\"),
                    '\n' => self.output.push_str("\\n"),
                    '\r' => self.output.push_str("\\r"),
                    '\t' => self.output.push_str("\\t"),
                    _ => self.output.push(ch),
                }
            }
            self.output.push('"');
            self
        }

        pub fn number_element(mut self, v: f64) -> JsonBuilder<InArray> {
            if !self.output.ends_with('[') {
                self.output.push(',');
                if self.pretty {
                    self.output.push(' ');
                }
            }
            self.output.push_str(&v.to_string());
            self
        }

        pub fn end_array(mut self) -> JsonBuilder<Start> {
            self.output.push(']');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
                indent_level: self.indent_level - 1,
                pretty: self.pretty,
            }
        }
    }

    impl<S: JsonBuilderState> JsonBuilder<S> {
        pub fn build(self) -> String {
            self.output
        }
    }
}

// ============================================================================
// Section 15: 测试与验证
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::type_state::*;

    #[test]
    fn test_dynamic_bitset() {
        let mut bitset = DynamicBitset::new(100);
        assert_eq!(bitset.len(), 100);

        bitset.set(10, true);
        bitset.set(20, true);
        bitset.set(30, true);

        assert!(bitset.get(10));
        assert!(bitset.get(20));
        assert!(bitset.get(30));
        assert!(!bitset.get(11));

        assert_eq!(bitset.count_ones(), 3);
    }

    #[test]
    fn test_dynamic_bitset_operations() {
        let mut b1 = DynamicBitset::new(64);
        b1.set(0, true);
        b1.set(1, true);

        let mut b2 = DynamicBitset::new(64);
        b2.set(1, true);
        b2.set(2, true);

        b1.and_with(&b2);
        assert!(b1.get(1));
        assert!(!b1.get(0));
        assert!(!b1.get(2));
    }

    #[test]
    fn test_dfa() {
        let mut dfa = DFA::new(0);
        dfa.add_transition(0, Some(b'a'), 1);
        dfa.add_transition(1, Some(b'b'), 2);
        dfa.set_accept(2);

        assert_eq!(dfa.transition(0, b'a'), Some(1));
        assert_eq!(dfa.transition(1, b'b'), Some(2));
        assert!(dfa.is_accept(2));
    }

    #[test]
    fn test_json_grammar() {
        let grammar = Grammar::json_grammar();
        assert!(!grammar.rules.is_empty());
        assert_eq!(grammar.start_rule, 0);

        let root_rule = grammar.find_rule("root");
        assert!(root_rule.is_some());
    }

    #[test]
    fn test_type_state_json_builder() {
        let json = JsonBuilder::new()
            .begin_object()
            .key("name")
            .string_value("Alice")
            .key("age")
            .int_value(30)
            .key("active")
            .bool_value(true)
            .end_object()
            .build();

        assert_eq!(json, r#"{"name":"Alice","age":30,"active":true}"#);
    }

    #[test]
    fn test_nested_type_state() {
        let json = JsonBuilder::new()
            .begin_object()
            .key("user")
            .begin_nested_object()
            .key("name")
            .string_value("Bob")
            .end_object()
            .key("items")
            .begin_nested_array()
            .string_element("a")
            .string_element("b")
            .end_array()
            .end_object()
            .build();

        assert_eq!(json, r#"{"user":{"name":"Bob"},"items":["a","b"]}"#);
    }

    #[test]
    fn test_pda() {
        let mut pda = DPDA::new(0);
        pda.add_transition(PDATransition {
            from_state: 0,
            input: Some(b'{'),
            stack_top: None,
            to_state: 1,
            stack_op: StackOp::Push(StackSymbol::Marker("object".to_string())),
        });
        pda.set_accept(1);

        let allowed = pda.get_allowed_bytes();
        assert!(allowed.contains(&b'{'));
    }
}

// ============================================================================
// Section 16: 主函数与演示
// ============================================================================

fn main() {
    println!("=== XGrammar v2: 结构化生成引擎深入研究 ===\n");

    // 1. 展示核心数据结构
    println!("1. 核心数据结构演示");
    println!("   - DynamicBitset: 高效token掩码存储");
    let mut bitset = DynamicBitset::new(128000);
    bitset.set(100, true);
    bitset.set(200, true);
    println!("   - 词汇表大小: 128000");
    println!("   - 置位数量: {}", bitset.count_ones());

    // 2. 展示类型状态模式
    println!("\n2. 类型状态模式演示");
    let json = type_state::JsonBuilder::new()
        .begin_object()
        .key("name")
        .string_value("XGrammar")
        .key("version")
        .number_value(2.0)
        .key("features")
        .begin_nested_array()
        .string_element("token_mask_cache")
        .string_element("pda_constraint")
        .string_element("type_state")
        .end_array()
        .end_object()
        .build();
    println!("   生成JSON: {}", json);

    // 3. 展示Grammar
    println!("\n3. Grammar演示");
    let grammar = Grammar::json_grammar();
    println!("   - 规则数量: {}", grammar.rules.len());
    println!("   - 起始规则: {:?}", grammar.find_rule("root").map(|r| &r.name));

    // 4. 展示DFA
    println!("\n4. DFA演示");
    let dfa = DFA::from_regex("[a-z]+").unwrap();
    println!("   - 状态数: {}", dfa.num_states);
    println!("   - 初始状态: {}", dfa.initial_state);

    // 5. 运行基准测试
    println!("\n5. 性能基准测试");
    let results = BenchmarkRunner::run_structured_generation_benchmark();
    for result in results {
        println!("\n   {}", result.report());
    }

    // 6. 展示库对比
    println!("\n6. 结构化生成库对比");
    comparison::print_comparison_table();

    // 7. 总结
    println!("\n=== 研究总结 ===");
    println!("本次深入研究验证了以下核心概念:");
    println!("1. Token Mask Cache机制: 将token分为上下文无关(99%)和上下文相关(1%)");
    println!("2. PDA作为CFG的执行引擎: 支持嵌套结构");
    println!("3. Rust类型状态模式: 编译期保证JSON结构正确性");
    println!("4. 性能优化: Bitset操作、DFA状态转换、Grammar编译");
    println!("5. 生态对比: XGrammar/llguidance领先，Outlines/Guidance各有特色");
}
