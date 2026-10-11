# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""candidate.py — admit, prompt for, and install a generated last-error implementation (Sprint 031;
C since Sprint 035).

A candidate is C source for the contract's surface as the guest runtime declares it
(tools/guest_runtime/runtime.h):

    void w2m_set_last_error(u32 code);
    u32 w2m_get_last_error(void);

`w2m_teb()` returns the calling thread's TEB; its `last_error_value` (TEB+0x68, where Windows
keeps it; Sprint 034) may be read and written and `client_thread` read. The declared signatures
carry the parameter rule Sprint 032 enforced by hand for Python: GetLastError takes no argument,
so it cannot read one.

`admit` refuses, before anything compiles: preprocessor lines, assembly, loops, goto, string and
character literals, pointer syntax other than `w2m_teb()->`, division and remainder (an x86 host
traps on division by zero, AArch64 does not), and every call except `w2m_teb()`. Then the source
must compile with `-Wall -Werror` against runtime.h. Admitted code runs in two places:

  managed_vm — compiled into the guest runtime image (src/w2m/guest_runtime.py) and run in the VM;
  model      — compiled for the host as a shared library whose TEB is a host buffer, and called
               from kernel32.py's SetLastError/GetLastError through ctypes.

The check limits what a local code model's output can do inside the harness. It is not a
security boundary against an adversary who controls the generator.
"""

from __future__ import annotations

import ctypes
import hashlib
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

from w2m.contract import Contract
from w2m.kernel32 import Kernel32

ROOT = Path(__file__).resolve().parents[2]
RUNTIME_SOURCE = ROOT / "tools" / "guest_runtime"
HOST_CACHE = ROOT / "build" / "candidates"
SIGNATURES = {"SetLastError": "void w2m_set_last_error(u32 code)", "GetLastError": "u32 w2m_get_last_error(void)"}
FIELDS = {"last_error_value": "read and write", "client_thread": "read"}
KEYWORDS = {"void", "u32", "u64", "unsigned", "int", "long", "const", "return", "if", "else"}
REFUSED_WORDS = {"asm", "__asm", "__asm__", "goto", "while", "for", "do", "switch", "extern",
                 "volatile", "register", "static", "sizeof", "typedef", "struct", "union", "enum"}
TEB_OFFSETS = {"last_error_value": 0x68, "client_thread": 0x48}


class CandidateError(ValueError):
    """The candidate source is outside the admitted language or interface, or does not compile."""


@dataclass(frozen=True)
class Candidate:
    source: str
    sha256: str                       # "sha256:<hex>" of the source bytes


def sha256_of(source: str) -> str:
    return "sha256:" + hashlib.sha256(source.encode()).hexdigest()


def _strip_comments(src: str) -> str:
    return re.sub(r"//[^\n]*|/\*.*?\*/", " ", src, flags=re.S)


def admit(source: str, names: tuple[str, ...]) -> Candidate:
    """Admit `source` when it defines exactly the functions for `names` with their declared
    signatures, uses only the admitted C, and compiles against runtime.h."""
    code = _strip_comments(source)
    for line_no, line in enumerate(code.splitlines(), 1):
        if line.lstrip().startswith("#"):
            raise CandidateError(f"line {line_no}: preprocessor lines are not allowed")
    if re.search(r"[\"'\[\]]", code):
        raise CandidateError("string or character literals and subscripts are not allowed")
    if re.search(r"[/%]", code):
        raise CandidateError("division and remainder are not allowed")
    words = set(re.findall(r"[A-Za-z_]\w*", code))
    refused = sorted(words & REFUSED_WORDS)
    if refused:
        raise CandidateError(f"{refused[0]} is not allowed")
    if re.search(r"->", re.sub(r"w2m_teb\(\)\s*->\s*(" + "|".join(FIELDS) + r")\b", "", code)):
        raise CandidateError(f"only w2m_teb()->{{{', '.join(FIELDS)}}} may be dereferenced")
    ops = code.replace("&&", " and ").replace("&=", " andeq ").replace("*=", " muleq ")
    before = r"(?:^|[=(,;{}!~+\-*&|^<>?:]|\breturn)\s*"          # where a unary operator stands
    cast = r"\(\s*(?:const\s+)?(?:void|u32|u64|unsigned|int|long)[\s\w]*\)\s*"   # (type) before a unary op
    if re.search(before + r"[*&]", ops, re.M) or re.search(cast + r"[*&]", ops) or re.search(r"\*\s*\)", ops):
        raise CandidateError("pointer syntax is not allowed")
    wanted = {SIGNATURES[n].split("(")[0].split()[-1]: SIGNATURES[n] for n in names}
    defined = re.findall(r"\b(?:void|u32|u64|unsigned|int)\s+(\w+)\s*\(([^)]*)\)\s*\{", code)
    if sorted(n for n, _ in defined) != sorted(wanted):
        raise CandidateError(f"defines {sorted(n for n, _ in defined)}; required {sorted(wanted)}")
    for name, params in defined:
        want = wanted[name].split("(", 1)[1].rstrip(")")
        if " ".join(params.split()) != want:
            raise CandidateError(f"{name}: the signature must be {wanted[name]}")
    for callee in re.findall(r"\b([A-Za-z_]\w*)\s*\(", code):
        if callee not in ("w2m_teb", "if", "return") and callee not in wanted:
            raise CandidateError(f"call to {callee} is not allowed")
    for name in wanted:
        if re.search(rf"\b{name}\s*\(", re.sub(rf"\b(?:void|u32)\s+{name}\s*\(", "", code)):
            raise CandidateError(f"{name} calls itself")
    _host_library(source)                                      # compiles with -Wall -Werror
    return Candidate(source, sha256_of(source))


PRELUDE = "extern unsigned char w2m_host_teb[];\n"
HOST_SHIM = "unsigned char w2m_host_teb[0x1800] __attribute__((aligned(16)));\n"


def _host_library(source: str) -> Path:
    """Compile the candidate for this host as a shared library; its TEB is `w2m_host_teb`."""
    from w2m.guest_runtime import api_header
    header = (RUNTIME_SOURCE / "runtime.h").read_text()
    ids = api_header()                              # runtime.h includes the generated ids (Sprint 049)
    key = hashlib.sha256("\0".join([source, header, ids, PRELUDE, HOST_SHIM]).encode()).hexdigest()
    lib = HOST_CACHE / f"{key}.so"
    if lib.exists():
        return lib
    HOST_CACHE.mkdir(parents=True, exist_ok=True)
    work = HOST_CACHE / key
    work.mkdir(exist_ok=True)
    (work / "prelude.h").write_text(PRELUDE)
    (work / "api_ids.h").write_text(ids)
    (work / "shim.c").write_text(HOST_SHIM)
    (work / "candidate.c").write_text('#include "runtime.h"\n' + source)
    r = subprocess.run(["clang", "-std=c11", "-O1", "-Wall", "-Werror", "-shared", "-fPIC",
                        "-include", str(work / "prelude.h"), f"-I{RUNTIME_SOURCE}", f"-I{work}",
                        "-DW2M_TEB_VA=((u64)w2m_host_teb)", "-o", str(lib),
                        str(work / "candidate.c"), str(work / "shim.c")],
                       capture_output=True, text=True)
    if r.returncode:
        msg = re.sub(r"\S*/candidate\.c:(\d+)", lambda m: f"line {int(m.group(1)) - 1}", r.stderr)
        raise CandidateError(f"does not compile: {msg.strip()[:600]}")
    return lib


def install(kernel: Kernel32, cand: Candidate) -> None:
    """Route the model runtime's SetLastError/GetLastError through the candidate (host build).
    The thread's last-error value and id are copied into the host TEB before each call and the
    value back after it."""
    lib = ctypes.CDLL(str(_host_library(cand.source)))
    teb = ctypes.addressof(ctypes.c_ubyte.in_dll(lib, "w2m_host_teb"))
    last_error = ctypes.c_uint32.from_address(teb + TEB_OFFSETS["last_error_value"])
    thread_id = ctypes.c_uint64.from_address(teb + TEB_OFFSETS["client_thread"])
    lib.w2m_set_last_error.argtypes, lib.w2m_set_last_error.restype = [ctypes.c_uint32], None
    lib.w2m_get_last_error.argtypes, lib.w2m_get_last_error.restype = [], ctypes.c_uint32

    def enter() -> None:
        last_error.value = kernel.th.last_error & 0xFFFFFFFF
        thread_id.value = kernel.th.tid

    def set_last_error() -> None:
        enter()
        lib.w2m_set_last_error(kernel.argument(0) & 0xFFFFFFFF)
        kernel.th.last_error = last_error.value

    def get_last_error() -> int:
        enter()
        value = int(lib.w2m_get_last_error())
        kernel.th.last_error = last_error.value
        return value

    kernel.handlers["SetLastError"] = set_last_error
    kernel.handlers["GetLastError"] = get_last_error


def extract_code(reply: str) -> str:
    """The first fenced code block of a model reply, or the whole reply when it has none."""
    m = re.search(r"```(?:c|C)?[ \t]*\n(.*?)```", reply, re.S)
    return (m.group(1) if m else reply).strip() + "\n"


def prompt(contract: Contract, names: tuple[str, ...], feedback: str | None) -> str:
    """The generation prompt: the contract's summary and documented evidence, the interface,
    and on a repair attempt the verifier's report. The scenario's values are not in the prompt:
    contract 0.1.0's one fixed value (1234) let a constant pass; 0.2.0 picks values at run
    time."""
    quotes = "\n".join(f"- {e.source_ref}\n  \"{e.quote}\"" for e in contract.evidence
                       if e.category == "documented" and e.quote)
    text = f"""Write C implementations of the Windows kernel32 functions {', '.join(names)}.

Contract {contract.contract_id} version {contract.version}: {contract.summary}
Applicability: {contract.applicability}

Documented behavior:
{quotes}

Interface (exact):
- Define exactly these functions with these signatures: {'; '.join(SIGNATURES[n] for n in names)}.
- `w2m_teb()` returns the calling thread's environment block. `w2m_teb()->last_error_value` (u32) is the calling thread's last-error code; you may read and assign it. `w2m_teb()->client_thread` is the calling thread's id (read only).
- `u32` is a 32-bit unsigned integer. Do not include headers; they are provided.
- Allowed: local variables, arithmetic other than division and remainder, comparisons, if/else, return. Not allowed: preprocessor lines, loops, goto, pointers, arrays, string literals, assembly, calls to any function other than w2m_teb().

Reply with one ```c code block and nothing else."""
    if feedback:
        text += f"\n\nA previous candidate was rejected. Fix it. The verifier reported:\n{feedback}"
    return text
