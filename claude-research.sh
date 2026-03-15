#!/bin/bash
# Claude Code 研究代理调用脚本
# 定时触发 Claude Code 执行研究任务

# 设置工作目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 日志文件
LOG_DIR="$SCRIPT_DIR/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/claude_$(date +%Y%m%d_%H%M%S).log"

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log "=========================================="
log "调用 Claude Code 执行研究"
log "=========================================="

# 获取当前研究方向
load_direction() {
    python3 -c "
import json
with open('research_plan.json', 'r') as f:
    plan = json.load(f)
    current = plan.get('current_direction', None)
    if current and current in plan['directions']:
        d = plan['directions'][current]
        print(d['name'] + '|' + d['file'] + '|' + d['question'] + '|' + current)
    else:
        print('工具设计|directions/10_tool_design.md|深入研究|tool_design')
" 2>/dev/null
}

DIRECTION_INFO=$(load_direction)

if [ -z "$DIRECTION_INFO" ]; then
    DIRECTION="工具设计"
    DOC="directions/10_tool_design.md"
    QUESTION="深入研究"
    DIR_KEY="tool_design"
else
    DIRECTION=$(echo "$DIRECTION_INFO" | cut -d'|' -f1)
    DOC=$(echo "$DIRECTION_INFO" | cut -d'|' -f2)
    QUESTION=$(echo "$DIRECTION_INFO" | cut -d'|' -f3)
    DIR_KEY=$(echo "$DIRECTION_INFO" | cut -d'|' -f4)
fi

log "研究方向: $DIRECTION ($DIR_KEY)"
log "核心问题: $QUESTION"

# 研究指令
RESEARCH_PROMPT="你在进行状态空间架构的自主研究。

当前研究方向: $DIRECTION
核心问题: $QUESTION

请按照以下步骤执行：

1. 阅读 $DOC 了解当前研究方向
2. 针对核心问题「$QUESTION」进行深度研究
3. 搜索相关论文和项目
4. 更新研究文档，添加：
   - ## 研究发现 (你的调研结果)
   - ## 架构洞察 (对新架构的新理解)
   - ## 待验证假设 (下一步需要验证的想法)
   - ## 下一步研究方向
5. 如有实现想法，在 drafts/ 目录创建 Rust 代码草稿

注意：
- 这是单一方向研究模式，专注当前方向
- 研究时间约 25-40 分钟
- 使用中文
- 完成后自动提交"

log "执行 Claude Code 研究..."

# 调用 Claude Code
cd "$SCRIPT_DIR"

claude -p \
    --dangerously-skip-permissions \
    --add-dir "$SCRIPT_DIR" \
    --max-budget-usd 5 \
    "$RESEARCH_PROMPT" 2>&1 | tee -a "$LOG_FILE"

EXIT_CODE=${PIPESTATUS[0]}

log "Claude Code 退出码: $EXIT_CODE"

if [ $EXIT_CODE -eq 0 ]; then
    log "研究完成，检查是否需要评估..."

    # 检查是否有新更改
    if [ -n "$(git status --porcelain | grep -v results.tsv)" ]; then
        log "发现新更改，运行评估..."

        # 运行评估
        EVAL_OUTPUT=$(python3 evaluate.py . 2>&1)
        EVAL_EXIT=$?

        echo "$EVAL_OUTPUT" | tee -a "$LOG_FILE"

        if [ $EVAL_EXIT -eq 0 ]; then
            log "评估成功"

            # 提交
            git add directions/ drafts/ evaluate.py research_plan.json 2>/dev/null || true
            git commit -m "research: $DIRECTION - 自主研究完成" 2>/dev/null || true

            # 推送
            git push origin master 2>/dev/null || log "推送失败"

            log "已提交并推送"
        else
            log "评估失败"
        fi
    else
        log "无新更改"
    fi
else
    log "研究执行失败"
fi

log "=========================================="
log "Claude Code 研究任务完成"
log "=========================================="

exit $EXIT_CODE
