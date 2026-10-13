# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The reference profile (docs/windows-reference-machine.md R1, §12): what machine a run came from, written as JSON
# to the path given. run-job.ps1 runs it before each job; it reads, never changes, the machine.
param([Parameter(Mandatory)] [string] $Out)
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

# Efficiency class per logical processor (G1): GetSystemCpuSetInformation, SYSTEM_CPU_SET_INFORMATION.CpuSet.
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class W2mCpuSets {
  [DllImport("kernel32.dll", SetLastError=true)]
  static extern bool GetSystemCpuSetInformation(IntPtr info, uint len, out uint ret, IntPtr proc, uint flags);
  public static int[] EfficiencyClasses() {
    uint n; GetSystemCpuSetInformation(IntPtr.Zero, 0, out n, IntPtr.Zero, 0);
    IntPtr b = Marshal.AllocHGlobal((int)n);
    try {
      if (!GetSystemCpuSetInformation(b, n, out n, IntPtr.Zero, 0)) throw new System.ComponentModel.Win32Exception();
      var r = new System.Collections.Generic.List<int>(); int off = 0;
      while (off < n) { r.Add(Marshal.ReadByte(b, off + 20)); off += Marshal.ReadInt32(b, off); }
      return r.ToArray();
    } finally { Marshal.FreeHGlobal(b); }
  }
}
'@

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
                                 efficiency_classes = [W2mCpuSets]::EfficiencyClasses() }
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
  defender         = [ordered]@{ real_time = $mp.RealTimeProtectionEnabled; exclusions = @((Get-MpPreference).ExclusionPath) }
  servicing_busy   = [bool](Get-Process TrustedInstaller -ErrorAction SilentlyContinue)
  power_scheme     = ((powercfg /getactivescheme) -replace '^.*\(|\)$', '')
  clang            = $clang
  taken            = (Get-Date).ToUniversalTime().ToString('o')
}
$profile | ConvertTo-Json -Depth 5 | Set-Content -Path $Out -Encoding utf8
