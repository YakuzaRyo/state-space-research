//! Rust类型系统实现状态空间 - 代码草稿
//! 研究方向: 09_rust_type_system
//! 创建时间: 2026-03-11 13:05:49

use std::marker::PhantomData;

// ============================================================================
// 示例1: 基础Typestate模式 - 交通灯状态机
// ============================================================================

/// 状态标记类型
pub struct Red;
pub struct Yellow;
pub struct Green;

/// 使用PhantomData编码状态的交通灯
pub struct TrafficLight<S> {
    _state: PhantomData<S>,
}

impl TrafficLight<Red> {
    pub fn new() -> Self {
        TrafficLight { _state: PhantomData }
    }

    /// 红灯只能切换到绿灯
    pub fn next(self) -> TrafficLight<Green> {
        println!("Red -> Green");
        TrafficLight { _state: PhantomData }
    }
}

impl TrafficLight<Green> {
    /// 绿灯只能切换到黄灯
    pub fn next(self) -> TrafficLight<Yellow> {
        println!("Green -> Yellow");
        TrafficLight { _state: PhantomData }
    }
}

impl TrafficLight<Yellow> {
    /// 黄灯只能切换到红灯
    pub fn next(self) -> TrafficLight<Red> {
        println!("Yellow -> Red");
        TrafficLight { _state: PhantomData }
    }
}

// ============================================================================
// 示例2: Const Generics状态编码 - 带重试次数的连接状态机
// ============================================================================

/// 使用const泛型编码状态ID
#[derive(Debug)]
pub struct State<const ID: u32>;

/// 状态类型别名
pub type Disconnected = State<0>;
pub type Connecting = State<1>;
pub type Connected = State<2>;
pub type Failed = State<3>;

/// 带最大重试次数的连接状态机
#[derive(Debug)]
pub struct Connection<S, const MAX_RETRIES: u32> {
    _state: PhantomData<S>,
    retry_count: u32,
    address: String,
}

impl<const N: u32> Connection<Disconnected, N> {
    pub fn new(address: impl Into<String>) -> Self {
        Connection {
            _state: PhantomData,
            retry_count: 0,
            address: address.into(),
        }
    }

    pub fn connect(self) -> Connection<Connecting, N> {
        println!("Starting connection to {}...", self.address);
        Connection {
            _state: PhantomData,
            retry_count: self.retry_count,
            address: self.address,
        }
    }
}

impl<const N: u32> Connection<Connecting, N> {
    /// 尝试连接，成功则进入Connected，失败则检查重试次数
    pub fn attempt(self, success: bool) -> Result<Connection<Connected, N>, Connection<Failed, N>> {
        if success {
            println!("Connection established!");
            Ok(Connection {
                _state: PhantomData,
                retry_count: 0,
                address: self.address,
            })
        } else {
            println!("Connection failed, retry count: {}", self.retry_count + 1);
            Err(Connection {
                _state: PhantomData,
                retry_count: self.retry_count + 1,
                address: self.address,
            })
        }
    }
}

impl<const N: u32> Connection<Failed, N> {
    /// 检查是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < N
    }

    /// 重试连接
    pub fn retry(self) -> Result<Connection<Connecting, N>, Connection<Disconnected, N>> {
        if self.retry_count < N {
            println!("Retrying connection (attempt {}/{})...", self.retry_count + 1, N);
            Ok(Connection {
                _state: PhantomData,
                retry_count: self.retry_count,
                address: self.address,
            })
        } else {
            println!("Max retries exceeded, giving up.");
            Err(Connection {
                _state: PhantomData,
                retry_count: 0,
                address: self.address,
            })
        }
    }
}

impl<const N: u32> Connection<Connected, N> {
    pub fn send(&self, data: &str) {
        println!("Sending data: {}", data);
    }

    pub fn disconnect(self) -> Connection<Disconnected, N> {
        println!("Disconnecting...");
        Connection {
            _state: PhantomData,
            retry_count: 0,
            address: self.address,
        }
    }
}

// ============================================================================
// 示例3: GATs模式 - 状态相关数据类型
// ============================================================================

/// 定义状态trait，使用GATs定义状态相关数据
pub trait StateData {
    type Data: Default;
    fn get_data(&self) -> &Self::Data;
    fn get_data_mut(&mut self) -> &mut Self::Data;
}

/// 空闲状态数据
#[derive(Default)]
pub struct IdleData {
    queue: Vec<String>,
}

/// 运行状态数据
#[derive(Default)]
pub struct RunningData {
    current_task: Option<String>,
    progress: u32,
}

/// 停止状态数据
#[derive(Default)]
pub struct StoppedData {
    completed_tasks: Vec<String>,
    final_stats: String,
}

/// 空闲状态
pub struct Idle {
    data: IdleData,
}

/// 运行状态
pub struct Running {
    data: RunningData,
}

/// 停止状态
pub struct Stopped {
    data: StoppedData,
}

impl Default for Idle {
    fn default() -> Self {
        Idle { data: IdleData::default() }
    }
}

impl Default for Running {
    fn default() -> Self {
        Running { data: RunningData::default() }
    }
}

impl Default for Stopped {
    fn default() -> Self {
        Stopped { data: StoppedData::default() }
    }
}

impl StateData for Idle {
    type Data = IdleData;
    fn get_data(&self) -> &IdleData { &self.data }
    fn get_data_mut(&mut self) -> &mut IdleData { &mut self.data }
}

impl StateData for Running {
    type Data = RunningData;
    fn get_data(&self) -> &RunningData { &self.data }
    fn get_data_mut(&mut self) -> &mut RunningData { &mut self.data }
}

impl StateData for Stopped {
    type Data = StoppedData;
    fn get_data(&self) -> &StoppedData { &self.data }
    fn get_data_mut(&mut self) -> &mut StoppedData { &mut self.data }
}

/// 使用GATs的工作流状态机
pub struct Workflow<S: StateData> {
    state: S,
}

impl Workflow<Idle> {
    pub fn new() -> Self {
        Workflow { state: Idle::default() }
    }

    pub fn enqueue(&mut self, task: impl Into<String>) {
        self.state.get_data_mut().queue.push(task.into());
        println!("Task enqueued");
    }

    pub fn start(mut self) -> Workflow<Running> {
        let task = self.state.get_data_mut().queue.pop();
        println!("Starting workflow with task: {:?}", task);

        let mut running = Running::default();
        running.data.current_task = task;
        running.data.progress = 0;

        Workflow { state: running }
    }
}

impl Workflow<Running> {
    pub fn update_progress(&mut self, progress: u32) {
        self.state.get_data_mut().progress = progress;
        println!("Progress updated to {}%", progress);
    }

    pub fn complete(mut self) -> Workflow<Stopped> {
        let task = self.state.get_data_mut().current_task.take();
        println!("Task {:?} completed", task);

        let mut stopped = Stopped::default();
        if let Some(t) = task {
            stopped.data.completed_tasks.push(t);
        }
        stopped.data.final_stats = "Workflow completed successfully".to_string();

        Workflow { state: stopped }
    }
}

impl Workflow<Stopped> {
    pub fn get_stats(&self) -> &str {
        &self.state.get_data().final_stats
    }
}

// ============================================================================
// 示例4: 类型级状态转换验证 - 使用trait约束
// ============================================================================

/// 定义允许的状态转换
pub trait ValidTransition<From, To> {
    fn validate();
}

/// 文档状态
pub struct Draft;
pub struct Review;
pub struct Published;
pub struct Archived;

/// 文档状态机，使用trait约束验证转换
pub struct Document<S> {
    _state: PhantomData<S>,
    content: String,
    version: u32,
}

impl Document<Draft> {
    pub fn new(content: impl Into<String>) -> Self {
        Document {
            _state: PhantomData,
            content: content.into(),
            version: 1,
        }
    }

    pub fn submit_for_review(self) -> Document<Review> {
        println!("Submitting document for review...");
        Document {
            _state: PhantomData,
            content: self.content,
            version: self.version,
        }
    }
}

impl Document<Review> {
    pub fn approve(self) -> Document<Published> {
        println!("Document approved!");
        Document {
            _state: PhantomData,
            content: self.content,
            version: self.version + 1,
        }
    }

    pub fn reject(self) -> Document<Draft> {
        println!("Document rejected, returning to draft.");
        Document {
            _state: PhantomData,
            content: self.content,
            version: self.version,
        }
    }
}

impl Document<Published> {
    pub fn read(&self) -> &str {
        &self.content
    }

    pub fn archive(self) -> Document<Archived> {
        println!("Archiving published document...");
        Document {
            _state: PhantomData,
            content: self.content,
            version: self.version,
        }
    }
}

impl Document<Archived> {
    pub fn restore(self) -> Document<Draft> {
        println!("Restoring archived document to draft...");
        Document {
            _state: PhantomData,
            content: self.content,
            version: self.version + 1,
        }
    }
}

// ============================================================================
// 示例5: 分层状态空间 - 嵌套状态机
// ============================================================================

/// 网络连接状态（外层状态机）
pub struct NetworkConnected;
pub struct NetworkDisconnected;

/// 会话状态（内层状态机）
pub struct SessionActive;
pub struct SessionInactive;

/// 网络会话管理器 - 分层状态空间
pub struct NetworkSession<NetworkState, SessionState> {
    _network: PhantomData<NetworkState>,
    _session: PhantomData<SessionState>,
    server_addr: String,
    session_id: Option<u64>,
}

impl NetworkSession<NetworkDisconnected, SessionInactive> {
    pub fn new(server_addr: impl Into<String>) -> Self {
        NetworkSession {
            _network: PhantomData,
            _session: PhantomData,
            server_addr: server_addr.into(),
            session_id: None,
        }
    }

    pub fn connect(self) -> NetworkSession<NetworkConnected, SessionInactive> {
        println!("Connecting to {}...", self.server_addr);
        NetworkSession {
            _network: PhantomData,
            _session: PhantomData,
            server_addr: self.server_addr,
            session_id: None,
        }
    }
}

impl NetworkSession<NetworkConnected, SessionInactive> {
    pub fn login(self, user: &str) -> NetworkSession<NetworkConnected, SessionActive> {
        let session_id = user.len() as u64 * 12345; // 模拟会话ID生成
        println!("User {} logged in, session ID: {}", user, session_id);
        NetworkSession {
            _network: PhantomData,
            _session: PhantomData,
            server_addr: self.server_addr,
            session_id: Some(session_id),
        }
    }

    pub fn disconnect(self) -> NetworkSession<NetworkDisconnected, SessionInactive> {
        println!("Disconnecting from network...");
        NetworkSession {
            _network: PhantomData,
            _session: PhantomData,
            server_addr: self.server_addr,
            session_id: None,
        }
    }
}

impl NetworkSession<NetworkConnected, SessionActive> {
    pub fn send_data(&self, data: &str) {
        if let Some(id) = self.session_id {
            println!("[Session {}] Sending: {}", id, data);
        }
    }

    pub fn logout(self) -> NetworkSession<NetworkConnected, SessionInactive> {
        println!("Logging out from session {:?}...", self.session_id);
        NetworkSession {
            _network: PhantomData,
            _session: PhantomData,
            server_addr: self.server_addr,
            session_id: None,
        }
    }

    pub fn disconnect(self) -> NetworkSession<NetworkDisconnected, SessionInactive> {
        println!("Force disconnecting active session {:?}...", self.session_id);
        NetworkSession {
            _network: PhantomData,
            _session: PhantomData,
            server_addr: self.server_addr,
            session_id: None,
        }
    }
}

// ============================================================================
// 测试和演示
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_light() {
        let light = TrafficLight::<Red>::new();
        let light = light.next(); // Red -> Green
        let light = light.next(); // Green -> Yellow
        let _light = light.next(); // Yellow -> Red
    }

    #[test]
    fn test_connection_with_retry() {
        // 创建一个最多重试3次的连接
        let conn: Connection<Disconnected, 3> = Connection::new("192.168.1.1:8080");
        let conn = conn.connect();

        // 模拟失败然后成功
        match conn.attempt(false) {
            Ok(_) => panic!("Should have failed"),
            Err(failed) => {
                assert!(failed.can_retry());
                let conn = failed.retry().unwrap();
                let conn = conn.attempt(true).unwrap();
                conn.send("Hello, World!");
                let _conn = conn.disconnect();
            }
        }
    }

    #[test]
    fn test_workflow() {
        let mut workflow = Workflow::<Idle>::new();
        workflow.enqueue("Task 1");
        workflow.enqueue("Task 2");

        let mut workflow = workflow.start();
        workflow.update_progress(50);
        workflow.update_progress(100);

        let workflow = workflow.complete();
        assert_eq!(workflow.get_stats(), "Workflow completed successfully");
    }

    #[test]
    fn test_document_lifecycle() {
        let doc = Document::<Draft>::new("Initial content");
        let doc = doc.submit_for_review();
        let doc = doc.approve();
        assert_eq!(doc.read(), "Initial content");
        let doc = doc.archive();
        let _doc = doc.restore();
    }

    #[test]
    fn test_nested_state_machine() {
        let session = NetworkSession::<NetworkDisconnected, SessionInactive>::new("server.example.com");
        let session = session.connect();
        let session = session.login("alice");
        session.send_data("Hello!");
        let session = session.logout();
        let _session = session.disconnect();
    }
}

fn main() {
    println!("=== Rust Type System State Space Implementation ===\n");

    // 演示1: 交通灯
    println!("--- Traffic Light Demo ---");
    let light = TrafficLight::<Red>::new();
    let light = light.next();
    let light = light.next();
    let _light = light.next();
    println!();

    // 演示2: 带重试的连接
    println!("--- Connection with Retry Demo ---");
    let conn: Connection<Disconnected, 2> = Connection::new("example.com:443");
    let conn = conn.connect();
    match conn.attempt(false) {
        Err(failed) => {
            if let Ok(conn) = failed.retry() {
                if let Ok(conn) = conn.attempt(true) {
                    conn.send("Secure data");
                    let _ = conn.disconnect();
                }
            }
        }
        Ok(_) => {}
    }
    println!();

    // 演示3: 工作流
    println!("--- Workflow Demo ---");
    let mut workflow = Workflow::<Idle>::new();
    workflow.enqueue("Process data");
    let mut workflow = workflow.start();
    workflow.update_progress(75);
    let workflow = workflow.complete();
    println!("Stats: {}", workflow.get_stats());
    println!();

    // 演示4: 文档生命周期
    println!("--- Document Lifecycle Demo ---");
    let doc = Document::<Draft>::new("My Document");
    let doc = doc.submit_for_review();
    let doc = doc.approve();
    println!("Reading: {}", doc.read());
    let doc = doc.archive();
    let _doc = doc.restore();
    println!();

    // 演示5: 嵌套状态机
    println!("--- Nested State Machine Demo ---");
    let session = NetworkSession::<NetworkDisconnected, SessionInactive>::new("api.example.com");
    let session = session.connect();
    let session = session.login("user123");
    session.send_data("Request data");
    let session = session.logout();
    let _session = session.disconnect();

    println!("\n=== All demos completed successfully! ===");
}
