# OpenClaw 定时任务配置
# 状态空间架构研究自动化

## 任务信息
- **任务名称**: state-space-research
- **执行频率**: 每2小时一次
- **Cron表达式**: `0 */2 * * *`
- **超时时间**: 1800秒 (30分钟)
- **思考级别**: high
- **会话类型**: isolated

## 任务指令
```
请执行状态空间架构研究任务：
1. 阅读 state-space-research/RESEARCH_AGENT.md 了解任务要求
2. 根据当前时间确定研究方向
3. 阅读对应的研究方向文档
4. 进行深度研究和分析
5. 更新研究文档和代码
6. 提交到GitHub
```

## 设置方法

### 方法1: 使用OpenClaw Cron (推荐)
如果OpenClaw支持cron命令，使用以下配置：

```json
{
  "name": "state-space-research",
  "schedule": "0 */2 * * *",
  "action": "sessions_spawn",
  "params": {
    "task": "请阅读工作区中的state-space-research/RESEARCH_AGENT.md文件，按照其中的指令执行状态空间架构研究任务。完成后将研究成果提交到GitHub。",
    "mode": "run",
    "thinking": "high",
    "timeoutSeconds": 1800,
    "label": "state-space-research"
  }
}
```

### 方法2: Windows任务计划程序
1. 打开"任务计划程序"
2. 创建基本任务：
   - 名称: "State Space Research"
   - 触发器: 每天，重复间隔2小时
   - 操作: 启动程序
     - 程序: `powershell.exe`
     - 参数: `-ExecutionPolicy Bypass -File "C:\Users\11846\.openclaw-autoclaw\workspace\state-space-research\run-research.ps1"`

### 方法3: 手动触发
在OpenClaw对话中直接说：
- "开始状态空间架构研究"
- "执行研究任务"

或者使用sessions_spawn命令：
```
sessions_spawn(
  task="请阅读state-space-research/RESEARCH_AGENT.md并执行研究任务",
  mode="run",
  thinking="high",
  timeoutSeconds=1800,
  label="state-space-research"
)
```

## 研究时间表
每天13次研究执行：
- 🌅 上午: 9:45, 10:45, 11:45
- 🌞 下午/晚上: 14:45, 15:45, 16:45, 17:45, 18:45, 19:45, 20:45, 21:45, 22:45, 23:45

## Git工作流
- **master分支**: 每次研究后推送
- **stable分支**: 每日23:45同步
- **标签**: daily-YYYY-MM-DD 每日报告

## 监控与日志
- 研究日志: `logs/RESEARCH_LOG.md`
- 动态方向: `logs/DIRECTIONS_DYNAMIC.md`
- 每日报告: `daily/YYYY-MM-DD.md`

---
*创建时间: 2026-03-10*
*状态: ✅ 研究Agent已启动，正在执行第一次测试*
