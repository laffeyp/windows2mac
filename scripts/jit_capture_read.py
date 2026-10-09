"""jit_capture_read.py — read a tools/jit_capture capture file and run the decoder differential
over every captured method body (Sprint 033).

Record layout (little-endian): u64 code_start, u32 code_size, u32 name_len, name, code bytes.
A method body is decoded linearly from its first byte: Mono emits one method per region with its
code first, so a linear pass covers it until data or padding stops it, and the pass reports where.

Usage: uv run python scripts/jit_capture_read.py <capture.bin> [--json out.json]
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

from w2m.x86dec import DecodeError, decode
from w2m.xedref import decode_many


@dataclass
class Method:
    start: int
    name: str
    code: bytes


def read(path: Path) -> tuple[list[Method], int]:
    """The records in a capture, and the count of trailing bytes that do not form a whole record
    (the process was stopped mid-write)."""
    d = path.read_bytes()
    out: list[Method] = []
    p = 0
    while p + 16 <= len(d):
        start, size, n = struct.unpack_from("<QII", d, p)
        if p + 16 + n + size > len(d):
            break
        name = d[p + 16:p + 16 + n].decode("utf-8", "replace")
        out.append(Method(start, name, d[p + 16 + n:p + 16 + n + size]))
        p += 16 + n + size
    return out, len(d) - p


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("capture", type=Path)
    ap.add_argument("--json", type=Path)
    a = ap.parse_args(argv)
    methods, torn = read(a.capture)
    assemblies = Counter(m.name.split(":")[0].rsplit(".", 1)[0] if ":" in m.name else "(other)"
                         for m in methods)
    positions: list[bytes] = []
    stops: Counter[str] = Counter()
    for m in methods:
        off = 0
        while off < len(m.code):
            raw = m.code[off:off + 15]
            try:
                ins = decode(raw)
            except DecodeError as e:
                stops[type(e).__name__] += 1
                break
            positions.append(raw)
            off += ins.length
    xed = decode_many(positions)
    ours: Counter[str] = Counter()
    length_differs = iclass_differs = xed_refused = 0
    for raw, x in zip(positions, xed, strict=True):
        got = decode(raw)
        ours[got.iclass] += 1
        if not x.ok:
            xed_refused += 1
        elif x.length != got.length:
            length_differs += 1
        elif x.iclass != got.iclass:
            iclass_differs += 1
    total_code = sum(len(m.code) for m in methods)
    print(f"methods {len(methods)}; code bytes {total_code}; torn tail bytes {torn}")
    print(f"instructions decoded {len(positions)}; linear passes stopped early {sum(stops.values())} "
          f"{dict(stops)}")
    print(f"vs XED: refused {xed_refused}; length differs {length_differs}; iclass differs "
          f"{iclass_differs}; distinct iclasses {len(ours)}")
    for name, n in assemblies.most_common(8):
        print(f"  {n:7} {name}")
    if a.json:
        a.json.write_text(json.dumps({
            "methods": len(methods), "code_bytes": total_code, "torn_tail_bytes": torn,
            "instructions": len(positions), "stops": dict(stops), "xed_refused": xed_refused,
            "length_differs": length_differs, "iclass_differs": iclass_differs,
            "iclasses": dict(ours.most_common())}, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
