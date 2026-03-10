# 研究轨迹日志 (Research Trails)

> 详细研究过程记录，包含完整的5步研究流程

## 目录结构

```
logs/trails/
├── README.md                   # 本文件
├── EXAMPLE_TRAIL.md            # 格式示例
├── 01_core_principles/         # 方向1: 核心原则
├── 02_refinement_calculus/     # 方向2: 形式化方法
├── ...
└── 12_engineering_roadmap/     # 方向12: 工程路径
```

## 日志文件命名规范

```
{direction_id}/
└── YYYYMMDD_HHMM_{agent_id}_trail.md
```

## 快速查看

```bash
# 查看最新研究轨迹
cat logs/trails/05_type_constraints/20260310_1451_trail.md

# 查看所有轨迹
ls -la logs/trails/*/
```
