# 研究方向 07: 分层设计 (Layered Design)

## 核心问题

**Syntax→Semantic→Pattern→Domain 如何转换？**

这是一个关于编译器/语言处理器架构的核心问题。通过两轮深入研究，我们探索了如何使用Rust的类型系统实现类型安全的四层架构转换。

---

## 版本历史

- **v1 (2026-03-10)**: 第一轮研究，基于解析器组合子和类型检查器的实现
- **v2 (2026-03-11)**: 第二轮深入研究，完整的Layer trait抽象和层间边界检查

---

## 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        Source Code                               │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  Layer 1: Syntax Layer                                          │
│  ├─ Lexer: 词法分析                                               │
│  ├─ Parser: 语法分析                                              │
│  └─ Output: AST (抽象语法树)                                      │
└───────────────────────────┬─────────────────────────────────────┘
                            │ LayerBoundary<Syntax, Semantic>
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  Layer 2: Semantic Layer                                        │
│  ├─ Symbol Table: 符号表管理                                      │
│  ├─ Type Checker: 类型检查                                        │
│  └─ Output: TypedAST (类型化语法树)                               │
└───────────────────────────┬─────────────────────────────────────┘
                            │ LayerBoundary<Semantic, Pattern>
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  Layer 3: Pattern Layer                                         │
│  ├─ Constant Folding: 常量折叠                                    │
│  ├─ Dead Code Elimination: 死代码消除                             │
│  ├─ Inline Expansion: 内联展开                                    │
│  └─ Output: OptimizedAST (优化后的语法树)                         │
└───────────────────────────┬─────────────────────────────────────┘
                            │ LayerBoundary<Pattern, Domain>
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  Layer 4: Domain Layer                                          │
│  ├─ Code Generator: 代码生成器                                    │
│  ├─ Target: C / LLVM / WASM / Interpreter                        │
│  └─ Output: Target Code (目标代码)                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## 层间转换机制 (v2核心创新)

### 1. Layer Trait - 核心抽象

```rust
pub trait Layer {
    const ID: LayerId;
    type Input;
    type Output;
    type Context;

    fn transform(input: Self::Input, ctx: &mut Self::Context) -> LayerResult<Self::Output>;
}
```

每一层实现这个trait，确保：
- **类型安全**：输入输出类型在编译期确定
- **错误处理**：统一的错误类型LayerResult
- **上下文隔离**：每层有自己的上下文状态

### 2. LayerBoundary - 边界检查

```rust
pub struct LayerBoundary<From: Layer, To: Layer> {
    check_runtime: bool,
}
```

边界检查器实现渐进类型边界检查的思想：
- **静态保证**：编译期类型检查
- **动态检查**：可选的运行时验证
- **错误追踪**：精确的层间错误定位

### 3. 转换流程

```rust
impl LayerTransformer {
    pub fn syntax_to_semantic(ast: AstNode, ctx: &mut SemanticContext) -> LayerResult<TypedAst> {
        let boundary = LayerBoundary::<SyntaxLayer, SemanticLayer>::new(true);
        boundary.validate(&ast)?;  // 边界检查
        SemanticLayer::transform(ast, ctx)
    }
}
```

---

## 实现细节

### Syntax Layer (语法层)

**职责**：将源代码转换为抽象语法树(AST)

**组件**：
- `Lexer`: 词法分析，生成Token序列
- `Parser`: 递归下降语法分析，生成AST
- `Span`: 源代码位置追踪

**关键数据结构**：
```rust
pub enum AstNode {
    Program(Vec<AstNode>),
    Function(FunctionDecl),
    Struct(StructDecl),
    Let(LetStmt),
    Expr(Expr),
    // ...
}
```

### Semantic Layer (语义层)

**职责**：类型检查和符号解析

**组件**：
- `SymbolTable`: 分层作用域管理
- `TypeChecker`: 类型推导和验证
- `SemanticAnalyzer`: 语义分析主逻辑

**关键特性**：
- 支持嵌套作用域
- 类型推导（Hindley-Milner风格简化版）
- 可变性检查

### Pattern Layer (模式层)

**职责**：代码优化和转换

**优化策略**：

| 优化 | 描述 | 触发条件 |
|------|------|----------|
| 常量折叠 | 编译期计算常量表达式 | 操作数均为常量 |
| 死代码消除 | 删除不可达代码 | return后或false条件 |
| 内联展开 | 小函数内联 | 函数体≤3条语句 |

**示例**：
```rust
// 优化前
let x = 1 + 2 + 3;
if false { let y = 10; }

// 优化后
let x = 6;  // 常量折叠
// if分支被完全消除
```

### Domain Layer (领域层)

**职责**：目标代码生成

**支持目标**：
- `C`: 生成可移植的C代码
- `LLVM`: LLVM IR（待实现）
- `WASM`: WebAssembly（待实现）
- `Interpreter`: 直接解释执行

**类型映射**：
```rust
fn c_type(&self, ty: &Type) -> String {
    match ty {
        Type::Unit => "void",
        Type::Int => "int64_t",
        Type::Float => "double",
        Type::Bool => "bool",
        // ...
    }
}
```

---

## 渐进式边界实现

### 核心思想

借鉴渐进类型(Gradual Typing)的边界概念：

1. **静态边界**：编译期类型检查确保层间契约
2. **动态边界**：可选的运行时验证
3. **信任边界**：显式的层间信任关系

### 实现代码

```rust
pub fn validate(&self, output: &From::Output) -> LayerResult<()> {
    if self.check_runtime {
        self.runtime_check(output)?;
    }
    Ok(())
}
```

### 错误处理

```rust
pub enum LayerError {
    SyntaxError(String),
    TypeError(String),
    PatternError(String),
    DomainError(String),
    BoundaryViolation { from: LayerId, to: LayerId, reason: String },
}
```

---

## 性能考虑

### 编译期优化

1. **零成本抽象**：使用泛型和单态化
2. **内联展开**：关键路径函数标记`#[inline]`
3. **借用优化**：避免不必要的克隆

### 增量编译支持

```rust
pub struct IncrementalCompiler {
    cache: HashMap<LayerId, CacheEntry>,
}

impl IncrementalCompiler {
    pub fn compile_with_cache(&mut self, source: &str) -> CompileResult {
        // 检查每层缓存
        // 只重新编译变更的层
    }
}
```

---

## 应用场景

### 1. 领域特定语言(DSL)

分层架构特别适合DSL实现：
- Syntax层：DSL语法解析
- Semantic层：DSL语义验证
- Pattern层：DSL特定优化
- Domain层：生成目标平台代码

### 2. 安全关键系统

借鉴Certifying Compiler思想：
- 每层输出可验证的中间表示
- 层间转换可追踪和审计
- 支持形式化验证

### 3. 多目标编译

通过Domain层的多后端支持：
- 同一前端支持多种目标
- 目标特定的优化
- 渐进式Lowering

---

## 与现有工作的关系

### MLIR (Multi-Level IR)

相似点：
- 多层抽象
- 渐进式Lowering
- Dialect系统

差异点：
- 本实现使用Rust类型系统保证安全
- 更轻量级，适合嵌入式场景
- 显式的层间边界检查

### Certifying Compiler (2025)

借鉴思想：
- 四层架构
- 增量验证
- 翻译验证而非完整验证

---

## 两轮研究对比

| 维度 | v1 (2026-03-10) | v2 (2026-03-11) |
|------|-----------------|-----------------|
| **核心抽象** | 解析器组合子 | Layer trait |
| **层间通信** | 直接函数调用 | 类型安全的LayerBoundary |
| **错误处理** | String错误 | 结构化LayerError |
| **代码规模** | ~800行 | ~3345行 |
| **优化支持** | 基础 | 常量折叠、死代码消除、内联 |
| **目标平台** | Rust/Python/JS | C（可扩展） |
| **类型系统** | 基础类型 | 完整类型系统（含引用、数组） |
| **渐进边界** | 无 | 显式边界检查 |

---

## 未来方向

### 短期（1-2周）

1. **完善错误报告**：添加源代码位置信息
2. **更多优化**：循环展开、公共子表达式消除
3. **LLVM后端**：生成LLVM IR

### 中期（1-2月）

1. **增量编译**：完整的缓存机制
2. **并行编译**：层内并行处理
3. **LSP支持**：语言服务器协议

### 长期（3-6月）

1. **形式化验证**：关键层的正确性证明
2. **JIT编译**：即时编译支持
3. **调试信息**：完整的源码映射

---

## 代码位置

- **v1实现**: `drafts/20260310_1534_layered_compiler.rs`
- **v2实现**: `drafts/20260311_1000_layered_design_v2.rs` (3345行)
- **研究轨迹**: `logs/trails/07_layered_design/20260311_1000_layered_v2_trail.md`

---

## 参考资料

1. [A Layered Certifying Compiler Architecture (2025)](https://webspace.science.uu.nl/~swier004/publications/2025-funarch.pdf)
2. [MLIR: Multi-Level Intermediate Representation](https://mlir.llvm.org/)
3. [Towards Practical Gradual Typing (ECOOP 2015)](https://www2.ccs.neu.edu/racket/pubs/ecoop2015-takikawa-et-al.pdf)
4. [Deep and Shallow Types for Gradual Languages (PLDI 2022)](https://users.cs.utah.edu/~blg/publications/apples-to-apples/g-pldi-2022.pdf)
5. [Rust Compiler Performance Survey 2025](https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results/)

---

## 总结

通过本次深入研究，我们：

1. **验证了技术假设**：Rust的trait系统确实可以实现类型安全的层间转换
2. **实现了完整原型**：包含词法分析、语法分析、类型检查、优化、代码生成
3. **探索了渐进边界**：借鉴渐进类型思想实现层间边界检查
4. **证明了可行性**：分层架构可以实现接近零成本的抽象

核心洞察：**分层架构的关键在于层间接口的设计**。通过精心设计的类型契约，可以实现：
- 编译期安全保证
- 运行时性能优化
- 模块化和可扩展性

第二轮研究相比第一轮，在抽象层次、类型安全、优化能力等方面都有显著提升，验证了持续深入研究的价值。
