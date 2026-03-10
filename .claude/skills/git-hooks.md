# Git Hooks - 自动化Git提交

自动化Git提交流程，包括代码检查、提交信息生成和subagent研究自动提交。

## 功能

### 1. Pre-commit Hook
- Rust代码格式检查 (rustfmt)
- Python语法检查
- 敏感信息检测
- 大文件警告

### 2. Prepare-commit-msg Hook
- 根据变更文件自动生成提交信息前缀
- 研究方向检测 (llm-navigator, rust-types等)
- 变更统计 (+insertions/-deletions)

### 3. Post-commit Hook
- 提交成功通知
- 自动记录到月度提交日志
- 显示提交统计

### 4. Subagent自动提交
研究agent完成后自动提交研究成果。

## 安装

```bash
./install-hooks.sh
```

## 使用方法

### 常规提交
```bash
git add .
git commit  # 自动触发hooks
```

### Subagent研究自动提交

**方式1: Shell脚本**
```bash
./system/hooks/research-commit.sh "llm-navigator" 2400 "agent-001"
```

**方式2: Python脚本**
```bash
python system/hooks/subagent-autocommit.py llm-navigator 2400 agent-001
```

**参数:**
- `方向ID`: llm-navigator, rust-types, structured-gen, layered-design, type-constraints, engineering
- `持续时间`: 秒数
- `任务ID`: agent标识符

### 提交信息格式

研究提交自动生成格式:
```
🧭 research(llm-navigator): 深度研究 (40min) ⭐⭐⭐

研究方向: 08_llm_as_navigator
研究时长: 40分钟 (2400秒)
任务ID: agent-001
完成时间: 2026-03-10T21:00:00
质量评级: ⭐⭐⭐
```

## 研究方向Emoji映射

| 方向 | Emoji | Scope |
|------|-------|-------|
| LLM导航器 | 🧭 | llm-navigator |
| Rust类型系统 | 🦀 | rust-types |
| 结构化生成 | 📝 | structured-gen |
| 分层架构 | 🥪 | layered-design |
| 类型约束 | 🔒 | type-constraints |
| 工程路线图 | 🗺️ | engineering |

## 质量评级

| 时长 | 评级 | Emoji |
|------|------|-------|
| ≥30分钟 | 深度研究 | ⭐⭐⭐ |
| ≥25分钟 | 完整研究 | ⭐⭐ |
| ≥20分钟 | 标准研究 | ⭐ |
| <20分钟 | 快速探索 | ⚡ |

## 文件位置

```
.githooks/
├── pre-commit           # 提交前检查
├── prepare-commit-msg   # 提交信息生成
└── post-commit          # 提交后处理

system/hooks/
├── research-commit.sh       # Shell提交脚本
└── subagent-autocommit.py   # Python自动提交
```
