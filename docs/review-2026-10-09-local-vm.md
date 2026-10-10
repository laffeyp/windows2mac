# Review 2026-10-09 — the local Windows VM under UTM

Scope: the VM `w2m-windows` (UTM 5.0.6, id 9153CC3C-6487-435D-BD9F-8CE7B7DDE71E), the Decisions that set it up and gave it its role (BLACKBOARD 2026-10-09, entries 103 and 104), and `scripts/vm_guest.py` (untracked, version of 21:19). The VM's role per Decision 104: the reference for Windows behavior (not CPU behavior), the game installed once, the Steam client logged in so `SteamAPI_Init` succeeds, a rendering trial with UTM's experimental DirectX driver, a self-hosted Actions runner and SSH for automation.

## Method

Read the QEMU command line of the running VM (process 60245), the VM's guest state through UTM's scripting interface (the guest agent), the host's network path, and the script. Every finding below names its evidence. Not read: TN3179's full text (the page returned only its title), the UTM guest tools 0.1.271 release contents, the VM's `config.plist` (the container denies listing).

## 1. Verdict

The VM boots, logs in user `w2m` automatically, runs the guest agent and sshd, and has 183.7 GB free. Three things stand between it and its role. The host cannot reach it over the network, so SSH and an Actions runner cannot work. Its display device has no 3D path, so the DirectX trial cannot happen and Direct3D 11 falls to Microsoft's software renderer, as on the hosted runner. And everything the agent starts runs as LocalSystem in session 0, where Steam, windows and input do not exist. `vm_guest.py` has fixed the worst automation fault (exit codes) but keeps four defects that will surface as false failures or hangs.

## 2. Blocking

### L1 — The host cannot reach the guest: macOS Local Network privacy on Ghostty

`ssh w2m@192.168.64.2` fails at once with "No route to host". The guest side is correct: the guest has 192.168.64.2 on the vmnet-shared bridge, `sshd` is RUNNING, `netstat` shows it listening on 0.0.0.0:22 and [::]:22, and the rule "OpenSSH (w2m)" allows inbound on every profile (the network profile is Public; the stock "OpenSSH SSH Server Preview (sshd)" rule is Private-only and does not apply). The host has the guest's MAC in its ARP table, and the guest pings the host (192.168.64.1, 0% loss).

The failure is on the host and is not specific to the VM. From the same shell, TCP connections to the home router (192.168.4.1, ports 53 and 80) fail the same way, inside and outside Claude Code's command sandbox. The shell's ancestry is `ghostty → login → zsh`. macOS 15 and later deny local-network connections to an app without Local Network permission and fail them with EHOSTUNREACH ("No route to host"); the responsible app for a command-line tool is the terminal that hosts it. The mechanism is inferred from that behavior and the router test, not quoted from TN3179.

Fix: System Settings › Privacy & Security › Local Network › enable Ghostty, then restart Ghostty. Until then SSH, `scp`, an SMB share and a self-hosted runner reached from the host all fail; the guest agent works because it travels over a virtio-serial port, not the network.

### L2 — No 3D display, so no DirectX trial

QEMU runs with `-vga none -device virtio-ramfb` and SPICE `gl=off`. The guest's only display adapter is "Red Hat VirtIO GPU DOD controller" 22.7.38.43 at 1024×768, a display-only driver. A game started in this VM gets Microsoft Basic Render Driver for Direct3D 11, which is what the hosted runner already gave in run 38006671518.

UTM's release notes for the DirectX driver: 5.0.4 "Added support for Neptune backend in virglrenderer for DirectX support"; 5.0.5 "DirectX 12 is now supported with D3DMetal backend (when available)"; "During the install of the guest tools, you will be prompted to select the experimental graphics driver"; "You must (re)install the latest UTM guest tools experimental 3D graphics driver"; an older driver must be removed first with "the uninstall script provided on the guest tools install ISO"; "we recommend a fresh install because the driver is still experimental and its use could lead to instability"; "Apple CoreGL backend does not support Vulkan or DirectX". virglrenderer reaches a guest only through QEMU's GPU-accelerated virtio-gpu device; this VM has none. The unattended install loaded drivers from guest tools 0.1.271 with no driver choice; whether 0.1.271 carries the experimental driver was not checked.

Decision 104's revisit condition ("if UTM's DirectX cannot render the game") cannot be judged until the VM has a GPU-accelerated display device, a renderer backend other than CoreGL, and the experimental driver installed by the guest tools installer, preferably on a fresh install as the notes advise.

### L3 — Session 0: the agent cannot start Steam, the game, or input

The guest agent (`qemu-ga`, service running as LocalSystem) starts every process in session 0, which has no desktop. `query user` shows `w2m` on the console in session 2. Consequences:

- `SteamAPI_Init` needs the Steam client running as the same user in the same session. A game started by the agent runs as SYSTEM in session 0 and fails it, as on the runner.
- A window and a D3D device created in session 0 never reach the screen; `SendInput` and window messages cannot cross into session 2. The play-mode key loop that ended run 38006671518 red has the same problem here.
- A self-hosted GitHub Actions runner installed as a service runs in session 0 too. For play runs it must run as `w2m` in the interactive session (started at logon), not as a service.

`vm_guest.py session` works around this with a scheduled task under `-LogonType Interactive`, which is the right mechanism; its defects are in §3. Steam is not installed (`C:\Program Files (x86)\Steam\steam.exe` absent); its first login needs the user at the console for Steam Guard.

## 3. `scripts/vm_guest.py`

What it gets right: it measured that `utmctl exec` returns 0 whatever the guest command does, and that `utmctl file push`/`pull` return 0 on failure, and it replaced the first with UTM's AppleScript `execute … with output capturing` and checks every transfer by sha256. This review reproduced the first: `utmctl exec … cmd.exe /c exit 7` returned 0; the AppleScript path returned 7.

- **V1 — A race makes task runs report false results** (lines 82–84 and 149–152). The loop waits only while the task's State is `Running`. After `Start-ScheduledTask` the state can still be `Ready` or `Queued`, so the loop can exit before the task starts. `screenshot` then reads the previous run's `LastTaskResult` and fails with 11 (no PNG); `session` throws on a missing `session.code`. Fix: wait for `session.code` (or the PNG) to appear, or for `LastRunTime` to change and the state to leave `Running`, with a deadline.
- **V2 — No timeouts.** The AppleScript poll (lines 44–48) and both PowerShell loops run forever if the guest command never exits; a hung game run hangs the caller with no record. Each needs a deadline and a kill of the guest process on expiry.
- **V3 — Errors that do not terminate report success** (line 145). `$c = $LASTEXITCODE` is null for a script that writes an error without `exit`, and null becomes 0. Most cmdlet failures are of that kind. Fix: set `$ErrorActionPreference = 'Stop'` inside the task's command, or fail on any error record in `session.out`.
- **V4 — One task name and one set of files for every caller.** `w2m-screenshot`, `w2m-session`, `C:\w2m\session.ps1`, `session.out` and `session.code` are fixed names. Two callers at once (two agent sessions, or the many concurrent item runs the progress review §6.3 proposes) overwrite each other's script and read each other's output. Fix: a per-call id in the task name and file names.
- **V5 — Output over 16 MiB is cut silently.** QEMU's agent caps captured output at `GUEST_EXEC_MAX_OUTPUT (16 * 1024 * 1024)` per stream and sets `out-truncated`; the script never reads that flag. Large outputs belong in a file pulled and hashed.
- **V6 — `exec` has a command-line ceiling.** `-EncodedCommand` carries the script as base64 of UTF-16, 8/3 of its character count; Windows caps a command line at 32,767 characters, so `exec` scripts over about 12,000 characters fail. `session` avoids it by pushing a file.
- **V7 — `push` reads the whole file into memory and moves it through the agent.** That suits scripts and probes, not the game's 15.6 GB; the game should arrive through Steam or SteamCMD inside the guest, or over SSH once L1 is fixed.

## 4. Configuration risks

- **R1 — Install media still boots first.** The Windows ISO is boot index 0 and `w2m-setup.iso` (Autounattend.xml with the disk layout) is index 1; the NVMe disk is index 2. The VM has restarted at least once (System event 1074). Each boot passes the "press any key to boot from CD" prompt; a keystroke in that window (UTM keystroke injection, or `sendkey` on the monitor in R2) starts Setup, and the answer file repartitions the disk. Eject both drives.
- **R2 — An unauthenticated monitor on TCP.** QEMU runs with `-monitor tcp:127.0.0.1:4444,server,nowait`, a custom argument. Any local process can `quit`, `system_reset` or send keys to the VM. Remove it once installation work is done.
- **R3 — Defender and the indexer load the VM.** MsMpEng leads guest CPU time (533 s), then System, explorer and SearchIndexer; the QEMU process stood at 98% of a host core and 21.7 GB resident. No Defender exclusions exist. Real-time scanning will run over every file Steam writes and every probe pushed to `C:\w2m`; a launcher that rewrites import tables, as `jit_capture` does, is the kind of behavior Defender inspects (not tested for this binary). Exclusions for the Steam library and `C:\w2m` belong in the setup script.
- **R4 — Windows Update can restart the VM mid-run.** `wuauserv` is running and event 1074 records a restart. Active hours or paused updates are needed for long runs. The VM also has no TPM (QEMU has no TPM device; Setup used LabConfig bypasses), which can withhold feature updates.
- **R5 — The clock never syncs.** `w32tm` reports source "Local CMOS Clock" and no successful sync; the guest read 8 s ahead of the host. Records that join guest and host timestamps need w32time syncing.
- **R6 — Activation.** LicenseStatus 5 (notification): the KMS client key has no KMS server. Windows works; a watermark can appear in screenshots and break image comparisons. The user's key is already recorded as pending.
- **R7 — The VM shares the host's fast cores with w2m's own measurements.** The host (Apple M5 Max) reports 6 cores at performance level 0 and 12 at level 1; the VM takes 8 vCPUs. w2m figures such as the ≈27 µs VM exit, taken while this VM runs, are taken under load. Measurements should record whether the Windows VM was running, or pause it.
- **R8 — Prism changes more than CPU verdicts.** Decision 104 already limits the VM to Windows behavior. Timing (QueryPerformanceCounter frequency and cost), the CPUID that Mono and Unity read (so their feature-dependent paths), and per-call cost differ from the x86 runners. Call counts and thread counts measured here may differ from runs 38006665419 and 38006671518 for that reason; each record should name the profile (Windows on Arm, Prism) as the Decision requires.
- **R9 — The interactive session can lock.** Screenshots and input need an unlocked desktop in session 2; screen-off and lock timeouts were not checked.

## 5. Order of repair

1. Grant Ghostty Local Network access (L1); confirm `ssh w2m@192.168.64.2`.
2. Eject both ISOs and drop the TCP monitor (R1, R2).
3. Fix V1–V4 in `vm_guest.py` before anything depends on it.
4. Add Defender exclusions, set update hours, enable time sync, disable screen lock (R3, R4, R5, R9).
5. Install Steam in the interactive session and log in once at the console (L3).
6. For the DirectX trial: a GPU-accelerated display device with a non-CoreGL backend and the experimental driver from current guest tools, on a fresh install as UTM advises (L2). Until then the VM renders no better than the hosted runner.

## Sources

- QEMU command line of process 60245 (`ps`), 2026-10-09 21:14.
- Guest queries through UTM AppleScript `execute … with output capturing`: `ipconfig`, `sc query sshd`, `netstat -an`, `Get-NetFirewallRule`, `Get-NetConnectionProfile`, `Win32_VideoController`, `Get-Package`, `Get-Process`, `Get-Tpm`, `SoftwareLicensingProduct`, `w32tm /query /status`, `Get-MpPreference`, System events 41/6008/1074, `query user`, `HKLM\…\CurrentVersion` (26H2, build 26300.9457, Professional).
- Host: `nc`/`ssh` to 192.168.64.2:22 and 192.168.4.1:53/80; `arp -an`; `sysctl hw.perflevel0.physicalcpu hw.perflevel1.physicalcpu machdep.cpu.brand_string`; `utmctl exec --help` ("The return value of the command will be returned from this tool") against the measured 0 for `exit 7`.
- UTM release notes v5.0.4, v5.0.5, v5.0.6: https://newreleases.io/project/github/utmapp/UTM/release/v5.0.4, https://newreleases.io/project/github/utmapp/UTM/release/v5.0.5, https://newreleases.io/project/github/utmapp/UTM/release/v5.0.6 (mirrors of github.com/utmapp/UTM releases).
- QEMU `qga/commands.c` (master): `GUEST_EXEC_MAX_OUTPUT`, `out_truncated`.
- Apple TN3179, Understanding local network privacy: https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy (title only retrieved; behavior inferred from the router test).
- BLACKBOARD 2026-10-09 Decisions 103, 104; `scripts/vm_guest.py`.
