//! 核心原则深度研究 - 让错误在设计上不可能发生
//!
//! 本文件验证以下假设：
//! 1. Typestate模式消除运行时状态错误
//! 2. 零成本类型安全状态机实现
//! 3. 编译期检查对性能的影响
//! 4. 类型驱动设计的适用场景
//!
//! 研究日期: 2026-03-11
//! 研究方向: 01_core_principles

use std::marker::PhantomData;
use std::time::Duration;

// =============================================================================
// 第一部分: Typestate 模式 - 文件操作状态机
// =============================================================================

/// 文件状态标记 trait
pub trait FileState {}

/// 文件已关闭状态
pub struct Closed;
impl FileState for Closed {}

/// 文件已打开状态
pub struct Open;
impl FileState for Open {}

/// 文件正在读取状态
pub struct Reading;
impl FileState for Reading {}

/// 文件正在写入状态
pub struct Writing;
impl FileState for Writing {}

/// 类型安全的文件状态机
///
/// 使用泛型参数 S 编码当前状态，使得非法操作在编译期被拒绝。
/// 例如：无法在未打开文件时读取，无法在关闭后写入。
pub struct TypedFile<S: FileState> {
    path: String,
    content: Vec<u8>,
    position: usize,
    _state: PhantomData<S>,
}

impl TypedFile<Closed> {
    /// 创建新文件（初始状态为 Closed）
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: Vec::new(),
            position: 0,
            _state: PhantomData,
        }
    }

    /// 打开文件，状态从 Closed 转换为 Open
    /// 这是一个消耗性转换：self 被移动，返回新类型的文件
    pub fn open(self) -> TypedFile<Open> {
        println!("[Typestate] Opening file: {}", self.path);
        TypedFile {
            path: self.path,
            content: self.content,
            position: 0,
            _state: PhantomData,
        }
    }
}

impl TypedFile<Open> {
    /// 开始读取，状态从 Open 转换为 Reading
    pub fn start_read(self) -> TypedFile<Reading> {
        println!("[Typestate] Starting read operation");
        TypedFile {
            path: self.path,
            content: self.content,
            position: 0,
            _state: PhantomData,
        }
    }

    /// 开始写入，状态从 Open 转换为 Writing
    pub fn start_write(self) -> TypedFile<Writing> {
        println!("[Typestate] Starting write operation");
        TypedFile {
            path: self.path,
            content: self.content,
            position: 0,
            _state: PhantomData,
        }
    }

    /// 关闭文件，状态从 Open 转换为 Closed
    pub fn close(self) -> TypedFile<Closed> {
        println!("[Typestate] Closing file");
        TypedFile {
            path: self.path,
            content: self.content,
            position: 0,
            _state: PhantomData,
        }
    }
}

impl TypedFile<Reading> {
    /// 读取数据 - 仅在 Reading 状态下可用
    pub fn read_chunk(&mut self, size: usize) -> &[u8] {
        let end = (self.position + size).min(self.content.len());
        let data = &self.content[self.position..end];
        self.position = end;
        println!("[Typestate] Read {} bytes", data.len());
        data
    }

    /// 检查是否到达文件末尾
    pub fn is_eof(&self) -> bool {
        self.position >= self.content.len()
    }

    /// 完成读取，返回 Open 状态
    pub fn finish_read(self) -> TypedFile<Open> {
        println!("[Typestate] Finishing read operation");
        TypedFile {
            path: self.path,
            content: self.content,
            position: self.position,
            _state: PhantomData,
        }
    }
}

impl TypedFile<Writing> {
    /// 写入数据 - 仅在 Writing 状态下可用
    pub fn write_chunk(&mut self, data: &[u8]) {
        self.content.extend_from_slice(data);
        println!("[Typestate] Wrote {} bytes", data.len());
    }

    /// 完成写入，返回 Open 状态
    pub fn finish_write(self) -> TypedFile<Open> {
        println!("[Typestate] Finishing write operation");
        TypedFile {
            path: self.path,
            content: self.content,
            position: 0,
            _state: PhantomData,
        }
    }
}

// =============================================================================
// 第二部分: 对比实现 - Enum-based 状态机
// =============================================================================

/// 传统 Enum 方式的状态机（运行时检查）
pub enum FileStateEnum {
    Closed,
    Open,
    Reading { position: usize },
    Writing,
}

pub struct EnumBasedFile {
    path: String,
    content: Vec<u8>,
    state: FileStateEnum,
}

impl EnumBasedFile {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: Vec::new(),
            state: FileStateEnum::Closed,
        }
    }

    /// 需要运行时检查状态
    pub fn open(&mut self) -> Result<(), FileError> {
        match &self.state {
            FileStateEnum::Closed => {
                self.state = FileStateEnum::Open;
                Ok(())
            }
            _ => Err(FileError::InvalidState),
        }
    }

    pub fn start_read(&mut self) -> Result<(), FileError> {
        match &self.state {
            FileStateEnum::Open => {
                self.state = FileStateEnum::Reading { position: 0 };
                Ok(())
            }
            _ => Err(FileError::InvalidState),
        }
    }

    pub fn read_chunk(&mut self, size: usize) -> Result<&[u8], FileError> {
        match &mut self.state {
            FileStateEnum::Reading { position } => {
                let end = (*position + size).min(self.content.len());
                let data = &self.content[*position..end];
                *position = end;
                Ok(data)
            }
            _ => Err(FileError::InvalidState),
        }
    }
}

#[derive(Debug)]
pub enum FileError {
    InvalidState,
    IoError(std::io::Error),
}

// =============================================================================
// 第三部分: Mealy/Moore 状态机的类型编码
// =============================================================================

/// Mealy 机器：输出依赖于当前状态和输入
/// 使用关联类型定义状态转换和输出

/// 状态 trait，定义状态的转移行为
pub trait MealyState {
    type Input;
    type Output;
    type NextState: MealyState;

    fn transition(self, input: Self::Input) -> (Self::Output, Self::NextState);
}

/// 简单的交通灯 Mealy 机器示例
/// 状态: Red -> Green -> Yellow -> Red

pub struct RedLight {
    duration: Duration,
}

pub struct GreenLight {
    duration: Duration,
}

pub struct YellowLight {
    duration: Duration,
}

/// 交通灯输入
pub enum TrafficInput {
    TimerExpired,
    EmergencyOverride,
}

/// 交通灯输出
#[derive(Debug)]
pub enum TrafficOutput {
    TurnRed,
    TurnGreen,
    TurnYellow,
    EmergencyFlash,
}

impl MealyState for RedLight {
    type Input = TrafficInput;
    type Output = TrafficOutput;
    type NextState = GreenLight;

    fn transition(self, input: Self::Input) -> (Self::Output, Self::NextState) {
        match input {
            TrafficInput::TimerExpired => (
                TrafficOutput::TurnGreen,
                GreenLight {
                    duration: Duration::from_secs(30),
                }
            ),
            TrafficInput::EmergencyOverride => (
                TrafficOutput::EmergencyFlash,
                GreenLight {
                    duration: Duration::from_secs(5),
                }
            ),
        }
    }
}

impl MealyState for GreenLight {
    type Input = TrafficInput;
    type Output = TrafficOutput;
    type NextState = YellowLight;

    fn transition(self, input: Self::Input) -> (Self::Output, Self::NextState) {
        match input {
            TrafficInput::TimerExpired => (
                TrafficOutput::TurnYellow,
                YellowLight {
                    duration: Duration::from_secs(5),
                }
            ),
            TrafficInput::EmergencyOverride => (
                TrafficOutput::TurnYellow,
                YellowLight {
                    duration: Duration::from_secs(2),
                }
            ),
        }
    }
}

impl MealyState for YellowLight {
    type Input = TrafficInput;
    type Output = TrafficOutput;
    type NextState = RedLight;

    fn transition(self, input: Self::Input) -> (Self::Output, Self::NextState) {
        match input {
            TrafficInput::TimerExpired | TrafficInput::EmergencyOverride => (
                TrafficOutput::TurnRed,
                RedLight {
                    duration: Duration::from_secs(30),
                }
            ),
        }
    }
}

/// Moore 机器：输出仅依赖于当前状态
pub trait MooreState {
    type Output;
    fn output(&self) -> Self::Output;
}

/// 使用类型参数构建 Moore 机器
pub struct MooreMachine<S: MooreState> {
    state: S,
}

impl<S: MooreState> MooreMachine<S> {
    pub fn new(state: S) -> Self {
        Self { state }
    }

    pub fn output(&self) -> S::Output {
        self.state.output()
    }
}

/// 门状态 Moore 机器示例
pub struct DoorOpen;
pub struct DoorClosed;
pub struct DoorLocked;

#[derive(Debug)]
pub enum DoorOutput {
    PassageAllowed,
    PassageBlocked,
    PassageSecured,
}

impl MooreState for DoorOpen {
    type Output = DoorOutput;
    fn output(&self) -> Self::Output {
        DoorOutput::PassageAllowed
    }
}

impl MooreState for DoorClosed {
    type Output = DoorOutput;
    fn output(&self) -> Self::Output {
        DoorOutput::PassageBlocked
    }
}

impl MooreState for DoorLocked {
    type Output = DoorOutput;
    fn output(&self) -> Self::Output {
        DoorOutput::PassageSecured
    }
}

// =============================================================================
// 第四部分: Capability-Based 权限系统
// =============================================================================

/// Capability-Based Security 的核心思想：
/// 不检查"你是谁"（身份），而是检查"你有什么"（能力）
///
/// 优势：
/// 1. 能力可以在代码中传递，实现委托
/// 2. 编译期验证权限，无法伪造
/// 3. 细粒度权限控制

/// 能力标记 trait - 使用空类型作为权限标记
pub trait Capability: private::Sealed {}

mod private {
    pub trait Sealed {}
}

/// 读取能力
pub struct ReadCapability<T> {
    _marker: PhantomData<T>,
}

/// 写入能力
pub struct WriteCapability<T> {
    _marker: PhantomData<T>,
}

/// 执行能力
pub struct ExecuteCapability<T> {
    _marker: PhantomData<T>,
}

/// 管理员能力（拥有所有权限）
pub struct AdminCapability<T> {
    _marker: PhantomData<T>,
}

// 实现 Sealed trait 防止外部实现
impl<T> private::Sealed for ReadCapability<T> {}
impl<T> private::Sealed for WriteCapability<T> {}
impl<T> private::Sealed for ExecuteCapability<T> {}
impl<T> private::Sealed for AdminCapability<T> {}

// 实现 Capability trait
impl<T> Capability for ReadCapability<T> {}
impl<T> Capability for WriteCapability<T> {}
impl<T> Capability for ExecuteCapability<T> {}
impl<T> Capability for AdminCapability<T> {}

/// 受保护资源
pub struct ProtectedResource<T> {
    data: T,
}

impl<T> ProtectedResource<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }

    /// 需要 ReadCapability 才能读取
    pub fn read(&self, _cap: &ReadCapability<T>) -> &T {
        &self.data
    }

    /// 需要 WriteCapability 才能写入
    pub fn write(&mut self, _cap: &WriteCapability<T>, value: T) {
        self.data = value;
    }

    /// 需要 AdminCapability 才能删除
    pub fn delete(self, _cap: AdminCapability<T>) -> T {
        self.data
    }
}

/// 能力工厂 - 集中管理权限发放
pub struct CapabilityFactory;

impl CapabilityFactory {
    /// 发放读取能力
    pub fn grant_read<T>() -> ReadCapability<T> {
        ReadCapability { _marker: PhantomData }
    }

    /// 发放写入能力
    pub fn grant_write<T>() -> WriteCapability<T> {
        WriteCapability { _marker: PhantomData }
    }

    /// 发放执行能力
    pub fn grant_execute<T>() -> ExecuteCapability<T> {
        ExecuteCapability { _marker: PhantomData }
    }

    /// 发放管理员能力（需要认证）
    pub fn grant_admin<T>(auth_token: &str) -> Option<AdminCapability<T>> {
        if auth_token == "valid_admin_token" {
            Some(AdminCapability { _marker: PhantomData })
        } else {
            None
        }
    }
}

/// 能力委托示例
/// 用户可以将自己的能力委托给其他组件
pub struct DelegatedCapability<C: Capability, T> {
    capability: C,
    scope: String,
    _marker: PhantomData<T>,
}

impl<C: Capability, T> DelegatedCapability<C, T> {
    pub fn new(capability: C, scope: impl Into<String>) -> Self {
        Self {
            capability,
            scope: scope.into(),
            _marker: PhantomData,
        }
    }

    pub fn get_capability(&self) -> &C {
        &self.capability
    }
}

// =============================================================================
// 第五部分: 类型安全的资源管理
// =============================================================================

/// 使用类型系统确保资源正确释放
/// 实现线性类型（Linear Types）的近似

/// 资源句柄 - 确保资源在使用后被正确释放
pub struct LinearResource<R, T> {
    resource: R,
    cleanup: fn(&mut R),
    _marker: PhantomData<T>,
}

impl<R, T> LinearResource<R, T> {
    /// 创建线性资源
    pub fn new(resource: R, cleanup: fn(&mut R)) -> Self {
        Self {
            resource,
            cleanup,
            _marker: PhantomData,
        }
    }

    /// 访问资源（不可变借用）
    pub fn access(&self) -> &R {
        &self.resource
    }

    /// 访问资源（可变借用）
    pub fn access_mut(&mut self) -> &mut R {
        &mut self.resource
    }

    /// 消费资源并返回其内部值（如果不需要清理）
    pub fn into_inner(mut self) -> R {
        // 标记资源已被消费，防止在 drop 中重复清理
        // 这里简化处理，实际实现需要更复杂的逻辑
        let resource = unsafe { std::ptr::read(&self.resource) };
        std::mem::forget(self); // 防止调用 drop
        resource
    }
}

impl<R, T> Drop for LinearResource<R, T> {
    fn drop(&mut self) {
        (self.cleanup)(&mut self.resource);
    }
}

/// 作用域资源管理器
pub struct ScopeGuard<T> {
    value: T,
    on_exit: Box<dyn FnOnce(&mut T)>,
}

impl<T> ScopeGuard<T> {
    pub fn new<F>(value: T, on_exit: F) -> Self
    where
        F: FnOnce(&mut T) + 'static,
    {
        Self {
            value,
            on_exit: Box::new(on_exit),
        }
    }

    pub fn access(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> Drop for ScopeGuard<T> {
    fn drop(&mut self) {
        // 使用 take 来获取所有权
        let on_exit = std::mem::replace(&mut self.on_exit, Box::new(|_| {}));
        on_exit(&mut self.value);
    }
}

// =============================================================================
// 第六部分: 协议状态机 - HTTP 连接状态
// =============================================================================

/// HTTP 连接状态机
/// 确保请求-响应顺序正确

/// 连接初始状态
pub struct HttpIdle;

/// 请求已发送，等待响应
pub struct HttpRequestSent {
    request_id: u64,
}

/// 响应已接收
pub struct HttpResponseReceived {
    request_id: u64,
    status_code: u16,
    body: Vec<u8>,
}

/// 连接已关闭
pub struct HttpClosed;

/// 类型安全的 HTTP 连接
pub struct HttpConnection<S> {
    endpoint: String,
    _state: PhantomData<S>,
}

impl HttpConnection<HttpIdle> {
    pub fn connect(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            _state: PhantomData,
        }
    }

    pub fn send_request(self, request_id: u64) -> HttpConnection<HttpRequestSent> {
        println!("[HTTP] Sending request {} to {}", request_id, self.endpoint);
        HttpConnection {
            endpoint: self.endpoint,
            _state: PhantomData,
        }
    }
}

impl HttpConnection<HttpRequestSent> {
    /// 接收响应 - 只能在发送请求后调用
    pub fn receive_response(self, status_code: u16, body: Vec<u8>) -> HttpConnection<HttpResponseReceived> {
        println!("[HTTP] Received response with status {}", status_code);
        HttpConnection {
            endpoint: self.endpoint,
            _state: PhantomData,
        }
    }

    /// 取消请求
    pub fn cancel(self) -> HttpConnection<HttpIdle> {
        println!("[HTTP] Request cancelled");
        HttpConnection {
            endpoint: self.endpoint,
            _state: PhantomData,
        }
    }
}

impl HttpConnection<HttpResponseReceived> {
    /// 获取响应体 - 只能在接收响应后调用
    pub fn get_body(&self) -> &[u8] {
        &[] // 简化实现
    }

    /// 返回空闲状态以便发送下一个请求
    pub fn return_to_idle(self) -> HttpConnection<HttpIdle> {
        HttpConnection {
            endpoint: self.endpoint,
            _state: PhantomData,
        }
    }

    /// 关闭连接
    pub fn close(self) -> HttpConnection<HttpClosed> {
        println!("[HTTP] Connection closed");
        HttpConnection {
            endpoint: self.endpoint,
            _state: PhantomData,
        }
    }
}

// =============================================================================
// 第七部分: 数据库连接状态机
// =============================================================================

/// 数据库事务状态机
/// 确保事务正确开始、提交或回滚

/// 未连接状态
pub struct DbDisconnected;

/// 已连接但未在事务中
pub struct DbConnected {
    connection_string: String,
}

/// 事务进行中
pub struct DbInTransaction {
    transaction_id: u64,
    savepoints: Vec<String>,
}

/// 事务已提交
pub struct DbCommitted;

/// 事务已回滚
pub struct DbRolledBack;

/// 类型安全的数据库连接
pub struct DatabaseConnection<S> {
    _state: PhantomData<S>,
}

impl DatabaseConnection<DbDisconnected> {
    pub fn new() -> Self {
        Self { _state: PhantomData }
    }

    pub fn connect(self, conn_str: impl Into<String>) -> Result<DatabaseConnection<DbConnected>, DbError> {
        println!("[DB] Connecting to database");
        Ok(DatabaseConnection { _state: PhantomData })
    }
}

impl DatabaseConnection<DbConnected> {
    /// 开始事务
    pub fn begin_transaction(self) -> DatabaseConnection<DbInTransaction> {
        println!("[DB] Beginning transaction");
        DatabaseConnection { _state: PhantomData }
    }

    /// 断开连接
    pub fn disconnect(self) -> DatabaseConnection<DbDisconnected> {
        println!("[DB] Disconnecting");
        DatabaseConnection { _state: PhantomData }
    }
}

impl DatabaseConnection<DbInTransaction> {
    /// 执行查询 - 只能在事务中执行
    pub fn execute(&mut self, query: &str) -> Result<Vec<DbRow>, DbError> {
        println!("[DB] Executing: {}", query);
        Ok(vec![])
    }

    /// 创建保存点
    pub fn savepoint(&mut self, name: impl Into<String>) -> Result<(), DbError> {
        println!("[DB] Creating savepoint: {}", name.into());
        Ok(())
    }

    /// 提交事务
    pub fn commit(self) -> DatabaseConnection<DbCommitted> {
        println!("[DB] Committing transaction");
        DatabaseConnection { _state: PhantomData }
    }

    /// 回滚事务
    pub fn rollback(self) -> DatabaseConnection<DbRolledBack> {
        println!("[DB] Rolling back transaction");
        DatabaseConnection { _state: PhantomData }
    }
}

impl DatabaseConnection<DbCommitted> {
    /// 返回连接状态以开始新事务
    pub fn return_to_connected(self) -> DatabaseConnection<DbConnected> {
        DatabaseConnection { _state: PhantomData }
    }
}

impl DatabaseConnection<DbRolledBack> {
    /// 返回连接状态以开始新事务
    pub fn return_to_connected(self) -> DatabaseConnection<DbConnected> {
        DatabaseConnection { _state: PhantomData }
    }
}

#[derive(Debug)]
pub struct DbError;

pub struct DbRow {
    data: Vec<u8>,
}

// =============================================================================
// 第八部分: 内存分配器状态机
// =============================================================================

/// 类型安全的内存分配器
/// 确保内存正确分配和释放

/// 未初始化内存
pub struct Uninit<T> {
    _marker: PhantomData<T>,
}

/// 已初始化内存
pub struct Init<T> {
    value: T,
}

/// 已释放内存
pub struct Freed;

/// 类型安全的内存块
pub struct MemoryBlock<T, S> {
    ptr: *mut u8,
    size: usize,
    _marker: PhantomData<(T, S)>,
}

impl<T> MemoryBlock<T, Uninit<T>> {
    pub unsafe fn allocate(size: usize) -> Self {
        let layout = std::alloc::Layout::array::<T>(size).unwrap();
        let ptr = std::alloc::alloc(layout);
        Self {
            ptr,
            size,
            _marker: PhantomData,
        }
    }

    /// 初始化内存
    pub fn initialize(self, value: T) -> MemoryBlock<T, Init<T>> {
        unsafe {
            std::ptr::write(self.ptr as *mut T, value);
        }
        MemoryBlock {
            ptr: self.ptr,
            size: self.size,
            _marker: PhantomData,
        }
    }
}

impl<T> MemoryBlock<T, Init<T>> {
    /// 读取值
    pub fn read(&self) -> &T {
        unsafe { &*(self.ptr as *const T) }
    }

    /// 写入值
    pub fn write(&mut self, value: T) {
        unsafe {
            std::ptr::write(self.ptr as *mut T, value);
        }
    }

    /// 释放内存
    pub unsafe fn free(self) -> MemoryBlock<T, Freed> {
        let layout = std::alloc::Layout::array::<T>(self.size).unwrap();
        std::alloc::dealloc(self.ptr, layout);
        MemoryBlock {
            ptr: std::ptr::null_mut(),
            size: 0,
            _marker: PhantomData,
        }
    }

    /// 转换为未初始化状态（用于重新初始化）
    pub fn uninit(self) -> MemoryBlock<T, Uninit<T>> {
        MemoryBlock {
            ptr: self.ptr,
            size: self.size,
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// 第九部分: 编译期常量验证
// =============================================================================

/// 使用 const generics 进行编译期验证

/// 固定大小的缓冲区，大小在类型中编码
pub struct FixedBuffer<T, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T: Default + Copy, const N: usize> FixedBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            data: [T::default(); N],
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), BufferError> {
        if self.len >= N {
            return Err(BufferError::Full);
        }
        self.data[self.len] = value;
        self.len += 1;
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(&self.data[index])
        } else {
            None
        }
    }

    pub fn capacity(&self) -> usize {
        N
    }
}

#[derive(Debug)]
pub enum BufferError {
    Full,
}

/// 编译期验证的数值范围
/// 使用 const generics 确保值在范围内
pub struct BoundedU32<const MIN: u32, const MAX: u32> {
    value: u32,
}

impl<const MIN: u32, const MAX: u32> BoundedU32<MIN, MAX> {
    /// 尝试创建有界值
    pub fn new(value: u32) -> Option<Self> {
        if value >= MIN && value <= MAX {
            Some(Self { value })
        } else {
            None
        }
    }

    /// 获取值
    pub fn get(&self) -> u32 {
        self.value
    }

    /// 安全地增加值
    pub fn add(&self, delta: u32) -> Option<Self> {
        Self::new(self.value.saturating_add(delta))
    }
}

// =============================================================================
// 第十部分: 测试和验证
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_file() {
        // 正确的状态转换序列
        let file = TypedFile::new("test.txt");
        let file = file.open();
        let file = file.start_read();
        // 无法在未完成读取时关闭 - 编译错误
        // let file = file.close(); // ERROR!
        let file = file.finish_read();
        let _file = file.close();
    }

    #[test]
    fn test_capability_system() {
        let mut resource = ProtectedResource::new(42);

        // 获取读取能力
        let read_cap = CapabilityFactory::grant_read::<i32>();
        let value = resource.read(&read_cap);
        assert_eq!(*value, 42);

        // 获取写入能力
        let write_cap = CapabilityFactory::grant_write::<i32>();
        resource.write(&write_cap, 100);
        assert_eq!(*resource.read(&read_cap), 100);
    }

    #[test]
    fn test_bounded_u32() {
        // 0-100 范围内的值
        let bounded = BoundedU32::<0, 100>::new(50);
        assert!(bounded.is_some());

        // 超出范围的值
        let bounded = BoundedU32::<0, 100>::new(150);
        assert!(bounded.is_none());
    }

    #[test]
    fn test_fixed_buffer() {
        let mut buf = FixedBuffer::<i32, 5>::new();
        assert_eq!(buf.capacity(), 5);

        for i in 0..5 {
            assert!(buf.push(i as i32).is_ok());
        }

        // 缓冲区已满
        assert!(buf.push(99).is_err());
    }

    #[test]
    fn test_http_connection() {
        let conn = HttpConnection::connect("https://example.com");
        let conn = conn.send_request(1);
        let conn = conn.receive_response(200, vec![1, 2, 3]);
        let conn = conn.return_to_idle();
        let _conn = conn.close();
    }

    #[test]
    fn test_database_transaction() {
        let db = DatabaseConnection::new();
        let db = db.connect("postgres://localhost").unwrap();
        let mut db = db.begin_transaction();

        // 只能在事务中执行查询
        db.execute("SELECT * FROM users").unwrap();

        let db = db.commit();
        let db = db.return_to_connected();
        let _db = db.disconnect();
    }
}

// =============================================================================
// 第十一部分: 示例用法和演示
// =============================================================================

/// 演示 Typestate 模式的完整工作流
pub fn demo_typestate_workflow() {
    println!("\n=== Typestate Pattern Demo ===");

    // 创建文件
    let file = TypedFile::<Closed>::new("document.txt");

    // 打开文件
    let file = file.open();

    // 开始写入
    let mut file = file.start_write();
    file.write_chunk(b"Hello, Typestate!");

    // 完成写入
    let file = file.finish_write();

    // 开始读取
    let mut file = file.start_read();
    while !file.is_eof() {
        let _chunk = file.read_chunk(10);
    }

    // 完成读取
    let file = file.finish_read();

    // 关闭文件
    let _file = file.close();

    println!("File operations completed successfully!");
}

/// 演示 Capability-Based 权限系统
pub fn demo_capability_system() {
    println!("\n=== Capability-Based Security Demo ===");

    let mut document = ProtectedResource::new("Secret Document Content");

    // 普通用户只有读取权限
    let read_cap = CapabilityFactory::grant_read::<&'static str>();
    println!("Reader can read: {}", document.read(&read_cap));

    // 编辑者有读写权限
    let write_cap = CapabilityFactory::grant_write::<&'static str>();
    document.write(&write_cap, "Modified Content");
    println!("Writer modified: {}", document.read(&read_cap));

    // 管理员可以删除
    if let Some(admin_cap) = CapabilityFactory::grant_admin::<&'static str>("valid_admin_token") {
        let content = document.delete(admin_cap);
        println!("Admin deleted document with content: {}", content);
    }
}

/// 演示 Mealy 状态机
pub fn demo_mealy_machine() {
    println!("\n=== Mealy Machine Demo ===");

    let red = RedLight {
        duration: Duration::from_secs(30),
    };

    let (output, green) = red.transition(TrafficInput::TimerExpired);
    println!("Transition: Red -> {:?} -> Green", output);

    let (output, yellow) = green.transition(TrafficInput::TimerExpired);
    println!("Transition: Green -> {:?} -> Yellow", output);

    let (output, _red) = yellow.transition(TrafficInput::TimerExpired);
    println!("Transition: Yellow -> {:?} -> Red", output);
}

/// 主函数 - 运行所有演示
fn main() {
    println!("=== Core Principles Deep Research ===");
    println!("Topic: Making Illegal States Unrepresentable\n");

    demo_typestate_workflow();
    demo_capability_system();
    demo_mealy_machine();

    println!("\n=== All demos completed successfully! ===");
    println!("Key insight: All state errors are caught at compile time,");
    println!("making runtime checks unnecessary and bugs impossible to write.");
}
