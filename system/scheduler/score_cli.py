#!/usr/bin/env python3
"""
评分管理CLI - 增量更新评分，无需读取整个JSON

用法:
    python score_cli.py add [direction] [duration_seconds]   # 添加一次运行记录
    python score_cli.py reduce [reason]                      # 减分（记录原因）
    python score_cli.py status                               # 显示当前状态
    python score_cli.py reset                                # 重置所有数据
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
                "above_25_min": 0,
                "below_20_min": 0,
                "avg_duration_seconds": 0
            }
        }
        with open(SCORE_FILE, 'w', encoding='utf-8') as f:
            json.dump(initial_data, f)


def calculate_score(duration_seconds: int) -> tuple:
    """计算单次评分"""
    if duration_seconds >= 1500:  # 25分钟
        return 1, "EXCELLENT"
    elif duration_seconds < 1200:  # 20分钟
        return -1, "INSUFFICIENT"
    else:
        return 0, "ACCEPTABLE"


def cmd_add(direction: str = "未知方向", duration: str = "1500"):
    """添加一次运行记录"""
    try:
        duration_seconds = int(duration)
    except ValueError:
        print(f"错误: 持续时间必须是整数秒")
        sys.exit(1)

    acquire_lock()
    try:
        ensure_file_exists()

        # 计算评分
        score, level = calculate_score(duration_seconds)

        # 读取并更新数据
        with open(SCORE_FILE, 'r+', encoding='utf-8') as f:
            data = json.load(f)

            # 添加记录
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

            if level == "EXCELLENT":
                data["stats"]["above_25_min"] += 1
            elif level == "INSUFFICIENT":
                data["stats"]["below_20_min"] += 1

            # 更新平均时间
            total_duration = sum(r["duration_seconds"] for r in data["runs"])
            data["stats"]["avg_duration_seconds"] = total_duration // len(data["runs"])

            # 写回文件
            f.seek(0)
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.truncate()

        # 输出结果
        duration_min = duration_seconds / 60
        symbol = "+" if score > 0 else ("-" if score < 0 else "=")
        print(f"[{symbol}] 记录添加成功: {direction}")
        print(f"    持续时间: {duration_min:.1f}分钟")
        print(f"    得分: {score:+d} ({level})")
        print(f"    总评分: {data['total_score']}")

    finally:
        release_lock()


def cmd_reduce(reason: str = "未说明原因"):
    """手动减分（记录惩罚）"""
    acquire_lock()
    try:
        ensure_file_exists()

        with open(SCORE_FILE, 'r+', encoding='utf-8') as f:
            data = json.load(f)

            # 添加惩罚记录
            penalty_record = {
                "timestamp": datetime.now().isoformat(),
                "direction": f"PENALTY: {reason}",
                "duration_seconds": 0,
                "score": -1,
                "level": "PENALTY"
            }
            data["runs"].append(penalty_record)
            data["total_score"] -= 1

            # 写回文件
            f.seek(0)
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.truncate()

        print(f"[-] 减分记录: {reason}")
        print(f"    当前总评分: {data['total_score']}")

    finally:
        release_lock()


def cmd_status():
    """显示当前状态"""
    ensure_file_exists()

    with open(SCORE_FILE, 'r', encoding='utf-8') as f:
        data = json.load(f)

    print("=" * 60)
    print("研究任务评分状态")
    print("=" * 60)
    print(f"总评分: {data['total_score']}")
    print(f"总执行次数: {data['stats']['total_runs']}")
    print(f"优秀(≥25分钟): {data['stats']['above_25_min']}")
    print(f"不足(<20分钟): {data['stats']['below_20_min']}")

    if data["stats"]["total_runs"] > 0:
        avg = data["stats"]["avg_duration_seconds"] / 60
        print(f"平均持续时间: {avg:.1f}分钟")

    # 最近5次
    if data["runs"]:
        print("\n最近5次记录:")
        for run in data["runs"][-5:]:
            symbol = "+" if run["score"] > 0 else ("-" if run["score"] < 0 else "=")
            duration_min = run["duration_seconds"] / 60 if run["duration_seconds"] > 0 else 0
            direction = run["direction"][:20]
            print(f"  [{symbol}] {direction:20s} {duration_min:5.1f}min {run['score']:+d}")

    print("=" * 60)


def cmd_reset():
    """重置所有数据"""
    acquire_lock()
    try:
        if os.path.exists(SCORE_FILE):
            backup = f"{SCORE_FILE}.backup.{datetime.now().strftime('%Y%m%d_%H%M%S')}"
            os.rename(SCORE_FILE, backup)
            print(f"已备份旧数据到: {backup}")

        ensure_file_exists()
        print("评分数据已重置")

    finally:
        release_lock()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    command = sys.argv[1].lower()

    if command == "add":
        direction = sys.argv[2] if len(sys.argv) > 2 else "未知方向"
        duration = sys.argv[3] if len(sys.argv) > 3 else "1500"
        cmd_add(direction, duration)

    elif command == "reduce":
        reason = " ".join(sys.argv[2:]) if len(sys.argv) > 2 else "未说明原因"
        cmd_reduce(reason)

    elif command == "status":
        cmd_status()

    elif command == "reset":
        cmd_reset()

    else:
        print(f"未知命令: {command}")
        print(__doc__)
        sys.exit(1)


if __name__ == "__main__":
    main()
