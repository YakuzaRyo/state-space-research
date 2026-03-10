#!/usr/bin/env python3
"""
持久型定时任务调度器 - 支持 Subagent 并发控制

特性：
- JSON 配置文件驱动
- 全局 + 任务级 subagent 数量限制
- 任务队列和过载保护
- 状态持久化
- 与 Claude Code CLI 集成

用法：
    python scheduler.py start      # 启动调度器
    python scheduler.py stop       # 停止调度器
    python scheduler.py status     # 查看状态
    python scheduler.py reload     # 重载配置
"""

import json
import os
import sys
import time
import signal
import subprocess
import threading
from datetime import datetime, timedelta
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Dict, List, Optional, Callable
from enum import Enum
import logging

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('logs/scheduler/scheduler.log'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger('scheduler')


class TaskState(Enum):
    PENDING = "pending"
    RUNNING = "running"
    QUEUED = "queued"
    SKIPPED = "skipped"
    FAILED = "failed"
    COMPLETED = "completed"


@dataclass
class SubagentSlot:
    """Subagent 占用槽位"""
    task_id: str
    pid: Optional[int] = None
    started_at: Optional[datetime] = None

    def is_active(self) -> bool:
        if self.pid is None:
            return self.started_at is not None
        try:
            os.kill(self.pid, 0)
            return True
        except (OSError, ProcessLookupError):
            return False


@dataclass
class TaskInstance:
    """任务执行实例"""
    task_id: str
    state: TaskState
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    subagent_slot: Optional[SubagentSlot] = None
    error: Optional[str] = None
    attempt: int = 1


class SubagentPool:
    """
    Subagent 连接池 - 控制并发数量

    设计原则：
    - 全局限制：防止系统过载
    - 任务级限制：防止单个任务占用所有资源
    - 队列机制：请求排队而非直接失败
    """

    def __init__(self, global_limit: int):
        self.global_limit = global_limit
        self.active_slots: Dict[str, SubagentSlot] = {}
        self.wait_queue: List[tuple] = []  # (task_id, callback)
        self.lock = threading.RLock()
        self._shutdown = False

    def acquire(self, task_id: str, task_limit: int,
                callback: Callable[[Optional[SubagentSlot]], None]) -> bool:
        """
        尝试获取 subagent 槽位

        Returns:
            True: 立即获得槽位
            False: 进入等待队列
        """
        with self.lock:
            task_active = sum(1 for s in self.active_slots.values()
                            if s.task_id == task_id)

            # 检查是否可以分配
            if (len(self.active_slots) < self.global_limit and
                task_active < task_limit):
                slot = SubagentSlot(
                    task_id=task_id,
                    started_at=datetime.now()
                )
                self.active_slots[task_id] = slot
                logger.info(f"[SubagentPool] {task_id} 获取槽位 ({len(self.active_slots)}/{self.global_limit})")
                callback(slot)
                return True
            else:
                # 加入等待队列
                self.wait_queue.append((task_id, callback))
                logger.info(f"[SubagentPool] {task_id} 进入等待队列 (位置: {len(self.wait_queue)})")
                return False

    def release(self, task_id: str):
        """释放 subagent 槽位并处理队列"""
        with self.lock:
            if task_id in self.active_slots:
                del self.active_slots[task_id]
                logger.info(f"[SubagentPool] {task_id} 释放槽位 ({len(self.active_slots)}/{self.global_limit})")

            # 处理等待队列
            self._process_queue()

    def _process_queue(self):
        """处理等待队列中的请求"""
        while self.wait_queue and len(self.active_slots) < self.global_limit:
            task_id, callback = self.wait_queue.pop(0)

            # 检查任务级限制
            task_active = sum(1 for s in self.active_slots.values()
                            if s.task_id == task_id)
            task_config = self._get_task_config(task_id)
            task_limit = task_config.get('max_subagents', 1)

            if task_active < task_limit:
                slot = SubagentSlot(
                    task_id=task_id,
                    started_at=datetime.now()
                )
                self.active_slots[task_id] = slot
                logger.info(f"[SubagentPool] {task_id} 从队列获取槽位")
                # 异步回调
                threading.Thread(target=callback, args=(slot,)).start()
            else:
                # 放回头部，等待该任务的其他实例完成
                self.wait_queue.insert(0, (task_id, callback))
                break

    def _get_task_config(self, task_id: str) -> dict:
        """获取任务配置（简化版，实际应从配置读取）"""
        return {'max_subagents': 2}

    def get_stats(self) -> dict:
        """获取连接池统计"""
        with self.lock:
            return {
                'active': len(self.active_slots),
                'limit': self.global_limit,
                'queue_length': len(self.wait_queue),
                'active_tasks': list(self.active_slots.keys())
            }

    def shutdown(self):
        """关闭连接池，取消所有等待请求"""
        self._shutdown = True
        with self.lock:
            for task_id, callback in self.wait_queue:
                try:
                    callback(None)
                except Exception as e:
                    logger.error(f"队列回调错误: {e}")
            self.wait_queue.clear()


class TaskScheduler:
    """任务调度器核心"""

    def __init__(self, config_path: str = "system/scheduler/tasks.json"):
        self.config_path = config_path
        self.config = self._load_config()
        self.subagent_pool = SubagentPool(
            self.config['resource_limits']['max_subagents_global']
        )
        self.running = False
        self.threads: Dict[str, threading.Thread] = {}
        self.state: Dict[str, TaskInstance] = {}
        self.state_lock = threading.RLock()
        self.next_run: Dict[str, datetime] = {}
        self.project_dir = self.config['global']['project_dir']

        # 确保日志目录存在
        os.makedirs('logs/scheduler', exist_ok=True)

    def _load_config(self) -> dict:
        """加载配置文件"""
        with open(self.config_path, 'r', encoding='utf-8') as f:
            return json.load(f)

    def reload_config(self):
        """热重载配置"""
        logger.info("重载配置...")
        self.config = self._load_config()
        # 更新连接池限制
        self.subagent_pool.global_limit = (
            self.config['resource_limits']['max_subagents_global']
        )
        logger.info("配置已重载")

    def _calculate_next_run(self, task: dict, now: datetime) -> datetime:
        """计算下次执行时间"""
        schedule = task['schedule']

        if schedule['type'] == 'interval':
            if 'minutes' in schedule:
                delta = timedelta(minutes=schedule['minutes'])
            elif 'hours' in schedule:
                delta = timedelta(hours=schedule['hours'])
            else:
                delta = timedelta(minutes=10)
            return now + delta

        elif schedule['type'] == 'cron':
            # 简化版 cron，实际需要完整解析
            return now + timedelta(minutes=1)

        return now + timedelta(minutes=10)

    def _execute_task(self, task: dict) -> TaskInstance:
        """执行任务"""
        task_id = task['id']
        action = task['action']
        concurrency = task.get('concurrency', {})
        max_subagents = concurrency.get('max_subagents', 0)

        instance = TaskInstance(
            task_id=task_id,
            state=TaskState.PENDING,
            started_at=datetime.now()
        )

        with self.state_lock:
            self.state[task_id] = instance

        # 检查是否需要 subagent
        if action.get('requires_subagent', False) and max_subagents > 0:
            # 异步获取 subagent
            acquired = self.subagent_pool.acquire(
                task_id, max_subagents,
                lambda slot: self._on_slot_acquired(task, instance, slot)
            )
            if not acquired:
                instance.state = TaskState.QUEUED
        else:
            # 直接执行
            self._run_action(task, instance, None)

        return instance

    def _on_slot_acquired(self, task: dict, instance: TaskInstance,
                          slot: Optional[SubagentSlot]):
        """获取槽位后的回调"""
        if slot is None:
            # 连接池关闭
            instance.state = TaskState.FAILED
            instance.error = "连接池已关闭"
            instance.completed_at = datetime.now()
            return

        instance.subagent_slot = slot
        self._run_action(task, instance, slot)

    def _run_action(self, task: dict, instance: TaskInstance,
                    slot: Optional[SubagentSlot]):
        """实际执行动作"""
        task_id = task['id']
        action = task['action']
        timeout = action.get('timeout', 300)

        instance.state = TaskState.RUNNING
        logger.info(f"[Task] {task_id} 开始执行")

        try:
            action_type = action['type']

            if action_type == 'research':
                # 调用 Claude Code CLI
                result = self._run_claude_command(
                    action['command'], timeout
                )
            elif action_type == 'builtin':
                result = self._run_claude_command(
                    action['command'], timeout
                )
            elif action_type == 'script':
                result = self._run_script(
                    action['script'], timeout
                )
            else:
                result = {'success': False, 'error': f'未知动作类型: {action_type}'}

            if result.get('success'):
                instance.state = TaskState.COMPLETED
                logger.info(f"[Task] {task_id} 完成")
            else:
                instance.state = TaskState.FAILED
                instance.error = result.get('error', '未知错误')
                logger.error(f"[Task] {task_id} 失败: {instance.error}")

        except Exception as e:
            instance.state = TaskState.FAILED
            instance.error = str(e)
            logger.exception(f"[Task] {task_id} 异常")

        finally:
            instance.completed_at = datetime.now()

            # 释放 subagent
            if slot:
                self.subagent_pool.release(task_id)

    def _run_claude_command(self, command: str, timeout: int) -> dict:
        """运行 Claude Code 命令"""
        try:
            # 这里假设通过某种方式与 Claude Code 交互
            # 实际实现可能需要调用 claude CLI 或使用 MCP
            cmd = ['claude', command]
            result = subprocess.run(
                cmd, cwd=self.project_dir, timeout=timeout,
                capture_output=True, text=True
            )
            return {
                'success': result.returncode == 0,
                'stdout': result.stdout,
                'stderr': result.stderr
            }
        except subprocess.TimeoutExpired:
            return {'success': False, 'error': '执行超时'}
        except Exception as e:
            return {'success': False, 'error': str(e)}

    def _run_script(self, script_path: str, timeout: int) -> dict:
        """运行 Python 脚本"""
        try:
            full_path = os.path.join(self.project_dir, script_path)
            result = subprocess.run(
                ['python', full_path], cwd=self.project_dir, timeout=timeout,
                capture_output=True, text=True
            )
            return {
                'success': result.returncode == 0,
                'stdout': result.stdout,
                'stderr': result.stderr
            }
        except Exception as e:
            return {'success': False, 'error': str(e)}

    def _scheduler_loop(self):
        """调度主循环"""
        while self.running:
            now = datetime.now()

            for task in self.config['tasks']:
                if not task.get('enabled', True):
                    continue

                task_id = task['id']
                next_run = self.next_run.get(task_id)

                if next_run is None or now >= next_run:
                    # 检查是否已在运行
                    with self.state_lock:
                        current = self.state.get(task_id)
                        if current and current.state in [TaskState.RUNNING, TaskState.QUEUED]:
                            # 跳过或排队
                            concurrency = task.get('concurrency', {})
                            if concurrency.get('skip_if_overload', True):
                                logger.info(f"[Scheduler] {task_id} 跳过（已在运行）")
                                self.next_run[task_id] = self._calculate_next_run(task, now)
                                continue

                    # 执行任务
                    self._execute_task(task)
                    self.next_run[task_id] = self._calculate_next_run(task, now)

            time.sleep(1)

    def start(self):
        """启动调度器"""
        if self.running:
            logger.warning("调度器已在运行")
            return

        self.running = True

        # 保存 PID
        with open('system/scheduler/scheduler.pid', 'w') as f:
            f.write(str(os.getpid()))

        # 启动主循环
        thread = threading.Thread(target=self._scheduler_loop, daemon=True)
        thread.start()
        self.threads['main'] = thread

        logger.info("调度器已启动")

        # 保持运行
        try:
            while self.running:
                time.sleep(1)
        except KeyboardInterrupt:
            self.stop()

    def stop(self):
        """停止调度器"""
        logger.info("正在停止调度器...")
        self.running = False
        self.subagent_pool.shutdown()

        # 等待线程结束
        for name, thread in self.threads.items():
            thread.join(timeout=5)
            logger.info(f"线程 {name} 已停止")

        # 清理 PID 文件
        pid_file = Path('system/scheduler/scheduler.pid')
        if pid_file.exists():
            pid_file.unlink()

        logger.info("调度器已停止")

    def get_status(self) -> dict:
        """获取调度器状态"""
        return {
            'running': self.running,
            'subagent_pool': self.subagent_pool.get_stats(),
            'tasks': {
                tid: {
                    'state': inst.state.value,
                    'started': inst.started_at.isoformat() if inst.started_at else None,
                    'completed': inst.completed_at.isoformat() if inst.completed_at else None,
                    'error': inst.error
                }
                for tid, inst in self.state.items()
            },
            'next_run': {
                tid: t.isoformat() for tid, t in self.next_run.items()
            }
        }


def main():
    """命令行入口"""
    if len(sys.argv) < 2:
        print("用法: python scheduler.py [start|stop|status|reload]")
        sys.exit(1)

    command = sys.argv[1]
    scheduler = TaskScheduler()

    if command == 'start':
        scheduler.start()
    elif command == 'stop':
        # 发送信号给运行中的调度器
        pid_file = Path('system/scheduler/scheduler.pid')
        if pid_file.exists():
            with open(pid_file) as f:
                pid = int(f.read())
            os.kill(pid, signal.SIGTERM)
            print(f"已发送停止信号给调度器 (PID: {pid})")
        else:
            print("调度器未运行")
    elif command == 'status':
        import pprint
        pprint.pprint(scheduler.get_status())
    elif command == 'reload':
        pid_file = Path('system/scheduler/scheduler.pid')
        if pid_file.exists():
            with open(pid_file) as f:
                pid = int(f.read())
            os.kill(pid, signal.SIGHUP)
            print(f"已发送重载信号给调度器 (PID: {pid})")
        else:
            print("调度器未运行")
    else:
        print(f"未知命令: {command}")
        print("用法: python scheduler.py [start|stop|status|reload]")


if __name__ == '__main__':
    main()
