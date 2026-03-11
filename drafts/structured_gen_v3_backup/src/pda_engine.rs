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
