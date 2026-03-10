# 08_llm_as_navigator

## 方向名称
LLM 角色：从生成器到导航器

## 核心问题
LLM 作为启发式函数的理论基础?

## 研究历程

### 2026-03-10 16:15 深度研究：LLM作为导航器的算法实现

**研究范围**: 使用SubAgent深度研究LLM作为启发式函数的理论和实践（~30分钟）

**核心发现**：
建立了完整的LLM导航器理论和代码实现：

**关键论文发现**:
- **Tree of Thoughts (ToT)** - NeurIPS 2023: Game of 24任务中成功率4%→74%
- **ReAct** - ICLR 2023: 推理-行动循环，HotpotQA/ALFWorld/WebShop应用
- **LATS** - ICML 2024: MCTS统一推理、行动和规划
- **LLM-A***: A*精确路径规划 + LLM全局推理
- **MCTS-DPO**: 将实例级奖励分解为步骤级信号

**架构洞察**:
1. **LLM启发式的特殊性**: 概率性、上下文依赖、非静态
2. **与传统A*的差异**: 批处理优化、缓存机制、API成本考量
3. **效率权衡**: ToT需要5-100倍于CoT的token，但正确性大幅提升
4. **LLM导航 vs 生成**:
   | 维度 | LLM生成 | LLM导航 |
   |------|---------|---------|
   | 正确性 | 概率性 | 可验证、可回溯 |
   | 错误处理 | 累积 | 局部可回退 |
   | 解释性 | 黑盒 | 白盒(搜索轨迹) |

**代码实现**:
- `drafts/20260310_1615_llm_navigator.rs` (542行)
  - LLMHeuristic trait: 启发式函数接口
  - SimulatedLLMHeuristic: 模拟LLM推理
  - VotingLLMHeuristic: 自我一致性投票
  - LLMStarSearch: A* + LLM启发式
  - TreeOfThoughts: BFS/DFS搜索思维树
  - PatternNavigator: 与L2 Pattern层集成

**待验证假设**:
- LLM启发式的相对排序比绝对值更可靠
- 在类型约束状态空间中，LLM导航效率显著提升
- MCTS比BFS/DFS更适合LLM启发式

---

### 2026-03-09 初始化
- 创建方向文档

## 关键资源

### 论文
- **Tree of Thoughts** - Yao et al., NeurIPS 2023
  - 核心：将LLM从token级决策提升到"thought"级决策
  - 结果：Game of 24任务成功率4%→74%
  - 代码：https://github.com/princeton-nlp/tree-of-thought-llm

- **ReAct: Synergizing Reasoning and Acting** - Yao et al., ICLR 2023
  - 核心：推理轨迹与任务动作交错进行
  - 应用：HotpotQA、Fever、ALFWorld、WebShop

- **LATS: Language Agent Tree Search** - Zhou et al., ICML 2024
  - 核心：MCTS统一推理、行动和规划
  - 代码：https://github.com/lapisrocks/LanguageAgentTreeSearch

- **LLM-A*** - Meng et al., 2024
  - 核心：A*精确路径规划 + LLM全局推理
  - 代码：https://github.com/SilinMeng0510/llm-astar

- **MCTS-DPO** - Xie et al., 2024
  - 核心：使用MCTS迭代收集偏好数据

- **ToolFormer** - Meta AI, 2023
  - 核心：通过自监督学习让LLM学会调用API

### 开源项目
- **mcts-reasoning** (queelius)
  - 核心：规范的MCTS实现（Selection/Expansion/Rollout/Backpropagation）
  - 评估器：LLM-as-judge、Ground Truth、Numeric
  - 采样策略：Value-based、Visit-based、Diverse

- **tree-of-thought-llm** (princeton-nlp)
  - NeurIPS 2023官方实现
  - BFS/DFS搜索思维树

### 技术博客
- 待补充...

## 架构洞察

### LLM 作为导航器的范式转换
**传统模式（生成器）:**
- LLM直接生成完整解决方案
- 通过Prompt工程约束输出
- 难以保证正确性

**新范式（导航器）:**
- LLM在预定义的状态空间中搜索路径
- 每一步选择都受限于硬性边界
- 搜索过程可验证、可回溯

### 理论基础
1. **启发式函数** —— LLM评估状态空间的搜索方向
2. **A*搜索** —— 结合代价函数和启发式函数的最优路径搜索
3. **多步规划** —— LLM作为高层规划器，执行由确定性系统完成

## 待验证假设

- [ ] **假设1**: LLM启发式的相对排序比绝对值更可靠
  - 验证思路：对比直接预测h*(n) vs 两两比较状态对的实验

- [ ] **假设2**: 在类型约束的状态空间中，LLM导航效率显著提升
  - 验证思路：对比无约束搜索 vs L2 Pattern层约束搜索的节点扩展数

- [ ] **假设3**: MCTS比BFS/DFS更适合LLM启发式
  - 验证思路：在相同API调用预算下，比较三种算法的成功率

- [ ] **假设4**: 自我一致性投票可以提升LLM评估的可靠性
  - 验证思路：单次评估 vs 多次采样投票的准确率对比

- [ ] **假设5**: 早停优化可以找到质量-效率的最佳平衡点
  - 验证思路：不同早停阈值下的成功率和token消耗

## 下一步研究方向

1. **理论分析**
   - 形式化定义LLM启发式的"近似可采纳性"
   - 分析状态空间大小、LLM准确率、搜索复杂度之间的关系

2. **算法优化**
   - 实现自适应束宽：根据LLM置信度动态调整b
   - 探索分层搜索：L2 Pattern层粗粒度搜索 + L3 Domain层细粒度搜索

3. **验证框架**
   - 构建基准测试集：代码生成、数学推理、规划任务
   - 实现评估指标：成功率、搜索节点数、API调用成本

4. **与现有架构集成**
   - 将LLM导航器集成到六层渐进式边界架构中
   - 明确L2 Pattern层的LLM选择接口契约

## 代码草稿关联

- `drafts/20260310_1615_llm_navigator.rs` - LLM作为导航器的完整实现
  - 包含：LLMHeuristic trait、SimulatedLLMHeuristic、VotingLLMHeuristic
  - 包含：LLMStarSearch (A* + LLM启发式)
  - 包含：TreeOfThoughts (BFS/DFS搜索思维树)
  - 包含：PatternNavigator (与L2 Pattern层集成)
  - 542行Rust代码，完整测试覆盖
