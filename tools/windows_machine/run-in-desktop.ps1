# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# Runs run-job.ps1 in w2m's signed-in desktop session (docs/windows-reference-machine-2026-10-12.md G7, Q2). An SSH command runs
# in session 0, without a desktop: a child's STARTUPINFO desktop is empty there and `winsta0\default` on the runners
# (calibration 2026-10-13, family console). A scheduled task whose principal logs on interactively runs in the
# session of that user's sign-in; w2m signs in automatically at boot. The task is registered, started, waited for
# and removed; its exit code is run-job.ps1's.
param([Parameter(Mandatory)] [string] $Job)
$ErrorActionPreference = 'Stop'

$name = 'w2m-job-' + (Split-Path $Job -Leaf)
# job.json `console_window` (hidden, minimized, normal): the job's own console window. Hidden leaves no application in
# the foreground; on the runners another application held it (family windows, msg.game.9: WM_ACTIVATEAPP's lParam).
$window = (Get-Content (Join-Path $Job 'job.json') -Raw | ConvertFrom-Json).console_window
if (-not $window) { $window = 'hidden' }
$log = Join-Path $Job 'out\run-job.log'             # saved with the results
New-Item -ItemType Directory -Force (Join-Path $Job 'out') | Out-Null
$pwsh = 'C:\Program Files\PowerShell\7\pwsh.exe'
$arguments = "-NoProfile -NonInteractive -WindowStyle $window -Command `"& '$Job\run-job.ps1' -Job '$Job' *> '$log'; exit `$LASTEXITCODE`""
$action = New-ScheduledTaskAction -Execute $pwsh -Argument $arguments -WorkingDirectory $Job
$principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\w2m" -LogonType Interactive -RunLevel Highest
# a task left behind by a dropped SSH session is removed here, before the next job registers its own
Get-ScheduledTask -TaskName 'w2m-job-*' -ErrorAction SilentlyContinue | Where-Object State -ne 'Running' |
  Unregister-ScheduledTask -Confirm:$false
$minutes = (Get-Content (Join-Path $Job 'job.json') -Raw | ConvertFrom-Json).limit_minutes
if (-not $minutes) { $minutes = 60 }
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes $minutes) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
# A fresh session for each job (job.json `fresh_session`, default true): the desktop's own state carries from one
# job to the next (calibration 2026-10-13: a window-class atom's order, the application in the foreground, and
# where CW_USEDEFAULT places a window, each matching the runners only in a new session). w2m is signed out and
# Winlogon signs it in again (ForceAutoLogon = 1, provision.ps1).
$fresh = (Get-Content (Join-Path $Job 'job.json') -Raw | ConvertFrom-Json).fresh_session
if ($null -eq $fresh -or $fresh) {
  if (-not ('W2mWts' -as [type])) {
    Add-Type -Name W2mWts -Namespace '' -MemberDefinition '[DllImport("wtsapi32.dll", SetLastError=true)] public static extern bool WTSLogoffSession(System.IntPtr server, int session, bool wait);'
  }
  $old = Get-Process explorer -IncludeUserName -ErrorAction SilentlyContinue | Where-Object UserName -like '*\w2m'
  if ($old) { [void][W2mWts]::WTSLogoffSession([IntPtr]::Zero, $old.SessionId, $true) }
  $signedIn = [Diagnostics.Stopwatch]::StartNew()
  do {
    Start-Sleep -Milliseconds 500
    $new = Get-Process explorer -IncludeUserName -ErrorAction SilentlyContinue | Where-Object { $_.UserName -like '*\w2m' -and $_.Id -ne $old.Id }
    if ($signedIn.Elapsed.TotalMinutes -ge 3) { throw 'w2m was not signed in again within 3 minutes' }
  } while (-not $new)
  Start-Sleep -Seconds 15          # the shell's start-up programs settle before the job's first window
}
Register-ScheduledTask -TaskName $name -Action $action -Principal $principal -Settings $settings -Force | Out-Null
try {
  if (-not (Get-Process explorer -IncludeUserName -ErrorAction SilentlyContinue | Where-Object UserName -like '*\w2m')) {
    throw 'w2m is not signed in: no desktop session to run in'
  }
  Start-ScheduledTask -TaskName $name
  # wait while it runs or has not yet started; a task that has not started within 2 minutes is a failure
  $started = [Diagnostics.Stopwatch]::StartNew()
  do {
    Start-Sleep -Milliseconds 250
    $notYet = (Get-ScheduledTaskInfo -TaskName $name).LastTaskResult -eq 0x41303
    if ($notYet -and $started.Elapsed.TotalMinutes -ge 2) { throw "task $name did not start within 2 minutes" }
  } while ((Get-ScheduledTask -TaskName $name).State -eq 'Running' -or $notYet)
  $code = (Get-ScheduledTaskInfo -TaskName $name).LastTaskResult
} finally {
  Unregister-ScheduledTask -TaskName $name -Confirm:$false
  if (Test-Path $log) { Get-Content $log }
}
exit $code
