"""measure_file_generation.py — can a model write file.c inside the Sprint 042 pipeline? (a measurement)

User decisions 2026-10-10: the sprint is the pipeline, not the model; the measurement uses a cloud
model, since local models cannot do it; and it runs in substrate's session topology (the model
harness loop: one ModelReply per model call with its stop reason and token counts, the exact
prompt in PromptComposed, tool calls and results on the record), not in a loop of our own. The
hand loop this replaces is at commit 55247f6, with its three result sets.

The session gets one turn. Its workspace holds what a person implementing file.c would get, and
not file.c itself: TASK.md, runtime.h, the probe that printed Windows's rows, the contract, and the
host file primitives. Tools: read_file, list_dir, grep, edit_file and write_file over that
workspace, and two of w2m's:
  build  compiles the workspace's file.c into the guest runtime; returns clang's errors or "built"
  judge  runs Windows's own probe binary in the managed VM over that file.c; returns the rows that
         differ from the contract (with what was printed and what Windows printed), or, when all
         188 are equal, runs scripts/file_run.py's item run on both runner binaries and returns its
         verdicts and record
The model decides when to build, judge, read and edit. The turn ends when the model answers
without a tool call (or the step budget forces it); the stepper then sends /exit.

Usage: uv run python scripts/measure_file_generation.py --model kimi-k2.7-code:cloud [--steps 80]
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import time
from collections import Counter
from collections.abc import AsyncIterator, Callable
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

import file_run  # noqa: E402
from substrate import api  # noqa: E402
from substrate.adapters import OllamaResponder  # noqa: E402
from substrate.topologies.session import UserMessage, session_topology  # noqa: E402
from substrate.topologies.session.vocabulary import (  # noqa: E402
    PRODUCER_KIND_DRIVER_STEPPER, SESSION_ENDED, TRIGGER_ID_DRIVER_STEPPER_ON_RETURNED, TURN_END_KINDS)
from substrate.topologies.tool_loop.tools import Tool, full_suite  # noqa: E402
from w2m import file_bridge, guest_runtime  # noqa: E402
from w2m.contract import ExactRows  # noqa: E402
from w2m.record import producer, run_with_provenance  # noqa: E402
from w2m.vm_backend import build_runtime  # noqa: E402

WORKSPACE_TOOLS = ("read_file", "list_dir", "grep", "edit_file", "write_file")
NO_ARGUMENTS: dict[str, Any] = {"type": "object", "properties": {}, "required": []}

TASK = """# Task: write file.c for w2m

w2m runs Windows x64 programs on macOS. It translates the program's x86-64 code and runs it inside
a virtual machine on Apple silicon. The program's kernel32 imports are served by a "guest
runtime": freestanding C compiled for AArch64 with clang {cflags}, linked into the VM. There is no
C library: no memcpy, no memset, no headers but "runtime.h" (read it: it defines u64, u32 and u8
and nothing else). Arguments arrive per the Microsoft x64 calling convention: the first four in
the C parameters, and `rsp` is the guest stack pointer at the call, so the fifth argument is at
*(u64 *)(rsp + 0x28), the sixth at rsp + 0x30, the seventh at rsp + 0x38. Guest pointers can be
dereferenced directly. Each function's return value goes to RAX.

Write `file.c` in this directory, defining exactly these functions:

  u64 k32_CreateFileW(u64 name, u64 desired_access, u64 share_mode, u64 security_attributes, u64 rsp);
      creation disposition at rsp+0x28 (u32), flags and attributes at rsp+0x30 (u32), template at rsp+0x38
  u64 k32_CloseHandle(u64 handle);
  u64 k32_DeleteFileW(u64 name);
  u64 k32_ReadFile(u64 handle, u64 buffer, u64 count, u64 bytes_read, u64 rsp);   overlapped at rsp+0x28
  u64 k32_SetFilePointerEx(u64 handle, u64 distance, u64 new_pointer, u64 move_method);
  u64 w2m_file_write(u64 handle, u64 buffer, u64 count, u64 written);
      WriteFile on a non-stdout handle; runtime.c has already set *(u32 *)written = 0 and checked
      that `written` is not NULL and lpOverlapped is NULL; return 1 on success, 0 on failure

The last-error code is w2m_teb()->last_error_value. Reach the Mac only through
w2m_host(op, a0, a1, a2, a3, &second) with the W2M_FILE_* primitives (host_file_primitives.md).
Return from each k32_ function through w2m_logged(W2M_API_<name>, arg0, return_value), which
records the call and returns its third argument. Call w2m_unimplemented(W2M_API_<name>, detail)
for an argument form you do not support; it stops the run. Handle values must not be 0x74
(standard output).

The judge runs create_file.c, compiled by Windows's clang, in w2m over your file.c. Its output
must equal the rows in contract.json exactly, row for row and field for field. Those rows are
what Windows Server 2025 and 2022 printed.

Files here: runtime.h, create_file.c (the probe), contract.json, host_file_primitives.md.
Tools: `build` compiles file.c and returns the compiler's errors or "built"; `judge` runs the probe
over file.c and returns the rows that differ, or the item run's verdict once every row is equal.
Work until `judge` reports the item run accepted, then answer with one line saying so.
"""


def workspace(where: Path) -> None:
    where.mkdir(parents=True)
    (where / "TASK.md").write_text(TASK.format(cflags=" ".join(guest_runtime.CFLAGS)))
    shutil.copy(guest_runtime.SOURCE / "runtime.h", where / "runtime.h")
    shutil.copy(ROOT / "probes" / "create_file" / "create_file.c", where / "create_file.c")
    shutil.copy(file_run.CONTRACT, where / "contract.json")
    (where / "host_file_primitives.md").write_text(file_bridge.__doc__ or "")


def w2m_tools(where: Path, runs: Path, model: str, session_id: str,
              judged: list[dict[str, Any]]) -> dict[str, Tool]:
    def source() -> str:
        path = where / "file.c"
        if not path.exists():
            raise FileNotFoundError("file.c does not exist yet; write it with write_file")
        return path.read_text()

    def build(_args: list[Any]) -> str:
        try:
            build_runtime(None, source())
        except guest_runtime.BuildError as e:
            return str(e)[:4000]
        return "built"

    def judge(_args: list[Any]) -> dict[str, Any]:
        text = source()
        try:
            build_runtime(None, text)
        except guest_runtime.BuildError as e:
            judged.append({"outcome": "build_error"})
            return {"outcome": "build_error", "compiler": str(e)[:4000]}
        c = file_run.contract()
        assert isinstance(c.acceptance, ExactRows)
        out, stop, _ = file_run.run_in_vm(file_run.RUNS["38031352273"][1], text)
        unequal = file_run.unequal_rows(c, out)
        if unequal or stop != "ExitProcess":
            printed = {}
            for line in out.splitlines():
                try:
                    printed[json.loads(line).get("s")] = line.decode()
                except (json.JSONDecodeError, UnicodeDecodeError):
                    continue
            want = {r["s"]: json.dumps(r, separators=(",", ":")) for r in c.acceptance.rows}
            judged.append({"outcome": "discrepancy", "unequal_rows": len(unequal), "stop": stop})
            return {"outcome": "discrepancy", "run_stop": stop, "unequal_rows": f"{len(unequal)} of 188",
                    "first_unequal": [{"windows": want[r], "yours": printed.get(r, "(not printed)")}
                                      for r in unequal[:10]]}
        s = file_run.item_run(runs, text, generator_ref=f"ollama:{model} in session {session_id}")
        accepted = s["events"].get("CANDIDATE_ACCEPTED", 0) == 1
        record = str(Path(s["record"]).relative_to(ROOT))
        judged.append({"outcome": "accepted" if accepted else "item_run_discrepancy", "record": record,
                       "candidate_sha256": s["candidate_sha256"]})
        return {"outcome": "item run accepted" if accepted else "item run discrepancy",
                "verdicts": s["verdicts"], "record": record}

    return {"build": Tool("build", "build() -> compiles file.c into the guest runtime: 'built' or the "
                          "compiler's errors", False, build, NO_ARGUMENTS),
            "judge": Tool("judge", "judge() -> runs the probe over file.c in w2m: the rows that differ "
                          "from Windows's, or the item run's verdict once all 188 are equal", False, judge,
                          NO_ARGUMENTS)}


def measurement_topology(model: str, steps: int, where: Path, runs: Path, session_id: str,
                         judged: list[dict[str, Any]]) -> Callable[[api.TopologyBuilder], None]:
    return session(model, steps, where, session_id, w2m_tools(where, runs, model, session_id, judged))


def session(model: str, steps: int, where: Path, session_id: str,
            w2m: dict[str, Tool]) -> Callable[[api.TopologyBuilder], None]:
    """Substrate's session topology: one turn, the workspace tools and `w2m`'s, then /exit
    (shared with scripts/model_family.py)."""
    tools = {k: v for k, v in full_suite(where, owner=session_id).items() if k in WORKSPACE_TOOLS}
    tools |= w2m
    turns = ("Read TASK.md and do the task it describes.", "/exit")
    base = session_topology(
        driver=OllamaResponder(model, temperature=0.2, num_ctx=131072, timeout=1200.0),
        driver_name=f"ollama:{model}", driver_context_tokens=131072,
        seed="You are a C programmer working in a directory with tools. Use the tools; do not ask questions.",
        tools=tools, max_turns=1, turn_max_steps=steps, session_id=session_id, workspace_path=str(where))

    async def stepper(inp: Any) -> AsyncIterator[UserMessage]:
        i = int(inp.get("turn_index", 0))
        if i < len(turns):
            yield UserMessage(text=turns[i], turn_index=i, assembled_prompt="", slash_source="w2m")

    def topo(b: api.TopologyBuilder) -> None:
        base(b)
        b.producer_kind(PRODUCER_KIND_DRIVER_STEPPER, schemas=[UserMessage], schema_version=1,
                        factory=lambda: producer(stepper), deterministic=True)
        b.initial(PRODUCER_KIND_DRIVER_STEPPER, input={"turn_index": 0})
        b.trigger(TRIGGER_ID_DRIVER_STEPPER_ON_RETURNED, subscription=api.Subscription(kinds=TURN_END_KINDS),
                  predicate=lambda ctx: int(ctx.event.payload.get("turn_index", 0)) + 1 < len(turns),
                  starts=PRODUCER_KIND_DRIVER_STEPPER,
                  input_builder=lambda ctx: {"turn_index": int(ctx.event.payload.get("turn_index", 0)) + 1},
                  policy=api.PerEvent())
        b.termination(api.threshold_count(SESSION_ENDED, 1))

    return topo


def summarize(record: Path, model: str, where: Path) -> dict[str, Any]:
    """Everything from the session record (blobs resolved: a long reply or result is a blob)."""
    evs = list(api.read_record(record, resolve_blobs=True))
    kinds = Counter(e["kind"] for e in evs)
    replies = [e["payload"] for e in evs if e["kind"] == "ModelReply"]
    tool_of = {e["payload"]["call_id"]: e["payload"]["tool"] for e in evs if e["kind"] == "ToolCall"}
    judgments = []
    for e in evs:
        p = e["payload"]
        if e["kind"] == "ToolResult" and tool_of.get(p.get("call_id")) == "judge":
            out = p.get("output") if p.get("ok") else {"outcome": "judge raised", "error": p.get("error")}
            judgments.append({"step": p.get("step"), "outcome": out.get("outcome"),
                              "unequal_rows": out.get("unequal_rows"), "record": out.get("record"),
                              "error": out.get("error")})
    return {"model": f"ollama:{model}", "session_record": str(record.relative_to(ROOT)),
            "outcome": judgments[-1]["outcome"] if judgments else "never judged",
            "model_calls": len(replies), "stop_reasons": dict(Counter(r["stop_reason"] for r in replies)),
            "prompt_tokens": sum(r["usage"].get("prompt_tokens", 0) for r in replies),
            "completion_tokens": sum(r["usage"].get("completion_tokens", 0) for r in replies),
            "tool_calls": dict(Counter(tool_of.values())), "judgments": judgments,
            "final_file_c": str((where / "file.c").relative_to(ROOT)) if (where / "file.c").exists() else None,
            "events": dict(kinds)}


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--steps", type=int, default=80)
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    ap.add_argument("--summarize", type=Path, help="write the summary of an existing session record")
    a = ap.parse_args(argv)
    if a.summarize:
        session_id = a.summarize.name
        summary = summarize(a.summarize.resolve(), a.model, a.runs / "file-generation" / session_id)
        (a.runs / f"{session_id}.json").write_text(json.dumps(summary, indent=1) + "\n")
        print(json.dumps({k: v for k, v in summary.items() if k != "judgments"}, indent=1))
        return 0
    name = re.sub(r"[^a-z0-9.-]+", "-", a.model.lower())
    session_id = f"file-generation-session-{name}-{time.time_ns()}"
    where = a.runs / "file-generation" / session_id
    workspace(where)
    judged: list[dict[str, Any]] = []
    result = run_with_provenance(a.runs / session_id, measurement_topology(a.model, a.steps, where, a.runs, session_id, judged))
    summary = summarize(Path(result.record_root), a.model, where)
    (a.runs / f"{session_id}.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k != "judgments"}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
