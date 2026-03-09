# 04_deterministic_arch

## 方向名称
确定性架构：Praetorian

## 核心问题
Thin Agent + Fat Platform 如何工作?

## 研究历程

### 2026-03-09 初始化
- 创建方向文档
- 待研究...

## 关键资源

### 论文/博客
- **Praetorian: Deterministic AI Orchestration**
  - Thin Agent (<150行) + Fat Platform
  - Gateway模式动态路由技能
  - 确定性Hooks在LLM上下文外强制执行
  - "将AI转变为软件供应链的确定性组件"

### 开源项目
- 待补充...

### 技术博客
- 待补充...

## 架构洞察

### Praetorian 核心机制
1. **Thin Agent** —— 极简Agent逻辑（<150行），专注于意图识别
2. **Fat Platform** —— 丰富的确定性运行时，包含所有业务逻辑
3. **Gateway模式** —— 动态路由技能请求到确定性处理模块
4. **确定性Hooks** —— 在LLM上下文外强制执行约束

### 与状态空间的结合点
- Fat Platform 就是状态空间的物理实现
- Gateway 作为状态空间的入口守卫
- Thin Agent 在状态空间内"导航"，而非"生成"

## 待验证假设
- [ ] 待补充...

## 下一步研究方向
- 待补充...
