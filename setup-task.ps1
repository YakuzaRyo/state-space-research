$a = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument '-ExecutionPolicy Bypass -WindowStyle Hidden -File "C:\Users\11846\.openclaw-autoclaw\workspace\state-space-research\run-research.ps1"'
$t = New-ScheduledTaskTrigger -Once -At '09:45AM' -RepetitionInterval (New-TimeSpan -Hours 2)
Register-ScheduledTask -TaskName 'StateSpaceResearch' -Action $a -Trigger $t -Description 'State Space Architecture Research Automation' -RunLevel Highest
