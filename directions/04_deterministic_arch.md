# 04_deterministic_arch

## 方向名称
确定性架构：Praetorian

## 核心问题
Thin Agent + Fat Platform 如何工作?

## 研究历程

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

### 2026-03-10 深度研究完成
- **Web Research**: 完成Praetorian Gateway模式、确定性AI编排、状态机执行研究
- **关键发现**:
  1. Praetorian平台使用39+专业化Agent，通过16阶段状态机协调
  2. Gateway模式实现"Intent-Based Context Loading"，避免加载整个领域知识库
  3. Thin Agent严格<150行，Execution Cost从~24,000 tokens降至~2,700 tokens
  4. 8层防御深度(Layer 1-8)确保约束执行，即使Agent试图绕过
- **假设验证**:
  - H1 (Gateway隔离非确定性): **已验证** - Gateway通过确定性规则匹配路由请求
  - H2 (16阶段覆盖执行路径): **已验证** - 状态机支持智能阶段跳过(BugFix跳过5个阶段)
  - H3 (Thin Agent足够表达复杂逻辑): **已验证** - <150行Agent通过Gateway调用Fat Platform能力
- **代码实现**: `drafts/20260310_1526_deterministic_arch.rs`
  - Gateway路由器实现
  - 16阶段状态机
  - Thin Agent框架(<150行约束)
  - Fat Platform接口

## 关键资源

### 论文/博客
- **Praetorian: Deterministic AI Orchestration** (2025)
  - 来源: https://www.praetorian.com/blog/deterministic-ai-orchestration-a-platform-architecture-for-autonomous-development/
  - Thin Agent (<150行) + Fat Platform
  - Gateway模式动态路由技能
  - 确定性Hooks在LLM上下文外强制执行
  - "将AI转变为软件供应链的确定性组件"
  - 8层防御深度架构

- **OpenClaw + Lobster: Deterministic Multi-Agent Pipeline** (2026)
  - 来源: https://dev.to/ggondim/how-i-built-a-deterministic-multi-agent-dev-pipeline-inside-openclaw-and-contributed-a-missing-4ool
  - 状态机控制流程，非LLM决定下一步
  - 4 projects x 3 roles = 12并发Agent会话
  - 子工作流循环支持(code->review最多3次迭代)

- **Orchestrating AI Agents in Production** (2026)
  - 来源: https://hatchworks.com/blog/ai-agents/orchestrating-ai-agents/
  - 9种生产级编排模式
  - 确定性状态机编排(混合方法)
  - Supervisor + Specialists模式

### 开源项目
- **OpenClaw**: 多Agent平台，支持39+ Agent并发
- **Lobster**: OpenClaw的工作流引擎，支持子工作流循环

### 技术博客
- 待补充...

## 架构洞察

### Praetorian 核心机制
1. **Thin Agent** —— 极简Agent逻辑（<150行），专注于意图识别
2. **Fat Platform** —— 丰富的确定性运行时，包含所有业务逻辑
3. **Gateway模式** —— 动态路由技能请求到确定性处理模块
4. **确定性Hooks** —— 在LLM上下文外强制执行约束

### 8层防御深度 (Defense in Depth)
| Layer | 机制 | 作用 |
|-------|------|------|
| L1 | CLAUDE.md | 会话启动时加载完整规则集 |
| L2 | Skills | 按需调用的程序工作流 |
| L3 | Agent定义 | 角色特定行为、强制技能列表 |
| L4 | UserPromptSubmit Hooks | 每次提示注入提醒 |
| L5 | PreToolUse Hooks | 动作前阻塞(Agent优先执行) |
| L6 | PostToolUse Hooks | 验证Agent工作输出位置 |
| L7 | SubagentStop Hooks | 阻止提前退出 |
| L8 | Stop Hooks | 质量门、迭代限制、反馈循环 |

### 16阶段标准编排模板
```
Phase 1: Setup          → Phase 9:  DesignVerification
Phase 2: Triage         → Phase 10: DomainCompliance
Phase 3: CodebaseDiscovery → Phase 11: CodeQuality
Phase 4: SkillDiscovery → Phase 12: TestPlanning
Phase 5: Complexity     → Phase 13: Testing
Phase 6: Brainstorming  → Phase 14: CoverageVerification
Phase 7: ArchitectingPlan → Phase 15: TestQuality
Phase 8: Implementation → Phase 16: Completion
```

### 智能阶段跳过
| 工作类型 | 跳过的阶段 | 剩余阶段数 |
|----------|-----------|-----------|
| BUGFIX | 5,6,7,9,12 | 11 |
| SMALL | 5,6,7,9 | 12 |
| MEDIUM | 无 | 16 |
| LARGE | 无(更严格) | 16 |

### 与状态空间的结合点
- Fat Platform 就是状态空间的物理实现
- Gateway 作为状态空间的入口守卫
- Thin Agent 在状态空间内"导航"，而非"生成"
- 16阶段状态机定义状态空间的遍历路径
- Hooks在状态空间边界强制执行约束

## 待验证假设
- [x] H1: Gateway模式可有效隔离LLM非确定性
- [x] H2: 16阶段状态机覆盖所有执行路径
- [x] H3: Thin Agent (<150行)足够表达复杂逻辑

## 下一步研究方向
1. **Hook实现细节**: 研究PreToolUse/PostToolUse Hooks的具体实现机制
2. **Context Compaction**: 上下文压缩算法的实现细节
3. **Parallel Agent Dispatch**: 并行Agent调度的冲突解决机制
4. **Self-Annealing**: 平台自我修复能力的实现
5. **Heterogeneous LLM Routing**: 多模型路由决策机制
