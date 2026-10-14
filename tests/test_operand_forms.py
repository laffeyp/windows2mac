# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Operand forms (recentering step 5): the guards no class run can reach. The lifter puts a register's store before
the flags op that reads its old value, and every lowering of a 32-bit operation reads only the low 32 bits of its
operands, so a forward past an old-value reader and a 32-bit register read whole change no instruction the class runs
hold. These op lists put the reader first and give a 32-bit load a 64-bit reader, as a later lift or lowering could."""

from __future__ import annotations

from w2m.vm_backend import _operand_forms

RAX, RBX = 0, 3


def last_reads(ops: tuple) -> dict[int, int]:
    from w2m.vm_backend import _temps_read
    last: dict[int, int] = {}
    for i, op in enumerate(ops):
        for t in _temps_read(op):
            last[t] = i
    return last


def test_a_result_is_not_forwarded_past_a_reader_of_the_old_value() -> None:
    ops = (("ld", 1, ("r", RAX, 64, False)), ("ld", 2, ("r", RBX, 64, False)), ("alu", 3, "add", 1, 2, 64),
           ("flags", "add", 1, 2, 3, 64), ("st", ("r", RAX, 64, False), 3))
    alias, _quiet, forward = _operand_forms(ops, last_reads(ops), set())
    assert alias == {1: RAX, 2: RBX}
    assert forward == {}                                  # the flags op reads RAX's old value after the add


def test_a_result_with_dead_flags_is_computed_in_place() -> None:
    ops = (("ld", 1, ("r", RAX, 64, False)), ("ld", 2, ("r", RBX, 64, False)), ("alu", 3, "add", 1, 2, 64),
           ("flags", "add", 1, 2, 3, 64), ("st", ("r", RAX, 64, False), 3))
    dead = {3}
    last = {1: 2, 2: 2, 3: 4}                            # the dead flags op reads nothing
    alias, _quiet, forward = _operand_forms(ops, last, dead)
    assert alias == {1: RAX, 2: RBX} and forward == {3: RAX}


def test_a_32_bit_register_with_a_64_bit_reader_is_copied() -> None:
    ops = (("ld", 1, ("r", RAX, 32, False)), ("ld", 2, ("r", RBX, 64, False)), ("alu", 3, "add", 1, 2, 64),
           ("st", ("r", RBX, 64, False), 3))
    alias, _quiet, _forward = _operand_forms(ops, last_reads(ops), set())
    assert 1 not in alias                                 # the add would read RAX's upper half


def test_a_32_bit_register_read_by_32_bit_ops_is_read_in_place() -> None:
    ops = (("ld", 1, ("r", RAX, 32, False)), ("ld", 2, ("r", RBX, 32, False)), ("alu", 3, "add", 1, 2, 32),
           ("st", ("r", RAX, 32, False), 3))
    alias, _quiet, forward = _operand_forms(ops, last_reads(ops), set())
    assert alias == {1: RAX, 2: RBX} and forward == {3: RAX}
