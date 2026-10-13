# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The laptop's configuration as one script that can run again at any time (docs/windows-reference-machine.md §9;
# SETUP.md records how the machine first got here). Run as an administrator over SSH:
#   scp tools/windows_machine/provision.ps1 w2m-laptop:C:/w2m/bin/ && ssh w2m-laptop C:/w2m/bin/provision.ps1
# With -AutoSignInPassword it also sets w2m's automatic sign-in (the password from the Mac's Keychain, service
# w2m-laptop-local-account). It installs no tool: it checks that each pinned tool answers and stops if one does not.
param([string] $AutoSignInPassword = '')
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
function Step([string] $what) { "== $what" }

Step 'SSH: key-only, PowerShell 7, starts with Windows'
$cfg = "$env:ProgramData\ssh\sshd_config"
$text = (Get-Content $cfg) -replace '^#?\s*PasswordAuthentication\s+\w+', 'PasswordAuthentication no'
if (-not ($text -match '^PasswordAuthentication no')) { $text = @('PasswordAuthentication no') + $text }
Set-Content $cfg $text -Encoding ascii
$keys = "$env:ProgramData\ssh\administrators_authorized_keys"
icacls.exe $keys /inheritance:r /grant 'Administrators:F' /grant 'SYSTEM:F' | Out-Null
New-ItemProperty -Path HKLM:\SOFTWARE\OpenSSH -Name DefaultShell -PropertyType String -Force -Value 'C:\Program Files\PowerShell\7\pwsh.exe' | Out-Null
Set-Service sshd -StartupType Automatic

Step 'Firewall: port 22 from the local subnet, on a private network only'
if (-not (Get-NetFirewallRule -Name w2m-sshd -ErrorAction SilentlyContinue)) {
  New-NetFirewallRule -Name w2m-sshd -DisplayName 'OpenSSH Server (w2m)' -Direction Inbound -Protocol TCP -LocalPort 22 -Action Allow | Out-Null
}
Set-NetFirewallRule -Name w2m-sshd -Profile Private -RemoteAddress LocalSubnet -Enabled True
foreach ($r in '{71015B69-A96E-42E6-B6F9-CF38796A71B4}', 'OpenSSH-Server-In-TCP') {      # the installers' rules, open to any address
  if (Get-NetFirewallRule -Name $r -ErrorAction SilentlyContinue) { Disable-NetFirewallRule -Name $r }
}
Get-NetConnectionProfile | Where-Object NetworkCategory -ne 'Private' | ForEach-Object { "  network '$($_.Name)' is $($_.NetworkCategory): SSH closed on it" }

Step 'Power: never sleep, never hibernate, no Modern Standby (read at boot)'
foreach ($m in 'ac', 'dc') { foreach ($s in 'monitor', 'standby', 'hibernate', 'disk') { powercfg /change "$s-timeout-$m" 0 } }
foreach ($v in '/setacvalueindex', '/setdcvalueindex') {
  powercfg $v SCHEME_CURRENT SUB_BUTTONS LIDACTION 0
  powercfg $v SCHEME_CURRENT SUB_SLEEP UNATTENDSLEEP 0
  powercfg $v SCHEME_CURRENT SUB_SLEEP STANDBYIDLE 0
  powercfg $v SCHEME_CURRENT SUB_VIDEO VIDEOIDLE 0
}
powercfg /setactive SCHEME_CURRENT
powercfg /hibernate off
New-ItemProperty HKLM:\SYSTEM\CurrentControlSet\Control\Power -Name PlatformAoAcOverride -PropertyType DWord -Value 0 -Force | Out-Null
New-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power' -Name HiberbootEnabled -PropertyType DWord -Value 0 -Force | Out-Null

Step 'Folders: administrators and SYSTEM write, users read (C:\w2m\tools is on the machine Path)'
foreach ($d in 'C:\w2m', 'C:\w2m\bin', 'C:\w2m\jobs', 'C:\w2m\games', 'C:\w2m\tools', 'D:\a') { New-Item -ItemType Directory -Force $d | Out-Null }
foreach ($d in 'C:\w2m', 'D:\a') {
  # explicit on the top folder only; everything below inherits it (grants with (OI)(CI) do not apply to a file, so
  # /inheritance:r with /T would leave each file an empty list)
  icacls.exe $d /inheritance:r /grant:r '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-545:(OI)(CI)RX' /Q | Out-Null
  Get-ChildItem $d -Force | ForEach-Object { icacls.exe $_.FullName /reset /T /C /Q | Out-Null }
}
Add-MpPreference -ExclusionPath 'C:\w2m', 'D:\a'

Step 'Machine Path: LLVM, C:\w2m\tools; no per-user folders'
$path = [Environment]::GetEnvironmentVariable('Path', 'Machine') -split ';' | Where-Object { $_ -and $_ -notmatch '^C:\\Users\\' }
foreach ($d in 'C:\Program Files\LLVM\bin', 'C:\w2m\tools') { if ($path -notcontains $d) { $path += $d } }
[Environment]::SetEnvironmentVariable('Path', ($path -join ';'), 'Machine')
[Environment]::SetEnvironmentVariable('UV_PYTHON_INSTALL_DIR', 'C:\w2m\python', 'Machine')
[Environment]::SetEnvironmentVariable('UV_PYTHON_BIN_DIR', 'C:\w2m\tools', 'Machine')

Step 'Windows Update: paused for the measurement series (Home allows 35 days)'
$until = (Get-Date).ToUniversalTime().AddDays(35).ToString('yyyy-MM-ddTHH:mm:ssZ')
$now = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
$ux = 'HKLM:\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings'
New-Item $ux -Force | Out-Null
foreach ($k in 'PauseFeatureUpdates', 'PauseQualityUpdates') {
  Set-ItemProperty $ux "${k}StartTime" $now; Set-ItemProperty $ux "${k}EndTime" $until
}
Set-ItemProperty $ux PauseUpdatesStartTime $now; Set-ItemProperty $ux PauseUpdatesExpiryTime $until
"  paused until $until"

Step 'Sync and suggestions off'
$p = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\OneDrive'; New-Item $p -Force | Out-Null; Set-ItemProperty $p DisableFileSyncNGSC 1 -Type DWord
$p = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\SettingSync'; New-Item $p -Force | Out-Null
Set-ItemProperty $p DisableSettingSync 2 -Type DWord; Set-ItemProperty $p DisableSettingSyncUserOverride 1 -Type DWord
$p = 'HKLM:\SOFTWARE\Policies\Microsoft\Edge'; New-Item $p -Force | Out-Null; Set-ItemProperty $p SyncDisabled 1 -Type DWord

Step 'The w2m account and its automatic sign-in (an accepted risk: section 4 of the plan)'
if (-not (Get-LocalUser -Name w2m -ErrorAction SilentlyContinue)) { throw 'no local user w2m: create it first (SETUP.md 5)' }
$wl = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
$pl = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\PasswordLess\Device'
New-Item $pl -Force | Out-Null; Set-ItemProperty $pl DevicePasswordLessBuildVersion 0 -Type DWord
$p = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\Personalization'; New-Item $p -Force | Out-Null; Set-ItemProperty $p NoLockScreen 1 -Type DWord
if ($AutoSignInPassword) {
  Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class W2mLsa {
  [StructLayout(LayoutKind.Sequential)] struct US { public ushort Length, MaximumLength; public IntPtr Buffer; }
  [StructLayout(LayoutKind.Sequential)] struct OA { public int Length; public IntPtr Root, Name; public uint Attributes; public IntPtr Sd, Qos; }
  [DllImport("advapi32.dll")] static extern uint LsaOpenPolicy(IntPtr system, ref OA attrs, uint access, out IntPtr handle);
  [DllImport("advapi32.dll")] static extern uint LsaStorePrivateData(IntPtr handle, ref US key, ref US data);
  [DllImport("advapi32.dll")] static extern uint LsaClose(IntPtr handle);
  [DllImport("advapi32.dll")] static extern uint LsaNtStatusToWinError(uint status);
  static US Make(string s) { var u = new US(); u.Buffer = Marshal.StringToHGlobalUni(s); u.Length = (ushort)(s.Length * 2); u.MaximumLength = (ushort)(u.Length + 2); return u; }
  public static uint Store(string key, string value) {
    var oa = new OA(); oa.Length = Marshal.SizeOf(typeof(OA)); IntPtr h;
    uint s = LsaOpenPolicy(IntPtr.Zero, ref oa, 0x00000020, out h);   // POLICY_CREATE_SECRET
    if (s != 0) return LsaNtStatusToWinError(s);
    var k = Make(key); var d = Make(value);
    s = LsaStorePrivateData(h, ref k, ref d); LsaClose(h);
    Marshal.FreeHGlobal(k.Buffer); Marshal.FreeHGlobal(d.Buffer);
    return LsaNtStatusToWinError(s);
  }
}
'@
  $r = [W2mLsa]::Store('DefaultPassword', $AutoSignInPassword)     # an LSA secret, as Sysinternals Autologon keeps it
  if ($r -ne 0) { throw "LsaStorePrivateData: win32 error $r" }
  Set-ItemProperty $wl AutoAdminLogon '1'; Set-ItemProperty $wl DefaultUserName 'w2m'; Set-ItemProperty $wl DefaultDomainName $env:COMPUTERNAME
}
Set-ItemProperty $wl ForceAutoLogon '1'      # signed in again after each sign-out (a fresh session per job)
Remove-ItemProperty $wl DefaultPassword -ErrorAction SilentlyContinue      # never a plain-text copy
"  automatic sign-in: $((Get-ItemProperty $wl).AutoAdminLogon) as $((Get-ItemProperty $wl).DefaultUserName)"

Step 'Pinned tools (installs: SETUP.md 6 and 7)'
$want = [ordered]@{       # each answer piped through Out-String: pwsh refuses a bare (& exe) inside a script block
  clang  = { (& clang --version | Out-String) -match 'clang version 20\.1\.8' }
  pwsh   = { (& 'C:\Program Files\PowerShell\7\pwsh.exe' -NoProfile -Command '$PSVersionTable.PSVersion.ToString()' | Out-String) -match '^7\.' }
  git    = { (& git --version | Out-String) -match '^git version' }
  uv     = { (& C:\w2m\tools\uv.exe --version | Out-String) -match '^uv ' }
  zstd   = { (& C:\w2m\tools\zstd.exe --version 2>&1 | Out-String) -match 'Zstandard CLI' }
  python = { (& C:\w2m\tools\python.exe --version 2>&1 | Out-String) -match '^Python 3\.12' }
  msvc   = { Test-Path 'C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.*' }
}
foreach ($k in $want.Keys) { if (-not (& $want[$k])) { throw "tool $k missing or not the pinned version" }; "  $k ok" }

Restart-Service sshd -ErrorAction SilentlyContinue      # the running session survives; new sessions see the Path
"provisioned"
