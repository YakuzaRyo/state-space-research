#!/usr/bin/env python3
"""
元调度器 (Meta Scheduler) - 定时任务自更新系统

功能：
- 每两天重新创建/更新定时任务配置
- 继承上一次的所有任务定义
- 自动版本递增
- 备份历史配置
- 基于执行历史动态调整参数

运行时机：
- 定期执行（每两天）
- 手动触发（配置变更时）
"""

import json
import os
import shutil
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any


class MetaScheduler:
    """元调度器 - 管理任务配置的演进"""

    CONFIG_PATH = "system/scheduler/tasks.json"
    BACKUP_DIR = "system/scheduler/backups"
    HISTORY_PATH = "system/scheduler/task_history.json"

    def __init__(self):
        self.config = self._load_config()
        self.history = self._load_history()

    def _load_config(self) -> dict:
        """加载当前配置"""
        with open(self.CONFIG_PATH, 'r', encoding='utf-8') as f:
            return json.load(f)

    def _load_history(self) -> dict:
        """加载执行历史"""
        if os.path.exists(self.HISTORY_PATH):
            with open(self.HISTORY_PATH, 'r', encoding='utf-8') as f:
                return json.load(f)
        return {'generations': [], 'task_stats': {}}

    def _save_history(self):
        """保存执行历史"""
        with open(self.HISTORY_PATH, 'w', encoding='utf-8') as f:
            json.dump(self.history, f, indent=2, ensure_ascii=False)

    def _backup_current_config(self):
        """备份当前配置"""
        os.makedirs(self.BACKUP_DIR, exist_ok=True)
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_path = f"{self.BACKUP_DIR}/tasks_{timestamp}_v{self.config['version']}.json"
        shutil.copy2(self.CONFIG_PATH, backup_path)
        print(f"[MetaScheduler] 配置已备份: {backup_path}")
        return backup_path

    def _analyze_task_performance(self) -> Dict[str, Any]:
        """分析任务执行性能，用于动态调整参数"""
        stats = {}

        # 从日志目录分析（简化版）
        log_dir = Path("logs/scheduler")
        if log_dir.exists():
            for log_file in log_dir.glob("*.log"):
                # 解析日志统计成功率、平均执行时间等
                pass

        # 从历史记录分析
        for gen in self.history.get('generations', []):
            for task_id, task_stats in gen.get('task_stats', {}).items():
                if task_id not in stats:
                    stats[task_id] = {
                        'total_runs': 0,
                        'success_count': 0,
                        'fail_count': 0,
                        'avg_duration': 0
                    }
                stats[task_id]['total_runs'] += task_stats.get('runs', 0)
                stats[task_id]['success_count'] += task_stats.get('success', 0)
                stats[task_id]['fail_count'] += task_stats.get('fail', 0)

        return stats

    def _evolve_task_config(self, task: dict, stats: dict) -> dict:
        """
        基于历史表现演进任务配置

        调整策略：
        - 成功率 < 50%: 降低频率，增加重试次数
        - 执行时间过长: 调整超时设置
        - 频繁失败: 禁用或降低并发
        """
        task_id = task['id']
        task_stats = stats.get(task_id, {})
        total = task_stats.get('total_runs', 0)

        if total == 0:
            return task

        success_rate = task_stats.get('success_count', 0) / total
        new_task = json.loads(json.dumps(task))  # 深拷贝

        # 根据成功率调整
        if success_rate < 0.5:
            print(f"[MetaScheduler] {task_id} 成功率低 ({success_rate:.1%})，调整配置")

            # 增加重试次数
            if 'retry' in new_task:
                new_task['retry']['max_attempts'] = min(
                    new_task['retry'].get('max_attempts', 1) + 1, 5
                )

            # 降低并发（如果是subagent任务）
            if 'concurrency' in new_task:
                new_task['concurrency']['max_subagents'] = max(
                    new_task['concurrency'].get('max_subagents', 1) - 1, 0
                )

        # 高成功率任务可以适当增加频率或并发
        elif success_rate > 0.9 and total > 10:
            print(f"[MetaScheduler] {task_id} 表现优秀 ({success_rate:.1%})，优化配置")
            # 可以在这里添加优化逻辑

        return new_task

    def _inherit_previous_tasks(self, new_config: dict) -> dict:
        """
        继承上一次的所有定时任务

        继承规则：
        - 保留所有非元任务（id != meta-scheduler）
        - 如果任务已在配置中，进行智能合并
        - 保持任务的执行历史连续性
        """
        # 加载上一代配置（从备份）
        backup_files = sorted(
            Path(self.BACKUP_DIR).glob("tasks_*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True
        )

        if not backup_files:
            print("[MetaScheduler] 无历史配置，使用当前配置")
            return new_config

        # 加载上一个版本
        with open(backup_files[0], 'r', encoding='utf-8') as f:
            previous_config = json.load(f)

        # 获取当前任务ID集合
        current_task_ids = {t['id'] for t in new_config['tasks']}

        # 继承历史任务
        inherited_count = 0
        for prev_task in previous_config.get('tasks', []):
            task_id = prev_task['id']

            # 跳过元调度器自身（避免重复）
            if task_id == 'meta-scheduler':
                continue

            # 如果任务不存在于当前配置，添加它
            if task_id not in current_task_ids:
                print(f"[MetaScheduler] 继承历史任务: {task_id}")
                new_config['tasks'].append(prev_task)
                inherited_count += 1

        print(f"[MetaScheduler] 共继承 {inherited_count} 个历史任务")
        return new_config

    def _increment_version(self, config: dict) -> dict:
        """递增配置版本"""
        version = config.get('version', '1.0')
        try:
            major, minor = version.split('.')
            new_version = f"{major}.{int(minor) + 1}"
        except ValueError:
            new_version = "1.1"

        config['version'] = new_version
        config['last_updated'] = datetime.now().isoformat()
        print(f"[MetaScheduler] 版本更新: {version} -> {new_version}")
        return config

    def run(self):
        """执行元调度"""
        print("=" * 60)
        print("[MetaScheduler] 开始重新创建定时任务")
        print("=" * 60)

        # 1. 备份当前配置
        backup_path = self._backup_current_config()

        # 2. 分析任务性能
        stats = self._analyze_task_performance()
        print(f"[MetaScheduler] 已分析 {len(stats)} 个任务的历史表现")

        # 3. 创建新配置（基于当前配置）
        new_config = json.loads(json.dumps(self.config))

        # 4. 继承历史任务
        new_config = self._inherit_previous_tasks(new_config)

        # 5. 演进每个任务的配置
        evolved_tasks = []
        for task in new_config['tasks']:
            if task['id'] != 'meta-scheduler':  # 元调度器自身不需要演进
                evolved_task = self._evolve_task_config(task, stats)
                evolved_tasks.append(evolved_task)
            else:
                evolved_tasks.append(task)

        new_config['tasks'] = evolved_tasks

        # 6. 递增版本
        new_config = self._increment_version(new_config)

        # 7. 添加生成记录
        generation = {
            'timestamp': datetime.now().isoformat(),
            'version': new_config['version'],
            'backup_path': backup_path,
            'task_count': len(new_config['tasks']),
            'task_ids': [t['id'] for t in new_config['tasks']],
            'stats_summary': {
                tid: {
                    'success_rate': s.get('success_count', 0) / max(s.get('total_runs', 1), 1),
                    'total_runs': s.get('total_runs', 0)
                }
                for tid, s in stats.items()
            }
        }
        self.history['generations'].append(generation)
        self.history['task_stats'] = stats

        # 8. 保存新配置
        with open(self.CONFIG_PATH, 'w', encoding='utf-8') as f:
            json.dump(new_config, f, indent=2, ensure_ascii=False)

        # 9. 保存历史
        self._save_history()

        print("=" * 60)
        print(f"[MetaScheduler] 完成！新配置版本: {new_config['version']}")
        print(f"[MetaScheduler] 总任务数: {len(new_config['tasks'])}")
        print("=" * 60)

        return new_config


def main():
    """CLI入口"""
    scheduler = MetaScheduler()
    scheduler.run()


if __name__ == '__main__':
    main()
