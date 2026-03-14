#!/bin/bash
# 自动化研究脚本 - 非交互式版本
# 由 cron 或其他自动化工具调用

# 设置工作目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

log "=========================================="
log "     状态空间研究代理 (自动模式)"
log "=========================================="

# 获取当前时间
HOUR=$(date +%H)
DATE=$(date +%Y-%m-%d)
TIME_TAG=$(date +%H%M)

log "当前时间: $DATE $(date +%H:%M)"

# 从 JSON 配置文件加载研究方向
DIRECTION_INFO=$(python3 -c "
import json
import sys
hour = $HOUR
with open('research_plan.json', 'r') as f:
    plan = json.load(f)
    for d in plan['directions'].values():
        if hour in d['hours']:
            print(d['name'] + '|' + d['file'] + '|' + d['question'])
            sys.exit(0)
    # 默认
    d = list(plan['directions'].values())[0]
    print(d['name'] + '|' + d['file'] + '|' + d['question'])
" 2>/dev/null)

if [ -z "$DIRECTION_INFO" ]; then
    DIRECTION="综合研究"
    DOC="10_tool_design.md"
    QUESTION="深入研究"
else
    DIRECTION=$(echo "$DIRECTION_INFO" | cut -d'|' -f1)
    DOC=$(echo "$DIRECTION_INFO" | cut -d'|' -f2)
    QUESTION=$(echo "$DIRECTION_INFO" | cut -d'|' -f3)
fi

log "研究方向: $DIRECTION"
log "核心问题: $QUESTION"

# 保存研究开始时间
START_TIME=$(date +%s)

# 检查是否有待提交的更改
if [ -z "$(git status --porcelain | grep -v results.tsv)" ]; then
    log "无待提交更改，跳过"
    exit 0
fi

log "发现未提交的更改，运行评估..."

# 运行评估
EVAL_OUTPUT=$(python3 evaluate.py . 2>&1)
EVAL_EXIT=$?

echo "$EVAL_OUTPUT"

# 提取分数
if [ $EVAL_EXIT -eq 0 ]; then
    CURRENT_SCORE=$(echo "$EVAL_OUTPUT" | grep "总分:" | awk '{print $2}')
    DOC_QUALITY=$(echo "$EVAL_OUTPUT" | grep "文档质量:" | awk '{print $2}')
    CODE_QUALITY=$(echo "$EVAL_OUTPUT" | grep "代码质量:" | awk '{print $2}')
    REFERENCES=$(echo "$EVAL_OUTPUT" | grep "引用数量:" | awk -F'[()]' '{print $2}' | cut -d' ' -f1)
    HYPOTHESES=$(echo "$EVAL_OUTPUT" | grep "新假设:" | awk -F'[()]' '{print $2}' | cut -d' ' -f1)
    VERIFIED=$(echo "$EVAL_OUTPUT" | grep "已验证:" | awk -F'[()]' '{print $2}' | cut -d' ' -f1)
    EVAL_STATUS="success"

    log "评估成功，分数: $CURRENT_SCORE"
else
    CURRENT_SCORE=0
    DOC_QUALITY=0
    CODE_QUALITY=0
    REFERENCES=0
    HYPOTHESES=0
    VERIFIED=0
    EVAL_STATUS="error"

    log "评估失败，退出码: $EVAL_EXIT"
fi

# 获取上次分数
LAST_LINE=$(tail -n 1 results.tsv 2>/dev/null)
if [ -n "$LAST_LINE" ] && [ "$LAST_LINE" != "commit" ]; then
    LAST_SCORE=$(echo "$LAST_LINE" | cut -f2)
else
    LAST_SCORE=0
fi

# 决定 keep 或 discard
log "上次分数: $LAST_SCORE"
log "当前分数: $CURRENT_SCORE"

if [ "$LAST_SCORE" = "0" ] || [ "$CURRENT_SCORE" -ge "$LAST_SCORE" ] 2>/dev/null; then
    STATUS="keep"
    log "分数提高或持平 → KEEP"
else
    STATUS="discard"
    log "分数降低 → DISCARD"

    # 回退更改
    log "回退更改..."
    git checkout -- . 2>/dev/null || true
    log "已回退更改"
fi

# 记录到 results.tsv
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

if [ "$EVAL_STATUS" = "success" ]; then
    FINAL_STATUS="$STATUS"
else
    FINAL_STATUS="crash"
fi

echo -e "$COMMIT\t$CURRENT_SCORE\t$DOC_QUALITY\t$CODE_QUALITY\t$REFERENCES\t$HYPOTHESES\t$VERIFIED\t$FINAL_STATUS\t$DIRECTION - $QUESTION" >> results.tsv

# 计算研究时间
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
ELAPSED_MIN=$((ELAPSED / 60))

log "总用时: ${ELAPSED_MIN}分钟"
log "最终状态: $FINAL_STATUS"

# 提交 (仅当 keep 且评估成功时)
if [ "$FINAL_STATUS" = "keep" ] && [ "$EVAL_STATUS" = "success" ]; then
    log "提交更改..."

    git add directions/ drafts/ evaluate.py 2>/dev/null || true
    git commit -m "research($TIME_TAG): $DIRECTION - 得分: $CURRENT_SCORE 分" || true

    # 推送到 GitHub
    log "推送到 GitHub..."
    git push origin master 2>/dev/null || log "推送失败 (可能需要手动)"

    log "推送完成"
fi

log "=========================================="
log "     研究完成!"
log "=========================================="

exit 0
