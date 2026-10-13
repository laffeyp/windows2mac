# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""vm_guest.py — control of the local Windows VM over SSH (BLACKBOARD 2026-10-09: the local VM's
role; review docs/review-2026-10-09-local-vm.md).

One SSH connection, multiplexed (`Host w2m-vm` in ~/.ssh/config: ControlMaster, ControlPersist), to
the guest's OpenSSH server (Win32-OpenSSH 10.0.0.0, DefaultShell cmd.exe). On it:

- `Guest`: one persistent `powershell.exe` reading commands on stdin. A call sends one line (the
  script, UTF-16 base64, run as a script block with `$ErrorActionPreference = 'Stop'`, so an error
  record fails the call, review V3) and reads output up to a marker line carrying the call's id and
  exit code. No process starts per call; a call costs one round trip. Every call has a deadline
  (V2); on expiry the session is killed and the call fails.
- `desktop()`: a script run in the console session (windows, input and the screen live there; SSH
  logons do not), through the resident desktop agent (tools/vm_desktop/agent.ps1): started at
  every logon by scheduled task w2m-desktop, it preloads System.Drawing and UI Automation and
  answers on 127.0.0.1:47001 inside the guest; the socket stays open in the SSH-side session, so a
  desktop call is one more local round trip (review L3; replaces a scheduled task per call, V1, V4).
- `push`/`pull`: scp over the same connection, each checked by sha256 on both sides.
- `sync_clock()`: the guest clock set from the Mac's, offset measured over one round trip.

Measured 2026-10-09: a one-off `ssh w2m-vm <cmd>` 30 ms; a `Guest.run` call is reported by
`--selftest`.

Usage:
  uv run python scripts/vm_guest.py run <powershell>        output; exit code is the script's
  uv run python scripts/vm_guest.py desktop <powershell>    the same, in the console session
  uv run python scripts/vm_guest.py install-agent           (re)install and start the desktop agent
  uv run python scripts/vm_guest.py screenshot <out.png>
  uv run python scripts/vm_guest.py push <local> <guest path>
  uv run python scripts/vm_guest.py pull <guest path> <local>
  uv run python scripts/vm_guest.py sync-clock
  uv run python scripts/vm_guest.py selftest
"""

from __future__ import annotations

import base64
import hashlib
import os
import select
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass
from pathlib import Path

HOST = "w2m-vm"
GUEST_DIR = r"C:\w2m"

# One line per call: decode the script, run it with errors as exceptions, then print the marker.
# `exit N` inside a script ends the whole PowerShell process; the session reports N and restarts.
WRAP = ("$ErrorActionPreference='Stop';$ProgressPreference='SilentlyContinue';$global:LASTEXITCODE=0;"
        "$c=0;try{{& ([scriptblock]::Create([Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('{b64}'))))"
        " *>&1 | Out-String -Width 4096 -Stream; if($LASTEXITCODE){{$c=$LASTEXITCODE}}}}"
        "catch{{($_ | Out-String -Width 4096).TrimEnd(); $c=1}};[Console]::Out.WriteLine('{marker} '+$c)")

AGENT_SOURCE = Path(__file__).resolve().parents[1] / "tools" / "vm_desktop" / "agent.ps1"
AGENT_GUEST = r"C:\w2m\desktop\agent.ps1"
AGENT_PORT = 47001

# Registers the agent to start at every logon of the console user and starts it now; returns when
# its socket accepts a connection, or fails at the deadline.
INSTALL_AGENT = r"""
$u = (Get-CimInstance Win32_ComputerSystem).UserName
if (-not $u) { throw 'no user is logged on at the console' }
$a = New-ScheduledTaskAction -Execute powershell.exe -Argument '-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File %AGENT%'
$t = New-ScheduledTaskTrigger -AtLogOn -User $u
$p = New-ScheduledTaskPrincipal -UserId $u -LogonType Interactive
$s = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName w2m-desktop -Action $a -Trigger $t -Principal $p -Settings $s -Force | Out-Null
Stop-ScheduledTask -TaskName w2m-desktop
Start-ScheduledTask -TaskName w2m-desktop
$deadline = (Get-Date).AddSeconds(30)
while ($true) {
    try { $c = [Net.Sockets.TcpClient]::new('127.0.0.1', %PORT%); $c.Close(); break } catch {}
    if ((Get-Date) -gt $deadline) { throw 'desktop agent did not open its port within 30 s' }
    Start-Sleep -Milliseconds 20
}
"""

# Defined once per SSH-side session: sends one script (UTF-16 base64) to the desktop agent over a
# connection kept open in that session. Defining it once matters: Defender scans every script block
# before it runs, and sending this client with each call cost ~300 ms a call (measured 2026-10-09).
DESKTOP_FUNCTION = r"""
function global:W2M-Desktop([string]$b) {
if (-not $global:w2mDesk -or -not $global:w2mDesk.Connected) {
    $global:w2mDesk = [Net.Sockets.TcpClient]::new('127.0.0.1', %PORT%)
    $st = $global:w2mDesk.GetStream()
    $global:w2mDeskR = New-Object IO.StreamReader($st)
    $global:w2mDeskW = New-Object IO.StreamWriter($st)
    $global:w2mDeskW.AutoFlush = $true
}
$global:w2mDeskW.WriteLine($b)
$line = $global:w2mDeskR.ReadLine()
if ($null -eq $line) { $global:w2mDesk = $null; throw 'desktop agent closed the connection' }
$i = $line.IndexOf(' ')
[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($line.Substring($i + 1))).TrimEnd()
$global:LASTEXITCODE = [int]$line.Substring(0, $i)
}
"""


class GuestError(RuntimeError):
    """A guest call failed to complete (deadline, lost connection, failed transfer)."""


@dataclass
class Call:
    code: int
    output: str
    seconds: float


class Guest:
    """One persistent PowerShell in the guest over the multiplexed SSH connection."""

    def __init__(self) -> None:
        self.proc: subprocess.Popen[bytes] | None = None
        self.buf = b""
        self.ready = False

    def _start(self) -> subprocess.Popen[bytes]:
        if self.proc is None or self.proc.poll() is not None:
            self.proc = subprocess.Popen(
                ["ssh", HOST, "powershell.exe", "-NoLogo", "-NoProfile", "-NonInteractive",
                 "-ExecutionPolicy", "Bypass", "-Command", "-"],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            self.buf = b""
            self.ready = False
        return self.proc

    def run(self, script: str, timeout: float = 60.0) -> Call:
        p = self._start()
        if not self.ready:
            self.ready = True
            r = self.run(DESKTOP_FUNCTION.replace("%PORT%", str(AGENT_PORT)), timeout)
            if r.code:
                raise GuestError(f"session setup: {r.output.strip()}")
        assert p.stdin and p.stdout
        marker = f"__w2m_{uuid.uuid4().hex}"
        b64 = base64.b64encode(script.encode("utf-16-le")).decode()
        t0 = time.monotonic()
        p.stdin.write((WRAP.format(b64=b64, marker=marker) + "\n").encode())
        p.stdin.flush()
        fd = p.stdout.fileno()
        end = marker.encode()
        while True:
            i = self.buf.find(end)
            if i >= 0:
                j = self.buf.find(b"\n", i)
                if j >= 0:
                    out = self.buf[:i].decode("utf-8", "replace").replace("\r", "")
                    code = int(self.buf[i + len(end):j].strip())
                    self.buf = self.buf[j + 1:]
                    return Call(code, out, time.monotonic() - t0)
            left = timeout - (time.monotonic() - t0)
            if left <= 0:
                self.close()
                raise GuestError(f"guest call passed its {timeout} s deadline")
            ready, _, _ = select.select([fd], [], [], left)
            if ready:
                chunk = os.read(fd, 65536)
                if not chunk:                                  # the script ran `exit N`
                    code = p.wait()
                    out = self.buf.decode("utf-8", "replace").replace("\r", "")
                    self.proc, self.buf, self.ready = None, b"", False
                    return Call(code, out, time.monotonic() - t0)
                self.buf += chunk

    def close(self) -> None:
        if self.proc and self.proc.poll() is None:
            self.proc.kill()
            self.proc.wait()
        self.proc = None


_guest = Guest()


def run(script: str, timeout: float = 60.0) -> Call:
    return _guest.run(script, timeout)


def _sha256_guest(path: str) -> str:
    r = run(f"(Get-FileHash -Algorithm SHA256 -LiteralPath '{path}').Hash")
    if r.code:
        raise GuestError(f"hash {path}: {r.output.strip()}")
    return r.output.strip().lower()


def _scp_path(guest: str) -> str:
    return "/" + guest.replace("\\", "/")                     # C:\w2m\x -> /C:/w2m/x


def push(local: Path, guest: str) -> None:
    r = subprocess.run(["scp", "-q", "-o", "ConnectTimeout=15", "-o", "ServerAliveInterval=15", str(local),
                        f"{HOST}:{_scp_path(guest)}"], capture_output=True, text=True, timeout=1800)
    if r.returncode:
        raise GuestError(f"push {guest}: {r.stderr.strip()}")
    if _sha256_guest(guest) != hashlib.sha256(local.read_bytes()).hexdigest():
        raise GuestError(f"push {guest}: guest copy differs from {local}")


def pull(guest: str, local: Path) -> None:
    r = subprocess.run(["scp", "-q", "-o", "ConnectTimeout=15", "-o", "ServerAliveInterval=15",
                        f"{HOST}:{_scp_path(guest)}", str(local)], capture_output=True, text=True, timeout=1800)
    if r.returncode:
        raise GuestError(f"pull {guest}: {r.stderr.strip()}")
    if hashlib.sha256(local.read_bytes()).hexdigest() != _sha256_guest(guest):
        raise GuestError(f"pull {guest}: local copy differs")


def install_agent() -> None:
    """Copy tools/vm_desktop/agent.ps1 into the guest and (re)start it in the console session."""
    r = run(r"New-Item -ItemType Directory -Force C:\w2m\desktop | Out-Null")
    if r.code:
        raise GuestError(r.output)
    push(AGENT_SOURCE, AGENT_GUEST)
    r = run(INSTALL_AGENT.replace("%AGENT%", AGENT_GUEST).replace("%PORT%", str(AGENT_PORT)), 45)
    if r.code:
        raise GuestError(f"install desktop agent: {r.output.strip()}")


def desktop(script: str, timeout: float = 60.0) -> Call:
    """Run `script` in the console session through the resident desktop agent."""
    b64 = base64.b64encode(script.encode("utf-16-le")).decode()
    return run(f"W2M-Desktop '{b64}'", timeout)


def screenshot(out: Path) -> None:
    """The console session's screen as PNG, sent back over the agent's socket with its sha256."""
    r = desktop("Get-ScreenPng")
    if r.code:
        raise GuestError(f"screenshot: {r.output.strip()}")
    digest, b64 = r.output.strip().split(" ", 1)
    data = base64.b64decode(b64)
    if hashlib.sha256(data).hexdigest() != digest:
        raise GuestError("screenshot: bytes differ from the guest's sha256")
    out.write_bytes(data)


def sync_clock() -> float:
    """Set the guest clock to the Mac's (UTC); return the offset found, in ms (guest minus host).
    The guest's reading is taken mid-way through one round trip of the open session."""
    run("1")                                                    # session warm
    t0 = time.time()
    r = run("[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()")
    t1 = time.time()
    offset = float(r.output.strip()) - (t0 + t1) / 2 * 1000
    s = run(f"Set-Date -Adjust ([TimeSpan]::FromMilliseconds({-offset:.0f})) | Out-Null")
    if s.code:
        raise GuestError(f"set clock: {s.output.strip()}")
    return offset


def selftest() -> int:
    """Exit codes, errors, deadline and per-call time, against known answers."""
    checks: list[tuple[str, object, bool]] = []
    run("1")
    calls = [run("$null") for _ in range(20)]
    per = sorted(c.seconds for c in calls)[10] * 1000
    checks.append(("median call ms", per, per < 50))
    checks.append(("output", run("'abc'").output.strip(), run("'abc'").output.strip() == "abc"))
    checks.append(("error -> 1", run("Get-Item C:\\nope").code, run("Get-Item C:\\nope").code == 1))
    checks.append(("native exit 7", run("cmd /c exit 7").code, run("cmd /c exit 7").code == 7))
    checks.append(("exit 9 ends session", run("exit 9").code, True))
    checks.append(("session restarts", run("'again'").output.strip(), run("'again'").output.strip() == "again"))
    d = [desktop("'x'") for _ in range(10)]
    dper = sorted(c.seconds for c in d)[5] * 1000
    checks.append(("desktop median ms", round(dper, 1), dper < 250 and all(c.output.strip() == "x" for c in d)))
    checks.append(("desktop session id", desktop("[Diagnostics.Process]::GetCurrentProcess().SessionId").output.strip(),
                   desktop("[Diagnostics.Process]::GetCurrentProcess().SessionId").output.strip() != "0"))
    checks.append(("desktop error -> 1", desktop("Get-Item C:\\nope").code, desktop("Get-Item C:\\nope").code == 1))
    shot = Path("/tmp/w2m-selftest.png")
    t = time.monotonic()
    screenshot(shot)
    st = time.monotonic() - t
    checks.append(("screenshot ms", round(st * 1000), st < 2.0 and shot.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"))
    t = time.monotonic()
    try:
        run("Start-Sleep 5", timeout=1)
        checks.append(("deadline", "no error", False))
    except GuestError:
        checks.append(("deadline s", round(time.monotonic() - t, 2), time.monotonic() - t < 2))
    for name, value, ok in checks:
        print(f"{'ok  ' if ok else 'FAIL'} {name}: {value}")
    return 0 if all(ok for _, _, ok in checks) else 1


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 2
    cmd, rest = argv[0], argv[1:]
    try:
        if cmd in ("run", "desktop"):
            r = (run if cmd == "run" else desktop)(" ".join(rest))
            sys.stdout.write(r.output)
            return r.code
        if cmd == "screenshot":
            screenshot(Path(rest[0]))
        elif cmd == "push":
            push(Path(rest[0]), rest[1])
        elif cmd == "pull":
            pull(rest[0], Path(rest[1]))
        elif cmd == "install-agent":
            install_agent()
        elif cmd == "sync-clock":
            print(f"offset before: {sync_clock():.0f} ms; after: {sync_clock():.0f} ms")
        elif cmd == "selftest":
            return selftest()
        else:
            print(__doc__)
            return 2
    except GuestError as e:
        print(e, file=sys.stderr)
        return 1
    finally:
        _guest.close()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
