"""candidate.py — admit, prompt for, and install a generated kernel32 implementation (Sprint 031).

A candidate is Python source for the functions a contract governs. It reaches the runtime only
through `admit`, which parses it and allows a fixed set of syntax. Each function takes
`(thread, args)`. `thread.last_error` may be read and assigned, and `thread.tid` read. `args` is
the tuple of the function's declared parameters, in order (PARAMETER_COUNT; at most four, from
RCX, RDX, R8, R9). Sprint 032: with four registers always passed, a GetLastError returning
`args[0]` read RCX, which can still hold SetLastError's argument. The function returns an int,
which the runtime places in RAX, or None. No calls, imports, loops, comprehensions, lambdas,
or other attributes are admitted, and the code runs with empty builtins.

The check limits what a local code model's output can do inside the harness. It is not a
security boundary against an adversary who controls the generator.
"""

from __future__ import annotations

import ast
import hashlib
import re
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any

from w2m.contract import Contract
from w2m.kernel32 import M64, Kernel32, Thread

CandidateFunction = Callable[[Any, tuple[int, ...]], Any]

ALLOWED_NODES = (
    ast.Module, ast.FunctionDef, ast.arguments, ast.arg, ast.Return, ast.Assign, ast.AugAssign,
    ast.Expr, ast.Constant, ast.Name, ast.Load, ast.Store, ast.Attribute, ast.Subscript,
    ast.BinOp, ast.UnaryOp, ast.BoolOp, ast.Compare, ast.If, ast.IfExp, ast.Pass,
    ast.Add, ast.Sub, ast.Mult, ast.FloorDiv, ast.Mod, ast.BitAnd, ast.BitOr, ast.BitXor,
    ast.LShift, ast.RShift, ast.Invert, ast.Not, ast.USub, ast.UAdd, ast.And, ast.Or,
    ast.Eq, ast.NotEq, ast.Lt, ast.LtE, ast.Gt, ast.GtE, ast.Is, ast.IsNot,
)
READABLE = {"last_error", "tid"}
# Declared parameters, from the Microsoft Learn signatures (retrieved 2026-10-08):
# void SetLastError(DWORD dwErrCode); DWORD GetLastError(void).
PARAMETER_COUNT = {"SetLastError": 1, "GetLastError": 0}
WRITABLE = {"last_error"}


class CandidateError(ValueError):
    """The candidate source is outside the admitted language or interface."""


@dataclass(frozen=True)
class Candidate:
    source: str
    sha256: str                       # "sha256:<hex>" of the source bytes
    functions: dict[str, CandidateFunction]


class _ThreadView:
    """What a candidate sees of the calling thread."""

    __slots__ = ("_th",)
    _th: Thread

    def __init__(self, th: Thread) -> None:
        object.__setattr__(self, "_th", th)

    @property
    def tid(self) -> int:
        return int(self._th.tid)

    @property
    def last_error(self) -> int:
        return int(self._th.last_error)

    @last_error.setter
    def last_error(self, v: int) -> None:
        if not isinstance(v, int):
            raise CandidateError(f"thread.last_error assigned {type(v).__name__}")
        self._th.last_error = v & 0xFFFFFFFF


def sha256_of(source: str) -> str:
    return "sha256:" + hashlib.sha256(source.encode()).hexdigest()


def admit(source: str, names: tuple[str, ...]) -> Candidate:
    """Parse `source` and admit it when it defines exactly `names` in the allowed language."""
    try:
        tree = ast.parse(source)
    except SyntaxError as e:
        raise CandidateError(f"syntax error: {e.msg} at line {e.lineno}") from None
    defined = []
    for top in tree.body:
        if isinstance(top, ast.Expr) and isinstance(top.value, ast.Constant):
            continue                                         # a module docstring
        if not isinstance(top, ast.FunctionDef):
            raise CandidateError(f"line {top.lineno}: only function definitions are allowed")
        params = [a.arg for a in top.args.args]
        if params != ["thread", "args"] or top.args.vararg or top.args.kwarg or \
                top.args.kwonlyargs or top.args.defaults or top.decorator_list or top.returns:
            raise CandidateError(f"{top.name}: the signature must be ({', '.join(['thread', 'args'])})")
        defined.append(top.name)
    if sorted(defined) != sorted(names):
        raise CandidateError(f"defines {sorted(defined)}; required {sorted(names)}")
    for top in tree.body:
        if isinstance(top, ast.FunctionDef):
            count = PARAMETER_COUNT[top.name]
            for node in ast.walk(top):
                if isinstance(node, ast.Subscript) and isinstance(node.slice, ast.Constant) and \
                        isinstance(node.slice.value, int) and node.slice.value >= count:
                    raise CandidateError(f"line {node.lineno}: {top.name} has {count} parameter"
                                         f"{'' if count == 1 else 's'}; args[{node.slice.value}] "
                                         "does not exist")
    for node in ast.walk(tree):
        if not isinstance(node, ALLOWED_NODES):
            raise CandidateError(f"line {getattr(node, 'lineno', '?')}: {type(node).__name__} "
                                 "is not allowed")
        if isinstance(node, ast.Name) and node.id.startswith("_"):
            raise CandidateError(f"line {node.lineno}: name {node.id} is not allowed")
        if isinstance(node, ast.Attribute):
            ok = isinstance(node.value, ast.Name) and node.value.id == "thread" and (
                node.attr in (WRITABLE if isinstance(node.ctx, ast.Store) else READABLE))
            if not ok:
                raise CandidateError(f"line {node.lineno}: attribute .{node.attr} is not allowed")
        if isinstance(node, ast.Subscript) and not (
                isinstance(node.value, ast.Name) and node.value.id == "args"
                and isinstance(node.slice, ast.Constant) and node.slice.value in (0, 1, 2, 3)):
            raise CandidateError(f"line {node.lineno}: only args[0] to args[3] may be indexed")
        if isinstance(node, ast.Constant) and not isinstance(node.value, (int, str, type(None))):
            raise CandidateError(f"line {node.lineno}: constant {node.value!r} is not allowed")
    namespace: dict[str, Any] = {"__builtins__": {}}
    exec(compile(tree, "<candidate>", "exec"), namespace)    # noqa: S102 — admitted code only
    return Candidate(source, sha256_of(source), {n: namespace[n] for n in names})


def install(kernel: Kernel32, cand: Candidate) -> None:
    """Replace the kernel32 handlers the candidate defines."""

    def bind(name: str, fn: CandidateFunction) -> Callable[[], int | None]:
        def handler() -> int | None:
            args = tuple(kernel.argument(i) for i in range(PARAMETER_COUNT[name]))
            ret = fn(_ThreadView(kernel.th), args)
            if ret is None:
                return None
            if isinstance(ret, bool) or not isinstance(ret, int):
                raise CandidateError(f"returned {type(ret).__name__}, not int or None")
            return ret & M64
        return handler

    for name, fn in cand.functions.items():
        kernel.handlers[name] = bind(name, fn)


def extract_code(reply: str) -> str:
    """The first fenced code block of a model reply, or the whole reply when it has none."""
    m = re.search(r"```(?:python|py)?[ \t]*\n(.*?)```", reply, re.S)
    return (m.group(1) if m else reply).strip() + "\n"


def prompt(contract: Contract, names: tuple[str, ...], feedback: str | None) -> str:
    """The generation prompt: the contract's summary and documented evidence, the interface,
    and on a repair attempt the verifier's report. The scenario's values are not in the prompt:
    contract 0.1.0's one fixed value (1234) let a constant pass; 0.2.0 picks values at run
    time."""
    quotes = "\n".join(f"- {e.source_ref}\n  \"{e.quote}\"" for e in contract.evidence
                       if e.category == "documented" and e.quote)
    text = f"""Write Python implementations of the Windows kernel32 functions {', '.join(names)}.

Contract {contract.contract_id} version {contract.version}: {contract.summary}
Applicability: {contract.applicability}

Documented behavior:
{quotes}

Interface (exact):
- Define exactly these functions, each as `def NAME(thread, args):`: {', '.join(names)}.
- `thread.last_error` is the calling thread's last-error code (an int); you may read and assign it.
- `thread.tid` is the calling thread's id (read only).
- `args` is a tuple of the function's declared parameters, in order: {'; '.join(f"{n}: {PARAMETER_COUNT[n]}" for n in names)}.
- Return an int (the function's return value) or None (for a function that returns no value).
- Allowed: assignments, arithmetic, comparisons, if/else, return. Not allowed: imports, function calls, loops, classes, other attributes, names starting with "_".

Reply with one ```python code block and nothing else."""
    if feedback:
        text += f"\n\nA previous candidate was rejected. Fix it. The verifier reported:\n{feedback}"
    return text
