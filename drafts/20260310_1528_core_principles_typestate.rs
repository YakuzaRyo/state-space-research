// Typestate模式深度研究 - 复杂业务状态机实现
// 研究方向: 01_core_principles - 核心原则
// 时间: 2026-03-10

use std::marker::PhantomData;
use std::time::{SystemTime, Duration};

// ============================================================================
// 1. 订单状态机 (Created→Paid→Shipped→Delivered)
// ============================================================================

// 状态类型定义 - 零大小类型
pub struct Created;
pub struct Paid { payment_id: String, paid_at: SystemTime }
pub struct Shipped { tracking_number: String, shipped_at: SystemTime }
pub struct Delivered { delivered_at: SystemTime, signature: Option<String> }
pub struct Cancelled { reason: String, cancelled_at: SystemTime }

// 订单上下文数据
#[derive(Debug, Clone)]
pub struct OrderContext {
    pub order_id: String,
    pub customer_id: String,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
}

#[derive(Debug, Clone)]
pub struct OrderItem {
    pub sku: String,
    pub quantity: u32,
    pub unit_price: f64,
}

// 订单状态机泛型结构
pub struct Order<State> {
    context: OrderContext,
    state_data: State,
    _marker: PhantomData<State>,
}

// 状态trait界定
pub trait OrderState {}
impl OrderState for Created {}
impl OrderState for Paid {}
impl OrderState for Shipped {}
impl OrderState for Delivered {}
impl OrderState for Cancelled {}

// Created状态实现
impl Order<Created> {
    pub fn new(context: OrderContext) -> Self {
        Order {
            context,
            state_data: Created,
            _marker: PhantomData,
        }
    }

    /// 支付转换: Created -> Paid
    pub fn pay(self, payment_id: String) -> Order<Paid> {
        println!("[Order] {}: Created -> Paid (payment_id: {})",
            self.context.order_id, payment_id);
        Order {
            context: self.context,
            state_data: Paid {
                payment_id,
                paid_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }

    /// 取消转换: Created -> Cancelled
    pub fn cancel(self, reason: String) -> Order<Cancelled> {
        println!("[Order] {}: Created -> Cancelled (reason: {})",
            self.context.order_id, reason);
        Order {
            context: self.context,
            state_data: Cancelled {
                reason,
                cancelled_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }
}

// Paid状态实现
impl Order<Paid> {
    /// 获取支付信息 - 仅在Paid状态可用
    pub fn payment_info(&self) -> (&str, SystemTime) {
        (&self.state_data.payment_id, self.state_data.paid_at)
    }

    /// 发货转换: Paid -> Shipped
    pub fn ship(self, tracking_number: String) -> Order<Shipped> {
        println!("[Order] {}: Paid -> Shipped (tracking: {})",
            self.context.order_id, tracking_number);
        Order {
            context: self.context,
            state_data: Shipped {
                tracking_number,
                shipped_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }

    /// 退款并取消: Paid -> Cancelled
    pub fn refund_and_cancel(self, reason: String) -> Order<Cancelled> {
        println!("[Order] {}: Paid -> Cancelled with refund (reason: {})",
            self.context.order_id, reason);
        Order {
            context: self.context,
            state_data: Cancelled {
                reason: format!("Refunded: {}", reason),
                cancelled_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }
}

// Shipped状态实现
impl Order<Shipped> {
    /// 获取物流信息 - 仅在Shipped状态可用
    pub fn tracking_info(&self) -> (&str, SystemTime) {
        (&self.state_data.tracking_number, self.state_data.shipped_at)
    }

    /// 送达转换: Shipped -> Delivered
    pub fn deliver(self, signature: Option<String>) -> Order<Delivered> {
        println!("[Order] {}: Shipped -> Delivered", self.context.order_id);
        Order {
            context: self.context,
            state_data: Delivered {
                delivered_at: SystemTime::now(),
                signature,
            },
            _marker: PhantomData,
        }
    }
}

// Delivered状态实现
impl Order<Delivered> {
    /// 获取送达信息 - 仅在Delivered状态可用
    pub fn delivery_info(&self) -> (SystemTime, Option<&str>) {
        (self.state_data.delivered_at, self.state_data.signature.as_deref())
    }

    /// 完成订单，返回最终状态
    pub fn complete(self) -> CompletedOrder {
        CompletedOrder {
            order_id: self.context.order_id,
            customer_id: self.context.customer_id,
            total_amount: self.context.total_amount,
            delivered_at: self.state_data.delivered_at,
        }
    }
}

// Cancelled状态实现
impl Order<Cancelled> {
    /// 获取取消原因 - 仅在Cancelled状态可用
    pub fn cancellation_reason(&self) -> &str {
        &self.state_data.reason
    }
}

// 已完成订单（终态）
pub struct CompletedOrder {
    pub order_id: String,
    pub customer_id: String,
    pub total_amount: f64,
    pub delivered_at: SystemTime,
}

// ============================================================================
// 2. 支付状态机 (Initiated→Authorized→Captured)
// 实现Mealy机: 输出依赖于状态和输入事件
// ============================================================================

pub struct PaymentInitiated;
pub struct PaymentAuthorized {
    auth_code: String,
    authorized_amount: f64,
    expires_at: SystemTime,
}
pub struct PaymentCaptured {
    capture_amount: f64,
    captured_at: SystemTime,
    transaction_id: String,
}
pub struct PaymentFailed { error_code: String, error_message: String }
pub struct PaymentRefunded { refund_amount: f64, refunded_at: SystemTime }

#[derive(Debug, Clone)]
pub struct PaymentContext {
    pub payment_id: String,
    pub merchant_id: String,
    pub currency: String,
}

pub struct Payment<State> {
    context: PaymentContext,
    amount: f64,
    state_data: State,
    _marker: PhantomData<State>,
}

// Mealy机输出类型
#[derive(Debug)]
pub enum PaymentOutput {
    AuthSuccess { auth_code: String },
    AuthFailure { reason: String },
    CaptureSuccess { transaction_id: String },
    CaptureFailure { reason: String },
    RefundSuccess { refund_id: String },
}

impl Payment<PaymentInitiated> {
    pub fn new(context: PaymentContext, amount: f64) -> Self {
        Payment {
            context,
            amount,
            state_data: PaymentInitiated,
            _marker: PhantomData,
        }
    }

    /// Mealy转换: 输出依赖于输入事件
    pub fn authorize(self, card_token: &str) -> Result<(PaymentOutput, Payment<PaymentAuthorized>), (PaymentOutput, Payment<PaymentFailed>)> {
        println!("[Payment] {}: Authorizing ${} with card {}",
            self.context.payment_id, self.amount, card_token);

        // 模拟授权逻辑
        if card_token.starts_with("valid") {
            let auth_code = format!("AUTH-{}", uuid::Uuid::new_v4());
            let output = PaymentOutput::AuthSuccess { auth_code: auth_code.clone() };
            let new_state = Payment {
                context: self.context,
                amount: self.amount,
                state_data: PaymentAuthorized {
                    auth_code: auth_code.clone(),
                    authorized_amount: self.amount,
                    expires_at: SystemTime::now() + Duration::from_secs(3600),
                },
                _marker: PhantomData,
            };
            Ok((output, new_state))
        } else {
            let output = PaymentOutput::AuthFailure {
                reason: "Invalid card".to_string()
            };
            let new_state = Payment {
                context: self.context,
                amount: self.amount,
                state_data: PaymentFailed {
                    error_code: "CARD_DECLINED".to_string(),
                    error_message: "Card was declined".to_string(),
                },
                _marker: PhantomData,
            };
            Err((output, new_state))
        }
    }
}

impl Payment<PaymentAuthorized> {
    pub fn auth_code(&self) -> &str {
        &self.state_data.auth_code
    }

    pub fn authorized_amount(&self) -> f64 {
        self.state_data.authorized_amount
    }

    /// 部分或全额捕获
    pub fn capture(self, amount: f64) -> Result<(PaymentOutput, Payment<PaymentCaptured>), (PaymentOutput, Payment<PaymentFailed>)> {
        if amount > self.state_data.authorized_amount {
            return Err((
                PaymentOutput::CaptureFailure {
                    reason: "Capture amount exceeds authorized amount".to_string()
                },
                Payment {
                    context: self.context,
                    amount: self.amount,
                    state_data: PaymentFailed {
                        error_code: "INVALID_AMOUNT".to_string(),
                        error_message: "Capture amount too high".to_string(),
                    },
                    _marker: PhantomData,
                }
            ));
        }

        let transaction_id = format!("TXN-{}", uuid::Uuid::new_v4());
        let output = PaymentOutput::CaptureSuccess {
            transaction_id: transaction_id.clone()
        };
        let new_state = Payment {
            context: self.context,
            amount: self.amount,
            state_data: PaymentCaptured {
                capture_amount: amount,
                captured_at: SystemTime::now(),
                transaction_id,
            },
            _marker: PhantomData,
        };
        Ok((output, new_state))
    }

    /// 取消授权
    pub fn void(self) -> Payment<PaymentRefunded> {
        println!("[Payment] {}: Authorization voided", self.context.payment_id);
        Payment {
            context: self.context,
            amount: self.amount,
            state_data: PaymentRefunded {
                refund_amount: 0.0,
                refunded_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }
}

impl Payment<PaymentCaptured> {
    pub fn transaction_id(&self) -> &str {
        &self.state_data.transaction_id
    }

    pub fn captured_amount(&self) -> f64 {
        self.state_data.capture_amount
    }

    /// 退款
    pub fn refund(self, amount: f64) -> Result<(PaymentOutput, Payment<PaymentRefunded>), Payment<PaymentFailed>> {
        if amount > self.state_data.capture_amount {
            return Err(Payment {
                context: self.context,
                amount: self.amount,
                state_data: PaymentFailed {
                    error_code: "REFUND_EXCEEDS_CAPTURE".to_string(),
                    error_message: "Refund amount exceeds captured amount".to_string(),
                },
                _marker: PhantomData,
            });
        }

        let refund_id = format!("REF-{}", uuid::Uuid::new_v4());
        let output = PaymentOutput::RefundSuccess { refund_id };
        let new_state = Payment {
            context: self.context,
            amount: self.amount,
            state_data: PaymentRefunded {
                refund_amount: amount,
                refunded_at: SystemTime::now(),
            },
            _marker: PhantomData,
        };
        Ok((output, new_state))
    }
}

// ============================================================================
// 3. 工作流状态机 (Draft→Review→Approved→Published)
// ============================================================================

pub struct Draft;
pub struct InReview { reviewer_id: String, submitted_at: SystemTime };
pub struct Approved { approver_id: String, approved_at: SystemTime, comments: String };
pub struct Published { published_at: SystemTime, version: u32 };
pub struct Rejected { rejected_by: String, rejected_at: SystemTime, reason: String };
pub struct Archived { archived_at: SystemTime };

#[derive(Debug, Clone)]
pub struct WorkflowContext {
    pub workflow_id: String,
    pub content_type: ContentType,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub enum ContentType {
    Article,
    Document,
    Image,
    Video,
}

pub struct Workflow<State> {
    context: WorkflowContext,
    content: String,
    version: u32,
    state_data: State,
    _marker: PhantomData<State>,
}

// Draft状态
impl Workflow<Draft> {
    pub fn new(context: WorkflowContext, content: String) -> Self {
        Workflow {
            context,
            content,
            version: 1,
            state_data: Draft,
            _marker: PhantomData,
        }
    }

    /// 编辑内容 - 仅在Draft状态可用
    pub fn edit(&mut self, new_content: String) {
        self.content = new_content;
        self.version += 1;
        println!("[Workflow] {}: Content edited (v{})",
            self.context.workflow_id, self.version);
    }

    /// 提交审核
    pub fn submit_for_review(self, reviewer_id: String) -> Workflow<InReview> {
        println!("[Workflow] {}: Draft -> InReview (reviewer: {})",
            self.context.workflow_id, reviewer_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: InReview {
                reviewer_id,
                submitted_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }
}

// InReview状态
impl Workflow<InReview> {
    pub fn reviewer_id(&self) -> &str {
        &self.state_data.reviewer_id
    }

    /// 批准
    pub fn approve(self, approver_id: String, comments: String) -> Workflow<Approved> {
        println!("[Workflow] {}: InReview -> Approved by {}",
            self.context.workflow_id, approver_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: Approved {
                approver_id,
                approved_at: SystemTime::now(),
                comments,
            },
            _marker: PhantomData,
        }
    }

    /// 拒绝
    pub fn reject(self, rejected_by: String, reason: String) -> Workflow<Rejected> {
        println!("[Workflow] {}: InReview -> Rejected by {}: {}",
            self.context.workflow_id, rejected_by, reason);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: Rejected {
                rejected_by,
                rejected_at: SystemTime::now(),
                reason,
            },
            _marker: PhantomData,
        }
    }

    /// 退回修改
    pub fn request_changes(self) -> Workflow<Draft> {
        println!("[Workflow] {}: InReview -> Draft (changes requested)",
            self.context.workflow_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: Draft,
            _marker: PhantomData,
        }
    }
}

// Approved状态
impl Workflow<Approved> {
    pub fn approver_id(&self) -> &str {
        &self.state_data.approver_id
    }

    pub fn approval_comments(&self) -> &str {
        &self.state_data.comments
    }

    /// 发布
    pub fn publish(self) -> Workflow<Published> {
        println!("[Workflow] {}: Approved -> Published", self.context.workflow_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: Published {
                published_at: SystemTime::now(),
                version: self.version,
            },
            _marker: PhantomData,
        }
    }
}

// Rejected状态
impl Workflow<Rejected> {
    pub fn rejection_reason(&self) -> &str {
        &self.state_data.reason
    }

    /// 重新编辑
    pub fn revise(self) -> Workflow<Draft> {
        println!("[Workflow] {}: Rejected -> Draft (revision)",
            self.context.workflow_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: Draft,
            _marker: PhantomData,
        }
    }
}

// Published状态
impl Workflow<Published> {
    pub fn published_version(&self) -> u32 {
        self.state_data.version
    }

    /// 归档
    pub fn archive(self) -> Workflow<Archived> {
        println!("[Workflow] {}: Published -> Archived", self.context.workflow_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version,
            state_data: Archived {
                archived_at: SystemTime::now(),
            },
            _marker: PhantomData,
        }
    }

    /// 创建新版本（回到Draft）
    pub fn create_new_version(self) -> Workflow<Draft> {
        println!("[Workflow] {}: Published -> Draft (new version)",
            self.context.workflow_id);
        Workflow {
            context: self.context,
            content: self.content,
            version: self.version + 1,
            state_data: Draft,
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// 4. 组合状态机 - 并行/嵌套状态
// ============================================================================

// 并行状态组合示例：订单同时有支付状态和物流状态
pub struct OrderWithParallelStates<P, S> {
    order_context: OrderContext,
    payment_state: PhantomData<P>,
    shipping_state: PhantomData<S>,
}

// 支付子状态
pub struct PayPending;
pub struct PayCompleted;
pub struct PayFailedState;

// 物流子状态
pub struct ShipPending;
pub struct ShipInProgress;
pub struct ShipCompleted;

// 并行状态组合实现
impl OrderWithParallelStates<PayPending, ShipPending> {
    pub fn new(order_context: OrderContext) -> Self {
        OrderWithParallelStates {
            order_context,
            payment_state: PhantomData,
            shipping_state: PhantomData,
        }
    }

    /// 支付完成，但尚未发货
    pub fn complete_payment(self) -> OrderWithParallelStates<PayCompleted, ShipPending> {
        println!("[Parallel] Payment completed, awaiting shipment");
        OrderWithParallelStates {
            order_context: self.order_context,
            payment_state: PhantomData,
            shipping_state: PhantomData,
        }
    }
}

impl OrderWithParallelStates<PayCompleted, ShipPending> {
    /// 开始发货
    pub fn start_shipping(self) -> OrderWithParallelStates<PayCompleted, ShipInProgress> {
        println!("[Parallel] Shipping started");
        OrderWithParallelStates {
            order_context: self.order_context,
            payment_state: PhantomData,
            shipping_state: PhantomData,
        }
    }
}

impl OrderWithParallelStates<PayCompleted, ShipInProgress> {
    /// 发货完成
    pub fn complete_shipping(self) -> OrderWithParallelStates<PayCompleted, ShipCompleted> {
        println!("[Parallel] Shipping completed");
        OrderWithParallelStates {
            order_context: self.order_context,
            payment_state: PhantomData,
            shipping_state: PhantomData,
        }
    }
}

impl OrderWithParallelStates<PayCompleted, ShipCompleted> {
    /// 订单完全完成
    pub fn finalize(self) -> CompletedOrder {
        CompletedOrder {
            order_id: self.order_context.order_id,
            customer_id: self.order_context.customer_id,
            total_amount: self.order_context.total_amount,
            delivered_at: SystemTime::now(),
        }
    }
}

// ============================================================================
// 5. 嵌套状态机 - 复杂工作流中的子状态机
// ============================================================================

// 文档审核子状态机（嵌套在工作流中）
pub struct DocumentReview<State> {
    document_id: String,
    state: State,
}

pub struct AwaitingReview;
pub struct UnderReview { reviewer: String, started_at: SystemTime };
pub struct ReviewCompleted { reviewer: String, approved: bool, comments: String };

impl DocumentReview<AwaitingReview> {
    pub fn new(document_id: String) -> Self {
        DocumentReview {
            document_id,
            state: AwaitingReview,
        }
    }

    pub fn start_review(self, reviewer: String) -> DocumentReview<UnderReview> {
        DocumentReview {
            document_id: self.document_id,
            state: UnderReview {
                reviewer: reviewer.clone(),
                started_at: SystemTime::now(),
            },
        }
    }
}

impl DocumentReview<UnderReview> {
    pub fn complete_review(self, approved: bool, comments: String) -> DocumentReview<ReviewCompleted> {
        DocumentReview {
            document_id: self.document_id,
            state: ReviewCompleted {
                reviewer: self.state.reviewer,
                approved,
                comments,
            },
        }
    }
}

// 包含嵌套审核的工作流
pub struct WorkflowWithNestedReview<W, R> {
    workflow: W,
    document_review: R,
}

// 创建包含嵌套审核的工作流
impl Workflow<Draft> {
    pub fn attach_document_review(self, document_id: String) -> WorkflowWithNestedReview<Workflow<Draft>, DocumentReview<AwaitingReview>> {
        WorkflowWithNestedReview {
            workflow: self,
            document_review: DocumentReview::new(document_id),
        }
    }
}

// 嵌套状态转换
impl WorkflowWithNestedReview<Workflow<Draft>, DocumentReview<AwaitingReview>> {
    pub fn start_document_review(self, reviewer: String) -> WorkflowWithNestedReview<Workflow<Draft>, DocumentReview<UnderReview>> {
        WorkflowWithNestedReview {
            workflow: self.workflow,
            document_review: self.document_review.start_review(reviewer),
        }
    }
}

impl WorkflowWithNestedReview<Workflow<Draft>, DocumentReview<UnderReview>> {
    pub fn complete_document_review(self, approved: bool, comments: String) -> WorkflowWithNestedReview<Workflow<Draft>, DocumentReview<ReviewCompleted>> {
        WorkflowWithNestedReview {
            workflow: self.workflow,
            document_review: self.document_review.complete_review(approved, comments),
        }
    }
}

impl WorkflowWithNestedReview<Workflow<Draft>, DocumentReview<ReviewCompleted>> {
    /// 根据审核结果决定下一步
    pub fn proceed(self) -> Result<Workflow<InReview>, Workflow<Draft>> {
        if self.document_review.state.approved {
            Ok(self.workflow.submit_for_review("system".to_string()))
        } else {
            println!("[Nested] Document review failed, staying in Draft");
            Ok(self.workflow)  // 实际上应该返回修改后的Draft
        }
    }
}

// ============================================================================
// 6. 测试和演示
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_state_machine() {
        let context = OrderContext {
            order_id: "ORD-001".to_string(),
            customer_id: "CUST-001".to_string(),
            items: vec![OrderItem {
                sku: "SKU-001".to_string(),
                quantity: 2,
                unit_price: 29.99,
            }],
            total_amount: 59.98,
        };

        let order = Order::new(context);
        let order = order.pay("PAY-123".to_string());
        let (payment_id, _) = order.payment_info();
        assert_eq!(payment_id, "PAY-123");

        let order = order.ship("TRACK-456".to_string());
        let (tracking, _) = order.tracking_info();
        assert_eq!(tracking, "TRACK-456");

        let order = order.deliver(Some("John Doe".to_string()));
        let (_, signature) = order.delivery_info();
        assert_eq!(signature, Some("John Doe"));

        let completed = order.complete();
        assert_eq!(completed.order_id, "ORD-001");
    }

    #[test]
    fn test_workflow_state_machine() {
        let context = WorkflowContext {
            workflow_id: "WF-001".to_string(),
            content_type: ContentType::Article,
            created_by: "user@example.com".to_string(),
        };

        let mut workflow = Workflow::new(context, "Initial content".to_string());
        workflow.edit("Updated content".to_string());

        let workflow = workflow.submit_for_review("reviewer@example.com".to_string());
        let workflow = workflow.approve("manager@example.com".to_string(), "Looks good".to_string());

        assert_eq!(workflow.approver_id(), "manager@example.com");
        assert_eq!(workflow.approval_comments(), "Looks good");

        let workflow = workflow.publish();
        assert_eq!(workflow.published_version(), 2);
    }

    #[test]
    fn test_parallel_states() {
        let context = OrderContext {
            order_id: "ORD-PARALLEL".to_string(),
            customer_id: "CUST-001".to_string(),
            items: vec![],
            total_amount: 100.0,
        };

        let order = OrderWithParallelStates::<PayPending, ShipPending>::new(context);
        let order = order.complete_payment();
        let order = order.start_shipping();
        let order = order.complete_shipping();
        let completed = order.finalize();

        assert_eq!(completed.order_id, "ORD-PARALLEL");
    }
}

// ============================================================================
// 7. 编译期验证演示（这些代码如果取消注释会编译失败）
// ============================================================================

/*
// 错误示例1: 在未支付时尝试发货
fn invalid_ship_before_pay() {
    let context = OrderContext {
        order_id: "ORD-001".to_string(),
        customer_id: "CUST-001".to_string(),
        items: vec![],
        total_amount: 100.0,
    };
    let order = Order::new(context);
    // 编译错误: Created状态没有ship方法
    // let order = order.ship("TRACK-001".to_string());
}

// 错误示例2: 在已发货后尝试支付
fn invalid_pay_after_ship() {
    let context = OrderContext { ... };
    let order = Order::new(context);
    let order = order.pay("PAY-001".to_string());
    let order = order.ship("TRACK-001".to_string());
    // 编译错误: Shipped状态没有pay方法
    // let order = order.pay("PAY-002".to_string());
}

// 错误示例3: 在Draft状态尝试发布
fn invalid_publish_from_draft() {
    let context = WorkflowContext { ... };
    let workflow = Workflow::new(context, "content".to_string());
    // 编译错误: Draft状态没有publish方法
    // let workflow = workflow.publish();
}
*/

// ============================================================================
// 依赖项说明 (Cargo.toml)
// ============================================================================
/*
[dependencies]
uuid = { version = "1.0", features = ["v4"] }

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
*/
