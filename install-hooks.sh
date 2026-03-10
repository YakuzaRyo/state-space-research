#!/bin/bash
#
# 安装Git Hooks
# 将自定义hooks安装到.git/hooks目录

set -e

echo "🔧 安装Git Hooks..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="$SCRIPT_DIR/.githooks"
GIT_HOOKS_DIR="$SCRIPT_DIR/.git/hooks"

# 检查目录
if [ ! -d "$HOOKS_DIR" ]; then
    echo "❌ 错误: .githooks目录不存在"
    exit 1
fi

# 备份现有的hooks
echo "📦 备份现有hooks..."
BACKUP_DIR="$SCRIPT_DIR/.githooks/backup-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

for hook in pre-commit prepare-commit-msg post-commit; do
    if [ -f "$GIT_HOOKS_DIR/$hook" ] && [ ! -f "$GIT_HOOKS_DIR/$hook.sample" ]; then
        cp "$GIT_HOOKS_DIR/$hook" "$BACKUP_DIR/"
        echo "  备份: $hook"
    fi
done

# 安装新hooks
echo ""
echo "🚀 安装新hooks..."

for hook in pre-commit prepare-commit-msg post-commit; do
    if [ -f "$HOOKS_DIR/$hook" ]; then
        cp "$HOOKS_DIR/$hook" "$GIT_HOOKS_DIR/"
        chmod +x "$GIT_HOOKS_DIR/$hook"
        echo "  ✓ 安装: $hook"
    fi
done

# 配置git使用项目目录下的hooks
git config core.hooksPath "$GIT_HOOKS_DIR"

echo ""
echo "✅ Git Hooks安装完成!"
echo ""
echo "已安装的hooks:"
ls -la "$GIT_HOOKS_DIR"/* 2>/dev/null | grep -v sample | awk '{print "  - " $9}'
echo ""
echo "📖 使用方法:"
echo "  git commit  # 自动触发hooks"
echo "  ./system/hooks/research-commit.sh <方向> <秒数> <任务ID>  # 研究提交"
