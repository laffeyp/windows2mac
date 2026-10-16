# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 027: with W2M_REQUIRE_REFERENCES=1 (set in CI), a skipped test fails. Tests skip when
XED, llvm-objdump, or the corpus is absent; CI provides all three, so a skip there means the suite
checked less than it claims (review 2026-10-08 §5).

Exception (Sprint 030): a test marked `hvf` needs Hypervisor.framework, which exists only on
macOS; on any other host it skips without failing. The macOS gates (scripts/gates.sh) run it.
Exception (Sprint 085): a test marked `depot` reads a game's installed files (~/w2m-depots), which exist only on the
user's Mac and never in CI; where they are absent it skips without failing. The local gates run it."""

from __future__ import annotations

import os
import sys
from collections.abc import Generator
from pathlib import Path
from typing import Any

import pytest

DEPOTS = Path.home() / "w2m-depots"


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line("markers", "hvf: needs Hypervisor.framework (macOS on Apple silicon)")
    config.addinivalue_line("markers", "depot: reads a game's installed files under ~/w2m-depots (the user's Mac only)")


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    for item in items:
        if item.get_closest_marker("hvf") and sys.platform != "darwin":
            item.add_marker(pytest.mark.skip(reason="hvf: Hypervisor.framework needs macOS"))


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo[None]) -> Generator[None, Any, None]:
    outcome = yield
    report = outcome.get_result()
    hvf_off_mac = item.get_closest_marker("hvf") is not None and sys.platform != "darwin"
    no_depot = item.get_closest_marker("depot") is not None and not DEPOTS.is_dir()
    if os.environ.get("W2M_REQUIRE_REFERENCES") == "1" and report.skipped and not hvf_off_mac and not no_depot:
        report.outcome = "failed"
        report.longrepr = f"skipped under W2M_REQUIRE_REFERENCES=1: {report.longrepr}"
