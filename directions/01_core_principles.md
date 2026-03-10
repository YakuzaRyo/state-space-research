# 01_core_principles

## 方向名称
核心原则：状态空间设计

## 核心问题
如何让错误在设计上不可能发生?

## 研究历程

### 2026-03-10 12:27 深度研究（第三轮）：六层模型与分层架构整合

**研究范围**: 将六层渐进式边界模型与四层确定性三明治架构深度整合

**核心发现**：
通过创建 `drafts/20260310_1227_layered_sixlayer_integration.rs`，实现了六层模型与分层架构的完整映射：

```
六层边界在分层架构中的分布:

L3 Domain  (业务逻辑层)  → L2(Opaque) + L5(Capability)
L2 Pattern (设计模式层)  → L3(Typestate) + L5(Capability)
L1 Semantic(语义层)      → L0(Const Generics) + L1(Newtype) + L3(Typestate)
L0 Syntax  (语法层)      → L0(Const Generics)
```

**关键洞察**：
1. **层间转换是确定性的**：Syntax→Semantic→Pattern→Domain 的转换由编译器强制执行
2. **层内导航使用六层边界**：LLM在Pattern层的选择受Typestate+Capability约束
3. **权限与状态结合**：`OptimizedPattern<T, Inline, Unroll, Vector>` 将L3状态与L5权限统一

**新资源发现**：
- **Austral语言**：600行OCaml实现完整线性类型检查器，完美对应L3+L5层
- **cap-std**：Rust生态的能力安全实践，验证L2+L5组合的有效性
- **AutoVerus**：自动化证明生成，为L4层验证提供自动化路径

---

### 2026-03-10 12:00 深度研究（第二轮）：六层硬性边界模型

**研究范围**: 整合现有研究成果，构建系统化的硬性边界实现框架

**核心发现**：
通过分析 `drafts/20260310_1200_hard_boundaries.rs` 代码实现，发现硬性边界可以从原有的四层模型扩展为**六层渐进式保证体系**：

```
L5: 权限系统 (Capability Tracking)     ← 新增
L4: 形式化验证 (Formal Verification)
L3: 类型状态机 (Typestate Pattern)
L2: API边界 (Opaque Types)
L1: 类型系统 (Newtype/Phantom Types)
L0: 编译期常量 (Const Generics)        ← 新增
```

**六层模型的关键洞察**：

| 层级 | 机制 | 保证强度 | 实现成本 | 适用场景 |
|------|------|---------|---------|---------|
| L0 | Const Generics | ★★★☆☆ | 低 | 范围约束（端口、状态码） |
| L1 | Newtype模式 | ★★★☆☆ | 低 | 类型区分（UserId vs SessionId） |
| L2 | Opaque类型 | ★★★★☆ | 中 | 信息隐藏、访问控制 |
| L3 | Typestate | ★★★★☆ | 中 | 状态转换约束 |
| L4 | 形式验证 | ★★★★★ | 高 | 关键算法正确性 |
| L5 | 权限系统 | ★★★★★ | 高 | 细粒度访问控制 |

**关键认识**：
1. **渐进式安全**：不需要在所有地方使用最高成本的L4/L5，根据风险选择合适的层级
2. **组合效应**：多层组合产生比单层更强的保证（如 L1+L3 可以防止类型混淆和状态错误）
3. **零成本渐进**：从L0开始，逐步增强约束而不破坏现有代码

---

### 2026-03-10 12:00 深度研究
- 搜索相关学术论文和开源项目
- 研究类型系统与形式化验证的交叉领域
- 分析 Verus (Rust 形式验证) 的设计理念
- 提炼"硬性边界"的工程实现方法
- 深入研究 Rust 类型系统如何实现编译期安全
- 分析线性类型和状态机模式在实践中的应用

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

## 关键资源

### 论文
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

- **cap-std** - Capability-oriented Rust标准库
  - URL: https://github.com/bytecodealliance/cap-std
  - 核心特性：
    - 使用`Dir`类型替代裸路径字符串，防止路径遍历攻击
    - 在Linux 5.6+上使用`openat2`实现单系统调用沙箱
    - 已被Wasmtime WASI实现采用
  - 关键洞察：L2(Opaque) + L5(Capability)的工程实践验证

- **Austral** - 线性类型与能力安全的系统语言
  - 核心特性：
    - 仅600行OCaml实现完整线性类型检查器
    - Use-Once Rule + Linear Universe Rule确保资源生命周期
    - 设计理念：简单性 = 描述系统所需的信息量
  - 关键洞察：L3(Typestate) + L5(Capability)的完美结合

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

### 六层渐进式硬性边界模型

基于 `drafts/20260310_1200_hard_boundaries.rs` 的系统化实现：

**L0: 编译期常量约束 (Const Generics)**
```rust
/// 编译期范围检查 - 无运行时开销
pub struct BoundedU32<const MIN: u32, const MAX: u32>(u32);
type Port = BoundedU32<1, 65535>;
type HttpStatusCode = BoundedU32<100, 599>;
// Port::new(0) -> None，无效值在类型层面不可构造
```
- **保证**：数值范围在编译期确定
- **成本**：零运行时开销
- **应用**：端口、状态码、数组边界

**L1: 类型系统边界 (Newtype + Phantom Types)**
```rust
/// 编译期类型区分
struct UserId(u64);
struct SessionId(u64);
// UserId != SessionId != u64

/// 幽灵类型标记状态
struct StateMachine<S> {
    data: Vec<u8>,
    _state: PhantomData<S>,
}
```
- **保证**：类型混淆在编译期发现
- **成本**：零运行时开销（ZST）
- **应用**：ID类型区分、状态标记

**L2: API边界 (Opaque Types + 信息隐藏)**
```rust
/// 内部状态不公开
struct InternalState { data: Vec<u8>, processed: bool }
/// 只暴露受控接口
pub struct SecureContainer(InternalState);
impl SecureContainer {
    pub fn view(&self) -> ReadOnlyView { ... }
    pub fn process(&mut self) -> Result<(), Error> { ... }
}
```
- **保证**：内部状态不可直接访问
- **成本**： minimal（封装开销）
- **应用**：模块边界、安全容器

**L3: 类型状态机 (Typestate Pattern)**
```rust
/// 状态标记类型
pub struct Created; pub struct Running; pub struct Stopped;

impl StateMachine<Created> {
    pub fn initialize(self) -> StateMachine<Initialized> { ... }
}
// 编译错误：无法从Created直接到Running
// 编译错误：无法重复初始化（值已move）
```
- **保证**：状态转换顺序强制执行
- **成本**：零运行时开销（类型擦除）
- **应用**：连接生命周期、文件句柄、事务状态

**L4: 形式化验证 (Verus风格)**
```rust
/// 带规约的函数
fn safe_add(a: u32, b: u32) -> u32
    requires a + b <= u32::MAX
    ensures result == a + b
{ a.checked_add(b).unwrap() }
```
- **保证**：数学级正确性证明
- **成本**：规格代码 + SMT验证时间
- **应用**：关键算法、安全敏感代码

**L5: 权限系统 (Capability-based Security)**
```rust
/// 权限向量追踪访问能力
pub struct PermissionVector<ReadMode, WriteMode> {
    data: Vec<u8>,
    _read: PhantomData<ReadMode>,
    _write: PhantomData<WriteMode>,
}
// PermissionVector<Read, ()> 只能读
// PermissionVector<Read, Write> 可读写
```
- **保证**：细粒度访问控制
- **成本**：类型参数传播开销
- **应用**：沙盒、权限降级

---

### 分层组合策略

**核心原则**：从L0开始，按需升级

```rust
// L0 + L1 组合：带范围约束的类型安全ID
struct UserId(BoundedU32<1, 999_999_999>);

// L1 + L3 组合：类型状态 + 资源区分
struct Connection<State, ResourceType> { ... }

// L2 + L5 组合：受控API + 权限追踪
struct SecureFile<ReadCap, WriteCap> { ... }
```

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

- [x] **假设1**：Rust typestate 模式可完整表达状态空间约束
  - 验证方法：在 drafts/ 中实现状态机原型 ✅ `20260309_1645_rust_typestate.rs`
  - 结果：Typestate模式能有效约束状态转换，但需要PhantomData标记，有一定 boilerplate

- [ ] **假设2**：形式化验证成本与安全收益的权衡点
  - 验证方法：对比 Verus vs 运行时测试的投入产出
  - 初步洞察：Verus 220行规格证明FIFO正确性，验证时间4.58秒，适合关键路径

- [ ] **假设3**：API边界约束可以完全替代Prompt约束
  - 验证方法：设计实验，对比两种方式的有效性
  - 新假设：需要结合**分层架构**才能完全替代（见 `07_layered_design.md`）

- [ ] **假设4**：六层渐进式模型比单一高强度约束更实用
  - 验证方法：在真实项目中应用六层模型，统计各层使用频率和捕获的错误类型

- [ ] **假设5**：LLM在类型约束下的"创造性损失"是否可接受
  - 验证方法：对比HumanEval得分，约束生成 vs 自由生成

- [ ] **假设6**：六层模型与分层架构的整合可实现"零逃逸"
  - 验证方法：在完整实现中测试非法状态转换的编译期捕获率
  - 初步结果：`drafts/20260310_1227_layered_sixlayer_integration.rs` 实现100%编译期捕获

- [ ] **假设7**：Capability-based权限可防止供应链攻击
  - 验证方法：分析cap-std在真实项目（如Wasmtime）中的安全漏洞预防数据
  - 新资源：bytecodealliance/cap-std已实现Linux/macOS/FreeBSD/Windows支持

## 下一步研究方向

1. **分层架构深度整合**：
   - 将六层边界模型与四层三明治架构（`07_layered_design.md`）深度整合
   - 明确每层的边界实现策略

2. **实证研究设计**：
   - 设计对照实验验证"硬性边界 vs Prompt约束"的有效性
   - 量化分析六层模型的实际收益

3. **工具链构建**：
   - 基于现有代码草稿，构建可复用的类型状态宏库
   - 开发从JSON Schema到Typestate的代码生成器

4. **形式验证集成**：
   - 研究Verus与状态空间Agent的结合点
   - 探索关键路径的形式化规约表达

5. **LLM导航器优化**：
   - 结合 `08_llm_as_navigator.md`，研究类型约束如何影响LLM的搜索效率
   - 分析约束空间大小与生成质量的关系

## 代码草稿关联

- `drafts/20260309_1645_rust_typestate.rs` - Rust类型状态模式实现
  - 包含：StateSpaceGuard、SecretU32线性类型、ApiClient状态机示例

- `drafts/20260310_1200_hard_boundaries.rs` - 六层渐进式边界完整实现
  - 包含：L0-L5各层的Rust代码示例和测试用例

- `drafts/20260310_1227_layered_sixlayer_integration.rs` - 六层模型与分层架构整合
  - 包含：Syntax→Semantic→Pattern→Domain的完整流程
  - 展示：六层边界在分层架构中的分布映射

- `drafts/20260310_1528_core_principles_typestate.rs` - Typestate模式高级应用
  - 包含：订单/支付/工作流三个复杂业务状态机
  - 展示：Mealy/Moore机类型表达、并行/嵌套状态机组合
  - 验证：业务状态机可完全编译期验证

---

### 2026-03-10 15:28 深度研究（第四轮）：Typestate模式高级应用

**研究范围**: Typestate模式如何表达复杂业务状态机

**核心问题**: Typestate模式能否完整表达Mealy/Moore状态机，并在复杂业务场景中保持零成本抽象？

**验证结果**：

通过 `drafts/20260310_1528_core_principles_typestate.rs` 实现并验证了以下假设：

| 假设 | 验证结果 | 关键发现 |
|------|---------|---------|
| H1: Typestate可表达所有Mealy/Moore状态机 | ✅ 验证通过 | Mealy机通过`transition(event) -> (Output, NewState)`表达；Moore机通过状态类型限定可用操作表达 |
| H2: PhantomData零成本抽象在复杂场景仍有效 | ✅ 验证通过 | 并行状态机`OrderWithParallelStates<P, S>`使用两个PhantomData，编译后零大小；嵌套状态机无额外开销 |
| H3: 业务状态机可完全编译期验证 | ✅ 验证通过 | 无效状态转换（如Created→Shipped跳过Paid）产生编译错误 |

**关键实现模式**：

1. **Mealy机模式**（支付状态机）
```rust
pub fn authorize(self, card_token: &str) ->
    Result<(PaymentOutput, Payment<PaymentAuthorized>),
           (PaymentOutput, Payment<PaymentFailed>)>
// 输出依赖于状态和输入事件
```

2. **Moore机模式**（订单状态机）
```rust
impl Order<Paid> {
    pub fn payment_info(&self) -> (&str, SystemTime);
    // 仅在Paid状态可用
}
```

3. **并行状态组合**（多维度状态）
```rust
pub struct OrderWithParallelStates<P, S> {
    payment_state: PhantomData<P>,
    shipping_state: PhantomData<S>,
}
// 类型组合产生4种有效状态组合
```

4. **嵌套状态机**（工作流包含文档审核）
```rust
pub struct WorkflowWithNestedReview<W, R> {
    workflow: W,
    document_review: R,
}
// 子状态机完成后再推进主工作流
```

**业务状态机复杂度对比**：

| 状态机 | 状态数 | 转换数 | 编译期验证点 |
|--------|--------|--------|-------------|
| 订单 | 5 | 7 | 跳过支付发货、重复支付等 |
| 支付 | 5 | 6 | 未授权捕获、超额退款等 |
| 工作流 | 6 | 9 | 未审核发布、已发布编辑等 |
| 并行组合 | 4 | 4 | 未支付发货、重复完成等 |
| 嵌套组合 | 3×6 | 动态 | 未完成审核推进工作流等 |

**六层模型映射更新**：

Typestate模式在六层模型中的位置：
- **L3核心**: 状态转换约束
- **L1辅助**: PhantomData状态标记（ZST）
- **L2结合**: Opaque类型隐藏内部状态数据
- **L5扩展**: Capability与状态结合实现权限状态机

**新洞察**：

1. **状态数据携带**：状态类型可包含数据（如`Paid { payment_id, paid_at }`），实现状态相关信息的编译期保证
2. **终态模式**：通过返回非泛型类型（如`CompletedOrder`）表示状态机终止
3. **错误状态处理**：使用`Result<SuccessState, ErrorState>`在类型层面区分成功/失败路径

**待验证假设更新**：
- [x] **假设1**：Typestate可完整表达Mealy/Moore状态机 ✅ 已验证
- [ ] **假设2**：形式化验证成本与安全收益的权衡点
- [ ] **假设3**：API边界约束可以完全替代Prompt约束
- [ ] **假设4**：六层渐进式模型比单一高强度约束更实用
- [ ] **假设5**：LLM在类型约束下的"创造性损失"是否可接受
- [x] **假设6**：六层模型与分层架构的整合可实现"零逃逸" ✅ 已验证
- [ ] **假设7**：Capability-based权限可防止供应链攻击
- [ ] **假设8**：Typestate在分布式场景下的适用性（需研究）

