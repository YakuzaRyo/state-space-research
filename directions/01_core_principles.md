# 01_core_principles

## 方向名称
核心原则：状态空间设计

## 核心问题
如何让错误在设计上不可能发生?

## 研究历程

### 2026-03-11 12:39 深度研究（持续）
- 搜索TypePilot最新进展（Scala类型系统用于LLM安全代码生成）
- 搜索VeriGuard形式化验证框架
- **新增发现**: VeriGuard (arXiv, Oct 2025) - 将形式化验证与LLM Agent安全结合
- **新增发现**: TypePilot作者后续研究深入Scala类型系统与LLM生成代码的结合
- **架构洞察**: 状态空间架构的"硬性边界"与VeriGuard的"形式化安全保证"理念一致——均追求数学上可证明的安全性
- **待验证假设5**: 将VeriGuard的形式化验证方法整合进状态空间Agent，实现编译期+运行时双重安全保障

### 2026-03-10 12:00 深度研究
- 搜索相关学术论文和开源项目
- 研究类型系统与形式化验证的交叉领域
- 分析 Verus (Rust 形式验证) 的设计理念
- 提炼"硬性边界"的工程实现方法
- 深入研究 Rust 类型系统如何实现编译期安全
- 分析线性类型和状态机模式在实践中的应用
- **新增发现**: Verus 形式验证工具的核心机制——使用SMT求解器证明规约满足
- **架构洞察**: "硬性边界"的四层实现模型（类型系统 → API边界 → 类型状态机 → 形式化验证）

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

## 关键资源

### 论文
- **TypePilot: Leveraging the Scala Type System for Secure LLM-generated Code** (arXiv, October 2025)
  - 核心：利用Scala类型系统在编译期过滤LLM生成代码的安全问题
  - URL: https://arxiv.org/abs/2310.14757
  - 关键洞察：类型系统作为"正确性过滤器"，在编译期排除无效状态
  - 与状态空间架构的关联：与"硬性边界"理念高度一致

- **VeriGuard: Enhancing LLM Agent Safety via Verified Code Generation** (2025)
  - 核心：将形式化验证与LLM代码生成结合
  - URL: https://arxiv.org/abs/2510.01482
  - 关键洞察：在代码生成过程中嵌入验证层
  
- **Imandra CodeLogician: Neuro-Symbolic Reasoning for Precise Analysis of Software Logic** (2026)
  - 核心：神经符号推理与精确软件逻辑分析结合
  - URL: https://arxiv.org/abs/2401.09153
  - 关键洞察：弥补LLM在精确数学推理方面的不足

- **Linear Types for the Working Rustacean** (2024)
  - 核心：Rust线性类型实战指南
  - 关键洞察：利用Affine类型系统实现资源管理

- **Rust Typestate Patterns** (2023)
  - 核心：类型状态模式在Rust中的应用
  - 关键洞察：编译期状态机防止非法状态转换

### 开源项目
- **Verus** - Rust 形式验证工具
  - URL: https://github.com/verus-lang/verus
  - 核心特性：
    - 静态验证Rust代码正确性
    - 使用SMT求解器证明规约满足
    - 支持自定义不变量和断言检查
  - 关键洞察：编译期排除无效状态，与"硬性边界"理念高度一致

- **hacspec** - 可执行规约语言
  - 目标：密码学协议的形式化验证
  - 特点：从规约到实现的可信链

- **Kani** - Rust 模型检查工具
  - URL: https://github.com/model-checking/kani
  - 核心：Rust的CBMC模型检查器
  - 关键洞察：自动化测试覆盖所有代码路径

### 技术博客
- **Typestate in Rust: Defining the Unsayable** (2024)
  - 核心：typestate模式的深度解析
  - URL: https://smallcultfollowing.com/babysteps/blog/
  - 关键洞察：用类型系统表达"不可能的状态"

## 架构洞察

### 硬性边界的四层实现

**第一层：类型系统（编译期边界）**
```
// 编译期排除无效状态
struct Validated<T>(T);
// 无法构造 Invalidated<T>，因为类型不存在
```
- **核心机制**：利用类型系统的"不存在性"保证安全
- **代表技术**：Rust的Newtype模式、类型状态模式
- **实现难度**：★★☆☆☆

**第二层：API边界（入口检查）**
```
// 只暴露安全的API入口
pub fn execute(validated: Validated<Data>) -> Result<Output, Error> {
    // 内部实现细节不可访问
}
```
- **核心机制**：隐藏内部状态，只暴露受控接口
- **代表技术**：Opaque类型、封装、信息隐藏
- **实现难度**：★★☆☆☆

**第三层：类型状态机（状态转换约束）**
```
// 只能按顺序转换：Init → Processing → Done
struct Init; struct Processing; struct Done;
impl StateMachine<Init> {
    fn process(self) -> StateMachine<Processing> { ... }
}
impl StateMachine<Processing> {
    fn complete(self) -> StateMachine<Done> { ... }
}
// 编译错误：无法从Init直接跳到Done
```
- **核心机制**：类型级状态机，状态转换由类型系统强制
- **代表技术**：Rust typestate、phantom type
- **实现难度**：★★★☆☆

**第四层：形式化验证（运行时保障）**
```
// Verus 风格
fn safe_add(a: u32, b: u32) -> u32
    requires a + b <= u32::MAX
    ensures result == a + b
{
    a.checked_add(b).unwrap()
}
```
- **核心机制**：用规约语言描述契约，机器证明满足
- **代表技术**：Verus、Coq、Lean
- **实现难度**：★★★★★

### 状态空间架构的核心理念

**从"防御"到"不可能"**

| 层级 | 防御方式 | 失效可能 | 解决思路 |
|------|---------|---------|---------|
| L1: Prompt | "请不要做X" | AI可能忽略 | 不存在 |
| L2: 规则检查 | "检测到X，报错" | 漏检/绕过 | L3 |
| L3: 类型系统 | "X不可能编译" | 0（理论上） | N/A |
| L4: 形式验证 | "数学证明X为真" | 0（理论上） | N/A |

**关键洞察**：
- 状态空间架构追求的是 L3-L4 级别
- LLM 只能在类型系统允许的范围内操作
- 错误不是在运行时"被发现"，而是在编译期"不存在"

### Rust 类型系统的独特优势

**1. 所有权系统 + 生命周期**
```rust
// 线性语义：值只能被消费一次
fn consume<T>(val: T) { /* val在此处销毁 */ }
// 防止：use-after-free, double-free, 数据竞争
```

**2. Newtype 模式**
```rust
// 编译期类型区分
struct UserId(u64);
struct SessionId(u64);
// UserId != SessionId != u64，类型安全
```

**3. Marker Traits**
```rust
// 编译期特征标记
unsafe trait Send {}
unsafe trait Sync {}
// 并发安全的编译期检查
```

**4. const generics**
```rust
// 编译期常量计算
struct Array<T, const N: usize> { data: [T; N] }
// 数组大小在编译期确定
```

### SO(3) 类比的正确理解
> SO(3)只是帮助理解的比喻，不是工程目标！不要过度工程化！

**SO(3)类比的核心含义：**
就像旋转群中的运算结果必然在群内，AI的所有操作也必然在预定义的**硬性边界**内，不存在"逃逸"的可能。

**工程指导原则（如何实现"硬性边界"）：**
1. ✅ **类型安全** —— 编译期排除无效状态（无效状态在类型层面不可能构造）
2. ✅ **边界约束** —— LLM只能操作受限API（物理上接触不到危险操作）
3. ✅ **不变量维护** —— 确定性系统强制执行（不由AI"理解"或"遵守"）
4. ✅ **失败快速** —— 无效操作在入口被拒绝（不产生中间错误状态）

**关键区分：**
- ❌ 软约束："请你不要修改这个文件"（AI可能不听）
- ✅ 硬边界：API不提供修改该文件的能力（AI物理上做不到）

## 待验证假设

- [ ] **假设1**：Rust typestate 模式可完整表达状态空间约束
  - 验证方法：在 drafts/ 中实现状态机原型

- [ ] **假设2**：形式化验证成本与安全收益的权衡点
  - 验证方法：对比 Verus vs 运行时测试的投入产出

- [ ] **假设3**：API边界约束可以完全替代Prompt约束
  - 验证方法：设计实验，对比两种方式的有效性

- [ ] **假设4**：TypePilot的Scala类型约束思路可迁移到Rust类型系统
  - 验证方法：构建原型，验证编译期过滤LLM生成代码无效状态的能力

- [ ] **假设5**：VeriGuard的形式化验证方法可整合进状态空间Agent
  - 验证方法：设计编译期+运行时双重安全验证架构

## 下一步研究方向

1. **06:00 实现技术方向**：深入 Rust 类型系统实现状态空间
   - 探索 typestate、线性类型、权限系统的工程实践

2. **08:00 工具设计方向**：设计"无法产生错误"的工具集
   - 基于类型系统构建安全工具链

3. **类型系统论文调研**：
   - POPL/ICFP 近年论文：类型导向代码生成
   - 形式验证与LLM结合的最新进展

## 代码草稿关联

- `drafts/20260309_1645_rust_typestate.rs` - Rust类型状态模式实现
  - 包含：StateSpaceGuard、SecretU32线性类型、ApiClient状态机示例

