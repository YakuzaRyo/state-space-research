# 10_tool_design

## 方向名称
工具设计：无缺陷工具集

## 核心问题
如何设计'无法产生错误'的工具?

## 研究历程

### 2026-03-10 20:30 深度研究：类型安全的CLI工具框架

**研究范围**: 使用SubAgent深度研究无缺陷工具设计（~30分钟）

**核心发现**：
建立了完整的类型安全CLI工具设计框架：

**关键资源**:
- **全函数式编程**: Agda依赖类型、Idris Totality Checking、Elm无运行时异常
- **确定性构建**: Nix（纯函数式包管理）、Bazel（Hermetic Builds）
- **Typestate模式**: Cliffle博客、Stanford CS242讲义、ZtM指南
- **FC-IS架构**: Functional Core, Imperative Shell模式、Stillwater库
- **Rust CLI最佳实践**: PeerDH错误处理、分层配置设计

**六层边界在CLI中的映射**:
| 层级 | CLI应用 | 实现 |
|------|---------|------|
| L0 | 参数数量、缓冲区大小约束 | Const Generics |
| L1 | 区分CLI/Env/File输入 | Newtype |
| L2 | 隐藏内部状态 | Opaque Types |
| L3 | 确保执行顺序 | Typestate (Parse→Validate→Execute) |
| L4 | 资源权限追踪 | Linear Types |
| L5 | IO权限控制 | Capability |

**代码实现**:
- `drafts/20260310_2030_tool_design.rs` (540行)
  - ConfigBuilder: L3 Typestate状态机
  - BoundedConfig: L0 Const Generics
  - CliInput/EnvInput/FileInput: L1 Newtype
  - SecureFileHandle: L4+L5权限系统
  - core/shell模块: Functional Core, Imperative Shell架构

**架构洞察**:
- 失败快速: 在ConfigBuilder阶段验证，而非运行时
- 渐进式披露: 分层配置（CLI > Env > File > Default）
- Effect trait: 抽象所有副作用，便于测试

---

### 2026-03-09 初始化
- 创建方向文档

## 关键资源

### 论文/文献
- **Total Functional Programming** - Agda依赖类型
- **Idris Totality Checking** - 全函数式编程
- **Elm Architecture** - 无运行时异常设计
- **Functional Core, Imperative Shell** - Gary Bernhardt

### 开源项目
- **Nix** - 纯函数式包管理器
  - URL: https://nixos.org/
  - 核心: 相同输入总是产生相同输出，加密哈希路径

- **Bazel** - 确定性构建系统
  - URL: https://bazel.build/
  - 核心: Hermetic Builds, 沙箱执行, 远程缓存

- **Stillwater** - Rust FC-IS实现
  - URL: https://entropicdrift.com/projects/stillwater/
  - 核心: Effect, Validation, IO类型

- **ripgrep/fd/bat** - Rust CLI优秀实践
  - 核心: 高性能、类型安全、良好错误处理

### 开源项目
- 待补充...

### 技术博客
- 待补充...

## 代码草稿关联

- `drafts/20260310_2030_tool_design.rs` - 类型安全的CLI工具框架完整实现
  - 包含: ConfigBuilder (L3 Typestate状态机)
  - 包含: BoundedConfig (L0 Const Generics)
  - 包含: CliInput/EnvInput/FileInput (L1 Newtype)
  - 包含: SecureFileHandle (L4+L5权限系统)
  - 包含: core/shell模块 (Functional Core, Imperative Shell)
  - 540行Rust代码，完整测试覆盖

## 架构洞察

### 无缺陷工具设计原则
1. **纯函数** —— 相同的输入总是产生相同的输出，无副作用
2. **副作用隔离** —— IO操作与业务逻辑分离
3. **不变量** —— 系统设计时就不允许非法状态存在
4. **失败快速** —— 错误在最早可能点被捕获

### 六层边界在CLI中的映射

| 层级 | CLI应用 | 实现 |
|------|---------|------|
| L0 | 参数数量、缓冲区大小约束 | Const Generics |
| L1 | 区分CLI/Env/File输入 | Newtype |
| L2 | 隐藏内部状态 | Opaque Types |
| L3 | 确保执行顺序 | Typestate |
| L4 | 资源权限追踪 | Linear Types |
| L5 | IO权限控制 | Capability |

### Functional Core, Imperative Shell架构

```
┌─────────────────────────────────────────┐
│           CLI Binary (Shell)            │
│  - 参数解析、IO操作、副作用管理          │
├─────────────────────────────────────────┤
│         Command Handler (Shell)         │
│  - 协调纯函数和副作用                    │
├─────────────────────────────────────────┤
│        Business Logic (Core)            │
│  - 纯函数，无副作用，易测试              │
└─────────────────────────────────────────┘
```

### 工具链设计
- CLI设计：命令行接口作为状态空间的入口
- 配置验证：配置加载时即验证合法性
- 渐进式披露：简单任务简单，复杂任务可行
- Effect Trait：抽象所有副作用，便于测试

## 待验证假设

- [ ] **假设1**: Typestate模式在大型CLI项目中不会导致类型爆炸
  - 验证思路: 分析ripgrep、fd等项目代码，统计类型状态数量

- [ ] **假设2**: Functional Core, Imperative Shell在Rust CLI中的性能开销可忽略
  - 验证思路: 对比纯函数核心与内联IO操作的基准测试

- [ ] **假设3**: 六层边界可以系统化应用到任何CLI工具设计
  - 验证思路: 选择3-5个不同领域CLI工具，应用六层边界并评估

- [ ] **假设4**: 分层配置验证比即时验证有更好的用户体验
  - 验证思路: 用户研究，对比两种模式的错误信息清晰度

## 下一步研究方向

1. **Rust CLI工具案例深度分析**
   - 深入阅读ripgrep、fd、bat源代码
   - 提取架构设计模式

2. **Typestate模式在复杂CLI工作流中的应用**
   - 研究多阶段命令的类型安全实现
   - 探索命令依赖关系的编译期验证

3. **Effect System在Rust CLI中的轻量级实现**
   - 对比trait-based DI、Free Monad、Algebraic Effects
   - 开发结合Typestate和Effect的CLI框架原型

4. **六层边界的量化评估**
   - 定义度量指标：编译期错误捕获率、运行时错误率、代码复杂度
   - 在实际项目中对比应用六层边界前后的指标变化
