# 05_ssd_hardware_optimization

## 方向名称
SSD硬件优化：Mamba-2状态空间对偶算法的Rust实现

## 核心问题
如何在Rust中高效实现Mamba-2的SSD算法以优化内存布局和计算性能?

## 研究历程

### 2026-03-09 10:46 深入研究（任务自动执行）
**研究发现**：
- **Mamba-2 SSD核心**: 将SSM表示为结构化矩阵乘法，允许使用张量核心（Tensor Cores）加速，相比Mamba-1的序列扫描更高效
- **内存布局关键**: 状态矩阵的列优先（column-major）存储对缓存友好，状态维度通常设为16/32/64以匹配SIMD宽度
- **Rust生态缺口**: 现有`burn`、`candle`框架缺乏原生的SSM算子，需通过`wgpu`或`cudarc`实现自定义kernel

**架构洞察**:
- **计算图优化**: SSD算法可将SSM计算表示为`softmax(QK^T)V`的变体，允许复用FlashAttention的IO-aware优化技术
- **状态传递边界**: 在序列并行场景下，块间状态传递需维护`h_block_end = A^chunk_size * h_block_start + B_local*u_local`的累积，需仔细处理浮点精度累积误差
- **硬件约束**: 当状态维度N=64时，A矩阵（N×N）可完全驻留于共享内存（48KB），避免全局内存往返

**代码产出**:
- `drafts/20260309_0949_ssd_memlayout.rs` - SSD内存布局Rust草稿

### 2026-03-09 初始化
- 创建方向文档

## 关键资源

### 论文
- **Mamba-2: State Space Duality** (Dao & Gu, 2024)
- **FlashAttention-3** - 内存分层与kernel fusion技术

### 开源项目
- `burn` - Rust深度学习框架
- `candle` - HuggingFace Rust ML框架
- `wgpu` - Rust WebGPU实现

## 架构洞察

### SOA vs AOS内存布局
- **AOS (Array of Structures)**: 传统布局，不利于SIMD加载
- **SOA (Structure of Arrays)**: 向量化友好，状态矩阵可按列优先存储

### 分块策略
- **chunk_size**: 128或256，平衡并行度与A^t计算精度
- **寄存器分块**: 使用`f32x16` AVX-512向量化

## 待验证假设
- [ ] 使用f16x8向量类型计算离散化步骤可减少40%内存带宽
- [ ] 块大小128/256在现代CPU (AVX-512)上达到最佳throughput/latency平衡
- [ ] 通过`const generics`在编译期确定state_dim (N)，允许LLVM完全展开内层循环

## 下一步研究方向
- 基于`wgpu`的compute shader原型验证内存布局假设
- 对比row-major vs column-major在RTX 4090上的实际带宽差异
