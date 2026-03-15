#!/bin/bash
# 自动化研究脚本 - 非交互式版本
# 支持单一研究方向模式，自动切换方向

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

# 加载当前研究方向
load_direction() {
    python3 -c "
import json
with open('research_plan.json', 'r') as f:
    plan = json.load(f)
    current = plan.get('current_direction', None)
    if current and current in plan['directions']:
        d = plan['directions'][current]
        print(d['name'] + '|' + d['file'] + '|' + d['question'] + '|' + str(d.get('phase', 0)) + '|' + str(d.get('priority', 0)))
    else:
        print('tool_design|directions/10_tool_design.md|深入研究|2|3')
" 2>/dev/null
}

DIRECTION_INFO=$(load_direction)

if [ -z "$DIRECTION_INFO" ]; then
    DIRECTION="tool_design"
    DOC="directions/10_tool_design.md"
    QUESTION="深入研究"
    PHASE=2
    PRIORITY=3
else
    DIRECTION=$(echo "$DIRECTION_INFO" | cut -d'|' -f1)
    DOC=$(echo "$DIRECTION_INFO" | cut -d'|' -f2)
    QUESTION=$(echo "$DIRECTION_INFO" | cut -d'|' -f3)
    PHASE=$(echo "$DIRECTION_INFO" | cut -d'|' -f4)
    PRIORITY=$(echo "$DIRECTION_INFO" | cut -d'|' -f5)
fi

log "研究方向: $DIRECTION (阶段: $PHASE, 优先级: $PRIORITY)"
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

# 提取分数 (使用新的简洁格式)
if [ $EVAL_EXIT -eq 0 ]; then
    CURRENT_SCORE=$(echo "$EVAL_OUTPUT" | grep "^score:" | awk '{print $2}')
    DOC_QUALITY=$(echo "$EVAL_OUTPUT" | grep "^doc_quality:" | awk '{print $2}')
    CODE_QUALITY=$(echo "$EVAL_OUTPUT" | grep "^code_quality:" | awk '{print $2}')
    REFERENCES=$(echo "$EVAL_OUTPUT" | grep "^references:" | awk '{print $2}')
    HYPOTHESES=$(echo "$EVAL_OUTPUT" | grep "^hypotheses:" | awk '{print $2}')
    VERIFIED=$(echo "$EVAL_OUTPUT" | grep "^verified:" | awk '{print $2}')
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

# 比较分数 (转换为整数比较)
LAST_SCORE_INT=$(echo "$LAST_SCORE" | cut -d'.' -f1 2>/dev/null || echo "0")
CURRENT_SCORE_INT=$(echo "$CURRENT_SCORE" | cut -d'.' -f1 2>/dev/null || echo "0")

if [ "$LAST_SCORE" = "0" ] || [ "$CURRENT_SCORE_INT" -ge "$LAST_SCORE_INT" ]; then
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

    git add directions/ drafts/ evaluate.py research_plan.json 2>/dev/null || true
    git commit -m "research($TIME_TAG): $DIRECTION - 得分: $CURRENT_SCORE 分" || true

    # 推送到 GitHub
    log "推送到 GitHub..."
    git push origin master 2>/dev/null || log "推送失败 (可能需要手动)"

    log "推送完成"

    # 检查是否需要切换研究方向
    # 当分数达到 100 时切换
    if [ "$CURRENT_SCORE_INT" -ge 100 ]; then
        log "分数已达 100，准备切换研究方向..."

        # 查找下一个研究方向
        NEXT_DIRECTION=$(python3 -c "
import json
with open('research_plan.json', 'r') as f:
    plan = json.load(f)

current = plan.get('current_direction', 'tool_design')
current_priority = plan['directions'].get(current, {}).get('priority', 0)

# 找下一个更高优先级的
next_key = None
next_priority = 999
for key, d in plan['directions'].items():
    p = d.get('priority', 999)
    if p > current_priority and p < next_priority:
        next_priority = p
        next_key = key

if next_key:
    print(next_key)
else:
    # 如果没有更高的优先级，回到第一个
    first_key = min(plan['directions'].items(), key=lambda x: x[1].get('priority', 999))[0]
    print(first_key)
" 2>/dev/null)

        if [ -n "$NEXT_DIRECTION" ] && [ "$NEXT_DIRECTION" != "$DIRECTION" ]; then
            log "切换到下一个研究方向: $NEXT_DIRECTION"

            # 更新 research_plan.json
            python3 -c "
import json
with open('research_plan.json', 'r') as f:
    plan = json.load(f)

current = '$DIRECTION'
next_dir = '$NEXT_DIRECTION'

# 更新所有方向状态
for k, d in plan['directions'].items():
    if k == next_dir:
        d['status'] = 'active'
    else:
        d['status'] = 'pending'

plan['current_direction'] = next_dir

with open('research_plan.json', 'w') as f:
    json.dump(plan, f, ensure_ascii=False, indent=2)

print(f'已切换到: {next_dir}')
"

            log "研究方向已切换"
        fi
    fi
fi

log "=========================================="
log "     研究完成!"
log "=========================================="

exit 0
