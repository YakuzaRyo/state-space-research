# State Space Architecture Research

[![Research Status](https://img.shields.io/badge/research-active-brightgreen)](https://github.com/YakuzaRyo/state-space-research/commits/master)
[![Last Updated](https://img.shields.io/badge/last%20updated-2026--03--09-blue)](https://github.com/YakuzaRyo/state-space-research/releases)
[![Daily Reports](https://img.shields.io/badge/daily%20reports-13%2Fday-orange)](./daily/)

> 探索状态空间架构(State Space Architecture)作为下一代 AI 工程系统的理论基础与实现路径

## 研究愿景

**核心命题**: 如何将 LLM 的生成能力嵌入到一个逻辑上严格的状态空间中，让 LLM 在这个闭合空间里导航，而不是试图通过 Prompt 去纠正它跳出空间的行为？

**SO(3) 类比**: 就像旋转群中的运算结果必然在群内，AI 的所有操作也必然在预定义的硬性边界内，不存在"逃逸"的可能。所有输入输出在这个边界内的状态空间中都是确定的。

## 研究架构

```
┌─────────────────────────────────────────────────────────────┐
│                    状态空间架构 (State Space)                  │
├─────────────────────────────────────────────────────────────┤
│  Layer 4 │ Domain Space   │ 业务逻辑层 - 领域建模              │
│  Layer 3 │ Pattern Space  │ 设计模式层 - 组合规则              │
│  Layer 2 │ Semantic Space │ 语义层 - 类型系统                  │
│  Layer 1 │ Syntax Space   │ 语法层 - 约束机制                  │
├─────────────────────────────────────────────────────────────┤
│  LLM as Navigator │ 从"生成器"到"导航器"的范式转换        │
├─────────────────────────────────────────────────────────────┤
│  Hard Boundaries  │ 硬性边界 - 物理约束，非 Prompt 约束     │
└─────────────────────────────────────────────────────────────┘
```

## 目录结构

```
state-space-research/
├── 📁 daily/                    # 每日研究汇总 (由 23:45 任务生成)
│   └── YYYY-MM-DD.md
├── 📁 directions/               # 方向深度研究报告
│   ├── 01_core_principles.md   # 核心原则：状态空间设计
│   ├── 02_refinement_calculus.md   # Refine4LLM 精化演算
│   ├── 03_structured_generation.md # XGrammar 结构化生成
│   ├── 04_deterministic_arch.md    # Praetorian 确定性架构
│   ├── 05_type_constraints.md      # Type-Constrained Generation
│   ├── 06_formal_verification.md   # Clover/Dafny 验证集成
│   ├── 07_layered_design.md        # 分层状态空间设计
│   ├── 08_llm_as_navigator.md      # LLM 作为导航器
│   ├── 09_rust_type_system.md      # Rust 类型系统实现
│   ├── 10_tool_design.md           # 无缺陷工具集设计
│   ├── 11_comparison.md            # 对比分析现有架构
│   └── 12_engineering_roadmap.md   # 工程路径与落地策略
├── 📁 drafts/                   # Rust 代码草稿
│   └── YYYYMMDD_HHMM_方向.rs
├── 📁 logs/                     # 日志与动态方向管理
│   ├── DIRECTIONS_DYNAMIC.md   # 活跃方向池 (动态更新)
│   └── RESEARCH_LOG.md         # 研究日志汇总
├── 📄 RESEARCH_PLAN.md          # 研究计划
└── 📄 README.md                 # 本文件
```

## 活跃研究方向

查看 [DIRECTIONS_DYNAMIC.md](./logs/DIRECTIONS_DYNAMIC.md) 了解当前活跃方向池。

**基础方向轮询表** (12个核心领域):

| 时间 | 方向 | 核心问题 |
|------|------|----------|
| 00:00 | 核心原则 | 如何让错误在设计上不可能发生? |
| 02:00 | Refine4LLM | 程序精化如何约束 LLM 生成? |
| 04:00 | XGrammar | 如何在 token 级别约束 LLM 输出? |
| 06:00 | Praetorian | Thin Agent + Fat Platform 如何工作? |
| 08:00 | Type-Constrained | 类型系统如何指导代码生成? |
| 10:00 | Clover/Dafny | 形式验证如何过滤 LLM 输出? |
| 12:00 | 分层设计 | Syntax→Semantic→Pattern→Domain 如何转换? |
| 14:00 | LLM 导航器 | LLM 作为启发式函数的理论基础? |
| 16:00 | Rust 类型系统 | 如何用 Rust 类型系统实现状态空间? |
| 18:00 | 工具设计 | 如何设计'无法产生错误'的工具? |
| 20:00 | 对比分析 | Claude Code/OpenCode 的根本缺陷是什么? |
| 22:00 | 工程路径 | 如何构建可落地的状态空间 Agent? |

## 研究方法论

### 工程指导原则

**硬性边界的核心含义**:
- ❌ 软约束: "请你不要修改这个文件" (AI 可能不听)
- ✅ 硬边界: API 不提供修改该文件的能力 (AI 物理上做不到)

**实现策略**:
1. **类型安全** —— 编译期排除无效状态
2. **边界约束** —— LLM 只能操作受限 API
3. **不变量维护** —— 确定性系统强制执行
4. **失败快速** —— 无效操作在入口被拒绝

**重要澄清**: SO(3) 只是帮助理解的比喻，不是工程目标。不要用群论/范畴论实现代码状态空间，不要追求数学优雅而增加不必要的复杂度。

### 研究执行节奏

**每日研究时间表**:
- 🌅 上午段: 9:45, 10:45, 11:45
- 🌞 下午/晚上段: 14:45, 15:45, 16:45, 17:45, 18:45, 19:45, 20:45, 21:45, 22:45, 23:45
- **总计**: 每天 13 次研究执行

**产出节奏**:
- 9:45-22:45: 常规研究 → 更新方向报告 → 提交到 master
- **23:45**: 汇总日报 → 同步到 stable → 打标签发布

## Git 工作流

### 分支说明

| 分支 | 用途 | 更新频率 |
|------|------|----------|
| `master` | 开发分支，包含所有日常研究更新 | 每次执行后推送 |
| `stable` | 稳定分支，只包含归档的完整报告 | 每日 23:45 同步 |

### 提交规范

```
research(HH:MM): [方向名] - 简要描述
update: [描述]
daily(YYYY-MM-DD): 日报汇总 - 执行N次，研究M个方向
stable(YYYY-MM-DD): 日报归档
```

### 标签管理

- `daily-YYYY-MM-DD`: 每日报告快照
- 通过 [Releases](https://github.com/YakuzaRyo/state-space-research/releases) 查看历史版本

## 关键洞察 (来自已有研究)

1. **Refine4LLM (POPL 2025)**
   - 形式化规约驱动，非自然语言
   - 精化法则库预定义，LLM 从中选择
   - ATP 验证保证正确性
   - 实验: 精化步骤减少 74%，通过率提升至 82%

2. **Praetorian 确定性 AI 编排**
   - Thin Agent (<150行) + Fat Platform
   - Gateway 模式动态路由技能
   - 确定性 Hooks 在 LLM 上下文外强制执行

3. **XGrammar (陈天奇团队)**
   - 字节级 PDA 处理不规则 token 边界
   - 自适应掩码缓存，比现有方案快 100 倍
   - 端到端接近零开销

4. **Type-Constrained Code Generation (ICLR 2025)**
   - 类型系统作为"正确性空间"定义
   - 前缀自动机实现类型约束解码
   - HumanEval 编译错误减少一半以上

## 如何阅读本仓库

1. **每日概览**: 查看 [daily/](./daily/) 目录下的日报文件
2. **深度研究**: 查看 [directions/](./directions/) 目录下各方向的详细分析
3. **代码实现**: 查看 [drafts/](./drafts/) 目录下的 Rust 代码草稿
4. **动态追踪**: 查看 [DIRECTIONS_DYNAMIC.md](./logs/DIRECTIONS_DYNAMIC.md) 了解活跃方向

## 关联资源

- **GitHub 仓库**: https://github.com/YakuzaRyo/state-space-research
- **研究报告**: 通过 [Releases](https://github.com/YakuzaRyo/state-space-research/releases) 查看每日归档

## 贡献者

- [@YakuzaRyo](https://github.com/YakuzaRyo) - 研究主导

---

*最后更新: 2026-03-09 | 研究状态: 活跃进行中*
