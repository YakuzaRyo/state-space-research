#!/bin/bash
# 研究代理自动化脚本
# 自动执行研究、评估、记录结果

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "     状态空间研究代理 v2.0"
echo "=========================================="

# 获取当前时间
HOUR=$(date +%H)
DATE=$(date +%Y-%m-%d)
TIME_TAG=$(date +%H%M)

echo -e "${GREEN}[INFO]${NC} 当前时间: $DATE $(date +%H:%M)"
echo -e "${GREEN}[INFO]${NC} Hour: $HOUR"

# 确定研究方向
case $HOUR in
    00|12)
        DIRECTION="核心原则"
        DOC="01_core_principles.md"
        QUESTION="如何让错误在设计上不可能发生?"
        ;;
    02|14)
        DIRECTION="分层设计"
        DOC="07_layered_design.md"
        QUESTION="Syntax→Semantic→Pattern→Domain如何转换?"
        ;;
    04|16)
        DIRECTION="LLM导航器"
        DOC="08_llm_as_navigator.md"
        QUESTION="LLM作为启发式函数的理论基础?"
        ;;
    06|18)
        DIRECTION="实现技术"
        DOC="09_rust_type_system.md"
        QUESTION="如何用Rust类型系统实现状态空间?"
        ;;
    08|20)
        DIRECTION="工具设计"
        DOC="10_tool_design.md"
        QUESTION="如何设计'无法产生错误'的工具?"
        ;;
    10|22)
        DIRECTION="对比分析"
        DOC="11_comparison.md"
        QUESTION="Claude Code/OpenCode的根本缺陷是什么?"
        ;;
    *)
        # 默认研究方向
        DIRECTION="综合研究"
        DOC="10_tool_design.md"
        QUESTION="深入研究工具设计"
        ;;
esac

echo -e "${YELLOW}[INFO]${NC} 研究方向: $DIRECTION"
echo -e "${YELLOW}[INFO]${NC} 核心问题: $QUESTION"

# 保存研究开始时间
START_TIME=$(date +%s)

# 检查是否有待提交的更改
if [ -n "$(git status --porcelain | grep -v results.tsv)" ]; then
    echo -e "${YELLOW}[INFO]${NC} 发现未提交的更改"

    # 获取上次分数
    LAST_SCORE=$(tail -n 1 results.tsv | cut -f2)

    if [ -z "$LAST_SCORE" ] || [ "$LAST_SCORE" = "score" ]; then
        LAST_SCORE=0
    fi

    echo -e "${GREEN}[INFO]${NC} 上次分数: $LAST_SCORE"
else
    echo -e "${GREEN}[INFO]${NC} 无待提交更改，跳过评估"
    LAST_SCORE=0
fi

echo ""
echo -e "${GREEN}[STEP 1]${NC} 开始深度研究..."
echo "=========================================="

# 读取现有研究文档
if [ -f "directions/$DOC" ]; then
    echo -e "${YELLOW}[INFO]${NC} 阅读现有文档: directions/$DOC"
    # 显示最后的研究记录
    tail -n 30 "directions/$DOC" 2>/dev/null || true
fi

echo ""
echo -e "${GREEN}[INFO]${NC} 请进行深度研究 (25-40分钟)"
echo -e "${YELLOW}[提示]${NC} 研究方向: $DIRECTION"
echo -e "${YELLOW}[提示]${NC} 核心问题: $QUESTION"
echo ""
echo -e "${YELLOW}[提示]${NC} 研究完成后，请运行以下命令评估:"
echo -e "${YELLOW}        python3 evaluate.py .${NC}"
echo ""

# 等待用户完成研究
read -p "按回车键继续评估 (或输入 'skip' 跳过) " -r
if [[ $REPLY == "skip" ]]; then
    echo -e "${YELLOW}[INFO]${NC} 跳过评估"
    exit 0
fi

echo ""
echo -e "${GREEN}[STEP 2]${NC} 运行评估..."
echo "=========================================="

# 运行评估
python3 evaluate.py . | tee /tmp/eval_output.txt

# 提取分数
CURRENT_SCORE=$(grep "总分:" /tmp/eval_output.txt | awk '{print $2}')
DOC_QUALITY=$(grep "文档质量:" /tmp/eval_output.txt | awk '{print $2}')
CODE_QUALITY=$(grep "代码质量:" /tmp/eval_output.txt | awk '{print $2}')
REFERENCES=$(grep "引用数量:" /tmp/eval_output.txt | awk -F'[()]' '{print $2}' | cut -d' ' -f1)
HYPOTHESES=$(grep "新假设:" /tmp/eval_output.txt | awk -F'[()]' '{print $2}' | cut -d' ' -f1)
VERIFIED=$(grep "已验证:" /tmp/eval_output.txt | awk -F'[()]' '{print $2}' | cut -d' ' -f1)

echo ""
echo -e "${GREEN}[STEP 3]${NC} 记录结果..."
echo "=========================================="

# 获取当前 commit
COMMIT=$(git rev-parse --short HEAD)

# 决定 keep 或 discard
echo -e "${YELLOW}[INFO]${NC} 上次分数: $LAST_SCORE"
echo -e "${YELLOW}[INFO]${NC} 当前分数: $CURRENT_SCORE"

# 比较分数 (如果上次分数为0，则保留)
if [ "$LAST_SCORE" = "0" ] || (( $(echo "$CURRENT_SCORE >= $LAST_SCORE" | bc -l) )); then
    STATUS="keep"
    echo -e "${GREEN}[RESULT]${NC} 分数提高或持平 → KEEP"
else
    STATUS="discard"
    echo -e "${RED}[RESULT]${NC} 分数降低 → DISCARD"

    # 回退更改
    echo -e "${YELLOW}[INFO]${NC} 回退更改..."
    git checkout -- .
    echo -e "${GREEN}[INFO]${NC} 已回退更改"
fi

# 记录到 results.tsv
echo -e "${GREEN}[INFO]${NC} 记录到 results.tsv"
echo -e "commit\t$COMMIT\t$CURRENT_SCORE\t$DOC_QUALITY\t$CODE_QUALITY\t$REFERENCES\t$HYPOTHESES\t$VERIFIED\t$STATUS\t$DIRECTION - $QUESTION" >> results.tsv

# 计算研究时间
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
ELAPSED_MIN=$((ELAPSED / 60))

echo ""
echo -e "${GREEN}[STEP 4]${NC} 研究统计..."
echo "=========================================="
echo -e "总用时: ${ELAPSED_MIN}分钟"
echo -e "分数: $CURRENT_SCORE"
echo -e "状态: $STATUS"

# 提交 (仅当 keep 时)
if [ "$STATUS" = "keep" ]; then
    echo ""
    echo -e "${GREEN}[STEP 5]${NC} 提交更改..."
    echo "=========================================="

    git add directions/ drafts/ evaluate.py 2>/dev/null || true
    git commit -m "research($TIME_TAG): $DIRECTION - 得分: $CURRENT_SCORE 分"

    # 推送到 GitHub
    echo -e "${GREEN}[INFO]${NC} 推送到 GitHub..."
    git push origin master || echo -e "${RED}[ERROR]${NC} 推送失败"
fi

echo ""
echo "=========================================="
echo "     研究完成!"
echo "=========================================="
