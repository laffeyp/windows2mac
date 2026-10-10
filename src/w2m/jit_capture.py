# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""jit_capture.py — read the capture file tools/jit_capture writes (Sprint 033).

Format 2 (header "W2MJ", u32 2) holds CPUID records, compiled methods (jit_done) with their
assembly image, and code buffers (jit_code_buffer: trampolines and stubs). Format 1 (no header;
the first runner capture, run 37979176570) holds methods only: u64 code_start, u32 code_size,
u32 name_len, name, code bytes. All integers little-endian.

Mono reports a compiled method twice: as a code buffer of type METHOD (no image name) and through
jit_done (with its image); jit_done can also fire again for some wrappers (mini.c). `regions()`
keeps one region per code_start, preferring the jit_done record.

A specific trampoline is `call <generic trampoline>` followed by a length byte (4 or 8) and the
argument: "The trampoline code will obtain the argument from the instruction stream"
(Unity-Technologies/mono unity-2019.4-mbe, mono/mini/tramp-amd64.c,
mono_arch_create_specific_trampoline). Its code ends at that call: `Region.code_ends_at_first_call`.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

MAGIC = b"W2MJ"
KIND_CPUID, KIND_METHOD, KIND_BUFFER = 0, 1, 2
BUFFER_TYPES = ("method_buffer", "method_trampoline", "unbox_trampoline", "imt_trampoline",
                "generics_trampoline", "specific_trampoline", "helper", "monitor",
                "delegate_invoke", "exception_handling")      # MonoProfilerCodeBufferType, profiler.h


class CaptureError(ValueError):
    """The file is not a capture this module can read."""


@dataclass(frozen=True)
class Region:
    """One code region Mono generated: a compiled method or a code buffer."""

    start: int
    code: bytes
    name: str            # method full name, or the buffer's description
    image: str           # assembly image name for methods; "" when unknown
    kind: str            # "method" (jit_done) or a BUFFER_TYPES entry

    @property
    def code_ends_at_first_call(self) -> bool:
        return self.kind == "specific_trampoline"


@dataclass
class Capture:
    format: int
    cpuid: dict[tuple[int, int], tuple[int, int, int, int]] = field(default_factory=dict)
    records: list[Region] = field(default_factory=list)
    torn_tail_bytes: int = 0         # bytes after the last whole record (process stopped mid-write)

    def regions(self) -> list[Region]:
        """One region per code_start, in first-seen order; a jit_done record replaces a
        METHOD-typed code buffer at the same address (it carries the image name)."""
        by_start: dict[int, Region] = {}
        for r in self.records:
            old = by_start.get(r.start)
            if old is None or (old.kind == "method_buffer" and r.kind == "method"):
                by_start[r.start] = r
        return list(by_start.values())


def parse(data: bytes) -> Capture:
    if data[:4] == MAGIC:
        (fmt,) = struct.unpack_from("<I", data, 4)
        if fmt != 2:
            raise CaptureError(f"capture format {fmt} is not known")
        return _parse_2(data)
    return _parse_1(data)


def _parse_1(d: bytes) -> Capture:
    cap = Capture(format=1)
    p = 0
    while p + 16 <= len(d):
        start, size, n = struct.unpack_from("<QII", d, p)
        if p + 16 + n + size > len(d):
            break
        name = d[p + 16:p + 16 + n].decode("utf-8", "replace")
        cap.records.append(Region(start, d[p + 16 + n:p + 16 + n + size], name, "", "method"))
        p += 16 + n + size
    cap.torn_tail_bytes = len(d) - p
    return cap


def _parse_2(d: bytes) -> Capture:
    cap = Capture(format=2)
    p = 8
    while p + 4 <= len(d):
        (kind,) = struct.unpack_from("<I", d, p)
        q = p + 4
        if kind == KIND_CPUID:
            if q + 4 > len(d):
                break
            (count,) = struct.unpack_from("<I", d, q)
            q += 4
            if q + 24 * count > len(d):
                break
            for _ in range(count):
                leaf, sub, a, b, c, dx = struct.unpack_from("<6I", d, q)
                cap.cpuid[(leaf, sub)] = (a, b, c, dx)
                q += 24
        elif kind == KIND_METHOD:
            if q + 20 > len(d):
                break
            start, size, nl, il = struct.unpack_from("<QIII", d, q)
            q += 20
            if q + nl + il + size > len(d):
                break
            name = d[q:q + nl].decode("utf-8", "replace")
            image = d[q + nl:q + nl + il].decode("utf-8", "replace")
            q += nl + il
            cap.records.append(Region(start, d[q:q + size], name, image, "method"))
            q += size
        elif kind == KIND_BUFFER:
            if q + 20 > len(d):
                break
            start, size, t, dl = struct.unpack_from("<QIII", d, q)
            q += 20
            if q + dl + size > len(d):
                break
            desc = d[q:q + dl].decode("utf-8", "replace")
            q += dl
            kind_name = BUFFER_TYPES[t] if t < len(BUFFER_TYPES) else f"buffer_type_{t}"
            cap.records.append(Region(start, d[q:q + size], desc, "", kind_name))
            q += size
        else:
            raise CaptureError(f"record kind {kind} at offset {p} is not known")
        p = q
    cap.torn_tail_bytes = len(d) - p
    return cap


def cpu_features(cap: Capture) -> dict[str, bool]:
    """The CPUID bits that change which instructions Mono's amd64 JIT emits (SDM Vol 2A, CPUID)."""
    one = cap.cpuid.get((1, 0))
    seven = cap.cpuid.get((7, 0))
    if one is None:
        return {}
    ecx, edx = one[2], one[3]
    f = {"sse2": bool(edx >> 26 & 1), "sse3": bool(ecx & 1), "ssse3": bool(ecx >> 9 & 1),
         "sse4_1": bool(ecx >> 19 & 1), "sse4_2": bool(ecx >> 20 & 1), "popcnt": bool(ecx >> 23 & 1),
         "avx": bool(ecx >> 28 & 1), "fma": bool(ecx >> 12 & 1)}
    if seven is not None:
        f |= {"avx2": bool(seven[1] >> 5 & 1), "bmi1": bool(seven[1] >> 3 & 1),
              "bmi2": bool(seven[1] >> 8 & 1), "avx512f": bool(seven[1] >> 16 & 1)}
    return f
