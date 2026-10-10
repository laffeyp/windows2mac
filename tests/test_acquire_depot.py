# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 038: the acquisition driver resolves an app's own Windows depots from SteamCMD's app info
(not depots shared from another app), and reports auth_required instead of success when SteamCMD
has no cached login."""

from __future__ import annotations

import sys
from collections import Counter
from pathlib import Path

import pytest
from substrate.api import read_record

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import acquire_depot  # noqa: E402

APP_INFO = (ROOT / "tests" / "fixtures" / "steamcmd-app-info-409720.txt").read_text()


def test_parse_vdf_nested_blocks() -> None:
    v = acquire_depot.parse_vdf('"a" { "b" "1" "c" { "d" "x" } } "e" "2"')
    assert v == {"a": {"b": "1", "c": {"d": "x"}}, "e": "2"}


def test_resolve_keeps_own_windows_depots_only(monkeypatch: pytest.MonkeyPatch) -> None:
    """BioShock 2 Remastered lists four redistributable depots shared from app 228980 and one from
    1523210; its own Windows depot is 409721 (a regex resolver returned 228983 and 1523211)."""
    monkeypatch.setattr(acquire_depot, "steamcmd", lambda *a, **k: APP_INFO)
    build, depots = acquire_depot.resolve(409720)
    assert build == 8552776 and depots == [409721]


def test_missing_install_without_cached_login_is_auth_required(monkeypatch: pytest.MonkeyPatch,
                                                               tmp_path: Path) -> None:
    def fake(*args: str, **_: object) -> str:
        return APP_INFO if "anonymous" in args else "Cached credentials not found.\npassword: "
    monkeypatch.setattr(acquire_depot, "steamcmd", fake)
    assert acquire_depot.main(["409720", str(tmp_path / "empty"), "--runs", str(tmp_path)]) == 0
    rec = next(tmp_path.glob("acquire-409720-*"))
    kinds = Counter(e["kind"] for e in read_record(rec) if not e["kind"].startswith("substrate."))
    assert kinds["DISTRIBUTION_RESOLVED"] == 1 and kinds["ACQUISITION_FAILED"] == 1
    assert kinds["DISTRIBUTION_ACQUIRED"] == 0
    failed = next(e["payload"] for e in read_record(rec) if e["kind"] == "ACQUISITION_FAILED")
    assert failed["reason"] == "auth_required"
