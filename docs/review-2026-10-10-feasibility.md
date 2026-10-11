# Review 2026-10-10 — feasibility

Question: will w2m become what `windows_to_macos_research_direction_v0_2.md` says it is — `w2m steam run <appid>` launching an unmodified Windows game on this Mac, built by "the project's own generation and automated verification system"? And what in the repository is false on its face, cargo-culted (the form of a practice without its function), or overtaken by an obvious better practice?

## Method

Read v0.2 §1, §2, §5, §7; `docs/plan.md`; the guest runtime (19 C files, 2,284 lines); `src/w2m/vm_backend.py`'s constants, transport and code cache; `heap.c`, `virtual_memory.c`; `scripts/measure_file_generation.py`; substrate `docs/api.md` on replay levels. Ran the frontier on `Hail to the Rainbow.exe` and an import census of `UnityPlayer.dll` and `mono-2.0-bdwgc.dll`. Read the latest CI runs. 

## 1. Verdict

The CPU half will become what it says. The decoder, 141-plus instruction classes judged against Intel CPUs case by case, CONTEXT reproduced at faults, and the VM with 4 KB pages and TSO are built on the right method and on the right platform choice. Nothing found there is false.

The Windows half is where the claim and the method part ways. The method — probe Windows, record rows, write code until the rows match — works for deterministic, table-like functions, and it moved the game from 13 instructions to 403,044 in one night (frontier rerun today: stop `SHELL32.dll!CommandLineToArgvW`, 115 services served). It does not fit the parts that decide whether a game plays: the window manager and message ordering, COM, Direct3D 11 and DXGI, audio, and timing under 41 threads. And it is judged in a way (§2, F2) that cannot tell an implementation from a replay of its test rows. An independent judge already exists and is unused (§4, S1).

## 2. False on its face

### F1 — "≈27 µs per VM exit" is the Python pipe, and "600 round trips a frame" spends the whole frame

Decision 97 (2026-10-09) measured "a VM exit to the host and back costs about 27 µs (lockstep: 11,931 exits in 335 ms including a 0.9 µs model step each), so a 16.7 ms frame affords about 600 round trips". Lockstep returns to Python on every step through `vm_monitor`'s text protocol (`fgets`/`sscanf` on stdin, `readline` in `vm_backend.py` line 1561). The figure is the cost of that transport, not of a Hypervisor.framework exit. `docs/plan.md` phase 6 carries it forward as the graphics budget ("one round trip ≈27 µs, about 600 per frame"). 600 × 27 µs = 16.2 ms, the entire 16.7 ms frame, leaving nothing for the game. Graphics through a VM boundary has one workable shape — an asynchronous command ring with fences and no synchronous round trip per call — and the plan should state that as the rule, with the transport measured apart from Python.

### F2 — Acceptance on the rows the writer was shown

Each Windows family's contract holds Windows's rows as its acceptance (`exact_rows`), and the implementation is written against those same rows. `measure_file_generation.py` gives the model `contract.json` ("must equal the rows in contract.json exactly, row for row and field for field"), the probe's source, and a judge that returns each differing row "with what was printed and what Windows printed". The agent-written families follow the same loop. "Accepted" therefore means "reproduces these rows", which a lookup on scenario names would also satisfy; nothing on the record distinguishes the two. This is the textbook train-on-the-test error. The standard remedies: rows held out from the writer and judged only at acceptance; properties and invariants checked on generated inputs (the instruction oracle already does this with random states — the Windows side does not); and an independent corpus (S1).

### F3 — "441 records replay" is a file-integrity check

`scripts/gates.sh` prints "441 replay at level 2" beside the test count. Substrate's `docs/api.md` defines level 2 as "stream + counts" plus verifying "every TriggerFired input hash"; it re-runs nothing. The check proves the record files are internally consistent. It says nothing about whether w2m still behaves as the record says. KIT_DIARY #26 shows the consequence: 139 class records each said CANDIDATE_ACCEPTED twice and every check passed.

### F4 — A 4 MB code cache with no eviction, for a 21 MB code section

`CODE_CACHE_SIZE = 0x400000`, and a full cache raises `CompileError("code cache full")` (`vm_backend.py` 1505–1506); `flush_translations` resets to a fixed point for tests, not on overflow. `UnityPlayer.dll`'s `.text` is 21,308,928 bytes and Mono's 3,712,800, before Mono's generated code. Translated AArch64 is larger than the x86 it comes from. The constant is sized for probes. Every production translator handles a full cache (QEMU flushes all translation blocks and restarts; others evict by region).

### F5 — The process heap is a bump allocator with an unsplit free list

`heap.c` takes the first free block at least as large as the request (a 1 MB free block serves a 16-byte request whole), never splits or coalesces, scans the free list linearly, has no lock, and rejects any heap handle but the process heap. It passes its contract's rows. UnityPlayer called HeapAlloc only 1,346 times in 180 s on the runner, but Mono called it 987,170 times and HeapFree 870,525 times in the same run (`jit_capture.bin.imports.tsv`; correction 2026-10-10: an earlier draft counted UnityPlayer alone and called the impact small). An unsplit first-fit list under a million allocate-free pairs fragments and scans; several threads will break it outright; the replacement is off the shelf (dlmalloc runs freestanding).

## 3. Cargo-culted

### C1 — "Generation" for hand-written code

v0.2's working premise is that "complete behavioral specifications can be turned into correct implementations by the project's own generation and automated verification system". In practice the agent session wrote all 141-plus instruction rules (Sprint 041: "rules are hand-drafted") and the Windows families; a cloud model wrote two families as measurements (Sprint 042, Sprint 044's environment family). The records call the agent's own file a CANDIDATE_GENERATED. That is a fine way to build software, and the evidence for it is strong, but the vocabulary dresses ordinary programming as a generation system, and the premise ("complete specifications") does not hold for any Windows contract: each is a sample of rows.

### C2 — Records with no reader

Acquisition (SteamCMD), contract registration, probe authoring and the six differentials each write a substrate record with a causal chain (Sprints 038, 043). The only consumer of those records is the replay check (F3). No decision on file was made by reading a record, and a doubled decision in 139 records went unnoticed until an unrelated sprint printed event counts. Git and the test suite already record who changed what and whether it passes. Records earn their keep where something reads them (a frontier run that starts item runs on a stop; a ledger that reopens a discrepancy); the rest is form.

### C3 — Sprint ceremony at the scale of minutes

Sprint 046 is 40 commits between 04:16 and 05:54, closed, then reopened "until CI is green". Sprint cards with scope, signal, artifact and observation contracts, a Rubber Duck Pass and a dual-contract close now wrap units of work that last minutes. The observation contracts and planted faults inside them do real work (they caught real defects: KIT_DIARY #21–#26); the sprint boundary around them does not, and the sprint count is no measure of progress.

## 4. Superseded by an obvious practice

### S1 — Wine's conformance tests, an independent corpus, unused

Decision 2026-10-07 names Wine's per-DLL tests (`dlls/*/tests/*.c`, with results on real Windows 10 and 11 at test.winehq.org) as an evidence source. No contract, probe or script references them. They are written by people who did not write w2m's code, they run on Windows, and they cover the same functions — the hold-out set F2 lacks, usable without using any Wine code.

## 5. Not false, but walls on the way

- **CI and every Windows reference stop on one account.** The last runs (38074187842, 38053699604) did not start: "recent account payments have failed or your spending limit needs to be increased". GitHub's hosted runners are the only Windows and Intel reference; while the account is blocked, no new row and no new CPU verdict can be made, and Sprint 046 cannot close by its own rule.
- **One thread.** The game peaked at 41 threads; Mono's Boehm GC stops the world with SuspendThread and GetThreadContext. The design (M:N on up to 64 vCPUs, forward rule, CONTEXT at any boundary) is the right one; none of it runs yet.
- **The translator runs in Python at run time.** Every new block is lifted and encoded in Python on a dispatch miss. That is a sound prototype; frame time needs ahead-of-time translation and caching, which phase 7 names.

## 6. What to change

1. Restate the transport rule (F1): no synchronous round trip per graphics call; measure a raw Hypervisor.framework exit apart from the Python pipe; remove "600 per frame".
2. Split every Windows contract into rows shown and rows held out (F2); add Wine's tests for the same functions as a third, independent judge (S1), run on the runner first so their expected results are Windows's.
3. Label the replay check as integrity, not evidence (F3); delete records nothing reads (C2).
4. Code cache: flush on full and size it to the game (F4). Heap: a real allocator before threads (F5).
5. Restore the GitHub account's ability to run jobs; until then nothing new can be judged against Windows.

## Sources

- Commands 2026-10-10: `scripts/run_frontier.py` on `Hail to the Rainbow.exe` (403,044 instructions; `SHELL32.dll!CommandLineToArgvW`); `w2m.pe.parse` over `UnityPlayer.dll` and `mono-2.0-bdwgc.dll`; `gh run view 38074187842`.
- Repository: v0.2 §1, §5, §7, §12.2; `docs/plan.md` lines 22, 54, 63; BLACKBOARD Decisions 97, 107, 2026-10-07 sources; `src/w2m/vm_backend.py` lines 63–64, 86, 1052–1056, 1505–1506, 1554–1561; `tools/vm_monitor/vm_monitor.c` line 144; `tools/guest_runtime/heap.c`; `scripts/measure_file_generation.py` lines 10–15, 87–92; `store/game-runs/38006665419/jit_capture.bin.imports.tsv`; KIT_DIARY #26.
- substrate `docs/api.md` lines 768–771 (replay levels).
