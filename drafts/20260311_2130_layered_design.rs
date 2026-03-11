// ============================================================================
// 分层架构研究：Syntax → Semantic → Pattern → Domain 转换验证
// 文件名: 20260311_2130_layered_design.rs
// 研究方向: 07_layered_design - 分层设计
// ============================================================================

// =============================================================================
// LAYER 1: SYNTAX (语法层)
// =============================================================================
// 语法层关注代码的结构表示 - 如何解析和组织文本
// 在Rust中，这对应于AST（抽象语法树）和基本语法结构

/// 语法层：原始标记和基本结构
pub mod syntax {
    /// 原始标记类型（类比Token）
    #[derive(Debug, Clone, PartialEq)]
    pub enum Token {
        Identifier(String),
        Number(i64),
        Keyword(Keyword),
        Symbol(char),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum Keyword {
        Let,
        Fn,
        Struct,
        Impl,
    }

    /// 语法节点：无意义的结构容器
    #[derive(Debug, Clone)]
    pub struct SyntaxNode {
        pub kind: NodeKind,
        pub children: Vec<SyntaxNode>,
        pub text: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum NodeKind {
        Root,
        Expression,
        Statement,
        Declaration,
    }

    /// 语法解析器：将文本转为语法树（无语义检查）
    pub struct SyntaxParser;

    impl SyntaxParser {
        pub fn parse(input: &str) -> SyntaxNode {
            // 简化：仅演示语法层只关心结构
            SyntaxNode {
                kind: NodeKind::Root,
                children: vec![],
                text: input.to_string(),
            }
        }
    }
}

// =============================================================================
// LAYER 2: SEMANTIC (语义层)
// =============================================================================
// 语义层赋予语法结构意义 - 类型、作用域、约束
// 这是 Syntax → Semantic 的转换关键

pub mod semantic {
    use super::syntax::*;

    /// 语义类型：给语法节点赋予类型意义
    #[derive(Debug, Clone, PartialEq)]
    pub enum Type {
        Integer,
        Boolean,
        String,
        Function(Box<Type>, Box<Type>), // 函数类型: 参数 -> 返回
        Generic(String, Vec<Constraint>), // 带约束的泛型
        DomainSpecific(String), // 领域特定类型标记
    }

    /// 约束：类型的能力要求
    #[derive(Debug, Clone, PartialEq)]
    pub enum Constraint {
        Addable,
        Comparable,
        Display,
        Custom(String),
    }

    /// 语义节点：语法节点 + 类型信息
    #[derive(Debug, Clone)]
    pub struct SemanticNode {
        pub syntax: SyntaxNode,
        pub ty: Type,
        pub scope: Scope,
    }

    /// 作用域：变量绑定环境
    #[derive(Debug, Clone, Default)]
    pub struct Scope {
        pub bindings: std::collections::HashMap<String, Type>,
        pub parent: Option<Box<Scope>>,
    }

    impl Scope {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn lookup(&self, name: &str) -> Option<&Type> {
            self.bindings.get(name).or_else(|| {
                self.parent.as_ref().and_then(|p| p.lookup(name))
            })
        }
    }

    /// 语义分析器：Syntax → Semantic 转换
    pub struct SemanticAnalyzer;

    impl SemanticAnalyzer {
        /// 核心转换：给语法节点赋予语义
        pub fn analyze(node: &SyntaxNode) -> Result<SemanticNode, String> {
            // 简化示例：根据语法结构推断类型
            let ty = match node.kind {
                NodeKind::Expression => Type::Integer, // 简化推断
                NodeKind::Declaration => Type::DomainSpecific("unknown".to_string()),
                _ => Type::String,
            };

            Ok(SemanticNode {
                syntax: node.clone(),
                ty,
                scope: Scope::new(),
            })
        }

        /// 类型检查：验证约束满足
        pub fn check_constraints(ty: &Type, constraints: &[Constraint]) -> bool {
            // 简化：实际实现会检查类型是否满足所有约束
            match ty {
                Type::Integer => constraints.iter().all(|c| {
                    matches!(c, Constraint::Addable | Constraint::Comparable | Constraint::Display)
                }),
                _ => true,
            }
        }
    }
}

// =============================================================================
// LAYER 3: PATTERN (模式层)
// =============================================================================
// 模式层提取可复用的抽象结构
// Semantic → Pattern 转换：从具体类型中提取通用模式

pub mod pattern {
    use super::semantic::*;

    /// 模式特征：可复用的行为抽象
    /// 这是从语义层提取的通用模式
    pub trait Pattern: Sized {
        type Input;
        type Output;

        /// 应用模式，进行转换
        fn apply(&self, input: Self::Input) -> Self::Output;

        /// 组合两个模式
        fn compose<P: Pattern<Input = Self::Output>>(self, other: P) -> ComposedPattern<Self, P> {
            ComposedPattern { first: self, second: other }
        }
    }

    /// 组合模式：模式的顺序组合
    pub struct ComposedPattern<P1, P2> {
        first: P1,
        second: P2,
    }

    impl<P1, P2> Pattern for ComposedPattern<P1, P2>
    where
        P1: Pattern,
        P2: Pattern<Input = P1::Output>,
    {
        type Input = P1::Input;
        type Output = P2::Output;

        fn apply(&self, input: Self::Input) -> Self::Output {
            let intermediate = self.first.apply(input);
            self.second.apply(intermediate)
        }
    }

    // -------------------------------------------------------------------------
    // 具体模式实现
    // -------------------------------------------------------------------------

    /// 转换模式：类型A -> 类型B的通用转换
    pub struct TransformPattern<F, A, B> {
        transformer: F,
        _phantom: std::marker::PhantomData<(A, B)>,
    }

    impl<F, A, B> TransformPattern<F, A, B>
    where
        F: Fn(A) -> B,
    {
        pub fn new(f: F) -> Self {
            Self {
                transformer: f,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    impl<F, A, B> Pattern for TransformPattern<F, A, B>
    where
        F: Fn(A) -> B,
    {
        type Input = A;
        type Output = B;

        fn apply(&self, input: A) -> B {
            (self.transformer)(input)
        }
    }

    /// 验证模式：检查输入是否满足条件
    pub struct ValidationPattern<F, T> {
        validator: F,
        _phantom: std::marker::PhantomData<T>,
    }

    impl<F, T> ValidationPattern<F, T>
    where
        F: Fn(&T) -> bool,
    {
        pub fn new(f: F) -> Self {
            Self {
                validator: f,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    impl<F, T> Pattern for ValidationPattern<F, T>
    where
        F: Fn(&T) -> bool,
        T: Clone,
    {
        type Input = T;
        type Output = Result<T, String>;

        fn apply(&self, input: T) -> Self::Output {
            if (self.validator)(&input) {
                Ok(input)
            } else {
                Err("Validation failed".to_string())
            }
        }
    }

    /// 管道模式：数据流处理链
    pub struct PipelinePattern<Stages> {
        stages: Stages,
    }

    impl<Stages> PipelinePattern<Stages> {
        pub fn new(stages: Stages) -> Self {
            Self { stages }
        }
    }

    // 宏辅助：简化模式定义
    #[macro_export]
    macro_rules! pattern {
        ($name:ident: $in:ty => $out:ty = $body:expr) => {
            pub struct $name;
            impl Pattern for $name {
                type Input = $in;
                type Output = $out;
                fn apply(&self, input: Self::Input) -> Self::Output {
                    $body(input)
                }
            }
        };
    }
}

// =============================================================================
// LAYER 4: DOMAIN (领域层)
// =============================================================================
// 领域层将模式特化到具体应用场景
// Pattern → Domain 转换：实例化通用模式到领域特定实现

pub mod domain {
    use super::pattern::*;
    use super::semantic::*;

    /// 领域上下文：包含领域特定配置
    #[derive(Debug, Clone)]
    pub struct DomainContext {
        pub name: String,
        pub constraints: Vec<Constraint>,
        pub rules: Vec<DomainRule>,
    }

    #[derive(Debug, Clone)]
    pub enum DomainRule {
        MaxDepth(usize),
        RequiredField(String),
        CustomRule(String),
    }

    /// 领域特定语言（DSL）构造器
    /// 这是Pattern层到Domain层的桥梁
    pub struct DslBuilder<Context> {
        context: Context,
        patterns: Vec<Box<dyn Fn(&Context, Type) -> Type>>,
    }

    impl<Context> DslBuilder<Context> {
        pub fn new(context: Context) -> Self {
            Self {
                context,
                patterns: vec![],
            }
        }

        pub fn with_pattern<F>(mut self, pattern: F) -> Self
        where
            F: Fn(&Context, Type) -> Type + 'static,
        {
            self.patterns.push(Box::new(pattern));
            self
        }

        /// 构建领域特定类型
        pub fn build(&self, base: Type) -> Type {
            self.patterns.iter().fold(base, |ty, pat| pat(&self.context, ty))
        }
    }

    // -------------------------------------------------------------------------
    // 具体领域实现：编译器领域示例
    // -------------------------------------------------------------------------

    /// 编译器领域：展示四层完整转换
    pub mod compiler_domain {
        use super::*;
        use crate::pattern::Pattern;

        /// 编译器阶段（领域概念）
        #[derive(Debug, Clone)]
        pub enum CompilerStage {
            Lexing,
            Parsing,
            SemanticAnalysis,
            Optimization,
            CodeGen,
        }

        /// 编译器管道：领域特定的模式实现
        pub struct CompilerPipeline {
            stages: Vec<CompilerStage>,
        }

        impl CompilerPipeline {
            pub fn new() -> Self {
                Self {
                    stages: vec![
                        CompilerStage::Lexing,
                        CompilerStage::Parsing,
                        CompilerStage::SemanticAnalysis,
                        CompilerStage::Optimization,
                        CompilerStage::CodeGen,
                    ],
                }
            }

            /// 执行编译：Syntax -> Semantic -> Pattern -> Domain
            pub fn compile(&self, source: &str) -> Result<CompiledOutput, String> {
                // Step 1: Syntax Layer
                let syntax = super::super::syntax::SyntaxParser::parse(source);
                println!("[Syntax] Parsed: {:?}", syntax.kind);

                // Step 2: Semantic Layer
                let semantic = super::super::semantic::SemanticAnalyzer::analyze(&syntax)
                    .map_err(|e| format!("Semantic error: {}", e))?;
                println!("[Semantic] Type: {:?}", semantic.ty);

                // Step 3: Pattern Layer - 应用编译模式
                let transform = TransformPattern::new(|s: super::super::semantic::SemanticNode| {
                    format!("Pattern({:?})", s.ty)
                });
                let pattern_result = transform.apply(semantic);
                println!("[Pattern] Transformed: {}", pattern_result);

                // Step 4: Domain Layer - 生成领域输出
                let output = CompiledOutput {
                    code: pattern_result,
                    stage_count: self.stages.len(),
                };
                println!("[Domain] Generated output with {} stages", output.stage_count);

                Ok(output)
            }
        }

        #[derive(Debug)]
        pub struct CompiledOutput {
            pub code: String,
            pub stage_count: usize,
        }
    }

    // -------------------------------------------------------------------------
    // 具体领域实现：数据处理领域示例
    // -------------------------------------------------------------------------

    pub mod data_domain {

        /// 数据流处理领域
        pub struct DataPipeline<T> {
            validators: Vec<Box<dyn Fn(&T) -> bool>>,
            transformers: Vec<Box<dyn Fn(T) -> T>>,
        }

        impl<T> DataPipeline<T> {
            pub fn new() -> Self {
                Self {
                    validators: vec![],
                    transformers: vec![],
                }
            }

            pub fn validate<F>(mut self, validator: F) -> Self
            where
                F: Fn(&T) -> bool + 'static,
            {
                self.validators.push(Box::new(validator));
                self
            }

            pub fn transform<F>(mut self, transformer: F) -> Self
            where
                F: Fn(T) -> T + 'static,
            {
                self.transformers.push(Box::new(transformer));
                self
            }

            pub fn process(&self, input: T) -> Result<T, String>
            where
                T: Clone,
            {
                // 验证阶段
                for validator in &self.validators {
                    if !validator(&input) {
                        return Err("Validation failed".to_string());
                    }
                }

                // 转换阶段
                let mut result = input;
                for transformer in &self.transformers {
                    result = transformer(result);
                }

                Ok(result)
            }
        }
    }
}

// =============================================================================
// 验证与测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::Pattern;

    #[test]
    fn test_syntax_layer() {
        let node = syntax::SyntaxParser::parse("test input");
        assert_eq!(node.kind, syntax::NodeKind::Root);
        assert_eq!(node.text, "test input");
    }

    #[test]
    fn test_semantic_layer() {
        // 测试Root节点返回String类型（默认行为）
        let syntax = syntax::SyntaxParser::parse("42");
        let semantic = semantic::SemanticAnalyzer::analyze(&syntax).unwrap();
        assert_eq!(semantic.ty, semantic::Type::String);

        // 测试Expression节点返回Integer类型
        let expr_syntax = syntax::SyntaxNode {
            kind: syntax::NodeKind::Expression,
            children: vec![],
            text: "42".to_string(),
        };
        let expr_semantic = semantic::SemanticAnalyzer::analyze(&expr_syntax).unwrap();
        assert_eq!(expr_semantic.ty, semantic::Type::Integer);
    }

    #[test]
    fn test_pattern_layer() {
        let transform = pattern::TransformPattern::new(|x: i32| x * 2);
        assert_eq!(transform.apply(21), 42);

        let validation = pattern::ValidationPattern::new(|x: &i32| *x > 0);
        assert!(validation.apply(42).is_ok());
        assert!(validation.apply(-1).is_err());
    }

    #[test]
    fn test_pattern_composition() {
        use crate::pattern::Pattern;

        let double = pattern::TransformPattern::new(|x: i32| x * 2);
        let add_ten = pattern::TransformPattern::new(|x: i32| x + 10);

        let composed = double.compose(add_ten);
        assert_eq!(composed.apply(5), 20); // (5 * 2) + 10 = 20
    }

    #[test]
    fn test_domain_compiler() {
        let pipeline = domain::compiler_domain::CompilerPipeline::new();
        let result = pipeline.compile("fn main() {}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_domain_data_pipeline() {
        let pipeline = domain::data_domain::DataPipeline::<i32>::new()
            .validate(|x| *x > 0)
            .transform(|x| x * 2)
            .transform(|x| x + 1);

        assert_eq!(pipeline.process(5).unwrap(), 11); // (5 * 2) + 1 = 11
        assert!(pipeline.process(-1).is_err());
    }

    #[test]
    fn test_constraint_checking() {
        let ty = semantic::Type::Integer;
        let constraints = vec![
            semantic::Constraint::Addable,
            semantic::Constraint::Comparable,
        ];
        assert!(semantic::SemanticAnalyzer::check_constraints(&ty, &constraints));
    }

    #[test]
    fn test_dsl_builder() {
        let context = domain::DomainContext {
            name: "test".to_string(),
            constraints: vec![semantic::Constraint::Display],
            rules: vec![],
        };

        let builder = domain::DslBuilder::new(context)
            .with_pattern(|_, ty| semantic::Type::Generic("T".to_string(), vec![]));

        let result = builder.build(semantic::Type::Integer);
        match result {
            semantic::Type::Generic(name, _) => assert_eq!(name, "T"),
            _ => panic!("Expected Generic type"),
        }
    }
}

// =============================================================================
// 主函数：演示四层转换
// =============================================================================

fn main() {
    println!("========================================");
    println!("分层架构研究: Syntax → Semantic → Pattern → Domain");
    println!("========================================\n");

    // 示例1: 编译器领域演示
    println!("--- 示例1: 编译器领域 ---");
    let compiler = domain::compiler_domain::CompilerPipeline::new();
    match compiler.compile("fn example() -> i32 { 42 }") {
        Ok(output) => println!("编译成功: {:?}\n", output),
        Err(e) => println!("编译失败: {}\n", e),
    }

    // 示例2: 数据处理领域演示
    println!("--- 示例2: 数据处理领域 ---");
    let data_pipeline = domain::data_domain::DataPipeline::<i32>::new()
        .validate(|x| *x >= 0)
        .validate(|x| *x <= 100)
        .transform(|x| x * x)
        .transform(|x| x / 2);

    match data_pipeline.process(10) {
        Ok(result) => println!("数据处理结果: {} (10^2 / 2 = 50)\n", result),
        Err(e) => println!("处理失败: {}\n", e),
    }

    // 示例3: 模式组合演示
    println!("--- 示例3: 模式组合 ---");
    use crate::pattern::Pattern;

    let parse = pattern::TransformPattern::new(|s: &str| s.parse::<i32>().unwrap_or(0));
    let process = pattern::TransformPattern::new(|x: i32| x * 2 + 1);
    let format = pattern::TransformPattern::new(|x: i32| format!("Result: {}", x));

    let workflow = parse.compose(process).compose(format);
    let result = workflow.apply("21");
    println!("模式组合结果: {} (parse '21' -> 21 * 2 + 1 = 43)\n", result);

    // 示例4: 类型约束演示
    println!("--- 示例4: 类型约束系统 ---");
    let numeric = semantic::Type::Integer;
    let generic = semantic::Type::Generic(
        "T".to_string(),
        vec![
            semantic::Constraint::Addable,
            semantic::Constraint::Display,
        ],
    );

    println!("数值类型满足Addable约束: {}",
        semantic::SemanticAnalyzer::check_constraints(&numeric, &[semantic::Constraint::Addable]));
    println!("泛型类型: {:?}", generic);

    println!("\n========================================");
    println!("研究完成: 四层转换机制验证成功");
    println!("========================================");
}
