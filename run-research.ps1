# 研究任务执行脚本
# 用于手动触发或定时任务调用

param(
    [string]$Workspace = "C:\Users\11846\.openclaw-autoclaw\workspace\state-space-research"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  状态空间架构研究任务" -ForegroundColor Cyan
Write-Host "  $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $Workspace

# 检查Git状态
Write-Host "[1/4] 检查Git仓库状态..." -ForegroundColor Yellow
$status = git status --porcelain
if ($status) {
    Write-Host "警告: 工作区有未提交的更改" -ForegroundColor Yellow
    git status
} else {
    Write-Host "✓ 工作区干净" -ForegroundColor Green
}

# 拉取最新代码
Write-Host ""
Write-Host "[2/4] 拉取远程更新..." -ForegroundColor Yellow
git pull origin master
Write-Host "✓ 代码已更新" -ForegroundColor Green

# 确定研究方向
$Hour = [int](Get-Date -Format "HH")
$DirectionInfo = switch ($Hour) {
    {$_ -in @(0,12)} { "核心原则 - directions/01_core_principles.md" }
    {$_ -in @(2,14)} { "分层设计 - directions/07_layered_design.md" }
    {$_ -in @(4,16)} { "LLM导航器 - directions/08_llm_as_navigator.md" }
    {$_ -in @(6,18)} { "实现技术 - directions/09_rust_type_system.md" }
    {$_ -in @(8,20)} { "工具设计 - directions/10_tool_design.md" }
    {$_ -in @(10,22)} { "对比分析 - directions/11_comparison.md" }
    default { "工程路径 - directions/12_engineering_roadmap.md" }
}

Write-Host ""
Write-Host "[3/4] 研究方向: $DirectionInfo" -ForegroundColor Yellow
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  请AI Agent执行以下任务:" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. 阅读 RESEARCH_AGENT.md 了解任务要求"
Write-Host "2. 阅读对应的研究方向文档"
Write-Host "3. 进行深度研究和分析"
Write-Host "4. 更新研究文档和代码"
Write-Host "5. 提交到GitHub"
Write-Host ""
Write-Host "提示: 使用 sessions_spawn 启动研究agent" -ForegroundColor Gray
Write-Host "示例: sessions_spawn(task='执行状态空间架构研究', thinking='high', timeoutSeconds=1800)" -ForegroundColor Gray
Write-Host ""
