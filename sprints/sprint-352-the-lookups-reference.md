# Sprint 352 — A handle's lookup holds a reference on its object, and every blocked wait keeps its objects

```yaml
---
id: 352
status: done
phase: 6
pass_kind: functional
---
```

## scope

Card 2 of `docs/design-handles-and-sync-objects-2026-11-26.md` ("The cards that follow", lines 344-353; the rule at
166-231). Sprint 350 built the two counts, `handles` and `refs`, in `struct object`. It gave a reference to the waiter
entries of a semaphore, an auto-reset event, a mutex and a wait for all.

This card makes the rule whole, still under the runtime word:
- **The lookup.** `object_of` takes a reference, and every caller drops it when done with the object. There are 13
  calls in `tools/guest_runtime/sync_objects.c`. Its users in `file.c` (264, 345-352) and `processes.c` go through
  `w2m_object_*` and drop likewise. The design names this card as one concept over three files.
- **Every blocked wait.**
  - A wait of any kind that blocks keeps its objects' references in its entry: a manual-reset event, a thread, a
    process, and a wait for any of several among them.
  - Its retry, `WaitForMultipleObjects` included, reads its objects from the entry, not by its handles again.
  - So a handle closed during any wait leaves the wait on its object, and the entry is FREE only at `refs` 0.
- **WaitForMultipleObjects on a closed handle.** It stops the run today (`sync_objects.c` 461: `if (!os[i])
  w2m_unimplemented(...)`). Microsoft Learn's WaitForMultipleObjects documents WAIT_FAILED with the last error for an
  invalid handle; quote the page. Answer WAIT_FAILED and ERROR_INVALID_HANDLE, as `wait_one` does at 413-418, for a
  handle that names no object when the call begins. Hold the answer to a Windows row.
- **The Windows rows.** A probe committed by the Supervisor as a "Sprint 352 card amendment:" commit (Sprint 349's
  kind). Write it under `probes/` and send its path.
  - **Defined, compared.** WaitForMultipleObjects of a closed handle beside a valid one, its return and last error.
  - **Recorded, not compared.** A worker waits on a manual-reset event. The main thread duplicates its handle, closes
    the original, sets the event through the duplicate, and records the worker's return and last error (the design's
    row for card 2; Microsoft Learn leaves a wait on a handle closed during it undefined).
  - A wait for any of a closed event and an open one, the open one then set, its index returned; recorded, not
    compared.
- **The judge.** A `run_loop.rs` test runs the probe on one vCPU and on two, holding the defined rows to Windows' and
  the recorded ones to their documented range. It must fail on today's code: the stop at 461, or a wait that does not
  end.
- **The family runs on two vCPUs (Sprint 351) and the batch run pass unchanged.**
- **Not here:** the handle table's lock, the sync objects' lock, the woken mark.

## prerequisites

- 351 (closed)

## context_files

- `docs/design-handles-and-sync-objects-2026-11-26.md` 135-365 (the design and the cards).
- `tools/guest_runtime/sync_objects.c` whole; `tools/guest_runtime/file.c` 255-270 and 340-360; the `w2m_object_*`
  calls in `tools/guest_runtime/processes.c` (`grep -n 'w2m_object_' tools/guest_runtime/processes.c`).
- `probes/wait_closed_handle/` and its test in `crates/w2m_vm/tests/run_loop.rs`
  (`a_semaphore_made_after_a_waited_handle_is_closed_keeps_its_unit_on_one_and_two_vcpus`) for the probe's and the
  judge's form.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, naming `sync_objects.c`; extend its reason, and name
  `file.c` and `processes.c` if they change).
- `.github/workflows/windows-probe.yml`; card 350's result for the Windows run, the stored binary and the walk
  fixture's entry.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; when an object's entry is freed, and what a wait reads.

### Invariants

- The one-vCPU and two-vCPU family runs, the batch run and Sprint 350's tests pass unchanged.
- No lock is added; every change runs under the runtime word.

## artifact contract

### Files modified

- `tools/guest_runtime/sync_objects.c`, `file.c` and `processes.c` as the drops need, `crates/w2m_vm/tests/run_loop.rs`,
  `crates/guest_runtime/tests/python_image.rs`, the probe's stored binary, runs and walk fixture entry, and any held
  line the gates name.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - Each `object_of` call and the line that drops its reference.
  - Each wait's return paths and what each drops.
  - Microsoft Learn's WaitForMultipleObjects text quoted.
  - The Windows run's rows.
  - The test's rows on one vCPU and two, before the change and after.
- **Planted faults:**
  - (1) One caller's drop removed, a reference left. A test shows the entry never freed: a create after enough
    open-and-close cycles fails, or the probe's rows differ.
  - (2) The retry of WaitForMultipleObjects reading its handles again, not its entry. The probe's test fails.
  - (3) The stop at 461 restored. The probe's test fails on the stop.

  Before the change, read each judge's reach.

## done criteria

Every lookup of a sync object holds a reference until its caller is done, every blocked wait keeps its objects until
it ends, a handle closed during any wait leaves the wait on its object, and WaitForMultipleObjects of a closed handle
answers as Windows does instead of stopping the run.

## self-review checklist

- [x] Scope names one verifiable property: the lookup's reference and every wait's, as the design's card 2.
- [x] context_files cite the design, the runtime C, the probe and judge form, and the Windows run's procedure.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the drops, the paths, the rows; three numbered plants, their reach read.
- [x] Sweet spot: the design names it one concept over three files, as here.

## amendment (2026-11-26)

The gates' exit ledger (`exit_ledger::each_process_parts_from_the_frozen_host_where_held`) parted from the frozen host
at main row 31, not 114. Builder-352's experiment found the cause: GetCommandLineA's return value. The pointer is
`&command_line_a`, `console.c`'s static buffer. It moved from 0x7F000446E120 to 0x7F000446F120 because the runtime
image's `__text` grew by 1,396 bytes, past a page with 660 bytes of slack, so `__DATA` moved up one page. The guest reads
the same string through the pointer, and Windows defines only what it points to. Any later growth of the runtime C
crosses the same boundary.

- **The judge's change, in this card.** The exit ledger hashes the process area with each 8-byte word that falls
  inside the runtime image's range renamed to the frozen image's address of the same symbol and offset, as
  `batch_run.rs` names runtime-image addresses as `name+0xoffset` against the frozen image (Sprint 272, its lines
  211-217). Change `exit_ledger.rs` or `run.rs`, whichever holds the hash, and record which.
- **Its judge.** A test holds that a runtime-image pointer renamed by symbol and offset hashes the same at two image
  layouts, and a word outside the image is hashed unchanged.
- **Plant 4.** The renaming taken out: the exit ledger parts at main row 31 again.
- **No held line is added to `store/exit_ledger/differences-2026-10-29.tsv` for this move.**

## result

- **The lookup** (`tools/guest_runtime/sync_objects.c`): `object_of` takes a reference (`o->refs++`), and
  `object_done` drops it. There are 12 calls and the definition, 13 lines in all, each now with its drop:
  - `w2m_object_is_token`, `w2m_object_is_open`, `w2m_object_mapping_of`, `w2m_object_kind`, `w2m_object_thread_of`:
    each copies its value out and calls `object_done` before it returns.
  - SetEvent, ResetEvent: one lookup each (before, `must` looked the handle up a second time), dropped before the log.
  - ReleaseSemaphore, ReleaseMutex, GetThreadId, GetProcessId: `must(api, h, object_of(h), ...)`, now given the
    lookup's object, with a drop on every return (each FALSE return included).
  - `wait_one` and WaitForMultipleObjects: the lookup's reference is dropped at the return, or kept by the entry at a
    block.
- **file.c and processes.c are unchanged.** Their `w2m_object_*` calls answer values (a kind, a count, a flag
  pointer into `handle_flags[]`, a descriptor), never an object. Each accessor drops its own reference before it
  returns, so no reference leaves `sync_objects.c`. `w2m_object_close` and `w2m_object_flags` use `slot_of` and take
  none.
- **Every blocked wait keeps its objects.** `waiter_add` keeps the lookup's reference and adds none of its own.
  - `wait_one` enters an entry for any kind that blocks (before, only a semaphore, an auto-reset event or a mutex).
    Its paths: WAIT_FAILED with ERROR_INVALID_HANDLE on a first try only; a grant, a signaled object (taken first) or
    a timeout drop the entry on a retry and the lookup's reference on a first try; a block keeps them.
  - WaitForMultipleObjects reads a retry's objects from its entries: a wait-all from `all_waiters[a].objects`, a
    wait-any from `entries_of`, which keys each entry by its slot. A first try's references enter one entry per
    object, of any kind. Its paths: WAIT_FAILED drops the references taken so far; a grant, a signaled object or a
    timeout drop the entry (`all_drop` or `waiters_drop`) or the lookups' references (`objects_done`); a block keeps
    them.
  - `grant` returns at once for a kind that is not handed over, so a manual-reset event's entry is never granted.
- **WaitForMultipleObjects of a closed handle:** the stop at 461 is now WAIT_FAILED and ERROR_INVALID_HANDLE. A handle
  below the objects' base still stops the run. Microsoft Learn, WaitForMultipleObjects: "WAIT_FAILED (DWORD)0xFFFFFFFF:
  The function has failed. To get extended error information, call GetLastError." and "If one of these handles is
  closed while the wait is still pending, the function's behavior is undefined."
- **The Windows run:** run 45813040124, `probes/wait_closed_kinds` as committed in 191ce59fb.
  - Runner `windows-2025`, 10.0.26100.33828, an AMD EPYC 9V74. Three runs, each exit 0 (0.436, 0.423 and 0.436 s),
    the bodies alike byte for byte:
    `{"s":"any_closed","wait":4294967295,"error":6,"after":0}`,
    `{"s":"all_closed","wait":4294967295,"error":6,"after":0}`,
    `{"s":"main","duplicated":1,"closed":1,"set":1,"ended":0,"closed_first":1,"set_second":1,"ended_any":0}`,
    `{"s":"manual_worker","wait":0,"error":0}`, `{"s":"any_worker","wait":1,"error":0}`, and `cycles` with 600 in
    each count and `closes` 3000.
  - Windows' recorded rows fall inside the design's range: WAIT_OBJECT_0 at the set, the set event's index.
  - Its files are in `store/probe-runs/45813040124/` and its binary in `store/binaries/af9e903d1282….exe`. The walk
    fixture gained its entry: 654 instructions, all walked, 179 block starts. `fixtures.rs` now reads 287 images and
    644,291 instructions.
- **The test** `run_loop::the_waits_keep_their_objects_and_a_closed_handle_fails_wait_for_multiple_objects_on_one_and_two_vcpus`:
  - Before the change (HEAD's `sync_objects.c`), it failed on 1 vCPU, "stopped at HostRequest op 4:
    WaitForMultipleObjects argument form 0x2004". The stored binary stopped there on 2 vCPUs too.
  - After the change, on 1 vCPU and on 2, every row equals Windows' body, the recorded rows included. The test passes.
- `crates/guest_runtime/tests/python_image.rs`: the reason for `sync_objects.c` in `DEPARTURES` is extended with
  Sprint 352's change.
- **Plant 1** "stopped at HostRequest op 4: DuplicateHandle argument form 0x100" (the test's stop assertion at
  `run_loop.rs` 1764, on 1 vCPU): SetEvent's `object_done(o);` (`sync_objects.c` 351) was replaced by `(void)0;`. Every
  round's event then stayed taken until `handle_for` found no free entry of 256. Restored by `cp` and checked by
  `cmp`.
- **Plant 2** "wait_closed_kinds on 1 vCPUs, any_worker.wait Some(4294967295) outside the design's range [1, 258]: the set's index or the timeout":
  line 522, `for (u32 i = 0; !retry && i < n; i++) {`, became `for (u32 i = 0; i < n;
  i++) {`, so a retry looked its handles up again. Restored and checked by `cmp`.
- **Plant 3** "stopped at HostRequest op 4: WaitForMultipleObjects argument form 0x2004" (the stop assertion, 1 vCPU):
  line 525 became the stop at 461, `w2m_unimplemented(W2M_API_WaitForMultipleObjects, hs[i]);`, unconditional.
  Restored and checked by `cmp`.
- **The exit ledger's parting** (the amendment). The first gates run failed `exit_ledger::each_process_parts_from_the_frozen_host_where_held`
  with 1,150 process cells (991 main, 159 child 4100) and 34 request-word and written-hash cells.
  - A scratch build dumped the process area at the ledger's rows 33-36. With the new C, the 2 differing bytes are
    the second byte of `&command_line_a`, 0x7F000446E120 against 0x7F000446F120: the guest's saved RAX (STATE_OFF + 0)
    and ring record 121's return (api 0x3f, GetCommandLineA).
  - The cause: the runtime image's `__text` grew from 0x2121c to 0x21790 bytes. `__TEXT` had 660 bytes of slack below
    0x7F000446E000; it now ends at 0x7F000446E2DC, so `__DATA` moved up one page.
  - The request and written cells were the runtime's own buffers moved likewise (a1 0x7F0004479DBA to
    0x7F000447ADBA).
- **The judge's change:** `crates/w2m_vm/src/exit_ledger.rs`, a `Renaming` that names each word in the runtime
  image's range by the run's image's symbol and offset and writes it as the frozen image's address of that symbol.
  - The ledger applies it to each aligned word of the process area before the hash, and to the request's words and
    each write's address, the same move.
  - `crates/w2m_vm/src/bin/w2m-vm.rs`'s `ledger_at` gives every ledger the run makes, the game's and a child's, the
    renaming. It reads the frozen image through `guest_runtime::frozen_python_image`, the image `batch_run.rs` reads.
  - The ledger then passes on the held file unchanged; no held line was added.
  - Its unit test is `exit_ledger::tests::a_runtime_pointer_hashes_the_same_at_two_layouts_and_other_words_unchanged`.
    It holds that a pointer one page apart hashes as the frozen one, that the two unrenamed areas hash apart, that a
    word outside the image is unchanged, and that a whole row (request words, written hash and process hash) is
    written at the frozen layout.
  - The negative case (reviewer-4's twenty-second review, Medium 1): `command_line_a+0x10` and `command_line_w+0x10`
    hash apart once renamed, as do `command_line_a+0x10` and `+0x18`. `command_line_a+0x18` renames to 0x7F000446E128,
    its offset kept.
  - A symbol the frozen image lacks is refused by name: `Renaming::word` answers "0x7f000446f518 names
    added_since+0x8, a symbol the frozen image lacks". `Ledger::resume` returns that error, and the run stops at
    "the exit ledger: …".
  - The batch run, with the refusal in place, reaches its held stop (ReferenceEnd, 92.3 s), and the ledger passes.
- **Plant 4** "where the Rust host parts from the frozen host moved from store/exit_ledger/differences-2026-10-29.tsv" (`exit_ledger.rs` 204), its first found line `main 31 process 661fd1bc…`, 1,150 process lines:
  on the final code, `exit_ledger.rs` 171, `Some(r) => guest_runtime::sha256::hex(&r.area(process)?),`, became
  `Some(_) => guest_runtime::sha256::hex(process),`. w2m-vm was rebuilt and the batch run recorded by
  `scripts/batch-run-record.sh`. Restored by `cp`, checked by `cmp`, and w2m-vm rebuilt.
- **Plant 5** "assertion `left != right` failed: command_line_a+0x10 and command_line_a+0x18, renamed" (`exit_ledger.rs` 343, the renaming's test):
  line 74, `Some(f) => Ok(f + (word - at)),`, became `Some(f) => Ok(*f),`, so `name+0x8` maps as `name`. Restored by
  `cp` and checked by `cmp`.
- The plants ran before the final gates, so the gated tree is the restored one.

- **The gates:** the first run (11:51:55Z) failed on `exit_ledger::each_process_parts_from_the_frozen_host_where_held` (the parting the amendment met); the second (12:50:52Z) was stopped partway when the negative case was added; the final run (12:56:26Z start): `exit 0 in 412 s	default	2026-11-26T13:03:18Z	sprint 352	191ce59fb92cbb34ead11d939787c3142ce68089	e9fd8404e8bf6d3965b3b9eb80b1f92c8def3512856c7a2409391b10260de3f2	root=1cd7e7b8ed5781ead5e9c921d124d13b9e06888c	rust=all python=0 samples=0	base=487219cbf7a5e808ce886d857494ab20ea64cd88`
