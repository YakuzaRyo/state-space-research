#!/usr/bin/env python3
"""
研究任务评分管理 CLI

用法:
    python score_cli.py add <方向> <持续时间秒>    # 添加评分记录
    python score_cli.py status                       # 显示状态
    python score_cli.py adjust <分数> [原因]        # 手动调整分数
    python score_cli.py reset                        # 重置数据
"""

import json
import os
import sys
from datetime import datetime
from pathlib import Path

SCORE_FILE = "system/scheduler/scores.json"
LOCK_FILE = "system/scheduler/scores.lock"


def acquire_lock():
    """获取文件锁，防止并发写入"""
    import time
    while os.path.exists(LOCK_FILE):
        time.sleep(0.01)
    with open(LOCK_FILE, 'w') as f:
        f.write(str(os.getpid()))


def release_lock():
    """释放文件锁"""
    if os.path.exists(LOCK_FILE):
        os.remove(LOCK_FILE)


def ensure_file_exists():
    """确保评分文件存在"""
    if not os.path.exists(SCORE_FILE):
        os.makedirs(os.path.dirname(SCORE_FILE), exist_ok=True)
        initial_data = {
            "total_score": 0,
            "runs": [],
            "stats": {
                "total_runs": 0,
                "excellent": 0,
                "acceptable": 0,
                "insufficient": 0,
                "total_duration_seconds": 0
            }
        }
        with open(SCORE_FILE, 'w', encoding='utf-8') as f:
            json.dump(initial_data, f, indent=2)


def load_data():
    """加载评分数据"""
    ensure_file_exists()
    with open(SCORE_FILE, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_data(data):
    """保存评分数据"""
    with open(SCORE_FILE, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)


def calculate_score(duration_seconds: int) -> tuple:
    """计算单次评分"""
    if duration_seconds >= 1500:  # 25分钟
        return 1, "EXCELLENT"
    elif duration_seconds < 1200:  # 20分钟
        return -1, "INSUFFICIENT"
    else:
        return 0, "ACCEPTABLE"


def format_duration(seconds: int) -> str:
    """格式化持续时间为可读格式"""
    minutes = seconds // 60
    if minutes >= 60:
        hours = minutes // 60
        mins = minutes % 60
        return f"{hours}h{mins}m"
    return f"{minutes}m"


def cmd_add(direction: str, duration_str: str):
    """添加评分记录"""
    try:
        duration_seconds = int(duration_str)
    except ValueError:
        print(f"❌ 错误: 持续时间必须是整数秒")
        sys.exit(1)

    acquire_lock()
    try:
        data = load_data()
        score, level = calculate_score(duration_seconds)

        run_record = {
            "timestamp": datetime.now().isoformat(),
            "direction": direction,
            "duration_seconds": duration_seconds,
            "score": score,
            "level": level
        }

        data["runs"].append(run_record)
        data["total_score"] += score
        data["stats"]["total_runs"] += 1
        data["stats"]["total_duration_seconds"] += duration_seconds

        if level == "EXCELLENT":
            data["stats"]["excellent"] += 1
        elif level == "ACCEPTABLE":
            data["stats"]["acceptable"] += 1
        else:
            data["stats"]["insufficient"] += 1

        save_data(data)

        symbol = "+" if score > 0 else ("-" if score < 0 else "=")
        print(f"[{symbol}] {direction}")
        print(f"    时长: {format_duration(duration_seconds)}")
        print(f"    得分: {score:+d} ({level})")
        print(f"    总分: {data['total_score']}")

    finally:
        release_lock()


def cmd_status():
    """显示评分状态"""
    data = load_data()

    print("=" * 50)
    print("📊 研究任务评分")
    print("=" * 50)
    print(f"总评分: {data['total_score']}")
    print(f"总次数: {data['stats']['total_runs']}")

    if data["stats"]["total_runs"] > 0:
        stats = data["stats"]
        avg = data["stats"]["total_duration_seconds"] // data["stats"]["total_runs"]
        print(f"平均时长: {format_duration(avg)}")
        print(f"")
        print(f"  ⭐ EXCELLENT (≥25min): {stats['excellent']}")
        print(f"  ✓ ACCEPTABLE (20-25min): {stats['acceptable']}")
        print(f"  ✗ INSUFFICIENT (<20min): {stats['insufficient']}")

    if data["runs"]:
        print(f"")
        print("最近5次记录:")
        for run in data["runs"][-5:]:
            symbol = "+" if run["score"] > 0 else ("-" if run["score"] < 0 else "=")
            duration = format_duration(run["duration_seconds"])
            direction = run["direction"][:20]
            print(f"  [{symbol}] {direction:20s} {duration:6s} {run['score']:+d}")

    print("=" * 50)


def cmd_adjust(score_str: str, *reason_parts):
    """手动调整分数"""
    try:
        score = int(score_str)
    except ValueError:
        print(f"❌ 错误: 分数必须是整数")
        sys.exit(1)

    reason = " ".join(reason_parts) if reason_parts else "手动调整"

    acquire_lock()
    try:
        data = load_data()

        record = {
            "timestamp": datetime.now().isoformat(),
            "direction": f"ADJUST: {reason}",
            "duration_seconds": 0,
            "score": score,
            "level": "ADJUSTMENT"
        }

        data["runs"].append(record)
        data["total_score"] += score

        save_data(data)

        symbol = "+" if score > 0 else "-"
        print(f"[{symbol}] 调整: {reason}")
        print(f"    变化: {score:+d}")
        print(f"    总分: {data['total_score']}")

    finally:
        release_lock()


def cmd_reset():
    """重置数据"""
    acquire_lock()
    try:
        if os.path.exists(SCORE_FILE):
            backup = f"{SCORE_FILE}.backup.{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            os.rename(SCORE_FILE, backup)
            print(f"💾 已备份: {backup}")

        ensure_file_exists()
        print("✅ 评分数据已重置")

    finally:
        release_lock()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    command = sys.argv[1].lower()

    if command == "add":
        if len(sys.argv) < 4:
            print("❌ 用法: score add <方向> <持续时间秒>")
            sys.exit(1)
        cmd_add(sys.argv[2], sys.argv[3])

    elif command == "status":
        cmd_status()

    elif command == "adjust":
        if len(sys.argv) < 3:
            print("❌ 用法: score adjust <分数> [原因]")
            sys.exit(1)
        cmd_adjust(sys.argv[2], *sys.argv[3:])

    elif command == "reset":
        cmd_reset()

    else:
        print(f"❌ 未知命令: {command}")
        print(__doc__)
        sys.exit(1)


if __name__ == "__main__":
    main()
