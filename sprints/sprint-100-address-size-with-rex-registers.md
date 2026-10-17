# Sprint 100 — 32-bit addressing through R8D-R15D

```yaml
---
id: 100
status: active
phase: 5
pass_kind: functional
---
```

## scope

The fifth of the five entries deferred that should not have been (user decision 2026-10-16), the open question of
Sprint 089: does lift() address through R8D-R15D under the 0x67 prefix with 64 bits? It does. `_operand` (src/w2m/lift.py)
takes 32-bit addressing only when the base's name starts with E (or there is no base and the index's does). The decoder
names a REX-extended 32-bit base R8D-R15D (`67 41 8b 00`, mov eax, [r8d]: `M:-:R8D:INVALID:-:0:4`; `67 42 8b 04 00`
indexes by R8D under an EAX base, already 32-bit), so a base R8D-R15D addresses with all 64 bits of R8-R15.

**Known first.**

- Intel SDM Vol 1 3.3.7 (Address Calculations in 64-Bit Mode): with the address-size prefix in 64-bit mode the
  effective address is 32 bits, computed from the 32-bit registers, R8D-R15D among them, and zero-extended. A
  question the manual answers is not tested on hardware (user rule, 2026-10-16).
- The class case sets hold 11,202 0x67 cases, 944 with a base's upper half nonzero, and their classes were accepted
  against Intel's results (Sprint 089): those cases did not tell the two rules apart, so the judge here is a test
  made for the defect, and the case sets are re-run to show nothing else moved.

## prerequisites

- 099.

## context_files

- src/w2m/lift.py (`_operand`, `REG`); src/w2m/symbolic_lift.py; scripts/stencil_table.py, store/stencil/; src/w2m/vm_backend.py (the `addr` op's 32-bit truncation); tests/test_lift
  equivalents (tests/test_instruction_count.py's VM runner for a run in the VM).

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged; every class's verdict against the Xeon unchanged.

## artifact contract

- Found after the card was written, by the class of the error: `symbolic_lift.py` repeats the rule (its templates
  must equal lift), and the VM compiler's stencil table, derived from compile_insn on the old lift, holds the 64-bit
  address for every such form (`tests/test_vm_compiler_c.py`'s equal-words judge failed on `67 47 8d 04 11`, lea eax,
  [r9d + r10d]). Both are changed; the table is re-derived from its record (`scripts/stencil_table.py`) and rendered
  again (`scripts/gen_stencil_table.py`).
- `scripts/semantics_differential.py` `_place` reads the address size to set a base register: a 32-bit address gets a
  random upper half, which draws from the case generator's random stream. Under the new rule the R8D-R15D bases drew
  32 bits more, every later case changed, and 19 tests over the frozen 039 and 041 sets failed on case files that
  were no longer the ones the CPUs ran. The placement keeps the old rule for those bases (KIT_DIARY #23); both rules
  reach the same address for the values it sets. The re-derived table also covers the iclasses held back since
  Sprints 096 and 098, so `HOST_LOWERED` is empty and the VM compiles them again (the equal-words judges, 15 tests,
  pass on them).
- `_operand` takes the address size from the base register's width in `REG` (the index's with no base); a test runs
  `mov eax, [r8d]` and `lea eax, [r8d + eax]` in the VM with R8's upper half set and reads the 32-bit address;
  `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judge:** the test's loads read the bytes at the zero-extended 32-bit address; every class accepts as before (456
  of 458); every family accepts; the batch run conforms (168 of 168 lines, 0 divergences).
- **Plant:** the old rule (E-named only) restored: the test must fail.

## done criteria

Every 0x67 address is computed in 32 bits as the SDM defines, every judge holds, CI green.

## observed
