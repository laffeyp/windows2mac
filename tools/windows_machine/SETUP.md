# The Windows reference machine: setup record

What was done to the laptop on arrival, 2026-10-13, in order (`docs/windows-reference-machine.md` §9, §12). Commands
after step 2 ran from the Mac over SSH as `ssh w2m-laptop`, each PowerShell block sent with `-EncodedCommand`.

This file is the history. The configuration itself is `provision.ps1`, which can run again at any time and sets
every setting below; the installs (sections 2, 6 and 7) are the parts it checks but does not perform.

## 1. The machine before

HP Victus 15-fa2xxx, Windows 11 Home 25H2 build 26200.7840, signed in with a Microsoft account (local name `laffe`).
Preinstalled: McAfee, HP's apps and services, OneDrive with Desktop, Documents and Pictures redirected into it.

## 2. SSH, at the laptop (the one manual step)

OpenSSH Server's feature-on-demand download (`Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0`)
stalled, and winget's `msstore` source failed its certificate check. What worked, in an administrator PowerShell:

```powershell
winget install --source winget "openssh preview"
Start-Service sshd
```

McAfee was uninstalled through Settings before the first connection.

## 3. The controller's key

The Mac's key is `~/.ssh/w2m_laptop` (ed25519, comment `w2m-mac-controller`); `~/.ssh/config` names the laptop
`w2m-laptop`. The first login used the account's password once, to run:

```powershell
$a = "$env:ProgramData\ssh\administrators_authorized_keys"
Set-Content $a '<the Mac key's public line>' -Encoding ascii
icacls.exe $a /inheritance:r /grant Administrators:F /grant SYSTEM:F
Set-Service sshd -StartupType Automatic
New-NetFirewallRule -Name w2m-sshd -DisplayName 'OpenSSH Server (w2m)' -Direction Inbound -Protocol TCP -LocalPort 22 -Action Allow -Profile Any
```

Then, with the key:

```powershell
New-ItemProperty -Path HKLM:\SOFTWARE\OpenSSH -Name DefaultShell -PropertyType String -Force -Value "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
(Get-Content C:\ProgramData\ssh\sshd_config) -replace '^#?\s*PasswordAuthentication\s+\w+', 'PasswordAuthentication no' | Set-Content C:\ProgramData\ssh\sshd_config -Encoding ascii
Restart-Service sshd
Set-NetFirewallRule -Name w2m-sshd -RemoteAddress LocalSubnet
Disable-NetFirewallRule -Name '{71015B69-A96E-42E6-B6F9-CF38796A71B4}', 'OpenSSH-Server-In-TCP'   # the installer's rules, open to any address
```

Checked: `ssh -o PubkeyAuthentication=no w2m-laptop` is refused (`publickey,keyboard-interactive`).

## 4. Removed

- McAfee's leftover `C:\Program Files\McAfee`.
- Appx packages, for all users and from the provisioned image (`Remove-AppxPackage -AllUsers`,
  `Remove-AppxProvisionedPackage -Online`): every `AD2F1837.*` (HP: PC Hardware Diagnostics, Printer Control,
  Privacy Settings, Support Assistant, System Event Utility, myHP, OMEN Command Center), C27EB4BA.DropboxOEM,
  DTSInc.DTSSoundUnbound, Clipchamp.Clipchamp, MSTeams, aimgr, Microsoft.BingNews, Microsoft.BingSearch,
  Microsoft.BingWeather, Microsoft.Copilot, Microsoft.GamingApp, Microsoft.GetHelp,
  Microsoft.MicrosoftSolitaireCollection, Microsoft.MixedRealityLink, Microsoft.Office.ActionsServer,
  Microsoft.OfficePushNotificationUtility, Microsoft.OutlookForWindows, Microsoft.PowerAutomateDesktop,
  Microsoft.StartExperiencesApp, Microsoft.Todos, Microsoft.Whiteboard, Microsoft.Windows.DevHome,
  Microsoft.WindowsFeedbackHub, Microsoft.Xbox.TCUI, Microsoft.XboxGamingOverlay, Microsoft.XboxSpeechToTextOverlay,
  Microsoft.YourPhone, Microsoft.ZuneMusic, MicrosoftCorporationII.MicrosoftFamily, MicrosoftCorporationII.QuickAssist,
  MicrosoftWindows.Client.WebExperience, MicrosoftWindows.CrossDevice, Microsoft.WindowsAlarms,
  Microsoft.WindowsSoundRecorder.
- HP services stopped and disabled: HPAppHelperCap, HPDiagsCap, HPNetworkCap, HPOmenCap, HPSysInfoCap,
  HpTouchpointAnalyticsService.
- OneDrive: process stopped, `OneDriveSetup.exe /uninstall`, then
  `HKLM:\SOFTWARE\Policies\Microsoft\Windows\OneDrive` `DisableFileSyncNGSC` = 1. Desktop, Personal and My Pictures
  (and their known-folder GUID values) in `User Shell Folders` set back to `%USERPROFILE%\<folder>`; the local
  `C:\Users\laffe\OneDrive` and `%LOCALAPPDATA%\Microsoft\OneDrive` deleted after the uninstall, so the cloud copy
  was untouched. `C:\Users\Public\Desktop\Microsoft Edge.lnk` deleted.
- Sync and suggestions: `HKLM:\SOFTWARE\Policies\Microsoft\Windows\SettingSync` `DisableSettingSync` = 2,
  `DisableSettingSyncUserOverride` = 1; `HKLM:\SOFTWARE\Policies\Microsoft\Edge` `SyncDisabled` = 1;
  `ContentDeliveryManager` values (SubscribedContent-310093/338388/338389/338393/353694/353696, SystemPane
  Suggestions, SoftLanding, SilentInstalledApps, ContentDeliveryAllowed, OemPreInstalledApps, PreInstalledApps) = 0;
  `UserProfileEngagement` `ScoobeSystemSettingEnabled` = 0.

Kept: Microsoft 365, the Copilot desktop app, the NVIDIA App and drivers, Defender, the Windows firewall.

## 5. Machine settings

Sleep. The laptop entered Modern Standby (S0 low-power idle, its only standby state) at 14:15:41 and 14:23:14 with
the display off, which cut SSH. Every timeout on AC and battery is 0, hibernation is off, and Modern Standby is off
(`PlatformAoAcOverride` = 0, read at boot); after the restart `powercfg /a` lists no sleep state as available.

```powershell
foreach ($m in 'ac','dc') { foreach ($s in 'monitor','standby','hibernate','disk') { powercfg /change "$s-timeout-$m" 0 } }
foreach ($v in '/setacvalueindex','/setdcvalueindex') {
  powercfg $v SCHEME_CURRENT SUB_BUTTONS LIDACTION 0
  powercfg $v SCHEME_CURRENT SUB_SLEEP UNATTENDSLEEP 0
  powercfg $v SCHEME_CURRENT SUB_SLEEP STANDBYIDLE 0
  powercfg $v SCHEME_CURRENT SUB_VIDEO VIDEOIDLE 0
}
powercfg /setactive SCHEME_CURRENT
powercfg /hibernate off
New-ItemProperty HKLM:\SYSTEM\CurrentControlSet\Control\Power -Name PlatformAoAcOverride -PropertyType DWord -Value 0 -Force
New-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power' -Name HiberbootEnabled -PropertyType DWord -Value 0 -Force
shutdown /r /t 5
```

```powershell
New-Item C:\w2m\bin, C:\w2m\jobs, C:\w2m\games -ItemType Directory -Force
Add-MpPreference -ExclusionPath C:\w2m
```

The local administrator `w2m` (§9): `New-LocalUser -Name w2m -PasswordNeverExpires -AccountNeverExpires`,
`Add-LocalGroupMember -Group Administrators -Member w2m`. Its password is random and kept in the Mac's Keychain
(service `w2m-laptop-local-account`) for automatic sign-in (G7). The controller's key logs in as `w2m`.

## 6. Toolchain

```powershell
winget install --id <id> --exact --source winget --silent --accept-package-agreements --accept-source-agreements --disable-interactivity
```

for Microsoft.PowerShell, Git.Git, Meta.Zstandard and astral-sh.uv (each exit 0), and LLVM.LLVM `--version 20.1.8`,
the runners' clang (run 38304235237's log: `clang version 20.1.8`, `Target: x86_64-pc-windows-msvc`).
Python.Python.3.12 through winget hung with TrustedInstaller busy and was stopped; Python 3.12 came from
`uv python install 3.12`.

The tools must be machine-wide, since jobs run as `w2m`: winget is registered per user and is absent for `w2m`, the
winget uv and zstd went to `laffe`'s profile, and winget's PowerShell 7 was the Store package. So:

```powershell
# uv, uvx, zstd copied from laffe's WinGet\Packages to C:\w2m\tools; Python into C:\w2m\python
[Environment]::SetEnvironmentVariable('UV_PYTHON_INSTALL_DIR', 'C:\w2m\python', 'Machine')
[Environment]::SetEnvironmentVariable('UV_PYTHON_BIN_DIR', 'C:\w2m\tools', 'Machine')
C:\w2m\tools\uv.exe python install 3.12 --default          # cpython 3.12.15
# machine Path += C:\Program Files\LLVM\bin; C:\w2m\tools
# PowerShell 7.6.7 from the release MSI (Authenticode: Microsoft Corporation, valid)
msiexec /i PowerShell-7.6.7-win-x64.msi /qn /norestart ADD_PATH=1
New-ItemProperty -Path HKLM:\SOFTWARE\OpenSSH -Name DefaultShell -PropertyType String -Force -Value 'C:\Program Files\PowerShell\7\pwsh.exe'
Restart-Service sshd
```

As `w2m`: pwsh 7.6.7, git, clang and llvm-rc 20.1.8, zstd, uv, python and tar all resolve.

## 7. The linker and SDK

clang for `x86_64-pc-windows-msvc` links with MSVC's link.exe and libraries and the Windows SDK; without them the
first probe build failed (`unable to find a Visual Studio installation`). The runners' binaries say which linker
built them (the PE optional header's linker version): all 141 Server 2025 binaries in `store/binaries` were linked
by 14.51 (Visual Studio 2026), all 132 Server 2022 binaries by 14.44 (Visual Studio 2022).

```powershell
vs_BuildTools.exe --quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools `
  --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.26100
```

from `https://aka.ms/vs/17/release/vs_BuildTools.exe` (Visual Studio 2022); Visual Studio 2026's Build Tools
(`https://aka.ms/vs/stable/vs_buildtools.exe`, per Microsoft Learn's command-line page) follow for 14.51.

## 8. The runners' layout, the desktop session, automatic sign-in

Calibration's console family showed two differences the executor made, not the machine: a child's desktop name
(empty from SSH's session 0, `winsta0\default` on the runners) and the module path's length (58 against 38,
`D:\a\windows2mac\windows2mac\probe.exe`). So jobs unpack at `D:\a\windows2mac\windows2mac` (D: is the second
NVMe disk, already NTFS) and run in w2m's desktop session through a scheduled task (`run-in-desktop.ps1`).
`Add-MpPreference -ExclusionPath D:\a`.

Automatic sign-in as `w2m`: the password stored as the LSA secret `DefaultPassword` through `LsaStorePrivateData`
(as Sysinternals Autologon stores it; no plain-text `DefaultPassword` value in the Winlogon key), Winlogon
`AutoAdminLogon` = 1, `DefaultUserName` = w2m, `DefaultDomainName` = WINDOWSMACHINE, `PasswordLess\Device`
`DevicePasswordLessBuildVersion` = 0, `Personalization` `NoLockScreen` = 1. After the restart at 15:09:08,
explorer.exe ran as `windowsmachine\w2m` in session 1.

## 9. The review of 2026-10-13

- The firewall rule `w2m-sshd` now applies on the Private profile only.
- `C:\w2m` and `D:\a`: administrators and SYSTEM full control, Users read and execute, set on the two folders and
  inherited below. The first attempt ran `icacls /inheritance:r ... /T`, which left every file below with an
  empty list (`(OI)(CI)` grants do not apply to a file); `icacls <child> /reset /T` repaired them, and a job
  running at that moment hung and was stopped.
- The machine Path lost `C:\Users\Administrator\AppData\Local\Microsoft\WindowsApps`.
- Windows Update paused to 2026-11-17 (the `UX\Settings` pause values; Home allows 35 days). KB5054156, KB5128942
  and KB5127647 had installed on 2026-10-13 before the pause.
- Tool versions, recorded in every profile.json from now on: clang 20.1.8, link.exe 14.51.36231 (Visual Studio
  2026 Build Tools; 2022's 14.44.35207 also present), PowerShell 7.6.7, git 2.55.0.windows.5, uv 0.13.0,
  zstd 1.5.7, Python 3.12.15.
