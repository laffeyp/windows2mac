# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""laptop_dtrace.py — system-call traces recorded on the Windows laptop with DTrace (Sprint 092; docs/solution-2026-10-16.md,
E1 and E3).

DTrace is installed on the laptop as Microsoft documents it (Microsoft Learn, "DTrace on Windows": the MSI, `bcdedit /set
dtrace ON`, a restart); its syscall provider gives each NT system call's entry and return with its arguments.

`start` (E1): builds probes/hello on the laptop with the probe executor's flags, runs it under
tools/laptop_dtrace/process_start.d as a child of `cmd /c` (so every probe is armed before the process exists), and
writes store/laptop-traces/process-start-hello.tsv.gz: a header (the build, the D script's and hello.exe's sha256),
then every line in timestamp order. It prints the start's summary: the calls before main's first act (its NtCreateFile
of the trace file), the distinct calls, and the names and ports the start opens.

`frames` (E3): arms tools/laptop_dtrace/wait_for_process.d on the laptop, runs the game job (scripts/windows_machine.py
game, one execution in play mode with --pmc, which records DXGI's Present events per second), and once the game has
run ATTACH seconds attaches tools/laptop_dtrace/frame_calls.d to it, counting its system calls and win32u calls over
WINDOW seconds from DTrace's second FROM. Frames are the game's NtGdiDdDDIPresent calls in the same window; the run's
DXGI Present starts (presents.tsv) at the game's seconds ATTACH + FROM on must agree within 1%. Any DTrace message on
stderr (drops among them), a window off 10 s by more than 0.1 s, or no frames fails the run. Writes
store/laptop-traces/frame-calls-hail-to-the-rainbow.tsv: each call's kind, count and count per frame.

`session` (Sprint 093, E2's judge): a person plays the game for up to SESSION seconds (scripts/windows_machine.py game
--person --record, full screen, no keys sent, the keyboard and mouse kept as inputs.jsonl). tools/laptop_dtrace/
session_system.d, armed before the game starts, counts every NT system call of each process whose name contains
"Hail"; once the game has run 5 s, tools/laptop_dtrace/session_win32u.d attaches to it and counts its win32u calls,
the three unprobed ntdll stubs, and its NtGdiDdDDIPresent calls each 15 s. Both print cumulative blocks every 15 s, so
the last block is the session's total. Writes store/laptop-traces/session-<run>/ (the two outputs, gzipped) and prints
the calls and the frames per second, 15 s at a time.

Usage: uv run python scripts/laptop_dtrace.py start
       uv run python scripts/laptop_dtrace.py frames
       uv run python scripts/laptop_dtrace.py session [--input <recorded session>] [--pmc] [--seconds N]
"""

from __future__ import annotations

import argparse
import base64
import gzip
import hashlib
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOST = "w2m-laptop"
STAGE = "C:/w2m-stage/e1"
DTRACE = r"C:\Program Files\DTrace\dtrace.exe"
CLANG = r"C:\Program Files\LLVM\bin\clang.exe"
OUT = ROOT / "store" / "laptop-traces"
PORT_CALLS = ("NtAlpcConnectPort", "NtAlpcConnectPortEx", "NtConnectPort", "NtSecureConnectPort")


def powershell(script: str) -> str:
    """Run a PowerShell script on the laptop (encoded, so no quoting crosses ssh); its output."""
    encoded = base64.b64encode(script.encode("utf-16-le")).decode()
    r = subprocess.run(["ssh", HOST, f"powershell -NoProfile -EncodedCommand {encoded}"], capture_output=True, text=True)
    if r.returncode != 0:      # the laptop's own error text, not only the exit status
        raise RuntimeError(f"powershell exit {r.returncode}\n{r.stdout}\n{r.stderr}")
    return "\n".join(line for line in r.stdout.splitlines() if not line.startswith(("#< CLIXML", "<Objs")))


def windows_build() -> str:
    """The laptop's Windows build and update revision, as 26200.7840."""
    return powershell('$k=Get-ItemProperty "HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"; '
                      '"$($k.CurrentBuild).$($k.UBR)"').strip()


def ordered(text: str) -> list[list[str]]:
    """The trace's lines, split on tabs, in timestamp order (DTrace's buffers are per CPU); a call's entry and the
    name read in the same probe share a timestamp, so the entry comes first, then the name, then any return."""
    rows = [line.split("\t") for line in text.replace("\r", "").splitlines() if line]
    return sorted(rows, key=lambda r: (int(r[1]), "ENR".index(r[0])))


def summary(rows: list[list[str]], first_act: str) -> dict[str, object]:
    """Calls before main's first act (the NtCreateFile naming `first_act`), distinct calls, names and ports."""
    named = next((r for r in rows if r[0] == "N" and r[3] == "NtCreateFile" and r[5].endswith(first_act)), None)
    main_at = next((i for i, r in enumerate(rows) if named and r[0] == "E" and r[1:4] == named[1:4]), len(rows))
    before = [r for r in rows[:main_at] if r[0] == "E"]
    every = [r for r in rows if r[0] == "E"]
    return {"calls_before_main": len(before), "distinct_before_main": len({r[3] for r in before}),
            "calls": len(every), "distinct": len({r[3] for r in every}),
            "by_call_before_main": Counter(r[3] for r in before),
            "names_before_main": [(r[3], r[5]) for r in rows[:main_at] if r[0] == "N"],
            "ports": [(r[3], r[5]) for r in rows if r[0] == "N" and r[3] in PORT_CALLS]}


def start() -> int:
    script = ROOT / "tools" / "laptop_dtrace" / "process_start.d"
    subprocess.run(["ssh", HOST, f'cmd /c "mkdir {STAGE.replace("/", chr(92))} 2>nul"'], check=False)
    for src in (ROOT / "probes" / "hello" / "hello.c", script):
        subprocess.run(["scp", "-q", str(src), f"{HOST}:{STAGE}/{src.name}"], check=True)
    stage = STAGE.replace("/", "\\")
    out = powershell(f"""$ProgressPreference="SilentlyContinue"; cd {stage}
& "{CLANG}" -O1 -Wall -Werror "-Wl,/Brepro" -o hello.exe hello.c -ladvapi32 -lbcrypt
if ($LASTEXITCODE -ne 0) {{ "clang exit $LASTEXITCODE"; exit 1 }}
"hello " + (Get-FileHash hello.exe).Hash.ToLower()
$k=Get-ItemProperty "HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"; "build $($k.CurrentBuild).$($k.UBR)"
Remove-Item start.txt -ErrorAction SilentlyContinue
& "{DTRACE}" -q -s process_start.d -o start.txt -c "cmd.exe /c {stage}\\hello.exe e1 {stage}\\trace.txt" hello.exe
"dtrace exit $LASTEXITCODE" """)
    print(out)
    facts = {key: line[len(key) + 1:] for line in out.splitlines() for key in ("hello", "build", "dtrace exit")
             if line.startswith(key + " ")}
    if facts.get("dtrace exit") != "0" or "hello" not in facts:
        return 1
    text = subprocess.run(["ssh", HOST, f"type {stage}\\start.txt"], capture_output=True, text=True, check=True).stdout
    rows = ordered(text)
    OUT.mkdir(parents=True, exist_ok=True)
    header = (f"# build {facts['build']}; process_start.d sha256 {hashlib.sha256(script.read_bytes()).hexdigest()}; "
              f"hello.exe sha256 {facts['hello']}; probes/hello/hello.c, clang -O1, run as cmd /c child\n")
    body = "".join("\t".join(r) + "\n" for r in rows)
    (OUT / "process-start-hello.tsv.gz").write_bytes(gzip.compress((header + body).encode(), mtime=0))
    s = summary(rows, "\\trace.txt")
    print(f"calls {s['calls']} ({s['distinct']} distinct); before main {s['calls_before_main']} "
          f"({s['distinct_before_main']} distinct)")
    for call, n in sorted(s["by_call_before_main"].items(), key=lambda kv: (-kv[1], kv[0])):  # type: ignore[attr-defined]
        print(f"  {n:4d} {call}")
    for call, name in s["names_before_main"]:  # type: ignore[attr-defined]
        print(f"  name {call}: {name}")
    for call, name in s["ports"]:  # type: ignore[attr-defined]
        print(f"  port {call}: {name}")
    return 0


ATTACH, FROM, WINDOW = 60, 20, 10        # the first scene is at 62 s (recentering 2026-10-14): calls at 80-90 s


def win32u_stubs() -> set[str]:
    """win32u's exports that are system-call stubs (mov r10, rcx; mov eax, imm32), from the copied C:\\Windows (091):
    1,486 of 1,534; the pid provider also lists 7 exports that are not stubs (ABI_Get_ForegroundWindow, among others,
    reads memory and enters no kernel)."""
    from w2m.pe import _rva_to_offset, parse
    data = (Path.home() / "w2m-depots" / "windows" / "Windows" / "System32" / "win32u.dll").read_bytes()
    image = parse(data)
    return {name for name, rva in image.exports
            if data[_rva_to_offset(image.sections, rva):][:4] == bytes.fromhex("4c8bd1b8")}


KINDS = {"S": "system", "N": "system (ntdll stub)", "W": "win32u", "X": "win32u export, not a system call"}


def frames() -> int:
    stage = "C:\\w2m-stage\\e3"
    subprocess.run(["ssh", HOST, f'cmd /c "mkdir {stage} 2>nul"'], check=False)
    for name in ("wait_for_process.d", "frame_calls.d"):
        subprocess.run(["scp", "-q", str(ROOT / "tools" / "laptop_dtrace" / name), f"{HOST}:C:/w2m-stage/e3/{name}"],
                       check=True)
    tracer = f"""cd {stage}; Remove-Item calls.txt, errors.txt -ErrorAction SilentlyContinue
$game = "$(& "{DTRACE}" -q -s wait_for_process.d Hail {ATTACH} | Select-Object -First 1)".Trim()
"game pid $game"
if (-not $game) {{ "dtrace exit no-game"; exit 1 }}
& "{DTRACE}" -q -s frame_calls.d -p $game -o calls.txt {FROM} {WINDOW} 2> errors.txt
"dtrace exit $LASTEXITCODE"
Get-Content errors.txt | ForEach-Object {{ "dtrace stderr $_" }}"""
    encoded = base64.b64encode(tracer.encode("utf-16-le")).decode()
    trace = subprocess.Popen(["ssh", HOST, f"powershell -NoProfile -EncodedCommand {encoded}"], stdout=subprocess.PIPE,
                             text=True)
    game = subprocess.run([sys.executable, str(ROOT / "scripts" / "windows_machine.py"), "game", "--executions", "1",
                           "--seconds", "150", "--mode", "play", "--pmc", "--capture", "off"],
                          capture_output=True, text=True)
    print(game.stdout[-2000:])
    out = trace.communicate()[0]
    print(out)
    if game.returncode != 0 or "1 of 1 executions back" not in game.stdout:
        print("the game job failed")
        return 1
    if trace.returncode != 0 or "dtrace exit 0" not in out or "dtrace stderr" in out:   # drops and errors fail the run
        print("the trace failed")
        return 1
    calls = subprocess.run(["ssh", HOST, f"type {stage}\\calls.txt"], capture_output=True, text=True,
                           check=True).stdout.replace("\r", "")
    rows = [line.split("\t") for line in calls.splitlines() if line and line[0] in "SNW"]
    window = next(line.split("\t") for line in calls.splitlines() if line.startswith("T\t"))
    window_ns = int(window[2]) - int(window[1])
    stubs = win32u_stubs()
    rows = [["X" if r[0] == "W" and r[1] not in stubs else r[0], r[1], r[2]] for r in rows]
    # Frames in DTrace's own window: the game's NtGdiDdDDIPresent calls (one a frame); presents.tsv's DXGI Present
    # starts at the game's seconds ATTACH + FROM on are the check (a second clock, so only near agreement is asked).
    frame_count = sum(int(r[2]) for r in rows if r[0] == "W" and r[1] == "NtGdiDdDDIPresent")
    run = next((w for w in game.stdout.split() if w.startswith("lap-")), "").rstrip(":")
    presents = ROOT / "build" / "game-runs" / f"{run}-01" / "pmc" / "presents.tsv"
    per_second: Counter[int] = Counter()
    for line in presents.read_text().splitlines()[1:]:
        cells = line.split("\t")
        if cells[3] == "Microsoft-Windows-DXGI/IDXGISwapChain_Present/win:Start":
            per_second[int(cells[0])] += int(cells[4])
    seconds = range(ATTACH + FROM, ATTACH + FROM + WINDOW)
    dxgi_count = sum(per_second[t] for t in seconds)
    print(f"window {window_ns} ns; frames {frame_count} (NtGdiDdDDIPresent), presents.tsv {dxgi_count} (seconds "
          f"{seconds.start}-{seconds.stop - 1})")
    if frame_count == 0 or abs(frame_count - dxgi_count) > 0.01 * dxgi_count or abs(window_ns - WINDOW * 10**9) > 10**8:
        print("the frame count or the window disagrees")
        return 1
    OUT.mkdir(parents=True, exist_ok=True)
    script = ROOT / "tools" / "laptop_dtrace" / "frame_calls.d"
    lines = [f"# build {windows_build()}; run {run}-01; window {window_ns} ns; frames {frame_count} (NtGdiDdDDIPresent "
             f"in the window; presents.tsv seconds {seconds.start}-{seconds.stop - 1}: {dxgi_count}); frame_calls.d "
             f"sha256 {hashlib.sha256(script.read_bytes()).hexdigest()}",
             "kind\tcall\tcount\tper_frame"]
    for kind, call, n in sorted(rows, key=lambda r: (list(KINDS).index(r[0]), -int(r[2]), r[1])):
        lines.append(f"{KINDS[kind]}\t{call}\t{n}\t{int(n) / frame_count:.2f}")
    (OUT / "frame-calls-hail-to-the-rainbow.tsv").write_text("\n".join(lines) + "\n")
    total = {k: sum(int(r[2]) for r in rows if r[0] == k) for k in KINDS}
    print(f"system calls {total['S'] + total['N']} ({(total['S'] + total['N']) / frame_count:.1f} a frame; "
          f"{total['N']} through the three unprobed ntdll stubs), win32u system calls {total['W']} "
          f"({total['W'] / frame_count:.1f} a frame), other win32u exports {total['X']}")
    return 0


SESSION = 7200


def last_counts(text: str, kind: str) -> dict[str, int]:
    """The last block's counts of `kind` lines in a session output, keyed by call (by "pid call" for S lines). A
    pid-attached DTrace that exits with its target prints its aggregations once more in DTrace's own format (measured
    2026-10-16 on notepad: "  NtUserUnregisterClass   1"), which this does not read: the 15 s blocks before it hold
    all but the last seconds."""
    counts: dict[str, int] = {}
    for line in text.replace("\r", "").splitlines():
        if line.startswith("B\t"):
            counts = {}
        elif line.startswith(kind + "\t"):
            cells = line.split("\t")
            counts[" ".join(cells[1:-1])] = int(cells[-1])
    return counts


def session(inputs: str | None = None, pmc: bool = False, seconds: int = SESSION) -> int:
    stage = "C:\\w2m-stage\\session"
    subprocess.run(["ssh", HOST, f'cmd /c "mkdir {stage} 2>nul"'], check=False)
    for name in ("wait_for_process.d", "session_system.d", "session_win32u.d"):
        subprocess.run(["scp", "-q", str(ROOT / "tools" / "laptop_dtrace" / name), f"{HOST}:C:/w2m-stage/session/{name}"],
                       check=True)
    # The system-call DTrace is stopped by this script once the game has gone (it ends only after its seconds); the
    # win32u DTrace ends with its target (measured on notepad, 2026-10-16). An empty pid would make -p read -o as the
    # pid (measured: "dtrace: invalid pid: -o"), so a missing or ended game fails the run here.
    tracer = f"""cd {stage}; Remove-Item system.txt, win32u.txt, *.err -ErrorAction SilentlyContinue
$sys = Start-Process "{DTRACE}" -ArgumentList '-q -s session_system.d Hail {seconds + 600} -o system.txt' -PassThru `
  -NoNewWindow -RedirectStandardError system.err
$game = "$(& "{DTRACE}" -q -s wait_for_process.d Hail 5 | Select-Object -First 1)".Trim()
"game pid $game"
if (-not $game -or -not (Get-Process -Id $game -ErrorAction SilentlyContinue)) {{
  "dtrace exit no-game"; Stop-Process -Id $sys.Id -Force; exit 1 }}
& "{DTRACE}" -q -s session_win32u.d -p $game -o win32u.txt 2> win32u.err
"win32u dtrace exit $LASTEXITCODE"
[void]$sys.WaitForExit(20000)
if (-not $sys.HasExited) {{ Stop-Process -Id $sys.Id -Force }}
Get-Content system.err, win32u.err | ForEach-Object {{ "dtrace stderr $_" }}"""
    encoded = base64.b64encode(tracer.encode("utf-16-le")).decode()
    trace = subprocess.Popen(["ssh", HOST, f"powershell -NoProfile -EncodedCommand {encoded}"], stdout=subprocess.PIPE,
                             text=True)
    job = ["--input", inputs] if inputs else ["--record"]           # a replay of a recorded session, or a person
    game = subprocess.run([sys.executable, str(ROOT / "scripts" / "windows_machine.py"), "game", "--executions", "1",
                           "--seconds", str(seconds), "--mode", "play", "--person", *job, "--capture", "off",
                           *(["--pmc"] if pmc else [])], capture_output=True, text=True)
    print(game.stdout[-2000:])
    out = trace.communicate()[0]
    print(out)
    run = next((w for w in game.stdout.split() if w.startswith("lap-")), "").rstrip(":")
    dest = OUT / f"session-{run or 'unknown'}"
    dest.mkdir(parents=True, exist_ok=True)
    texts = {}
    for name in ("system.txt", "win32u.txt"):
        texts[name] = subprocess.run(["ssh", HOST, f"type {stage}\\{name}"], capture_output=True,
                                     text=True).stdout.replace("\r", "")
        (dest / f"{name}.gz").write_bytes(gzip.compress(texts[name].encode(), mtime=0))
    system = last_counts(texts["system.txt"], "S")
    win32u = last_counts(texts["win32u.txt"], "W")
    ntdll = last_counts(texts["win32u.txt"], "N")
    pids = sorted({k.split()[0] for k in system})
    calls = sorted({k.split()[1] for k in system} | set(ntdll))
    stubs = win32u_stubs()
    print(f"processes {pids}; NT calls {len(calls)} kinds, {sum(system.values()) + sum(ntdll.values())} calls; "
          f"win32k calls {len([c for c in win32u if c in stubs])} kinds, "
          f"{sum(n for c, n in win32u.items() if c in stubs)} calls")
    fps = [int(line.split("\t")[1]) / 15 for line in texts["win32u.txt"].splitlines() if line.startswith("F\t")]
    print("frames a second, 15 s at a time: " + " ".join(f"{f:.0f}" for f in fps))
    failed = game.returncode != 0 or "1 of 1 executions back" not in game.stdout or "dtrace stderr" in out \
        or not system or not win32u
    print("the session failed" if failed else f"session stored in {dest.relative_to(ROOT)}")
    return 1 if failed else 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("what", choices=["start", "frames", "session"])
    ap.add_argument("--input", help="session: replay games/hail-to-the-rainbow/inputs/<name>.jsonl instead of a person")
    ap.add_argument("--pmc", action="store_true", help="session: also record presents and their times (--pmc)")
    ap.add_argument("--seconds", type=int, default=SESSION, help="session: the game's seconds at most")
    a = ap.parse_args(argv)
    if a.what == "session":
        return session(a.input, a.pmc, a.seconds)
    return start() if a.what == "start" else frames()


if __name__ == "__main__":
    sys.exit(main())
