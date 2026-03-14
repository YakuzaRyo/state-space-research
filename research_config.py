#!/usr/bin/env python3
"""
研究配置加载器
从 research_plan.json 加载研究方向配置
"""

import json
import os
from pathlib import Path

def load_research_plan(json_path: str = None) -> dict:
    """加载研究计划 JSON"""
    if json_path is None:
        json_path = Path(__file__).parent / "research_plan.json"

    with open(json_path, 'r', encoding='utf-8') as f:
        return json.load(f)

def get_direction_by_hour(hour: int, plan: dict = None) -> dict:
    """根据小时数获取研究方向"""
    if plan is None:
        plan = load_research_plan()

    for key, direction in plan['directions'].items():
        if hour in direction['hours']:
            return direction

    # 默认返回第一个方向
    return list(plan['directions'].values())[0]

def get_all_directions(plan: dict = None) -> list:
    """获取所有研究方向"""
    if plan is None:
        plan = load_research_plan()
    return list(plan['directions'].values())

def print_current_direction(hour: int = None):
    """打印当前研究方向"""
    if hour is None:
        hour = int(__import__('datetime').datetime.now().strftime('%H'))

    plan = load_research_plan()
    direction = get_direction_by_hour(hour, plan)

    print("=" * 50)
    print(f"当前时间: {hour}:00")
    print("=" * 50)
    print(f"研究方向: {direction['name']}")
    print(f"核心问题: {direction['question']}")
    print(f"文档文件: {direction['file']}")
    print(f"时间窗口: {direction['hours']}")
    print("-" * 50)
    print("研究主题:")
    for topic in direction['topics']:
        print(f"  - {topic}")
    print("=" * 50)

def print_evaluation_info():
    """打印评估指标信息"""
    plan = load_research_plan()
    eval_config = plan['evaluation']

    print("\n" + "=" * 50)
    print("评估指标")
    print("=" * 50)

    for metric, config in eval_config['metrics'].items():
        if isinstance(config, dict) and 'weight' in config:
            print(f"\n{metric}: {config['weight']}分")
            if 'checks' in config:
                for check in config['checks']:
                    print(f"  - {check}")
            if 'per_item' in config:
                print(f"  每项: +{config['per_item']}分 (上限: {config['max']})")

def test_json():
    """测试 JSON 加载"""
    plan = load_research_plan()

    print("=" * 50)
    print("研究计划信息")
    print("=" * 50)
    print(f"版本: {plan['version']}")
    print(f"名称: {plan['name']}")
    print(f"目标: {plan['research_goal']}")
    print(f"总分数: {plan['evaluation']['total_score']}")
    print(f"研究方向数: {len(plan['directions'])}")

    # 测试按小时获取方向
    print("\n--- 按时间测试 ---")
    for hour in [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22]:
        direction = get_direction_by_hour(hour, plan)
        print(f"{hour:02d}:00 -> {direction['name']}")

    print_evaluation_info()

if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1:
        if sys.argv[1] == "--hour":
            hour = int(sys.argv[2]) if len(sys.argv) > 2 else None
            print_current_direction(hour)
        elif sys.argv[1] == "--eval":
            print_evaluation_info()
        else:
            print("用法:")
            print("  python3 research_config.py           # 测试加载")
            print("  python3 research_config.py --hour   # 当前研究方向")
            print("  python3 research_config.py --eval  # 评估指标")
    else:
        test_json()
