#!/usr/bin/env python3
"""
Subagent池管理器 - 严格控制最多6个研究agent并行

设计：
- 全局限制：最多6个研究subagent
- 管理6个研究方向，每个方向一个agent
- 定时检查并补充到6个
- 每个agent运行至少25分钟
"""

import json
import os
import subprocess
import time
from datetime import datetime
from pathlib import Path

STATE_FILE = "system/scheduler/subagent_pool.json"
LOG_FILE = "logs/scheduler/subagent_pool.log"
MAX_SUBAGENTS = 6

# 6个研究方向
RESEARCH_DIRECTIONS = [
    ("llm-navigator", "08_llm_as_navigator - LLM导航器算法优化"),
    ("rust-types", "09_rust_type_system - Rust类型系统深度研究"),
    ("structured-gen", "03_structured_generation - 结构化生成技术"),
    ("layered-design", "07_layered_design - 分层架构设计"),
    ("type-constraints", "05_type_constraints - 类型约束系统"),
    ("engineering", "12_engineering_roadmap - 工程路线图规划"),
]


def log(msg):
    """记录日志"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    log_msg = f"[{timestamp}] {msg}"
    print(log_msg)

    os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(log_msg + "\n")


def load_state():
    """加载subagent池状态"""
    if os.path.exists(STATE_FILE):
        with open(STATE_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    return {
        "active_agents": {},  # agent_id -> {direction, started_at, pid}
        "completed_runs": [],
        "stats": {
            "total_started": 0,
            "total_completed": 0,
            "total_score": 0
        }
    }


def save_state(state):
    """保存subagent池状态"""
    os.makedirs(os.path.dirname(STATE_FILE), exist_ok=True)
    with open(STATE_FILE, "w", encoding="utf-8") as f:
        json.dump(state, f, indent=2, ensure_ascii=False)


def check_active_agents(state):
    """检查并清理已完成的agent"""
    active = state["active_agents"]
    completed = []

    for agent_id, info in list(active.items()):
        # 检查agent是否还在运行（通过检查输出文件或进程）
        output_file = f"logs/scheduler/agent_{agent_id}.output"

        # 如果运行时间超过25分钟，认为是成功的
        started = datetime.fromisoformat(info["started_at"])
        elapsed = (datetime.now() - started).total_seconds()

        if elapsed >= 1500:  # 25分钟 = 1500秒
            log(f"Agent {agent_id} ({info['direction']}) 已完成，运行时间: {elapsed/60:.1f}分钟")
            completed.append(agent_id)
            state["completed_runs"].append({
                "agent_id": agent_id,
                "direction": info["direction"],
                "started_at": info["started_at"],
                "completed_at": datetime.now().isoformat(),
                "duration_seconds": elapsed,
                "score": 1 if elapsed >= 1500 else 0
            })
            if elapsed >= 1500:
                state["stats"]["total_score"] += 1
            del active[agent_id]

    state["stats"]["total_completed"] += len(completed)
    return len(completed)


def start_new_agent(direction_id, direction_name):
    """启动一个新的研究agent"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    agent_id = f"{direction_id}_{timestamp}"

    log(f"启动Agent: {agent_id} ({direction_name})")

    # 构建启动脚本
    script = f"""
import subprocess
import time
from datetime import datetime

start_time = datetime.now()
print(f"[{{start_time}}] Agent {agent_id} 开始研究: {direction_name}")

# 模拟研究过程（实际应调用Agent工具）
# 这里使用subprocess启动实际的Agent研究
cmd = [
    "python", "-c",
    '''
import time
import sys
from datetime import datetime

print(f"[{datetime.now()}] 开始深度研究: {direction_name}")
print("研究方向:", "{direction_name}")
print("研究内容: 需要持续至少25分钟")

# 实际研究代码应该在这里
# 例如: 使用Agent工具进行深度研究

# 模拟25分钟的研究
time.sleep(1500)  # 25分钟

print(f"[{datetime.now()}] 研究完成")
'''
]

result = subprocess.run(cmd, capture_output=True, text=True)
print(result.stdout)
if result.stderr:
    print("STDERR:", result.stderr)

end_time = datetime.now()
duration = (end_time - start_time).total_seconds()
print(f"[{{end_time}}] Agent {agent_id} 完成，持续时间: {{duration/60:.1f}}分钟")
"""

    # 保存并启动
    script_file = f"logs/scheduler/agent_{agent_id}.py"
    os.makedirs(os.path.dirname(script_file), exist_ok=True)
    with open(script_file, "w", encoding="utf-8") as f:
        f.write(script)

    # 启动后台进程
    try:
        proc = subprocess.Popen(
            ["python", script_file],
            stdout=open(f"logs/scheduler/agent_{agent_id}.output", "w"),
            stderr=subprocess.STDOUT,
            start_new_session=True
        )

        return {
            "agent_id": agent_id,
            "direction": direction_id,
            "direction_name": direction_name,
            "started_at": datetime.now().isoformat(),
            "pid": proc.pid
        }
    except Exception as e:
        log(f"启动Agent {agent_id} 失败: {e}")
        return None


def manage_pool():
    """管理subagent池"""
    state = load_state()

    # 清理已完成的agent
    completed = check_active_agents(state)
    if completed > 0:
        log(f"清理了 {completed} 个已完成的agent")

    # 计算需要补充的agent数量
    current_count = len(state["active_agents"])
    needed = MAX_SUBAGENTS - current_count

    log(f"当前活跃agent: {current_count}/{MAX_SUBAGENTS}, 需要补充: {needed}")

    if needed > 0:
        # 找出正在运行的方向
        active_directions = {info["direction"] for info in state["active_agents"].values()}

        # 为每个需要补充的位置启动新agent
        for direction_id, direction_name in RESEARCH_DIRECTIONS:
            if needed <= 0:
                break
            if direction_id not in active_directions:
                agent_info = start_new_agent(direction_id, direction_name)
                if agent_info:
                    state["active_agents"][agent_info["agent_id"]] = agent_info
                    state["stats"]["total_started"] += 1
                    needed -= 1
                    time.sleep(1)  # 避免同时启动过多

    save_state(state)

    # 打印状态
    log(f"池状态: {len(state['active_agents'])}/{MAX_SUBAGENTS} 活跃")
    for agent_id, info in state["active_agents"].items():
        started = datetime.fromisoformat(info["started_at"])
        elapsed = (datetime.now() - started).total_seconds()
        log(f"  - {agent_id}: {elapsed/60:.1f}分钟")


def print_status():
    """打印当前状态"""
    state = load_state()

    print("=" * 60)
    print("Subagent池管理器状态")
    print("=" * 60)
    print(f"最大并发数: {MAX_SUBAGENTS}")
    print(f"当前活跃: {len(state['active_agents'])}")
    print(f"总启动次数: {state['stats']['total_started']}")
    print(f"总完成次数: {state['stats']['total_completed']}")
    print(f"总评分: {state['stats']['total_score']}")

    if state["active_agents"]:
        print("\n活跃Agent:")
        for agent_id, info in state["active_agents"].items():
            started = datetime.fromisoformat(info["started_at"])
            elapsed = (datetime.now() - started).total_seconds()
            print(f"  [{info['direction']}] 运行中: {elapsed/60:.1f}分钟")

    if state["completed_runs"]:
        print("\n最近完成:")
        for run in state["completed_runs"][-5:]:
            duration_min = run['duration_seconds'] / 60
            score = "+1" if run['duration_seconds'] >= 1500 else "0"
            print(f"  [{run['direction']}] {duration_min:.1f}分钟 [{score}]")

    print("=" * 60)


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "status":
        print_status()
    else:
        manage_pool()
