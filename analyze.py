#!/usr/bin/env python3
"""
研究进度可视化脚本
生成 ASCII 图表展示研究进度
"""

import sys
import os
from pathlib import Path

def load_results():
    """加载 results.tsv"""
    results_path = Path("results.tsv")
    if not results_path.exists():
        print("results.tsv 不存在")
        return []

    results = []
    with open(results_path) as f:
        lines = f.readlines()
        if len(lines) <= 1:
            return []

        header = lines[0].strip().split('\t')
        for line in lines[1:]:
            if line.strip():
                parts = line.strip().split('\t')
                if len(parts) >= 8:
                    results.append({
                        'commit': parts[0],
                        'score': float(parts[1]),
                        'doc': float(parts[2]),
                        'code': float(parts[3]),
                        'refs': int(parts[4]),
                        'hypo': int(parts[5]),
                        'verified': int(parts[6]) if len(parts) > 6 else 0,
                        'status': parts[7],
                        'desc': parts[8] if len(parts) > 8 else ''
                    })
    return results

def print_progress_chart(results):
    """打印进度图表"""
    print("\n" + "=" * 60)
    print("              研究进度图表")
    print("=" * 60)

    if not results:
        print("\n暂无数据")
        return

    # 统计
    total = len(results)
    keep = sum(1 for r in results if r['status'] == 'keep')
    discard = sum(1 for r in results if r['status'] == 'discard')
    avg_score = sum(r['score'] for r in results) / total

    print(f"\n总实验数: {total}")
    print(f"保留: {keep} | 丢弃: {discard}")
    print(f"平均分数: {avg_score:.1f}")

    # 分数趋势
    print("\n分数趋势 (每次实验):")
    print("-" * 40)

    for i, r in enumerate(results):
        score = r['score']
        status = r['status']
        commit = r['commit']

        # 绘制进度条
        bar_len = int(score / 2)
        bar = "█" * bar_len + "░" * (50 - bar_len)

        status_icon = "✓" if status == "keep" else "✗"

        print(f"{i+1:2d}. [{status_icon}] {commit} {score:5.1f} |{bar}|")

    # 最佳分数
    best = max(results, key=lambda x: x['score'])
    print("-" * 40)
    print(f"最佳: {best['score']} (commit: {best['commit']})")

    # 指标雷达
    print("\n\n指标分布:")
    print("-" * 40)

    latest = results[-1]
    print(f"文档质量: {latest['doc']:.1f}/40  {'█' * int(latest['doc']/4)}{'░' * (10-int(latest['doc']/4))}")
    print(f"代码质量: {latest['code']:.1f}/30  {'█' * int(latest['code']/3)}{'░' * (10-int(latest['code']/3))}")
    print(f"引用数量: {latest['refs']:3d}/15  {'█' * min(latest['refs'],15)}{'░' * (15-min(latest['refs'],15))}")
    print(f"新假设:   {latest['hypo']:3d}/10  {'█' * min(latest['hypo'],5)}{'░' * (5-min(latest['hypo'],5))}")
    print(f"已验证:   {latest['verified']:3d}/10  {'█' * min(latest['verified'],5)}{'░' * (5-min(latest['verified'],5))}")

    print("\n" + "=" * 60)

def print_summary():
    """打印摘要信息"""
    print("\n可用命令:")
    print("  python3 analyze.py         # 查看进度图表")
    print("  python3 evaluate.py .     # 运行评估")
    print("  ./run-research.sh         # 运行研究流程")
    print("  git log --oneline         # 查看提交历史")

if __name__ == "__main__":
    os.chdir("/home/ume/state-space-research")
    results = load_results()
    print_progress_chart(results)
    print_summary()
