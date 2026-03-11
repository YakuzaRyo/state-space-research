# 06_formal_verification

## 方向名称
形式验证：Clover/Dafny/Verus/Kani 验证集成

## 核心问题
形式验证如何过滤 LLM 输出?

## 研究历程

### 2026-03-11 深度研究

#### 1. Web Research 关键发现

**2025年最新研究突破：**

| 项目 | 核心贡献 | 关键数据 | 来源 |
|------|----------|----------|------|
| **AutoVerus** | 多智能体LLM自动生成Rust证明 | >90%成功率，50%在<30秒解决 | OOPSLA 2025 Distinguished Artifact Award |
| **dafny-annotator** | AI辅助Dafny规格生成 | 成功率15.7%→50.6% | 2025年6月发布 |
| **VerusSync** | 并发验证工具包 | 并发证明与顺序证明同样简单 | 2025年Verus扩展 |
| **PGS框架** | PBT桥接LLM生成与验证 | pass@1提升23.1%-37.3% | arXiv:2506.18315 |

**CLEVER基准测试启示 (2025):**
- 161个形式验证问题测试
- GPT-4o: 84.5%编译率，**0.6%证明成功率**
- Claude-3.7: 87%编译率，**0.6%证明成功率**
- **关键洞察**: LLM能写运行代码，但极难写可证明正确代码

**Kani并发验证现状 (2025):**
- ❌ 仍不支持async/await
- ❌ 并发特性超出范围
- ⚠️ 遇到并发代码时按顺序处理并发出警告
- 底层CBMC支持并发，但Kani前端需要开发

**Verus并发验证突破:**
- ✅ VerusSync支持并发验证
- ✅ 已验证Node Replication并发库
- ✅ AWS Nitro隔离引擎生产应用
- ✅ 比IronSync快两个数量级

---

#### 2. 提出的假设与验证

**H1: 形式验证无法完全保证LLM生成代码的安全性** - 置信度: **高**
- ✅ **已验证**
- 依据: Kani有界验证无法覆盖无限状态空间
- 依据: CLEVER基准显示LLM证明成功率仅0.6%
- 依据: 形式验证只能验证规约中指定的属性

**H2: Kani验证器与LLM的集成应采用CEGIS循环架构** - 置信度: **高**
- ✅ **已验证**
- 依据: 具体反例比通用错误消息显著提升修复成功率(16% vs 6%)
- 依据: AutoVerus多智能体架构成功率>90%
- 依据: 93%案例显示改进或无变化，仅7%退化

**H3: 形式验证对LLM响应时间的影响在可接受范围内** - 置信度: **中**
- ⚠️ **部分成立**
- Flux(<1分钟)、Kani(1-5分钟): 适合交互式
- Verus(5-30分钟)、Dafny(10-60分钟): 边缘或不适合
- **结论**: 需要分层验证策略

**H4: 内存安全和panic-free属性最适合形式验证** - 置信度: **高**
- ✅ **已验证**
- 依据: Kani在AWS Firecracker成功验证I/O速率限制器
- 依据: 功能正确性证明需要更多人工干预(循环不变量)
- 依据: 这些属性可完全自动化验证

---

#### 3. 代码实现验证

**Kani验证模式:**
```rust
#[kani::proof]
fn verify_llm_binary_search() {
    let arr: [i32; 5] = kani::any();
    let key: i32 = kani::any();
    kani::assume(/* 前置条件 */);
    let result = llm_binary_search(&arr, key);
    assert!(/* 后置条件 */);
}
```

**CEGIS循环架构:**
```rust
pub struct CegisLoop<G, V> {
    generator: G,  // LLM代码生成
    verifier: V,   // 形式验证器
    max_iterations: usize,
}
// 循环: 生成 -> 验证 -> 反例反馈 -> 修复
```

**Clover六步一致性检查:**
1. anno-sound: 代码满足形式规约
2. anno-complete: 规约能重建等价代码
3. anno2doc: 规约与文档一致性
4. doc2anno: 文档与规约一致性
5. code2doc: 代码与文档一致性
6. doc2code: 文档与代码一致性

**验证过滤器设计:**
```rust
pub struct VerificationFilter {
    level: VerificationLevel,  // L1-L4
    verifiers: Vec<...>,
}
// 只有通过验证的代码才能进入状态空间
```

---

#### 4. 假设边界条件和限制

| 假设 | 边界条件 | 限制 |
|------|----------|------|
| H1 | 有界验证适用于有限状态 | 并发代码在Kani中不支持 |
| H2 | 无理论收敛保证 | 依赖LLM理解反例能力 |
| H3 | 验证时间指数增长 | 不适合实时交互场景 |
| H4 | 需要准确规约定义 | unsafe代码支持有限 |

---

### 2026-03-10 深度研究

#### 1. Clover框架：闭环可验证代码生成

**Clover** (Closed-Loop Verifiable Code Generation) 是斯坦福大学与VMware Research开发的革命性框架，通过一致性检查解决AI生成代码的可信度问题。

**核心机制**:
- **三组件一致性检查**: Code(实现) + Docstrings(文档) + Formal annotations(形式规约)
- **六步验证流程**:
  1. `anno-sound`: 代码满足形式规约 (Dafny/Verus验证)
  2. `anno-complete`: 规约强度足以重建等价代码
  3. `anno2doc`: 规约与文档一致性
  4. `doc2anno`: 文档与规约一致性
  5. `code2doc`: 代码与文档一致性
  6. `doc2code`: 文档与代码一致性

**关键创新**:
- **重构测试**: 组件间相互重建并检查等价性
- **零假阳性保证**: 无错误代码能通过全部六项检查
- **评估结果**: CloverBench上87%接受率，0%假阳性

**参考**:
- [Clover论文](https://theory.stanford.edu/~barrett/pubs/SSP+24.pdf)
- [Stanford AI Lab博客](http://ai.stanford.edu/blog/clover/)

---

#### 2. Dafny：Microsoft的验证感知编程语言

**Dafny** 由K. Rustan M. Leino在Microsoft Research开发，现由AWS维护，是业界最成熟的验证感知语言之一。

**核心特性**:
```dafny
method BinarySearch(a: array<int>, key: int) returns (index: int)
  requires forall i, j :: 0 <= i < j < a.Length ==> a[i] <= a[j]  // 前置: 数组有序
  ensures 0 <= index ==> a[index] == key      // 后置1: 找到时正确
  ensures index < 0 ==> forall k :: 0 <= k < a.Length ==> a[k] != key  // 后置2: 未找到时不存在
{
  // 实现与验证...
}
```

**规格语法**:
- `requires`: 前置条件
- `ensures`: 后置条件
- `modifies`: 帧条件(可修改对象)
- `invariant`: 循环不变量
- `decreases`: 终止度量
- `forall`/`exists`: 量词

**AWS生产应用**:
- AWS Encryption SDK
- AWS Database Encryption SDK
- AWS认证引擎

**参考**:
- [Dafny官方文档](https://dafny.org/dafny/DafnyRef/DafnyRef)
- [Dafny教程PDF](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/12/krml221.pdf)

---

#### 3. Verus：Rust原生验证工具

**Verus** 是CMU/VMware/Microsoft Research联合开发的SMT-based Rust验证工具，允许使用Rust本身编写规约和证明。

**三模式系统**:
| 模式 | 属性 | 描述 |
|------|------|------|
| `#[spec]` | Ghost代码 | 不检查线性，可自由复制 |
| `#[proof]` | Ghost代码 | 检查线性，表示权限 |
| `#[exec]` | 可执行代码 | 编译为机器码 |

**规格示例**:
```rust
use vstd::prelude::*;

verus! {

fn octuple(x1: i8) -> (x8: i8)
    requires
        -16 <= x1 < 16,  // 前置: 防止溢出
    ensures
        x8 == 8 * x1,    // 后置: 功能正确
{
    let x2 = x1 + x1;
    let x4 = x2 + x2;
    x4 + x4
}

}
```

**AutoVerus (2025)**:
- 使用多智能体LLM系统自动生成Rust代码的正确性证明
- 150个非平凡证明任务上成功率>90%
- 50%在<30秒或3次LLM调用内解决
- 获得OOPSLA 2025 Distinguished Artifact Award

**参考**:
- [Verus教程](https://verus-lang.github.io/verus/guide/)
- [AutoVerus论文](https://www.microsoft.com/en-us/research/publication/autoverus-automated-proof-generation-for-rust-code/)

---

#### 4. Kani：AWS的Rust模型检查器

**Kani** 是AWS开发的位精确模型检查器，使用有界模型检查验证Rust代码。

**工作原理**:
- 将Rust MIR转换为C goto语句
- 使用CBMC作为底层验证引擎
- 符号执行与部分约束输入

**验证能力**:
- ✅ 内存安全(空指针、越界)
- ✅ 用户断言(`assert!`)
- ✅ Panic-free验证
- ✅ 算术溢出检测
- ✅ 部分unsafe代码
- ❌ 并发构造(暂不支持)

**AWS Firecracker生产应用**:
- 验证I/O速率限制器
- 验证VirtIO传输层
- 27个Kani harness集成到CI(~15分钟)

**参考**:
- [Kani博客](https://model-checking.github.io/kani-verifier-blog/)
- [Firecracker验证案例](https://model-checking.github.io/kani-verifier-blog/2023/08/31/using-kani-to-validate-security-boundaries-in-aws-firecracker.html)

---

#### 5. Creusot：Why3后端的Rust验证

**Creusot** 是Inria开发的演绎验证工具，将Rust代码翻译为WhyML进行验证。

**技术特点**:
- **PEARLITE规约语言**: 支持预言(`final`操作符`^`)
- **预言编码**: 基于RustHorn的mutable borrow编码
- **一阶逻辑**: 编码为FOL而非分离逻辑，验证速度快一个数量级

**验证工作流**:
```
Rust + PEARLITE注解 → WhyML → VCs → SMT求解器(Z3/CVC5/Alt-Ergo)
```

**CreuSAT案例**:
- 世界最快的演绎验证SAT求解器
- 展示了生产级验证的可行性

**参考**:
- [Creusot论文](https://jhjourdan.mketjh.fr/pdf/denis2022creusot.pdf)
- [CreuSAT论文](https://sarsko.github.io/_pages/SarekSkotåm_thesis.pdf)

---

#### 6. Flux：精化类型验证器

**Flux** (UC San Diego) 是受Liquid Types启发的Rust精化类型系统。

**核心特性**:
- 基于CHC的liquid类型推断
- Houdini算法自动化
- `&strg`强引用扩展支持强更新

**示例**:
```rust
#[flux::sig(fn(&strg Vec<i32>[@n]) -> i32{v: 0 <= v && v < n})]
fn pop(v: &mut Vec<i32>) -> i32 {
    v.pop().unwrap()
}
```

**限制**:
- 仅支持safe Rust
- 动态选择的mutable reference跟踪精度有限
- 高阶函数支持有限

**参考**:
- [Flux GitHub](https://github.com/flux-rs/flux)

---

#### 7. 验证技术对比

| 工具 | 方法 | Unsafe支持 | 并发支持 | 自动化 | 成熟度 |
|------|------|------------|----------|--------|--------|
| **Kani** | 有界模型检查 | ✅ 完整 | ❌ 不支持 | ⭐⭐⭐⭐⭐ | 生产级(AWS) |
| **Verus** | SMT-based | ⚠️ 有限 | ✅ 支持 | ⭐⭐⭐⭐ | 快速成熟 |
| **Creusot** | 演绎验证 | ❌ 不支持 | ❌ 不支持 | ⭐⭐⭐⭐ | 研究级 |
| **Prusti** | 分离逻辑 | ❌ 不支持 | ❌ 不支持 | ⭐⭐⭐ | 2024停止维护 |
| **Flux** | 精化类型 | ❌ 不支持 | ❌ 不支持 | ⭐⭐⭐⭐⭐ | 研究级 |

---

#### 8. SMT求解器基础

**Z3 vs CVC5**:
| 特性 | Z3 | CVC5 |
|------|-----|------|
| 来源 | Microsoft Research | Stanford/Iowa |
| 许可证 | MIT | BSD-3-Clause |
| Python API | 原生成熟 | 兼容Z3风格 |
| 量词处理 | 强 | 竞争力强 |
| 特殊特性 | 程序验证优化 | SyGuS合成、插值 |

**验证条件生成(VCG)**:
- 计算最弱前置条件(Weakest Precondition)
- 生成验证条件(VCs)
- SMT求解器自动证明

**参考**:
- [Z3教程](https://www.cs.colostate.edu/~cs440/spring19/slides/z3-tutorial.pdf)
- [CVC5论文](https://www-cs.stanford.edu/~preiner/publications/2022/BarbosaBBKLMMMN-TACAS22.pdf)

---

#### 9. LLM+形式验证集成方案

**CEGIS循环** (Counterexample-Guided Inductive Synthesis):
```
LLM生成代码 → 形式验证器检查 → 反例反馈 → LLM修复 → 循环
```

**关键发现**:
- 具体反例比通用错误消息显著提升修复成功率(16% vs 6%)
- 93%的案例显示改进或无变化，仅7%退化
- CEGIS风格循环正成为LLM+形式验证的标准模式

**主要研究**:
1. **Verity**: Neuro-symbolic合成框架，Z3+LLM
2. **Neuro-symbolic Loop Invariant Inference (ASE 2024)**: BMC+LLM三类型反馈
3. **Dehallucinating LLMs**: 反例驱动的迭代精化
4. **Eudoxus 2.0**: BMC引导的自动形式化改进

---

#### 10. 形式规约作为状态约束

**状态空间应用**:
- **形式规约作为状态约束**: 定义有效状态的数学边界
- **验证作为状态转移检查**: 确保每次转移保持约束
- **不变量维护机制**: 自动推导最弱不变量保持条件

**B方法/Event-B中的不变量维护**:
- 通过精化关系构建模型序列
- 证明义务(POs)验证不变量保持
- 量化消除(QE)自动推导最弱不变量保持条件

---

#### 11. 安全保证级别

**DO-178C航空软件标准**:
| DAL | 失效条件 | 目标数 | 形式方法适用性 |
|-----|----------|--------|----------------|
| A | 灾难性 | 71 | 强烈推荐 |
| B | 危险 | 69 | 推荐 |
| C | 重大 | 62 | 可选 |
| D | 轻微 | 26 | 可选 |
| E | 无影响 | 0 | 不适用 |

**seL4与EAL7**:
- seL4的形式验证**超越EAL7要求**
- EAL7仅要求设计到实现的非形式映射
- seL4提供到二进制的形式证明
- ~130万行机器检查证明

**L4验证层实现路径**:
1. **L1**: 类型安全 (Rust编译器)
2. **L2**: 内存安全 (MIRI, Kani)
3. **L3**: 功能正确性 (Verus, Creusot)
4. **L4**: 完整形式验证 (seL4级别)

---

## 关键资源

### 2025年最新论文
- [AutoVerus: Automated Proof Generation for Rust Code](http://jaylorch.us.s3.amazonaws.com/publications/autoverus.pdf) - OOPSLA 2025
- [Dafny as Verification-Aware Intermediate Language](https://fugumt.com/fugumt/paper_check/2501.06283v1_enmode) - 2025
- [Use Property-Based Testing to Bridge LLM Code Generation and Validation](https://ui.adsabs.harvard.edu/abs/arXiv:2506.18315) - 2025
- [Clover: Closed-Loop Verifiable Code Generation](https://theory.stanford.edu/~barrett/pubs/SSP+24.pdf)
- [Verus: Verifying Rust Programs using Linear Ghost Types](https://www.research-collection.ethz.ch/handle/20.500.11850/610518)
- [Creusot: Deductive Verification of Rust](https://jhjourdan.mketjh.fr/pdf/denis2022creusot.pdf)

### 开源项目
- Dafny: https://github.com/dafny-lang/dafny
- Verus: https://github.com/verus-lang/verus
- Kani: https://github.com/model-checking/kani
- Creusot: https://github.com/xldenis/creusot
- Flux: https://github.com/flux-rs/flux
- dafny-annotator: https://dafny.org/blog/2025/06/21/dafny-annotator/

### 技术文档
- [Dafny参考手册](https://dafny.org/dafny/DafnyRef/DafnyRef)
- [Verus教程](https://verus-lang.github.io/verus/guide/)
- [Kani文档](https://model-checking.github.io/kani/)
- [Kani Rust特性支持](https://model-checking.github.io/kani/rust-feature-support.html)

---

## 架构洞察

### 形式验证核心机制
1. **前置/后置条件** —— 明确定义函数的契约
2. **不变量验证** —— 循环和状态转换的不变量检查
3. **定理证明** —— 使用SMT求解器验证程序正确性
4. **反例驱动修复** —— 验证失败提供具体反馈指导LLM修正
5. **六步一致性检查** —— Clover框架的三组件相互验证

### 与状态空间的结合点
- **形式规约作为状态约束**: 定义有效状态边界
- **验证作为准入测试**: 只有通过验证的代码才能进入状态空间
- **不变量维护**: 验证确保状态转移保持约束
- **L4保证**: 完整形式验证提供最高级别安全保证

### 工具选择决策矩阵

| 场景 | 推荐工具 | 理由 | 验证时间 |
|------|----------|------|----------|
| 快速内存安全检查 | Kani | 自动化程度高，AWS生产验证 | 1-5分钟 |
| 功能正确性证明 | Verus | Rust原生，规格用Rust编写 | 5-30分钟 |
| 完整形式验证 | Dafny | 成熟稳定，AWS广泛使用 | 10-60分钟 |
| 精化类型验证 | Flux | 轻量级，自动化推断 | <1分钟 |
| 并发代码验证 | Verus | 唯一支持并发的Rust验证器 | 5-30分钟 |

---

## 待验证假设

### 已验证假设
- [x] **H1**: 形式验证无法完全保证LLM生成代码的安全性 → **已验证，成立(高置信度)**
- [x] **H2**: Kani验证器与LLM的集成应采用CEGIS循环架构 → **已验证，成立(高置信度)**
- [x] **H3**: 形式验证对LLM响应时间的影响在可接受范围内 → **已验证，部分成立(中置信度)**
- [x] **H4**: 内存安全和panic-free属性最适合形式验证 → **已验证，成立(高置信度)**

### 遗留假设
- [ ] AutoVerus多智能体架构在状态空间中的应用
- [ ] 分层验证策略(L1-L4)的性能权衡
- [ ] 验证缓存机制的有效性
- [ ] 并发代码验证与状态空间的集成

---

## 下一步研究方向

### 短期目标 (1-2周)
1. **实现Kani验证管道**: 集成cargo-kani，自动生成harness
2. **开发CEGIS循环原型**: 实现LLM生成+验证+反馈闭环
3. **集成到调度器**: 将验证作为任务系统的准入控制

### 中期目标 (1个月)
4. **分层验证架构**: 实现L1-L4渐进式验证策略
5. **验证缓存机制**: 避免重复验证，加速响应时间
6. **AutoVerus架构研究**: 多智能体验证在状态空间中的应用

### 长期目标 (3个月)
7. **生产级CI/CD集成**: 验证管道与持续集成结合
8. **状态空间深度集成**: 验证结果指导状态探索方向
9. **并发验证探索**: VerusSync在分布式系统中的应用

---

## 研究产出

### 代码草稿
- `drafts/20260311_形式验证.rs` - 包含验证假设的核心实现

### 轨迹日志
- `logs/trails/06_formal_verification/20260311_104721_kimi_trail.md` - 完整研究过程记录
