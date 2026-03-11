# 10_tool_design

## 方向名称
工具设计：无缺陷工具集

## 核心问题
如何设计'无法产生错误'的工具?

## 研究历程

### 2026-03-11 09:30 深度研究：无法产生错误的工具设计

**研究范围**: Web Research + 假设验证 + 代码实现（~35分钟）

**核心问题**: 如何设计'无法产生错误'的工具?

**Web Research发现**:

1. **API设计最佳实践**（来源：[REST API Best Practices](https://blog.postman.com/rest-api-best-practices/)）
   - 资源导向架构、状态无关认证、分层配置
   - 输入验证、错误分类、结构化输出

2. **CLI工具设计原则**（来源：[UX patterns for CLI tools](https://lucasfcosta.com/blog/ux-patterns-cli-tools)）
   - 早期失败、智能错误恢复、清晰错误消息
   - 干运行选项、超时机制、速率限制

3. **Rust类型安全API设计**（来源：[Rust API Guidelines - Type Safety](https://rust-lang.github.io/api-guidelines/type-safety.html)）
   - Newtype模式：静态区分语义不同的值
   - Typestate模式：编译期状态机
   - 自定义类型优于原始类型

4. **错误预防设计模式**（来源：[Design for Error Prevention](https://www.perpetualny.com/blog/design-for-error-prevention-solving-problems-before-they-start)）
   - 约束型输入模式、渐进式披露
   - 视觉提示、即时验证

**验证的假设**：

| 假设 | 验证结果 | 关键证据 |
|------|----------|----------|
| **技术假设1**: 编译时保证 > 运行时检查 | ✅ 验证 | Newtype/Typestate在编译期消除整类错误 |
| **技术假设2**: 边界验证 > 内部防御 | ✅ 验证 | 验证后的类型无需重复检查 |
| **技术假设3**: 显式设计 > 隐式约定 | ✅ 验证 | 枚举替代bool使状态显式 |
| **实现假设1**: Newtype模式可行 | ✅ 验证 | UserId/OrderId无法互换使用 |
| **实现假设2**: Typestate模式可行 | ✅ 验证 | 文件状态转换编译期强制执行 |
| **实现假设3**: 类型安全Builder可行 | ✅ 验证 | 未设置必需字段时编译错误 |
| **性能假设**: 安全设计零运行时开销 | ✅ 验证 | PhantomData编译期擦除 |
| **适用性假设**: 适用于高可靠性工具 | ✅ 验证 | 系统管理、数据迁移场景 |

**代码实现**: `drafts/20260311_0800_tool_design.rs` (380行)
- Newtype模式验证 (UserId/OrderId/ProductId)
- Typestate模式验证 (TypedFile<Closed/OpenForRead/OpenForWrite>)
- 边界验证模式 (NonEmptyString/PositiveInt)
- 类型安全Builder (HttpRequestBuilder<UrlState, MethodState>)
- 完整测试覆盖

**关键洞察**：
- 类型系统是最好的测试：编译错误比运行时测试更快反馈
- 非法状态不可表示：通过类型设计使错误状态在编译期就不可达
- 零成本抽象：所有类型安全机制在运行时完全擦除

---

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

- `drafts/20260311_0800_tool_design.rs` - 无法产生错误的工具设计验证
  - 包含: Newtype模式 (UserId/OrderId/ProductId)
  - 包含: Typestate模式 (TypedFile<Closed/OpenForRead/OpenForWrite>)
  - 包含: 边界验证 (NonEmptyString/PositiveInt)
  - 包含: 类型安全Builder (HttpRequestBuilder)
  - 包含: 完整测试覆盖
  - 380行Rust代码

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

- [x] **假设1**: 编译时保证优于运行时检查
  - 验证结果: ✅ 通过 - Newtype/Typestate在编译期消除整类错误

- [x] **假设2**: 边界验证优于内部防御
  - 验证结果: ✅ 通过 - 验证后的类型在内部无需重复检查

- [x] **假设3**: 类型安全设计零运行时开销
  - 验证结果: ✅ 通过 - PhantomData在编译期完全擦除

- [ ] **假设4**: Typestate模式在大型CLI项目中不会导致类型爆炸
  - 验证思路: 分析ripgrep、fd等项目代码，统计类型状态数量

- [ ] **假设5**: Functional Core, Imperative Shell在Rust CLI中的性能开销可忽略
  - 验证思路: 对比纯函数核心与内联IO操作的基准测试

- [ ] **假设6**: 六层边界可以系统化应用到任何CLI工具设计
  - 验证思路: 选择3-5个不同领域CLI工具，应用六层边界并评估

- [ ] **假设7**: 分层配置验证比即时验证有更好的用户体验
  - 验证思路: 用户研究，对比两种模式的错误信息清晰度

## 下一步研究方向

### 基于本次研究的调整

**优先级调整**：
- ✅ 已验证：Newtype、Typestate、边界验证、类型安全Builder
- 🔍 待验证：大型项目中的类型爆炸问题、性能基准测试

**新增研究方向**：

1. **Clap Derive与类型安全结合**
   - 研究如何将验证逻辑集成到clap的value_parser中
   - 实现从解析到验证的类型安全管道
   - 时间：1周内

2. **错误类型设计模式**
   - 设计结构化的错误类型层次
   - 实现用户友好的错误消息生成
   - 时间：1周内

### 原有方向（已调整优先级）

3. **Rust CLI工具案例深度分析** [优先级：高]
   - 深入阅读ripgrep、fd、bat源代码
   - 重点关注Newtype和Typestate的实际使用
   - 时间：2周内

4. **六层边界的量化评估** [优先级：中]
   - 定义度量指标：编译期错误捕获率、运行时错误率、代码复杂度
   - 在实际项目中对比应用六层边界前后的指标变化
   - 时间：1个月内

5. **Effect System在Rust CLI中的轻量级实现** [优先级：低]
   - 对比trait-based DI、Free Monad、Algebraic Effects
   - 开发结合Typestate和Effect的CLI框架原型
   - 时间：3个月内
