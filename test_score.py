#!/usr/bin/env python3
import sys
sys.path.insert(0, 'system/scheduler')

# 模拟score_cli的主要逻辑
import json
import os
from datetime import datetime

SCORE_FILE = "system/scheduler/scores.json"

def ensure_file_exists():
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
        print(f"Created: {SCORE_FILE}")

def cmd_status():
    ensure_file_exists()
    with open(SCORE_FILE, 'r', encoding='utf-8') as f:
        data = json.load(f)
    print("=" * 40)
    print("评分状态")
    print("=" * 40)
    print(f"总评分: {data['total_score']}")
    print(f"总执行: {data['stats']['total_runs']}")
    print("=" * 40)

if __name__ == "__main__":
    cmd_status()
