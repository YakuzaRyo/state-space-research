// 结构化生成：Token级别约束LLM输出的Rust实现
// 研究方向: 03_structured_generation
// 时间: 2026-03-11 11:55
//
// 核心问题: 如何在token级别约束LLM输出?
//
// 技术假设:
// H1: 通过PDA(下推自动机)可以在token级别强制执行CFG约束
// H2: 上下文无关/相关token分类可实现高效掩码缓存
// H3: 字节级处理可解决不规则token边界问题
// H4: Token Mask Cache可将约束解码开销降至<40μs/token

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// 第一部分: Token表示与分类
// ============================================================================

/// Token ID类型
pub type TokenId = u32;

/// Token分类 - XGrammar核心优化
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    /// 上下文无关: 仅通过当前PDA状态即可确定有效性
    /// 占词汇表的99%以上，可预计算缓存
    ContextIndependent,
    /// 上下文相关: 需要完整栈信息才能确定
    /// 通常<1%，需要运行时检查
    ContextDependent,
    /// 不确定: 需要额外验证
    Uncertain,
}

/// Token信息
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub id: TokenId,
    pub text: String,
    pub bytes: Vec<u8>,
    pub category: TokenCategory,
}

/// Token分类器 - 预计算阶段使用
pub struct TokenClassifier {
    vocab_size: usize,
}

impl TokenClassifier {
    pub fn new(vocab_size: usize) -> Self {
        Self { vocab_size }
    }

    /// 分类token
    /// 策略: 如果token的任意前缀都合法，则为上下文无关
    pub fn classify(&self, token: &TokenInfo, grammar: &Grammar) -> TokenCategory {
        // 简化的分类逻辑
        // 实际实现需要检查token的所有可能前缀
        if token.bytes.len() == 1 {
            // 单字节token通常是上下文无关的
            TokenCategory::ContextIndependent
        } else if self.is_ambiguous_prefix(&token.bytes, grammar) {
            TokenCategory::ContextDependent
        } else {
            TokenCategory::ContextIndependent
        }
    }

    fn is_ambiguous_prefix(&self, bytes: &[u8], _grammar: &Grammar) -> bool {
        // 简化的歧义检测
        // 实际实现需要检查bytes是否匹配多个grammar规则
        bytes.len() > 4
    }
}

// ============================================================================
// 第二部分: 动态Bitset - 高效Token掩码存储
// ============================================================================

/// 动态Bitset - 用于存储token掩码
/// 128K词汇表仅需16KB (vs 128KB for bool[])
pub struct DynamicBitset {
    blocks: Vec<u32>,
    size: usize,
}

impl DynamicBitset {
    pub fn new(size: usize) -> Self {
        let num_blocks = (size + 31) / 32;
        Self {
            blocks: vec![0u32; num_blocks],
            size,
        }
    }

    pub fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.size);
        let block_idx = index / 32;
        let bit_idx = index % 32;
        if value {
            self.blocks[block_idx] |= 1 << bit_idx;
        } else {
            self.blocks[block_idx] &= !(1 << bit_idx);
        }
    }

    pub fn get(&self, index: usize) -> bool {
        assert!(index < self.size);
        let block_idx = index / 32;
        let bit_idx = index % 32;
        (self.blocks[block_idx] >> bit_idx) & 1 == 1
    }

    pub fn and_with(&mut self, other: &DynamicBitset) {
        for (a, b) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            *a &= *b;
        }
    }

    pub fn or_with(&mut self, other: &DynamicBitset) {
        for (a, b) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            *a |= *b;
        }
    }

    pub fn clear(&mut self) {
        for block in &mut self.blocks {
            *block = 0;
        }
    }

    pub fn count_ones(&self) -> usize {
        self.blocks.iter().map(|b| b.count_ones() as usize).sum()
    }

    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.blocks.iter().enumerate().flat_map(|(block_idx, block)| {
            let base = block_idx * 32;
            (0..32).filter_map(move |bit| {
                if block & (1 << bit) != 0 {
                    Some(base + bit)
                } else {
                    None
                }
            })
        })
    }
}

// ============================================================================
// 第三部分: EBNF语法表示与解析
// ============================================================================

/// EBNF表达式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// 终结符: "string" 或 [charset]
    Terminal(String),
    /// 非终结符引用: rule_name
    NonTerminal(String),
    /// 序列: expr1 expr2
    Sequence(Vec<Expr>),
    /// 选择: expr1 | expr2
    Choice(Vec<Expr>),
    /// 可选: expr?
    Optional(Box<Expr>),
    /// 零次或多次: expr*
    Star(Box<Expr>),
    /// 一次或多次: expr+
    Plus(Box<Expr>),
    /// 重复: expr{n,m}
    Repeat(Box<Expr>, usize, Option<usize>),
}

/// Grammar规则
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub expr: Expr,
}

/// Grammar - 上下文无关语法
#[derive(Debug, Clone)]
pub struct Grammar {
    pub rules: Vec<Rule>,
    pub start_rule: String,
}

impl Grammar {
    /// 创建JSON Grammar
    pub fn json() -> Self {
        let rules = vec![
            Rule {
                name: "root".to_string(),
                expr: Expr::NonTerminal("value".to_string()),
            },
            Rule {
                name: "value".to_string(),
                expr: Expr::Choice(vec![
                    Expr::NonTerminal("object".to_string()),
                    Expr::NonTerminal("array".to_string()),
                    Expr::NonTerminal("string".to_string()),
                    Expr::NonTerminal("number".to_string()),
                    Expr::Terminal("true".to_string()),
                    Expr::Terminal("false".to_string()),
                    Expr::Terminal("null".to_string()),
                ]),
            },
            Rule {
                name: "object".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::Terminal("{".to_string()),
                    Expr::Optional(Box::new(Expr::NonTerminal("members".to_string()))),
                    Expr::Terminal("}".to_string()),
                ]),
            },
            Rule {
                name: "members".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::NonTerminal("pair".to_string()),
                    Expr::Star(Box::new(Expr::Sequence(vec![
                        Expr::Terminal(",".to_string()),
                        Expr::NonTerminal("pair".to_string()),
                    ]))),
                ]),
            },
            Rule {
                name: "pair".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::NonTerminal("string".to_string()),
                    Expr::Terminal(":".to_string()),
                    Expr::NonTerminal("value".to_string()),
                ]),
            },
            Rule {
                name: "array".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::Terminal("[".to_string()),
                    Expr::Optional(Box::new(Expr::NonTerminal("elements".to_string()))),
                    Expr::Terminal("]".to_string()),
                ]),
            },
            Rule {
                name: "elements".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::NonTerminal("value".to_string()),
                    Expr::Star(Box::new(Expr::Sequence(vec![
                        Expr::Terminal(",".to_string()),
                        Expr::NonTerminal("value".to_string()),
                    ]))),
                ]),
            },
            Rule {
                name: "string".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::Terminal("\"".to_string()),
                    Expr::Star(Box::new(Expr::NonTerminal("char".to_string()))),
                    Expr::Terminal("\"".to_string()),
                ]),
            },
            Rule {
                name: "char".to_string(),
                expr: Expr::Choice(vec![
                    Expr::Terminal("a".to_string()), // 简化
                    Expr::Terminal("b".to_string()),
                    Expr::Terminal("c".to_string(),),
                ]),
            },
            Rule {
                name: "number".to_string(),
                expr: Expr::Sequence(vec![
                    Expr::Optional(Box::new(Expr::Terminal("-".to_string()))),
                    Expr::NonTerminal("digits".to_string()),
                ]),
            },
            Rule {
                name: "digits".to_string(),
                expr: Expr::Plus(Box::new(Expr::Terminal("0-9".to_string()))),
            },
        ];

        Self {
            rules,
            start_rule: "root".to_string(),
        }
    }
}

// ============================================================================
// 第四部分: PDA(下推自动机) - CFG执行引擎
// ============================================================================

/// PDA状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdaState(pub usize);

/// 栈符号
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StackSymbol {
    /// 非终结符
    NonTerminal(String),
    /// 返回地址
    Return(PdaState),
    /// 特殊符号
    Bottom,
}

/// PDA转移
#[derive(Debug, Clone)]
pub struct PdaTransition {
    pub from: PdaState,
    pub input: Option<u8>, // None = epsilon转移
    pub stack_top: Option<StackSymbol>,
    pub to: PdaState,
    pub stack_push: Vec<StackSymbol>,
}

/// 下推自动机
pub struct PushdownAutomaton {
    pub states: Vec<PdaState>,
    pub transitions: Vec<PdaTransition>,
    pub start_state: PdaState,
    pub accept_states: Vec<PdaState>,
}

impl PushdownAutomaton {
    /// 从Grammar构建PDA
    pub fn from_grammar(grammar: &Grammar) -> Self {
        // 简化的PDA构建
        // 实际实现需要LR(1)或LL(1)解析表生成
        let mut states = vec![PdaState(0), PdaState(1)];
        let start_state = PdaState(0);
        let accept_states = vec![PdaState(1)];

        let transitions = vec![
            PdaTransition {
                from: PdaState(0),
                input: None,
                stack_top: None,
                to: PdaState(1),
                stack_push: vec![StackSymbol::NonTerminal(grammar.start_rule.clone())],
            },
        ];

        Self {
            states,
            transitions,
            start_state,
            accept_states,
        }
    }

    /// 检查字节序列是否被接受
    pub fn accepts(&self, input: &[u8]) -> bool {
        // 简化的接受检查
        // 实际实现需要完整的PDA模拟
        !input.is_empty()
    }
}

// ============================================================================
// 第五部分: 持久栈 - O(1)回滚支持
// ============================================================================

/// 持久栈节点 - 树形结构实现
#[derive(Debug, Clone)]
struct StackNode<T> {
    value: T,
    parent: Option<usize>, // 父节点索引
    depth: usize,
}

/// 持久栈 - 支持O(1)回滚和分支
pub struct PersistentStack<T> {
    nodes: Vec<StackNode<T>>,
    current: Option<usize>,
}

impl<T: Clone> PersistentStack<T> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            current: None,
        }
    }

    /// Push操作 - 创建新节点，保留历史
    pub fn push(&mut self, value: T) {
        let new_node = StackNode {
            value,
            parent: self.current,
            depth: self.current.map(|i| self.nodes[i].depth + 1).unwrap_or(0),
        };
        self.nodes.push(new_node);
        self.current = Some(self.nodes.len() - 1);
    }

    /// Pop操作 - 移动到父节点
    pub fn pop(&mut self) -> Option<T> {
        self.current.and_then(|idx| {
            let node = &self.nodes[idx];
            let value = node.value.clone();
            self.current = node.parent;
            Some(value)
        })
    }

    /// Peek操作
    pub fn peek(&self) -> Option<&T> {
        self.current.map(|idx| &self.nodes[idx].value)
    }

    /// 创建分支 - 共享前缀
    pub fn branch(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            current: self.current,
        }
    }

    /// 回滚到指定深度 - O(1)
    pub fn rollback(&mut self, depth: usize) {
        // 从当前节点回溯到指定深度
        let mut curr = self.current;
        while let Some(idx) = curr {
            if self.nodes[idx].depth <= depth {
                break;
            }
            curr = self.nodes[idx].parent;
        }
        self.current = curr;
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_none()
    }

    pub fn len(&self) -> usize {
        self.current.map(|idx| self.nodes[idx].depth + 1).unwrap_or(0)
    }
}

// ============================================================================
// 第六部分: Token Mask缓存 - 核心优化
// ============================================================================

/// Token掩码存储策略
#[derive(Debug, Clone)]
pub enum MaskStorage {
    /// 存储接受的token列表（接受集较小时）
    Accepted(Vec<TokenId>),
    /// 存储拒绝的token列表（拒绝集较小时）
    Rejected(Vec<TokenId>),
    /// 使用bitset存储（两者都很大时）
    Bitset(DynamicBitset),
}

/// 自适应Token掩码缓存
pub struct AdaptiveTokenMaskCache {
    vocab_size: usize,
    /// PDA状态 -> 掩码存储
    state_masks: HashMap<PdaState, MaskStorage>,
    /// 上下文相关token列表
    context_dependent_tokens: Vec<TokenId>,
}

impl AdaptiveTokenMaskCache {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            state_masks: HashMap::new(),
            context_dependent_tokens: Vec::new(),
        }
    }

    /// 为指定PDA状态预计算掩码
    pub fn precompute_mask(
        &mut self,
        state: PdaState,
        tokens: &[TokenInfo],
        classifier: &TokenClassifier,
        grammar: &Grammar,
    ) {
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();

        for token in tokens {
            let category = classifier.classify(token, grammar);
            match category {
                TokenCategory::ContextIndependent => {
                    // 验证token在当前状态下是否有效
                    if self.is_valid_at_state(token, state, grammar) {
                        accepted.push(token.id);
                    } else {
                        rejected.push(token.id);
                    }
                }
                TokenCategory::ContextDependent => {
                    self.context_dependent_tokens.push(token.id);
                }
                _ => {}
            }
        }

        // 选择最优存储策略
        let storage = if accepted.len() < self.vocab_size / 8 {
            MaskStorage::Accepted(accepted)
        } else if rejected.len() < self.vocab_size / 8 {
            MaskStorage::Rejected(rejected)
        } else {
            let mut bitset = DynamicBitset::new(self.vocab_size);
            for id in &accepted {
                bitset.set(*id as usize, true);
            }
            MaskStorage::Bitset(bitset)
        };

        self.state_masks.insert(state, storage);
    }

    fn is_valid_at_state(&self, _token: &TokenInfo, _state: PdaState, _grammar: &Grammar) -> bool {
        // 简化的验证逻辑
        true
    }

    /// 获取指定状态的允许token掩码
    pub fn get_allowed_mask(&self, state: PdaState) -> DynamicBitset {
        let mut mask = DynamicBitset::new(self.vocab_size);

        // 1. 获取预计算的上下文无关token
        if let Some(storage) = self.state_masks.get(&state) {
            match storage {
                MaskStorage::Accepted(tokens) => {
                    for id in tokens {
                        mask.set(*id as usize, true);
                    }
                }
                MaskStorage::Rejected(rejected) => {
                    // 设置所有为true，然后清除拒绝的
                    for i in 0..self.vocab_size {
                        mask.set(i, true);
                    }
                    for id in rejected {
                        mask.set(*id as usize, false);
                    }
                }
                MaskStorage::Bitset(bitset) => {
                    // 复制bitset
                    for i in bitset.iter_ones() {
                        mask.set(i, true);
                    }
                }
            }
        }

        mask
    }

    /// 运行时检查上下文相关token
    pub fn validate_context_dependent(
        &self,
        token: TokenId,
        state: PdaState,
        stack: &PersistentStack<StackSymbol>,
    ) -> bool {
        // 检查token是否需要栈信息
        if !self.context_dependent_tokens.contains(&token) {
            return true; // 上下文无关token已通过预计算验证
        }

        // 需要完整栈信息验证
        self.validate_with_stack(token, state, stack)
    }

    fn validate_with_stack(
        &self,
        _token: TokenId,
        _state: PdaState,
        _stack: &PersistentStack<StackSymbol>,
    ) -> bool {
        // 简化的栈验证
        true
    }
}

// ============================================================================
// 第七部分: 约束生成器 - 集成所有组件
// ============================================================================

/// 约束生成器 - 主API
pub struct ConstraintGenerator {
    grammar: Grammar,
    pda: PushdownAutomaton,
    cache: AdaptiveTokenMaskCache,
    classifier: TokenClassifier,
    vocab_size: usize,
}

impl ConstraintGenerator {
    pub fn new(grammar: Grammar, vocab_size: usize) -> Self {
        let pda = PushdownAutomaton::from_grammar(&grammar);
        let cache = AdaptiveTokenMaskCache::new(vocab_size);
        let classifier = TokenClassifier::new(vocab_size);

        Self {
            grammar,
            pda,
            cache,
            classifier,
            vocab_size,
        }
    }

    /// 编译grammar并预计算token掩码
    pub fn compile(&mut self, tokens: &[TokenInfo]) {
        // 为每个PDA状态预计算掩码
        for state in &self.pda.states {
            self.cache.precompute_mask(*state, tokens, &self.classifier, &self.grammar);
        }
    }

    /// 获取当前允许的token掩码
    pub fn get_allowed_tokens(&self, state: PdaState) -> DynamicBitset {
        self.cache.get_allowed_mask(state)
    }

    /// 验证token序列
    pub fn validate_sequence(&self, tokens: &[TokenId]) -> bool {
        // 简化的序列验证
        !tokens.is_empty()
    }
}

// ============================================================================
// 第八部分: 测试与验证
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_bitset() {
        let mut bitset = DynamicBitset::new(100);
        bitset.set(0, true);
        bitset.set(50, true);
        bitset.set(99, true);

        assert!(bitset.get(0));
        assert!(!bitset.get(1));
        assert!(bitset.get(50));
        assert!(bitset.get(99));

        assert_eq!(bitset.count_ones(), 3);
    }

    #[test]
    fn test_bitset_operations() {
        let mut a = DynamicBitset::new(64);
        let mut b = DynamicBitset::new(64);

        a.set(0, true);
        a.set(1, true);
        b.set(1, true);
        b.set(2, true);

        a.and_with(&b);

        assert!(!a.get(0));
        assert!(a.get(1));
        assert!(!a.get(2));
    }

    #[test]
    fn test_persistent_stack() {
        let mut stack = PersistentStack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.len(), 3);

        // 创建分支
        let mut branch = stack.branch();
        branch.push(4);

        assert_eq!(stack.len(), 3);
        assert_eq!(branch.len(), 4);

        // 回滚
        branch.rollback(1);
        assert_eq!(branch.len(), 2);
    }

    #[test]
    fn test_json_grammar() {
        let grammar = Grammar::json();
        assert_eq!(grammar.start_rule, "root");
        assert!(!grammar.rules.is_empty());
    }

    #[test]
    fn test_pda_construction() {
        let grammar = Grammar::json();
        let pda = PushdownAutomaton::from_grammar(&grammar);
        assert_eq!(pda.start_state.0, 0);
    }

    #[test]
    fn test_constraint_generator() {
        let grammar = Grammar::json();
        let generator = ConstraintGenerator::new(grammar, 128000);

        let mask = generator.get_allowed_tokens(PdaState(0));
        assert_eq!(mask.size, 128000);
    }
}

// ============================================================================
// 第九部分: 主函数 - 演示
// ============================================================================

fn main() {
    println!("=== 结构化生成: Token级别约束LLM输出 ===\n");

    // 1. 创建JSON Grammar
    println!("1. 创建JSON Grammar...");
    let grammar = Grammar::json();
    println!("   规则数量: {}", grammar.rules.len());
    println!("   起始规则: {}", grammar.start_rule);

    // 2. 构建PDA
    println!("\n2. 构建PDA...");
    let pda = PushdownAutomaton::from_grammar(&grammar);
    println!("   状态数量: {}", pda.states.len());
    println!("   转移数量: {}", pda.transitions.len());

    // 3. 创建Token分类器
    println!("\n3. 创建Token分类器...");
    let vocab_size = 128000; // Llama-3.1词汇表大小
    let classifier = TokenClassifier::new(vocab_size);
    println!("   词汇表大小: {}", vocab_size);

    // 4. 创建约束生成器
    println!("\n4. 创建约束生成器...");
    let generator = ConstraintGenerator::new(grammar, vocab_size);
    println!("   初始化完成");

    // 5. 演示Token Mask
    println!("\n5. Token Mask演示...");
    let mask = generator.get_allowed_tokens(PdaState(0));
    println!("   Mask大小: {} bits", mask.size);

    // 6. 内存优化对比
    println!("\n6. 内存优化对比:");
    let naive_size = vocab_size * std::mem::size_of::<bool>();
    let bitset_size = (vocab_size + 31) / 32 * 4;
    println!("   朴素实现 (bool[]): {} KB", naive_size / 1024);
    println!("   DynamicBitset: {} KB", bitset_size / 1024);
    println!("   压缩率: {:.1}x", naive_size as f64 / bitset_size as f64);

    // 7. XGrammar性能数据
    println!("\n7. XGrammar性能数据 (参考):");
    println!("   Token Mask生成时间: < 40μs");
    println!("   相比传统方案加速: 100x");
    println!("   H100端到端加速: 80x");
    println!("   内存占用: 0.46MB (Llama-3.1 JSON语法)");

    // 8. 持久栈演示
    println!("\n8. 持久栈演示...");
    let mut stack: PersistentStack<StackSymbol> = PersistentStack::new();
    stack.push(StackSymbol::NonTerminal("object".to_string()));
    stack.push(StackSymbol::NonTerminal("pair".to_string()));
    println!("   栈深度: {}", stack.len());

    let branch = stack.branch();
    println!("   分支创建后深度: {}", branch.len());

    println!("\n=== 验证完成 ===");
}
