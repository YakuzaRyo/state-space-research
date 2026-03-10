# 03_structured_generation

## 方向名称
结构化生成：XGrammar

## 核心问题
如何在 token 级别约束 LLM 输出?

## 研究历程

### 2026-03-10 14:31 深度研究
**研究范围**: XGrammar核心架构、结构化解码技术栈、Rust实现路径

**XGrammar核心架构分析**:

#### 1. GrammarCompiler编译器架构
- **输入**: EBNF语法、JSON Schema、正则表达式
- **处理流程**:
  1. 语法解析：将EBNF解析为AST (Grammar结构)
  2. 语法优化：内联(inlining)、等价状态合并
  3. FSM构建：为每个规则构建有限状态机
  4. Token Mask预计算：为所有可达状态计算自适应掩码
- **输出**: CompiledGrammar (包含grammar + adaptive_token_mask_cache)

#### 2. Token Mask Cache机制（99% token预检查）
XGrammar的核心优化是将token分为三类：
- **Context-independent tokens (上下文无关)**: 仅通过当前PDA状态即可确定有效性，占99%以上
- **Context-dependent tokens (上下文相关)**: 需要完整栈信息才能确定，通常<1%
- **Uncertain tokens (不确定)**: 需要运行时检查

**存储优化策略**:
```rust
enum StoreType {
    Accepted,      // 存储接受的token索引（当接受集较小时）
    Rejected,      // 存储拒绝的token索引（当拒绝集较小时）
    AcceptedBitset, // 使用bitset存储（当两者都很大时）
}
```

**性能数据**:
- JSON Schema工作负载：比现有方案快3.5倍
- CFG引导生成：比现有方案快10倍以上
- 端到端LLM推理：H100上快80倍
- 每token掩码生成：<40微秒

#### 3. Persistent Stack数据结构
- 基于**树形结构**存储多个并行栈（支持非确定性选择）
- 使用**引用计数**回收不再使用的节点
- 支持**O(1)回滚**：通过维护历史栈顶指针实现
- 支持**快速状态分支**：共享公共前缀

#### 4. 字节级PDA处理不规则token边界
- 传统方法：在字符级别处理，需要处理UTF-8多字节字符
- XGrammar方法：直接在**字节级别**处理
  - 支持sub-UTF8字符（处理不完整的UTF-8序列）
  - 无需重新编码tokenizer词汇表
  - 通过`sub_element_id`跟踪UTF-8多字节位置

#### 5. 100x加速的技术原理
1. **预计算**: 99%的token有效性在编译期确定
2. **上下文扩展**: 预处理技术将上下文相关token减少90%
3. **自适应存储**: 根据接受/拒绝集大小动态选择存储格式
4. **Trie前缀共享**: 利用词汇表排序共享公共前缀检查
5. **并行编译**: 使用多线程加速grammar编译
6. **GPU重叠**: 将grammar计算与GPU执行重叠

### 2026-03-09 10:47 深入研究
**研究发现**：
- XGrammar 通过将 token 分为**上下文无关**和**上下文相关**两类实现高效约束
- **自适应掩码缓存**：将 JSON 语法掩码存储从 160MB 降至 0.46MB（Llama-3.1）
- **上下文扩展**：预处理技术将上下文相关 token 减少 90%（从 1,134 降至 120）
- **持久执行栈**：基于持久数据结构，支持 O(1) 回滚和并行栈分支

**性能数据**：
- 每 token 约束解码延迟比现有方案快 **100 倍**
- H100 GPU 上端到端吞吐提升 **80 倍**
- 通过 WebAssembly 支持浏览器环境（M3 Max, iPhone 14 Pro Max）

**Rust 生态进展**：
- `xgrammar-rs` crate 已发布 - Rust bindings for XGrammar
- vLLM 已集成 xgrammar 作为结构化输出后端

### 2026-03-09 初始化
- 创建方向文档

## 关键资源

### 论文
- **XGrammar: Flexible and Efficient Structured Generation** (陈天奇团队, CMU Catalyst)
  - 字节级PDA处理不规则token边界
  - 自适应掩码缓存，比现有方案快100倍
  - 端到端接近零开销
  - [PDF](https://arxiv.org/pdf/2411.15100)

- **XGrammar 2: Dynamic and Efficient Structured Generation Engine for Agentic LLMs**
  - TagDispatch动态分派语义
  - JIT编译减少编译时间
  - 跨grammar缓存机制
  - Earley解析器扩展
  - [PDF](https://arxiv.org/pdf/2601.04426)

### 开源项目
- **[mlc-ai/xgrammar](https://github.com/mlc-ai/xgrammar)** - 官方实现 (C++/Python)
  - 核心C++后端
  - Python bindings
  - TypeScript/WebAssembly支持
- **[trymirai/xgrammar-rs](https://github.com/trymirai/xgrammar-rs)** - Rust bindings
- **[ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp)** - GBNF语法支持
- **[dottxt-ai/outlines](https://github.com/dottxt-ai/outlines)** - 对比方案

### 技术博客
- [Achieving Efficient, Flexible, and Portable Structured Generation with XGrammar](https://blog.mlc.ai/2024/11/22/achieving-efficient-flexible-and-portable-structured-generation-with-xgrammar)
- [Structured Decoding in vLLM: A Gentle Introduction](https://www.bentoml.com/blog/structured-decoding-in-vllm-a-gentle-introduction)
- [structured decoding, a guide for the impatient](https://aarnphm.xyz/posts/structured-decoding)

## 架构洞察

### XGrammar 核心机制
1. **字节级PDA** —— 处理不规则token边界，无需重新编码
2. **自适应掩码缓存** —— 动态调整掩码策略，最大化并行效率
3. **持久执行栈** —— 树形结构支持O(1)回滚和状态分支
4. **上下文扩展** —— 减少90%的上下文相关token
5. **零开销设计** —— 结构化约束不增加端到端延迟

### 技术对比矩阵

| 特性 | XGrammar | Outlines | SGLang | llama.cpp |
|------|----------|----------|--------|-----------|
| **语法支持** | CFG (EBNF) | FSM/Regex | CFG/Regex | CFG (GBNF) |
| **JSON Schema** | 完整支持 | 完整支持 | 完整支持 | 部分支持 |
| **Token Mask缓存** | 自适应缓存 | 无 | 有限 | 有限 |
| **持久栈** | 是 | 否 | 否 | 否 |
| **字节级处理** | 是 | 否 | 否 | 是 |
| **性能 (CFG)** | 10x基准 | 1x | 3-5x | 2-3x |
| **端到端开销** | 接近零 | 显著 | 中等 | 中等 |
| **并行编译** | 是 | 否 | 否 | 否 |
| **WebAssembly** | 是 | 否 | 否 | 是 |

### 与状态空间的结合点
- CFG约束解码将LLM输出限制在语法正确的空间内
- PDA自动机作为"硬性边界"的执行引擎
- 类型系统可以转化为CFG，指导结构化生成
- Token Mask Cache实现状态空间的快速查询

## GBNF语法格式

GBNF (GGML BNF) 是llama.cpp和XGrammar使用的语法格式：

```ebnf
# 基本结构
root ::= object
object ::= "{" (pair ("," pair)*)? "}"
pair ::= string ":" value
value ::= object | array | string | number | "true" | "false" | "null"
array ::= "[" (value ("," value)*)? "]"

# 字符类
string ::= "\"" char* "\""
char ::= [^"\\\x00-\x1F] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F]{4})
number ::= "-"? [0-9]+ ("." [0-9]+)? ([eE] [+-]? [0-9]+)?

# 重复
ws ::= [ \t\n]*          # 零次或多次
item ::= element+       # 一次或多次
opt ::= element?        # 零次或一次
range ::= element{1,10} # 1到10次
```

## Rust实现思路

### 核心模块设计
```rust
// 1. 动态bitset - 高效token掩码存储
pub struct DynamicBitset { ... }

// 2. 语法表示 - CFG/EBNF
pub struct Grammar { ... }
pub struct GrammarRule { ... }
pub struct GrammarExpr { ... }

// 3. FSM - 规则级自动机
pub struct FSM { ... }
pub struct CompactFSMWithStartEnd { ... }

// 4. Earley解析器 - 运行时匹配
pub struct EarleyParser { ... }
pub struct ParserState { ... }

// 5. Tokenizer信息
pub struct TokenizerInfo { ... }

// 6. 自适应掩码缓存
pub struct AdaptiveTokenMask { ... }
pub struct CompiledGrammar { ... }

// 7. 编译器
pub struct GrammarCompiler { ... }

// 8. 运行时匹配器
pub struct GrammarMatcher { ... }

// 9. LLM集成
pub struct GrammarLogitsProcessor { ... }
```

### 关键优化点
1. **内存布局**: 使用紧凑数组(Compact2DArray)减少内存碎片
2. **缓存友好**: 按字典序排序词汇表，利用前缀共享
3. **SIMD友好**: Bitset操作可使用SIMD加速
4. **零拷贝**: 解析器状态使用引用而非复制
5. **并行化**: Grammar编译可并行处理不同规则

## 待验证假设
- [ ] Token Mask Cache的命中率在实际工作负载中是否达到99%
- [ ] 持久栈的内存开销是否在可接受范围内
- [ ] 字节级处理对非英语语言的影响
- [ ] Rust实现的性能是否能接近C++版本

## 下一步研究方向
1. **XGrammar 2动态特性**: TagDispatch、JIT编译、跨grammar缓存
2. **与状态空间架构的集成**: 将PDA作为硬性边界执行引擎
3. **形式化验证**: 验证约束解码的正确性
4. **增量编译**: 支持grammar的动态更新
5. **GPU加速**: 将token mask计算 offload 到GPU

## 代码草稿
- [drafts/20260310_1431_structured_generation.rs](../drafts/20260310_1431_structured_generation.rs) - XGrammar核心Rust实现
