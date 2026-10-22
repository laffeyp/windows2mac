# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The reference profile (docs/windows-reference-machine-2026-10-12.md R1, §12): what machine a run came from, written as JSON
# to the path given. run-job.ps1 runs it before each job; it reads, never changes, the machine.
param([Parameter(Mandatory)] [string] $Out, [string] $Workspace = '')
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$os  = Get-CimInstance Win32_OperatingSystem
$cs  = Get-CimInstance Win32_ComputerSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$cv  = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
$cpk = 'HKLM:\HARDWARE\DESCRIPTION\System\CentralProcessor\0'    # read by name: pwsh cannot cast every value there
$dg  = Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard -ErrorAction SilentlyContinue
$mp  = Get-MpComputerStatus -ErrorAction SilentlyContinue
$clang = (& clang --version 2>$null | Select-Object -First 1)

. (Join-Path $PSScriptRoot 'cpusets.ps1')

$profile = [ordered]@{
  machine          = 'laptop'
  computer         = $env:COMPUTERNAME
  model            = "$($cs.Manufacturer) $($cs.Model)"
  bios             = (Get-CimInstance Win32_BIOS).SMBIOSBIOSVersion
  windows          = [ordered]@{ edition = $os.Caption; display_version = $cv.DisplayVersion; build = [int]$os.BuildNumber
                                 ubr = [int]$cv.UBR; architecture = $os.OSArchitecture }
  cpu              = [ordered]@{ name = $cpu.Name.Trim(); identifier = (Get-ItemPropertyValue $cpk Identifier); processor_id = $cpu.ProcessorId
                                 microcode = ([BitConverter]::ToString((Get-ItemPropertyValue $cpk 'Update Revision')) -replace '-', '')
                                 cores = $cpu.NumberOfCores; logical = $cpu.NumberOfLogicalProcessors
                                 sets = @([W2mCpuSets]::Sets() | ForEach-Object { [ordered]@{ logical = $_[0]; core = $_[1]
                                           efficiency_class = $_[2]; scheduling_class = $_[3] } }) }
  memory_bytes     = [int64]$cs.TotalPhysicalMemory
  gpus             = @(Get-CimInstance Win32_VideoController | ForEach-Object { [ordered]@{ name = $_.Name; driver = $_.DriverVersion } })
  display          = [ordered]@{ width = (Get-CimInstance Win32_VideoController | Where-Object CurrentHorizontalResolution | Select-Object -First 1).CurrentHorizontalResolution
                                 height = (Get-CimInstance Win32_VideoController | Where-Object CurrentVerticalResolution | Select-Object -First 1).CurrentVerticalResolution
                                 dpi = (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -ErrorAction SilentlyContinue).AppliedDPI }
  audio            = @(Get-CimInstance Win32_SoundDevice | ForEach-Object Name)
  hypervisor       = [ordered]@{ present = $cs.HypervisorPresent; vbs_status = $dg.VirtualizationBasedSecurityStatus
                                 security_services_running = @($dg.SecurityServicesRunning) }
  scheduler        = [ordered]@{ win32_priority_separation = (Get-ItemProperty HKLM:\SYSTEM\CurrentControlSet\Control\PriorityControl).Win32PrioritySeparation }
  user             = [ordered]@{ name = $env:USERNAME; profile = $env:USERPROFILE; temp = $env:TEMP }
  locale           = [ordered]@{ culture = (Get-Culture).Name; ui = (Get-UICulture).Name
                                 acp = (Get-ItemProperty HKLM:\SYSTEM\CurrentControlSet\Control\Nls\CodePage).ACP; time_zone = (Get-TimeZone).Id }
  defender         = [ordered]@{ real_time = $mp.RealTimeProtectionEnabled; exclusions = @((Get-MpPreference).ExclusionPath)
                                 product = $mp.AMProductVersion; engine = $mp.AMEngineVersion; signatures = $mp.AntivirusSignatureVersion }
  updates          = [ordered]@{ paused_until = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings' -ErrorAction SilentlyContinue).PauseUpdatesExpiryTime
                                 hotfixes = @(Get-HotFix | Sort-Object HotFixID | ForEach-Object HotFixID) }
  session          = [ordered]@{ id = (Get-Process -Id $PID).SessionId; workspace = $Workspace }
  servicing_busy   = [bool](Get-Process TrustedInstaller -ErrorAction SilentlyContinue)
  power_scheme     = ((powercfg /getactivescheme) -replace '^.*\(|\)$', '')
  tools            = [ordered]@{ clang = $clang
                                 linker = ((& clang -v -x c NUL -o NUL 2>&1 | Select-String 'link\.exe' | Select-Object -First 1) -split '"')[1]
                                 pwsh = $PSVersionTable.PSVersion.ToString()
                                 git = (& git --version 2>$null); uv = (& uv --version 2>$null); zstd = (& zstd --version 2>$null)
                                 python = (& python --version 2>$null) }
  taken            = (Get-Date).ToUniversalTime().ToString('o')
}
$profile | ConvertTo-Json -Depth 5 | Set-Content -Path $Out -Encoding utf8
