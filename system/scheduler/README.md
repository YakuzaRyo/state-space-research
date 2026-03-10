# 持久型定时任务调度系统

## 概述

这是一个为 Claude Code 设计的持久型定时任务调度系统，解决原生 session-only 定时任务的限制。

**核心特性：**
- ✅ 持久化运行（会话结束后继续运行）
- ✅ Subagent 并发控制（全局 + 任务级限制）
- ✅ 任务队列和过载保护
- ✅ 元调度器（每两天自更新）
- ✅ 配置热重载
- ✅ 状态持久化和备份

## 架构

```
system/scheduler/
├── README.md              # 本文档
├── tasks.json             # 任务配置文件
├── scheduler.py           # 主调度器
├── meta_scheduler.py      # 元调度器（自更新）
├── state.json             # 运行时状态
├── task_history.json      # 执行历史
├── scheduler.pid          # 进程ID
├── health_check.py        # 健康检查脚本
└── backups/               # 配置备份目录
    ├── tasks_20260310_120000_v1.0.json
    └── ...

logs/scheduler/
├── scheduler.log          # 调度器日志
└── ...
```

## 快速开始

### 1. 安装依赖

```bash
pip install apscheduler
```

### 2. 配置任务

编辑 `system/scheduler/tasks.json`：

```json
{
  "tasks": [
    {
      "id": "my-research",
      "name": "我的研究任务",
      "enabled": true,
      "schedule": {
        "type": "interval",
        "minutes": 30
      },
      "action": {
        "type": "research",
        "command": "/research 我的研究方向",
        "timeout": 600,
        "requires_subagent": true
      },
      "concurrency": {
        "max_subagents": 2,
        "queue_if_busy": true
      }
    }
  ]
}
```

### 3. 启动调度器

```bash
# Windows
python system/scheduler/scheduler.py start

# 或使用 PowerShell 创建 Windows 服务
```

### 4. 查看状态

```bash
python system/scheduler/scheduler.py status
```

## 配置详解

### 任务配置

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 任务唯一标识 |
| `name` | string | 任务名称 |
| `enabled` | boolean | 是否启用 |
| `schedule` | object | 调度配置 |
| `action` | object | 执行动作 |
| `concurrency` | object | 并发控制 |
| `retry` | object | 重试策略 |

### 调度类型

**间隔调度：**
```json
{
  "type": "interval",
  "minutes": 15
  // 或 "hours": 2, "days": 1
}
```

**Cron 调度：**
```json
{
  "type": "cron",
  "expression": "0 */6 * * *"
}
```

### 动作类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `research` | 调用 /research | `"/research 方向"` |
| `builtin` | 内置命令 | `"/clear"` |
| `script` | 执行脚本 | `"system/scheduler/xxx.py"` |

### 并发控制

```json
{
  "concurrency": {
    "max_subagents": 2,        // 该任务最大 subagent 数
    "queue_if_busy": true,     // 忙时排队
    "skip_if_overload": false  // 过载时跳过
  }
}
```

## 元调度器（Meta Scheduler）

**功能：** 每两天自动重新创建所有定时任务，基于历史表现优化配置。

**继承机制：**
1. 保留上一代所有任务定义
2. 分析任务成功率
3. 自动调整参数（重试次数、并发数等）
4. 版本递增和备份

**运行流程：**
```
备份当前配置
    ↓
分析任务历史表现
    ↓
继承上一代任务
    ↓
演进任务配置
    ↓
递增版本号
    ↓
保存新配置
```

## Subagent 连接池

**设计：**
- 全局限制：防止系统过载
- 任务级限制：防止单个任务占用所有资源
- 队列机制：请求排队而非直接失败

**流程：**
```
任务请求 subagent
    ↓
检查全局限制 → 否 → 进入队列
    ↓ 是
检查任务限制 → 否 → 进入队列
    ↓ 是
分配槽位 → 执行任务
    ↓
释放槽位 → 处理队列
```

## 命令参考

```bash
# 启动调度器
python system/scheduler/scheduler.py start

# 停止调度器
python system/scheduler/scheduler.py stop

# 查看状态
python system/scheduler/scheduler.py status

# 热重载配置
python system/scheduler/scheduler.py reload

# 手动运行元调度器
python system/scheduler/meta_scheduler.py
```

## Windows 服务部署

创建计划任务实现开机自启：

```powershell
# 创建任务
$action = New-ScheduledTaskAction `
    -Execute "python" `
    -Argument "system/scheduler/scheduler.py start" `
    -WorkingDirectory "D:\11846\state-space-research"

$trigger = New-ScheduledTaskTrigger `
    -AtStartup

$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable

Register-ScheduledTask `
    -TaskName "ClaudeScheduler" `
    -Action $action `
    -Trigger $trigger `
    -Settings $settings `
    -User "$env:USERNAME" `
    -RunLevel Highest
```

## 常见问题

**Q: 如何限制 subagent 数量？**
A: 在 `tasks.json` 中配置 `resource_limits.max_subagents_global` 和任务的 `concurrency.max_subagents`。

**Q: 任务执行失败后怎么办？**
A: 配置 `retry` 字段设置重试次数，或让元调度器自动调整。

**Q: 如何动态添加任务？**
A: 编辑 `tasks.json` 后执行 `python system/scheduler/scheduler.py reload`。

**Q: 元调度器会删除任务吗？**
A: 不会，它只继承和演进，不会主动删除任务。

## 扩展开发

添加新的动作类型：

```python
# 在 scheduler.py 的 _run_action 方法中添加
elif action_type == 'my_action':
    result = self._run_my_action(action, timeout)
```

## License

MIT
