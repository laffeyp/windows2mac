"""acquire_depot.py — a Steam app's Windows depot acquired with Valve's SteamCMD, on the signal
plane (Sprint 038).

Two steps, the second caused by the first:

  resolver  DISTRIBUTION_RESOLVED: the public branch's build id and the app's own Windows depots,
            read with SteamCMD's anonymous login (`+app_info_print`; no account needed)
  acquirer  DISTRIBUTION_ACQUIRED when the install directory holds that build
            (`steamapps/appmanifest_<app>.acf`) — content_sha256 hashes the sorted file manifest
            ("path\\tsize\\tsha256" per file, SteamCMD's own bookkeeping excluded); otherwise it runs
            `+app_update <app> validate` with the account's cached login and, when SteamCMD asks
            for a password, ACQUISITION_FAILED auth_required: a real account step is pending, and
            the run never reports success it did not see (v0.2 §4.1)

Usage: uv run python scripts/acquire_depot.py <app_id> <install_dir> [--user NAME] [--runs runs]
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import re
import subprocess
import sys
import time
from collections.abc import AsyncIterator
from pathlib import Path
from typing import Any

import msgspec
from substrate.api import PerEvent, Runtime, Subscription, TopologyBuilder, read_record, threshold_count

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m import VOCAB_VERSION, events  # noqa: E402

STEAMCMD = Path.home() / "w2m-tools" / "steamcmd" / "steamcmd.sh"


def steamcmd(*args: str, timeout: int = 600) -> str:
    r = subprocess.run([str(STEAMCMD), *args, "+quit"], capture_output=True, text=True, timeout=timeout)
    return r.stdout + r.stderr


def parse_vdf(text: str) -> dict[str, Any]:
    """Valve's text KeyValues: quoted keys, quoted values or { } blocks."""
    tokens = re.findall(r'"((?:[^"\\]|\\.)*)"|([{}])', text)
    stack: list[dict[str, Any]] = [{}]
    key: str | None = None
    for quoted, brace in tokens:
        if brace == "{":
            child: dict[str, Any] = {}
            stack[-1][key or ""] = child
            stack.append(child)
            key = None
        elif brace == "}":
            if len(stack) > 1:
                stack.pop()
            key = None
        elif key is None:
            key = quoted
        else:
            stack[-1][key] = quoted
            key = None
    return stack[0]


def resolve(app: int) -> tuple[int, list[int]]:
    """(public build id, the app's own Windows depot ids) from SteamCMD's anonymous app info: depots
    with a manifest of their own, `oslist` windows, and no `depotfromapp` (a depot shared from another
    app, such as the redistributables)."""
    text = steamcmd("+login", "anonymous", "+app_info_update", "1", "+app_info_print", str(app))
    start = text.index(f'"{app}"')
    info = parse_vdf(text[start:]).get(str(app), {})
    depots = info.get("depots", {})
    build = depots.get("branches", {}).get("public", {}).get("buildid")
    if build is None:
        raise RuntimeError(f"no public build id in SteamCMD's app info for {app}")
    own = [int(k) for k, v in depots.items() if k.isdigit() and isinstance(v, dict)
           and "windows" in v.get("config", {}).get("oslist", "") and "manifests" in v and "depotfromapp" not in v]
    return int(build), sorted(own)


def manifest_sha256(install: Path) -> tuple[int, str]:
    """(bytes, sha256 of the sorted "path\\tsize\\tsha256" lines) over the installed files."""
    lines, total = [], 0
    for p in sorted(q for q in install.rglob("*") if q.is_file() and q.relative_to(install).parts[0] != "steamapps"):
        h = hashlib.sha256()
        with p.open("rb") as f:
            while chunk := f.read(1 << 22):
                h.update(chunk)
        size = p.stat().st_size
        total += size
        lines.append(f"{p.relative_to(install).as_posix()}\t{size}\t{h.hexdigest()}")
    return total, "sha256:" + hashlib.sha256("\n".join(lines).encode()).hexdigest()


def installed_build(install: Path, app: int) -> int | None:
    acf = install / "steamapps" / f"appmanifest_{app}.acf"
    if not acf.exists():
        return None
    m = re.search(r'"buildid"\s*"(\d+)"', acf.read_text())
    return int(m.group(1)) if m else None


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("app", type=int)
    ap.add_argument("install", type=Path)
    ap.add_argument("--user", default="wackturbine")
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    t0 = time.monotonic()
    found: dict[str, Any] = {}

    async def harness(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        yield events.validate(events.SESSION_INIT(
            vocab_version=VOCAB_VERSION, profile=f"steam-app-{a.app}", contract_version=VOCAB_VERSION,
            harness_version="0.0.1", purpose="product"))

    async def resolver(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        build, depots = resolve(a.app)
        found.update(build=build, depots=depots)
        yield events.validate(events.DISTRIBUTION_RESOLVED(
            app_id=a.app, build_id=build, depot_ids=depots, platform="windows"))

    async def acquirer(_inp: Any) -> AsyncIterator[msgspec.Struct]:
        build = found["build"]
        if installed_build(a.install, a.app) != build:
            out = steamcmd("+@sSteamCmdForcePlatformType", "windows", "+force_install_dir", str(a.install),
                           "+login", a.user, "+app_update", str(a.app), "validate", timeout=7200)
            if "password:" in out or "Cached credentials not found" in out:
                yield events.validate(events.ACQUISITION_FAILED(
                    app_id=a.app, reason="auth_required",
                    diagnostic=f"SteamCMD has no cached login for {a.user}; the account owner logs in once in a terminal"))
                yield events.validate(events.SESSION_COMPLETED(
                    outcome="aborted", discrepancy_count=0, accepted_count=0,
                    elapsed_seconds=round(time.monotonic() - t0, 3)))
                return
            if installed_build(a.install, a.app) != build:
                yield events.validate(events.ACQUISITION_FAILED(
                    app_id=a.app, reason="tool_error", diagnostic=out.strip().splitlines()[-1][:300]))
                yield events.validate(events.SESSION_COMPLETED(
                    outcome="aborted", discrepancy_count=0, accepted_count=0,
                    elapsed_seconds=round(time.monotonic() - t0, 3)))
                return
        size, digest = manifest_sha256(a.install)
        found.update(bytes=size, digest=digest)
        yield events.validate(events.DISTRIBUTION_ACQUIRED(
            app_id=a.app, build_id=build, bytes=size, content_sha256=digest, tool="steamcmd"))
        yield events.validate(events.SESSION_COMPLETED(
            outcome="completed", discrepancy_count=0, accepted_count=0,
            elapsed_seconds=round(time.monotonic() - t0, 3)))

    def topology(b: TopologyBuilder) -> None:
        b.baseline(app_id=a.app, install=str(a.install), tool="steamcmd")
        b.producer_kind("harness", schemas=[events.SESSION_INIT], schema_version=1, factory=lambda: harness)
        b.producer_kind("distribution_resolver", schemas=[events.DISTRIBUTION_RESOLVED], schema_version=1,
                        factory=lambda: resolver)
        b.producer_kind("acquirer", schemas=[events.DISTRIBUTION_ACQUIRED, events.ACQUISITION_FAILED,
                        events.SESSION_COMPLETED], schema_version=1, factory=lambda: acquirer)
        b.initial("harness", input=None)
        b.trigger("resolve", subscription=Subscription(kinds=frozenset({"SESSION_INIT"})), predicate=lambda c: True,
                  starts="distribution_resolver", policy=PerEvent(), input_builder=lambda c: None)
        b.trigger("acquire", subscription=Subscription(kinds=frozenset({"DISTRIBUTION_RESOLVED"})),
                  predicate=lambda c: True, starts="acquirer", policy=PerEvent(), input_builder=lambda c: None)
        b.termination(threshold_count("SESSION_COMPLETED", 1))

    result = asyncio.run(Runtime(a.runs / f"acquire-{a.app}-{time.time_ns()}").run(topology))
    for e in read_record(result.record_root):
        if not e["kind"].startswith("substrate.") or e["kind"] == "substrate.ProducerFailed":
            print(e["kind"], {k: v for k, v in e["payload"].items() if k not in ("vocab_version", "harness_version")})
    print(f"record {result.record_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
