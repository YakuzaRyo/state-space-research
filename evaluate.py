#!/usr/bin/env python3
"""
评估函数 - 固定的 ground truth
AI Coding 框架研究任务的评估指标

研究目标：通过程序框架（Rust类型系统）强制定义边界，
让AI的产出一定遵循预先定义好的规范。

评估指标:
- 文档质量 (40分): 核心问题、研究发现、架构洞察、待验证假设
- 代码质量 (30分): 可编译、核心类型、关键模式
- 引用广度 (15分): 论文引用数量
- 创新性 (10分): 新假设数量
- 验证性 (10分): 已验证假设数量
- 框架完整性 (50分): 可运行的框架项目
"""

import os
import re
import sys
import traceback
import subprocess
from pathlib import Path
from dataclasses import dataclass
from typing import List, Dict, Optional

@dataclass
class ResearchScore:
    """研究评分结果"""
    total_score: float           # 总分 (0-100)
    doc_quality: float          # 文档质量 (0-40)
    code_quality: float         # 代码质量 (0-30)
    references: int             # 引用数量
    new_hypotheses: int         # 新假设数量
    verified_hypotheses: int    # 已验证假设数量
    framework_score: int         # 框架完整性 (0-50)
    status: str                 # 状态: "success", "crash", "error"
    error_message: str          # 错误信息
    details: Dict[str, str]     # 详细评分说明

    @staticmethod
    def error(score: float, message: str) -> 'ResearchScore':
        """创建错误结果"""
        return ResearchScore(
            total_score=score,
            doc_quality=0,
            code_quality=0,
            references=0,
            new_hypotheses=0,
            verified_hypotheses=0,
            framework_score=0,
            status="error",
            error_message=message,
            details={}
        )

    @staticmethod
    def crash() -> 'ResearchScore':
        """创建崩溃结果"""
        return ResearchScore(
            total_score=0,
            doc_quality=0,
            code_quality=0,
            references=0,
            new_hypotheses=0,
            verified_hypotheses=0,
            framework_score=0,
            status="crash",
            error_message="研究过程崩溃",
            details={}
        )


def evaluate_directions_doc(doc_path: Path) -> float:
    """评估单个文档的质量"""
    if not doc_path.exists():
        return 0.0

    content = doc_path.read_text(encoding='utf-8')
    score = 0.0

    # 1. 核心问题 (10分)
    if "## 核心问题" in content or "**核心问题**" in content:
        score += 10

    # 2. 研究发现 (10分)
    if "## 研究发现" in content or "**研究发现**" in content:
        score += 10

    # 3. 架构洞察 (10分)
    if "## 架构洞察" in content or "**架构洞察**" in content:
        score += 10

    # 4. 待验证假设 (10分)
    if "## 待验证假设" in content or "**待验证假设**" in content:
        score += 10

    return score


def evaluate_code_drafts(drafts_dir: Path) -> tuple[float, List[str]]:
    """评估代码草稿质量"""
    if not drafts_dir.exists():
        return 0.0, []

    score = 0.0
    compilable_files = []

    for draft in drafts_dir.glob("*.rs"):
        content = draft.read_text(encoding='utf-8')

        # 基础分 (5分)
        score += 5

        # 注释检查 (5分)
        comment_lines = len([l for l in content.split('\n') if l.strip().startswith('//')])
        score += min(comment_lines, 5)

        # 核心类型检查 (10分)
        core_types = ['struct', 'trait', 'enum', 'impl']
        if any(t in content for t in core_types):
            score += 10
            key_patterns = ['ToolState', 'ToolToken', 'Permission', 'ToolChain', 'InputFor', 'Rollback']
            for pattern in key_patterns:
                if pattern in content:
                    score += 2
                    break

        # 语法检查 (5分)
        if '{' in content and '}' in content:
            score += 5
            compilable_files.append(draft.name)
        else:
            score += 2

    return min(score, 30), compilable_files


def count_references(content: str) -> int:
    """统计参考文献数量"""
    count = 0
    count += len(re.findall(r'arXiv:\d{4}\.\d{4,5}', content))
    count += len(re.findall(r'\(\d{4}\)', content))
    count += len(re.findall(r'github\.com/[\w-]+/[\w-]+', content))
    project_keywords = ['Verus', 'Kani', 'Coq', 'Refine4LLM', 'XGrammar', 'MiniScope', 'DRIFT']
    for kw in project_keywords:
        count += content.count(kw)
    return count


def count_new_hypotheses(content: str) -> int:
    """统计新提出的假设数量"""
    hypotheses_sections = re.findall(
        r'(#{1,3}\s*待验证假设)(.*?)(?=#{1,3}\s*[^#]|$)',
        content, re.DOTALL
    )

    if not hypotheses_sections:
        return 0

    total_hypotheses = 0
    total_completed = 0

    for section in hypotheses_sections:
        section_text = section[1]
        hypotheses = re.findall(r'[-\[]\s*\]', section_text)
        total_hypotheses += len(hypotheses)
        total_completed += len(re.findall(r'\[x\]', section_text))

    return max(0, total_hypotheses - total_completed)


def count_verified_hypotheses(content: str) -> int:
    """统计已验证的假设数量"""
    hypotheses_sections = re.findall(
        r'(#{1,3}\s*待验证假设)(.*?)(?=#{1,3}\s*[^#]|$)',
        content, re.DOTALL
    )

    if not hypotheses_sections:
        return 0

    total_verified = 0
    for section in hypotheses_sections:
        section_text = section[1]
        verified = len(re.findall(r'\[x\]', section_text, re.IGNORECASE))
        total_verified += verified

    return total_verified


def evaluate_framework(base_path: Path) -> int:
    """评估框架完整性"""
    score = 0
    details = []

    # 检查 framework 目录
    framework_dir = base_path / "framework"
    if not framework_dir.exists():
        return 0, "无 framework 目录"

    # 1. Cargo.toml (10分)
    cargo_toml = framework_dir / "Cargo.toml"
    if cargo_toml.exists():
        score += 10
        details.append("+10 Cargo.toml")

        # 检查依赖
        content = cargo_toml.read_text()
        if "[dependencies]" in content:
            score += 5
            details.append("+5 有依赖")

    # 2. src 目录 (10分)
    src_dir = framework_dir / "src"
    if src_dir.exists():
        score += 10

        # 检查 lib.rs
        if (src_dir / "lib.rs").exists():
            score += 5
            details.append("+5 lib.rs")

        # 统计模块
        modules = list(src_dir.glob("*.rs"))
        score += min(len(modules) * 2, 10)
        details.append(f"+{min(len(modules) * 2, 10)} {len(modules)} 模块")

    # 3. 编译检查 (15分)
    try:
        import os
        env = os.environ.copy()
        cargo_path = os.path.expanduser("~/.cargo/bin")
        if os.path.exists(cargo_path):
            env["PATH"] = cargo_path + ":" + env.get("PATH", "")
        # 使用绝对路径
        manifest_path = cargo_toml.resolve()
        cwd_path = framework_dir.resolve()
        result = subprocess.run(
            ["cargo", "check", "--manifest-path", str(manifest_path)],
            capture_output=True,
            timeout=60,
            cwd=str(cwd_path),
            env=env
        )
        if result.returncode == 0:
            score += 15
            details.append("+15 可编译")
        else:
            details.append(f"编译失败")
    except:
        details.append("编译检查失败")

    # 4. 测试 (10分)
    test_files = list(framework_dir.glob("**/tests/**/*.rs"))
    test_files += list(framework_dir.glob("**/*test*.rs"))
    if test_files:
        score += 10
        details.append(f"+10 {len(test_files)} 测试文件")

    return min(score, 50), ", ".join(details)


def evaluate_research产出(research_dir: str = ".") -> ResearchScore:
    """主评估函数"""
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

    # 4. 创新性 - 新假设 (10分)
    hypo_count = count_new_hypotheses(total_content)
    hypothesis_score = min(hypo_count * 2, 10)

    # 5. 验证性 - 已验证假设 (10分)
    verified_count = count_verified_hypotheses(total_content)
    verified_score = min(verified_count * 3, 10)

    # 6. 框架完整性 (50分)
    framework_score, framework_details = evaluate_framework(base_path)

    # 总分 (上限100，但框架完整性单独计算)
    total = min(100, doc_quality + code_quality + reference_score + hypothesis_score + verified_score)

    return ResearchScore(
        total_score=total,
        doc_quality=doc_quality,
        code_quality=code_quality,
        references=ref_count,
        new_hypotheses=hypo_count,
        verified_hypotheses=verified_count,
        framework_score=framework_score,
        status="success",
        error_message="",
        details={
            "doc_breakdown": f"文档质量: {doc_quality:.1f}/40",
            "code_breakdown": f"代码质量: {code_quality:.1f}/30 ({len(compilable)}个可编译)",
            "ref_breakdown": f"引用数量: {ref_count} ({reference_score}分)",
            "hypo_breakdown": f"新假设: {hypo_count} ({hypothesis_score}分)",
            "verified_breakdown": f"已验证: {verified_count} ({verified_score}分)",
            "framework_breakdown": f"框架完整性: {framework_score}/50 ({framework_details})"
        }
    )


def print_score(score: ResearchScore):
    """打印评分结果"""
    print("=" * 50)
    print("         研究评估报告")
    print("=" * 50)
    print(f"总分: {score.total_score:.1f}/100")
    print(f"框架完整性: {score.framework_score}/50")
    print("-" * 50)
    print(f"文档质量: {score.doc_quality:.1f}/40")
    print(f"代码质量: {score.code_quality:.1f}/30")
    print(f"引用数量: {score.references} ({min(score.references, 15)}分)")
    print(f"新假设: {score.new_hypotheses} ({min(score.new_hypotheses * 2, 10)}分)")
    print(f"已验证: {score.verified_hypotheses} ({min(score.verified_hypotheses * 3, 10)}分)")

    if score.status == "crash":
        print(f"状态: {score.status.upper()} ⚠️")
    elif score.status == "error":
        print(f"状态: {score.status.upper()} ⚠️")
    else:
        print(f"状态: {score.status.upper()} ✓")

    print("-" * 50)
    print("评分详情:")
    for key, val in score.details.items():
        print(f"  {key}: {val}")

    if score.error_message:
        print(f"\n错误信息: {score.error_message}")

    print("=" * 50)


def safe_evaluate(research_dir: str) -> ResearchScore:
    """安全评估，捕获所有异常"""
    try:
        return evaluate_research产出(research_dir)
    except FileNotFoundError as e:
        print(f"错误: 目录不存在 - {e}", file=sys.stderr)
        return ResearchScore.error(0, f"目录不存在: {e}")
    except PermissionError as e:
        print(f"错误: 权限不足 - {e}", file=sys.stderr)
        return ResearchScore.error(0, f"权限不足: {e}")
    except Exception as e:
        print(f"错误: {e}", file=sys.stderr)
        traceback.print_exc()
        return ResearchScore.crash()


if __name__ == "__main__":
    import sys

    research_dir = sys.argv[1] if len(sys.argv) > 1 else "."

    print(f"评估目录: {research_dir}")
    score = safe_evaluate(research_dir)
    print_score(score)

    # 退出码: 0=成功, 1=错误, 2=崩溃
    if score.status == "success":
        sys.exit(0)
    elif score.status == "error":
        sys.exit(1)
    else:
        sys.exit(2)
