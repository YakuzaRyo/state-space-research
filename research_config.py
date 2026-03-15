#!/usr/bin/env python3
"""
研究配置加载器
从 research_plan.json 加载研究方向配置
支持单一研究方向模式
"""

import json
import os
from pathlib import Path
from datetime import datetime

def load_research_plan(json_path: str = None) -> dict:
    """加载研究计划 JSON"""
    if json_path is None:
        json_path = Path(__file__).parent / "research_plan.json"

    with open(json_path, 'r', encoding='utf-8') as f:
        return json.load(f)

def get_current_direction(plan: dict = None) -> dict:
    """获取当前研究方向"""
    if plan is None:
        plan = load_research_plan()

    # 优先使用配置中的当前方向
    current = plan.get('current_direction', None)

    if current and current in plan['directions']:
        direction = plan['directions'][current]
        direction['_key'] = current
        return direction

    # 降级到时间模式
    hour = int(datetime.now().hour)
    for key, d in plan['directions'].items():
        if hour in d.get('hours', []):
            d['_key'] = key
            return d

    # 默认返回第一个
    first = list(plan['directions'].values())[0]
    first['_key'] = list(plan['directions'].keys())[0]
    return first

def get_direction_by_key(key: str, plan: dict = None) -> dict:
    """根据 key 获取研究方向"""
    if plan is None:
        plan = load_research_plan()

    if key in plan['directions']:
        direction = plan['directions'][key]
        direction['_key'] = key
        return direction
    return None

def get_directions_by_phase(phase: int, plan: dict = None) -> list:
    """根据阶段获取研究方向"""
    if plan is None:
        plan = load_research_plan()

    results = []
    for key, d in plan['directions'].items():
        if d.get('phase') == phase:
            d['_key'] = key
            results.append(d)
    return sorted(results, key=lambda x: x.get('priority', 999))

def get_next_direction(plan: dict = None) -> dict:
    """获取下一个研究方向（按优先级）"""
    if plan is None:
        plan = load_research_plan()

    current = plan.get('current_direction', None)

    # 找到当前方向的优先级
    current_priority = 0
    if current and current in plan['directions']:
        current_priority = plan['directions'][current].get('priority', 0)

    # 找下一个更高优先级的
    for key, d in sorted(plan['directions'].items(), key=lambda x: x[1].get('priority', 999)):
        if d.get('priority', 999) > current_priority:
            d['_key'] = key
            return d

    return None

def switch_direction(key: str) -> bool:
    """切换当前研究方向"""
    plan_path = Path(__file__).parent / "research_plan.json"
    plan = load_research_plan(plan_path)

    if key not in plan['directions']:
        return False

    plan['current_direction'] = key

    # 更新所有方向状态
    for k, d in plan['directions'].items():
        if k == key:
            d['status'] = 'active'
        else:
            d['status'] = 'pending'

    with open(plan_path, 'w', encoding='utf-8') as f:
        json.dump(plan, f, ensure_ascii=False, indent=2)

    return True

def print_current_direction():
    """打印当前研究方向"""
    direction = get_current_direction()
    plan = load_research_plan()

    print("=" * 60)
    print(f"研究模式: {plan.get('research_mode', 'single')}")
    print(f"当前方向: {direction.get('name', 'Unknown')}")
    print("=" * 60)
    print(f"核心问题: {direction.get('question', '')}")
    print(f"文档文件: {direction.get('file', '')}")
    print(f"优先级: {direction.get('priority', 0)}")
    print(f"阶段: {direction.get('phase', 0)}")
    print(f"状态: {direction.get('status', 'unknown')}")
    print("-" * 60)
    print("研究主题:")
    for topic in direction.get('topics', []):
        print(f"  - {topic}")
    print("=" * 60)

def print_integration_info():
    """打印整合信息"""
    plan = load_research_plan()
    integration = plan.get('integration', {})

    print("\n" + "=" * 60)
    print("框架整合目标")
    print("=" * 60)
    print(f"目标: {integration.get('target', 'N/A')}")
    print(f"描述: {integration.get('description', 'N/A')}")
    print("-" * 60)
    print("模块映射:")
    modules = integration.get('modules', {})
    for key, desc in modules.items():
        print(f"  {key}: {desc}")
    print("-" * 60)
    print("路线图:")
    for roadmap in integration.get('roadmap', []):
        print(f"  阶段 {roadmap.get('phase', 0)}: {roadmap.get('name', '')}")
        for dir_key in roadmap.get('directions', []):
            if dir_key in plan['directions']:
                print(f"    - {plan['directions'][dir_key]['name']}")
    print("=" * 60)

def print_phase_status():
    """打印阶段状态"""
    plan = load_research_plan()

    print("\n" + "=" * 60)
    print("研究阶段状态")
    print("=" * 60)

    # 按阶段分组
    phases = {}
    for key, d in plan['directions'].items():
        phase = d.get('phase', 0)
        if phase not in phases:
            phases[phase] = []
        phases[phase].append({
            'key': key,
            'name': d.get('name', ''),
            'priority': d.get('priority', 0),
            'status': d.get('status', 'unknown')
        })

    for phase in sorted(phases.keys()):
        print(f"\n阶段 {phase}:")
        for d in sorted(phases[phase], key=lambda x: x['priority']):
            status_icon = "✓" if d['status'] == 'active' else "○"
            print(f"  [{status_icon}] {d['name']} (优先级: {d['priority']})")

    print("=" * 60)

if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1:
        if sys.argv[1] == "--current":
            print_current_direction()
        elif sys.argv[1] == "--integration":
            print_integration_info()
        elif sys.argv[1] == "--phase":
            print_phase_status()
        elif sys.argv[1] == "--switch" and len(sys.argv) > 2:
            key = sys.argv[2]
            if switch_direction(key):
                print(f"已切换到研究方向: {key}")
            else:
                print(f"切换失败: 未知方向 {key}")
        else:
            print("用法:")
            print("  python3 research_config.py           # 测试加载")
            print("  python3 research_config.py --current  # 当前研究方向")
            print("  python3 research_config.py --integration # 整合信息")
            print("  python3 research_config.py --phase    # 阶段状态")
            print("  python3 research_config.py --switch <key> # 切换方向")
    else:
        plan = load_research_plan()
        print("=" * 60)
        print("研究计划信息")
        print("=" * 60)
        print(f"版本: {plan['version']}")
        print(f"名称: {plan['name']}")
        print(f"目标: {plan['research_goal']}")
        print(f"研究模式: {plan.get('research_mode', 'single')}")
        print(f"当前方向: {plan.get('current_direction', 'N/A')}")
        print(f"总分数: {plan['evaluation']['total_score']}")
        print(f"研究方向数: {len(plan['directions'])}")
        print("=" * 60)
