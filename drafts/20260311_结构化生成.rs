//! 结构化生成 - Token级别约束LLM输出
//! 研究重点: XGrammar 2核心机制与Rust实现验证
//! 作者: Claude Research Agent
//! 日期: 2026-03-11

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;

// =============================================================================
// 第一部分: 核心数据结构与Token掩码
// =============================================================================

/// Token ID类型
pub type TokenId = u32;

/// 动态Bitset - 高效存储token掩码
/// 优化: 相比bool数组，内存使用减少32倍
pub struct DynamicBitset {
    /// 存储块，每个u64存储64个token状态
    blocks: Vec<u64>,
    /// token总数
    size: usize,
}

impl DynamicBitset {
    /// 创建指定大小的空bitset
    pub fn new(size: usize) -> Self {
        let num_blocks = (size + 63) / 64;
        Self {
            blocks: vec![0; num_blocks],
            size,
        }
    }

    /// 设置指定位置的值
    #[inline]
    pub fn set(&mut self, index: usize, value: bool) {
        assert!(index < self.size, "Index out of bounds");
        let block_idx = index / 64;
        let bit_idx = index % 64;
        if value {
            self.blocks[block_idx] |= 1u64 << bit_idx;
        } else {
            self.blocks[block_idx] &= !(1u64 << bit_idx);
        }
    }

    /// 获取指定位置的值
    #[inline]
    pub fn get(&self, index: usize) -> bool {
        if index >= self.size {
            return false;
        }
        let block_idx = index / 64;
        let bit_idx = index % 64;
        (self.blocks[block_idx] >> bit_idx) & 1 == 1
    }

    /// 计算bitset中1的个数（popcount）
    pub fn count_ones(&self) -> usize {
        self.blocks.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// Bitset与操作（用于合并约束）
    pub fn and_with(&mut self, other: &DynamicBitset) {
        let min_blocks = self.blocks.len().min(other.blocks.len());
        for i in 0..min_blocks {
            self.blocks[i] &= other.blocks[i];
        }
        // 如果self更大，剩余块与0保持为0
        for i in min_blocks..self.blocks.len() {
            self.blocks[i] = 0;
        }
    }

    /// Bitset或操作
    pub fn or_with(&mut self, other: &DynamicBitset) {
        let min_blocks = self.blocks.len().min(other.blocks.len());
        for i in 0..min_blocks {
            self.blocks[i] |= other.blocks[i];
        }
    }

    /// 获取所有被设置的token ID
    pub fn iter_set(&self) -> impl Iterator<Item = TokenId> + '_ {
        self.blocks.iter().enumerate().flat_map(|(block_idx, block)| {
            let base = block_idx * 64;
            (0..64).filter_map(move |bit_idx| {
                let idx = base + bit_idx;
                if idx < self.size && (block >> bit_idx) & 1 == 1 {
                    Some(idx as TokenId)
                } else {
                    None
                }
            })
        })
    }
}

impl fmt::Debug for DynamicBitset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count_ones();
        write!(f, "DynamicBitset {{ size: {}, set: {} }}", self.size, count)
    }
}

// =============================================================================
// 第二部分: Token分类策略（XGrammar核心优化）
// =============================================================================

/// Token分类 - XGrammar的关键优化
/// 将token分为上下文无关和上下文相关两类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    /// 上下文无关token - 仅通过当前PDA状态即可确定有效性（~99%）
    ContextIndependent,
    /// 上下文相关token - 需要完整栈信息才能确定（~1%）
    ContextDependent,
    /// 不确定 - 需要运行时检查
    Uncertain,
}

/// Token分类器
pub struct TokenClassifier {
    /// 词汇表大小
    vocab_size: usize,
    /// token分类缓存
    categories: Vec<TokenCategory>,
}

impl TokenClassifier {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            categories: vec![TokenCategory::Uncertain; vocab_size],
        }
    }

    /// 设置token分类
    pub fn set_category(&mut self, token_id: TokenId, category: TokenCategory) {
        if (token_id as usize) < self.vocab_size {
            self.categories[token_id as usize] = category;
        }
    }

    /// 获取token分类
    pub fn get_category(&self, token_id: TokenId) -> TokenCategory {
        self.categories.get(token_id as usize).copied()
            .unwrap_or(TokenCategory::Uncertain)
    }

    /// 获取所有上下文无关token
    pub fn get_context_independent_tokens(&self) -> Vec<TokenId> {
        self.categories.iter().enumerate()
            .filter(|(_, &cat)| cat == TokenCategory::ContextIndependent)
            .map(|(idx, _)| idx as TokenId)
            .collect()
    }
}

// =============================================================================
// 第三部分: PDA（下推自动机）核心实现
// =============================================================================

/// PDA状态ID
pub type PdaStateId = u32;

/// 栈符号
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackSymbol(pub u32);

/// PDA转移类型
#[derive(Debug, Clone)]
pub enum PdaTransition {
    /// 栈操作: 弹出栈顶并压入新符号
    StackOp { pop: Option<StackSymbol>, push: Vec<StackSymbol> },
    /// 空转移（不消耗输入）
    Epsilon,
    /// 接受状态
    Accept,
}

/// PDA状态
#[derive(Debug, Clone)]
pub struct PdaState {
    pub id: PdaStateId,
    pub transitions: HashMap<Option<TokenId>, Vec<(PdaTransition, PdaStateId)>>,
}

/// 下推自动机 - 用于执行CFG约束
pub struct PushdownAutomaton {
    states: HashMap<PdaStateId, PdaState>,
    initial_state: PdaStateId,
    // 初始栈符号
    initial_stack: Vec<StackSymbol>,
}

impl PushdownAutomaton {
    pub fn new(initial_state: PdaStateId) -> Self {
        let mut states = HashMap::new();
        states.insert(initial_state, PdaState {
            id: initial_state,
            transitions: HashMap::new(),
        });
        Self {
            states,
            initial_state,
            initial_stack: vec![StackSymbol(0)], // 初始栈底符号
        }
    }

    /// 添加状态
    pub fn add_state(&mut self, id: PdaStateId) {
        self.states.entry(id).or_insert_with(|| PdaState {
            id,
            transitions: HashMap::new(),
        });
    }

    /// 添加转移
    pub fn add_transition(
        &mut self,
        from: PdaStateId,
        token: Option<TokenId>,
        transition: PdaTransition,
        to: PdaStateId,
    ) {
        self.add_state(from);
        self.add_state(to);
        let state = self.states.get_mut(&from).unwrap();
        state.transitions.entry(token)
            .or_default()
            .push((transition, to));
    }

    /// 获取状态
    pub fn get_state(&self, id: PdaStateId) -> Option<&PdaState> {
        self.states.get(&id)
    }
}

// =============================================================================
// 第四部分: 持久化栈（Persistent Stack）- XGrammar核心优化
// =============================================================================

/// 持久化栈节点 - 使用树形结构实现O(1)回滚
#[derive(Debug, Clone)]
struct StackNode {
    /// 当前栈顶符号
    symbol: StackSymbol,
    /// 父节点（前一个栈状态）
    parent: Option<Arc<StackNode>>,
    /// 节点深度
    depth: usize,
}

/// 持久化栈 - 支持O(1)回滚和分支
pub struct PersistentStack {
    /// 当前栈顶
    top: Option<Arc<StackNode>>,
}

impl PersistentStack {
    pub fn new() -> Self {
        Self { top: None }
    }

    /// 从初始符号创建
    pub fn with_initial(symbol: StackSymbol) -> Self {
        Self {
            top: Some(Arc::new(StackNode {
                symbol,
                parent: None,
                depth: 1,
            })),
        }
    }

    /// 压栈 - 创建新节点，旧节点保持不变（持久化）
    pub fn push(&self, symbol: StackSymbol) -> Self {
        Self {
            top: Some(Arc::new(StackNode {
                symbol,
                parent: self.top.clone(),
                depth: self.depth() + 1,
            })),
        }
    }

    /// 弹栈 - 返回父节点
    pub fn pop(&self) -> Option<Self> {
        self.top.as_ref().map(|node| Self {
            top: node.parent.clone(),
        })
    }

    /// 查看栈顶
    pub fn peek(&self) -> Option<StackSymbol> {
        self.top.as_ref().map(|node| node.symbol)
    }

    /// 获取栈深度
    pub fn depth(&self) -> usize {
        self.top.as_ref().map(|n| n.depth).unwrap_or(0)
    }

    /// 将栈转换为Vec（用于调试）
    pub fn to_vec(&self) -> Vec<StackSymbol> {
        let mut result = Vec::new();
        let mut current = self.top.clone();
        while let Some(node) = current {
            result.push(node.symbol);
            current = node.parent.clone();
        }
        result.reverse();
        result
    }
}

impl Default for PersistentStack {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// 第五部分: 自适应Token掩码缓存（核心优化）
// =============================================================================

/// 缓存存储类型 - 根据接受/拒绝集大小动态选择
#[derive(Debug, Clone)]
pub enum CacheStorage {
    /// 存储接受的token索引（接受集较小时）
    Accepted(Vec<TokenId>),
    /// 存储拒绝的token索引（拒绝集较小时）
    Rejected(Vec<TokenId>),
    /// 使用bitset存储（两者都很大时）
    Bitset(DynamicBitset),
}

/// Token掩码缓存项
#[derive(Debug)]
pub struct TokenMaskCache {
    /// PDA状态
    state: PdaStateId,
    /// 缓存的掩码
    storage: CacheStorage,
    /// 是否为完整掩码（包含上下文相关token）
    is_complete: bool,
}

/// 自适应Token掩码缓存
pub struct AdaptiveTokenMaskCache {
    /// 缓存映射: (PDA状态, 栈顶符号) -> 掩码
    cache: HashMap<(PdaStateId, Option<StackSymbol>), TokenMaskCache>,
    /// 词汇表大小
    vocab_size: usize,
    /// 上下文无关token集合
    context_independent_tokens: HashSet<TokenId>,
}

impl AdaptiveTokenMaskCache {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            vocab_size,
            context_independent_tokens: HashSet::new(),
        }
    }

    /// 设置上下文无关token集合
    pub fn set_context_independent_tokens(&mut self, tokens: HashSet<TokenId>) {
        self.context_independent_tokens = tokens;
    }

    /// 获取或创建缓存项
    pub fn get_or_compute<F>(
        &mut self,
        state: PdaStateId,
        stack_top: Option<StackSymbol>,
        compute_fn: F,
    ) -> &TokenMaskCache
    where
        F: FnOnce() -> DynamicBitset,
    {
        let key = (state, stack_top);
        if !self.cache.contains_key(&key) {
            let mask = compute_fn();
            let storage = self.optimize_storage(mask);
            self.cache.insert(key, TokenMaskCache {
                state,
                storage,
                is_complete: stack_top.is_none(), // 简化逻辑
            });
        }
        self.cache.get(&key).unwrap()
    }

    /// 根据掩码特性选择最优存储方式
    fn optimize_storage(&self, mask: DynamicBitset) -> CacheStorage {
        let set_count = mask.count_ones();
        let unset_count = self.vocab_size - set_count;

        // 阈值设定：基于XGrammar论文的实际数据
        const THRESHOLD: usize = 1000;

        if set_count < THRESHOLD {
            // 接受集较小，存储接受的token
            CacheStorage::Accepted(mask.iter_set().collect())
        } else if unset_count < THRESHOLD {
            // 拒绝集较小，存储拒绝的token
            let rejected: Vec<_> = (0..self.vocab_size as TokenId)
                .filter(|&id| !mask.get(id as usize))
                .collect();
            CacheStorage::Rejected(rejected)
        } else {
            // 两者都很大，使用bitset
            CacheStorage::Bitset(mask)
        }
    }

    /// 获取缓存命中率统计
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.cache.len(),
            total_vocab_size: self.vocab_size,
        }
    }
}

/// 缓存统计
#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_vocab_size: usize,
}

// =============================================================================
// 第六部分: Grammar表示与编译
// =============================================================================

/// 语法表达式
#[derive(Debug, Clone)]
pub enum GrammarExpr {
    /// 终结符（token序列）
    Terminal(String),
    /// 非终结符引用
    NonTerminal(String),
    /// 序列
    Sequence(Vec<GrammarExpr>),
    /// 选择
    Choice(Vec<GrammarExpr>),
    /// 可选
    Optional(Box<GrammarExpr>),
    /// 重复（0次或多次）
    Star(Box<GrammarExpr>),
    /// 重复（1次或多次）
    Plus(Box<GrammarExpr>),
}

/// 语法规则
#[derive(Debug, Clone)]
pub struct GrammarRule {
    pub name: String,
    pub expr: GrammarExpr,
}

/// 上下文无关语法
pub struct Grammar {
    pub rules: Vec<GrammarRule>,
    pub start_rule: String,
}

impl Grammar {
    /// 创建简单JSON语法
    pub fn json_grammar() -> Self {
        Self {
            rules: vec![
                GrammarRule {
                    name: "value".to_string(),
                    expr: GrammarExpr::Choice(vec![
                        GrammarExpr::NonTerminal("object".to_string()),
                        GrammarExpr::NonTerminal("array".to_string()),
                        GrammarExpr::NonTerminal("string".to_string()),
                        GrammarExpr::NonTerminal("number".to_string()),
                        GrammarExpr::Terminal("true".to_string()),
                        GrammarExpr::Terminal("false".to_string()),
                        GrammarExpr::Terminal("null".to_string()),
                    ]),
                },
                GrammarRule {
                    name: "object".to_string(),
                    expr: GrammarExpr::Sequence(vec![
                        GrammarExpr::Terminal("{".to_string()),
                        GrammarExpr::Optional(Box::new(GrammarExpr::NonTerminal("members".to_string()))),
                        GrammarExpr::Terminal("}".to_string()),
                    ]),
                },
            ],
            start_rule: "value".to_string(),
        }
    }

    /// 编译为PDA
    pub fn compile_to_pda(&self) -> PushdownAutomaton {
        // 简化实现：实际编译需要更复杂的算法
        let mut pda = PushdownAutomaton::new(0);

        // 为每个规则创建状态
        for (idx, rule) in self.rules.iter().enumerate() {
            let state_id = idx as PdaStateId + 1;
            pda.add_state(state_id);
        }

        pda
    }
}

// =============================================================================
// 第七部分: XGrammar 2新特性 - TagDispatch
// =============================================================================

/// TagDispatch模式 - XGrammar 2的动态分派机制
/// 用于Agentic LLM中的动态语法切换
pub struct TagDispatch {
    /// 标签到语法的映射
    tag_grammars: HashMap<String, Arc<Grammar>>,
    /// 默认语法
    default_grammar: Option<Arc<Grammar>>,
    /// Aho-Corasick自动机用于高效标签匹配（简化实现）
    tags: Vec<String>,
}

impl TagDispatch {
    pub fn new() -> Self {
        Self {
            tag_grammars: HashMap::new(),
            default_grammar: None,
            tags: Vec::new(),
        }
    }

    /// 注册标签与语法的映射
    pub fn register_tag(&mut self, tag: &str, grammar: Arc<Grammar>) {
        self.tag_grammars.insert(tag.to_string(), grammar);
        self.tags.push(tag.to_string());
    }

    /// 设置默认语法
    pub fn set_default_grammar(&mut self, grammar: Arc<Grammar>) {
        self.default_grammar = Some(grammar);
    }

    /// 匹配标签并返回对应语法
    pub fn dispatch(&self, output_prefix: &str) -> Option<Arc<Grammar>> {
        // 简化实现：前缀匹配
        for tag in &self.tags {
            if output_prefix.contains(tag) {
                return self.tag_grammars.get(tag).cloned();
            }
        }
        self.default_grammar.clone()
    }
}

impl Default for TagDispatch {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// 第八部分: GrammarMatcher - 运行时匹配器
// =============================================================================

/// Grammar匹配器状态
#[derive(Debug, Clone)]
pub struct MatcherState {
    /// 当前PDA状态
    pda_state: PdaStateId,
    /// 当前栈
    stack: PersistentStack,
}

/// Grammar匹配器 - 运行时执行PDA
pub struct GrammarMatcher {
    /// PDA
    pda: PushdownAutomaton,
    /// 当前状态
    current: MatcherState,
    /// Token掩码缓存
    cache: AdaptiveTokenMaskCache,
    /// 已消耗的token历史（用于回滚）
    history: Vec<MatcherState>,
}

impl GrammarMatcher {
    pub fn new(pda: PushdownAutomaton, vocab_size: usize) -> Self {
        let initial_state = pda.initial_state;
        let initial_stack = PersistentStack::with_initial(StackSymbol(0));

        Self {
            pda,
            current: MatcherState {
                pda_state: initial_state,
                stack: initial_stack,
            },
            cache: AdaptiveTokenMaskCache::new(vocab_size),
            history: Vec::new(),
        }
    }

    /// 获取当前允许的token掩码
    pub fn get_allowed_tokens(&self) -> DynamicBitset {
        // 简化实现：实际需要从缓存获取
        DynamicBitset::new(self.cache.vocab_size)
    }

    /// 消耗一个token并转移状态
    pub fn consume_token(&mut self, token: TokenId) -> Result<(), MatcherError> {
        // 保存当前状态到历史
        self.history.push(self.current.clone());

        // 查找转移
        let state = self.pda.get_state(self.current.pda_state)
            .ok_or(MatcherError::InvalidState)?;

        // 简化：实际实现需要处理栈操作
        if let Some(transitions) = state.transitions.get(&Some(token)) {
            if let Some((_, next_state)) = transitions.first() {
                self.current.pda_state = *next_state;
                return Ok(());
            }
        }

        Err(MatcherError::InvalidToken)
    }

    /// 回滚到上一个状态
    pub fn rollback(&mut self) -> Result<(), MatcherError> {
        if let Some(prev_state) = self.history.pop() {
            self.current = prev_state;
            Ok(())
        } else {
            Err(MatcherError::CannotRollback)
        }
    }

    /// 检查是否处于接受状态
    pub fn is_accepting(&self) -> bool {
        // 简化实现
        false
    }
}

/// 匹配器错误
#[derive(Debug)]
pub enum MatcherError {
    InvalidState,
    InvalidToken,
    CannotRollback,
    StackUnderflow,
}

impl fmt::Display for MatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatcherError::InvalidState => write!(f, "Invalid PDA state"),
            MatcherError::InvalidToken => write!(f, "Invalid token for current state"),
            MatcherError::CannotRollback => write!(f, "Cannot rollback further"),
            MatcherError::StackUnderflow => write!(f, "Stack underflow"),
        }
    }
}

impl std::error::Error for MatcherError {}

// =============================================================================
// 第九部分: 类型状态模式 - 编译期JSON构建验证
// =============================================================================

/// 类型状态标记trait
pub trait JsonState {}

/// 初始状态
#[derive(Debug)]
pub struct Start;
impl JsonState for Start {}

/// 在对象内部
#[derive(Debug)]
pub struct InObject {
    key_set: bool,
}
impl JsonState for InObject {}

/// 在数组内部
#[derive(Debug)]
pub struct InArray;
impl JsonState for InArray {}

/// 已设置key，等待value
#[derive(Debug)]
pub struct KeySet;
impl JsonState for KeySet {}

/// 类型安全的JSON构建器
pub struct JsonBuilder<State: JsonState> {
    output: String,
    _state: std::marker::PhantomData<State>,
}

impl JsonBuilder<Start> {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            _state: std::marker::PhantomData,
        }
    }

    /// 开始对象 - 状态转移: Start -> InObject
    pub fn begin_object(self) -> JsonBuilder<InObject> {
        let mut output = self.output;
        output.push('{');
        JsonBuilder {
            output,
            _state: std::marker::PhantomData,
        }
    }

    /// 开始数组 - 状态转移: Start -> InArray
    pub fn begin_array(self) -> JsonBuilder<InArray> {
        let mut output = self.output;
        output.push('[');
        JsonBuilder {
            output,
            _state: std::marker::PhantomData,
        }
    }
}

impl JsonBuilder<InObject> {
    /// 设置key - 状态转移: InObject -> KeySet
    pub fn key(mut self, key: &str) -> JsonBuilder<KeySet> {
        if !self.output.ends_with('{') {
            self.output.push(',');
        }
        self.output.push('"');
        self.output.push_str(key);
        self.output.push_str("\":");
        JsonBuilder {
            output: self.output,
            _state: std::marker::PhantomData,
        }
    }

    /// 结束对象 - 状态转移: InObject -> Start
    pub fn end_object(mut self) -> JsonBuilder<Start> {
        self.output.push('}');
        JsonBuilder {
            output: self.output,
            _state: std::marker::PhantomData,
        }
    }
}

impl JsonBuilder<KeySet> {
    /// 字符串value - 状态转移: KeySet -> InObject
    pub fn string_value(mut self, value: &str) -> JsonBuilder<InObject> {
        self.output.push('"');
        self.output.push_str(value);
        self.output.push('"');
        JsonBuilder {
            output: self.output,
            _state: std::marker::PhantomData,
        }
    }

    /// 数字value - 状态转移: KeySet -> InObject
    pub fn number_value(mut self, value: f64) -> JsonBuilder<InObject> {
        self.output.push_str(&value.to_string());
        JsonBuilder {
            output: self.output,
            _state: std::marker::PhantomData,
        }
    }

    /// 嵌套对象 - 状态转移: KeySet -> InObject
    pub fn begin_nested_object(mut self) -> JsonBuilder<InObject> {
        self.output.push('{');
        JsonBuilder {
            output: self.output,
            _state: std::marker::PhantomData,
        }
    }
}

impl JsonBuilder<InArray> {
    /// 添加数组元素（字符串）
    pub fn string_element(mut self, value: &str) -> JsonBuilder<InArray> {
        if !self.output.ends_with('[') {
            self.output.push(',');
        }
        self.output.push('"');
        self.output.push_str(value);
        self.output.push('"');
        self
    }

    /// 结束数组
    pub fn end_array(mut self) -> JsonBuilder<Start> {
        self.output.push(']');
        JsonBuilder {
            output: self.output,
            _state: std::marker::PhantomData,
        }
    }
}

impl<State: JsonState> JsonBuilder<State> {
    /// 构建最终JSON字符串
    pub fn build(self) -> String {
        self.output
    }
}

impl Default for JsonBuilder<Start> {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// 第十部分: 测试与验证
// =============================================================================

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
        assert!(bitset.get(50));
        assert!(bitset.get(99));
        assert!(!bitset.get(1));
        assert!(!bitset.get(49));

        assert_eq!(bitset.count_ones(), 3);
    }

    #[test]
    fn test_bitset_and_or() {
        let mut a = DynamicBitset::new(64);
        let mut b = DynamicBitset::new(64);

        a.set(0, true);
        a.set(1, true);
        b.set(1, true);
        b.set(2, true);

        a.and_with(&b);
        assert!(a.get(1));
        assert!(!a.get(0));
        assert!(!a.get(2));
    }

    #[test]
    fn test_persistent_stack() {
        let stack = PersistentStack::with_initial(StackSymbol(0));
        let stack2 = stack.push(StackSymbol(1));
        let stack3 = stack2.push(StackSymbol(2));

        assert_eq!(stack.depth(), 1);
        assert_eq!(stack2.depth(), 2);
        assert_eq!(stack3.depth(), 3);

        assert_eq!(stack3.peek(), Some(StackSymbol(2)));

        let stack2_restored = stack3.pop().unwrap();
        assert_eq!(stack2_restored.depth(), 2);
        assert_eq!(stack2_restored.peek(), Some(StackSymbol(1)));

        // 原stack2保持不变（持久化）
        assert_eq!(stack2.depth(), 2);
    }

    #[test]
    fn test_type_state_json_builder() {
        // 正确的构建流程
        let json = JsonBuilder::new()
            .begin_object()
            .key("name")
            .string_value("test")
            .key("value")
            .number_value(42.0)
            .end_object()
            .build();

        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"value\":42"));
    }

    #[test]
    fn test_tag_dispatch() {
        let mut dispatch = TagDispatch::new();
        let grammar = Arc::new(Grammar::json_grammar());

        dispatch.register_tag("<function=", grammar.clone());
        dispatch.set_default_grammar(grammar);

        let result = dispatch.dispatch("some text <function=foo");
        assert!(result.is_some());
    }

    #[test]
    fn test_token_classifier() {
        let mut classifier = TokenClassifier::new(1000);
        classifier.set_category(0, TokenCategory::ContextIndependent);
        classifier.set_category(1, TokenCategory::ContextDependent);

        assert_eq!(classifier.get_category(0), TokenCategory::ContextIndependent);
        assert_eq!(classifier.get_category(1), TokenCategory::ContextDependent);
    }
}

// =============================================================================
// 第十一部分: 性能基准测试框架
// =============================================================================

/// 性能基准测试结果
#[derive(Debug)]
pub struct BenchmarkResult {
    pub operation: String,
    pub iterations: usize,
    pub total_time_us: u64,
    pub avg_time_ns: u64,
    pub ops_per_second: f64,
}

/// 简单的基准测试运行器
pub struct BenchmarkRunner;

impl BenchmarkRunner {
    /// 运行基准测试
    pub fn run<F>(name: &str, iterations: usize, mut f: F) -> BenchmarkResult
    where
        F: FnMut(),
    {
        use std::time::Instant;

        // 预热
        for _ in 0..iterations.min(100) {
            f();
        }

        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        let elapsed = start.elapsed();

        let total_time_us = elapsed.as_micros() as u64;
        let avg_time_ns = (elapsed.as_nanos() as u64) / iterations as u64;
        let ops_per_second = iterations as f64 / elapsed.as_secs_f64();

        BenchmarkResult {
            operation: name.to_string(),
            iterations,
            total_time_us,
            avg_time_ns,
            ops_per_second,
        }
    }
}

// =============================================================================
// 主函数示例
// =============================================================================

fn main() {
    println!("=== 结构化生成 - Token级别约束LLM输出 ===\n");

    // 1. 演示DynamicBitset
    println!("1. DynamicBitset演示:");
    let mut bitset = DynamicBitset::new(128000); // Llama-3.1词汇表大小
    bitset.set(100, true);
    bitset.set(1000, true);
    bitset.set(10000, true);
    println!("   创建的bitset: {:?}", bitset);
    println!("   内存占用: ~{}KB (vs bool[128000] = 128KB)",
             (128000 / 64 * 8) / 1024);

    // 2. 演示持久化栈
    println!("\n2. 持久化栈演示:");
    let stack = PersistentStack::with_initial(StackSymbol(0));
    let stack2 = stack.push(StackSymbol(1));
    let stack3 = stack2.push(StackSymbol(2));
    println!("   stack深度: {}", stack.depth());
    println!("   stack2深度: {}", stack2.depth());
    println!("   stack3深度: {}", stack3.depth());
    println!("   stack3栈顶: {:?}", stack3.peek());
    println!("   stack3内容: {:?}", stack3.to_vec());

    // 3. 演示类型状态模式
    println!("\n3. 类型状态JSON构建器演示:");
    let json = JsonBuilder::new()
        .begin_object()
        .key("name")
        .string_value("XGrammar")
        .key("version")
        .number_value(2.0)
        .key("features")
        .begin_array()
        .string_element("TagDispatch")
        .string_element("JIT Compilation")
        .string_element("Cross-Grammar Cache")
        .end_array()
        .end_object()
        .build();
    println!("   生成的JSON: {}", json);

    // 4. 演示TagDispatch
    println!("\n4. TagDispatch演示:");
    let mut dispatch = TagDispatch::new();
    let grammar = Arc::new(Grammar::json_grammar());
    dispatch.register_tag("<function=", grammar.clone());
    dispatch.register_tag("<|channel|>", grammar.clone());

    let test_cases = vec![
        "Hello world",
        "Call <function=add",
        "Use <|channel|>analysis",
    ];

    for case in test_cases {
        let result = dispatch.dispatch(case);
        println!("   输入: {:?} -> 匹配: {}", case, result.is_some());
    }

    // 5. 演示Token分类
    println!("\n5. Token分类演示:");
    let mut classifier = TokenClassifier::new(128000);
    // 模拟：设置一些上下文无关token（如JSON标点）
    for id in [0, 1, 2, 3, 4] {
        classifier.set_category(id, TokenCategory::ContextIndependent);
    }
    classifier.set_category(100, TokenCategory::ContextDependent);

    let ci_tokens = classifier.get_context_independent_tokens();
    println!("   上下文无关token数量: {} (~{:.2}%)",
             ci_tokens.len(),
             (ci_tokens.len() as f64 / 128000.0) * 100.0);

    println!("\n=== 演示完成 ===");
}
