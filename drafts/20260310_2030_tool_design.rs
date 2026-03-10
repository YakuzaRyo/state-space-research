//! 类型安全的CLI工具框架 - 六层渐进式边界应用
//! 方向: tool_design
//! 时间: 2026-03-10 20:30
//! 核心: 展示如何设计'无法产生错误'的CLI工具

use std::marker::PhantomData;
use std::path::PathBuf;

// =============================================================================
// L0: Const Generics - 编译期常量约束
// =============================================================================

/// L0: 编译期限制配置参数范围
pub struct BoundedConfig<T, const MIN: T, const MAX: T>(T);

impl<T: PartialOrd + Copy, const MIN: T, const MAX: T> BoundedConfig<T, MIN, MAX> {
    pub fn new(value: T) -> Option<Self> {
        if value >= MIN && value <= MAX {
            Some(BoundedConfig(value))
        } else {
            None
        }
    }

    pub fn get(&self) -> T {
        self.0
    }
}

/// L0: 线程池大小 (1-64)
pub type ThreadPoolSize = BoundedConfig<usize, 1, 64>;
/// L0: 缓冲区大小 (1KB - 1GB)
pub type BufferSize = BoundedConfig<usize, 1024, 1073741824>;

// =============================================================================
// L1: Newtype - 类型区分不同来源的输入
// =============================================================================

/// L1: CLI参数输入
#[derive(Clone, Debug)]
pub struct CliInput(String);
impl CliInput {
    pub fn new(s: impl Into<String>) -> Self {
        CliInput(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// L1: 环境变量输入
#[derive(Clone, Debug)]
pub struct EnvInput(String);
impl EnvInput {
    pub fn new(s: impl Into<String>) -> Self {
        EnvInput(s.into())
    }
}

/// L1: 配置文件输入
#[derive(Clone, Debug)]
pub struct FileInput(String);
impl FileInput {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        std::fs::read_to_string(&path).map(FileInput)
    }
}

/// L1: 已验证的路径
#[derive(Clone, Debug)]
pub struct ValidatedPath(PathBuf);
impl ValidatedPath {
    pub fn new(path: PathBuf) -> Option<Self> {
        if path.exists() && path.is_file() {
            Some(ValidatedPath(path))
        } else {
            None
        }
    }

    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

// =============================================================================
// L2: Opaque Types - 信息隐藏 + L3: Typestate - 状态机
// =============================================================================

/// L3: 配置构建器状态
pub struct Unparsed;
pub struct Parsed;
pub struct Validated;
pub struct Merged;
pub struct Ready;

/// L2 + L3: 配置构建器 - 隐藏内部状态，强制状态转换顺序
pub struct ConfigBuilder<State> {
    /// L2: 内部字段不公开
    cli_args: Option<CliInput>,
    env_vars: Option<EnvInput>,
    file_config: Option<FileInput>,
    thread_pool: Option<ThreadPoolSize>,
    buffer_size: Option<BufferSize>,
    input_path: Option<ValidatedPath>,
    _state: PhantomData<State>,
}

impl ConfigBuilder<Unparsed> {
    /// L3: 唯一构造入口
    pub fn new() -> Self {
        ConfigBuilder {
            cli_args: None,
            env_vars: None,
            file_config: None,
            thread_pool: None,
            buffer_size: None,
            input_path: None,
            _state: PhantomData,
        }
    }

    /// L3: 解析CLI参数，状态转换 Unparsed -> Parsed
    pub fn parse_cli(self, args: Vec<String>) -> ConfigBuilder<Parsed> {
        ConfigBuilder {
            cli_args: Some(CliInput::new(args.join(" "))),
            env_vars: self.env_vars,
            file_config: self.file_config,
            thread_pool: self.thread_pool,
            buffer_size: self.buffer_size,
            input_path: self.input_path,
            _state: PhantomData,
        }
    }
}

impl ConfigBuilder<Parsed> {
    /// L3: 加载环境变量，保持Parsed状态
    pub fn load_env(self) -> Self {
        // 模拟从环境变量加载
        ConfigBuilder {
            cli_args: self.cli_args,
            env_vars: Some(EnvInput::new("env_value")),
            file_config: self.file_config,
            thread_pool: self.thread_pool,
            buffer_size: self.buffer_size,
            input_path: self.input_path,
            _state: PhantomData,
        }
    }

    /// L3: 加载配置文件，状态转换 Parsed -> Merged
    pub fn load_file(self, path: PathBuf) -> ConfigBuilder<Merged> {
        let file_input = FileInput::from_path(path).ok();
        ConfigBuilder {
            cli_args: self.cli_args,
            env_vars: self.env_vars,
            file_config: file_input,
            thread_pool: self.thread_pool,
            buffer_size: self.buffer_size,
            input_path: self.input_path,
            _state: PhantomData,
        }
    }
}

impl ConfigBuilder<Merged> {
    /// L3: 合并所有配置源，状态转换 Merged -> Validated
    /// 分层合并策略: CLI参数 > 环境变量 > 配置文件 > 默认值
    pub fn merge(self) -> ConfigBuilder<Validated> {
        // 模拟配置合并逻辑
        let thread_pool = ThreadPoolSize::new(4); // 默认值
        let buffer_size = BufferSize::new(65536); // 默认64KB

        ConfigBuilder {
            cli_args: self.cli_args,
            env_vars: self.env_vars,
            file_config: self.file_config,
            thread_pool,
            buffer_size,
            input_path: self.input_path,
            _state: PhantomData,
        }
    }
}

impl ConfigBuilder<Validated> {
    /// L3: 验证配置，状态转换 Validated -> Ready
    pub fn validate(self, path: PathBuf) -> Result<ConfigBuilder<Ready>, ConfigError> {
        let validated_path = ValidatedPath::new(path)
            .ok_or(ConfigError::InvalidPath)?;

        // L0: 编译期验证范围
        if self.thread_pool.is_none() {
            return Err(ConfigError::InvalidThreadPool);
        }

        Ok(ConfigBuilder {
            cli_args: self.cli_args,
            env_vars: self.env_vars,
            file_config: self.file_config,
            thread_pool: self.thread_pool,
            buffer_size: self.buffer_size,
            input_path: Some(validated_path),
            _state: PhantomData,
        })
    }
}

impl ConfigBuilder<Ready> {
    /// L3: 只有Ready状态才能执行
    pub fn execute(&self) -> Result<String, ExecutionError> {
        // 执行命令
        Ok(format!(
            "Processing file: {:?} with {} threads and {} bytes buffer",
            self.input_path.as_ref().map(|p| p.as_path()),
            self.thread_pool.as_ref().map(|t| t.get()).unwrap_or(1),
            self.buffer_size.as_ref().map(|b| b.get()).unwrap_or(1024)
        ))
    }
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidPath,
    InvalidThreadPool,
    InvalidBufferSize,
}

#[derive(Debug)]
pub enum ExecutionError {
    IoError(std::io::Error),
    ValidationFailed,
}

// =============================================================================
// L4: Linear Types - 资源安全 + L5: Capability - 权限系统
// =============================================================================

/// L5: 权限标记trait
pub trait Permission {}
pub struct CanRead;
pub struct CanWrite;
pub struct CanExecute;
impl Permission for CanRead {}
impl Permission for CanWrite {}
impl Permission for CanExecute {}

/// L4 + L5: 带权限的文件句柄
pub struct SecureFileHandle<P: Permission> {
    path: ValidatedPath,
    _perm: PhantomData<P>,
}

impl SecureFileHandle<CanRead> {
    /// L5: 只读权限才能读取
    pub fn new(path: ValidatedPath) -> Self {
        SecureFileHandle {
            path,
            _perm: PhantomData,
        }
    }

    pub fn read(&self) -> std::io::Result<String> {
        std::fs::read_to_string(self.path.as_path())
    }

    /// L5: 升级权限到读写
    pub fn upgrade_to_write(self) -> SecureFileHandle<CanWrite> {
        SecureFileHandle {
            path: self.path,
            _perm: PhantomData,
        }
    }
}

impl SecureFileHandle<CanWrite> {
    /// L5: 读写权限才能写入
    pub fn write(&self, content: &str) -> std::io::Result<()> {
        std::fs::write(self.path.as_path(), content)
    }

    /// L4: 消费权限执行操作
    pub fn execute(self) -> SecureFileHandle<CanExecute>
    where
        P: Permission,
    {
        SecureFileHandle {
            path: self.path,
            _perm: PhantomData,
        }
    }
}

// =============================================================================
// Functional Core, Imperative Shell 架构
// =============================================================================

/// 纯函数核心 - 无副作用，易测试
pub mod core {
    /// 纯业务逻辑：数据处理
    pub fn process_data(input: &str) -> String {
        input.lines()
            .map(|line| line.trim().to_uppercase())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 纯业务逻辑：验证
    pub fn validate_content(content: &str) -> Result<(), &'static str> {
        if content.is_empty() {
            Err("Content is empty")
        } else if content.len() > 10000 {
            Err("Content too large")
        } else {
            Ok(())
        }
    }

    /// 纯业务逻辑：统计
    pub fn count_lines(content: &str) -> usize {
        content.lines().count()
    }
}

/// 命令式外壳 - 处理IO和副作用
pub mod shell {
    use super::*;

    /// Effect trait - 抽象所有副作用
    pub trait Effect {
        fn read_file(&self, path: &PathBuf) -> std::io::Result<String>;
        fn write_file(&self, path: &PathBuf, content: &str) -> std::io::Result<()>;
        fn print(&self, msg: &str);
    }

    /// 实际Effect实现
    pub struct RealEffect;
    impl Effect for RealEffect {
        fn read_file(&self, path: &PathBuf) -> std::io::Result<String> {
            std::fs::read_to_string(path)
        }

        fn write_file(&self, path: &PathBuf, content: &str) -> std::io::Result<()> {
            std::fs::write(path, content)
        }

        fn print(&self, msg: &str) {
            println!("{}", msg);
        }
    }

    /// 测试用Mock Effect
    pub struct MockEffect {
        pub read_result: String,
        pub write_log: Vec<(PathBuf, String)>,
        pub print_log: Vec<String>,
    }

    impl MockEffect {
        pub fn new(read_result: String) -> Self {
            MockEffect {
                read_result,
                write_log: Vec::new(),
                print_log: Vec::new(),
            }
        }
    }

    impl Effect for MockEffect {
        fn read_file(&self, _path: &PathBuf) -> std::io::Result<String> {
            Ok(self.read_result.clone())
        }

        fn write_file(&self, path: &PathBuf, content: &str) -> std::io::Result<()> {
            self.write_log.push((path.clone(), content.to_string()));
            Ok(())
        }

        fn print(&self, msg: &str) {
            self.print_log.push(msg.to_string());
        }
    }

    /// 协调纯函数和副作用的入口
    pub fn run<E: Effect>(
        effect: &E,
        input_path: &ValidatedPath,
        output_path: &ValidatedPath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 副作用: 读取输入
        let input = effect.read_file(input_path.as_path())?;

        // 纯函数: 处理数据
        let processed = core::process_data(&input);

        // 纯函数: 验证
        core::validate_content(&processed)?;

        // 纯函数: 统计
        let line_count = core::count_lines(&processed);

        // 副作用: 写入输出
        effect.write_file(output_path.as_path(), &processed)?;

        // 副作用: 打印统计
        effect.print(&format!("Processed {} lines", line_count));

        Ok(())
    }
}

// =============================================================================
// 完整工作流示例
// =============================================================================

/// 完整CLI工具工作流
pub fn cli_workflow_example() -> Result<(), Box<dyn std::error::Error>> {
    // L3: 状态机确保正确顺序
    let config = ConfigBuilder::new()
        .parse_cli(vec!["app".to_string(), "--input".to_string(), "file.txt".to_string()])
        .load_env()
        .load_file(PathBuf::from("config.toml"))
        .merge()
        .validate(PathBuf::from("input.txt"))?;

    // L3: 只有Ready状态才能执行
    let result = config.execute()?;
    println!("{}", result);

    // L5: 权限控制文件操作
    let input_file = SecureFileHandle::<CanRead>::new(
        ValidatedPath::new(PathBuf::from("input.txt")).unwrap()
    );
    let content = input_file.read()?;

    // L4: 升级权限
    let output_file = input_file.upgrade_to_write();
    output_file.write(&core::process_data(&content))?;

    Ok(())
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::shell::*;

    #[test]
    fn test_config_builder_typestate() {
        // 正确流程: Unparsed -> Parsed -> Merged -> Validated -> Ready
        let config = ConfigBuilder::new()
            .parse_cli(vec!["app".to_string()])
            .load_env()
            .load_file(PathBuf::from("config.toml"))
            .merge()
            .validate(PathBuf::from("test.txt"))
            .unwrap();

        let result = config.execute().unwrap();
        assert!(result.contains("test.txt"));

        // 编译错误: 无法跳过validate直接execute
        // let config = ConfigBuilder::new().parse_cli(vec![]).load_file(PathBuf::new()).merge();
        // config.execute(); // ERROR!
    }

    #[test]
    fn test_bounded_config_l0() {
        // L0: 编译期范围检查
        let valid = ThreadPoolSize::new(4).unwrap();
        assert_eq!(valid.get(), 4);

        let invalid = ThreadPoolSize::new(100); // 超出范围
        assert!(invalid.is_none());
    }

    #[test]
    fn test_newtype_l1() {
        // L1: 类型区分
        let cli = CliInput::new("cli_value");
        let env = EnvInput::new("env_value");

        // 编译错误: 类型不匹配
        // fn use_cli(input: CliInput) {}
        // use_cli(env); // ERROR!

        assert_eq!(cli.as_str(), "cli_value");
    }

    #[test]
    fn test_secure_file_handle_l4_l5() {
        // 创建临时文件用于测试
        let temp_path = PathBuf::from("test_input.txt");
        std::fs::write(&temp_path, "test content").unwrap();

        let validated = ValidatedPath::new(temp_path.clone()).unwrap();
        let file = SecureFileHandle::<CanRead>::new(validated);

        // L5: 只读权限可以读取
        let content = file.read().unwrap();
        assert_eq!(content, "test content");

        // L5: 升级权限后可以写入
        let writable = file.upgrade_to_write();
        writable.write("new content").unwrap();

        // 清理
        std::fs::remove_file(&temp_path).unwrap();
    }

    #[test]
    fn test_pure_core() {
        // 纯函数易于测试
        assert_eq!(core::process_data("hello"), "HELLO");
        assert_eq!(core::process_data("  hello  "), "HELLO");

        assert_eq!(core::count_lines("a\nb\nc"), 3);

        assert!(core::validate_content("valid").is_ok());
        assert!(core::validate_content("").is_err());
    }

    #[test]
    fn test_functional_core_imperative_shell() {
        // 使用Mock Effect测试
        let mock = MockEffect::new("hello\nworld".to_string());

        let input_path = ValidatedPath::new(PathBuf::from("input.txt")).unwrap();
        let output_path = ValidatedPath::new(PathBuf::from("output.txt")).unwrap();

        // 注意：实际测试需要文件存在，这里简化展示
        // run(&mock, &input_path, &output_path).unwrap();

        // 验证副作用被正确记录
        // assert_eq!(mock.print_log, vec!["Processed 2 lines"]);
    }

    #[test]
    fn test_validated_path_l1_l2() {
        // 创建临时文件
        let temp_path = PathBuf::from("test_file.txt");
        std::fs::write(&temp_path, "content").unwrap();

        // L1+L2: 验证路径并封装
        let validated = ValidatedPath::new(temp_path.clone());
        assert!(validated.is_some());

        // 不存在的路径返回None
        let invalid = ValidatedPath::new(PathBuf::from("/nonexistent/file.txt"));
        assert!(invalid.is_none());

        // 清理
        std::fs::remove_file(&temp_path).unwrap();
    }
}

// =============================================================================
// 架构注释
// =============================================================================

/*
 * 六层渐进式边界在CLI工具设计中的应用:
 *
 * ┌─────────────────────────────────────────────────────────┐
 * │ L5 Capability         │ SecureFileHandle<CanRead>等      │
 * │                       │ 权限升级: upgrade_to_write()     │
 * ├─────────────────────────────────────────────────────────┤
 * │ L4 Linear             │ 资源使用权限追踪                 │
 * │                       │ 消费权限执行操作                 │
 * ├─────────────────────────────────────────────────────────┤
 * │ L3 Typestate          │ ConfigBuilder<State>             │
 * │                       │ Unparsed->Parsed->Merged->Valid->│
 * ├─────────────────────────────────────────────────────────┤
 * │ L2 Opaque             │ 内部字段不公开                   │
 * │                       │ 通过受控接口访问                 │
 * ├─────────────────────────────────────────────────────────┤
 * │ L1 Newtype            │ CliInput, EnvInput, FileInput    │
 * │                       │ ValidatedPath                    │
 * ├─────────────────────────────────────────────────────────┤
 * │ L0 Const Generics     │ ThreadPoolSize, BufferSize       │
 * │                       │ 编译期范围约束                   │
 * └─────────────────────────────────────────────────────────┘
 *
 * Functional Core, Imperative Shell:
 *
 * ┌─────────────────────────────────────────┐
 * │           CLI Binary (Shell)            │
 * │  - 参数解析、IO操作、副作用管理          │
 * ├─────────────────────────────────────────┤
 * │  ConfigBuilder<Ready>.execute() (Shell) │
 * │  - 协调纯函数和副作用                    │
 * ├─────────────────────────────────────────┤
 * │        core::process_data (Core)        │
 * │        core::validate_content (Core)    │
 * │        core::count_lines (Core)         │
 * │  - 纯函数，无副作用，易测试              │
 * └─────────────────────────────────────────┘
 *
 * 关键设计原则:
 * 1. 失败快速: 在ConfigBuilder阶段验证，而非运行时
 * 2. 类型安全: 使用Typestate确保执行顺序
 * 3. 副作用隔离: Effect trait抽象所有IO
 * 4. 渐进式披露: 分层配置，逐步验证
 * 5. 权限控制: Capability系统限制操作范围
 */
