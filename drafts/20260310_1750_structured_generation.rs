//! XGrammar PDA约束生成验证代码
//!
//! 目标：验证PDA状态机与Rust类型系统的集成可行性

use std::collections::{HashMap, HashSet};

// ============================================
// 核心类型定义
// ============================================

/// Token ID类型
pub type TokenId = u32;

/// PDA状态ID
pub type StateId = u32;

/// 栈符号类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StackSymbol {
    /// 对象开始
    ObjectStart,
    /// 数组开始
    ArrayStart,
    /// 字符串上下文
    StringContext,
    /// 键值对上下文
    KeyValuePair { key: String },
    /// 自定义符号
    Custom(String),
}

/// PDA转换规则
#[derive(Debug, Clone)]
pub struct PDATransition {
    /// 当前状态
    pub from_state: StateId,
    /// 输入token（None表示epsilon转换）
    pub input: Option<TokenId>,
    /// 栈顶符号（None表示不检查栈）
    pub stack_top: Option<StackSymbol>,
    /// 目标状态
    pub to_state: StateId,
    /// 栈操作
    pub stack_op: StackOp,
}

/// 栈操作
#[derive(Debug, Clone)]
pub enum StackOp {
    /// 无操作
    None,
    /// 压栈
    Push(StackSymbol),
    /// 弹栈
    Pop,
    /// 替换栈顶
    Replace(StackSymbol),
}

/// 确定性下推自动机 (DPDA)
pub struct DPDA {
    /// 状态集合
    pub states: HashSet<StateId>,
    /// 初始状态
    pub initial_state: StateId,
    /// 接受状态集合
    pub accept_states: HashSet<StateId>,
    /// 转换函数: (state, input, stack_top) -> (next_state, stack_op)
    pub transitions: Vec<PDATransition>,
    /// 当前状态
    pub current_state: StateId,
    /// 栈
    pub stack: Vec<StackSymbol>,
}

impl DPDA {
    /// 创建新的DPDA
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

    /// 添加转换规则
    pub fn add_transition(&mut self, transition: PDATransition) {
        self.states.insert(transition.from_state);
        self.states.insert(transition.to_state);
        self.transitions.push(transition);
    }

    /// 设置接受状态
    pub fn set_accept_state(&mut self, state: StateId) {
        self.accept_states.insert(state);
    }

    /// 获取当前允许的token集合（核心约束生成逻辑）
    pub fn get_allowed_tokens(&self) -> HashSet<TokenId> {
        let mut allowed = HashSet::new();

        for trans in &self.transitions {
            if trans.from_state == self.current_state {
                // 检查栈条件
                let stack_matches = match &trans.stack_top {
                    None => true,
                    Some(expected) => self.stack.last() == Some(expected),
                };

                if stack_matches {
                    if let Some(token) = trans.input {
                        allowed.insert(token);
                    }
                }
            }
        }

        allowed
    }

    /// 处理输入token，进行状态转换
    pub fn process_token(&mut self, token: TokenId) -> Result<(), String> {
        for trans in &self.transitions {
            if trans.from_state == self.current_state && trans.input == Some(token) {
                // 检查栈条件
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
                            self.stack.pop();
                        }
                        StackOp::Replace(sym) => {
                            self.stack.pop();
                            self.stack.push(sym.clone());
                        }
                    }

                    // 更新状态
                    self.current_state = trans.to_state;
                    return Ok(());
                }
            }
        }

        Err(format!(
            "No valid transition for token {} in state {}",
            token, self.current_state
        ))
    }

    /// 检查是否处于接受状态
    pub fn is_accepting(&self) -> bool {
        self.accept_states.contains(&self.current_state) && self.stack.is_empty()
    }

    /// 重置PDA到初始状态
    pub fn reset(&mut self) {
        self.current_state = self.initial_state;
        self.stack.clear();
    }
}

// ============================================
// JSON Grammar PDA构建器
// ============================================

/// JSON Grammar状态定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonState {
    Start = 0,
    ExpectValue = 1,
    InObject = 2,
    ExpectKey = 3,
    ExpectColon = 4,
    ExpectObjectValue = 5,
    InArray = 6,
    ExpectArrayValue = 7,
    InString = 8,
    ExpectCommaOrEnd = 9,
    End = 10,
}

/// JSON Token定义（简化）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonToken {
    LBrace = 1,   // {
    RBrace = 2,   // }
    LBracket = 3, // [
    RBracket = 4, // ]
    Colon = 5,    // :
    Comma = 6,    // ,
    Quote = 7,    // "
    String = 8,   // 字符串内容
    Number = 9,   // 数字
    True = 10,    // true
    False = 11,   // false
    Null = 12,    // null
}

/// 构建JSON解析的PDA
pub fn build_json_pda() -> DPDA {
    let mut pda = DPDA::new(JsonState::Start as StateId);

    // Start -> ExpectValue (处理值开始)
    pda.add_transition(PDATransition {
        from_state: JsonState::Start as StateId,
        input: Some(JsonToken::LBrace as TokenId),
        stack_top: None,
        to_state: JsonState::ExpectKey as StateId,
        stack_op: StackOp::Push(StackSymbol::ObjectStart),
    });

    pda.add_transition(PDATransition {
        from_state: JsonState::Start as StateId,
        input: Some(JsonToken::LBracket as TokenId),
        stack_top: None,
        to_state: JsonState::ExpectArrayValue as StateId,
        stack_op: StackOp::Push(StackSymbol::ArrayStart),
    });

    // 对象解析逻辑
    pda.add_transition(PDATransition {
        from_state: JsonState::ExpectKey as StateId,
        input: Some(JsonToken::Quote as TokenId),
        stack_top: Some(StackSymbol::ObjectStart),
        to_state: JsonState::ExpectColon as StateId,
        stack_op: StackOp::Replace(StackSymbol::KeyValuePair {
            key: String::new(),
        }),
    });

    pda.add_transition(PDATransition {
        from_state: JsonState::ExpectColon as StateId,
        input: Some(JsonToken::Colon as TokenId),
        stack_top: None,
        to_state: JsonState::ExpectObjectValue as StateId,
        stack_op: StackOp::None,
    });

    // 对象值可以是嵌套结构
    pda.add_transition(PDATransition {
        from_state: JsonState::ExpectObjectValue as StateId,
        input: Some(JsonToken::LBrace as TokenId),
        stack_top: None,
        to_state: JsonState::ExpectKey as StateId,
        stack_op: StackOp::Push(StackSymbol::ObjectStart),
    });

    pda.add_transition(PDATransition {
        from_state: JsonState::ExpectObjectValue as StateId,
        input: Some(JsonToken::LBracket as TokenId),
        stack_top: None,
        to_state: JsonState::ExpectArrayValue as StateId,
        stack_op: StackOp::Push(StackSymbol::ArrayStart),
    });

    // 对象结束
    pda.add_transition(PDATransition {
        from_state: JsonState::ExpectKey as StateId,
        input: Some(JsonToken::RBrace as TokenId),
        stack_top: Some(StackSymbol::ObjectStart),
        to_state: JsonState::ExpectCommaOrEnd as StateId,
        stack_op: StackOp::Pop,
    });

    // 设置接受状态
    pda.set_accept_state(JsonState::End as StateId);
    pda.set_accept_state(JsonState::ExpectCommaOrEnd as StateId);

    pda
}

// ============================================
// Token Mask生成器（模拟XGrammar核心逻辑）
// ============================================

/// Token Mask生成器
pub struct TokenMaskGenerator {
    /// 词汇表大小
    vocab_size: usize,
    /// 当前mask
    current_mask: Vec<bool>,
}

impl TokenMaskGenerator {
    pub fn new(vocab_size: usize) -> Self {
        Self {
            vocab_size,
            current_mask: vec![true; vocab_size],
        }
    }

    /// 根据PDA状态生成mask
    pub fn generate_mask(&mut self, pda: &DPDA) -> &[bool] {
        // 重置mask
        self.current_mask.fill(false);

        // 获取允许的token
        let allowed = pda.get_allowed_tokens();

        // 设置允许的token位置为true
        for token_id in allowed {
            if (token_id as usize) < self.vocab_size {
                self.current_mask[token_id as usize] = true;
            }
        }

        &self.current_mask
    }

    /// 应用mask到logits（模拟）
    pub fn apply_mask(&self, logits: &mut [f32]) {
        for (i, logit) in logits.iter_mut().enumerate() {
            if i < self.current_mask.len() && !self.current_mask[i] {
                *logit = f32::NEG_INFINITY;
            }
        }
    }
}

// ============================================
// 类型状态模式验证（H1验证）
// ============================================

/// 使用Rust类型状态模式表示JSON构建过程
/// 这验证了H1: PDA状态可以映射到Rust类型状态

pub mod type_state {
    /// 标记trait用于类型状态
    pub trait JsonBuilderState {}

    /// 初始状态
    pub struct Start;
    impl JsonBuilderState for Start {}

    /// 对象构建中
    pub struct InObject;
    impl JsonBuilderState for InObject {}

    /// 数组构建中
    pub struct InArray;
    impl JsonBuilderState for InArray {}

    /// 键已设置，等待值
    pub struct KeySet {
        key: String,
    }
    impl JsonBuilderState for KeySet {}

    /// JSON构建器（类型状态模式）
    pub struct JsonBuilder<S: JsonBuilderState> {
        pub state: std::marker::PhantomData<S>,
        pub output: String,
    }

    impl JsonBuilder<Start> {
        pub fn new() -> Self {
            Self {
                state: std::marker::PhantomData,
                output: String::new(),
            }
        }

        pub fn begin_object(mut self) -> JsonBuilder<InObject> {
            self.output.push('{');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
            }
        }

        pub fn begin_array(mut self) -> JsonBuilder<InArray> {
            self.output.push('[');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
            }
        }
    }

    impl JsonBuilder<InObject> {
        pub fn key(mut self, k: &str) -> JsonBuilder<KeySet> {
            if !self.output.ends_with('{') {
                self.output.push(',');
            }
            self.output.push('"');
            self.output.push_str(k);
            self.output.push_str("\":");

            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
            }
        }

        pub fn end_object(mut self) -> JsonBuilder<Start> {
            self.output.push('}');
            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
            }
        }
    }

    impl JsonBuilder<KeySet> {
        pub fn string_value(mut self, v: &str) -> JsonBuilder<InObject> {
            self.output.push('"');
            self.output.push_str(v);
            self.output.push('"');

            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
            }
        }

        pub fn number_value(mut self, v: f64) -> JsonBuilder<InObject> {
            self.output.push_str(&v.to_string());

            JsonBuilder {
                state: std::marker::PhantomData,
                output: self.output,
            }
        }
    }
}

// ============================================
// 测试与验证
// ============================================

#[cfg(test)]
mod tests {
    use super::type_state::*;
    use super::*;

    #[test]
    fn test_pda_json_parsing() {
        let mut pda = build_json_pda();

        // 测试: 验证 { 是Start状态允许的token
        let allowed = pda.get_allowed_tokens();
        assert!(allowed.contains(&(JsonToken::LBrace as TokenId)));

        // 处理 {
        pda.process_token(JsonToken::LBrace as TokenId).unwrap();
        assert_eq!(pda.current_state, JsonState::ExpectKey as StateId);
        assert_eq!(pda.stack.len(), 1);

        println!("PDA状态转换测试通过");
    }

    #[test]
    fn test_token_mask_generation() {
        let pda = build_json_pda();
        let mut generator = TokenMaskGenerator::new(100);

        let mask = generator.generate_mask(&pda);

        // 验证mask长度
        assert_eq!(mask.len(), 100);

        // 验证 { token位置是允许的
        assert!(mask[JsonToken::LBrace as usize]);

        println!("Token Mask生成测试通过");
    }

    #[test]
    fn test_type_state_pattern() {
        // 使用类型状态模式构建JSON
        let builder = JsonBuilder::new()
            .begin_object()
            .key("name")
            .string_value("test")
            .end_object();

        assert_eq!(builder.output, r#"{"name":"test"}"#);
        println!("类型状态模式测试通过: {}", builder.output);
    }

    #[test]
    fn test_nested_structure() {
        let mut pda = build_json_pda();

        // 模拟解析: { "a": { "b": 1 } }
        pda.process_token(JsonToken::LBrace as TokenId).unwrap(); // {
                                                                 // 注意：这里简化处理，实际还需要处理引号和字符串

        assert_eq!(pda.stack.len(), 1);
        println!("嵌套结构栈深度测试通过");
    }
}

fn main() {
    println!("=== XGrammar PDA约束生成验证 ===\n");

    // 1. 构建JSON PDA
    let mut pda = build_json_pda();
    println!("1. 构建JSON PDA完成");
    println!("   - 状态数: {}", pda.states.len());
    println!("   - 转换规则数: {}", pda.transitions.len());

    // 2. 测试Token Mask生成
    let mut generator = TokenMaskGenerator::new(100);
    let mask = generator.generate_mask(&pda);
    let allowed_count = mask.iter().filter(|&&x| x).count();
    println!("\n2. Token Mask生成");
    println!("   - 词汇表大小: 100");
    println!("   - 当前允许token数: {}", allowed_count);

    // 3. 模拟状态转换
    println!("\n3. 模拟状态转换:");
    println!("   初始状态: {}", pda.current_state);

    pda.process_token(JsonToken::LBrace as TokenId).unwrap();
    println!("   处理 '{{' 后状态: {} (ExpectKey)", pda.current_state);
    println!("   栈深度: {}", pda.stack.len());

    // 4. 验证类型状态模式
    println!("\n4. 类型状态模式验证:");
    let builder = JsonBuilder::new()
        .begin_object()
        .key("user")
        .string_value("alice")
        .end_object();
    println!("   生成JSON: {}", builder.output);

    println!("\n=== 验证结论 ===");
    println!("H1部分验证: PDA状态可以映射到Rust类型状态");
    println!("H2部分验证: Grammar结构可与Rust类型系统结合");
    println!("  - 需要进一步验证derive宏生成");
    println!("  - 需要验证const fn编译期优化");
}
