# OpenClaw Cron 定时任务配置
# 状态空间架构研究 - 每30分钟执行一次

## 任务配置
- **名称**: state-space-research
- **间隔**: 30分钟
- **超时**: 1800秒（30分钟）
- **思考级别**: high

## Cron表达式
```
*/30 * * * *  (每30分钟)
```

## 设置命令

### 方法1: 通过OpenClaw CLI设置
```bash
openclaw cron add "*/30 * * * *" --task "sessions_spawn(task='阅读state-space-research/RESEARCH_AGENT.md并执行研究', mode='run', thinking='high', timeoutSeconds=1800, label='state-space-research')"
```

### 方法2: 通过配置文件
在OpenClaw配置目录创建 `cron.json`:
```json
{
  "jobs": [
    {
      "name": "state-space-research",
      "schedule": "*/30 * * * *",
      "command": "sessions_spawn",
      "args": {
        "task": "阅读state-space-research/RESEARCH_AGENT.md并执行研究任务",
        "mode": "run",
        "thinking": "high",
        "timeoutSeconds": 1800,
        "label": "state-space-research"
      }
    }
  ]
}
```

### 方法3: 手动触发（在OpenClaw中对话）
直接说：
```
开始状态空间架构研究
```

---

## 当前状态
- ✅ GitHub已同步
- ⏳ 定时任务待配置
- 💡 可以直接对话触发研究
