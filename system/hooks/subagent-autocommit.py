#!/usr/bin/env python3
"""
Subagent自动提交Hook

在研究agent完成时自动调用，将研究成果提交到git

用法:
    python subagent-autocommit.py <direction_id> <duration_seconds> <agent_id>
"""

import subprocess
import sys
import os
from datetime import datetime

# 研究方向映射
DIRECTION_MAP = {
    "llm-navigator": ("🧭", "llm-navigator", "08_llm_as_navigator"),
    "rust-types": ("🦀", "rust-types", "09_rust_type_system"),
    "structured-gen": ("📝", "structured-gen", "03_structured_generation"),
    "layered-design": ("🥪", "layered-design", "07_layered_design"),
    "type-constraints": ("🔒", "type-constraints", "05_type_constraints"),
    "engineering": ("🗺️", "engineering", "12_engineering_roadmap"),
}


def run_git(args, check=True):
    """运行git命令"""
    result = subprocess.run(
        ["git"] + args,
        capture_output=True,
        text=True,
        cwd=os.path.dirname(os.path.dirname(os.path.dirname(__file__)))
    )
    if check and result.returncode != 0:
        print(f"Git错误: {result.stderr}")
        return None
    return result.stdout.strip()


def get_git_status():
    """获取git状态"""
    status = run_git(["status", "--porcelain"], check=False)
    return status


def generate_commit_msg(direction_id, duration_sec, agent_id):
    """生成提交信息"""
    duration_min = duration_sec // 60

    # 获取方向信息
    emoji, scope, full_name = DIRECTION_MAP.get(
        direction_id, ("🔬", "research", direction_id)
    )

    # 根据时长确定质量等级
    if duration_min >= 30:
        quality = "深度研究"
        stars = "⭐⭐⭐"
    elif duration_min >= 25:
        quality = "完整研究"
        stars = "⭐⭐"
    elif duration_min >= 20:
        quality = "标准研究"
        stars = "⭐"
    else:
        quality = "快速探索"
        stars = "⚡"

    # 主提交信息
    subject = f"{emoji} research({scope}): {quality} ({duration_min}min)"

    # 详细描述
    body = f"""研究方向: {full_name}
研究时长: {duration_min}分钟 ({duration_sec}秒)
任务ID: {agent_id}
完成时间: {datetime.now().isoformat()}
质量评级: {stars}"""

    return subject, body


def auto_commit(direction_id, duration_sec, agent_id):
    """执行自动提交"""
    print("=" * 60)
    print("🤖 Subagent自动提交")
    print("=" * 60)

    # 检查git状态
    status = get_git_status()
    if not status:
        print("✓ 没有需要提交的变更")
        return 0

    files = [line for line in status.split('\n') if line.strip()]
    print(f"📁 待提交文件: {len(files)}个")

    # 生成提交信息
    subject, body = generate_commit_msg(direction_id, duration_sec, agent_id)

    print(f"\n📝 提交信息:\n   {subject}")
    print(f"\n📦 添加文件...")

    # 添加所有变更
    run_git(["add", "-A"])

    # 提交
    print(f"💾 执行提交...")
    run_git(["commit", "-m", subject, "-m", body])

    # 获取提交hash
    commit_hash = run_git(["rev-parse", "--short", "HEAD"])

    print(f"\n✅ 提交成功!")
    print(f"   Hash: {commit_hash}")

    # 更新评分
    try:
        update_score(direction_id, duration_sec)
    except Exception as e:
        print(f"⚠️  评分更新失败: {e}")

    print("=" * 60)
    return 0


def update_score(direction, duration_sec):
    """更新评分"""
    # 调用score_cli.py
    script_dir = os.path.dirname(os.path.abspath(__file__))
    score_cli = os.path.join(script_dir, "..", "scheduler", "score_cli.py")

    if os.path.exists(score_cli):
        subprocess.run(
            ["python", score_cli, "add", direction, str(duration_sec)],
            capture_output=True
        )
        print(f"📊 评分已更新")


def main():
    if len(sys.argv) < 4:
        print("用法: python subagent-autocommit.py <direction_id> <duration_sec> <agent_id>")
        print("\n方向ID:")
        for k in DIRECTION_MAP.keys():
            print(f"  - {k}")
        sys.exit(1)

    direction_id = sys.argv[1]
    duration_sec = int(sys.argv[2])
    agent_id = sys.argv[3]

    try:
        return auto_commit(direction_id, duration_sec, agent_id)
    except Exception as e:
        print(f"❌ 提交失败: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
