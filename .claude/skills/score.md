# Score - 研究任务评分管理

管理研究任务的评分系统

## 用法

### 添加评分记录（研究完成时）
当研究agent完成任务后，手动添加记录：

```
研究任务完成: [方向名]
持续时间: [X]分钟

调用评分追踪记录
```

我会自动执行：
```python
# 添加记录示例
score_cli.py add "LLM导航器" 2400   # 40分钟 = 2400秒
```

### 手动减分（违规时）
```
/score reduce [原因]
```

示例：
- `/score reduce 任务提前终止` - 记录违规扣分

### 查看状态
```
/score status
```

显示：
- 总评分
- 总执行次数
- 优秀(≥25分钟) / 不足(<20分钟)
- 最近5次记录
- 平均持续时间

### 重置数据
```
/score reset
```

## 评分规则

| 持续时间 | 得分 | 等级 |
|---------|------|------|
| ≥25分钟 (1500秒) | +1 | EXCELLENT |
| 20-25分钟 | 0 | ACCEPTABLE |
| <20分钟 (1200秒) | -1 | INSUFFICIENT |

## 脚本位置

`system/scheduler/score_cli.py`

命令：
```bash
python score_cli.py add [方向] [秒数]   # 加分
python score_cli.py reduce [原因]       # 减分
python score_cli.py status              # 状态
python score_cli.py reset               # 重置
```

## 当前状态

直接询问我"/score status"即可查看
