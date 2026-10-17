# Sprint 104 — N vCPUs, each on its own thread, each with its private translation chain

```yaml
---
id: 104
status: closed
phase: 5
pass_kind: architecture
---
```

## scope

Design B of `docs/design-several-vcpus.md` (Decision 2026-10-17), its first half: the machinery, no scheduling yet.

- **vm_monitor.c:** `vcpus <n> <channel_base>` after setup creates vCPUs 1..n-1, each on its own thread (hv_vcpu.h:
  one vCPU per thread, its calls from that thread), each with MPIDR_EL1 affinity Aff0 = its index, the system
  registers vCPU 0 holds copied at creation except TTBR0_EL1, which the host names per vCPU, and its own watchdog
  deadline; each serves the channel at `channel_base + index * PAGE` with today's protocol. vCPU 0 is unchanged.
- **EnTSO on every vCPU, vCPU 0 included.** The backend decision (BLACKBOARD Decision 2026-10-07) rests on guest TSO,
  and Sprint 009 measured MP's forbidden outcome 2,245 times in 5 million with ACTLR_EL1.EnTSO off and 0 in 50
  million with it on; w2m's VM never set it (no ACTLR write in vm_backend.py or vm_monitor.c at 72d170f7). One vCPU
  cannot show the gap; a second would. `HV_SYS_REG_ACTLR_EL1` is public (Sprint 008, hv_vcpu_types.h: "only allows
  getting / setting of the ACTLR_EL1.EnTSO bit (index 1)").
- **vm_backend.py:** each vCPU's level-0 table and the private chain below it to the monitor data page, the runtime
  stack and the state block; every other descriptor shared; every later write into a private-chain table written
  into each copy. The dispatcher's and the code-write path's TLBI VMALLE1 and IC IALLU become VMALLE1IS and IALLUIS.
  The process-wide words of the monitor data page (CLOCK_MODE, CLOCK_RATE, CLOCK_WARP, CLOCK_EPOCH, RECON_COUNT,
  START_MODE, FOREGROUND_MODE) are read and written on vCPU 0's page and copied to each other vCPU's at its start;
  moving them to a process page waits for Sprint 106, where a second vCPU first runs guest threads.

## prerequisites

- 103.

## context_files

- docs/design-several-vcpus.md; tools/vm_monitor/vm_monitor.c; src/w2m/vm_backend.py (`_write_translation_tables`,
  `_page_descriptor`, `Channel`, the sys-register setup, the dispatcher, `_write_exception_entry`); the SDK's
  hv_vcpu.h.

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged; with n = 1 every run behaves as before.

## artifact contract

- The monitor command, the private chains, the IS invalidations; `tests/test_vcpus.py`; `scripts/gates.sh` exit 0;
  CI `success` on the closing commit.

## observation contract

- **Judge (tests/test_vcpus.py, hvf):** with n = 4, each vCPU runs a few AArch64 words at the monitor's code area that
  store its MPIDR_EL1 into the monitor data page and the state block at their fixed addresses and read both back:
  four distinct values at one address, and each vCPU's page in the memory file holds its own; a page mapped by the
  host after the vCPUs started is readable on all four; TTBR0_EL1's CnP bit reads 0 and ACTLR_EL1.EnTSO reads 1 on all four.
- **Plant:** the private chain's monitor-data entry made shared (every vCPU's leaf pointing at vCPU 0's page): the
  four values collapse to one and the test fails.
- **No change at n = 1:** 68 of 68 families accept; the batch run conforms (168 of 168 lines, 0 divergences).

## done criteria

N vCPUs run on their own threads with their own state at shared addresses; nothing else changes; CI green.

## observed

- **Made** (9d9ce7f4): vm_monitor.c `vcpus`, one thread a vCPU with its own slot, watchdog and channel; vm_backend.py
  `_private_chain`, `_write_descriptor` (a parent write mirrored into each copy), `_start_vcpus`; the vCPU area
  (8 vCPUs, 0x20000 each) after the code-page set; ACTLR_EL1.EnTSO on every vCPU; TLBI VMALLE1IS and IC IALLUIS in the
  dispatcher and the code-write path, `tlbi vmalle1is` in runtime.h's w2m_tlb_flush (encodings checked against
  clang's assembler: d508831f, d508711f).
- **Judge** (tests/test_vcpus.py): vCPUs 1-3 read MPIDR_EL1 0x80000001-0x80000003, each read its own value back from
  the monitor data page and the state block at the shared addresses, each value in its own page of the memory file,
  vCPU 0's word untouched; all three read 0x5eed0fa11cafe from the heap page mapped after they started; ACTLR_EL1 2
  (EnTSO) and TTBR0_EL1 with CnP clear on all four.
- **Plant:** every vCPU's monitor-data leaf at vCPU 0's page: the private pages hold 0, vCPU 0's page the last
  store (0x80000003).
- **A defect of this sprint's, found by the gates:** `Channel.__init__` reused the name `base` for the buffer's
  process address and read its sequence word there; with one VM the address fell inside the 550 GB sparse file and
  read 0; with several VMs in one process it fell outside (11 tests failed, struct.error and BufferError). Fixed
  before the commit.
- **No change at one vCPU** (9d9ce7f4): 456 of 458 classes, 68 of 68 families; the batch run conforms, 168 of 168
  lines, 0 divergences (frontier-643a6283510c-1792226660662772000).
