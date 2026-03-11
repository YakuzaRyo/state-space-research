# 形式验证深度研究轨迹日志

**研究方向**: 06_formal_verification - 形式验证如何过滤LLM输出
**日期**: 2026-03-11
**研究时长**: ~28分钟
**研究员**: Claude Code

---

## 研究目标

探索形式验证工具（Clover、Dafny、Verus、Kani）如何用于验证和过滤LLM生成的代码输出，建立可验证的代码生成管道。

---

## Step 1: Web Research (8-10分钟)

### 1.1 Clover: 闭环可验证代码生成

**来源**: [Clover论文](https://theory.stanford.edu/~barrett/pubs/SSP+24.pdf)

**核心发现**:
- Clover是斯坦福大学与VMware Research开发的革命性框架
- 通过六步一致性检查解决AI生成代码的可信度问题
- **关键创新**: 将正确性验证转化为一致性检查

**六步验证流程**:
1. `anno-sound`: 代码满足形式规约 (Dafny/Verus验证)
2. `anno-complete`: 规约强度足以重建等价代码
3. `anno2doc`: 规约与文档一致性
4. `doc2anno`: 文档与规约一致性
5. `code2doc`: 代码与文档一致性
6. `doc2code`: 文档与代码一致性

**评估结果**:
- CloverBench上87%接受率 (k=10)
- **0%假阳性** - 无错误代码能通过全部六项检查
- 在MBPP-DFY-50中发现6个错误程序

### 1.2 Dafny: Microsoft的验证感知编程语言

**来源**: [Dafny官方文档](https://dafny.org/)

**核心特性**:
```dafny
method BinarySearch(a: array<int>, key: int) returns (index: int)
  requires forall i, j :: 0 <= i < j < a.Length ==> a[i] <= a[j]
  ensures 0 <= index ==> a[index] == key
  ensures index < 0 ==> forall k :: 0 <= k < a.Length ==> a[k] != key
```

**规格语法**:
- `requires`: 前置条件
- `ensures`: 后置条件
- `modifies`: 帧条件
- `invariant`: 循环不变量
- `decreases`: 终止度量

**生产应用**: AWS Encryption SDK、AWS Database Encryption SDK

### 1.3 Verus: Rust原生验证工具

**来源**: [Verus教程](https://verus-lang.github.io/event-sites/2024-sosp/)

**核心特性**:
- SMT-based Rust验证工具
- 使用Rust本身编写规格和证明
- 三模式系统: `#[spec]`、`#[proof]`、`#[exec]`

**示例**:
```rust
verus! {
fn octuple(x1: i8) -> (x8: i8)
    requires -16 <= x1 < 16,
    ensures x8 == 8 * x1,
{
    let x2 = x1 + x1;
    let x4 = x2 + x2;
    x4 + x4
}
}
```

**AutoVerus (2025)**: 使用多智能体LLM自动生成证明，150个任务成功率>90%

### 1.4 Kani: AWS的Rust模型检查器

**来源**: [Kani文档](https://model-checking.github.io/kani/)

**核心特性**:
- 位精确模型检查器
- 使用CBMC作为底层引擎
- 符号执行与部分约束输入

**验证能力**:
- 内存安全（空指针、越界）
- 用户断言
- Panic-free验证
- 算术溢出检测
- 部分unsafe代码支持

**基本验证模式**:
```rust
#[kani::proof]
fn check_my_property() {
    let input = kani::any();
    kani::assume(precondition(input));
    let output = function_under_test(input);
    assert!(meets_specification(input, output));
}
```

### 1.5 LLM输出形式验证研究

**来源**: [MATH-VF论文](https://arxiv.org/html/2505.20869v1)

**关键发现**:
- MATH-VF: Step-Wise Formal Verification for LLM-Based Mathematical Problem Solving
- **Formalizer**: 将自然语言转换为SimpleMath形式语言
- **Critic**: 集成SymPy和Z3-SMT求解器验证每一步

**优势**:
- 无需训练（training-free）
- 比PRM更稳定
- 允许前提与结论间的gap（比Coq更灵活）

---

## Step 2: 假设提出 (3-5分钟)

### 2.1 技术假设

**假设1**: 形式验证通过以下机制过滤LLM输出
- **前置/后置条件检查**: 验证函数契约
- **不变量验证**: 确保循环和状态转换的正确性
- **反例驱动**: 验证失败提供具体反馈指导修正

**假设2**: Clover六步一致性检查提供比单一验证更强的保证
- 重构测试捕获规格与实现间的不一致
- 零假阳性保证适用于关键系统

### 2.2 实现假设

**假设3**: Rust中可集成验证工具的方式
- Kani: 通过`#[kani::proof]`属性和`kani::any()`生成符号值
- Verus: 通过`verus!`宏和规格函数
- 外部工具: 通过命令行调用和解析输出

**假设4**: CEGIS循环可实现自动化验证-修复流程
- LLM生成代码 -> 形式验证 -> 反例反馈 -> LLM修复
- 具体反例比通用错误消息显著提升修复成功率(16% vs 6%)

### 2.3 性能假设

**假设5**: 验证开销的可接受范围
- Kani: 有界模型检查，适合小数组和有限循环
- Verus: SMT求解，复杂证明可能需要手动辅助
- 目标: 单次验证 < 5分钟，CEGIS循环 < 30分钟

### 2.4 适用性假设

**假设6**: 形式验证适用于以下场景
- 算法实现（排序、搜索）
- 状态转换系统
- 安全关键代码路径
- 不适合: 复杂业务逻辑、UI代码、自然语言处理

---

## Step 3: 验证实现 (10-12分钟)

### 3.1 代码实现

创建了完整的Rust验证框架草稿 (`drafts/20260311_0800_formal_verification.rs`)，包含:

**Part 1: Kani验证集成**
```rust
#[kani::proof]
fn verify_abs_with_precondition() {
    let x: i64 = kani::any();
    kani::assume(x != i64::MIN);  // 前置条件
    let result = x.abs();
    assert!(result >= 0);  // 后置条件
}
```

**Part 2: Clover验证器实现**
```rust
pub struct CloverVerifier {
    max_iterations: usize,
    timeout_secs: u64,
}

impl CloverVerifier {
    pub fn verify(&self, code: &str, annotations: &str, docstring: &str)
        -> VerificationReport {
        // 六步验证流程实现
    }
}
```

**Part 3: CEGIS循环实现**
```rust
pub struct CegisLoop {
    max_iterations: usize,
    verifier: CloverVerifier,
}

impl CegisLoop {
    pub fn run(&self, spec: &str, llm_generator: &mut dyn LlmGenerator)
        -> CegisResult {
        // 验证-反馈-修复循环
    }
}
```

**Part 4: 状态空间集成**
```rust
pub struct VerificationFilter {
    constraints: Vec<StateConstraint>,
}

impl VerificationFilter {
    pub fn validate(&self, state: &State) -> ValidationResult {
        // 验证状态是否满足形式约束
    }
}
```

### 3.2 关键验证点

1. **Kani验证模式验证**:
   - `kani::any()` 生成符号值
   - `kani::assume()` 设置前置条件
   - `assert!()` 验证后置条件
   - 发现i64::abs()溢出问题

2. **Clover六步验证验证**:
   - 代码满足形式规约 (anno-sound)
   - 规约能重建等价代码 (anno-complete)
   - 文档与实现一致性检查

3. **CEGIS循环验证**:
   - 反例反馈机制
   - 迭代修复流程
   - 最大迭代次数限制

---

## Step 4: 文档更新

更新了 `directions/06_formal_verification.md`，新增:

1. **Clover框架详细分析**
2. **Dafny验证语言介绍**
3. **Verus Rust验证工具**
4. **Kani模型检查器**
5. **验证技术对比表**
6. **LLM+形式验证集成方案**
7. **CEGIS循环实现**
8. **安全保证级别分析**

---

## Step 5: 下一步研究方向

### 5.1 短期目标 (1-2周)

1. **实现Kani验证管道**
   - 集成cargo-kani到构建流程
   - 为LLM生成代码自动生成harness
   - 实现验证结果解析和反例提取

2. **开发Clover风格验证器**
   - 实现六步一致性检查
   - 集成LLM进行语义等价性检查
   - 构建验证报告生成器

### 5.2 中期目标 (1个月)

1. **CEGIS循环完整实现**
   - LLM生成 + 验证 + 反馈闭环
   - 反例格式标准化
   - 修复成功率度量

2. **状态空间约束语言**
   - 基于Dafny/Verus规格语法
   - 约束到验证条件的自动转换
   - 运行时约束检查

### 5.3 长期目标 (3个月)

1. **分层验证架构**
   - L1: 类型安全 (Rust编译器)
   - L2: 内存安全 (MIRI, Kani)
   - L3: 功能正确性 (Verus, Creusot)
   - L4: 完整形式验证

2. **生产级集成**
   - CI/CD管道集成
   - 验证缓存和增量验证
   - 开发者体验优化

---

## 关键发现总结

### 形式验证过滤LLM输出的机制

1. **契约验证**: 通过前置/后置条件确保函数行为符合规格
2. **不变量检查**: 确保状态转换保持关键属性
3. **反例驱动**: 验证失败提供具体输入指导LLM修复
4. **一致性检查**: Clover六步流程捕获规格-实现-文档间的不一致

### 工具选择建议

| 场景 | 推荐工具 | 理由 |
|------|----------|------|
| 快速内存安全检查 | Kani | 自动化程度高，AWS生产验证 |
| 功能正确性证明 | Verus | Rust原生，规格用Rust编写 |
| 完整形式验证 | Dafny | 成熟稳定，AWS广泛使用 |
| 精化类型验证 | Flux | 轻量级，自动化推断 |

### 与状态空间的结合点

1. **形式规约作为状态约束**: 定义有效状态的数学边界
2. **验证作为准入测试**: 只有通过验证的代码才能进入状态空间
3. **不变量维护**: 验证确保状态转移保持约束
4. **反例指导探索**: 验证失败指导状态空间探索方向

---

## 参考资源

### 论文
- [Clover: Closed-Loop Verifiable Code Generation](https://theory.stanford.edu/~barrett/pubs/SSP+24.pdf)
- [Verus: Verifying Rust Programs using Linear Ghost Types](https://www.research-collection.ethz.ch/handle/20.500.11850/610518)
- [AutoVerus: Automated Proof Generation for Rust Code](https://www.microsoft.com/en-us/research/publication/autoverus-automated-proof-generation-for-rust-code/)
- [MATH-VF: Step-Wise Formal Verification for LLM-Based Mathematical Problem Solving](https://arxiv.org/html/2505.20869v1)

### 开源项目
- Dafny: https://github.com/dafny-lang/dafny
- Verus: https://github.com/verus-lang/verus
- Kani: https://github.com/model-checking/kani
- Creusot: https://github.com/xldenis/creusot

### 技术文档
- [Dafny参考手册](https://dafny.org/)
- [Verus教程](https://verus-lang.github.io/verus/guide/)
- [Kani文档](https://model-checking.github.io/kani/)

---

*研究完成时间: 2026-03-11*
*总时长: ~28分钟*
