# w2m in Rust: the port plan, to a frame

**Date:** 2026-10-24. **Status:** the order of work (Sprint 129). It replaces `docs/rust-port-plan-2026-10-21.md`,
which is history from today, by the user's decision of 2026-10-24: the full re-review of that day is the direction
(BLACKBOARD Decision 2026-10-24 user). The decisions it keeps stand: all of w2m in Rust and Rust only (2026-10-21);
no step runs Python and no Python is written (2026-10-23); substrate stays the harness (2026-10-22); and the order of
2026-10-16 (BLACKBOARD Decision 2026-10-16 user, the full review as the roadmap): "(6) the window on AppKit and the
first frame, through DXMT's d3d11 and dxgi; (7) Microsoft's DLLs over the w2m kernel, off the critical path".

The plan of 2026-10-21 was checked step against step and never as a whole against the goal. Its ten steps ended at
the compiler inside the VM and reached no frame; its host design sized itself from a test guest's poll loop and kept
the Python host's big lock. This plan states where it ends and what it must reach first, and every design and plan
from today passes `target/release/design_check` (Sprint 128) before it lands.

## Against the goal

**The endpoint is milestone B, then the first scene.** The goal is Hail to the Rainbow (Steam app 1699230, build
24247624) playable on this Mac at speed (user, 2026-10-16). Milestone A is batch-mode parity: the game writes the
168-line `unity.log` the Windows runner wrote (Sprint 033); the Python host met it at Sprint 068. Milestone B is the
game drawing its first frame through Metal; milestone C is play (`docs/plan-2026-10-09.md` 17-21). The first scene
closes B's path: the laptop reaches it after 325.4 billion instructions, in 62 s (`docs/recentering-2026-10-14.md`
35).

**The figures every step is held to** (the laptop running the game under Windows, and the Windows runner's batch run):

| Figure | Value | Source |
|---|---|---|
| System calls in play | 314 NT and 18 win32k a frame at 113 frames a second: about 37,600 a second | `docs/plan-2026-10-09.md` 23-24 (E3) |
| Their three largest | NtDelayExecution 114 a frame, NtWaitForSingleObject 100, NtReleaseSemaphore 64: 278 of 332, scheduler work | the same, 24-25 |
| Instructions | 41 million a frame at 112 frames a second, about 4.6 billion a second | `docs/recentering-2026-10-14.md` 35 |
| Threads | six threads busy; the busiest 28.9% of the instructions, about 1.33 billion a second | the same, 35-36, 46 |
| The first scene | 325.4 billion instructions, 62 s | the same, 35 |
| kernel32 calls answered in the guest | GetCurrentThreadId 27,773,497, TlsGetValue 23,291,261, QueryPerformanceCounter 16,912,633 in the 180 s batch run: about 154,000, 129,000 and 94,000 a second; Mono's HeapAlloc 987,170 | BLACKBOARD Decision 2026-10-09 ("Hot imports stay in the guest"), the feasibility review's F5 |

**Where w2m stands.** The Python host on one vCPU retired 2,095,145,119 instructions a second on the batch run
(Sprint 108's card, 89). That exceeds the busiest thread's 1.33 billion and falls short of the game's 4.6 billion,
which arrive on six threads at once: play needs the threads on several vCPUs in parallel. At about 14 µs a crossing to
the host, 37,600 calls a second would use 0.53 s of every second (`docs/plan-2026-10-09.md` 25-27); the Python host's
requests measured 25 to 114 µs each (`docs/postmortem-2026-10-21.md` 62, 159).

**Where the calls are answered.** To milestone B the game runs on w2m's own services, as today: its kernel32 calls
are answered by the C runtime inside the VM where it can (the waits, sleeps and releases, `tools/guest_runtime/
threads.c` and `sync_objects.c`, Sprints 094 to 097; the hot imports above), and by the host otherwise. The 37,600 NT
calls a second are what Windows's kernel sees for that work. If the 278 scheduler calls a frame stay inside the VM,
the host sees at most the other 54: about 6,100 a second at 113 frames (derived, not measured: no play stretch has run
under w2m). Two budgets follow, and the host's design is judged against both: the crossings to the host (6,100 to
37,600 a second), and the calls answered inside the VM under the runtime's locks (several hundred thousand a second
over six threads, by the hot imports alone).

**What each step moves.** Each step below names the figure it moves and the judge that measures it, on a record.
No step closes on a figure it cannot measure: Decision 2026-10-15's condition ("host compiles take more than 1% of a
play stretch's wall time") can fire only once play runs, so its measure comes with milestone B, at step 9.

## Against the systems that solved it

Each problem the port faces has been met before. The plan takes each system's answer, or says why not. The
quotes were copied from the sources on 2026-10-24.

- **A thread per vCPU, serving its own exits.** Firecracker runs "one or more vCPU threads (one per guest CPU core)",
  which "execute synchronous I/O and memory-mapped I/O operations on devices models"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md). crosvm names a thread
  `crosvm_vcpu{cpu_id}` per vCPU (`src/crosvm/sys/linux/vcpu.rs` 542) and locks each device, not the machine: "Each
  `BusDevice` is reference counted and wrapped in a mutex"
  (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md). Taken whole, at steps 3 and 6.
- **Translation on many vCPUs with no global lock.** QEMU removed its translation-block lock in 3.0 (2018): "In
  !user-mode, each vCPU has its own TCG state, so no locks needed. Per-page locks are used to protect the page
  descriptors. Per-TB locks are used in both modes to protect TB jumps" (Emilio G. Cota,
  https://github.com/qemu/qemu/commit/0ac20318ce16f4de288969b2007ef5a654176058). Its
  design note: "Each vCPU has its own TCG context and associated TCG region, thereby requiring no locking during
  translation", and lookups go through QHT, "designed for lockless lookups"
  (https://gitlab.com/qemu-project/qemu/-/blob/master/docs/devel/multi-thread-tcg.rst). Taken at steps 1 and 4: a
  region per vCPU, a lock-free lookup, a lock per page of guest code.
- **Translating ahead of time.** Rosetta 2 "sets to work right from the moment your application is being installed",
  and "If Rosetta newly encounters code that haven't been translated at install time, then we'll compile it on the
  fly" (Apple, WWDC20 session 10686, https://developer.apple.com/videos/play/wwdc2020/10686/). FEX caches the host
  code its JIT makes: "any code that FEX's JIT generates is cached to the disk ... when FEX needs to JIT code
  subsequently, it will attempt to look up code from that FOZ database first" (FEX-2609,
  https://fex-emu.com/FEX-2609/, 2026-09-07). Taken at step 5, Rosetta's split: the PE images ahead of time, Mono's
  generated code at run time.
- **NT waits without a server round trip.** Wine's server holds every object, and "the clients must send requests to
  the Wine server"; a wait is a request the server leaves unanswered "before the wait condition has been satisfied"
  (Wine Developer's Guide, Architecture Overview,
  https://gitlab.winehq.org/wine/wine/-/wikis/Wine-Developer's-Guide/Architecture-Overview). Wine then spent seven
  years taking waits out of that server. esync (2018) aimed "to execute all synchronization operations in
  "user-space", that is, without going through wineserver" (https://github.com/zfigura/wine/blob/esync/README.esync).
  fsync (Proton 4.11-1b, 2019) moved them to futexes, "which can reduce CPU usage compared to esync"
  (https://github.com/ValveSoftware/Proton/releases/tag/proton-4.11-1b). ntsync, usable from Linux 6.14 (2025),
  "exists because implementation in user-space, using existing tools, cannot match Windows performance while offering
  accurate semantics" (https://docs.kernel.org/userspace-api/ntsync.html). LWN names what the server added: "an
  interprocess-communication overhead to every locking operation" (https://lwn.net/Articles/961884/). Its author
  reported "frame rate improvements of 50 to 150 percent" in some games (the same article). Taken at steps 1 and 6: waits on
  one process's objects are answered inside its VM, and the supervisor sees only what crosses processes.
- **A spinning vCPU and a preempted holder.** "If a guest kernel spins on a preempted lock, it could live out the
  remainder of its time slice spinning and accomplishing no work" (Uhlig, LeVasseur, Skoglund and Dannowski, VM '04,
  https://www.usenix.org/legacy/events/vm04/tech/full_papers/uhlig/uhlig.pdf). Arm lets a hypervisor trap the
  guest's WFE (HCR_EL2.TWE), and Intel's PAUSE-loop exiting makes "a series of executions of PAUSE" exit (Intel SDM
  vol. 3, table 27-7). Apple's documentation does not say whether Hypervisor.framework traps WFE; QEMU's HVF backend
  handles a WFx exit (`EC_WFX_TRAP` in `target/arm/hvf/hvf.c`), so step 1 measures it on this Mac before relying on it.

## Order

**Done.** The oracle frozen at `python-final` (Sprint 123); the first host design (Sprint 124,
`docs/design-rust-host-2026-10-23.md`, history from today); Rust reaching substrate (125); the vocabulary in Rust
(126); Signal Reports in Rust (127); the design check (128).

1. **The host redesigned** (`docs/design-rust-host-2026-10-24.md`). Fix (b) of the Decision: no thread waits, on any
   service path, for a lock another thread's service holds. The runtime word guards the run table only; the runtime's
   own state takes a lock per part, and thread-local services take none; translation goes into per-vCPU regions with
   a lock-free lookup; one process's waits stay inside its VM, and the supervisor sees only what crosses processes; a
   vCPU that finds a word held waits instead of spinning on. Its budgets are the goal's: the crossings to the host
   and the calls answered inside the VM, not a test guest's poll loop. Gate: `design_check` and a review.
2. **Vocabulary 0.10.** The 24 tags no code fires (21 named only in generated code, 3 only in tests) deprecated by
   typed proposal (TAG_DEPRECATION_PROPOSED; TECHNIQUES.md #45), before the generator carries them into more Rust.
3. **The walking skeleton.** The Rust host, `w2m-vm`, runs one guest of `tests/data/timed_waits` on one vCPU, on
   the existing C guest runtime, built by a Rust build step as `src/w2m/guest_runtime.py` builds it today (clang over
   `tools/guest_runtime/`). It loads the guest, serves its exits, writes its record through substrate, and ends with
   the guest's stored output. The judge it needs, the golden check of that record, is written in Rust in the same
   sprint, and so on for every judge: each moves to Rust in the sprint whose code it judges, never ahead of it. It
   measures what the design left to measurement: an exit's time, the runtime word's hold times, whether a guest WFE
   exits. Gate: the guest's output and record equal the stored ones.
4. **One vCPU to the batch run.** The skeleton grows by component, each with the Python tests' values carried into
   Rust tests by hand and cited by file and line, and with its judge: loading (S2); the translator (S3) with the
   class judge; the model and slow paths (S4); every exit kind, with the family judge and the batch run's call
   comparison. Gate: the batch run conforms (168 lines, 0 divergences), 456 of 458 classes, 68 of 68 families, the
   guests in `tests/data` with their stored output. The Python host's code moves under `oracle/` (git mv) and leaves
   CI. Moves: instructions a second on one vCPU, against Sprint 108's 2,095,145,119.
5. **The PE images translated ahead of time.** UnityPlayer.dll first: in play it retires 76.9% of the game's
   instructions (Decision 2026-10-14); Mono's run-time code stays with the run-time compiler. Gate: the batch run
   conforms with the cache, and compiles at run time, counted on the record, fall by the share the cache covers.
6. **Several vCPUs.** The redesigned host on two vCPUs and more, with the runtime's locks split as the design sets
   (in the C runtime, changed only there; its port waits for step 10). It needs the design's supervisor: the game
   always starts UnityCrashHandler64.exe, and the Python host refused vCPU 0 waiting while another process exists
   (Sprint 120); a process per guest process removes that refusal. Gate: the litmus tests MP, SB, LB, 2+2W and IRIW;
   the 68 families on two vCPUs; the batch run on the wall clock; Sprint 122's guests (handshake, busychild,
   crosswake, pingpong, statsize) ending on two vCPUs in many runs, the count in the card (crosswake's hang, BLACKBOARD
   Decision 2026-10-23, is fixed here); instructions a second on four vCPUs against the game's 4.6 billion over six
   threads, on a guest that is not mostly waiting; the hot imports' rate on six threads at once against one.
7. **The play path to the first Present.** The game run with no arguments leaves the batch path at the main window:
   from seq 1538 the play run calls the window, IMM32, DXGI and Direct3D 11 functions (game run 38113343972; BLACKBOARD
   Decision 2026-10-11), none of which the batch run reaches. The play runs on Windows (game run 38113343972, the
   laptop's play runs) list the calls up to the first Present; each is served and judged against them, knowable
   first. Gate: the play run's call stream under w2m conforms to Windows's up to the first DXGI or Direct3D 11 call;
   a stub there would change every call after it, so the stream from that call to the first Present is judged at
   8b and 8c.
8. **Milestone B, in three steps.**
   - **8a. The window on AppKit's main thread.** The `w2m-vm` process's main thread runs the AppKit loop and owns the
     window; vCPU threads reach it by message. Gate: a test guest's CreateWindowExW opens a window, and its messages
     arrive in the order the laptop's play run recorded them.
   - **8b. DXMT's imports, and winemetal served by the host.** DXMT's d3d11.dll, dxgi.dll and winemetal.dll load as
     x86-64 PE DLLs and run translated. Their 200 imports are listed from the PE files first: 69 are served today and
     131 are not, "mostly the C runtime" (`docs/recentering-2026-10-14.md` 154). Loading Microsoft's own msvcrt.dll
     or ucrtbase.dll would move those 131 onto that DLL's import list, which reaches the api-ms-win-core sets,
     kernelbase and ntdll; so those two DLLs' imports are listed too, from the laptop's System32, and the shorter
     list decides between serving the 131 and loading the DLL. winemetal's unix calls
     reach the host through NtQueryVirtualMemory's library handle and a dispatcher in w2m's ntdll, and winemetal built
     for arm64 answers them (`docs/milestone-playable-slice-2026-10-13.md` 53-84). Gate: a Direct3D 11 test guest's
     frame reaches a CAMetalLayer, and every pointer that crosses the seam is a `WMTMemoryPointer` the host translates.
   - **8c. The first frame, then the first scene.** The game's first Present reaches Metal: milestone B. Then the
     first scene, measured against the laptop's 62 s and 325.4 billion instructions.
9. **Play measured.** Frames a second and system calls a second in play, against 112 and 37,600, with host compile
   time as a share of a play stretch (Decision 2026-10-15's condition, which can now fire), the supervisor's requests a
   second (the design's revisit condition), and the hot imports' rates in play, beside the batch run's.
10. **After the frame, off the critical path.** The C guest runtime (S5) and the compiler in the VM (S6) ported to
    Rust, `no_std` for `aarch64-unknown-none`, each with its judges (the stencil judges with S6); and Microsoft's DLLs
    over a w2m kernel inside the VM (Decision 2026-10-16 user, order item 7), each family moved to that path when its
    system-call trace conforms to the laptop's. Then milestone C.

Steps 1 and 2 are documents and the vocabulary; step 3 is the first Rust code that runs a guest. Each step runs as a
chain of sprints, one concept each (WORKING_AGREEMENT's hard rule 6 override). New code inside the VM before step 10
is Rust; the C runtime changes only where the design requires it (steps 3 and 6).

## What carries over

From `docs/rust-port-plan-2026-10-21.md`: the principle (keep the behavior, change the shape), the seams S1 to S7 and
their sizes, the set-up (toolchain 1.99.1 pinned, one workspace, Hypervisor.framework bindings written from Apple's
headers, signing by a cargo runner), and the vocabulary in Rust through generated types (Sprint 126). Its step 2 (the
judges built before their code) is withdrawn: the golden check, the class judge, the family judge, the call comparison
and the stencil judges move at steps 3, 4 and 10 with the code they judge.
