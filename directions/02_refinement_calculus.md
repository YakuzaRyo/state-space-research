# 02_refinement_calculus

## 方向名称
形式化方法：Refine4LLM 程序精化演算

## 核心问题
程序精化如何约束 LLM 生成?

## 研究历程

### 2026-03-11 深度研究（本次）
**研究重点**: Web Research + 假设验证 + Rust实现

**Web Research发现**:

#### 1. Refine4LLM (POPL 2025) - 核心突破
- **论文**: "Automated Program Refinement: Guide and Verify Code Large Language Model with Refinement Calculus"
- **核心贡献**: 首个将LLM与程序精化演算结合的框架
- **技术方案**:
  - 形式化规约驱动 (L_spec) 而非自然语言驱动
  - 精化法则库预定义 (Skip, Assignment, Sequential, Iteration, Alternation)
  - ATP (Z3, CVC4, Vampire, E-prover) 验证每次精化
  - 反例反馈引导LLM修正
- **实验结果**: 精化步骤减少74%，通过率提升至82%

#### 2. Flux (PLDI 2023) - Rust精化类型
- **论文**: "Flux: Liquid Types for Rust"
- **核心贡献**: 将Liquid Types引入Rust，与所有权机制结合
- **技术特点**:
  - 精化类型语法: `i32{v: 0 <= v}`
  - 自动推断循环不变式
  - Strong References支持可变借用追踪
  - CHC (Constrained Horn Clauses)后端求解

#### 3. Prusti - 契约式验证
- **核心贡献**: 基于Viper框架的Rust验证器
- **技术特点**:
  - `#[requires(...)]` / `#[ensures(...)]` 注解
  - `old(...)` 和 `result` 关键字
  - 分离逻辑支持

#### 4. 结构化生成技术
- **XGrammar**: 约束解码确保LLM输出符合语法
- **Outlines**: 基于正则/CFG的结构化生成
- **应用**: 将精化法则作为语法约束，限制LLM生成空间

**技术方案对比**:

| 方案 | 约束机制 | 验证方式 | 自动化程度 | 适用场景 |
|------|----------|----------|------------|----------|
| Refine4LLM | 精化法则 | ATP (Z3等) | 高 | 算法实现 |
| Flux | 精化类型 | SMT求解 | 高 | 系统编程 |
| Prusti | 契约注解 | Viper | 中 | 复杂规约 |
| XGrammar | 语法约束 | 解析器 | 极高 | 结构化输出 |

### 2026-03-10 深度研究
- 深入分析 POPL 2025 论文 "Automated Program Refinement: Guide and Verify Code Large Language Model with Refinement Calculus"
- 研究 LLM4PR (arXiv 2406.18616) 相关工作
- 研究 Morgan 精化演算 weakest precondition 基础理论
- 研究 Rust 形式化验证工具生态 (Verus, Flux, Prusti, RefinedRust)
- 完成 Rust 实现草稿

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

## 关键资源

### 核心论文

#### 1. Refine4LLM (POPL 2025)
**标题**: Automated Program Refinement: Guide and Verify Code Large Language Model with Refinement Calculus

**核心贡献**:
- 首个结合 LLM 与程序精化技术的框架
- 形式化规约驱动 (L_spec) 而非自然语言驱动
- 精化法则库预定义 (Skip, Assignment, Sequential Composition, Iteration, Alternation)
- ATP (自动定理证明器，如 Z3) 验证每次精化的正确性
- 实验结果：精化步骤减少 74%，通过率提升至 82% (vs 基线)

**架构组件**:
1. **形式系统**: 规范语言 L_spec、程序语言 L_pl、精化演算、证明义务生成
2. **非形式系统**: LLM 交互、提示构建、代码生成
3. **验证系统**: ATP 集成 (Z3, CoqHammer)、反例反馈

#### 2. Flux (PLDI 2023)
**标题**: Flux: Liquid Types for Rust

**核心贡献**:
- Liquid Types (精化类型) 与 Rust 所有权机制结合
- 自动推断精化类型和循环不变式
- Strong References 支持可变借用追踪
- CHC (Constrained Horn Clauses) 后端验证

**语法示例**:
```rust
#[flux::sig(fn(i32[@n]) -> i32{v: v > n})]
fn increment(x: i32) -> i32 {
    x + 1
}
```

**资源**:
- [Flux Book](https://flux-rs.github.io/)
- [论文 PDF](https://ranjitjhala.github.io/static/flux-pldi23.pdf)

#### 3. LLM4PR (arXiv 2406.18616)
**标题**: Towards Large Language Model Aided Program Refinement

**核心贡献**:
- 类似 Refine4LLM 的方法论
- 主动提示 (Actively Prompt) 与被动验证 (Passively Verify)
- 检索增强 LLM 与微调
- 规约树与精化库复用

### 理论基础

#### Morgan 精化演算 (Refinement Calculus)
**核心概念**:
- **规范语句**: `w:[pre, post]` - 框架 w 在 pre 条件下执行，满足 post 条件
- **精化关系**: `S ⊑ P` - 程序 P 精化规范 S (保持正确性)
- **Weakest Precondition (wp)**: 最弱前置条件演算

**核心精化法则**:

| 法则 | 名称 | 形式化描述 |
|------|------|-----------|
| Skip | 跳过法则 | 若 `pre ⇒ post`，则 `w:[pre, post] ⊑ skip` |
| Assignment | 赋值法则 | 若 `pre ⇒ post[E/x]`，则 `w,x:[pre, post] ⊑ x := E` |
| Sequential | 顺序组合 | `w:[pre, post] ⊑ w:[pre, mid]; w:[mid, post]` |
| Alternation | 分支法则 | 若 `pre ⇒ G1 ∨ G2`，则 `w:[pre, post] ⊑ if G1 then w:[pre∧G1, post] else w:[pre∧G2, post]` |
| Iteration | 迭代法则 | `w:[pre, post] ⊑ w:[pre, I]; while G do w:[I∧G, I∧V<V₀]`，其中 `post = I∧¬G` |

**Weakest Precondition 演算**:
- `wp(skip, Q) = Q`
- `wp(abort, Q) = false`
- `wp(x := E, Q) = Q[E/x]` (将 Q 中所有 x 替换为 E)
- `wp(C1; C2, Q) = wp(C1, wp(C2, Q))`
- `wp(if G then C1 else C2, Q) = (G ⇒ wp(C1, Q)) ∧ (¬G ⇒ wp(C2, Q))`

### Rust 形式化验证工具

#### 1. Verus
- **特点**: SMT-based (Z3)，支持 Rust 系统代码验证
- **语法**: `requires` / `ensures` / `decreases` 注解
- **适用**: 系统级代码、并发程序
- **链接**: https://github.com/verus-lang/verus

#### 2. Flux
- **特点**: Liquid Types (精化类型)，与 Rust 所有权机制结合
- **语法**: `#[flux::sig(fn(i32[@n]) -> i32{v: v > n})]`
- **适用**: 轻量级验证、数组边界检查
- **优势**: 自动推断循环不变式
- **链接**: https://github.com/flux-rs/flux

#### 3. Prusti
- **特点**: 基于 Viper 框架，支持分离逻辑
- **适用**: 复杂功能正确性规约
- **链接**: https://github.com/viperproject/prusti-dev

#### 4. RefinedRust
- **特点**: Coq 证明助手支持的基础精化类型系统
- **适用**: 高保证验证
- **链接**: https://plv.mpi-sws.org/refinedrust/

#### 5. Creusot
- **特点**: Why3 后端，支持丰富规约
- **适用**: 功能正确性验证
- **链接**: https://github.com/creusot-rs/creusot

### 开源项目

#### Refine4LLM 相关
- 论文实现 (未开源，但算法描述详细)
- 可参考 LLM4PR 的实现思路

#### ATP (自动定理证明器)
- **Z3**: Microsoft Research 的 SMT 求解器
- **Coq**: 交互式定理证明器
- **CoqHammer**: ATP 桥接 Coq

## 架构洞察

### Refine4LLM 核心机制

```
┌─────────────────────────────────────────────────────────────────┐
│                      Refine4LLM 架构                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│  │   L_spec     │────▶│   Refine     │────▶│    L_pl      │    │
│  │  (规约语言)   │     │   (精化)      │     │  (程序语言)   │    │
│  └──────────────┘     └──────────────┘     └──────────────┘    │
│         │                   │                     │             │
│         ▼                   ▼                     ▼             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    LLM Integration                       │   │
│  │  - 提示构建 (Prompt Construction)                        │   │
│  │  - 法则选择 (Law Selection)                              │   │
│  │  - 代码生成 (Code Generation)                            │   │
│  └─────────────────────────────────────────────────────────┘   │
│         │                   │                     │             │
│         ▼                   ▼                     ▼             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              ATP Verification (Z3/Coq)                   │   │
│  │  - 证明义务生成 (Proof Obligation Generation)             │   │
│  │  - 自动验证 (Automated Verification)                     │   │
│  │  - 反例反馈 (Counterexample Feedback)                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 关键设计决策

1. **形式化规约驱动 vs 自然语言驱动**
   - 优势：精确性、可验证性
   - 挑战：需要形式化方法知识
   - 解决方案：LLM 辅助规约形式化

2. **精化法则库设计**
   - 核心法则 (Core Laws): 基础、完备但步骤多
   - 扩展法则 (Extended Laws): 通过 E-graph 学习获得，减少精化深度
   - 法则选择：LLM 基于当前规约和约束选择

3. **ATP 验证机制**
   - 每次精化生成证明义务
   - Z3 等 SMT 求解器自动验证
   - 验证失败时提供反例反馈给 LLM

4. **反馈循环**
   ```
   规约 ──▶ LLM 选择法则 ──▶ 生成代码 ──▶ ATP 验证
    ▲                                      │
    └──────── 反例反馈 ◀── 验证失败 ◀─────┘
   ```

### 与状态空间架构的结合点

1. **状态空间视角**
   - 每个规约是一个状态
   - 精化法则是状态转移函数
   - ATP 验证确保状态转移的正确性

2. **硬性边界**
   - 精化必须在预定义的法则集合内进行
   - LLM 不能随意生成代码，必须遵循精化法则
   - 验证失败时回溯到上一个有效状态

3. **层次结构**
   - 高层规约 (抽象) ──▶ 低层规约 (具体)
   - 类似于状态空间中的抽象层次

## 研究假设与验证 (2026-03-11)

### 本次研究提出的假设

| 假设 | 描述 | 置信度 | 验证状态 |
|------|------|--------|----------|
| H1 | 精化法则可作为LLM的结构化生成约束 | 高 | 已验证 (Refine4LLM) |
| H2 | Rust类型系统可编码精化演算核心概念 | 高 | 代码实现完成 |
| H3 | Weakest Precondition演算可系统化验证 | 高 | 数学基础确立 |
| H4 | 与现有工具(Verus/Flux)集成可行 | 中 | 导出格式已实现 |
| H5 | 结构化解码(XGrammar)可加速精化选择 | 中 | 待验证 |

### 详细验证结果

#### H1: 技术假设 - 精化如何约束LLM生成

**约束机制**:
1. **状态空间限制**: 规范语句 `w:[pre, post]` 定义有效状态
2. **转移限制**: 仅允许预定义精化法则（Skip/Assignment/Sequential/Alternation/Iteration）
3. **验证要求**: 每步生成证明义务，ATP验证通过后方可继续
4. **反馈循环**: 验证失败提供反例，引导LLM修正

**关键洞察**: 将"生成正确代码"问题转化为"选择并应用正确精化法则"问题——搜索空间大幅缩小。

#### H2: 实现假设 - Rust精化框架

**实现组件**:
```rust
// 类型化规范
pub struct RefinedSpecification {
    pub frame: Vec<TypedVariable>,      // 带边界信息的变量
    pub precondition: RefinedPredicate,  // 结构化谓词
    pub postcondition: RefinedPredicate,
    pub refinement_depth: usize,         // 精化深度追踪
    pub parent: Option<Box<...>>,        // 支持回溯
}

// 精化法则（带证明义务）
pub fn assignment_law(...) -> (RefinedResult, Vec<ProofObligation>)
```

**验证工具集成**:
- Verus导出: `requires`/`ensures`语法
- Flux导出: 精化类型语法
- 支持Z3 SMT-LIB格式

#### H3: 性能假设 - 质量影响

**量化对比**:

| 指标 | 精化约束 | 直接LLM | 改进 |
|------|----------|---------|------|
| HumanEval通过率 | 82% | ~65% | +17% |
| 精化步骤 | 基准 | +74% | -74% |
| 错误检测 | 每步即时 | 最终验证 | 早期发现 |
| 验证方式 | 组合式 | 整体式 | 可组合 |

#### H4: 适用性假设 - 应用领域

**高度适用**:
- 安全关键系统（航空、医疗）
- 算法实现（数学规格清晰）
- 系统编程（内存+功能正确性）
- 密码学协议（安全属性可形式化）
- 教育场景（形式化方法教学）

**不太适用**:
- 探索性编程（规格不明）
- 快速原型（开销过高）
- UI/UX代码（难以形式化）
- 自然语言处理（语义规格困难）
- 创意编程（无明确正确性标准）

## Rust 实现思路

### 数据结构表示

```rust
// 规范语句: w:[pre, post]
struct Specification {
    frame: Vec<String>,           // 可变变量集合
    precondition: Predicate,      // 前置条件
    postcondition: Predicate,     // 后置条件
}

// 一阶逻辑谓词
enum Predicate {
    True, False,
    Eq(Term, Term),
    Lt(Term, Term),
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Implies(Box<Predicate>, Box<Predicate>),
    Forall(String, Box<Predicate>),
    Exists(String, Box<Predicate>),
}

// 程序命令
enum Command {
    Skip,
    Assignment(String, Term),
    Seq(Box<Command>, Box<Command>),
    If(Predicate, Box<Command>, Box<Command>),
    While { guard: Predicate, body: Box<Command>, invariant: Predicate, variant: Term },
    Spec(Specification),  // 混合程序
}
```

### 精化法则实现

```rust
struct RefinementLaws;

impl RefinementLaws {
    // Skip Law: 若 pre ⇒ post，则 w:[pre, post] ⊑ skip
    fn skip_law(spec: &Specification) -> RefinementResult {
        // 生成证明义务: pre ⇒ post
        let obligation = Predicate::Implies(
            Box::new(spec.precondition.clone()),
            Box::new(spec.postcondition.clone()),
        );
        // 验证后返回 skip
        RefinementResult::Success(Command::Skip)
    }

    // Assignment Law: 若 pre ⇒ post[E/x]，则 w,x:[pre, post] ⊑ x := E
    fn assignment_law(spec: &Specification, var: &str, expr: &Term) -> RefinementResult {
        let post_substituted = spec.postcondition.substitute(var, expr);
        let obligation = Predicate::Implies(
            Box::new(spec.precondition.clone()),
            Box::new(post_substituted),
        );
        RefinementResult::Success(Command::Assignment(var.to_string(), expr.clone()))
    }

    // Sequential Composition Law
    fn sequential_composition(spec: &Specification, mid: Predicate) -> RefinementResult {
        let spec1 = Specification::new(spec.frame.clone(), spec.precondition.clone(), mid.clone());
        let spec2 = Specification::new(spec.frame.clone(), mid, spec.postcondition.clone());
        RefinementResult::Success(Command::Seq(
            Box::new(Command::Spec(spec1)),
            Box::new(Command::Spec(spec2)),
        ))
    }
}
```

### 与 Verus/Flux 集成

```rust
trait VerifiableExport {
    fn to_verus(&self) -> String;
    fn to_flux(&self) -> String;
}

impl VerifiableExport for Specification {
    fn to_verus(&self) -> String {
        // 转换为 requires/ensures 语法
        format!(
            "fn spec_fn({})\n    requires {}\n    ensures {}\n{{ ... }}",
            self.frame.join(", "),
            format_predicate(&self.precondition),
            format_predicate(&self.postcondition)
        )
    }

    fn to_flux(&self) -> String {
        // 转换为精化类型语法
        format!(
            "#[flux::sig(fn(...) ensures ...)]"
        )
    }
}
```

## 研究轨迹

- **本次深度研究记录**: `logs/trails/02_refinement_calculus/20260311_1200_trail.md`
- **本次代码实现**: `drafts/20260311_Refine4LLM.rs` (939行)
- **历史研究记录**: `logs/trails/02_refinement_calculus/20260311_0800_refinement_trail.md`
- **历史代码实现**: `drafts/20260311_0800_refinement_calculus.rs` (约1150行)

## 下一步研究方向

### 近期 (1-2周)

1. **ATP集成**
   - [ ] 实现Z3 SMT-LIB导出
   - [ ] 连接实际定理证明器
   - [ ] 处理反例解析

2. **LLM集成**
   - [ ] 设计法则选择提示模板
   - [ ] 实现OpenAI/Anthropic API接口
   - [ ] 构建验证失败反馈循环

3. **扩展案例**
   - [ ] 二分查找完整精化
   - [ ] 数组排序算法
   - [ ] 链表操作

### 中期 (1-2月)

4. **Flux深度集成**
   - [ ] 生成Flux兼容精化类型
   - [ ] 自动不变式推断
   - [ ] 所有权感知规格生成

5. **精化法则学习**
   - [ ] 实现E-graph数据结构
   - [ ] 从历史精化学习新法则
   - [ ] 领域特定法则库

6. **结构化解码实验**
   - [ ] 使用XGrammar约束精化选择
   - [ ] 对比约束vs无约束的生成质量
   - [ ] 测量精化步骤减少率

### 长期 (3-6月)

7. **精化引导训练**
   - [ ] 精化法则选择微调数据集
   - [ ] 训练证明义务生成模型
   - [ ] 构建验证精化数据集

8. **IDE集成**
   - [ ] VS Code交互式精化扩展
   - [ ] 实时证明义务显示
   - [ ] 逐步精化调试

## 参考代码

- **最新实现**: `drafts/20260311_Refine4LLM.rs` (本次深度研究版本，939行)
- **早期实现**: `drafts/20260311_0800_refinement_calculus.rs` (基础版本，约1150行)
- **包含组件**: 规范语句、精化法则、wp演算、证明义务生成、LLM接口、Verus/Flux导出

## 来源

- [Refine4LLM Paper (POPL 2025)](https://popl25.sigplan.org/details/POPL-2025-popl-research-papers/69/Automated-Program-Refinement-Guide-and-Verify-Code-Large-Language-Model-with-Refinem)
- [Flux: Liquid Types for Rust](https://ranjitjhala.github.io/static/flux-pldi23.pdf)
- [Flux Book](https://flux-rs.github.io/)
- [Prusti Documentation](https://viperproject.github.io/prusti-dev/user-guide/)
- [Graz University of Technology - Hoare Logic notes](https://www.isec.tugraz.at/wp-content/uploads/2019/09/vt06-hoare-logic.pdf)
- [EPFL - Hoare Logic: Weakest Preconditions](https://lara.epfl.ch/w/_media/fv20/lec08-01.pdf)
