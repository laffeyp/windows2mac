# Sprint 354 — The woken mark: a thread readied before it blocks stays ready

```yaml
---
id: 354
status: done
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

## amendment (2026-11-26)

Reviewer-4's twenty-fourth review, Low 1. Sprint 353's 7.6% figure, which the Decision "How a readying in the VM on one
vCPU ends another vCPU's slice" rests on, came from `two_counters`, a guest kept only in the scratchpad
(`/private/tmp/claude-501/-Users-peterlaffey-port/18efb1a6-9978-416c-904b-05e124030429/scratchpad/cost/two_counters.c`
and `.exe`, run by `measure.sh` there). With the guest gone, that figure cannot be measured again: not at G5, which also
owes the dispatcher ISB's cost on a guest that is not mostly waiting, and not when card 5 changes the switch path.

- Commit `tests/data/timed_waits/two_counters.c` from that source, and its `.exe` built with its neighbours' flags.
  Check that the built `.exe` answers as the scratch one did: two vCPUs, ExitProcess 0, and its output the ticks.
- Give it its held stop line in `store/batch_run/first-stop-2026-10-28.tsv`, as `three_counters` has.
- Write in the result the one command that measures it, five runs on two vCPUs with a given `w2m-vm`, as `measure.sh`
  did. The Supervisor puts that command in the Decision's revisit text at the close.

## result

- **The class**, every path that readies a blocked thread, against the window card 5 opens (a wait's check under the
  sync objects' lock, then its park under the runtime word):
  - `w2m_ready_thread` (`threads.c`; `grant`, `grant_all`): in the window, a release granting under the sync objects'
    lock and readying under the word after the waiter's check. Covered.
  - `w2m_signal` (a manual-reset SetEvent; a shared event's set and a pipe's write, key 0; LeaveCriticalSection, the
    SRW releases, the condition variables' wakes): in the window for SetEvent's waiters. A thread there has written no
    `wait_key` (`w2m_block` writes it), so every thread another vCPU runs and not BLOCKED takes the mark, whatever it
    will wait on. Covered.
  - `wake_expired` (`threads.c`, at a slice's end) and `w2m_switch`'s warp: not in it. Both ready by deadline, a level,
    not an edge: the deadline enters the entry at the park, and the next slice's end or warp finds it.
  - The host's `set_status` READY from `wake_expired`, `warp_to_deadline`, `resume_idle` and `timed_waiter`: not in it,
    for the same reason.
  - The host's `schedule` SCHEDULE_END, every BLOCKED thread readied at a thread's end: in it, a wait on a thread's
    handle reading its end under the word before the sync objects' lock (the design, 296-299). Covered, `mark_woken`.
  - The host's `host_request::wake_blocked`, a kernel answer's wake (a shared event, another process, a pipe; a child's
    request; the turn's return, `run.rs` 919): in it, for a shared event's and a process's handle the same way.
    Covered, `mark_woken`.
  - The host's `add_thread` (a new thread READY) and `suspend`'s resume (status unchanged): no blocked thread readied.
- **The place:** the run entry's `reserved1`, its first word, entry offset 0x270 (`threads.c` `woken`, its
  `_Static_assert` at `W2M_RUN_MONITOR_OFF + 0x130`; `threads.rs` `E_WOKEN`). A TEB scratch word would serve as well;
  the entry wins because every reader and writer already holds the entry, the host clears it with the entry's own
  writes, and no new define is needed (a new `runtime_defines` entry moves the key `python_image.rs` builds the frozen
  image under). The Sprint 156 judge's entry bytes at creation are unchanged: the mark is 0. No entry or TEB is
  re-used (`add_thread` appends at the count; the host frees neither); the host clears the mark in `add_thread` and in
  `load`, the C switch at its load. The mark is set only on a thread another vCPU holds (`held_elsewhere`), so one vCPU
  never sets it.
- **The judge,** `run_loop::a_readying_before_the_block_leaves_the_thread_ready`, drives the image's own
  `w2m_ready_thread` and `w2m_signal`, and the host's `wake_blocked` and `schedule` SCHEDULE_END, on thread T held by
  vCPU owner 2 and READY, then the image's `w2m_block` as T. On today's code, before the change: "the woken mark,
  w2m_ready_thread: T's mark 0 after the readying; after its w2m_block T BLOCKED with the mark 0", the same for
  w2m_signal, wake_blocked and SCHEDULE_END. After: "the woken mark, w2m_ready_thread: T's mark 1 after the readying;
  after its w2m_block T READY with the mark 0", the same for the other three; the row with no readying reads "T
  BLOCKED with the mark 0" before and after. The host's clearing: `threads::tests::a_new_or_loaded_threads_woken_mark_starts_clear`.
- **The lost-wakeup guest,** `tests/data/timed_waits/lost_wakeup` (100,000 rounds over two auto-reset events, INFINITE
  waits), held for card 5 by `run_loop::the_lost_wakeup_guest_ends_inside_its_limit_on_two_vcpus` and its row in
  `first-stop-2026-10-28.tsv`: one run on one vCPU and 20 on two, each ExitProcess 0 and "done", each 1,800,073
  instructions against `LOST_WAKEUP_LIMIT` 20,000,000, both vCPUs running threads (runs by vCPU 199,040 to 199,735 on
  vCPU 0 and 198,831 to 199,634 on vCPU 1).
- **Plant 1** "w2m_ready_thread: mark 1 after the readying, T BLOCKED, mark 1" (`w2m_block`'s reading of the mark
  removed): the judge failed with that row and the same for `w2m_signal`, `wake_blocked` and SCHEDULE_END, each "mark
  1 after the readying, T BLOCKED, mark 1", under "a readying before T's block, T then READY with its mark clear (none:
  BLOCKED), against [...]".
- **Plant 2** "w2m_ready_thread: mark 0 after the readying, T BLOCKED, mark 0" (`w2m_ready_thread`'s setting removed):
  the judge failed with that row alone.
- **Plant 3** "w2m_signal: mark 0 after the readying, T BLOCKED, mark 0" (`w2m_signal`'s setting removed): the judge
  failed with that row alone. The host's paths in turn: `wake_blocked`'s `mark_woken` removed, "wake_blocked: mark 0
  after the readying, T BLOCKED, mark 0"; SCHEDULE_END's removed, "SCHEDULE_END: mark 0 after the readying, T BLOCKED,
  mark 0"; each that row alone.
- **Plant 4** "left: (1, Some(8), 1) right: (0, Some(8), 0)" (`add_thread`'s and `load`'s clearing removed): the test
  failed, "the mark of a new thread's entry set before it was made, the thread taken, and the taken thread's mark set
  before its load". Each plant restored by `cp` and checked by `cmp`.
- **The amendment:** `tests/data/timed_waits/two_counters.c` committed from the scratch source, its code unchanged and
  its header rewritten; its `.exe` built `x86_64-w64-mingw32-gcc -O1 -nostdlib -Wl,-e,start ... -lkernel32` (GCC
  16.2.0, the flags that rebuild `three_counters.exe` with only the PE timestamp and checksum differing), and differing
  from the scratch `.exe` in those four bytes alone. On two vCPUs it answered ExitProcess { code: 0 } and
  `0000000009c0c45a` (163,628,122 ticks); its held line is in `first-stop-2026-10-28.tsv`, its one-vCPU output a clock
  reading (`batch_run.rs` `CLOCK_GUESTS`). The command, five runs on two vCPUs with the `w2m-vm` at `$W2M_VM`, from the
  repository root: `for i in 1 2 3 4 5; do echo '{}' | ./scripts/sign_and_run.sh "$W2M_VM"
  tests/data/timed_waits/two_counters.exe --vcpus 2 --output build/two_counters.txt --stop build/two_counters.stop >
  /dev/null 2>&1; printf '%s\t%s\n' "$(cat build/two_counters.txt)" "$(cat build/two_counters.stop)"; done`. Run with
  this tree's `target/release/w2m-vm`: 156,286,409 to 168,916,255 ticks, each ExitProcess { code: 0 }.

exit 0 in 405 s	default	2026-11-26T15:06:03Z	sprint 354	a85737f80968c86599a49fa7bb2fd7518adf7a05	392dec108339f9a7ce20cbd067844b7789537c9b6a5b5638d5c44a70e489ef0d	root=0cdb4c03a698534c00dedf3c4ba08daf92debfc9	rust=all python=0 samples=0	base=6e7276e6a382721048ed0f750870a988a2df18b8
