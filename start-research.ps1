# OpenClaw Cron 任务配置
# 状态空间架构研究定时任务

# Cron表达式: 每2小时执行一次
$CronExpression = "0 */2 * * *"

# 任务配置
$TaskConfig = @{
    Name = "state-space-research"
    Schedule = $CronExpression
    Command = "sessions_spawn"
    Parameters = @{
        task = "请阅读 state-space-research/RESEARCH_AGENT.md 文件，按照其中的指令执行状态空间架构研究任务。完成后将研究成果提交到GitHub。"
        mode = "run"
        thinking = "high"
        timeoutSeconds = 1800
        label = "state-space-research"
    }
}

Write-Host "状态空间架构研究定时任务配置" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "任务名称: $($TaskConfig.Name)"
Write-Host "执行频率: 每2小时一次"
Write-Host "Cron表达式: $($TaskConfig.Schedule)"
Write-Host "超时时间: 30分钟"
Write-Host "思考级别: high"
Write-Host ""
Write-Host "配置说明:" -ForegroundColor Yellow
Write-Host "此配置文件用于设置OpenClaw的定时任务。"
Write-Host "请使用以下命令之一设置定时任务："
Write-Host ""
Write-Host "方法1: 使用OpenClaw CLI (如果支持)"
Write-Host "  openclaw cron add '$($TaskConfig.Schedule)' --task '$($TaskConfig.Parameters.task)'"
Write-Host ""
Write-Host "方法2: 使用系统任务计划程序 (Windows)"
Write-Host "  创建一个每小时触发的基本任务"
Write-Host "  操作: 启动程序"
Write-Host "  程序: powershell.exe"
Write-Host "  参数: -File `"$PSScriptRoot\run-research.ps1`""
Write-Host ""
Write-Host "方法3: 手动触发测试"
Write-Host "  在OpenClaw对话中直接说: '开始状态空间架构研究'"
Write-Host ""

# 导出为JSON配置
$JsonConfig = $TaskConfig | ConvertTo-Json -Depth 10
$ConfigPath = Join-Path $PSScriptRoot "cron-config.json"
$JsonConfig | Out-File -FilePath $ConfigPath -Encoding UTF8

Write-Host "配置已导出到: $ConfigPath" -ForegroundColor Green
