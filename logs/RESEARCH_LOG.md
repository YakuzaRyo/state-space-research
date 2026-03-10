# 状态空间架构研究日志

> 自动生成于每次定时任务执行
>  cron 表达式: 0 */2 * * * (每2小时)
>  **重要: 研究计划会根据每次的"下一步研究方向建议"动态更新**

---

## 研究档案

- **任务ID**: `state-space-research` 
- **执行频率**: 每2小时 (每天12次)
- **思考级别**: high
- **单次超时**: 1800秒 (30分钟)
- **会话类型**: isolated
- **通知渠道**: feishu
- **动态更新**: ✅ 根据研究成果自动调整后续方向

---

## 研究方向体系

### 基础方向 (固定，每天轮询)

| 时间 | 方向 | 核心问题 |
|------|------|----------|
| 00:00 | **核心原则: 状态空间设计** | 如何让错误在设计上不可能发生? |
| 02:00 | **形式化方法: Refine4LLM** | 程序精化如何约束LLM生成? |
| 04:00 | **结构化生成: XGrammar** | 如何在token级别约束LLM输出? |
| 06:00 | **确定性架构: Praetorian** | Thin Agent + Fat Platform如何工作? |
| 08:00 | **类型约束: Type-Constrained** | 类型系统如何指导代码生成? |
| 10:00 | **验证集成: Clover/Dafny** | 形式验证如何过滤LLM输出? |
| 12:00 | **分层设计: 多层空间投影** | Syntax→Semantic→Pattern→Domain如何转换? |
| 14:00 | **LLM角色: 从生成器到导航器** | LLM作为启发式函数的理论基础? |
| 16:00 | **实现技术: Rust类型系统** | 如何用Rust类型系统实现状态空间? |
| 18:00 | **工具设计: 无缺陷工具集** | 如何设计'无法产生错误'的工具? |
| 20:00 | **对比分析: vs现有架构** | Claude Code/OpenCode的根本缺陷是什么? |
| 22:00 | **工程路径: 从理论到实现** | 如何构建可落地的状态空间Agent? |

### 动态方向 (根据研究成果自动添加)

见 `DIRECTIONS_DYNAMIC.md` 文件，记录每次研究提出的新方向。

---

## 重要澄清：SO(3) 只是比喻

**Black Jack 明确：SO(3) 类比只是帮助理解的比喻，不是工程目标。**

**SO(3)类比的核心含义：**
> 就像旋转群中的运算结果必然在群内，AI的所有操作也必然在预定义的**硬性边界**内，不存在"逃逸"的可能。所有输入输出在这个边界内的状态空间中都是**确定的**。

| 类比的本意 | 不要过度工程化 |
|-----------|--------------|
| SO(3)群运算：结果必然在群内，无需检查 | ✅ 状态空间设计：AI操作必然在边界内 |
| 欧拉角：可能产生无效状态，需要规则避免 | ❌ Prompt约束：试图用规则纠正AI行为 |
| **硬性边界 = AI无法逃脱的物理限制** | ❌ 不要用群论实现，要用工程约束 |

**工程指导原则（如何实现"硬性边界"）：**
1. **类型安全** —— 编译期排除无效状态（无效状态在类型层面不可能构造）
2. **边界约束** —— LLM只能操作受限API（物理上接触不到危险操作）
3. **不变量维护** —— 确定性系统强制执行（不由AI"理解"或"遵守"）
4. **失败快速** —— 无效操作在入口被拒绝（不产生中间错误状态）

**关键区分：**
- ❌ 软约束："请你不要修改这个文件"（AI可能不听）
- ✅ 硬边界：API不提供修改该文件的能力（AI物理上做不到）

---

## 动态更新机制

### 如何工作

1. **每次研究完成后**：
   - 分析"下一步研究方向建议"
   - 将新方向追加到 `DIRECTIONS_DYNAMIC.md`
   - 新方向将整合进第二天的研究轮询

2. **方向优先级**：
   - 基础方向每天必执行（确保覆盖12个核心领域）
   - 动态方向根据重要性插入轮询
   - 重复或已解决的方向会被归档

3. **研究演进**：
   - 第1天：基础调研，发现新方向A、B
   - 第2天：执行方向A，发现新方向C
   - 第3天：执行方向C，深入具体实现...

### 方向记录格式

```markdown
## 动态研究方向记录

### 2026-03-08 方向12: 工程路径
**来源**: 方向12研究提出的建议
**新方向**:
1. XGrammar→Rust类型编译器 (优先级: 高)
   - 问题: 自动将JSON Schema转换为TypedState约束
   - 预期产出: 一个代码生成工具原型

2. Refine4LLM的Rust运行时 (优先级: 中)
   - 问题: 将精化演算整合进状态空间Agent
   - 预期产出: 核心数据结构定义
```

---

## 关键洞察 (来自已有研究)

1. **Refine4LLM (POPL 2025)**
   - 形式化规约驱动，非自然语言
   - 精化法则库预定义，LLM从中选择
   - ATP验证保证正确性
   - 实验：精化步骤减少74%，通过率提升至82%

2. **Praetorian 确定性AI编排**
   - Thin Agent (<150行) + Fat Platform
   - Gateway模式动态路由技能
   - 确定性Hooks在LLM上下文外强制执行
   - "将AI转变为软件供应链的确定性组件"

3. **XGrammar (陈天奇团队)**
   - 字节级PDA处理不规则token边界
   - 自适应掩码缓存，比现有方案快100倍
   - 端到端接近零开销

4. **Type-Constrained Code Generation (ICLR 2025)**
   - 类型系统作为"正确性空间"定义
   - 前缀自动机实现类型约束解码
   - HumanEval编译错误减少一半以上

---

## 研究产出目录

```
/root/.openclaw/workspace/research/state-space-architecture/
├── RESEARCH_PLAN.md              # 研究计划
├── logs/
│   ├── RESEARCH_LOG.md           # 本文件：汇总日志
│   └── DIRECTIONS_DYNAMIC.md     # 动态研究方向记录
├── daily/
│   └── YYYY-MM-DD.md             # 每日研究摘要
├── directions/                   # 12个基础方向的深度分析
│   ├── 01_core_principles.md
│   ├── 02_refinement_calculus.md
│   ├── 03_structured_generation.md
│   ├── 04_deterministic_arch.md
│   ├── 05_type_constraints.md
│   ├── 06_formal_verification.md
│   ├── 07_layered_design.md
│   ├── 08_llm_as_navigator.md
│   ├── 09_rust_type_system.md
│   ├── 10_tool_design.md
│   ├── 11_comparison.md
│   └── 12_engineering_roadmap.md
└── drafts/
    └── *.rs                      # Rust 代码草稿
```

---

## 执行日志

### 初始化 (2026-03-08)
- [x] 创建研究框架
- [x] 设置定时任务 (cron: 0 */2 * * *)
- [x] 初始化日志文件
- [x] 整合 Black Jack 调研报告
- [x] 明确 SO(3) 只是比喻，不是工程目标
- [x] 设置每次研究完成后通过 feishu 通知用户
- [x] **设置动态更新机制：根据"下一步研究方向建议"自动演进研究计划**
- [ ] 等待首次执行...

### 2026-03-09
- [x] 首次执行12个基础方向轮询
- [x] 根据研究成果生成 DIRECTIONS_DYNAMIC.md
- [x] 执行新提出的动态方向

### 2026-03-10

#### 20:54 工具设计 - Claude Code权限系统深入分析
- [x] 研究方向: 工具设计 - Claude Code权限系统深入分析
- [x] 核心问题: Claude Code权限系统的工作原理是什么？与状态空间架构的本质区别？
- [x] 产出: 分析Claude Code官方权限文档 (docs.anthropic.com)
- [x] 产出: 深入分析三层权限类型+五种权限模式
- [x] 产出: 识别Claude Code权限系统的脆弱性（运行时检查可被绕过）
- [x] 产出: 提出"Permission as Configuration" vs "Permission as Type"范式对比
- [x] 产出: 更新 directions/10_tool_design.md (新增Claude Code深入分析)

#### 20:24 工具设计 - 权限层次与最小权限原则（学术调研）
- [x] 研究方向: 工具设计 - 权限层次与最小权限原则
- [x] 核心问题: 如何将工具权限约束从"软约束"升级为"硬边界"?
- [x] 调研文献: MiniScope (arXiv:2512.11147, cs.CR) - 最小权限框架
- [x] 调研文献: DRIFT (arXiv:2506.12104, NeurIPS 2025) - 动态规则隔离
- [x] 调研文献: VIGIL - 工具流注入防御的验证前提交方案
- [x] 调研文献: ATLASS (arXiv:2503.10071) - 动态工具生成框架
- [x] 核心洞察: 现有方案（MiniScope/DRIFT/VIGIL）均为软约束，状态空间为硬边界
- [x] 核心洞察: ToolToken<P>将权限层次编码进类型系统，错误权限编译期报错
- [x] 核心洞察: LLMDispatcher<P>是LLM与类型系统的接口层，持有令牌不暴露给LLM
- [x] 产出: 更新 directions/10_tool_design.md (添加学术对比分析)
- [x] 产出: drafts/20260310_2024_tool_permission_hierarchy.rs (权限层次原型)

#### 20:00 工具设计深入研究（与Claude Code对比）
- [x] 研究方向: 工具设计 - 无缺陷工具设计
- [x] 核心问题: 如何设计"无法产生错误"的工具? 与现有AI工具的本质区别
- [x] 产出: 分析Claude Code源码，研究其权限系统架构
- [x] 产出: 深入分析硬边界 vs 软约束的工程实现对比
- [x] 产出: 验证Kani Model Checker在工具验证中的价值
- [x] 产出: 更新 directions/10_tool_design.md (新增Claude Code对比分析)

#### 18:58 工具设计 + Rust类型系统 交叉研究
- [x] 研究方向: 实现技术 (Rust类型系统) + 工具设计 交叉
- [x] 核心问题: 如何用Rust类型系统实现"无法产生错误"的工具?
- [x] 产出: 更新 directions/09_rust_type_system.md (融入工具设计洞察)
- [x] 产出: 更新 directions/10_tool_design.md (添加Rust实现细节)
- [x] 产出: drafts/20260310_1310_tool_design.rs (完整工具设计原型)
- [x] 产出: drafts/20260310_1828_rust_kani_state.rs (多维状态空间+Kani验证)

#### 18:28 Rust类型系统深入研究
- [x] 研究方向: Rust类型系统实现状态空间
- [x] 核心问题: 如何用Rust类型系统实现状态空间?
- [x] 产出: 深入分析Kani Verifier、hacspec
- [x] 产出: 更新 directions/09_rust_type_system.md
- [x] 产出: drafts/20260310_1828_rust_kani_state.rs

#### 18:00 实现技术方向初始化
- [x] 研究方向: Rust类型系统实现状态空间
- [x] 产出: 初始化 directions/09_rust_type_system.md 研究历程

