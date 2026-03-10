#!/usr/bin/env python3
"""
研究任务评分追踪器

规则：
- 任务启动间隔：10分钟
- 评分要求：25分钟（1500秒）
- 80%阈值：20分钟（1200秒）
- 75%阈值：18.75分钟（1125秒）
- 低于75%：-1分
- 高于80%：+1分

目标：每次研究运行至少25分钟以上
设计：任务10分钟启动一次，但要求持续25分钟，因此会重叠执行
"""

import json
import os
from datetime import datetime
from pathlib import Path

SCORE_FILE = "system/scheduler/scores.json"


def load_scores():
    """加载评分记录"""
    if os.path.exists(SCORE_FILE):
        with open(SCORE_FILE, 'r', encoding='utf-8') as f:
            return json.load(f)
    return {
        "total_score": 0,
        "runs": [],
        "stats": {
            "total_runs": 0,
            "above_80_percent": 0,
            "below_75_percent": 0,
            "avg_duration_seconds": 0
        }
    }


def save_scores(scores):
    """保存评分记录"""
    os.makedirs(os.path.dirname(SCORE_FILE), exist_ok=True)
    with open(SCORE_FILE, 'w', encoding='utf-8') as f:
        json.dump(scores, f, indent=2, ensure_ascii=False)


def calculate_score(duration_seconds: int) -> dict:
    """
    计算单次评分

    目标持续时间：1500秒（25分钟）
    - >= 1200秒 (80% = 20分钟)：+1分
    - < 1125秒 (75% = 18.75分钟)：-1分
    - 1125-1200秒：0分
    """
    target_duration = 1500  # 25分钟目标
    threshold_excellent = 1500  # 25分钟 = +1分
    threshold_fail = 1200  # 20分钟 = 分界线

    if duration_seconds >= threshold_excellent:
        score = 1
        level = "EXCELLENT"
    elif duration_seconds < threshold_fail:
        score = -1
        level = "INSUFFICIENT"
    else:
        score = 0
        level = "ACCEPTABLE"

    percentage = (duration_seconds / target_duration) * 100

    return {
        "score": score,
        "level": level,
        "duration_seconds": duration_seconds,
        "percentage": round(percentage, 1),
        "target_duration": target_duration,
        "threshold_excellent": threshold_excellent,
        "threshold_fail": threshold_fail
    }


def record_run(task_id: str, duration_seconds: int, details: str = ""):
    """记录一次任务执行"""
    scores = load_scores()
    result = calculate_score(duration_seconds)

    run_record = {
        "timestamp": datetime.now().isoformat(),
        "task_id": task_id,
        "duration_seconds": duration_seconds,
        "score": result["score"],
        "level": result["level"],
        "percentage": result["percentage"],
        "details": details
    }

    scores["runs"].append(run_record)
    scores["total_score"] += result["score"]
    scores["stats"]["total_runs"] += 1

    if result["level"] == "EXCELLENT":
        scores["stats"]["above_80_percent"] += 1
    elif result["level"] == "INSUFFICIENT":
        scores["stats"]["below_75_percent"] += 1

    # 计算平均持续时间
    total_duration = sum(r["duration_seconds"] for r in scores["runs"])
    scores["stats"]["avg_duration_seconds"] = total_duration / len(scores["runs"])

    save_scores(scores)

    return result, scores["total_score"]


def print_status():
    """打印当前评分状态"""
    scores = load_scores()

    print("=" * 60)
    print("研究任务评分追踪器")
    print("规则: 任务每10分钟启动, 要求持续25分钟以上")
    print("=" * 60)
    print(f"总评分: {scores['total_score']}")
    print(f"总执行次数: {scores['stats']['total_runs']}")
    print(f"优秀次数(≥25分钟): {scores['stats']['above_80_percent']}")
    print(f"不足次数(<20分钟): {scores['stats']['below_75_percent']}")

    if scores["stats"]["total_runs"] > 0:
        target_duration = 1500  # 25分钟目标
        avg_pct = (scores["stats"]["avg_duration_seconds"] / target_duration) * 100
        avg_min = scores["stats"]["avg_duration_seconds"] / 60
        print(f"平均持续时间: {scores['stats']['avg_duration_seconds']:.0f}秒 ({avg_min:.1f}分钟, {avg_pct:.1f}%)")

    # 最近5次执行
    if scores["runs"]:
        print("\n最近5次执行:")
        for run in scores["runs"][-5:]:
            symbol = "+" if run["score"] > 0 else ("-" if run["score"] < 0 else "=")
            duration_min = run['duration_seconds'] / 60
            print(f"  [{symbol}] {run['timestamp'][:19]} | {duration_min:5.1f}分钟 | "
                  f"{run['percentage']:5.1f}% | {run['level']}")

    print("=" * 60)


if __name__ == "__main__":
    print_status()
