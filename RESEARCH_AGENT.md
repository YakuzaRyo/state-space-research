# 状态空间架构研究 Agent 任务说明

## 任务概述
此文件用于 OpenClaw cron 定时触发研究任务。

## 执行指令
每次执行时，**研究时间必须 ≥ 25分钟**，请按照以下步骤进行深度研究：

### 1. 确定研究方向
根据当前小时（24小时制），选择对应的研究方向：

| 时间段 | 研究方向 | 核心问题 | 对应文档 |
|--------|---------|---------|---------|
| 00:00, 12:00 | 核心原则 | 如何让错误在设计上不可能发生? | directions/01_core_principles.md |
| 02:00, 14:00 | 分层设计 | Syntax→Semantic→Pattern→Domain如何转换? | directions/07_layered_design.md |
| 04:00, 16:00 | LLM导航器 | LLM作为启发式函数的理论基础? | directions/08_llm_as_navigator.md |
| 06:00, 18:00 | 实现技术 | 如何用Rust类型系统实现状态空间? | directions/09_rust_type_system.md |
| 08:00, 20:00 | 工具设计 | 如何设计'无法产生错误'的工具? | directions/10_tool_design.md |
| 10:00, 22:00 | 对比分析 | Claude Code/OpenCode的根本缺陷是什么? | directions/11_comparison.md |

### 2. 执行研究步骤

#### 步骤1: 阅读现有研究
- 阅读对应的 `directions/*.md` 文件
- 阅读 `logs/RESEARCH_LOG.md` 了解历史进展
- 阅读 `logs/DIRECTIONS_DYNAMIC.md` 查看活跃方向

#### 步骤2: 深度研究（必须 ≥ 25分钟）
针对当前方向的核心问题，使用 **SubAgent** 进行深度研究：

**SubAgent 研究要求**:
1. **搜索相关学术论文**（Refine4LLM, XGrammar, Praetorian等）
   - 使用 WebSearch 搜索最新论文
   - 使用 WebFetch 深度阅读关键论文
2. **查阅开源项目实现**
   - 搜索 GitHub 上的相关实现
   - 分析代码架构和设计模式
3. **思考架构设计的优缺点**
   - 对比不同方案的权衡
   - 提炼关键洞察
4. **代码实现**
   - 在 `drafts/` 目录创建 Rust 代码草稿
   - 实现关键数据结构和算法

**时间分配建议**:
- 文献搜索和阅读: 8-10分钟
- 开源项目分析: 5-7分钟
- 架构思考: 5-7分钟
- 代码实现: 5-7分钟
- 文档整理: 2-4分钟

#### 步骤3: 更新研究文档
将研究成果更新到对应的 `directions/*.md` 文件：
- 添加新的研究历程记录
- 更新关键资源列表
- 补充架构洞察
- 提出待验证假设
- 建议下一步研究方向

#### 步骤4: 代码实现（可选）
如果有具体的实现想法，在 `drafts/` 目录创建Rust代码草稿：
- 文件命名: `YYYYMMDD_HHMM_方向名.rs`
- 包含详细注释
- 实现关键数据结构和算法

#### 步骤5: 更新动态方向
如果研究发现新的方向：
- 更新 `logs/DIRECTIONS_DYNAMIC.md`
- 添加到活跃方向池（最多5个）
- 标记优先级和预期产出

#### 步骤6: 使用 SubAgent 执行深度研究
**重要**: 使用 `Agent` 工具创建专门的子代理来执行深度研究任务，以充分利用上下文空间。

```claude
Agent(
  subagent_type="Explore",
  prompt="""
  执行状态空间架构深度研究任务：

  研究方向: [当前方向名]
  核心问题: [方向的核心问题]
  对应文档: directions/XX_方向名.md

  任务要求：
  1. 深度阅读对应方向的文档和相关资料
  2. 搜索最新论文和开源项目（使用 WebSearch/WebFetch）
  3. 分析架构设计的优缺点
  4. 提炼关键洞察和待验证假设
  5. 如有具体实现想法，创建 Rust 代码草稿

  输出要求：
  - 研究摘要（研究方向、核心问题、调研结果、架构洞察）
  - 待验证假设列表
  - 代码草稿（如有）
  - 下一步研究方向建议
  """,
  description="深度研究: [方向名]"
)
```

#### 步骤7: 提交到GitHub (kimi-research分支)
```bash
# 确保在 kimi-research 分支
git checkout kimi-research

# 拉取最新变更（避免冲突）
git pull origin kimi-research

# 添加变更
git add .

# 提交（遵循规范格式）
git commit -m "research(HH:MM): [方向名] - 简要描述"

# 推送到远程 kimi-research 分支
git push origin kimi-research
```

### 3. 每日汇总（23:45执行）
如果是23:45的任务：
1. 汇总今天所有研究成果
2. 生成 `daily/YYYY-MM-DD.md` 日报
3. **推送到 kimi-research 分支**：
   ```bash
   git checkout kimi-research
   git pull origin kimi-research
   git add .
   git commit -m "daily(YYYY-MM-DD): 日报汇总 - 执行N次，研究M个方向"
   git push origin kimi-research
   ```
4. 同步到 stable 分支（归档）：
   ```bash
   git checkout stable
   git merge kimi-research -m "daily(YYYY-MM-DD): 日报归档"
   git tag "daily-YYYY-MM-DD"
   git push origin stable --tags
   git checkout kimi-research
   ```

## 研究方法论

### 核心原则
1. **硬性边界 vs 软约束**
   - ❌ 软约束: "请你不要修改这个文件"（AI可能不听）
   - ✅ 硬边界: API不提供修改该文件的能力（AI物理上做不到）

2. **工程指导原则**
   - 类型安全 —— 编译期排除无效状态
   - 边界约束 —— LLM只能操作受限API
   - 不变量维护 —— 确定性系统强制执行
   - 失败快速 —— 无效操作在入口被拒绝

3. **SO(3)类比的正确理解**
   - SO(3)只是帮助理解的比喻，不是工程目标
   - 不要用群论/范畴论实现代码状态空间
   - 不要追求数学优雅而增加不必要的复杂度

### 研究产出要求
每次研究必须产出：
1. **研究方向**: 本次聚焦的具体方向
2. **核心问题**: 需要回答的关键问题
3. **调研结果**: 搜索、阅读、思考的汇总
4. **架构洞察**: 对状态空间架构的新理解
5. **待验证假设**: 下一步需要验证的想法
6. **代码片段**: 如有，相关的Rust实现草稿

## 关键资源

### 已知重要论文
1. **Refine4LLM (POPL 2025)** - 程序精化约束LLM生成
2. **XGrammar (陈天奇团队)** - token级别结构化生成
3. **Type-Constrained Code Generation (ICLR 2025)** - 类型约束解码
4. **Praetorian** - 确定性AI编排

### 开源项目
- Verus - Rust形式验证
- hacspec - 可执行规约
- TypeSec - 类型安全工具

## 研究时间表

### 日间 & 夜间统一
- **间隔**: 30分钟
- **研究时间**: 9:00, 9:30, 10:00, 10:30, ..., 23:30
- **每日次数**: 30次

## 效率积分规则
- **总间隔时间**: 30分钟
- **研究时间 < 25分钟** → -1分（研究不充分）
- **研究时间 ≥ 25分钟** → +1分（研究深入）
- **研究时间 ≥ 28分钟** → +2分（研究非常深入）

⚠️ **强制要求**: 每次研究必须持续 **至少25分钟**，以确保深度和质量。

- 积分记录在 `AGENT_SCORE.md` 文件中
- 每次研究完成后自动更新积分

---
*最后更新: 2026-03-10*
