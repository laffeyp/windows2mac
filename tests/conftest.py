"""Sprint 027: with W2M_REQUIRE_REFERENCES=1 (set in CI), a skipped test fails. Tests skip when
XED, llvm-objdump, or the corpus is absent; CI provides all three, so a skip there means the suite
checked less than it claims (review 2026-10-08 §5)."""

from __future__ import annotations

import os
from collections.abc import Generator
from typing import Any

import pytest


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item: pytest.Item, call: pytest.CallInfo[None]) -> Generator[None, Any, None]:
    outcome = yield
    report = outcome.get_result()
    if os.environ.get("W2M_REQUIRE_REFERENCES") == "1" and report.skipped:
        report.outcome = "failed"
        report.longrepr = f"skipped under W2M_REQUIRE_REFERENCES=1: {report.longrepr}"
