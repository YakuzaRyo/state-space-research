#!/bin/bash
# Cron 自动研究脚本
# 由 cron 定时触发，自动执行研究流程

# 设置工作目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 日志文件
LOG_FILE="$SCRIPT_DIR/logs/cron_$(date +%Y%m%d).log"

# 记录开始
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cron 任务开始" >> "$LOG_FILE"

# 获取当前时间
HOUR=$(date +%H)
MINUTE=$(date +%M)

echo "[INFO] 当前时间: $HOUR:$MINUTE" >> "$LOG_FILE"

# 检查是否在研究时间范围内 (9:00 - 23:30)
if [ "$HOUR" -lt 9 ] || [ "$HOUR" -ge 23 ]; then
    echo "[SKIP] 不在研究时间范围内 (9:00-23:00)" >> "$LOG_FILE"
    exit 0
fi

# 检查是否是30分钟整点 (00, 30)
if [ "$MINUTE" != "00" ] && [ "$MINUTE" != "30" ]; then
    echo "[SKIP] 不是研究时间点" >> "$LOG_FILE"
    exit 0
fi

# 检查是否有正在运行的实例
LOCK_FILE="/tmp/state_space_research.lock"
if [ -f "$LOCK_FILE" ]; then
    PID=$(cat "$LOCK_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "[SKIP] 已有实例在运行 (PID: $PID)" >> "$LOG_FILE"
        exit 0
    else
        echo "[WARN] 发现 stale lock 文件，删除" >> "$LOCK_FILE"
        rm -f "$LOCK_FILE"
    fi
fi

# 创建锁文件
echo $$ > "$LOCK_FILE"
trap "rm -f $LOCK_FILE" EXIT

# 检查 git 状态
if [ -n "$(git status --porcelain | grep -v results.tsv)" ]; then
    echo "[INFO] 发现未提交的更改，检查是否需要评估..." >> "$LOG_FILE"

    # 运行评估
    EVAL_OUTPUT=$(python3 evaluate.py . 2>&1)
    EVAL_EXIT=$?

    if [ $EVAL_EXIT -eq 0 ]; then
        echo "[OK] 评估成功" >> "$LOG_FILE"
        echo "$EVAL_OUTPUT" >> "$LOG_FILE"
    else
        echo "[ERROR] 评估失败 (退出码: $EVAL_EXIT)" >> "$LOG_FILE"
        echo "$EVAL_OUTPUT" >> "$LOG_FILE"
    fi
else
    echo "[INFO] 无未提交更改，跳过" >> "$LOG_FILE"
fi

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cron 任务完成" >> "$LOG_FILE"
echo "---" >> "$LOG_FILE"

exit 0
