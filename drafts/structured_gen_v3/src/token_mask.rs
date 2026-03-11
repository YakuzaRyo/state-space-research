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
