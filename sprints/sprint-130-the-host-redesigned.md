# Sprint 130 — The host redesigned: no lock across a service

```yaml
---
id: 130
status: done
phase: 6
pass_kind: architecture
---
```

## scope

Fix (b) of the direction of 2026-10-24 (BLACKBOARD Decision 2026-10-24 user), step 1 of
`docs/rust-port-plan-2026-10-24.md`. `docs/design-rust-host-2026-10-24.md` replaces the design of 2026-10-23 and
changes four things, each stated against the goal's figures and the systems that solved it: the guest's runtime word
guards only the run table, and `w2m_host` gives it up around `hvc #8`; translation goes into a region per vCPU with a
lock-free lookup and a lock per guest code page for invalidation; one process's objects and waits stay inside its VM,
and the supervisor holds only what crosses processes; a vCPU spinning on a held word waits after a bounded spin, by
the futex protocol's three states. Its acceptance is the goal's figures, measured at the plan's steps, not the count
of the Python host's attributes.

## prerequisites

- 129 (closed)

## context_files

- `docs/design-rust-host-2026-10-23.md` (replaced); `docs/rust-port-plan-2026-10-24.md`
- `tools/guest_runtime/runtime.c` 20-36 (`w2m_host`), `runtime.h` 68-78 (the transport ops); `src/w2m/vm_backend.py`
  521-547 (`lock_runtime`, `unlock_runtime`); `tools/runtime_lock/runtime_lock.c`
- primary sources: QEMU commit 0ac20318ce16 and `docs/devel/multi-thread-tcg.rst`; crosvm `ARCHITECTURE.md`;
  Firecracker `docs/design.md`; Wine's Architecture Overview, esync's README, Proton 4.11-1b, the kernel's ntsync
  page, LWN 961884; WWDC20 10686; FEX-2609; Uhlig et al., VM '04; Drepper, "Futexes Are Tricky" 1.6

## signal contract

### Emits

None. A document.

### Consumes

The documents and sources above.

### Invariants

- The design of 2026-10-23 stays in place as history, with a pointer line added at its head.
- No code changes in this sprint: the changes to `runtime.c` and the in-VM compiler's cursor land in the plan's step
  3 and the steps that run them.

## artifact contract

### Files created

- `docs/design-rust-host-2026-10-24.md`

### Files modified

- `docs/design-rust-host-2026-10-23.md` (a pointer line); `BLACKBOARD.md`

### Content assertions

- The lock table holds no lock held across a service, a compile or a wait; the runtime word is a leaf.
- Every quoted source carries its URL; every figure cites its document and line or says it is derived.

### Command exit codes

- `target/release/design_check` returns 0 (2 documents checked)
- `scripts/gates.sh` returns 0

## observation contract

- **Expected value:** `design_check` passes both documents of 2026-10-24.
- **Planted fault:** the design with its QEMU source removed fails `design_check`, naming QEMU.

## done criteria

The Rust host's design holds no global lock: no thread waits, on any service path, for a lock another thread's
service holds.

## self-review checklist

- [x] Scope names one verifiable property: no lock across a service, a compile or a wait.
- [x] context_files name the code each change touches and every source quoted.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command and by reading the lock table.
- [x] Observation contract: the check's expected value and one planted fault.
- [x] One concept: the host's locks.

## result

- `docs/design-rust-host-2026-10-24.md` replaces the design of 2026-10-23, which points to it. Five changes, each
  against the goal's figures and the systems that solved it: (1) each runtime service takes the lock of what it
  touches or none (the three hottest imports none; a lock per heap; the run-table word only for the run table, a
  leaf); (2) a transport page per vCPU and no guest lock across `hvc #8`; (3) translation into per-vCPU regions with
  atomic dispatch slots, tombstones, per-page invalidation locks and per-vCPU epochs; (4) one process's waits in its
  VM, the supervisor only across processes; (5) three-state lock words, a bounded spin, then a wait in the host.
  BLACKBOARD Decision 2026-10-24 agent-resolved, marked `Architecture:`, names it.
- Its budgets are the goal's: the crossings to the host (6,100 to 37,600 a second) and the calls answered inside the
  VM (the hot imports, about 377,000 takings a second of one word today by three calls). One shared lock remains on
  purpose, the run-table word for scheduling (about 31,000 takings a second in play), with a measured revisit
  condition at step 6 and per-vCPU run queues as the named next form.
- The reviewer's two passes changed the draft: the first version left every runtime call under the word (change 1)
  and released it around `hvc #8` on a page all vCPUs share (change 2); change 3 lacked atomic slots, removal and
  reclamation; `files` was held across a host file call; the call trace, TLS allocation and the wait protocol across
  two locks were unstated.
- Expected value: `design_check` passes 2 documents. Planted fault: the QEMU sources removed fails, naming QEMU.
