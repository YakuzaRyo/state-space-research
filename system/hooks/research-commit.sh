#!/bin/bash
#
# Subagent研究任务自动提交Hook
# 在研究agent完成任务后自动执行git提交

set -e

# 参数
DIRECTION="$1"      # 研究方向
DURATION="$2"       # 持续时间（秒）
TASK_ID="$3"        # 任务ID

if [ -z "$DIRECTION" ]; then
    echo "❌ 错误: 未指定研究方向"
    exit 1
fi

# 转换为分钟
DURATION_MIN=$((DURATION / 60))

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🤖 Subagent研究任务自动提交"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📂 方向: ${DIRECTION}"
echo "⏱️  时长: ${DURATION_MIN}分钟"
echo "🆔 任务: ${TASK_ID}"
echo ""

# 检查是否有变更
if [ -z "$(git status --porcelain)" ]; then
    echo "✓ 没有需要提交的变更"
    exit 0
fi

# 获取变更文件列表
STAGED_FILES=$(git status --porcelain | wc -l)
echo "📁 待提交文件: ${STAGED_FILES}个"
git status --short | head -10
if [ "$STAGED_FILES" -gt 10 ]; then
    echo "... 等 $STAGED_FILES 个文件"
fi
echo ""

# 生成提交信息
# 根据方向选择emoji
 case "$DIRECTION" in
    *"llm-navigator"*|*"08"*)
        EMOJI="🧭"
        SCOPE="llm-navigator"
        ;;
    *"rust-type"*|*"09"*)
        EMOJI="🦀"
        SCOPE="rust-types"
        ;;
    *"structured"*|*"03"*)
        EMOJI="📝"
        SCOPE="structured-gen"
        ;;
    *"layered"*|*"07"*)
        EMOJI="🥪"
        SCOPE="layered-design"
        ;;
    *"type-constraint"*|*"05"*)
        EMOJI="🔒"
        SCOPE="type-constraints"
        ;;
    *"engineering"*|*"12"*)
        EMOJI="🗺️"
        SCOPE="engineering"
        ;;
    *)
        EMOJI="🔬"
        SCOPE="research"
        ;;
esac

# 根据持续时间生成评价
if [ "$DURATION_MIN" -ge 30 ]; then
    QUALITY="深度"
    QUALITY_EMOJI="⭐⭐⭐"
elif [ "$DURATION_MIN" -ge 25 ]; then
    QUALITY="完整"
    QUALITY_EMOJI="⭐⭐"
elif [ "$DURATION_MIN" -ge 20 ]; then
    QUALITY="标准"
    QUALITY_EMOJI="⭐"
else
    QUALITY="快速"
    QUALITY_EMOJI="⚡"
fi

# 构建提交信息
COMMIT_MSG="${EMOJI} research(${SCOPE}): ${DIRECTION} ${QUALITY}研究 (${DURATION_MIN}min) ${QUALITY_EMOJI}"

echo "📝 提交信息:"
echo "   ${COMMIT_MSG}"
echo ""

# 添加所有变更
echo "📦 添加文件到暂存区..."
git add -A

# 提交
echo "💾 执行提交..."
git commit -m "${COMMIT_MSG}" -m "研究时长: ${DURATION_MIN}分钟" -m "任务ID: ${TASK_ID}"

COMMIT_HASH=$(git rev-parse --short HEAD)
echo ""
echo "✅ 提交成功!"
echo "   Hash: ${COMMIT_HASH}"
echo ""

# 询问是否推送
read -p "🚀 是否推送到远程? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    BRANCH=$(git symbolic-ref --short HEAD)
    echo "推送至 origin/${BRANCH}..."
    git push origin "${BRANCH}"
    echo "✅ 推送完成!"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
