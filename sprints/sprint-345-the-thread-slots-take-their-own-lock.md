# Sprint 345 — The TLS and FLS slots take their own lock, not the runtime word

```yaml
---
id: 345
status: closed-partial
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

## amendment (2026-11-25)

The judge found a fault that predates this card. On two vCPUs, `thread_slots_race` stops at "guest runtime fault:
ELR ... ESR 0x2000000 (EC 0x0)" inside the code cache:
- 6 of 21 runs on HEAD's C and classes;
- 8 of 50 with this card's change;
- 0 of 42 on one vCPU.

The BLACKBOARD Decision "Card 345 amended for a fault its judge found" sets these:
- The probe's test is written and marked `#[ignore]`, naming card 346, so the gates do not run it. It is run by hand
  over at least 21 runs on two vCPUs and on one, with its pass rate and its `duplicate` and `stale` counts quoted.
  Plant 1 is judged on those runs.
- The fault is diagnosed by experiment, with no cause claimed beyond what is read: the block the faulting offset
  belongs to, the bytes at ELR at the stop and after, whether part 1 alone faults, and the other vCPU's state.
- The card closes `closed-partial`. Card 346 fixes the fault and lets the test into the gates.

## result

- **The proof, from `thread_storage.c` whole as it stood (179 lines).** The slots' shared state: `tls_used` (35),
  `fls_blocks` (38), `fls_callback` (39), `fls_used` (40), `tls_ready` (56), and each listed block's `value[]`. Stores
  and shared reads, service by service:
  - TlsAlloc (74-87): `tls_init` (67-72: reads `tls_ready`, stores "`tls_ready = 1;`", "`tls_used[0] = 1;`",
    "`tls_used[W2M_TLS_TAKEN] = 1;`"); reads `tls_used[i]` (77); "`tls_used[i] = 1;`" (80); "`*slot = 0;`" (82), the
    caller's TEB slot below 64 and above it the process-wide array at `W2M_TLS_EXPANSION_VA`; the caller's TEB
    expansion pointer (78-79). TlsFree (112-120): `tls_init`; reads `tls_used[index]` (115); "`tls_used[index] = 0;`"
    (116); "`if (slot) *slot = 0;`" (118).
  - FlsAlloc (122-132): reads `fls_used[i]` (124); "`fls_used[i] = 1;`" (125); "`for (struct fls_block *b =
    fls_blocks; b; b = b->next) b->value[i] = 0;`" (126), every thread's block; "`fls_callback[i] = callback;`" (127).
    FlsFree (170-179): reads `fls_used[index]` (172), the list, `fls_callback[index]` and each `b->value[index]`
    (173-174); "`fls_used[index] = 0;`" (176); each block's "`b->value[index] = 0;`" (177).
  - FlsSetValue (160-168): reads `fls_used[index]` (162); `fls_mine(1)` (43-55), on a thread's first set
    "`b->next = fls_blocks;`" and "`fls_blocks = b;`" (50-51), the new block zeroed (48-49) and "`*slot = b;`" (52)
    the caller's TEB; "`->value[index] = value;`" (166), the caller's block, which FlsAlloc and FlsFree also write.
  - FlsGetValue (134-143) and FlsGetValue2 (147-152): `fls_mine(0)` reads the caller's TEB; each reads
    `b->value[index]` (142, 150), which FlsAlloc and FlsFree write; `set_error` (58), the caller's TEB.
  The lock covers all of the shared state above. Calls: `w2m_heap_take` (46), from FlsSetValue's first set only, now
  under the slots' lock, slots then heap; heap.c's `hold` calls nothing while held (heap.c 20-26), so the heap's holder
  waits on no lock. `w2m_unimplemented` (47, 85, 115, 130, 172, 175) and `w2m_logged` (each service's return) are each
  now reached only after `let_slots_go`. The grep (`tls_used`, `fls_used`, `fls_blocks`, `fls_callback`, `tls_ready`,
  `0x17C8`, `0x1780`, `0x1480`, `W2M_TLS_TAKEN`, `W2M_TLS_EXPANSION_VA`, `k32_Tls`, `k32_Fls` over
  `tools/guest_runtime/` and `crates/`): no other runtime C. The host sets a new thread's own expansion pointer
  before it runs, "`(0x1780, teb + 0x2000),`" (`crates/w2m_vm/src/threads.rs` 1249), its TEB and no shared slot
  state; `crates/guest_layout/src/lib.rs` 162-163 define the two addresses.
- **The lock** (`thread_storage.c`): `static u32 slots_lock;` in the image's zeroed data beside the bitmaps,
  `hold_slots` (a plain load until it reads 0, then compare-and-swap of 0 for 1, acquire, then `tls_init`) and
  `let_slots_go` (a store of 0, release). Built image `build/guest_runtime_rust/258d8692….bin`, the lock word at
  0x7f00044a7dd8: `_k32_TlsAlloc` "`ldr w11, [x9, #0xdd8]; cbnz w11, 0x7f0004186be0; casa w11, w10, [x8]; cbnz w11,
  0x7f0004186be0`" at ...be0-...bec, the set-up's "`strb w11, [x10, #0xddc]; strb w11, [x9]; strb w11, [x9,
  #0x10]`" after it, "`stlr wzr, [x8]`" at ...c3c before "`bl _w2m_unimplemented`" at ...c48 and at ...cb0 before
  "`b _w2m_logged`" at ...cc0. Each of the seven inline: TlsFree `casa` ...df8, `stlr` ...e38 (then
  `bl _w2m_unimplemented` ...e44) and ...e8c; FlsAlloc ...ed0, ...f20 (bl ...f2c), ...f74; FlsGetValue ...fa0, ...ff8;
  FlsGetValue2 ...7040, ...7098; FlsSetValue `casa` ...7134, "`bl _w2m_heap_take`" ...719c under it, `stlr` ...71a8
  (then `bl _w2m_unimplemented` ...71b4), ...71f0 and ...7200; FlsFree ...7254, ...729c (bl ...72a8), ...72e0 (bl
  ...72ec), ...7318. No `bl` but `w2m_heap_take` lies between a `casa` and its `stlr`. TlsGetValue, TlsSetValue and
  TlsGetValue2 take none.
- **The table and the bridge.** `Lock::Slots` and `SLOTS_LOCK` (`services.rs`);
  `the_seven_services_of_lock_class_slots_are_the_tls_and_fls_slots` holds class slots to the seven, each defined in
  `thread_storage.c` and of class slots in clock modes 0, 1 and 2, TlsGetValue and TlsSetValue none, TlsGetValue2 the
  word. `a_bridge_of_lock_class_none_takes_no_runtime_word` (`guest.rs`) holds a slots-class bridge to class none's
  word for word, traced and untraced, timed and not. `DEPARTURES` names `thread_storage.c` with Sprint 345.
- **Before the work, the judge's reach:** the seven of class slots, the C unchanged, five runs on two vCPUs, each
  "stopped at HostRequest op 4: TlsFree argument form 0x5" (0x3, 0x5, 0x2, 0x5 in the others), no row written: two
  threads given one index, the second TlsFree finding it free. On one vCPU, two runs, every row Windows', `tls.overlap`
  191 and 191.
- **The Windows run:** run 45763850660 (`windows-2025`, Server 2025 10.0.26100.33828, an Intel Xeon Platinum 8573C),
  `probes/thread_slots_race` as committed 3522feb7f, three runs, each exit 0 in 0.262 to 0.274 s; its files in
  `store/probe-runs/45763850660/`, its binary `store/binaries/1c778788fa46….exe`, the run's `probe.sha256`. Alike but
  `tls.overlap`: `{"s":"tls.wait","ret":0}`;
  `{"s":"tls.a","rounds":20000,"out_of_indexes":0,"out_of_range":0,"duplicate":0,"free_failed":0}` and the same for
  `b`; `{"s":"fls.alloc","allocated":1,"event":1}`; `{"s":"fls","threads":16,"set_ok":16,"freed":1,"stale":0,"wait":0}`.
  `tls.overlap` 16,625 and 16,626, 17,069 and 17,069, 15,611 and 15,611.
- **The fault (the amendment).** On two vCPUs the probe stops in some runs at "HostFailure op 5: guest runtime fault:
  ELR 0x7f0000068134 ESR 0x2000000 (EC 0x0), not in translated code or not a data abort". Counts on two vCPUs: HEAD's
  C and classes 6 of 21; the seven on the word with the lock 12 of 21 and 23 of 50 (20 of them in part 1); this card's
  change 4 of 21, 8 of 50, 6 of 30 and 5 of 70, every one after the `fls.alloc` row, so part 1 never faulted in 171
  runs; one vCPU 0 of 42. Read by a temporary report in the fault's handler (`host_request.rs`, from a copy, restored
  by `cp`, checked by `cmp`), six faults: the word at ELR "`f1000af7`" (`subs x23, x23, #2`, the budget's count) at
  the stop and the same 50 ms later, the words before it the previous block's tail; ELR the start of the newest
  block, the cursor 0x54 past it (`VMC_CURSOR` 0x68064 for ELR 0x68010), its reconstruction entry "host
  0x7f0000068010 x86 0x140001c91"; no in-VM compile logged (`VMC_LOG_COUNT` 0). x86 0x140001c91 is `fls_side`'s
  `movl 0x13a9(%rip), %ecx` after "`callq *0x3d7(%rip)`" at 0x140001c8b, the return from WaitForSingleObject on
  `fls_freed`, before FlsGetValue: every fault was there. The other vCPU, read by a temporary line in the run loop
  (`run.rs`, restored the same way), five faults: "pc 0x7f0000001880 x17 0x140001c9d" in each (vCPU 1 faulting three
  times, vCPU 0 twice; a sixth fault printed no line for the other vCPU), x17 the x86 address after the same thread function's FlsGetValue call. The dispatcher's words
  there, read at the stop: "`d2921010 f2a13b70 f2cfe010 b9400212 b5fffff2`" at 0x1870-0x1880, `mov x16` of
  0x7f0009db9080 (the word's offset 0x80, `RUNTIME_LOCK`), "`ldr w18, [x16]`" at 0x187c and "`cbnz x18`" back to it
  at 0x1880: the wait loop of a take of the runtime word, `monitor_code.rs` 128-131's form. So the other vCPU was in
  the dispatcher waiting for the runtime word, not in guest code. Part 1 alone (a scratch copy of the probe ending
  after the `tls.overlap` row, built with `x86_64-w64-mingw32-gcc`, not committed): 40 of 40 runs on two vCPUs
  `ExitProcess { code: 0 }`, `duplicate` 0 in all 80 rows. No cause is claimed beyond these readings.
- **The test**
  `run_loop::two_threads_taking_and_freeing_tls_and_fls_slots_at_once_never_share_one_on_two_vcpus`, marked
  `#[ignore = "Sprint 346: the code-cache fault, ELR in MONITOR_VA + 0x640a0..0x68134, EC 0"]`: the Windows run's three bodies alike but `tls.overlap`; the probe through `w2m-vm` on two
  vCPUs, then on one, each `ExitProcess { code: 0 }` and its rows but `tls.overlap` equal to `body-1.jsonl`'s; on two,
  both counts above 1,000. Run by hand, `--run-ignored only`, 25 times: 19 passed, 6 failed on the fault, "thread_slots_race
  on 2 vCPUs; its rows ...", ELR 0x7f0000068014 (4), 0x…805c, 0x…802c. In all 25 runs `duplicate` 0 on both sides on two
  vCPUs, and in the 19 on one; `stale` 0 in every run that wrote the `fls` row. `tls.overlap` on two vCPUs 17,720 to
  19,537, on one 191 and 191 in each.
- **The floor,** 1,000: five times the one-vCPU figure, 191 in all 21 measured runs and the 19 passing ones, and under
  a third of the lowest two-vCPU figure but one, 3,072 in 71 runs (3,072 to 19,633); one run read 0 and 0, one thread's
  rounds done before the other's began.
- **The walk fixture:** `target/release/image_walk` on the binary, 752 instructions, all walked, 188 block starts, no
  table, none undecodable; put in by `jq` in the file's order, the `.exe` keys sorted, then the `.dll`, that order first
  checked to reproduce the stored file byte for byte (`cmp`); the diff the one entry. The doc comment
  (`crates/image_walk/tests/fixtures.rs`) made 283 images and 642,646 instructions, summed by `jq`; 116 tables.
- **Plant 1** "thread_slots_race on 2 vCPUs; its rows ; stderr ... left: \"HostRequest op 4: TlsFree argument form
  0x1\"": `hold_slots`' loop removed and `let_slots_go` made `(void)slots_lock;`, the seven of class slots; three runs
  of the test, each failing on its stop before any row, "TlsFree argument form 0x1", "0x7", "0x2": the second TlsFree
  of an index two threads were given. It fails on the stop, not on `duplicate` above 0 as the card foresaw: the probe
  writes part 1's rows after both threads end. Restored by `cp`, checked by `cmp`. A variant, FlsSetValue alone
  unlocked: 11 runs reached part 2 and each read `stale` 0, so part 2 did not catch an unlocked first set in 11 runs;
  it is a weak judge of the FLS list's link race.
- **What waits on card 346:** the code-cache fault's fix, and the probe's test let into the gates; built here are
  the lock, the class, the bridge test, `DEPARTURES`, the stored run, the walk fixture and the ignored test.
- **Plant 2** "assertion `left == right` failed: the services of lock class slots left: [\"KERNEL32!FlsAlloc\",
  \"KERNEL32!FlsFree\", \"KERNEL32!FlsGetValue\", \"KERNEL32!FlsGetValue2\", \"KERNEL32!TlsAlloc\",
  \"KERNEL32!TlsFree\"]": FlsSetValue moved from `SLOTS_LOCK` to `NO_LOCK` (`[&str; 8]`, `[&str; 6]`), by `sed`, one
  line for one; the none-class test failed too, "the services of lock class none left: [\"KERNEL32!FlsSetValue\",
  ...". Restored by `cp`, checked by `cmp`.
- **Files:** `tools/guest_runtime/thread_storage.c`, `crates/guest_runtime/src/services.rs`,
  `crates/guest_runtime/tests/python_image.rs`, `crates/w2m_vm/src/guest.rs` (its bridge test, not in the card's
  list), `crates/w2m_vm/tests/run_loop.rs`, `crates/image_walk/tests/fixtures.rs`,
  `store/translation-cache/walk-fixtures-2026-10-30.json`, `store/binaries/1c778788fa46….exe`,
  `store/probe-runs/45763850660/`.

- **The gates:** the selection "Rust tests: all"; nextest "379 tests run: 379 passed (2 slow), 2 skipped"; the timing stage "passed, 1035 ns an exit at load 3.45"; "gates: all passed". The first run's row (590 s, 06:18:42Z) stays, its tree before the ignore string's change. `exit 0 in 575 s	default	2026-11-26T06:31:33Z	sprint 345	3522feb7f53970e981de3ad045f246eb6a9eefc9	c955cabd218a3b72e14eae5e2dc23a343c9bfb1730514b1528ffcc7c78b24688	root=5b3aa609f1a36eda1bdfef8fd00da5042da77046	rust=all python=0 samples=0	base=5d66f3542ed37f871c94a871200d928416169e08`
