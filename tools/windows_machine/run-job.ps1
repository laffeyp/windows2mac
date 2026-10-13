# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The laptop's executor (docs/windows-reference-machine.md §3): runs one job the controller copied to
# C:\w2m\jobs\<id>\ and leaves the bundle in its out\ folder. A probe job is built and run as
# .github/workflows/windows-probe.yml's build and run steps do, so the two machines' rows differ only in the machine.
#
# A `binary` job runs a stored probe binary unchanged (calibration, §6 step 2): probe.exe (and the DLL it imports)
# come with the job, and the build step is skipped.
#
# The job folder holds job.json ({"kind": "probe", "probe", "execution_id", "repeat"}), tree.tar (a `git archive`
# of the committed tree the job names) and, for a probe with a `generate` step, substrate.tar (substrate.commit's
# tree). Every input's sha256 is in job.json `inputs`; a mismatch stops the job before anything runs. job.json
# `cpu_class` ("performance" or "efficiency") pins the job to that core type (G1); the processes it starts inherit
# the affinity, and each exec-<i>.json records it.
param([Parameter(Mandatory)] [string] $Job)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Note([string] $what) { "$((Get-Date).ToUniversalTime().ToString('HH:mm:ss.fff')) $what" }   # run-job.log
$spec = Get-Content (Join-Path $Job 'job.json') -Raw | ConvertFrom-Json
Note "job $($spec.execution_id): $($spec.kind) $($spec.probe), repeat $($spec.repeat), commit $($spec.commit)"
foreach ($p in $spec.inputs.PSObject.Properties) {
  $h = (Get-FileHash -Algorithm SHA256 (Join-Path $Job $p.Name)).Hash.ToLower()
  if ($h -ne $p.Value) { throw "input $($p.Name): sha256 $h, job names $($p.Value)" }
}

# The tree is unpacked where the runners check it out, D:\a\windows2mac\windows2mac, so the paths a probe reports
# (its module path, its current directory) are the runners' (calibration 2026-10-13: family console's title length).
# One workspace means one job at a time: the lock file is held open for the whole job.
$src = 'D:\a\windows2mac\windows2mac'; $out = Join-Path $Job 'out'
New-Item -ItemType Directory -Force 'D:\a', $out | Out-Null
# Windows releases the lock when the holding process dies. A job that waits 30 minutes for it stops.
$lock = $null; $waited = [Diagnostics.Stopwatch]::StartNew()
while (-not $lock) {
  try { $lock = [IO.File]::Open('D:\a\job.lock', 'OpenOrCreate', 'ReadWrite', 'None') }
  catch {
    if ($waited.Elapsed.TotalMinutes -ge 30) { throw "D:\a\job.lock held for 30 minutes by another job" }
    Start-Sleep -Milliseconds 200
  }
}
if (Test-Path $src) { Remove-Item $src -Recurse -Force }
New-Item -ItemType Directory -Force $src | Out-Null
Note "inputs verified; workspace lock after $([int]$waited.Elapsed.TotalSeconds) s"
tar -xf (Join-Path $Job 'tree.tar') -C $src
if ($LASTEXITCODE -ne 0) { throw "tar: $LASTEXITCODE" }
& (Join-Path $PSScriptRoot 'profile.ps1') -Out (Join-Path $out 'profile.json') -Workspace $src
. (Join-Path $PSScriptRoot 'cpusets.ps1')
$affinity = 'unpinned'
if ($spec.cpu_class) {
  $mask = Get-W2mAffinityMask $spec.cpu_class
  [Diagnostics.Process]::GetCurrentProcess().ProcessorAffinity = [IntPtr][int64]$mask
  $affinity = '0x{0:x}' -f $mask
}
Push-Location $src
try {
  if ($spec.kind -notin 'probe', 'binary') { throw "job kind $($spec.kind): probe and binary jobs only" }
  $probe = $spec.probe
  $cfg = "probes/$probe/probe.json"
  $c = if (Test-Path $cfg) { Get-Content $cfg -Raw | ConvertFrom-Json } else { $null }

  # generate (windows-probe.yml `generate`): substrate from the controller, not a clone
  if ($c -and $c.generate) {
    New-Item -ItemType Directory -Force substrate | Out-Null
    tar -xf (Join-Path $Job 'substrate.tar') -C substrate
    uv run --no-dev python ($c.generate -split ' ')
    if ($LASTEXITCODE -ne 0) { throw "generate: $LASTEXITCODE" }
  }

  if ($spec.kind -eq 'binary') {
    Copy-Item (Join-Path $Job 'probe.exe') probe.exe
    if ($c -and $c.dll) { Copy-Item (Join-Path $Job "$($c.dll.name).dll") "$($c.dll.name).dll" }
  } else {
    # build
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

  # run
  (Get-FileHash -Algorithm SHA256 probe.exe).Hash.ToLower() | Out-File -Encoding ascii -NoNewline "$out/probe.sha256"
  if ($spec.kind -eq 'probe') { Copy-Item probe.exe "$out/probe.exe" }      # a binary job's exe is already stored
  if ($c -and $c.dll) {
    Copy-Item "$($c.dll.name).dll" "$out/$($c.dll.name).dll"
    (Get-FileHash -Algorithm SHA256 "$($c.dll.name).dll").Hash.ToLower() | Out-File -Encoding ascii -NoNewline "$out/dll.sha256"
  }
  $mode = if ($c -and $c.mode) { $c.mode } else { 'args' }
  $cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
  $v = [Environment]::OSVersion.Version
  "laptop-x64-$($v.Major).$($v.Minor).$($v.Build).$($cv.UBR)-$($cv.InstallationType)" | Out-File -Encoding ascii -NoNewline "$out/profile.txt"
  (Get-CimInstance Win32_Processor | Select-Object -First 1).Name | Out-File -Encoding ascii -NoNewline "$out/cpu.txt"
  for ($i = 1; $i -le [int]$spec.repeat; $i++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    if ($mode -eq 'file') {
      ./probe.exe "probes/$probe/cases.bin" "$out/result-$i.bin"
      (Get-FileHash -Algorithm SHA256 "probes/$probe/cases.bin").Hash.ToLower() | Out-File -Encoding ascii -NoNewline "$out/cases.sha256"
    } elseif ($mode -eq 'stdout') {
      cmd /c "probe.exe > `"$out\body-$i.jsonl`""
    } else {
      ./probe.exe "$($spec.execution_id)-$i" "$out/trace-$i.jsonl" "$out/roles-$i.json"
    }
    $code = $LASTEXITCODE
    $sw.Stop()
    Note "execution $i exit $code in $($sw.Elapsed.TotalSeconds) s, affinity $affinity"
    "{`"exit_code`": $code, `"elapsed_seconds`": $($sw.Elapsed.TotalSeconds), `"cpu_class`": `"$(if ($spec.cpu_class) { $spec.cpu_class } else { 'any' })`", `"affinity`": `"$affinity`"}" | Out-File -Encoding ascii -NoNewline "$out/exec-$i.json"
    if ($code -ne 0) { break }        # a failing probe's output is the evidence (Sprint 034)
  }
} finally {
  Pop-Location
  Remove-Item $src -Recurse -Force -ErrorAction SilentlyContinue
  $lock.Dispose()
}
