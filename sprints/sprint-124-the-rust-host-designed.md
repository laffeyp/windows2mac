# Sprint 124 — the Rust host designed

```yaml
---
id: 124
status: done
phase: 6
pass_kind: architecture
---
```

## scope

Step 1 of `docs/rust-port-plan-2026-10-21.md`: `docs/design-rust-host-2026-10-23.md`, before any host code. It places
every piece of state the Python host keeps (each attribute set on `VM`, `Vcpu`, `Channel`, `Machine`, `Process`,
`GuestThread`, `FileBridge`, `VirtualMemory` and `Reservation`) in one of three kinds: owned by one vCPU's thread;
shared, under a named lock held only for the access; or written once before the first run and read-only after. It
names the Rust struct and layer (cpu, memory, nt, host, diagnostics) each lands in, and states, with sources, the two
requirements Decision 2026-10-23 set: a thread that serves one vCPU never slows another's, and every VM is closed and
its backing file removed when its process ends. It closes with a feasibility check against the plan's numeric gates
(KIT_DIARY #64): the exit rate a vCPU thread must serve (90,000 a second seen on 2026-10-22) and what each shared lock
costs it in time per exit, from a measurement on this machine.

## prerequisites

- 123 (closed; the oracle the design is judged against)

## context_files

- `sdd-kit-3/AGENTS.md`; `BLACKBOARD.md` Decisions 2026-10-21 (user), 2026-10-22 and 2026-10-23; `WORKING_AGREEMENT.md`
  (bridge mapping: Hypervisor.framework)
- `docs/rust-port-plan-2026-10-21.md` ("The new shape", step 7)
- `docs/design-several-vcpus-2026-10-17.md` (the Python host's design, designed twice)
- `src/w2m/vm_backend.py`, `kernel.py`, `scheduler.py`, `file_bridge.py`, `virtual_memory.py`; `tools/vm_monitor/vm_monitor.c`

## signal contract

### Emits

None: a design document fires no tag. The vocabulary (0.9) does not change.

### Consumes

None.

### Invariants

- No file under `src/`, `tools/` or `store/` changes.

## artifact contract

### Files created

- `docs/design-rust-host-2026-10-23.md`

### Content assertions

- The design has a state table with one row per attribute, 202 rows (`grep -c '^| `'` on the table): class,
  attribute, today's kind and guard, the Rust kind, lock and home, the line where it is first set.
- Every lock the table names has a row in the locks table: what it guards, who takes it, how long, its policy; and a
  lock order.
- The two requirements of Decision 2026-10-23 each have a section with the measurement behind them.

### Command exit codes

None: the sprint adds no program. (A Python check written for it was removed before its commit: w2m is Rust only,
user 2026-10-23.)

## observation contract

- **Expected value:** the table's row count equals the attribute count of the frozen host, 202, taken from
  `python-final`'s source on 2026-10-23; the frozen tree cannot add one.
- **Expected value:** the feasibility section's per-exit cost, measured, is under 1/90,000 s (11 µs) by a margin
  it states; a design over that budget is rewritten before the sprint closes.

## done criteria

The design places every attribute and its feasibility numbers are measured.

## self-review checklist

- [x] Scope names one verifiable property: every piece of host state placed.
- [x] context_files cover the Python host and its earlier design.
- [x] Emits: none, stated.
- [x] Artifact contract gradable by reading the table and its row count.
- [x] Observation contract: two expected values, one with a bound.
- [x] One file, one concept.
