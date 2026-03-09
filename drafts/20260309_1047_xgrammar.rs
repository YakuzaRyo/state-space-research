//! XGrammar Rust Bindings 使用示例
//! 方向: structured_generation
//! 时间: 2026-03-09 10:47

use xgrammar_rs::{Grammar, TokenizerInfo};

/// JSON Schema 到 Rust 类型的约束生成示例
pub fn json_schema_to_rust_types(schema: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 1. 从 JSON Schema 构建 Grammar
    let grammar = Grammar::from_json_schema(schema)?;
    
    // 2. 获取 tokenizer 信息（词汇表大小、token id 映射等）
    let tokenizer_info = TokenizerInfo::from_hf_tokenizer("meta-llama/Llama-3.1-8B-Instruct")?;
    
    // 3. 创建约束解码器
    let mut decoder = grammar.compile(&tokenizer_info)?;
    
    // 4. 在解码循环中应用约束
    // 伪代码示例：
    // for step in 0..max_tokens {
    //     let logits = model.forward(input_ids)?;
    //     let mask = decoder.get_next_token_mask()?;
    //     let masked_logits = apply_mask(logits, mask);
    //     let next_token = sample(masked_logits);
    //     decoder.accept_token(next_token)?;
    // }
    
    Ok("Generated Rust types".to_string())
}

/// 自适应掩码缓存结构示意
/// 参考 XGrammar 论文中的 adaptive mask cache
pub struct AdaptiveMaskCache {
    /// 上下文无关 token 的预计算掩码
    context_independent_masks: Vec<TokenMask>,
    /// 上下文相关 token 的动态验证缓存
    context_dependent_cache: std::collections::HashMap<StackState, TokenMask>,
}

pub struct TokenMask {
    /// accept-heavy: 存储 rejected tokens
    /// reject-heavy: 存储 accepted tokens  
    /// balanced: 压缩位集
    storage: MaskStorage,
}

pub enum MaskStorage {
    AcceptHeavy(Vec<u32>),  // rejected token ids
    RejectHeavy(Vec<u32>),  // accepted token ids
    CompressedBitSet(Vec<u8>),
}

/// 持久化栈结构示意
/// 支持 O(1) 分支和回滚
pub struct PersistentStack<T> {
    root: std::sync::Arc<StackNode<T>>,
}

pub struct StackNode<T> {
    value: T,
    parent: Option<std::sync::Arc<StackNode<T>>>,
}

impl<T: Clone> PersistentStack<T> {
    /// O(1) 分支 - 创建新的栈顶，共享前缀
    pub fn push(&self, value: T) -> Self {
        PersistentStack {
            root: std::sync::Arc::new(StackNode {
                value,
                parent: Some(self.root.clone()),
            }),
        }
    }
    
    /// O(1) 回滚到父状态
    pub fn pop(&self) -> Option<Self> {
        self.root.parent.as_ref().map(|p| PersistentStack {
            root: p.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_persistent_stack() {
        let stack1 = PersistentStack { root: std::sync::Arc::new(StackNode { value: 1, parent: None }) };
        let stack2 = stack1.push(2);
        let stack3 = stack2.push(3);
        
        // 回滚到 stack2，O(1)
        let rolled_back = stack3.pop().unwrap();
        assert_eq!(rolled_back.root.value, 2);
        
        // stack1 和 stack2 共享前缀
        assert!(std::sync::Arc::ptr_eq(&stack1.root, rolled_back.root.parent.as_ref().unwrap()));
    }
}
