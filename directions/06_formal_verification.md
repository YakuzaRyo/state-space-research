# 06_formal_verification

## 方向名称
形式验证：Clover/Dafny 验证集成

## 核心问题
形式验证如何过滤 LLM 输出?

## 研究历程

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

## 关键资源

### 论文
- **Clover: Verified Code Generation** (相关论文待补充)
- **Dafny: A Language and Program Verifier**

### 开源项目
- Dafny: https://github.com/dafny-lang/dafny
- CBMC: https://github.com/diffblue/cbmc

### 技术博客
- 待补充...

## 架构洞察

### 形式验证核心机制
1. **前置/后置条件** —— 明确定义函数的契约
2. **不变量验证** —— 循环和状态转换的不变量检查
3. **定理证明** —— 使用SMT求解器验证程序正确性

### 与状态空间的结合点
- 形式验证作为状态空间的"准入测试"
- 只有通过验证的代码才能进入状态空间
- 验证失败提供反馈，指导LLM调整生成策略

## 待验证假设
- [ ] 待补充...

## 下一步研究方向
- 待补充...
