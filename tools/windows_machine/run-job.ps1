# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The laptop's executor (docs/windows-reference-machine.md §3): runs one job the controller copied to
# C:\w2m\jobs\<id>\ and leaves the bundle in its out\ folder. A probe is built and run as
# .github/workflows/windows-probe.yml's build and run steps do, so the two machines' rows differ only in the machine.
#
# Kinds (job.json `kind`):
#   probe   builds probes/<probe> from tree.tar (the committed tree; substrate.tar for a `generate` step) and runs it
#           `repeat` times;
#   binary  runs a stored probe binary unchanged (probe.exe, and the DLL it imports, come with the job);
#   batch   runs many stored binaries one after another (calibration, §6 step 2): job.json `items` lists each item's
#           probe, execution id, binary (bin\<sha256>.exe) and DLL (bin\<sha256>.dll, its name `dll_name`);
#           tree.tar holds probes\; each item's rows go to out\<execution id>\. One profile, one copy each way, one
#           lock for the whole batch, a fresh workspace for each item.
# Every input's sha256 is in job.json `inputs`; a mismatch stops the job before anything runs. job.json `cpu_class`
# ("performance" or "efficiency") pins the job to that core type (G1); the processes it starts inherit the affinity,
# and each exec-<i>.json records it.
param([Parameter(Mandatory)] [string] $Job)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Note([string] $what) { "$((Get-Date).ToUniversalTime().ToString('HH:mm:ss.fff')) $what" }   # run-job.log
$spec = Get-Content (Join-Path $Job 'job.json') -Raw | ConvertFrom-Json
if ($spec.kind -notin 'probe', 'binary', 'batch') { throw "job kind $($spec.kind): probe, binary and batch jobs only" }
Note "job $($spec.execution_id): $($spec.kind) $($spec.probe), repeat $($spec.repeat), commit $($spec.commit)"
foreach ($p in $spec.inputs.PSObject.Properties) {
  $h = (Get-FileHash -Algorithm SHA256 (Join-Path $Job $p.Name)).Hash.ToLower()
  if ($h -ne $p.Value) { throw "input $($p.Name): sha256 $h, job names $($p.Value)" }
}

# The probe runs where the runners check the tree out, D:\a\windows2mac\windows2mac, so the paths it reports (its
# module path, its current directory) are the runners' (calibration 2026-10-13: family console's title length). One
# workspace means one job at a time: the lock file is held open for the whole job. Windows releases it when the
# holding process dies; a job that waits 30 minutes for it stops.
$src = 'D:\a\windows2mac\windows2mac'; $out = Join-Path $Job 'out'; $tree = Join-Path $Job 'tree'
New-Item -ItemType Directory -Force 'D:\a', $out, $tree | Out-Null
$lock = $null; $waited = [Diagnostics.Stopwatch]::StartNew()
while (-not $lock) {
  try { $lock = [IO.File]::Open('D:\a\job.lock', 'OpenOrCreate', 'ReadWrite', 'None') }
  catch {
    if ($waited.Elapsed.TotalMinutes -ge 30) { throw "D:\a\job.lock held for 30 minutes by another job" }
    Start-Sleep -Milliseconds 200
  }
}
Note "inputs verified; workspace lock after $([int]$waited.Elapsed.TotalSeconds) s"

try {
  tar -xf (Join-Path $Job 'tree.tar') -C $tree
  if ($LASTEXITCODE -ne 0) { throw "tar: $LASTEXITCODE" }
  & (Join-Path $PSScriptRoot 'profile.ps1') -Out (Join-Path $out 'profile.json') -Workspace $src
  . (Join-Path $PSScriptRoot 'cpusets.ps1')
  $affinity = 'unpinned'
  if ($spec.cpu_class) {
    $mask = Get-W2mAffinityMask $spec.cpu_class
    [Diagnostics.Process]::GetCurrentProcess().ProcessorAffinity = [IntPtr][int64]$mask
    $affinity = '0x{0:x}' -f $mask
  }
  # The error mode a probe inherits through cmd (family time_and_error_mode, error_mode.inherited_from_cmd): the
  # runners' launcher passes 0x8003 (SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX); the
  # scheduled task passes 0x8001 (calibration 2026-10-13). SetErrorMode here is inherited by every process started.
  if (-not ('W2mErrorMode' -as [type])) {
    Add-Type -Name W2mErrorMode -Namespace '' -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetErrorMode(uint mode);'
  }
  $previousMode = [W2mErrorMode]::SetErrorMode(0x8003)
  Note ("error mode 0x{0:x} (was 0x{1:x})" -f 0x8003, $previousMode)
  $cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
  $v = [Environment]::OSVersion.Version
  $machine = "laptop-x64-$($v.Major).$($v.Minor).$($v.Build).$($cv.UBR)-$($cv.InstallationType)"
  $cpuName = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name
  Note "profile taken"

  # A fresh workspace: the whole committed tree for a probe build, the probe's own folder for a stored binary.
  function Reset-Workspace([string] $probe, [bool] $whole) {
    if (Test-Path $src) { Remove-Item $src -Recurse -Force }
    New-Item -ItemType Directory -Force $src | Out-Null
    if ($whole) { Copy-Item (Join-Path $tree '*') $src -Recurse -Force }
    elseif (Test-Path (Join-Path $tree "probes\$probe")) {
      New-Item -ItemType Directory -Force (Join-Path $src 'probes') | Out-Null
      Copy-Item (Join-Path $tree "probes\$probe") (Join-Path $src 'probes') -Recurse -Force
    }
  }

  # probe.exe in the workspace, run `repeat` times as windows-probe.yml's run step runs it; the bundle goes to $to.
  function Invoke-Probe([string] $probe, [string] $execution, [string] $to, [bool] $keepExe) {
    New-Item -ItemType Directory -Force $to | Out-Null
    $cfg = "probes/$probe/probe.json"
    $c = if (Test-Path $cfg) { Get-Content $cfg -Raw | ConvertFrom-Json } else { $null }
    (Get-FileHash -Algorithm SHA256 probe.exe).Hash.ToLower() | Out-File -Encoding ascii -NoNewline "$to/probe.sha256"
    if ($keepExe) { Copy-Item probe.exe "$to/probe.exe" }      # a stored binary is already in store/binaries
    if ($c -and $c.dll) {
      if ($keepExe) { Copy-Item "$($c.dll.name).dll" "$to/$($c.dll.name).dll" }
      (Get-FileHash -Algorithm SHA256 "$($c.dll.name).dll").Hash.ToLower() | Out-File -Encoding ascii -NoNewline "$to/dll.sha256"
    }
    $mode = if ($c -and $c.mode) { $c.mode } else { 'args' }
    $machine | Out-File -Encoding ascii -NoNewline "$to/profile.txt"
    $cpuName | Out-File -Encoding ascii -NoNewline "$to/cpu.txt"
    for ($i = 1; $i -le [int]$spec.repeat; $i++) {
      $sw = [System.Diagnostics.Stopwatch]::StartNew()
      # each execution under a 10-minute limit (audit 2026-10-13). The probe starts exactly as the runners start it
      # (pwsh calling cmd natively, so its standard handles are theirs); a watchdog process kills probe.exe from the
      # workspace if it outlives the limit and leaves a marker, and the execution records exit_code -1, timed_out.
      $marker = Join-Path $Job "timed-out-$i"
      $watch = Start-Process pwsh -WindowStyle Hidden -PassThru -ArgumentList '-NoProfile', '-Command',
        "Start-Sleep 600; Set-Content '$marker' x; Get-Process probe -ErrorAction SilentlyContinue | Where-Object Path -like '$src\*' | Stop-Process -Force"
      if ($mode -eq 'file') {
        ./probe.exe "probes/$probe/cases.bin" "$to/result-$i.bin"
        (Get-FileHash -Algorithm SHA256 "probes/$probe/cases.bin").Hash.ToLower() | Out-File -Encoding ascii -NoNewline "$to/cases.sha256"
      } elseif ($mode -eq 'stdout') {
        cmd /c "probe.exe > `"$to\body-$i.jsonl`""
      } else {
        ./probe.exe "$execution-$i" "$to/trace-$i.jsonl" "$to/roles-$i.json"
      }
      $exit = $LASTEXITCODE
      Stop-Process -Id $watch.Id -Force -ErrorAction SilentlyContinue
      $timedOut = Test-Path $marker
      $code = if ($timedOut) { -1 } else { $exit }
      $sw.Stop()
      Note "$execution execution $i exit $code in $($sw.Elapsed.TotalSeconds) s, affinity $affinity"
      "{`"exit_code`": $code, `"timed_out`": $(if ($timedOut) { 'true' } else { 'false' }), `"elapsed_seconds`": $($sw.Elapsed.TotalSeconds), `"cpu_class`": `"$(if ($spec.cpu_class) { $spec.cpu_class } else { 'any' })`", `"affinity`": `"$affinity`"}" | Out-File -Encoding ascii -NoNewline "$to/exec-$i.json"
      if ($code -ne 0) { break }        # a failing probe's output is the evidence (Sprint 034)
    }
  }

  if ($spec.kind -eq 'batch') {
    foreach ($item in $spec.items) {
      Reset-Workspace $item.probe $false
      Push-Location $src
      try {
        Copy-Item (Join-Path $Job "bin\$($item.binary).exe") probe.exe
        if ($item.dll) { Copy-Item (Join-Path $Job "bin\$($item.dll).dll") "$($item.dll_name).dll" }
        Invoke-Probe $item.probe $item.execution_id (Join-Path $out $item.execution_id) $false
      } catch {
        Note "$($item.execution_id) failed: $_"        # one item's failure is its evidence; the batch goes on
      } finally { Pop-Location }
    }
  } else {
    $probe = $spec.probe
    Reset-Workspace $probe ($spec.kind -eq 'probe')
    Push-Location $src
    try {
      $cfg = "probes/$probe/probe.json"
      $c = if (Test-Path $cfg) { Get-Content $cfg -Raw | ConvertFrom-Json } else { $null }
      if ($spec.kind -eq 'binary') {
        Copy-Item (Join-Path $Job 'probe.exe') probe.exe
        if ($c -and $c.dll) { Copy-Item (Join-Path $Job "$($c.dll.name).dll") "$($c.dll.name).dll" }
      } else {
        # generate (windows-probe.yml `generate`): substrate from the controller, not a clone
        if ($c -and $c.generate) {
          New-Item -ItemType Directory -Force substrate | Out-Null
          tar -xf (Join-Path $Job 'substrate.tar') -C substrate
          uv run --no-dev python ($c.generate -split ' ')
          if ($LASTEXITCODE -ne 0) { throw "generate: $LASTEXITCODE" }
        }
        clang --version
        $flags = @('-O1', '-Wall', '-Werror'); $libs = @('-ladvapi32', '-lbcrypt')
        if ($c) { $flags = $c.cflags -split ' '; $libs = $c.libs -split ' ' }
        $source = if ($c -and $c.source) { $c.source } else { "probes/$probe/$probe.c" }
        if ($c -and $c.dll) {
          $d = $c.dll
          clang ($d.cflags -split ' ') -shared '-Wl,/Brepro' "-Wl,/implib:$($d.name).lib" -o "$($d.name).dll" $d.source ($d.libs -split ' ')
          if ($LASTEXITCODE -ne 0) { throw "dll build: $LASTEXITCODE" }
          $libs += "$($d.name).lib"
        }
        if ($c -and $c.rc) {
          llvm-rc /nologo /I . /fo probe.res $c.rc
          if ($LASTEXITCODE -ne 0) { throw "llvm-rc: $LASTEXITCODE" }
          $libs += 'probe.res'
        }
        clang @flags '-Wl,/Brepro' -o probe.exe $source @libs
        if ($LASTEXITCODE -ne 0) { throw "build: $LASTEXITCODE" }
      }
      Invoke-Probe $probe $spec.execution_id $out ($spec.kind -eq 'probe')
    } finally { Pop-Location }
  }
} finally {
  Remove-Item $src, $tree -Recurse -Force -ErrorAction SilentlyContinue
  $lock.Dispose()
  Note "done"
}
