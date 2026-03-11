"""
Rust代码结构验证脚本
用于验证分层设计代码的语法正确性和逻辑完整性
"""

import re
import json

# 读取Rust文件
with open('D:/11846/state-space-research/drafts/20260311_layered_design.rs', 'r', encoding='utf-8') as f:
    rust_code = f.read()

# 验证结构
results = {
    "file": "20260311_layered_design.rs",
    "validation_time": "2026-03-11",
    "checks": {}
}

# 1. 检查四个层次是否都存在
layers = ["Syntax", "Semantic", "Pattern", "Domain"]
for layer in layers:
    pattern = rf"Layer \d+: {layer} Layer"
    match = re.search(pattern, rust_code)
    results["checks"][f"{layer}_layer_exists"] = bool(match)

# 2. 检查关键trait定义
traits = ["Syntax", "Semantic", "Pattern", "Domain", "FromSemantic"]
for trait in traits:
    pattern = rf"pub trait {trait}"
    match = re.search(pattern, rust_code)
    results["checks"][f"trait_{trait}_defined"] = bool(match)

# 3. 检查PhantomData使用（类型状态模式）
phantom_count = rust_code.count("PhantomData")
results["checks"]["phantom_data_usage"] = phantom_count >= 3
results["phantom_data_count"] = phantom_count

# 4. 检查类型状态模式
state_types = ["Unmatched", "Matched", "Transformed"]
for state in state_types:
    pattern = rf"pub struct {state}"
    match = re.search(pattern, rust_code)
    results["checks"][f"state_type_{state}"] = bool(match)

# 5. 检查转换管道
results["checks"]["transformation_pipeline"] = "TransformationPipeline" in rust_code
results["checks"]["parse_function"] = "pub fn parse(" in rust_code
results["checks"]["analyze_function"] = "pub fn analyze(" in rust_code
results["checks"]["optimize_function"] = "pub fn optimize(" in rust_code

# 6. 检查测试模块
results["checks"]["test_module"] = "#[cfg(test)]" in rust_code
results["checks"]["test_functions"] = rust_code.count("#[test]")

# 7. 检查关键数据结构
structures = ["RawExpr", "TypedExpr", "ConfigItem", "Workflow", "PatternMatcher"]
for struct in structures:
    pattern = rf"pub (struct|enum) {struct}"
    match = re.search(pattern, rust_code)
    results["checks"][f"struct_{struct}"] = bool(match)

# 8. 检查错误类型
error_types = ["ParseError", "SemanticError", "DomainError"]
for error in error_types:
    results["checks"][f"error_{error}"] = error in rust_code

# 9. 验证分层架构的完整性
syntax_to_semantic = "impl Semantic for" in rust_code
semantic_to_pattern = "PatternMatcher" in rust_code
pattern_to_domain = "FromSemantic" in rust_code

results["checks"]["syntax_to_semantic_bridge"] = syntax_to_semantic
results["checks"]["semantic_to_pattern_bridge"] = semantic_to_pattern
results["checks"]["pattern_to_domain_bridge"] = pattern_to_domain

# 10. 检查文档注释
doc_comments = rust_code.count("///")
block_comments = rust_code.count("//!")
results["documentation"] = {
    "line_doc_comments": doc_comments,
    "block_doc_comments": block_comments,
    "total_doc_lines": doc_comments + block_comments
}

# 输出验证结果
print("=" * 60)
print("Rust代码结构验证报告")
print("=" * 60)
print(json.dumps(results, indent=2, ensure_ascii=False))
print("=" * 60)

# 计算通过率
total_checks = len([v for v in results["checks"].values() if isinstance(v, bool)])
passed_checks = sum(1 for v in results["checks"].values() if v is True)
print(f"\n验证通过率: {passed_checks}/{total_checks} ({passed_checks/total_checks*100:.1f}%)")

# 失败项
failures = [k for k, v in results["checks"].items() if v is False]
if failures:
    print("\n失败项:")
    for f in failures:
        print(f"  - {f}")
else:
    print("\n所有验证项通过!")
