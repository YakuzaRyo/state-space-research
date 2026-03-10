# 状态空间架构研究 - 快速开始指南

## 🚀 快速开始

### 方法1: 手动触发研究（推荐）

在OpenClaw对话中直接说：
```
开始状态空间架构研究
```

或者使用命令：
```
sessions_spawn(
  task="请阅读state-space-research/RESEARCH_AGENT.md并执行研究任务",
  mode="run",
  thinking="high",
  timeoutSeconds=1800,
  label="state-space-research"
)
```

### 方法2: 使用启动器脚本

查看研究状态：
```powershell
cd C:\Users\11846\.openclaw-autoclaw\workspace\state-space-research
.\Start-Research.ps1 -Status
```

手动触发研究：
```powershell
.\Start-Research.ps1
```

强制执行（忽略2小时间隔）：
```powershell
.\Start-Research.ps1 -Force
```

### 方法3: 自动化定时任务

#### Windows任务计划程序
1. 打开"任务计划程序"
2. 创建基本任务：
   - 名称: State Space Research
   - 触发器: 每天，重复间隔2小时，持续时间24小时
   - 操作: 启动程序
     - 程序: `powershell.exe`
     - 参数: `-ExecutionPolicy Bypass -File "C:\Users\11846\.openclaw-autoclaw\workspace\state-space-research\Start-Research.ps1"`

## 📊 研究时间表

| 时间段 | 研究方向 | 核心问题 |
|--------|---------|---------|
| 00:00, 12:00 | 核心原则 | 如何让错误在设计上不可能发生? |
| 02:00, 14:00 | 分层设计 | Syntax→Semantic→Pattern→Domain如何转换? |
| 04:00, 16:00 | LLM导航器 | LLM作为启发式函数的理论基础? |
| 06:00, 18:00 | 实现技术 | 如何用Rust类型系统实现状态空间? |
| 08:00, 20:00 | 工具设计 | 如何设计'无法产生错误'的工具? |
| 10:00, 22:00 | 对比分析 | Claude Code/OpenCode的根本缺陷是什么? |

## 📁 目录结构

```
state-space-research/
├── directions/           # 12个研究方向的深度分析
│   ├── 01_core_principles.md
│   ├── 02_refinement_calculus.md
│   ├── ...
│   └── 12_engineering_roadmap.md
├── drafts/              # Rust代码实现草稿
├── logs/                # 研究日志
│   ├── RESEARCH_LOG.md
│   └── DIRECTIONS_DYNAMIC.md
├── daily/               # 每日报告（自动生成）
├── RESEARCH_AGENT.md    # Agent任务说明
├── Start-Research.ps1   # 研究启动器脚本
├── .last-research       # 上次研究时间记录
└── README.md            # 项目说明
```

## 🔍 查看研究进展

### 查看日志
```powershell
# 研究日志
cat logs/RESEARCH_LOG.md

# 动态方向
cat logs/DIRECTIONS_DYNAMIC.md

# 每日报告
cat daily/2026-03-10.md
```

### 查看Git历史
```powershell
# 查看最近的研究提交
git log --oneline --grep="research"

# 查看特定方向的更新
git log --oneline --grep="核心原则"
```

## ⚙️ 配置说明

### 研究Agent配置
- **思考级别**: high（深度思考）
- **超时时间**: 1800秒（30分钟）
- **执行频率**: 每2小时一次
- **会话类型**: isolated（独立会话）

### Git工作流
- **master分支**: 开发分支，每次研究后推送
- **stable分支**: 稳定分支，每日23:45同步
- **标签**: daily-YYYY-MM-DD（每日报告快照）

## 🎯 研究产出

每次研究会产出：
1. **研究方向**: 本次聚焦的具体方向
2. **核心问题**: 需要回答的关键问题
3. **调研结果**: 搜索、阅读、思考的汇总
4. **架构洞察**: 对状态空间架构的新理解
5. **待验证假设**: 下一步需要验证的想法
6. **代码片段**: 相关的Rust实现草稿（可选）

## 📝 手动编辑研究方向

如果你想手动编辑某个研究方向：
1. 打开 `directions/XX_方向名.md` 文件
2. 添加你的研究内容
3. 提交到Git：
   ```bash
   git add directions/XX_方向名.md
   git commit -m "research: 手动更新XX方向"
   git push origin master
   ```

## 🐛 故障排除

### Agent没有自动触发
- 检查 `.last-research` 文件是否存在
- 使用 `Start-Research.ps1 -Status` 查看状态
- 使用 `Start-Research.ps1 -Force` 强制执行

### Git推送失败
- 检查网络连接
- 确保有推送权限
- 尝试 `git pull origin master` 先拉取更新

### 研究任务超时
- 增加timeoutSeconds参数（最大3600秒）
- 检查是否有太多并发任务
- 查看OpenClaw日志获取详细错误

## 📚 相关资源

- [研究计划](RESEARCH_PLAN.md)
- [Agent任务说明](RESEARCH_AGENT.md)
- [Cron配置](CRON_CONFIG.md)
- [GitHub仓库](https://github.com/YakuzaRyo/state-space-research)

---
*最后更新: 2026-03-10*
