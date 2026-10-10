"""measure_file_generation.py — can a model write file.c inside the Sprint 042 pipeline? (a measurement)

User decisions 2026-10-10: the sprint is the pipeline, not the model; the measurement uses a cloud
model, since local models cannot do it. Nothing depends on the outcome.

The model gets what a person implementing file.c would get, and not file.c itself: the contract
(Windows's 188 rows and its evidence), the probe that printed them, runtime.h, the host file
primitives (src/w2m/file_bridge.py's docstring) and the functions to define. Each attempt's
file.c goes through scripts/file_run.py's item run, on its own record, judged by the same rows in
the managed VM. A build error, a stopped run or the unequal rows go back to the model for the next
attempt. The summary (model, prompt sha256, each attempt's outcome and token counts) lands in
store/runs/file-generation-<model>.json.

Usage: uv run python scripts/measure_file_generation.py --model kimi-k2.7-code:cloud [--attempts 4]
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import re
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

import file_run  # noqa: E402
from w2m import file_bridge, guest_runtime  # noqa: E402
from w2m.candidate import extract_code  # noqa: E402
from w2m.vm_backend import MONITOR_VA, RUNTIME_IMAGE, runtime_defines  # noqa: E402

TASK = """Write tools/guest_runtime/file.c for w2m, which runs Windows x64 programs on macOS.

w2m translates the program's x86-64 code and runs it inside a virtual machine on Apple silicon. The
program's kernel32 imports are served by a "guest runtime": freestanding C compiled for AArch64 with
clang {cflags}, linked into the VM. There is no C library: no memcpy, no memset, no headers but
"runtime.h". The program's arguments arrive per the Microsoft x64 calling convention: the first four
in the C parameters, and `rsp` is the guest stack pointer at the call, so the fifth argument is at
*(u64 *)(rsp + 0x28), the sixth at rsp + 0x30, the seventh at rsp + 0x38. Guest pointers can be
dereferenced directly. The return value of each function goes to RAX.

Define exactly these functions:

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
w2m_host(op, a0, a1, a2, a3, &second) with the W2M_FILE_* primitives below. Call
w2m_logged(W2M_API_<name>, arg0, return_value) to return from each k32_ function (it records the call
and returns its third argument). Call w2m_unimplemented(W2M_API_<name>, detail) for any argument
form you do not support (it stops the run). Handle values must not be 0x74 (standard output).

Your implementation is judged by running the probe below, compiled by Windows's clang, in w2m: its
output must equal the contract's rows exactly, row for row and field for field. Those rows are what
Windows Server 2025 and 2022 printed.

Reply with the whole file.c in one ```c fenced block.

=== runtime.h ===
{runtime_h}

=== host file primitives (src/w2m/file_bridge.py) ===
{bridge}

=== the probe (probes/create_file/create_file.c) ===
{probe}

=== the contract (contracts/{contract_id}/{version}.json) ===
{contract}
"""


def base_prompt() -> str:
    doc = file_bridge.__doc__ or ""
    return TASK.format(
        cflags=" ".join(guest_runtime.CFLAGS), runtime_h=(guest_runtime.SOURCE / "runtime.h").read_text(),
        bridge=doc, probe=(ROOT / "probes" / "create_file" / "create_file.c").read_text(),
        contract_id=file_run.CONTRACT_ID, version=file_run.VERSION, contract=file_run.CONTRACT.read_text())


def feedback(source: str, outcome: dict[str, Any]) -> str:
    """The previous attempt's own file.c and what the judge said about it."""
    mine = f"Your previous file.c:\n```c\n{source}\n```\n"
    if outcome["outcome"] == "build_error":
        return mine + f"It did not compile:\n{outcome['detail']}\n"
    rows = outcome.get("unequal_rows", [])
    return mine + (f"It ran in w2m and was judged: {outcome['detail']}. "
            f"{len(rows)} of the contract's rows were not reproduced exactly; the first of them: {rows[:12]}. "
            "Here is what it printed for those rows:\n" + "\n".join(outcome.get("printed", [])[:12]) + "\n")


def longest_c_block(reply: str) -> str:
    """The longest fenced block of the reply: a reply that explains with snippets before the
    whole file must be judged on the file (the first-block rule cut two attempts short)."""
    blocks = re.findall(r"```[a-zA-Z]*\n(.*?)```", reply, re.S)
    return max(blocks, key=len) if blocks else extract_code(reply)


def attempt(source: str, runs: Path, model: str, k: int) -> dict[str, Any]:
    try:
        guest_runtime.build(MONITOR_VA + RUNTIME_IMAGE, runtime_defines(), None, source)
    except guest_runtime.BuildError as e:
        return {"outcome": "build_error", "detail": str(e)[:3000]}
    c = file_run.contract()
    out, stop, seconds = file_run.run_in_vm(file_run.RUNS["38031352273"][1], source)
    unequal = file_run.unequal_rows(c, out)
    printed = {}
    for line in out.splitlines():
        try:
            printed[json.loads(line).get("s")] = line.decode()
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
    s = file_run.item_run(runs, source, generator_ref=f"ollama:{model} attempt {k}")
    accepted = s["events"].get("CANDIDATE_ACCEPTED", 0) == 1
    return {"outcome": "accepted" if accepted else "discrepancy",
            "detail": "conforms on both runner binaries" if accepted else
                      f"run stop {stop!r}; first divergence {next(iter(s['divergence'].values()))}",
            "unequal_rows": unequal, "printed": [printed.get(r, f"(no row {r})") for r in unequal[:12]],
            "record": str(Path(s["record"]).relative_to(ROOT)), "vm_seconds": round(seconds, 3)}


async def measure(model: str, attempts: int, runs: Path) -> dict[str, Any]:
    from substrate.adapters import OllamaResponder, call_responder_metered
    responder = OllamaResponder(model, temperature=0.2, num_ctx=131072, timeout=1200.0)
    prompt0 = base_prompt()
    out: dict[str, Any] = {"model": f"ollama:{model}", "prompt_sha256": "sha256:" + hashlib.sha256(prompt0.encode()).hexdigest(),
                           "prompt_chars": len(prompt0), "attempts": []}
    store = runs / "file-generation" / re.sub(r"[^a-z0-9.-]+", "-", model.lower())
    store.mkdir(parents=True, exist_ok=True)
    prompt = prompt0
    for k in range(1, attempts + 1):
        t0 = time.monotonic()
        reply, usage = await call_responder_metered(responder, prompt)
        (store / f"attempt-{k}.reply.md").write_text(reply)
        source = longest_c_block(reply)
        (store / f"attempt-{k}.c").write_text(source)
        result = await asyncio.to_thread(attempt, source, runs, model, k)   # item_run runs its own event loop
        result.update({"attempt": k, "source_sha256": "sha256:" + hashlib.sha256(source.encode()).hexdigest(),
                       "source": str((store / f"attempt-{k}.c").relative_to(ROOT)),
                       "reply": str((store / f"attempt-{k}.reply.md").relative_to(ROOT)),
                       "prompt_tokens": usage.prompt_tokens, "completion_tokens": usage.completion_tokens,
                       "model_wall_ms": usage.wall_ms, "tokens_estimated": usage.estimated,
                       "seconds": round(time.monotonic() - t0, 1)})
        out["attempts"].append({k2: v for k2, v in result.items() if k2 != "printed"} |
                               {"unequal_rows": result.get("unequal_rows", [])[:40],
                                "unequal_row_count": len(result.get("unequal_rows", []))})
        print(f"attempt {k}: {result['outcome']}; {result['detail'][:200]}; "
              f"{len(result.get('unequal_rows', []))} rows unequal; {result['seconds']} s", flush=True)
        if result["outcome"] == "accepted":
            break
        prompt = prompt0 + "\n\n" + feedback(source, result)
    out["outcome"] = out["attempts"][-1]["outcome"]
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--attempts", type=int, default=4)
    ap.add_argument("--runs", type=Path, default=ROOT / "store" / "runs")
    a = ap.parse_args(argv)
    summary = asyncio.run(measure(a.model, a.attempts, a.runs))
    name = re.sub(r"[^a-z0-9.-]+", "-", a.model.lower())
    (a.runs / f"file-generation-{name}.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k != "attempts"}))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
