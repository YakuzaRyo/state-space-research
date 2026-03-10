# 02_refinement_calculus

## 方向名称
形式化方法：Refine4LLM 程序精化演算

## 核心问题
程序精化如何约束 LLM 生成?

## 研究历程

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

#### 2. LLM4PR (arXiv 2406.18616)
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

## 待验证假设

- [ ] Rust 类型系统可以表达精化演算的核心概念
- [ ] 与 Verus/Flux 的集成可以自动化证明义务验证
- [ ] LLM 可以有效地选择精化法则并生成满足约束的代码
- [ ] 反例反馈机制可以有效引导 LLM 修正代码
- [ ] 精化法则库可以通过 E-graph 学习自动扩展

## 下一步研究方向

1. **实现完整的证明义务生成器**
   - 支持所有核心精化法则
   - 生成 SMT-LIB 格式输出供 Z3 验证

2. **LLM 集成接口**
   - 设计提示模板
   - 实现反例反馈处理
   - 支持回溯机制

3. **与 Verus 深度集成**
   - 将精化过程转换为 Verus 注解
   - 利用 Verus 的 SMT 后端验证

4. **精化法则学习**
   - 实现 E-graph 数据结构
   - 从历史精化过程中学习新法则

5. **案例研究**
   - 平方根算法 (已完成)
   - 二分查找
   - 数组排序
   - 链表操作

## 参考代码

- 草稿实现: `drafts/20260311_0200_refinement_calculus.rs`
- 包含：规范语句、精化法则、wp 演算、证明义务生成、LLM 接口、Verus/Flux 导出
