# State Space Architecture Research Launcher
# Manual trigger for research tasks

param(
    [switch]$Force,
    [switch]$Status
)

$Workspace = "C:\Users\11846\.openclaw-autoclaw\workspace\state-space-research"
$LastResearchFile = Join-Path $Workspace ".last-research"

Write-Host "========================================"
Write-Host "  State Space Research Launcher"
Write-Host "  $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host "========================================"
Write-Host ""

if ($Status) {
    Write-Host "Research Status:" -ForegroundColor Yellow
    Write-Host ""
    
    if (Test-Path $LastResearchFile) {
        $LastResearch = Get-Content $LastResearchFile
        $LastTime = [DateTime]::Parse($LastResearch)
        $Elapsed = (Get-Date) - $LastTime
        $Hours = [Math]::Floor($Elapsed.TotalHours)
        $Minutes = $Elapsed.Minutes
        
        Write-Host "Last research: $LastTime"
        Write-Host "Elapsed: ${Hours}h ${Minutes}m"
        
        if ($Hours -ge 2) {
            Write-Host "Status: Ready for new research" -ForegroundColor Green
        } else {
            Write-Host "Status: Not due yet" -ForegroundColor Yellow
        }
    } else {
        Write-Host "Last research: No record"
        Write-Host "Status: First research needed" -ForegroundColor Green
    }
    
    Write-Host ""
    Write-Host "Git Status:" -ForegroundColor Yellow
    Set-Location $Workspace
    git status --short
    
    Write-Host ""
    Write-Host "Recent commits:" -ForegroundColor Yellow
    git log --oneline -3
    
    exit 0
}

if (-not $Force) {
    if (Test-Path $LastResearchFile) {
        $LastResearch = Get-Content $LastResearchFile
        $LastTime = [DateTime]::Parse($LastResearch)
        $Elapsed = (Get-Date) - $LastTime
        
        if ($Elapsed.TotalHours -lt 2) {
            Write-Host "Less than 2 hours since last research, skipping"
            Write-Host "Use -Force to override"
            exit 0
        }
    }
}

Write-Host "Preparing to launch research task..."
Write-Host ""

(Get-Date -Format "yyyy-MM-ddTHH:mm:ssK") | Out-File -FilePath $LastResearchFile -Encoding UTF8

Write-Host "========================================"
Write-Host "  Research Task Command"
Write-Host "========================================"
Write-Host ""
Write-Host "Execute this in OpenClaw chat:" -ForegroundColor Yellow
Write-Host ""
Write-Host "sessions_spawn("
Write-Host "  task='Read state-space-research/RESEARCH_AGENT.md and execute research',"
Write-Host "  mode='run',"
Write-Host "  thinking='high',"
Write-Host "  timeoutSeconds=1800,"
Write-Host "  label='state-space-research'"
Write-Host ")"
Write-Host ""
Write-Host "Or just say: 'start state space research'"
Write-Host ""

$Hour = [int](Get-Date -Format "HH")
$Direction = switch ($Hour) {
    {$_ -in @(0,12)} { "Core Principles" }
    {$_ -in @(2,14)} { "Layered Design" }
    {$_ -in @(4,16)} { "LLM Navigator" }
    {$_ -in @(6,18)} { "Implementation" }
    {$_ -in @(8,20)} { "Tool Design" }
    {$_ -in @(10,22)} { "Comparison" }
    default { "Engineering Roadmap" }
}

Write-Host "Current direction: $Direction" -ForegroundColor Green
Write-Host "Estimated time: 30 minutes" -ForegroundColor Green
Write-Host ""
