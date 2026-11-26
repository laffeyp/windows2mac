# Sprint 354 — The woken mark: a thread readied before it blocks stays ready

```yaml
---
id: 354
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 3 of `docs/design-handles-and-sync-objects-2026-11-26.md` ("The wait across two locks", 268-281; "The cards that
follow", 354-357). Card 5 will take waits off the runtime word. A wait will then check its object under the sync
objects' lock, let that lock go, and only then take the word to park in `w2m_block`. A readying that falls between the
two is lost today. `w2m_ready_thread` (`tools/guest_runtime/threads.c` 150-159) readies only a BLOCKED thread, so it
does nothing in that window, and `w2m_block` (91-116) then blocks the thread, which wakes only at its deadline, or never
for INFINITE. The woken mark closes the window, under the word:
- `w2m_ready_thread` sets the mark on a thread that is not BLOCKED;
- `w2m_block` finds it set, clears it, and leaves the thread READY, so the switch retries the call at once.

- **The class first.** The design names `w2m_ready_thread` alone, but a lost readying is a class. List every path that
  readies a blocked thread, in the C runtime and in the host: at least `w2m_ready_thread`, `w2m_signal` (130-145),
  `wake_expired`, and the host's `set_status` READY. Give one line a path, saying whether a readying on that path can
  fall in the window once card 5 lands, and why. `w2m_signal` matches by `wait_key`, which `w2m_block` writes, so a
  thread in the window carries no key yet: say what the mark must cover for it, and cover it. A path left out needs its
  grounds in a Decision.
- **Where the mark lives.** Choose its place in the thread's run-table entry (`struct w2m_run_entry`, threads.c 38-46,
  `reserved1`) or its TEB scratch (`struct w2m_thread_scratch`, `runtime.h`), whichever the host's layout tests and
  `python_image.rs`' frozen key leave free. State the choice with its grounds. The host must clear or ignore the mark
  where it re-uses an entry or a TEB.
- **On one vCPU and under today's word nothing changes.** The window does not exist while the word covers the whole
  call, so the batch run, `wait_handoff` and every family run must pass unchanged.
- **The judge must reach today's code.** A run-level race cannot reach the mark until card 5 (the design, 356-357), so
  this card's judge drives the mark directly: a test that runs the runtime image's own `w2m_ready_thread` on a thread
  that is not yet BLOCKED, then `w2m_block` on that thread, and reads the thread READY with the mark clear. Do the same
  for each other path the class list covers. Form: as Sprint 341's `the_call_ring_stays_exact_and_within_itself_on_two_vcpus`
  drives the image's own `w2m_logged` (`crates/w2m_vm/tests/run_loop.rs` 1935-1948), or a test in the C's own harness
  if one serves. The design's lost-wakeup guest (two threads ping-pong two auto-reset events 100,000 rounds with
  INFINITE waits, on two vCPUs, ending inside its instruction limit) is built here too, and held, for card 5 to plant.
- **Not here:** any lock; the sync objects' state.

## prerequisites

- 353 (closed)

## context_files

- `docs/design-handles-and-sync-objects-2026-11-26.md`: in full.
- `tools/guest_runtime/threads.c`: in full. Also `tools/guest_runtime/runtime.h` (`struct w2m_thread_scratch`), and
  `tools/guest_runtime/sync_objects.c` (each call of `w2m_block`, `w2m_ready_thread` and `w2m_signal`).
- `crates/w2m_vm/src/threads.rs` (`set_status`, `readied`, `add_thread`, and every place an entry or TEB is
  (re)initialized).
- `crates/w2m_vm/tests/run_loop.rs` 1935-2050 (the image-driving test form), and `wait_handoff`'s tests.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, the frozen key).
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; a thread's state between a readying and its block.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- The 273 family runs on two vCPUs pass as at Sprint 351, and `three_counters` as at Sprint 353.

## artifact contract

### Files modified

- `tools/guest_runtime/threads.c`, `runtime.h` or the run entry's layout as the place needs, the host's clearing,
  the judge, the lost-wakeup guest and its test, and any held line the gates name (`DEPARTURES`, the batch run's held
  stops).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list, one line a readying path.
  - The mark's place and its grounds.
  - The judge's reading for each covered path: READY with the mark clear, and on today's code, before the change, BLOCKED.
  - The lost-wakeup guest's runs on two vCPUs: 20 runs, each ending inside its limit, with their instruction counts.
- **Planted faults:**
  - (1) `w2m_block`'s reading of the mark removed. The judge fails, the thread BLOCKED.
  - (2) `w2m_ready_thread`'s setting of the mark removed. The judge fails.
  - (3) Each further path the class list covers, its setting removed in turn. The judge fails.
  - (4) The host's clearing removed. A test fails, a re-used entry or TEB starting with the mark set.

## done criteria

Every path that readies a blocked thread is listed and judged against the window card 5 opens. A readying before the
thread blocks leaves it READY, read directly on the runtime image for each covered path. One vCPU and today's two-vCPU
runs are unchanged. The lost-wakeup guest is built and held for card 5.

## self-review checklist

- [x] Scope names one verifiable property: a readying before the block is not lost.
- [x] context_files cite the design, the C, the host's writers and the image-driving test form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the class list, the place, the direct readings before and after; numbered plants.
- [x] Sweet spot: `threads.c` with its judge; one concept, the woken mark.

## result

GATES-LINE
