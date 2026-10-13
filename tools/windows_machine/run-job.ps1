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
#           lock for the whole batch, a fresh workspace for each item;
#   game    runs the game as .github/workflows/windows-game.yml does, `executions` times one after another
#           (job.json `game`, `exe`, `seconds`, `mode`): the launcher built from tree.tar's tools\jit_capture, the
#           game mirrored from C:\w2m\games\<game> to D:\w2m\game (the runners' GAME_DIR) and checked against
#           files.json, C:\w2m\app a junction to it, and before each execution the game's saved state removed
#           (HKCU\Software\<company>\<product>, LocalLow\<company>\<product>; §5) so each starts as a runner's does.
# Every input's sha256 is in job.json `inputs`; a mismatch stops the job before anything runs. job.json `cpu_class`
# ("performance" or "efficiency") pins the job to that core type (G1); the processes it starts inherit the affinity,
# and each exec-<i>.json records it.
param([Parameter(Mandatory)] [string] $Job)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Note([string] $what) { "$((Get-Date).ToUniversalTime().ToString('HH:mm:ss.fff')) $what" }   # run-job.log
$spec = Get-Content (Join-Path $Job 'job.json') -Raw | ConvertFrom-Json
if ($spec.kind -notin 'probe', 'binary', 'batch', 'game') { throw "job kind $($spec.kind): probe, binary, batch and game jobs only" }
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
  function Reset-Workspace([string] $probe, [bool] $whole) {   # $whole: the whole of tree.tar
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

  if ($spec.kind -eq 'game') {
    Reset-Workspace '' $true
    Push-Location $src
    try {
      # build launcher (windows-game.yml)
      $m = @()
      if (Test-Path "games/$($spec.game)/manifest.xml") {
        "1 24 `"games/$($spec.game)/manifest.xml`"" | Out-File -Encoding ascii manifest.rc
        llvm-rc /nologo /I . /fo manifest.res manifest.rc
        if ($LASTEXITCODE -ne 0) { throw "llvm-rc: $LASTEXITCODE" }
        $m = @('manifest.res', '-Wl,/manifest:no')
      }
      clang -O1 -Wall -Werror -o jit_capture.exe tools/jit_capture/jit_capture.c '-Wl,/subsystem:windows' @m
      if ($LASTEXITCODE -ne 0) { throw "jit_capture build: $LASTEXITCODE" }
      clang -O1 -Wall -Werror -shared -o handler_trace.dll tools/jit_capture/handler_trace.c
      if ($LASTEXITCODE -ne 0) { throw "handler_trace build: $LASTEXITCODE" }
      clang -O1 -Wall -Werror -o copy_resources.exe tools/jit_capture/copy_resources.c
      if ($LASTEXITCODE -ne 0) { throw "copy_resources build: $LASTEXITCODE" }
      Note "launcher built"
      $pristine = "C:\w2m\games\$($spec.game)"; $gameDir = 'D:\w2m\game'; $app = 'C:\w2m\app'; $exe = $spec.exe
      $listing = (Get-Content "games/$($spec.game)/files.json" -Raw | ConvertFrom-Json).listing
      $info = Get-Content (Join-Path $pristine "$($exe)_Data\app.info")
      $company = $info[0].Trim(); $product = $info[1].Trim()
      for ($i = 1; $i -le [int]$spec.executions; $i++) {
        # the game as the release unpacks it: mirrored from the pristine copy (the launcher's install and the
        # game's own writes undone), every file checked against files.json, nothing else present
        robocopy $pristine $gameDir /MIR /NFL /NDL /NJH /NJS /NP | Out-Null
        if ($LASTEXITCODE -ge 8) { throw "robocopy: $LASTEXITCODE" }
        $bad = @(foreach ($f in $listing) {
          $q = Join-Path $gameDir $f.path
          if (-not (Test-Path -LiteralPath $q) -or (Get-FileHash -LiteralPath $q -Algorithm SHA256).Hash.ToLower() -ne $f.sha256) { $f.path }
        })
        $present = (Get-ChildItem $gameDir -Recurse -File).Count
        if ($bad.Count -or $present -ne $listing.Count) { throw "game tree: $($bad.Count) files differ from files.json, $present present of $($listing.Count)" }
        Note "execution $i`: game tree verified, $present files"
        # install launcher (windows-game.yml)
        Move-Item "$gameDir\$exe.exe" "$gameDir\$exe.original.exe"
        Copy-Item jit_capture.exe "$gameDir\$exe.exe"
        .\copy_resources.exe "$gameDir\$exe.original.exe" "$gameDir\$exe.exe"
        if ($LASTEXITCODE -ne 0) { throw "copy_resources: $LASTEXITCODE" }
        Copy-Item handler_trace.dll "$gameDir\handler_trace.dll"
        if (Test-Path $app) { cmd /c rmdir $app }
        New-Item -ItemType Junction -Path $app -Target $gameDir | Out-Null
        New-Item -ItemType Directory -Force C:\Users\w2m\AppData\Local\Temp | Out-Null
        # the initial state (§5): no saved settings, no log, no game process
        Remove-Item "HKCU:\Software\$company\$product" -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item "$env:USERPROFILE\AppData\LocalLow\$company\$product" -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item C:\w2m\unity.log -Force -ErrorAction SilentlyContinue
        # run (windows-game.yml)
        if (Test-Path out) { Remove-Item out -Recurse -Force }
        New-Item -ItemType Directory -Force out | Out-Null
        $machine | Out-File -Encoding ascii -NoNewline out/profile.txt
        $cpuName | Out-File -Encoding ascii -NoNewline out/cpu.txt
        $env:W2M_JIT_CAPTURE = "$PWD\out\jit_capture.bin"
        $env:TMP = 'C:\Users\w2m\AppData\Local\Temp'; $env:TEMP = 'C:\Users\w2m\AppData\Local\Temp'
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $gameArgs = if ($spec.mode -eq 'play') { @('-screen-fullscreen', '0', '-screen-width', '1280', '-screen-height', '720', '-logFile', '"C:\w2m\unity.log"') }
                    else { @('-batchmode', '-nographics', '-logFile', '"C:\w2m\unity.log"') }
        $p = Start-Process -FilePath "$app\$exe.exe" -WorkingDirectory $app -ArgumentList $gameArgs -PassThru
        $deadline = (Get-Date).AddSeconds([int]$spec.seconds)
        $shell = New-Object -ComObject WScript.Shell
        $done = $false
        while ((Get-Date) -lt $deadline -and -not $done) {
          $done = $p.WaitForExit(3000)
          if (-not $done -and $spec.mode -eq 'play') { [void]$shell.AppActivate($p.Id); $shell.SendKeys('{ENTER}'); $shell.SendKeys(' ') }
        }
        $sw.Stop()
        if (-not $done) { Stop-Process -Id $p.Id -Force; [void]$p.WaitForExit(30000); $code = $null } else { $code = $p.ExitCode }
        Get-Process | Where-Object { $_.Path -and ($_.Path.StartsWith($gameDir) -or $_.Path.StartsWith($app)) } | ForEach-Object {
          Stop-Process -Id $_.Id -Force; [void]$_.WaitForExit(30000)
        }
        if (Test-Path C:\w2m\unity.log) { Copy-Item C:\w2m\unity.log out\unity.log }
        $exit = if ($null -eq $code) { 'null' } else { "$code" }
        # exec.json as the runners write it, with the game's exact arguments and mode added (a laptop run says what
        # it ran; a runner run's arguments are only in its workflow)
        $argsJson = ConvertTo-Json -Compress @($gameArgs)
        "{`"exited_by_itself`": $($done.ToString().ToLower()), `"exit_code`": $exit, `"elapsed_seconds`": $($sw.Elapsed.TotalSeconds), `"mode`": `"$($spec.mode)`", `"seconds`": $($spec.seconds), `"arguments`": $argsJson}" | Out-File -Encoding ascii -NoNewline out/exec.json
        if (Test-Path out\jit_capture.bin) {        # the store keeps the capture's digest, not the capture
          (Get-FileHash -Algorithm SHA256 out\jit_capture.bin).Hash.ToLower() | Out-File -Encoding ascii -NoNewline out\jit_capture.sha256
          Remove-Item out\jit_capture.bin
        }
        Move-Item out (Join-Path $out "$i")
        Note "execution $i`: exited_by_itself $done after $([int]$sw.Elapsed.TotalSeconds) s"
      }
    } finally { Pop-Location; if (Test-Path C:\w2m\app) { cmd /c rmdir C:\w2m\app } }
  } elseif ($spec.kind -eq 'batch') {
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
