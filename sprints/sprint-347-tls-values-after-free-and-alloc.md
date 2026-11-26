# Sprint 347 — Another thread's TLS value after TlsFree and TlsAlloc, as Windows gives it

```yaml
---
id: 347
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's fifteenth review, Low 1, recorded as the Decision "TlsFree's reach settled by a Windows row". It found
that w2m's TlsFree clears the freed index in the calling thread's TEB alone (`tools/guest_runtime/thread_storage.c`,
`k32_TlsFree`). TlsAlloc does the same: it clears only the caller's slot (`k32_TlsAlloc`, `*slot = 0`). Microsoft
Learn documents TlsAlloc as "When a TLS index is allocated, its storage slots are initialized to NULL" (TlsAlloc,
Remarks), and the return-value section reads "The slots for the index are initialized to zero". The TlsFree page says
nothing of what a free does to any thread's value. So a thread that set an index, which another thread then freed and
allocated again, may read its old value in w2m where Windows reads 0.

- **The probe.** `probes/tls_free_reach/` is committed with this card and compiles against the mingw-w64 headers. A
  worker sets its value at an index, and so does the main thread. The main thread frees the index, and the worker
  reads it, along with the last error. The main thread allocates again and notes whether it got the same index. The
  worker reads again, and the main thread reads its own slot. Each step waits on an event, so no row depends on
  scheduling. The pass runs once at a TEB slot (below 64) and once at an expansion slot (64 and up).
- **Its Windows rows.** Run it on the Windows runner by `.github/workflows/windows-probe.yml`, three runs. Keep the
  binary and the runs' files and add the walk fixture's entry, as card 345 did. The rows must read alike in the three
  runs; if they do not, stop and say so. Before any change, run the probe through `w2m-vm` on one vCPU and quote where
  its rows differ from Windows'.
- **The change, as the rows require.** Make TlsAlloc and TlsFree give every thread's slot what Windows gives it,
  zeroing an index in every live thread's TEB, or its expansion array, where Windows reads 0:
  - do it under the slots' lock (Sprint 345), never holding it across `w2m_unimplemented` or `w2m_logged`;
  - find how the runtime C reaches every live thread's TEB (`tools/guest_runtime/threads.c`, the run table) and quote
    it;
  - a slot another thread reads at the same moment with TlsGetValue, which takes no lock, is that application's race,
    as it is on Windows. State that with the lines.

  If the rows show Windows leaving the worker's value, change nothing for that call and record the row as the
  contract.
- **The judge.** A `run_loop.rs` test runs the probe through `w2m-vm` on one vCPU and on two, and holds every row to
  Windows'.
- **Not here:**
  - FLS, whose FlsFree already clears every thread's block;
  - any other lock class.

## prerequisites

- 346 (closed)

## context_files

- `probes/tls_free_reach/tls_free_reach.c` and `probe.json` (this card's commit).
- `tools/guest_runtime/thread_storage.c` whole (the slots' lock, Sprint 345), and `tools/guest_runtime/threads.c`'s run
  table and thread records.
- `crates/w2m_vm/tests/run_loop.rs`: card 345's test, `program_through_w2m_vm`.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, already naming `thread_storage.c`; extend its reason).
- `.github/workflows/windows-probe.yml`; card 345's result
  (`sprints/sprint-345-the-thread-slots-take-their-own-lock.md`) for the Windows run, the stored binary and the walk
  fixture's entry.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; what two kernel32 services write.

### Invariants

- Every existing gate passes unchanged, the family runs among them (`contracts/windows.thread_storage.*`).
- The slots' lock is never held across `w2m_unimplemented` or `w2m_logged`.

## artifact contract

### Files modified

- `tools/guest_runtime/thread_storage.c`, `crates/w2m_vm/tests/run_loop.rs`, `crates/guest_runtime/tests/python_image.rs`,
  `crates/image_walk/tests/fixtures.rs` (its totals), `store/translation-cache/walk-fixtures-2026-10-30.json` (one
  entry), `store/binaries/` (the probe's binary), `store/probe-runs/<run id>/`.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The Windows run's id, its runner and CPU, and its three runs' rows.
  - The probe's rows through `w2m-vm` before the change, each difference named.
  - After the change, the rows on one vCPU and on two, equal to Windows'.
  - The lines that reach every live thread's TEB.
- **Planted faults:**
  - (1) TlsAlloc clearing the caller's slot alone, as before. The test fails on `worker_after_again`, if Windows
    reads 0 there.
  - (2) TlsFree's clearing limited to the caller, if the rows require more. The test fails on `worker_after_free`.

  If a plant has no row to fail because Windows leaves the value, say so and name it. Before the change, read each
  judge's reach: the test failing on the rows w2m gives today.

## done criteria

Another thread's TLS value after TlsFree and after TlsAlloc of the same index is what Windows gives, at a TEB slot and
at an expansion slot, on one vCPU and on two.

## self-review checklist

- [x] Scope names one verifiable property: another thread's TLS value after a free and a new allocation, as Windows.
- [x] context_files cite the probe, the slot C and the run table, the judge's form, the Windows run's procedure.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: Windows' rows, w2m's rows before and after; two numbered plants, their reach read.
- [x] Sweet spot: `thread_storage.c` and a test, with the stored probe; one concept, a TLS index's reach.

## amendment (2026-11-26)

Builder-347's experiment found a defect that stops a run, so it is fixed in this card (the user's Decision of
2026-10-18). The first thread's expansion pointer (TEB+0x1780) is set only by its own TlsAlloc past 63
(`thread_storage.c` 143-144), while created threads get theirs from the host (`threads.rs` 1249). So when another
thread takes an index of 64 or more, the first thread's TlsSetValue of it stops at "HostRequest op 4: TlsSetValue
argument form 0x40", on one vCPU and on two.

- `probes/tls_expansion_first/` is committed with this amendment. Run it on the Windows runner, three runs, keep the
  binary and the files, and add its walk fixture's entry.
- Fix it so that every thread can use an index any thread allocated, as Microsoft documents for TlsAlloc: "After a TLS
  index has been allocated, each thread of the process can use it to access its own TLS storage slot."
- Before writing the fix, list every path that reads TEB+0x1780 or calls `tls_slot`, and each thread that can lack the
  pointer, and close each one.
- A `run_loop.rs` test holds its rows on one vCPU and on two to Windows'.
- Plant 3: the fix taken out. The test fails on the stop.

## result

- **The Windows run:** run 45780352907 (`windows-2025`, Server 2025 10.0.26100.33828, an AMD EPYC 9V45),
  `probes/tls_free_reach` as committed 1a9054451, three runs, each exit 0 in 0.0204, 0.0128 and 0.0177 s; its files in
  `store/probe-runs/45780352907/`, its binary `store/binaries/0dd9cc91c64a….exe`, the run's `probe.sha256`. The three
  bodies are alike byte for byte (`cmp`):
  `{"s":"teb_slot","index_at_least_64":0,"freed":1,"worker_after_free":0,"worker_after_free_error":0,"same_index_again":1,"worker_after_again":0,"main_after_again":0,"wait":0}`;
  `{"s":"expansion","found":1}`; and `expansion_slot` the same as `teb_slot` but `"index_at_least_64":1`. Windows
  zeroes the worker's value at the index when the main thread frees it, at a TEB slot and at an expansion slot, and it
  reads 0 again after the index is taken anew. No row shows Windows leaving the worker's value, so both calls change.
- **Before the change,** the stored binary through `target/release/w2m-vm` built from 1a9054451, on one vCPU and on
  two, each `ExitProcess { code: 0 }` and the same rows:
  `{"s":"teb_slot",...,"worker_after_free":2827,"worker_after_free_error":0,"same_index_again":1,"worker_after_again":2827,"main_after_again":0,"wait":0}`,
  and the same at `expansion_slot`. Four fields differ from Windows': `teb_slot.worker_after_free`,
  `teb_slot.worker_after_again`, `expansion_slot.worker_after_free` and `expansion_slot.worker_after_again`, each 2827
  (0xB0B, the probe's `WORKER_VALUE`) against 0. Every other field is Windows', `main_after_again` 0 included.
- **The judge's reach, before the change:** the test failed on one vCPU,
  `tls_free_reach on 1 vCPUs, fields unlike Windows' ["\"teb_slot\".worker_after_again: 2827 against 0", "\"teb_slot\".worker_after_free: 2827 against 0", "\"expansion_slot\".worker_after_again: 2827 against 0", "\"expansion_slot\".worker_after_free: 2827 against 0"]`
  (the row's name then printed with its JSON quotes; the message now prints its text).
- **How the runtime C reaches every live thread's TEB:** the run table, which the host fills as it makes each thread:
  "`write_u64(mem, at + E_TEB, thread.teb);`" before "`write_u64(mem, RUN_TABLE + RT_COUNT, n + 1);`"
  (`crates/w2m_vm/src/threads.rs` 170-172), an entry never reused and an ended thread's status ENDED. `threads.c`
  reads another thread's TEB through its entry, "`return (struct w2m_thread_scratch *)(e->teb +
  W2M_THREAD_SCRATCH_OFF);`" (`scratch_of`, 70-71). A created thread's expansion array is its own, "`(0x1780, teb +
  0x2000),`" (`threads.rs` 1249); the first thread's is `W2M_TLS_EXPANSION_VA`, which before the amendment's fix its
  own TlsAlloc above 63 alone gave it.
- **The change** (`thread_storage.c`, line numbers of the final tree): `tls_slot_in(teb, index)` (100-104), the slot
  in a given TEB, `tls_slot` (106) now its call with the caller's; `tls_slot_given` (116-120, the amendment's fix,
  below); `struct run_head` (125), the entry's eight words before its registers, held by "`_Static_assert(sizeof(struct
  run_head) == W2M_RUN_X_OFF, ...)`" (126) to the layout; and `tls_clear_everywhere` (140-152): "`u64 *mine =
  tls_slot_given((u8 *)w2m_teb(), index); if (mine) *mine = 0;`", then "`u64 n = RUN_COUNT;`" and for each entry "`u64
  teb = e->teb; if (!teb || e->status == RUN_ENDED) continue;`", "`u64 *slot = tls_slot_given((u8 *)teb, index); if
  (slot) *(volatile u64 *)slot = 0;`". TlsAlloc calls it at 159 and TlsFree at 199, each between its `hold_slots` (155,
  192) and its `let_slots_go` (160, 200); the helper calls nothing, so the lock is never held across
  `w2m_unimplemented` or `w2m_logged`. **The race, as the lines state it:** each store is one aligned 8-byte store, single-copy atomic, made with
  no lock that the other thread's TlsGetValue or TlsSetValue takes, so a thread reading or writing the index at that
  moment sees its value or 0: that application's race on a freed or newly taken index, as on Windows.
- **After the change:** the test passed in 5.6 s, the rows on one vCPU and on two each equal to `body-1.jsonl`, byte
  for byte, `worker_after_free` and `worker_after_again` 0 in both passes.
- **The test** `run_loop::another_threads_tls_value_after_tlsfree_and_tlsalloc_is_windows_on_one_and_two_vcpus`: the
  Windows run's three bodies alike; the probe through `w2m-vm` on one vCPU, then on two, each `ExitProcess { code: 0
  }`, its rows by name against Windows', every field held and each unlike field named, then the whole text equal.
- **Plant 1** reached no judge (plants 1 and 2 run on the tree before the amendment's fix, their lines then 147 and
  187, now 159 and 199): TlsAlloc clearing the caller's slot alone ("`*tls_slot(i) = 0;`" at 147, by `sed`, one
  line for one) passed the test, every row Windows', since TlsFree's clear had already made the worker's value 0
  before the index was taken again; no row of this probe reads TlsAlloc's reach while TlsFree reaches every thread.
  Replaced: plants 1 and 2 together, the slot left at the worker's value when TlsAlloc runs (TlsFree's line 187 made
  "`{ u64 *slot = tls_slot(index); if (slot) *slot = 0; (void)tls_clear_everywhere; }`", the helper still named so
  the runtime builds under `-Werror`), failed "tls_free_reach on 1 vCPUs, fields unlike Windows'
  [\"teb_slot.worker_after_again: 2827 against 0\", \"teb_slot.worker_after_free: 2827 against 0\",
  \"expansion_slot.worker_after_again: 2827 against 0\", \"expansion_slot.worker_after_free: 2827 against 0\"]",
  where plant 2 alone reads `worker_after_again` 0: TlsAlloc's every-thread clear is what zeroes it there. A first
  form of the replacement, without `(void)tls_clear_everywhere;`, stopped at the runtime's build, "error: unused
  function 'tls_clear_everywhere' [-Werror,-Wunused-function]", before any row, and is not counted. Restored by `cp`,
  checked by `cmp`, after each.
- **Plant 2** "tls_free_reach on 1 vCPUs, fields unlike Windows' [\"teb_slot.worker_after_free: 2827 against 0\",
  \"expansion_slot.worker_after_free: 2827 against 0\"]": TlsFree's clearing limited to the caller (line 187 made
  "`{ u64 *slot = tls_slot(index); if (slot) *slot = 0; }`", by `sed`, one line for one). Restored by `cp`, checked
  by `cmp`.
- **The amendment's experiment** (a scratch probe built with `x86_64-w64-mingw32-gcc` in the session's scratch
  directory, no tracked file changed, after the first gates run): a worker takes indices until one is 64 or more; the
  first thread sets its value there. Through `w2m-vm` on one vCPU and on two, each stopped at "HostRequest op 4:
  TlsSetValue argument form 0x40", TlsSetValue's "`if (!slot) w2m_unimplemented(W2M_API_TlsSetValue, index);`".
- **The amendment's Windows run:** run 45783238565 (`windows-2025`, Server 2025 10.0.26100.33828, an AMD EPYC 7763),
  `probes/tls_expansion_first` as committed 003a0c1a9, three runs, each exit 0 in 0.0275, 0.0171 and 0.0168 s, the
  bodies alike byte for byte (`cmp`): `{"s":"expansion_first","wait":0,"high_at_least_64":1,"set":1,"got":1511,"error_after_get":0}`.
  Its files in `store/probe-runs/45783238565/`, its binary `store/binaries/e7bc99857b39….exe`.
- **Every reader of TEB+0x1780 and every caller of `tls_slot`** (grep of `0x1780`, `TEB_TLS_EXPANSION`,
  `TlsExpansion`, `tls_slot` and `TLS_EXPANSION` over `tools/` and `crates/`, C and Rust): in the runtime only
  `thread_storage.c`: `tls_slot_in` (102) reads it, and TlsAlloc's own set-up wrote it ("`if (i >= TLS_SLOTS &&
  *(u64 *)((u8 *)w2m_teb() + TEB_TLS_EXPANSION) == 0)`", then 144-145, now removed); `tls_slot` is called by TlsGetValue
  (174), TlsSetValue (185) and TlsGetValue2 (246), and `tls_slot_given` by `tls_clear_everywhere` (141, 149). In the
  host, `threads.rs` 1236 and 1249 alone write it, for a created thread. (`known_folders.c` 38's `0x1780` is a GUID's
  word.) **The threads that can lack it:** the host makes a TEB in two places only: a created thread's, allocated with
  "`(0x1780, teb + 0x2000),`" (`threads.rs` 1236-1249), and each process's first thread's, "`teb: PROCESS_VA +
  TEB_OFF,`" (`run.rs` 958; `process.rs` 324, whose TEB fields at 372-383 write no 0x1780). So the first thread of each
  process, at `W2M_TEB_VA`, is the one thread without an array, child processes' first threads among them.
- **The fix:** `tls_slot_given` (116-120): "`if (index >= TLS_SLOTS && !*expansion && teb == (u8 *)W2M_TEB_VA)
  *expansion = W2M_TLS_EXPANSION_VA;`", then the slot. `tls_clear_everywhere` finds every live thread's slot by it,
  under the slots' lock, whenever an index is allocated or freed, so once any index past 63 is taken the first thread,
  if live, has its array; TlsAlloc's caller-only set-up is removed. Each closed: TlsSetValue (185) of an allocated index
  past 63 finds a slot on every thread, the first by the fix and the others by their creation; TlsGetValue (174) and
  TlsGetValue2 (246) read the value set or 0; TlsSetValue's stop now marks an index past 63 when none was ever
  allocated, not in the rows. The first thread's array, `W2M_TLS_EXPANSION_VA`, is zero until given, nothing else
  writing it.
- **The amendment's test**
  `run_loop::the_first_thread_uses_an_expansion_index_another_thread_allocated_on_one_and_two_vcpus`: the three bodies
  alike; the probe through `w2m-vm` on one vCPU and on two, each `ExitProcess { code: 0 }` and its row equal to
  `body-1.jsonl`. Passed in 0.75 s; with it, run together, the slots test and this card's first test passed.
- **Plant 3** "assertion `left == right` failed: tls_expansion_first on 1 vCPUs; its rows ; ... left: \"HostRequest
  op 4: TlsSetValue argument form 0x40\" right: \"ExitProcess { code: 0 }\"": the fix taken out, the array given only
  to the caller as before (line 118's "`teb == (u8 *)W2M_TEB_VA)`" made "`teb == (u8 *)w2m_teb())`", by `sed`, one line
  for one). Restored by `cp`, checked by `cmp`.
- **The walk fixture:** `target/release/image_walk` on each binary: `0dd9cc91c64a….exe` 441 instructions, all walked,
  111 block starts; `e7bc99857b39….exe` 257, all walked, 68 block starts; neither has a table or an undecodable
  instruction. Each put in by `jq` in the file's order, the `.exe` keys sorted, then the `.dll`, that order first
  checked each time to reproduce the stored file byte for byte (`cmp`); the diff the two entries. The doc comment
  (`crates/image_walk/tests/fixtures.rs`) made 285 images and 643,344 instructions, summed by `jq`; 116 tables.
- **Files:** `tools/guest_runtime/thread_storage.c`, `crates/w2m_vm/tests/run_loop.rs`,
  `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`' reason for `thread_storage.c`),
  `crates/image_walk/tests/fixtures.rs`, `store/translation-cache/walk-fixtures-2026-10-30.json`,
  `store/binaries/0dd9cc91c64a….exe`, `store/binaries/e7bc99857b39….exe`, `store/probe-runs/45780352907/`,
  `store/probe-runs/45783238565/`.
- **The gates:** the first run's row (685 s, 08:24:27Z) stays, its tree before the amendment, and the builder's final run's (383 s, 08:36:37Z, root=dae28fb87), its tree before the supervisor named commit 003a0c1a9 in `signal_report`'s `BETWEEN_SPRINTS`. The run on the closing tree, by the supervisor: the selection "Rust tests: all"; "gates: steps 369 s of the run's 374 s, unplaced 5 s"; "gates: all passed". `exit 0 in 374 s	default	2026-11-26T09:00:50Z	sprint 347	003a0c1a9968bd1195907498b93f27c9c33d3ac7	29a59cbc4eed949381f31633850d6ed79ac8a19525d50b52cf5dec38ce082443	root=78564bdb3300c08e876d0a1b1412b8f6b339e49c	rust=all python=0 samples=0	base=abf9beaa9547aa604b755179ca812c345c748430`
