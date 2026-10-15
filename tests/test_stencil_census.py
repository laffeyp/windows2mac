# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The stencil census (Sprint 071): an instruction's words are its logged encoder calls, the words no encoder made and
its branch words, exactly; the census's forms with value classes give one encoder sequence for instructions that
differ only in their registers and values within a class, and two where a displacement leaves LDR's scaled range."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import stencil_census as sc  # noqa: E402
from w2m import a64_encoder, vm_backend  # noqa: E402
from w2m.lift import lift  # noqa: E402

EXIT = 0x80_0000_0000
CODES = [
    bytes.fromhex("4801d8"),            # add rax, rbx
    bytes.fromhex("4801ca"),            # add rdx, rcx
    bytes.fromhex("488b442408"),        # mov rax, [rsp+8]
    bytes.fromhex("488b4c2418"),        # mov rcx, [rsp+0x18]
    bytes.fromhex("488b8424f8ff0000"),  # mov rax, [rsp+0xfff8]: past LDR's scaled 12-bit offset (4095 * 8)
    bytes.fromhex("31c0"),              # xor eax, eax
    bytes.fromhex("e800010000"),        # call +0x100
    bytes.fromhex("0f8410000000"),      # jz +0x10
    bytes.fromhex("48c1e803"),          # shr rax, 3
    bytes.fromhex("f30f5ec1"),          # divss xmm0, xmm1
]


def calls() -> list[sc.Call]:
    return [(lift(code, 0x1_4000_1000), EXIT, None, vm_backend.ALL_FLAGS, False) for code in CODES]


def test_the_items_rebuild_each_instructions_words() -> None:
    log = sc.EncoderLog()
    words_of = {name: getattr(a64_encoder, name).__wrapped__ for name in dir(a64_encoder)
                if hasattr(getattr(a64_encoder, name), "__wrapped__")}
    for call in calls():
        words, items = sc.compile_logged(log, call, {})
        fixed = [it if it[0] != "branch" else (*it, w) for it, w in zip(items, sc._item_words(items, words))]
        assert sc.rebuild(fixed, words_of) == words, call[0]
        assert any(it[0] in ("enc", "enc32") for it in items), call[0]


def test_a_value_class_splits_a_form_where_the_code_does() -> None:
    log = sc.EncoderLog()
    near, near2, far = (calls()[i] for i in (2, 3, 4))
    sigs = []
    keys = []
    for call in (near, near2, far):
        _, items = sc.compile_logged(log, call, {})
        sigs.append(tuple((it[0], it[1], len(it[2])) if it[0] != "raw" else ("raw",) for it in items))
        key, values = sc.form(call)
        keys.append(sc.refined(key, values, len(values) - len(sc._values_only(call))))
    assert sc.form(near)[0] == sc.form(far)[0]          # one form before value classes
    assert sigs[0] == sigs[1] and keys[0] == keys[1]    # [rsp+8] and [rsp+0x18]: one stencil
    assert sigs[0] != sigs[2] and keys[0] != keys[2]    # [rsp+0xfff8]: another, and another refined form
