"""rules — one module per x86 instruction class (Sprint 041; docs/plan.md Phase 4).

Each module names the iclasses it lifts (`ICLASSES`), gives the lifting rule (`rule(ctx)`, which
appends lift.py operations to `ctx.b`), and, for a class whose AArch64 code is more than the shared
operations (the SSE classes), its lowering (`lower(a, op, reg_of, store)`). A class is the unit an
item run judges (scripts/class_run.py): the module's sha256 is the candidate's identity, so editing a
rule makes a new candidate that must be judged again. Shared machinery lives in the `_` modules.
"""

from __future__ import annotations

import hashlib
import importlib
import pkgutil
from collections.abc import Callable
from dataclasses import dataclass
from functools import cache
from pathlib import Path
from types import ModuleType
from typing import Any

from w2m.x86dec import Fields


@dataclass
class Ctx:
    b: Any                        # lift._OpBuilder
    ic: str
    f: Fields | None
    ops: tuple[str, ...]
    nxt: int                      # address of the next instruction
    address: int


@dataclass(frozen=True)
class Rule:
    module: str
    path: Path
    rule: Callable[[Ctx], None]
    lower: Callable[..., None] | None
    shared: tuple[Path, ...]                  # the `_` modules it uses, and lift.py's operand parsing

    @property
    def sha256(self) -> str:
        """The candidate's identity: its module and every shared module it depends on."""
        h = hashlib.sha256()
        for p in (self.path, *self.shared):
            h.update(p.name.encode() + b"\0" + p.read_bytes() + b"\0")
        return "sha256:" + h.hexdigest()


@cache
def registry() -> dict[str, Rule]:
    """iclass -> Rule, from every public module of this package."""
    out: dict[str, Rule] = {}
    for info in pkgutil.iter_modules(__path__):
        if info.name.startswith("_"):
            continue
        m: ModuleType = importlib.import_module(f"{__name__}.{info.name}")
        for ic in m.ICLASSES:
            if ic in out:
                raise RuntimeError(f"iclass {ic} has two rules: {out[ic].module} and {info.name}")
            here = Path(__file__).parent
            shared = tuple(here / f"{name}.py" for name in getattr(m, "SHARED", ())) + (here.parent / "lift.py",)
            out[ic] = Rule(info.name, Path(m.__file__ or ""), m.rule, getattr(m, "lower", None), shared)
    return out
