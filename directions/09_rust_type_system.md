# 09_rust_type_system

## 方向名称
Rust 类型系统实现

## 核心问题
如何用 Rust 类型系统实现状态空间?

## 研究历程

### 2026-03-09 16:45 深入研究（重试）
**研究发现**：
- **Verus验证器**: CMU开发的Rust程序验证工具，利用Rust线性类型系统简化SMT验证，将子结构逻辑（如分离逻辑）引入类型系统
- **Typestate模式**: Rust编译期强制执行运行时状态顺序，如"必须先认证才能发送消息"、"文件关闭后不可I/O"
- **hacspec**: 针对密码学代码的Rust子集，通过三层类型检查（Rust常规检查→hacspec语法转换→hacspec形式化规则）在编译期排除非恒定时间操作
- **TypeSec**: OpenCL的Rust封装，使用类型状态模式在编译期捕获9类API错误（空队列启动内核、飞行中缓冲区读写等），零运行时开销

**架构洞察**:
- **线性类型即权限**: Rust的`move`语义天然对应分离逻辑中的权限转移，类型系统的`drop`检查确保资源释放
- **编译期状态机**: 通过PhantomData标记状态，无效状态转移在编译期被拒绝，无需运行时检查
- **零成本抽象**: Typestate模式生成的汇编与原始API调用完全一致，cargo-bloat验证无额外代码体积

**关键数据**:
- Verus验证: 220行规格代码证明FIFO队列正确性，验证时间4.58秒
- TypeSec开销: 16MB缓冲区操作最坏情况+3.88%，100MB以上<0.7%
- hacspec安全: U8/U32等Secret Integer类型仅暴露恒定时间操作，除法等非恒定操作在类型层面不可用

### 2026-03-10 18:15 深度研究：L4形式验证层实现（六层模型整合）

**研究范围**: 使用SubAgent深度研究Rust形式验证生态（~30分钟）

**核心发现**：
建立了完整的Rust形式验证工具对比和L4层实现：

**验证工具对比矩阵**:
| 工具 | 后端 | 方法 | 自动化 | 适用场景 |
|------|------|------|--------|---------|
| Kani | CBMC/SMT | 模型检查 | 高 | unsafe代码、安全边界 |
| Verus | SMT (Z3) | 线性幽灵类型 | 中高 | 系统代码、并发协议 |
| Creusot | Why3 | 预言编码 | 中 | 算法正确性 |
| Prusti | Viper | 分离逻辑 | 中 | 复杂堆操作 |
| Aeneas | Lean/Coq/F* | 函数式翻译 | 低 | 密码学原语 |

**关键资源发现**:
- **Verus**: DARPA PROVERS资助，SOSP 2024 Distinguished Artifact Award
- **Kani**: AWS Firecracker生产部署，27个验证harnesses在CI中运行
- **Aeneas**: Microsoft SymCrypt移植到验证Rust
- **hacspec/hax**: libcrux形式验证加密库 (Signal使用)
- **Creusot**: CreuSAT - 世界最快的演绎验证SAT求解器

**架构洞察**:
- Rust所有权系统使形式验证可用FOL而非分离逻辑，降低复杂度一个数量级
- Rust vs OCaml vs C: Rust的`move`语义天然对应分离逻辑权限转移
- 形式验证成本-收益临界点：安全关键系统、基础设施代码、高价值资产保护

**代码实现**:
- `drafts/20260310_1815_rust_verification_l4.rs` (542行)
  - Verified<T, P>: L4层属性标记类型
  - VerifiedQueue: L3 Typestate + L4形式验证组合
  - Kani风格验证harnesses
  - Verus风格规范语法模拟
  - 六层渐进式边界完整展示

**新假设**:
- Rust所有权系统使得形式验证比C/OCaml简单一个数量级
- Typestate + 轻量级形式验证可实现"渐进式验证"
- AutoVerus等LLM辅助工具可将形式验证成本降低50%以上

---

### 2026-03-09 初始化
- 创建方向文档

## 关键资源

### 论文/文档
- Rust Type System documentation
- Linear Types
- Dependent Types in Rust (相关讨论)
- Kani Rust Verifier

### 开源项目
- **Verus** (CMU)
  - URL: https://github.com/verus-lang/verus
  - 状态: DARPA PROVERS资助，SOSP 2024 Distinguished Artifact Award
  - 核心: "Ask not what verification can do for Rust—ask what Rust can do for verification"
  - 应用: Microsoft存储系统验证、AWS评估中

- **Kani** (AWS)
  - URL: https://github.com/model-checking/kani
  - 状态: AWS Firecracker生产部署，27个验证harnesses在CI中运行
  - 核心: 符号执行 + CBMC后端 + SMT求解器
  - 发现: 6个bug (包括I/O rate limiter舍入错误)

- **Aeneas** (Inria/MSR)
  - URL: https://github.com/AeneasVerif/aeneas
  - 核心: Rust MIR -> 纯函数式模型 -> F*/Coq/Lean
  - 应用: Microsoft SymCrypt移植到验证Rust

- **hacspec/hax**
  - URL: https://github.com/hacspec/hax
  - 核心: Rust子集 -> F*/Coq/EasyCrypt/ProVerif
  - 应用: libcrux形式验证加密库 (Signal使用)

- **Creusot** (Inria)
  - URL: https://github.com/creusot-rs/creusot
  - 核心: Why3后端，预言编码可变借用
  - 成果: CreuSAT - 世界最快的演绎验证SAT求解器

### 技术博客
- 待补充...

## 架构洞察

### Rust 类型系统的优势
1. **所有权系统** —— 编译期内存安全，运行时零开销
2. **线性类型** —— 资源使用的一次性保证
3. **零成本抽象** —— 类型约束不增加运行时开销
4. **模式匹配** —— 穷尽性检查确保所有状态被处理

### 状态空间的 Rust 实现策略
1. **类型状态模式 (Typestate Pattern)** —— 将状态编码到类型中
2. **Phantom Types** —— 使用幽灵类型标记状态
3. **Const Generics** —— 编译期常量参数化
4. **Kani 验证** —— 模型检查验证状态空间属性

- **AutoVerus** - LLM辅助证明生成
  - 论文: "AutoVerus: Automated Proof Generation for Rust Code" (OOPSLA 2025)
  - 核心: 基于LLM自动生成Verus验证规范

### 技术博客
- Verus官方文档: https://verus-lang.github.io/verus/
- Kani验证器博客: https://model-checking.github.io/kani-verifier-blog/
- Rust verification生态综述

## 架构洞察

### Rust形式验证的独特优势

**vs C/OCaml/Idris对比**:

| 语言 | 类型系统范式 | 验证复杂度 | 关键差异 |
|------|-------------|-----------|---------|
| **Rust** | 所有权/线性类型 | FOL足够 | 编译期消除可变别名，无需分离逻辑 |
| **C** | 无类型安全 | 需要分离逻辑+内存模型 | `*`指针允许任意别名，验证极复杂 |
| **OCaml** | HM + GADTs | 需要分离逻辑 | `ref`类型允许可变别名，抽象破坏局部推理 |
| **Idris** | 依赖类型 | 类型即证明 | 最表达力但类型检查慢，适合研究 |

**关键洞察**: Rust的所有权系统使得形式验证可以使用一阶逻辑(FOL)而非分离逻辑，大幅降低验证复杂度。

### L4形式验证层在六层模型中的定位

```
L5 Capability: 权限系统控制验证范围
L4 Formal:     形式验证保证关键属性
               - Verified<T, P>: 属性标记
               - Kani: 模型检查验证harness
               - Verus风格: requires/ensures
L3 Typestate:  编译期状态转换验证
L2 Pattern:    LLM导航器选择验证策略
L1 Semantic:   类型安全的状态表示
L0 Syntax:     验证条件的可验证编码
```

### 验证工具选择指南

| 场景 | 推荐工具 | 理由 |
|------|---------|------|
| unsafe代码/安全边界 | Kani | 快速反馈，符号执行 |
| 并发协议/系统代码 | Verus | 线性幽灵类型，SMT自动化 |
| 算法正确性 | Creusot | Why3生态，证明可维护 |
| 复杂堆操作 | Prusti | Viper分离逻辑 |
| 密码学原语 | Aeneas | Lean/Coq高保证 |

## 待验证假设

- [ ] **假设1**: Rust所有权系统使得形式验证比C/OCaml简单一个数量级
  - 验证思路: 对比相同算法在Rust(Verus) vs C(VCC) vs OCaml(CFML)中的证明行数

- [ ] **假设2**: Typestate + 轻量级形式验证可实现"渐进式验证"
  - 验证思路: 实现从Typestate编译期检查逐步增强到Kani/Verus完整验证的系统

- [ ] **假设3**: AutoVerus等LLM辅助工具可将形式验证成本降低50%以上
  - 验证思路: 对比人工编写Verus规范 vs LLM生成+人工修正的效率

- [ ] **假设4**: Kani在unsafe Rust边界验证上比Verus更适合
  - 验证思路: Firecracker类项目中对比两种工具的发现bug能力和验证时间

## 下一步研究方向

1. **实现L4形式验证层的完整原型**
   - 基于现有代码草稿集成Kani验证harness
   - 添加Verus风格的规范语法

2. **对比验证: Kani vs Verus vs Creusot**
   - 选择标准算法(红黑树、FIFO队列)
   - 对比规范复杂度、验证时间、证明可维护性

3. **LLM导航器在验证策略选择中的应用**
   - 扩展LLM导航器实现
   - 基于上下文自动选择验证工具

4. **Rust验证生态的工业落地研究**
   - 深入分析AWS Firecracker的Kani应用实践
   - 研究Microsoft的Verus应用案例

## 代码草稿关联

- `drafts/20260310_1815_rust_verification_l4.rs` - L4形式验证层完整实现
  - 包含: Verified<T, P>属性标记类型
  - 包含: VerifiedQueue (L3+L4组合)
  - 包含: Kani风格验证harnesses
  - 包含: Verus风格规范语法模拟
  - 542行Rust代码
