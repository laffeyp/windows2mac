# Sprint 345 — The TLS and FLS slots take their own lock, not the runtime word

```yaml
---
id: 345
status: active
phase: 6
pass_kind: functional
---
```

## scope

The third lock class of Change 1 is **the TLS and FLS slots' lock** (`docs/design-rust-host-2026-10-24.md` 113-114:
"for TlsAlloc, TlsFree, FlsAlloc and FlsFree, which change the process's slot bitmap"). In the guest's lock order it
comes after the handle table and before a sync object, a heap and the run-table word. Sprint 343 built the heap's lock,
and this card moves the slot services off the runtime word onto the slots' own lock.

`tools/guest_runtime/thread_storage.c` (179 lines, read whole for this card) shows the class reaches further than the
design's four calls:
- `tls_used`, `fls_used` and `fls_callback` are process-wide. TlsAlloc, TlsFree, FlsAlloc and FlsFree change them.
  FlsSetValue reads `fls_used`.
- `fls_blocks` lists every thread's FLS block. FlsSetValue's first set on a thread takes a block from the heap and
  links it (`fls_mine`, 43-55). FlsAlloc and FlsFree walk the list and write each block's value at their index.
- `tls_init`'s `tls_ready` (56-72) is a one-time set-up with the race the heap's `ready` had (Sprint 343: a flag
  stored before the state it guards).

So the class is TlsAlloc, TlsFree, FlsAlloc, FlsFree, FlsSetValue, FlsGetValue and FlsGetValue2. TlsGetValue,
TlsSetValue and TlsGetValue2 touch only the calling thread's TEB and stay as they are.

- **The proof, before the change.** Quote, from `thread_storage.c` whole, every store each of the seven makes and
  every read of shared state, and name which the lock must cover. Then name every call each makes:
  - `w2m_heap_take` is taken under the slots' lock, the order slots then heap, which the design allows;
  - `w2m_unimplemented` and `w2m_logged` are never called with the lock held, as in Sprint 343.

  Also grep for any other runtime C that reads or writes the slots' state, and quote what it finds.
- **The lock.** It is a word taken and released as `heap.c`'s `hold` and `let_go` do: a plain load until it reads 0,
  then a compare-and-swap with acquire, and a release store. Choose its place and quote the built image's inline
  `casa` and `stlr`. `tls_init`'s set-up runs under it.
- **The service table.** A fourth lock class, the slots, for the seven. Their bridges take no runtime word, as for
  classes none and heap, the bridge's length kept (Sprint 342's Decision on the frozen host's code addresses).
- **The judge.** `probes/thread_slots_race/`, committed with this card, compiles against the mingw-w64 headers:
  - **Part 1:** threads A and B each run 20,000 rounds at once. Each round a thread takes 8 TLS indices, marks each
    by a compare-and-swap on a shared owner word, then clears the marks and frees the indices. Each index found held
    by the other thread counts as `duplicate`, and the rounds in which the other moved count as `tls.overlap`.
  - **Part 2:** 16 threads each make their FLS data by a first FlsSetValue of one index at once. The main thread
    frees the index, and each thread reads it back. A value left after the free counts as `stale`.
  - **Its Windows rows:** build and run it on the Windows runner (`.github/workflows/windows-probe.yml`, three runs).
    Keep the binary and the runs' files, and add the walk fixture's entry, as card 344 did
    (`store/probe-runs/45760720489/`, its result). Every row but `tls.overlap` must read alike in the three runs. If
    they do not, stop and say so.
  - **Its test:** a `run_loop.rs` test runs the probe through `w2m-vm` on two vCPUs, then on one. It holds every row
    but `tls.overlap` to Windows', and `tls.overlap` above a floor on two vCPUs, measured and grounded as card 344's
    was (`each_threads_last_error_tls_value_and_id_stay_its_own_on_two_vcpus`).
- **Not here:**
  - FlsFree's callbacks, which still stop the run;
  - Change 1's other classes;
  - any guest-visible change to what the services return.

## prerequisites

- 344 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` 95-140, Change 1, its classes and its order.
- `tools/guest_runtime/thread_storage.c` whole, `tools/guest_runtime/heap.c` (`hold`, `let_go`, Sprint 343) and
  `tools/guest_runtime/runtime.c` (`w2m_logged`, `w2m_unimplemented`).
- `probes/thread_slots_race/thread_slots_race.c` and `probe.json` (this card's commit).
- `crates/guest_runtime/src/services.rs` (`Lock`, `NO_LOCK`, `HEAP_LOCK`, their tests) and `crates/w2m_vm/src/guest.rs`
  (`bridge`, its heap-class test).
- `crates/w2m_vm/tests/run_loop.rs`: `each_threads_last_error_tls_value_and_id_stay_its_own_on_two_vcpus` (Sprint 344)
  and `program_through_w2m_vm`.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, which must name `thread_storage.c`).
- `.github/workflows/windows-probe.yml`; card 344's result (`sprints/sprint-344-each-threads-own-values-on-two-vcpus.md`)
  for the Windows run, the stored binary and the walk fixture's entry.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; which lock the slot services take, no new event.

### Invariants

- On one vCPU every slot service returns what it returns today, and the batch run, the exit ledger and the family runs
  pass unchanged.
- The slots' lock is never held across `w2m_unimplemented` or `w2m_logged`, and it is taken before the heap's, never
  after.

## artifact contract

### Files modified

- `tools/guest_runtime/thread_storage.c`, `crates/guest_runtime/src/services.rs` (the slots class),
  `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`), `crates/w2m_vm/tests/run_loop.rs`,
  `crates/image_walk/tests/fixtures.rs` (its totals), `store/translation-cache/walk-fixtures-2026-10-30.json` (one
  entry), `store/binaries/` (the probe's binary), `store/probe-runs/<run id>/`.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The proof quoted.
  - The built lock's `casa` and `stlr` quoted from the seven services.
  - The Windows run's id, its runner and CPU, and its three runs' rows.
  - The test's rows on two vCPUs and on one, `tls.overlap` on each, and the floor's grounds.
  - The table test naming the seven services of class slots.
- **Planted faults:**
  - (1) The slots' lock taken and released as no-ops while the seven stay off the word. The probe's test fails on two
    vCPUs on `duplicate` or `stale` above 0.
  - (2) FlsSetValue set to class none, not slots. The table test fails by name.

  Before the work, read each judge's reach. With the seven off the word and no lock yet, run the probe on two vCPUs
  and quote its rows.

## done criteria

The seven slot services take the slots' own lock and not the runtime word. The lock covers the slot bitmaps, the
callbacks, the FLS block list and the set-up, is never held across another lock but the heap's, and two threads taking
and freeing slots at once never share an index or lose an FLS block.

## self-review checklist

- [x] Scope names one verifiable property: the slot services off the word, on their own lock, safe under two vCPUs.
- [x] context_files cite Change 1, the slot C, the heap's lock form, the probe, the class and bridge, and the Windows
  run's procedure.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the proof, the built lock, Windows' rows, the test; two numbered plants, their reach read.
- [x] Sweet spot: `thread_storage.c` and `services.rs` plus a probe's test; one concept, the slots' lock.

## result

GATES-LINE
