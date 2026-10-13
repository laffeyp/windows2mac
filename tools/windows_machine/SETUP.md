# The Windows reference machine: setup record

What was done to the laptop on arrival, 2026-10-13, in order (`docs/windows-reference-machine.md` §9, §12). Commands
after step 2 ran from the Mac over SSH as `ssh w2m-laptop`, each PowerShell block sent with `-EncodedCommand`.

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

```powershell
powercfg /change standby-timeout-ac 0          # was 300 s
powercfg /change hibernate-timeout-ac 0
powercfg /setacvalueindex SCHEME_CURRENT SUB_BUTTONS LIDACTION 0
powercfg /setactive SCHEME_CURRENT
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
