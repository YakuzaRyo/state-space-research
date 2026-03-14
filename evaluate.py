#!/usr/bin/env python3
"""
评估函数 - 固定的 ground truth
AI Coding 框架研究任务的评估指标

研究目标：通过程序框架（Rust类型系统）强制定义边界，
让AI的产出一定遵循预先定义好的规范。
"""

import os
import re
from pathlib import Path
from dataclasses import dataclass
from typing import List, Dict

@dataclass
class ResearchScore:
    """研究评分结果"""
    total_score: float           # 总分 (0-100)
    doc_quality: float          # 文档质量 (0-40)
    code_quality: float         # 代码质量 (0-30)
    references: int             # 引用数量
    new_hypotheses: int         # 新假设数量
    details: Dict[str, str]     # 详细评分说明


def evaluate_directions_doc(doc_path: Path) -> float:
    """
    评估单个文档的质量
    检查是否有：核心问题、调研结果、架构洞察、待验证假设
    """
    if not doc_path.exists():
        return 0.0

    content = doc_path.read_text(encoding='utf-8')
    score = 0.0
    details = {}

    # 1. 核心问题 (10分)
    if "## 核心问题" in content or "**核心问题**" in content:
        score += 10
        details["core_question"] = "✓ 包含核心问题"
    else:
        details["core_question"] = "✗ 缺少核心问题"

    # 2. 调研结果 (10分)
    if "## 研究发现" in content or "**研究发现**" in content:
        score += 10
        details["findings"] = "✓ 包含研究发现"
    else:
        details["findings"] = "✗ 缺少研究发现"

    # 3. 架构洞察 (10分)
    if "## 架构洞察" in content or "**架构洞察**" in content:
        score += 10
        details["insights"] = "✓ 包含架构洞察"
    else:
        details["insights"] = "✗ 缺少架构洞察"

    # 4. 待验证假设 (10分)
    if "## 待验证假设" in content or "**待验证假设**" in content:
        score += 10
        details["hypotheses"] = "✓ 包含待验证假设"
    else:
        details["hypotheses"] = "✗ 缺少待验证假设"

    return score


def evaluate_code_drafts(drafts_dir: Path) -> tuple[float, List[str]]:
    """
    评估代码草稿质量
    - 是否可编译（语法正确）
    - 是否有详细注释
    - 是否实现核心类型
    """
    if not drafts_dir.exists():
        return 0.0, []

    score = 0.0
    compilable_files = []

    for draft in drafts_dir.glob("*.rs"):
        content = draft.read_text(encoding='utf-8')

        # 基础分：文件存在 (5分)
        score += 5

        # 注释检查 (5分)
        comment_lines = len([l for l in content.split('\n') if l.strip().startswith('//')])
        if comment_lines >= 3:
            score += 5
        else:
            score += min(comment_lines, 5)

        # 核心类型检查 (10分)
        core_types = ['struct', 'trait', 'enum', 'impl']
        has_core = any(t in content for t in core_types)
        if has_core:
            score += 10

            # 关键类型检查
            key_patterns = [
                'ToolState', 'ToolToken', 'Permission',
                'ToolChain', 'InputFor', 'Rollback'
            ]
            for pattern in key_patterns:
                if pattern in content:
                    score += 2
                    break

        # 语法检查（简单检查）
        if '{' in content and '}' in content:
            score += 5
            compilable_files.append(draft.name)
        else:
            score += 2

    return min(score, 30), compilable_files


def count_references(content: str) -> int:
    """统计参考文献数量"""
    count = 0

    # arXiv 引用
    count += len(re.findall(r'arXiv:\d{4}\.\d{4,5}', content))

    # 论文引用
    count += len(re.findall(r'\(\d{4}\)', content))  # (2025)

    # GitHub 引用
    count += len(re.findall(r'github\.com/[\w-]+/[\w-]+', content))

    # 项目引用
    project_keywords = ['Verus', 'Kani', 'Coq', 'Refine4LLM', 'XGrammar', 'MiniScope', 'DRIFT']
    for kw in project_keywords:
        count += content.count(kw)

    return count


def count_new_hypotheses(content: str) -> int:
    """统计新提出的假设数量"""
    # 查找待验证假设部分
    hypotheses_section = re.search(
        r'(## 待验证假设|待验证假设)(.*?)(?=##|$)',
        content,
        re.DOTALL
    )

    if not hypotheses_section:
        return 0

    section_text = hypotheses_section.group(2)
    # 统计 [ ] 或 - [ ] 格式的假设
    hypotheses = re.findall(r'[-\[]\s*\]', section_text)
    # 统计已完成的假设
    completed = len(re.findall(r'\[x\]', section_text))

    return max(0, len(hypotheses) - completed)


def evaluate_research产出(research_dir: str = ".") -> ResearchScore:
    """
    主评估函数

    评分标准：
    - 文档质量: 0-40分
    - 代码质量: 0-30分
    - 引用数量: 0-15分 (每引用得1分，上限15)
    - 创新性: 0-15分 (每个新假设得3分，上限15)
    """
    base_path = Path(research_dir)

    # 1. 文档质量评估 (40分)
    directions_dir = base_path / "directions"
    total_doc_score = 0.0

    if directions_dir.exists():
        for doc in directions_dir.glob("*.md"):
            total_doc_score += evaluate_directions_doc(doc)

    doc_quality = min(total_doc_score, 40.0)

    # 2. 代码质量评估 (30分)
    drafts_dir = base_path / "drafts"
    code_quality, compilable = evaluate_code_drafts(drafts_dir)

    # 3. 引用数量 (15分)
    total_content = ""
    if directions_dir.exists():
        for doc in directions_dir.glob("*.md"):
            total_content += doc.read_text(encoding='utf-8')

    if drafts_dir.exists():
        for draft in drafts_dir.glob("*.rs"):
            total_content += draft.read_text(encoding='utf-8')

    ref_count = count_references(total_content)
    reference_score = min(ref_count, 15)

    # 4. 创新性 - 新假设 (15分)
    hypo_count = count_new_hypotheses(total_content)
    hypothesis_score = min(hypo_count * 3, 15)

    # 总分
    total = doc_quality + code_quality + reference_score + hypothesis_score

    return ResearchScore(
        total_score=total,
        doc_quality=doc_quality,
        code_quality=code_quality,
        references=ref_count,
        new_hypotheses=hypo_count,
        details={
            "doc_breakdown": f"文档质量: {doc_quality:.1f}/40",
            "code_breakdown": f"代码质量: {code_quality:.1f}/30 ({len(compilable)}个可编译)",
            "ref_breakdown": f"引用数量: {ref_count} ({reference_score}分)",
            "hypo_breakdown": f"新假设: {hypo_count} ({hypothesis_score}分)"
        }
    )


def print_score(score: ResearchScore):
    """打印评分结果"""
    print("=" * 50)
    print("         研究评估报告")
    print("=" * 50)
    print(f"总分: {score.total_score:.1f}/100")
    print("-" * 50)
    print(f"文档质量: {score.doc_quality:.1f}/40")
    print(f"代码质量: {score.code_quality:.1f}/30")
    print(f"引用数量: {score.references} ({min(score.references, 15)}分)")
    print(f"新假设: {score.new_hypotheses} ({min(score.new_hypotheses * 3, 15)}分)")
    print("-" * 50)
    print("评分详情:")
    for key, val in score.details.items():
        print(f"  {key}: {val}")
    print("=" * 50)


if __name__ == "__main__":
    import sys

    research_dir = sys.argv[1] if len(sys.argv) > 1 else "/tmp/state-space-research"

    print(f"评估目录: {research_dir}")
    score = evaluate_research产出(research_dir)
    print_score(score)
