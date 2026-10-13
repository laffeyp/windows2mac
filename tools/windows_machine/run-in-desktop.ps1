# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# Runs run-job.ps1 in w2m's signed-in desktop session (docs/windows-reference-machine.md G7, Q2). An SSH command runs
# in session 0, without a desktop: a child's STARTUPINFO desktop is empty there and `winsta0\default` on the runners
# (calibration 2026-10-13, family console). A scheduled task whose principal logs on interactively runs in the
# session of that user's sign-in; w2m signs in automatically at boot. The task is registered, started, waited for
# and removed; its exit code is run-job.ps1's.
param([Parameter(Mandatory)] [string] $Job)
$ErrorActionPreference = 'Stop'

$name = 'w2m-job-' + (Split-Path $Job -Leaf)
$log = Join-Path $Job 'out\run-job.log'             # saved with the results
New-Item -ItemType Directory -Force (Join-Path $Job 'out') | Out-Null
$pwsh = 'C:\Program Files\PowerShell\7\pwsh.exe'
$arguments = "-NoProfile -NonInteractive -WindowStyle Hidden -Command `"& '$Job\run-job.ps1' -Job '$Job' *> '$log'; exit `$LASTEXITCODE`""
$action = New-ScheduledTaskAction -Execute $pwsh -Argument $arguments -WorkingDirectory $Job
$principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\w2m" -LogonType Interactive -RunLevel Highest
# a task left behind by a dropped SSH session is removed here, before the next job registers its own
Get-ScheduledTask -TaskName 'w2m-job-*' -ErrorAction SilentlyContinue | Where-Object State -ne 'Running' |
  Unregister-ScheduledTask -Confirm:$false
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Hours 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $name -Action $action -Principal $principal -Settings $settings -Force | Out-Null
try {
  if (-not (Get-Process explorer -IncludeUserName -ErrorAction SilentlyContinue | Where-Object UserName -like '*\w2m')) {
    throw 'w2m is not signed in: no desktop session to run in'
  }
  Start-ScheduledTask -TaskName $name
  do { Start-Sleep -Milliseconds 250 } while ((Get-ScheduledTask -TaskName $name).State -eq 'Running' -or
                                              (Get-ScheduledTaskInfo -TaskName $name).LastTaskResult -eq 0x41303)  # not yet run
  $code = (Get-ScheduledTaskInfo -TaskName $name).LastTaskResult
} finally {
  Unregister-ScheduledTask -TaskName $name -Confirm:$false
  if (Test-Path $log) { Get-Content $log }
}
exit $code
