# 07_layered_design - 分层设计研究

## 核心问题
Syntax → Semantic → Pattern → Domain 四层如何转换?

**研究日期**: 2026-03-10
**研究重点**: 编译器架构实现 - 如何实现四层转换管道

## 研究结论

### 四层架构设计（编译器实现）

基于Rust-Analyzer架构和RCL类型检查器的研究，实现完整的四层编译器管道：

#### Layer 1: Syntax层 (语法层)
- **职责**: 词法分析，将源代码转换为Token序列
- **实现**: `Lexer` - 基于Peekable字符迭代器
- **输出**: `Vec<Token>`
- **关键设计**:
  - 支持整数、浮点、字符串、布尔字面量
  - 完整的运算符和关键字集合
  - 注释跳过和错误处理

#### Layer 2: Semantic层 (语义层)
- **职责**: 语法分析，构建AST
- **实现**: `RecursiveDescentParser` + `Parser Combinator` trait
- **输出**: `Program` (包含函数、结构体、语句)
- **关键设计**:
  - 解析器组合子支持map/and_then/or/many/optional操作
  - 递归下降处理复杂语法结构
  - 表达式、语句、函数定义完整支持

#### Layer 3: Pattern层 (模式层)
- **职责**: 类型检查，过滤无效语义
- **实现**: `TypeChecker` - 基于TypeEnv环境
- **输出**: 类型验证通过的AST
- **关键设计**:
  - `check_expr`融合类型推断和检查（受RCL启发）
  - `is_subtype_of`实现子类型关系
  - `join`操作实现类型最小上界
  - 环境栈支持嵌套作用域

#### Layer 4: Domain层 (领域层)
- **职责**: 代码生成，映射到目标语言
- **实现**: `CodeGenerator` - 多目标代码模板
- **输出**: Rust/Python/JavaScript代码
- **关键设计**:
  - 统一的AST遍历框架
  - 目标特定的类型映射
  - 模板化的代码生成

### 层间转换机制

```
Source Code (String)
    ↓ [Syntax Layer] Lexer::next_token()
Token Stream (Vec<Token>)
    ↓ [Semantic Layer] Parser Combinator map()
AST (Program with Expr/Stmt/Function)
    ↓ [Pattern Layer] TypeChecker::check_program()
Validated AST (Type-safe)
    ↓ [Domain Layer] CodeGenerator::generate()
Target Code (Rust/Python/JS)
```

### 核心转换原理

1. **Syntax → Semantic: Parser Combinator转换**
   ```rust
   // map操作实现语法到语义转换
   ident().map(|name| Expr::Var(name))

   // and_then顺序组合
   match_token(Token::Let)
       .and_then(ident())
       .map(|(_, name)| name)
   ```

2. **Semantic → Pattern: 类型检查过滤器**
   ```rust
   // 类型检查作为过滤器
   pub fn check_expr(&mut self, expr: &Expr) -> Result<Type, String>

   // 子类型检查
   if !actual_type.is_subtype_of(&expected_type) {
       return Err("Type mismatch".to_string());
   }
   ```

3. **Pattern → Domain: 代码模板生成**
   ```rust
   // 目标特定的类型映射
   fn rust_type(&self, type_: &Type) -> String {
       match type_ {
           Type::Int => "i64".to_string(),
           Type::List(t) => format!("Vec<{}>", self.rust_type(t)),
       }
   }
   ```

### 类型系统实现

```rust
pub enum Type {
    Int, Float, String, Bool, Void, Any,
    List(Box<Type>),
    Function(Box<Type>, Box<Type>),
    Unknown,
}

impl Type {
    // 子类型关系
    pub fn is_subtype_of(&self, other: &Type) -> bool

    // 类型join（最小上界）
    pub fn join(&self, other: &Type) -> Type
}
```

### 编译器管道

```rust
pub struct CompilerPipeline {
    target: Target,  // Rust/Python/JavaScript
}

impl CompilerPipeline {
    pub fn compile(&self, source: &str) -> Result<String, Vec<String>> {
        // Step 1: Syntax Layer
        let tokens = self.lex(source)?;

        // Step 2: Semantic Layer
        let ast = self.parse(&tokens)?;

        // Step 3: Pattern Layer
        self.type_check(&ast)?;

        // Step 4: Domain Layer
        self.generate(&ast)
    }
}
```

## 假设验证结果（2026-03-10编译器实现）

| 假设 | 验证结果 | 说明 |
|------|----------|------|
| H1: 解析器组合子可实现Syntax→Semantic转换 | **已验证** | Parser trait的map操作将Token流转换为AST节点，and_then/or/many组合子实现复杂语法结构解析 |
| H2: 类型检查可作为Semantic→Pattern的过滤器 | **已验证** | TypeChecker通过check_expr方法过滤类型不匹配节点，TypeEnv维护作用域，is_subtype_of实现类型约束 |
| H3: 代码模板可实现Pattern→Domain生成 | **已验证** | CodeGenerator通过目标特定模板（rust_type/python_type/js_type）生成多目标代码 |

### 验证实现细节

**H1验证 - Parser Combinator实现**:
- 定义`Parser<T>` trait支持map/and_then/or/many/optional
- `MapParser`实现Syntax→Semantic转换
- `RecursiveDescentParser`构建完整AST

**H2验证 - 类型检查过滤器**:
- `TypeChecker::check_expr`融合推断和检查
- `TypeEnv`支持嵌套作用域（通过parent链接）
- `Type::join`实现类型最小上界
- 错误收集机制支持多错误报告

**H3验证 - 代码模板生成**:
- `CodeGenerator`支持Rust/Python/JavaScript三目标
- 统一的AST遍历框架
- 目标特定的类型映射（如Type::Int→i64/int/number）

## 关键发现（基于编译器架构研究）

### 发现1: Rust-Analyzer的Map-Reduce范式
- 索引阶段（Syntax层）与完整分析（Semantic层）分离
- 索引可增量更新，实现O(变更文件数)复杂度
- 延迟解析类型，结果memoized缓存

### 发现2: RCL类型检查器的融合设计
- `check_expr`同时处理推断和检查
- 自顶向下传递期望类型，实现精准错误定位
- `TypeDiff`三态：Ok | Defer（运行时检查）| Error

### 发现3: Parser Combinator的转换能力
- `map`操作天然支持Syntax→Semantic转换
- `and_then`顺序组合对应语法规则的顺序
- `or`选择组合支持备选语法路径

### 发现4: 分层编译器的Pipe-Filter架构
- 每层是独立的Filter，通过Pipe连接
- 统一使用Result类型传递错误或结果
- 支持多目标代码生成（Rust/Python/JS）

## 实现文件

- `drafts/20260310_1534_layered_compiler.rs` - 完整编译器实现（Lexer + Parser + TypeChecker + CodeGen）

## 下一步方向

1. **增量编译**: 实现Rust-Analyzer风格的增量索引和缓存
2. **泛型支持**: 扩展类型系统支持类型参数和约束
3. **模式匹配**: 在Pattern层实现更多编译器优化（常量折叠、尾递归）
4. **错误恢复**: 增强解析器的错误恢复能力，支持更多诊断信息
