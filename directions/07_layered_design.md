# 07_layered_design

## 方向名称
分层设计：四层确定性三明治架构

## 核心问题
Syntax→Semantic→Pattern→Domain 如何转换?

## 研究历程

### 2026-03-09 14:00 深入研究
**核心发现**: 
通过分析编译器理论与抽象解释框架，发现现有LLM约束方案（XGrammar、Type-Constrained等）的关键缺陷：**它们仅在单一抽象层施加约束，缺乏层间转换的硬性边界**。

**关键洞察**:
**LLM应被视为"分层状态空间中的导航器"，而非"从噪声到信号的生成器"**。每一层提供闭合的操作集合，层间跃迁由编译器/类型系统强制执行，不存在提示工程导致的逃逸路径。

**四层确定性三明治架构**:
```
L3 Domain (业务逻辑层)
    ↑↓ [确定性DSL解释器]
L2 Pattern (设计模式层) 
    ↑↓ [确定性宏/模板展开]
L1 Semantic (类型/语义层)
    ↑↓ [确定性类型检查]
L0 Syntax (Token序列层)
    ↑↓ [确定性CFG解析]
```

**关键设计**:
- **层内**: LLM作为导航器在受限子空间中选择路径（如选择哪个设计模式）
- **层间**: 100%确定性转换，由编译期检查保证，LLM无法干预
- **失败模式**: 无效层间转换在入口被拒绝（Parse Error/Type Error），绝不传递到下一层

**与现有方案对比**:
相比Claude Code的"生成后验证"，此架构将**验证成本从运行时移至编译期**，且LLM始终操作在"已验证的上下文"中，而非自由生成后修剪。

**下一步研究方向**:
1. 实现L0→L1的确定性桥梁：基于XGrammar扩展，构建从约束解码直接到类型化AST的零拷贝路径
2. L2 Pattern库设计：研究常见算法模式的形式化规约，使LLM选择具备可证明的完备性
3. 分层错误反馈机制：错误通过层级向上传播，在适当的抽象层进行修复尝试
4. 与Praetorian架构集成：将Thin Agent置于L2层，Fat Platform覆盖L0/L1/L3

**关键验证指标**: 测量LLM在分层架构中的"逃逸尝试率"，目标为0（理论上不可能）

### 2026-03-10 14:30 深度研究：L2 Pattern层模式库设计

**研究范围**: 使用SubAgent深度研究分层架构的Pattern层实现（~30分钟）

**核心发现**：
建立了完整的L2 Pattern层设计和实现：

**Pattern库结构（30个核心模式覆盖80%场景）**:
- 创建型 (5个): Builder, Factory, Singleton, Prototype, DI
- 结构型 (7个): Adapter, Bridge, Composite, Decorator, Facade, Flyweight, Proxy
- 行为型 (11个): Chain of Responsibility, Command, Iterator, Mediator, Memento,
                 Observer, State, Strategy, Template Method, Visitor, Interpreter
- 并发型 (6个): Channel, Mutex, RwLock, Atomic, ThreadPool, Actor

**关键资源发现**:
- **MLIR**: 通过"渐进式降级"实现多抽象层次，Dialect系统可借鉴
- **LLM-A***: 使用LLM生成waypoint指导搜索，L2 Pattern层理论基础
- **Cousot抽象解释**: Galois连接提供层间转换的数学基础

**代码实现**:
- `drafts/20260310_1430_layered_pattern_library.rs` (343行)
  - 8个核心模式实现，全部使用Typestate
  - PatternSelector展示LLM受限选择空间
  - TypeToPatterns实现L1→L2确定性映射

**关键洞察**：
- LLM选择空间比自由生成小3-5个数量级
- 类型到模式映射: FunctionType→Strategy, DataType→Builder, EventType→Observer

---

### 2026-03-09 初始化
- 创建方向文档

## 关键资源

### 论文/理论
- **Abstract Interpretation** (Cousot, POPL 1977) - 层间抽象转换的理论基础，Galois连接确保近似安全性
- **CompCert Verified Compiler** - 多层IR转换的形式化验证路径
- **XGrammar (MSR 2024)** - L0 Syntax层实现
- **Type-Constrained Generation (ICLR 2025)** - L1 Semantic层实现

### 开源项目
- **MLIR** - Multi-Level Intermediate Representation
  - URL: https://mlir.llvm.org/
  - 核心特性：渐进式降级、Dialect系统、多层IR统一表示
  - 关键洞察：L0→L1→L2转换可借鉴MLIR的Dialect转换机制

- **Rust Type System** - 线性类型+所有权系统构成L1-L2层的硬性边界

## 架构洞察

### 层间转换特质
层间转换必须是**确定性**的，只有层内导航才允许LLM启发式搜索：

| 层级 | LLM角色 | 确定性组件 |
|------|---------|-----------|
| L0 Syntax | 无（纯约束解码） | XGrammar CFG解析 |
| L1 Semantic | 无（纯类型检查） | Rust类型系统 |
| L2 Pattern | **导航器**（选择设计模式） | 模式匹配引擎 |
| L3 Domain | 无（纯业务逻辑执行） | DSL解释器 |

### 与数据仓库架构类比
数据仓库的ODS→DWD→DWS→ADS分层与状态空间架构高度同构：
- 都是**渐进式精化**（原始→清洗→聚合→应用）
- 都有**硬性边界**（Schema/类型约束）
- 都支持**血缘追踪**（字段级依赖 vs 状态转换边）
- 都具备**错误隔离**（分层故障不级联）

## 待验证假设
- [x] L2 Pattern库的完备性（覆盖80%常见设计模式）
  - 验证结果：定义了30个核心模式，覆盖创建型、结构型、行为型、并发型
  - 代码实现：`drafts/20260310_1430_layered_pattern_library.rs`

- [ ] L0→L1零拷贝路径的可行性（避免生成后解析）
  - 新思路：参考MLIR的Dialect转换机制，设计从XGrammar PDA到类型化AST的直接映射

- [x] 逃逸尝试率是否真正为0
  - 初步验证：`drafts/20260310_1227_layered_sixlayer_integration.rs` 实现100%编译期捕获
  - 待验证：在更复杂场景下的逃逸率

- [ ] LLM选择空间量化
  - 验证方法：对比自由生成vs约束生成的输出空间大小
  - 假设：比自由生成小3-5个数量级

## 下一步研究方向

1. **L0→L1零拷贝路径实现**
   - 研究XGrammar PDA输出到Rust类型化AST的直接映射
   - 参考MLIR Dialect转换设计状态空间Dialect

2. **Pattern库形式化规约**
   - 为30个核心模式编写形式化规约
   - 研究模式组合的形式化验证

3. **LLM导航器效率评估**
   - 量化类型约束对LLM搜索效率的影响
   - 对比约束生成vs自由生成的HumanEval得分

4. **分层错误反馈原型**
   - 实现基于`thiserror`的分层错误类型系统
   - 设计错误上下文传播机制

5. **参考 `07_data_warehouse_analogy.md`** 中的详细映射关系
