# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Sprint 076: instances made for a key keep the key; a stencil derived from them rebuilds fresh instances of the key,
drawn with another seed, word for word as the compiler compiles them."""

from __future__ import annotations

import random
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import stencil_census as sc  # noqa: E402
import stencil_table as st  # noqa: E402
from w2m import vm_backend, x86dec  # noqa: E402
from w2m.lift import lift  # noqa: E402

EXIT = 0x80_0000_0000
CODES = [
    bytes.fromhex("4801d8"),            # add rax, rbx
    bytes.fromhex("488b442408"),        # mov rax, [rsp+8]
    bytes.fromhex("488b4c8d10"),        # mov rcx, [rbp+rcx*4+0x10]
    bytes.fromhex("4883c007"),          # add rax, 7
    bytes.fromhex("48c1e803"),          # shr rax, 3
    bytes.fromhex("f30f5ec1"),          # divss xmm0, xmm1
    bytes.fromhex("0fc6c1aa"),          # shufps xmm0, xmm1, 0xaa
    bytes.fromhex("488d0510000000"),    # lea rax, [rip+0x10]
]


def test_instances_keep_the_key_and_the_stencil_rebuilds_fresh_ones() -> None:
    record = {"bytes": {lift(c, 0x1_4000_1000): {c} for c in CODES}}
    implicit = sc.implicit_registers(record)
    st._init({})
    for code in CODES:
        call = (lift(code, 0x1_4000_1000), EXIT, None, vm_backend.ALL_FLAGS, False)
        row = x86dec.decode(code).row or ""
        key = sc.decoded_form(call, code, implicit.get(row, frozenset()))[0]
        made = st.instances(call, code, key, implicit.get(row, frozenset()), random.Random(1))
        assert len(made) > 3, code.hex()
        for inst in made:
            assert sc.decoded_form(inst[0], inst[1], implicit.get(row, frozenset()))[0] == key
        samples = st.gather([(call, code)], key, implicit.get(row, frozenset()), 1, Counter())
        kind, stencil = st.derive(samples, {})
        assert kind == "stencil", code.hex()
        assert not any(s[0] == "other" for _, _, specs in stencil for s in specs), code.hex()
        fresh = st.instances(call, code, key, implicit.get(row, frozenset()), random.Random(2))[1:]
        assert fresh, code.hex()
        for inst in fresh:
            compiled = st._compiled(inst)
            assert compiled is not None
            assert st.rebuild(stencil, inst[2], {}, st._STATE["words_of"]) == compiled[0], (code.hex(), inst[1].hex())
