"""model_family.py — a cloud model writes a family's guest C file in substrate's session topology
(Sprint 044, review 2026-10-10 D7).

scripts/measure_file_generation.py's measurement (Sprint 042: kimi-k2.7-code wrote file.c, accepted,
46 model calls) applied to a family of scripts/family_run.py. The session gets one turn and a workspace
with what a person implementing the family would get, and not the agent's guest file: TASK.md,
runtime.h, the probe that printed Windows's rows, and each family's contract. Tools: read_file, list_dir,
grep, edit_file and write_file over the workspace, and two of w2m's:
  build  compiles the workspace's guest file into the guest runtime: clang's errors or "built"
  judge  runs each family's first runner binary in the managed VM over that file: the rows that differ
         from the contract; when every family's rows are equal, each family's item run (every runner
         binary, on its own record, generator_ref naming the model and session), and their verdicts
The model decides when to build, judge, read and edit; the turn ends when it answers without a tool
call or the step budget ends it.

Usage: uv run python scripts/model_family.py environment --model kimi-k2.7-code:cloud [--steps 80]
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

import family_run  # noqa: E402
import measure_file_generation as measure  # noqa: E402
from substrate import api  # noqa: E402
from substrate.topologies.tool_loop.tools import Tool  # noqa: E402
from w2m import guest_runtime  # noqa: E402
from w2m.contract import ExactRows  # noqa: E402
from w2m.record import run_with_provenance  # noqa: E402
from w2m.vm_backend import build_runtime  # noqa: E402


@dataclass(frozen=True)
class Task:
    families: tuple[str, ...]     # judged together: they share the guest file
    guest_file: str
    functions: str                # the signatures and what the runtime offers, for TASK.md


TASKS = {
    "environment": Task(
        ("environment_set", "environment_empty"), "environment.c",
        """  u64 k32_GetEnvironmentStringsW(void);
  u64 k32_FreeEnvironmentStringsW(u64 block);
  u64 k32_lstrlenW(u64 string);

The process's environment block is the host's: UTF-16 entries, each ending in a NUL, then one more NUL,
at the address W2M_ENVIRONMENT_VA (an empty block is a single NUL there). runtime.h declares
  u64 w2m_heap_take(u64 size);          a process-heap block of exactly `size` bytes (HeapSize reports it),
                                        or 0 when the arena is full
  void w2m_heap_release(u64 api, u64 p); gives a block back
Probes see the process heap through GetProcessHeap and HeapSize, which use the same blocks."""),
}

TASK_TEXT = """# Task: write {guest_file} for w2m

w2m runs Windows x64 programs on macOS. It translates the program's x86-64 code and runs it inside
a virtual machine on Apple silicon. The program's kernel32 imports are served by a "guest
runtime": freestanding C compiled for AArch64 with clang {cflags}, linked into the VM. There is no
C library: no memcpy, no memset, no headers but "runtime.h" (read it). Arguments arrive per the
Microsoft x64 calling convention: the first four in the C parameters, and a fifth parameter `rsp`,
when declared, is the guest stack pointer at the call (the fifth argument at *(u64 *)(rsp + 0x28)).
Guest pointers can be dereferenced directly. Each function's return value goes to RAX.

Write `{guest_file}` in this directory, defining exactly these functions:

{functions}

The last-error code is w2m_teb()->last_error_value. Return from each k32_ function through
w2m_logged(W2M_API_<name>, arg0, return_value), which records the call and returns its third
argument. Call w2m_unimplemented(W2M_API_<name>, detail) for an argument form you do not support;
it stops the run.

The judge runs the probe (probe_{probe}.c, compiled by Windows's clang) in w2m over your file, once per
contract: {contracts}. Each contract's rows are what Windows printed on the runners; the probe's
output in w2m must equal them exactly, row for row and field for field. Read the probe to see how
each contract's process is started (its command line and environment block).

Tools: `build` compiles {guest_file} and returns the compiler's errors or "built"; `judge` runs the
probe over it and returns the rows that differ, or the item runs' verdicts once every row is equal.
Work until `judge` reports every item run accepted, then answer with one line saying so.
"""


def workspace(where: Path, task: Task) -> None:
    fams = [family_run.FAMILIES[n] for n in task.families]
    probe = fams[0].probe
    where.mkdir(parents=True)
    contracts = []
    for f in fams:
        name = f"contract-{f.name}.json"
        shutil.copy(f.contract_path, where / name)
        contracts.append(f"{name} (environment block {list(f.environment)}, command line ending {f.arguments!r})")
    (where / "TASK.md").write_text(TASK_TEXT.format(
        guest_file=task.guest_file, cflags=" ".join(guest_runtime.CFLAGS), functions=task.functions,
        probe=probe, contracts="; ".join(contracts)))
    shutil.copy(guest_runtime.SOURCE / "runtime.h", where / "runtime.h")
    shutil.copy(ROOT / "probes" / probe / f"{probe}.c", where / f"probe_{probe}.c")


def w2m_tools(where: Path, task: Task, runs: Path, model: str, session_id: str) -> dict[str, Tool]:
    fams = [family_run.FAMILIES[n] for n in task.families]

    def source() -> str:
        path = where / task.guest_file
        if not path.exists():
            raise FileNotFoundError(f"{task.guest_file} does not exist yet; write it with write_file")
        return path.read_text()

    def build(_args: list[Any]) -> str:
        try:
            build_runtime(sources={task.guest_file: source()})
        except guest_runtime.BuildError as e:
            return str(e)[:4000]
        return "built"

    def judge(_args: list[Any]) -> dict[str, Any]:
        text = source()
        sources = {task.guest_file: text}
        try:
            build_runtime(sources=sources)
        except guest_runtime.BuildError as e:
            return {"outcome": "build_error", "compiler": str(e)[:4000]}
        differences = {}
        for f in fams:
            c = family_run.contract(f)
            assert isinstance(c.acceptance, ExactRows)
            out, stop, _ = family_run.run_in_vm(next(iter(f.runs.values())).binary, sources, f)
            unequal = family_run.unequal_rows(f, out)
            if unequal or stop != "ExitProcess":
                printed = {r.get("s"): r for r in f.rows(family_run.parse(out))}
                want = {r["s"]: r for r in c.acceptance.rows}
                differences[f.contract_path.parent.name] = {
                    "run_stop": stop, "unequal_rows": f"{len(unequal)} of {len(want)}",
                    "first_unequal": [{"windows": want[r], "yours": printed.get(r, "(not printed)")} for r in unequal[:10]]}
        if differences:
            return {"outcome": "discrepancy", "contracts": differences}
        verdicts = {}
        for f in fams:
            s = family_run.item_run(f, runs, sources, generator_ref=f"ollama:{model} in session {session_id}")
            verdicts[f.name] = {"accepted": s["events"].get("CANDIDATE_ACCEPTED", 0) == 1,
                                "record": str(Path(s["record"])), "verdicts": s["verdicts"]}
        every = all(v["accepted"] for v in verdicts.values())
        return {"outcome": "item runs accepted" if every else "item run discrepancy", "item_runs": verdicts}

    no_args = measure.NO_ARGUMENTS
    return {"build": Tool("build", f"build() -> compiles {task.guest_file} into the guest runtime: 'built' or the "
                          "compiler's errors", False, build, no_args),
            "judge": Tool("judge", f"judge() -> runs the probe over {task.guest_file} in w2m: the rows that differ "
                          "from Windows's, or the item runs' verdicts once all are equal", False, judge, no_args)}


def item_run_records(record: Path) -> dict[str, str]:
    """The family item runs the judge started, from its results on the session record."""
    out: dict[str, str] = {}
    for e in api.read_record(record, resolve_blobs=True):
        p = e["payload"]
        if e["kind"] == "ToolResult" and p.get("ok") and isinstance(p.get("output"), dict):
            for name, v in (p["output"].get("item_runs") or {}).items():
                out[name] = str(v["record"])           # family_run.item_run gives it relative to the repository
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("task", choices=sorted(TASKS))
    ap.add_argument("--model", required=True)
    ap.add_argument("--steps", type=int, default=80)
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    a = ap.parse_args(argv)
    task = TASKS[a.task]
    name = re.sub(r"[^a-z0-9.-]+", "-", a.model.lower())
    session_id = f"model-family-{a.task}-{name}-{time.time_ns()}"
    where = a.runs / "model-families" / session_id
    workspace(where, task)
    tools = w2m_tools(where, task, a.runs, a.model, session_id)
    result = run_with_provenance(a.runs / session_id, measure.session(a.model, a.steps, where, session_id, tools))
    summary = measure.summarize(Path(result.record_root), a.model, where)
    summary["final_file"] = str((where / task.guest_file).relative_to(ROOT)) if (where / task.guest_file).exists() else None
    summary["item_run_records"] = item_run_records(Path(result.record_root))
    (a.runs / f"{session_id}.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k != "judgments"}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
