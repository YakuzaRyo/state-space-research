# 12_engineering_roadmap

## 方向名称
工程路径：从理论到实现

## 核心问题
如何构建可落地的状态空间 Agent?

## 研究历程

### 2026-03-10 深度研究
- **研究时长**: 30+分钟
- **核心发现**:
  - Praetorian Gateway模式：确定性运行时包装非确定性LLM内核
  - XGrammar编译器架构：GrammarCompiler + Token Mask Cache + Persistent Stack
  - Claude Code架构：单线程主循环 + 确定性控制层（Hooks）
  - Rust Actor模型 + 状态机：Polar Signals的确定性模拟测试模式

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

## 关键资源

### 工程方法论
- **渐进式迁移策略**: 约束层 → 验证层 → 导航层 → 优化层
- **分层架构设计**: 状态机层 + Actor层 + LLM集成层 + 缓存层
- **工具链设计模式**: Builder模式 + 断路器 + 语义缓存

### 开源项目参考

| 项目 | 核心贡献 | 参考价值 |
|------|----------|----------|
| **XGrammar** (CMU) | 100x加速的结构化生成，Token Mask Cache | 编译器架构、缓存策略 |
| **Praetorian Gateway** | 确定性AI编排平台，16阶段状态机 | 状态空间管理、Gateway模式 |
| **Claude Code** | 单线程主循环，Hooks确定性控制层 | 控制流设计、Agent循环 |
| **Outlines** | FSM-based结构化输出 | 状态机约束生成 |
| **Pre³** (ACL 2025) | Pushdown Automata，20-30%更快 | 复杂语法处理 |
| **LangGraph** | 图状态机，持久化检查点 | 生产级状态管理 |
| **Rig** (ARC) | Rust AI Agent框架 | Rust实现参考 |
| **Polar Signals** | 状态机Actor + DST测试 | 架构模式、测试策略 |

### 技术博客与论文
- [Deterministic AI Orchestration](https://www.praetorian.com/blog/deterministic-ai-orchestration-a-platform-architecture-for-autonomous-development/) - Praetorian架构详解
- [XGrammar Paper](https://arxiv.org/pdf/2411.15100v1) - 结构化生成引擎
- [Claude Code Architecture](https://www.zenml.io/llmops-database/claude-code-agent-architecture-single-threaded-master-loop-for-autonomous-coding) - 单线程主循环设计
- [DST in Rust](https://www.polarsignals.com/blog/posts/2025/07/08/dst-rust) - 确定性模拟测试
- [Structured Decoding Guide](https://aarnphm.xyz/posts/structured-decoding) - 结构化解码技术栈

## 架构洞察

### 从理论到实现的工程路径

#### Phase 1: 核心状态机（MVP）
**目标**: 建立可运行的状态空间基础

**关键组件**:
- `State` / `StateType` / `StateId` - 状态定义
- `StateMachine` - 状态转换引擎
- `Transition` / `TransitionTrigger` - 转换规则
- 状态历史与回滚机制

**设计决策**:
- 使用Rust类型系统实现编译时状态约束
- 状态转换显式定义，拒绝非法转换
- 异步状态机支持并发状态查询

#### Phase 2: Actor系统层
**目标**: 实现隔离、容错、可测试的Agent架构

**关键模式**:
```
StateSpaceActor (消息处理)
    ↓
StateMachine (状态管理)
    ↓
StateValidator (验证规则)
```

**设计决策**:
- Actor模型实现消息传递隔离
- 每个连接/会话独立Actor
- 使用`tokio::sync::mpsc`实现mailbox模式

#### Phase 3: LLM集成层
**目标**: 可靠、高效、可观测的LLM调用

**关键组件**:
- `CircuitBreaker` - 故障隔离
- `SemanticCache` - 语义缓存（余弦相似度匹配）
- `LlmClient` trait - 多提供者抽象
- 结构化输出（JSON Schema约束）

**性能优化**:
- Prompt Caching: 90%成本降低，85%延迟降低
- Semantic Caching: 30%成本降低，15x响应加速
- 断路器模式防止级联故障

#### Phase 4: 可观测性层
**目标**: 生产级监控、调试、优化

**关键组件**:
- `TraceContext` - 分布式追踪
- `MetricsCollector` - 指标收集
- 结构化日志（tracing + OpenTelemetry）
- 状态历史可视化

### 技术栈选择建议

| 层级 | 推荐技术 | 备选方案 |
|------|----------|----------|
| **语言** | Rust | TypeScript (Node.js) |
| **运行时** | Tokio | async-std (已停止维护) |
| **序列化** | serde + schemars | prost (protobuf) |
| **LLM集成** | 原生HTTP + 结构化生成 | LiteLLM |
| **缓存** | Redis (语义缓存) | 内存LRU |
| **向量存储** | Qdrant / pgvector | Pinecone |
| **可观测性** | tracing + OpenTelemetry | Logfire |

### 关键工程决策与权衡

#### 1. 类型系统复杂度 vs 易用性
**选择**: 编译时类型安全 + 运行时灵活验证

- **类型安全**: 使用Rust类型系统防止非法状态转换
- **灵活验证**: 运行时`ValidationRule`支持动态规则
- **权衡**: 开发时严格，运行时灵活

#### 2. 性能优化策略
**延迟优化**:
- 语义缓存（相似查询直接返回）
- Prompt缓存（KV Cache复用）
- 预编译Grammar（XGrammar模式）

**吞吐量优化**:
- Actor并发模型
- 连接池管理
- 批处理请求

#### 3. 与现有工具链集成
**MCP (Model Context Protocol)**:
- Anthropic主导的行业标准
- 标准化工具/资源/提示接口
- 2025年已被OpenAI、Google采纳

**结构化生成**:
- XGrammar/Outlines集成
- JSON Schema约束输出
- 消除LLM输出不确定性

#### 4. 开发者体验设计
**API设计原则**:
- Builder模式配置（流畅接口）
- 类型化错误（thiserror）
- 异步优先（async/await）
- 完整文档 + doctests

**CLI设计**:
- 渐进式披露（简单→高级）
- 状态可视化
- 实时日志流

### 最小可行产品(MVP)设计

#### 核心组件优先级

**P0 - 必须有**:
1. 状态机核心（State/Transition/StateMachine）
2. 基本Actor系统（消息传递）
3. LLM客户端（带断路器）
4. 配置系统（Builder模式）

**P1 - 重要**:
1. 语义缓存
2. 结构化输出验证
3. 基础可观测性（日志/指标）
4. 内存管理（短期/长期记忆）

**P2 - 增强**:
1. 向量数据库存储
2. 分布式追踪
3. WebSocket实时通信
4. 高级验证规则

#### 迭代开发路线图

**Week 1-2: 核心状态机**
- 状态定义与类型系统
- 状态转换引擎
- 基础测试覆盖

**Week 3-4: Actor系统**
- Actor消息传递
- 状态机Actor封装
- 并发安全验证

**Week 5-6: LLM集成**
- 多提供者客户端
- 断路器实现
- 结构化输出

**Week 7-8: 缓存与优化**
- 语义缓存
- Prompt缓存
- 性能基准测试

**Week 9-10: 可观测性**
- 结构化日志
- 指标收集
- 追踪实现

**Week 11-12: 工具链与文档**
- CLI工具
- 完整文档
- 示例项目

### 实施路线图

```
Phase 1 (Month 1-2): 核心框架
├── 状态机引擎
├── Actor系统
└── 基础LLM集成

Phase 2 (Month 3-4): 生产就绪
├── 缓存系统
├── 可观测性
└── 错误处理/重试

Phase 3 (Month 5-6): 生态系统
├── MCP集成
├── 工具链CLI
└── 社区文档
```

## 待验证假设

- [x] Rust类型系统适合表达状态空间约束
- [x] Actor模型适合LLM Agent并发架构
- [x] 语义缓存可显著降低LLM调用成本
- [x] 结构化生成可消除输出不确定性
- [ ] 状态机方法相比ReAct的准确性优势
- [ ] 分层内存系统的实际效果

## 下一步研究方向

1. **形式验证集成**: 使用Kani/Dafny验证状态机正确性
2. **多Agent协调**: 基于MCP的Agent间通信
3. **自适应学习**: Agent从执行历史中学习优化策略
4. **边缘部署**: 嵌入式/边缘设备上的轻量级Agent

## 参考代码

完整MVP实现框架: `drafts/20260310_2200_engineering_roadmap.rs`

包含:
- 完整类型系统定义
- Actor系统实现
- 状态机引擎
- 断路器/缓存/验证
- Builder模式配置
- 测试示例
