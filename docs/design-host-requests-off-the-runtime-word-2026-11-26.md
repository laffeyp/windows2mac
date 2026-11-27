# Host requests off the runtime word: the host design's Change 2 (Sprint 366)

Change 2 of the host design (`docs/design-rust-host-2026-10-24.md` 155-171) reads: "a host request from the guest
holds no guest lock, on its own vCPU's page", and "Each of the 65 call sites is read for state it holds across the
request". The Decision "Step 6's order weighed again at Sprint 360" puts C2 first after G2, and the Decision "G2 met at
Sprint 364" says G2 is met. This document is that reading. It maps the transport page and the runtime word's take
around a request today, places each vCPU's own transport page, reads every `w2m_host(` line in the runtime's C, states
what the word still covers after C2, places the woken mark's window, and orders the build cards. Line numbers are those
of commit 5fd4d381d.

The reading's result, first. `grep -o 'w2m_host(' tools/guest_runtime/*.c | wc -l` counts 67 lines in 14 files: the
request's own definition (`runtime.c` 26), two wrappers' bodies (`runtime.c` 22 `host_call`, `threads.c` 83 `host`),
and 64 direct calls. The grep counts neither the wrappers' 13 callers nor `runtime.h` 341's `w2m_stack_grow`, which two
sites call. The class is 79 request sites: 39 hold guest state across the `hvc #8`, 40 hold none. After G3 (Sprints 367 and
368) a measuring card, Sprint 369, times the game's waits by request class; three prerequisite cards follow, then the
release cards in the order of the wait each removes, each built only above Change 1's bound, and a split of the host's
run lock placed by the same measurement.

## The state, as it is

**The transport page.** One page for the whole process: `TRANSPORT_OFF` 0x4000 in the process area
(`crates/guest_layout/src/lib.rs` 99), defined to the C as `W2M_TRANSPORT_VA`, `PROCESS_VA + TRANSPORT_OFF` (158), at
memory offset `MONITOR_SIZE + TRANSPORT_OFF`. Its layout is `struct w2m_transport` (`tools/guest_runtime/runtime.h`
74-78): the op at +0, `arg[0]` to `arg[3]` at +8 to +32, the second answer `arg[5]` at +48, the result at +56. Every
vCPU maps it alike: a later vCPU's private chain holds only its monitor data page, its state block and its 16 runtime
stack pages (`crates/w2m_vm/src/guest.rs` 1243-1268), and `VcpuPages::private` maps those three ranges alone
(`crates/w2m_vm/src/lib.rs` 107-119).

**The request.** `w2m_host` (`runtime.c` 26-36) stores the op and four arguments, executes `hvc #8`, and reads the
second word and the result. The comment at `runtime.c` 59-62 gives the rule today: "the transport page is every
vCPU's, so the request waits for the word".

**Who serves it.** The run loop's `hvc #8` path (`crates/w2m_vm/src/run.rs` 1966-2050) reads the op at
`MONITOR_SIZE + TRANSPORT_OFF` (1967), takes the run's lock by `lock.serve(pages)` (1974), "the guest's own hold of
the word", and calls `host_request::serve` (`crates/w2m_vm/src/host_request.rs` 457-752), which reads the request at
the constant `TRANSPORT` (136-138), vCPU 0's page, and writes the second word and the result there (491, 544, 587,
618, 655, 737-738). `child_request` (177) and `kernel_request` (320) write the second word there too. The other host
readers of that page: the kick's rewrite of the answer (`run.rs` 1398), the exit ledger's row (1901-1905), the thread
requests' record (1978-1980, 1985-1989), `cases.rs` 612 and `bin/w2m-vm.rs` 1137-1140.

**The word's take around a request.** The word is `RUN_TABLE + 0x80` (`monitor_code.rs` 90-91, `runtime.c` 57), taken
by compare-and-swap of 0 for the vCPU's `LOCK_OWNER` (`lock_runtime`, `monitor_code.rs` 118-137). A request is made
under one of four holds:
- *The bridge's.* A service of class `RuntimeWord` has its bridge take the word around its whole C (`guest.rs` 616:
  `word = lock == Lock::RuntimeWord || traced.is_some()`). That is every service not in `NO_LOCK`, `HEAP_LOCK`,
  `SLOTS_LOCK`, `HANDLES_LOCK` or `SYNC_LOCK` (`crates/guest_runtime/src/services.rs` 45-125, 37 names), and every
  service on clock mode 1 of the sync class and QueryPerformanceCounter off mode 0 (`lock_in`, 140-148).
- *The C's own.* `w2m_take_runtime_word` and `w2m_let_runtime_word_go` (`runtime.c` 115-135), by the guard
  `W2M_HELD(runtime_word)` (`runtime.h` 371, 386-392), taken for the page alone at `sync_objects.c` 288 and 683, from a
  wait's check to its park at 870, 982 and 1115, and by `take_word` (378-382) at 657.
- *The claim.* `claim_runtime_word` (`runtime.c` 63-71) in `w2m_unimplemented` (138), whatever the service's class.
- *The exception entry's.* `take_runtime` around `w2m_exception` (`monitor_code.rs` 960-967, `Site::Exception`).

**The host's assumptions that the word is held while it serves.**
- `GuestLock::serve` asserts "a served exit's lock taken without the runtime word" (`run.rs` 915-920), in release
  builds too; at `hvc #8` the guest's hold passes it.
- `KernelLock::take` asserts "the kernel lock taken without a runtime word" (`run.rs` 985-995; Sprint 362's card 4f1),
  held by `host_request.rs` 785-831's test.
- `host_take_runtime_word` returns at once for the served vCPU's own hold, since "the frozen host holds the word across
  a host request" (`monitor_code.rs` 148-171).
- `kick_and_wait` lets the requesting guest's word go for the kick and takes it back, then writes the answer again into
  vCPU 0's page, "which another vCPU's request may have overwritten while the word was let go" (`run.rs` 1371-1400).
- A request's serve writes the run table under the guest's hold: `wake_blocked` (`host_request.rs` 141-157),
  `create_thread`'s new entry (`threads.rs` 176), `mark_woken`, which passes the served vCPU's own thread by
  (`threads.rs` 648-672). A module load writes PEB.Ldr (`modules.rs` 230-240), which EnumProcessModules walks under
  the bridge's word (`module.c` 232-242).
- A child's KERNEL 33 is "the child's runtime word held across it as for every host request" (`host_request.rs`
  159-162), and the request server keeps one request record per child, "since the child's runtime word covers its
  `hvc #8`" (`docs/design-a-process-that-starts-another-2026-11-26.md` 207-209; `cross_process.rs` 79-123: STATE,
  ARGS 2-5, RESULT 6, SECOND 7 in each child's slot of 1,024 words).

## The per-vCPU page

**Where it lives.** vCPU `k`'s slot is `VCPU_AREA + k * VCPU_STRIDE`, 0x20000 bytes, 32 pages (`lib.rs` 90-92). Its
monitor data page, its state block and its 16 stack pages take 18; its tables follow, eight at most, from
`slot + 18 * PAGE` (`guest.rs` 1257-1282), 26 pages in all. The transport page becomes the slot's 19th page,
`slot + 18 * PAGE`, mapped at `PROCESS_VA + TRANSPORT_OFF`, the page beside the state block's `PROCESS_VA + STATE_OFF`
(0x3000); the tables move to `slot + 19 * PAGE`, 27 pages of 32. The page lies in the process area's first 2 MB block
with the state block, whose level-3 table the chain already makes (its key `(3, va >> 21)`, `guest.rs` 1304-1330), so
no table is added. vCPU 0's stays at `MONITOR_SIZE + TRANSPORT_OFF`.

**What of the layout moves.** Nothing inside the page: `struct w2m_transport` is unchanged, and so is `w2m_host`'s
text, since `W2M_TRANSPORT_VA` names the same guest address on every vCPU and each vCPU's tables map it to its own
page. `VcpuPages` gains `transport` (`lib.rs` 93-105), `private` maps `PROCESS_VA + TRANSPORT_OFF` to it, and
`private_chain` adds it to the private list (`guest.rs` 1262-1268).

**How the host finds the exiting vCPU's page.** It is served on that vCPU's own thread, whose `Place` holds its pages
(`run.rs` 1409-1418, 1729), and `GuestLock::serve(pages)` sets `served.vcpu` to them (921-923). Every reader named above
reads `served.vcpu.transport` or `pages.transport` in place of `MONITOR_SIZE + TRANSPORT_OFF`. Two vCPUs' requests can
no longer overwrite each other, and the kick's rewrite of the answer goes: no other vCPU writes this vCPU's page.

**Sprint 363's client slot, per vCPU thread.** Each child's slot of 1,024 words uses words 0 to 909 (`WOKE`,
`cross_process.rs` 109). Under C2 two vCPU threads of one child may each have a KERNEL 33 in flight, so the slot gains
a request record per vCPU, eight words each (state, four arguments, result, second word, one spare), at words
910 + 8k for k below `MAX_VCPUS`, 8: words 910 to 973, under `SLOT`. The doorbell stays one word; the server serves
each posted record under `kernel`, one at a time, as it serves a slot today. This is Change 4's "a request page per
vCPU thread" (`docs/design-rust-host-2026-10-24.md` 227). The first process's own KERNEL 33 takes `kernel` directly
and needs no record.

## Every call site, read

**The forms.** Change 2 names two:
- **A**: the host's answer first, the shared slot written after under the lock. A shared static buffer passed to the
  host is A with the buffer made the call's own (its frame on the vCPU's own runtime stack, or a heap block).
- **B**: the slot taken and marked pending under the lock, the lock let go, the host asked, the slot filled or freed
  under the lock again. A wait is B on the thread's run entry: the woken mark armed (pending), the word let go, the host
  asked, the park under the word.

"None" means nothing shared is held: the request is made with the word let go and nothing else changes. Under "today",
W is the bridge's word, C the C's own take, E the exception entry's, M1 the bridge's on clock mode 1 alone. "Card" is
the build card below that changes the row.

| # | Site | Request | Today, and the caller | Held across the `hvc #8` | Form | Card |
|---|---|---|---|---|---|---|
| 1 | bcrypt.c 30 | RANDOM | W, BCryptGenRandom | none; the caller's buffer | none | 4 |
| 2 | com.c 242 | RANDOM | W, CoCreateGuid | none; the caller's GUID | none | 4 |
| 3 | exceptions.c 107 | MODULE_AT | W or E, the walks (`lookup` 134-138, `w2m_image_function` 127) | none; the frame is on the thread's own stack | none | 4 |
| 4 | exceptions.c 145 | MODULE_AT | W or E, `known_code` in the walks | none | none | 4 |
| 5 | exceptions.c 276 | EXIT | W or E, `end_unhandled` 321-327 | the exception area, one for the process (`W2M_EXCEPTION_VA`), record, context and magic written just before | B: the area taken by a compare-and-swap of its magic word, then written; a second ender waits for the run's end | 11 |
| 6 | exceptions.c 312 | EXCEPTION, CAPTURE_XMM | W, `capture` for RaiseException, RtlUnwindEx, RtlCaptureContext | none; the CONTEXT is the frame's or the caller's | none | 4 |
| 7 | exceptions.c 595 | MODULE_AT | W, RtlPcToFileHeader | none | none | 4 |
| 8 | file.c 130 | FILE_STAT | W, `w2m_file_stat_full` for paths.c `kind_of`, file_search.c `fill`, known_folders.c 72 | paths.c's static `scratch` (37) and `kind_of`'s static `st` (175) | A, own buffers | 5 |
| 9 | file.c 134 | FILE_LOOKUP | W, `lookup` for LoadImageW, CreateFileW, DeleteFileW, GetTempFileNameW | the static `path` (96) and `normalize`'s static `full` (116); in CreateFileW the slot of row 14 | A, own buffers; with CreateFileW's slot B | 7 |
| 10 | file.c 147 | FILE_OPEN | W, LoadImageW's `w2m_file_read_all` | the static `path` | A, own buffer | 5 |
| 11 | file.c 149 | FILE_SIZE | W, the same | none; the descriptor is this call's | none | 4 |
| 12 | file.c 151 | FILE_READ | W, the same | windows.c's static `file` (696), which the host fills | A, a heap block of the call's | 5 |
| 13 | file.c 152 | FILE_CLOSE | W, the same | none | none | 4 |
| 14 | file.c 160 | FILE_STAT | W, `w2m_file_stat` for file_attributes.c 44 | the static `path` | A, own buffer | 5 |
| 15 | file.c 240 | FILE_OPEN | W, CreateFileW | a free `files[]` slot chosen (217-219) and not marked until 244; the static `path`; the sharing rule read over `files[]` before (233) | B: the slot pending with its id, access and share, the sharing check in the same holding; an EEXIST answer to a create sends the call back to its lookup | 7 |
| 16 | file.c 242 | FILE_TRUNCATE | W, CreateFileW | the same slot, its descriptor not yet published | B, within row 15's pending slot | 7 |
| 17 | file.c 268 | FILE_CLOSE | W, CloseHandle | the slot being closed, whose `fd` another vCPU's call may be using | B: the slot marked closing; the last use's drop makes FILE_CLOSE | 7 |
| 18 | file.c 290 | FILE_UNLINK | W, DeleteFileW | the static `path`; the sharing check over `files[]` made before (282-285) | B: a deleting mark on the id that row 15's pending take checks | 7 |
| 19 | file.c 313 | FILE_SIZE | W, WriteFile on an append-only handle | the slot looked up; its position written from the answer | B for the slot's use; A, the position the host's | 8 |
| 20 | file.c 314 | FILE_WRITE | W, WriteFile | the slot; `f->pos` read before (314) and written after (315); `clustered[]` after (316) | B for the slot's use; A, the position the host's | 8 |
| 21 | file.c 339 | FILE_READ | W, ReadFile | the slot; `f->pos` read before and written after (340) | B for the slot's use; A, the position the host's | 8 |
| 22 | file.c 393 | FILE_SIZE | W, GetFileSize | the slot looked up (391) | B, the slot's use | 7 |
| 23 | file.c 440 | FILE_FSTAT | W, GetFileInformationByHandleEx | the slot; `clustered[]` written after (446) | B, the slot's use; `clustered[]` after, A | 7 |
| 24 | file.c 465 | FILE_SIZE | W, SetFilePointerEx at FILE_END | the slot; `f->pos` written from the answer (471) | B for the slot's use; A, a seek the host makes | 8 |
| 25 | file.c 513 | FILE_OPEN | W, GetTempFileNameW | the static `path`; the name's absence read before (512) | A, own buffer; the open is `create_new`, so the host's EEXIST answer steps to the next number | 5 |
| 26 | file.c 515 | FILE_CLOSE | W, the same | none | none | 4 |
| 27 | file.c 552 | FILE_SIZE | W, CreateFileMappingW | the slot looked up (540) | B, the slot's use | 7 |
| 28 | file.c 560 | FILE_DUP | W, the same | the slot; the mapping's handle published after (561) | B, the slot's use; the handle after, A | 7 |
| 29 | file.c 582 | MAP_VIEW | W, MapViewOfFile | a mapping's descriptor and size, read with its reference dropped before the request (573; sync_objects.c 232-241) | B: the lookup's reference kept across; the mapping's FILE_CLOSE at its last reference | 9 |
| 30 | file_search.c 133 | FILE_LIST | W, FindFirstFileExW | a `searches[]` slot chosen (125-127) and its folder written (129-130), `used` set only at 193 | A: the folder in the call's own buffer, both listings made, the slot taken after | 6 |
| 31 | file_search.c 144 | FILE_LIST | W, the same | the same slot; the heap block `listing`, the call's own | A, as row 30 | 6 |
| 32 | guest_calls.c 26 | EXCEPTION, STUBS | W or E, `w2m_guest_stubs` | none; a set-once static the host answers alike each time | none | 4 |
| 33 | module.c 54 | MODULE | W, `find` for GetModuleHandleW, GetModuleHandleExW, GetModuleHandleA | GetModuleHandleA's name in the static `path` (186-189); none for the W forms | A, own buffer | 6 |
| 34 | module.c 62 | MODULE, a load | W, `load_named` for LoadLibraryExW, LoadLibraryW | the static `joined` (77) when the DLL directory joins; the init stub's slot is written after the answer (63-66), A already | A, own buffer | 6 |
| 35 | module.c 123 | MODULE_AT | W, GetModuleHandleExW from an address | none | none | 4 |
| 36 | module.c 138 | MODULE_PATH | W, GetModuleFileNameW | the static `path` (32), which the host writes | A, own buffer | 6 |
| 37 | module.c 160 | MODULE_PATH | W, GetModuleFileNameA | the static `path` | A, own buffer | 6 |
| 38 | module.c 198 | PROC | W, GetProcAddress | none; the caller's name | none | 4 |
| 39 | module.c 205 | MODULE_PATH, no buffer | W, FreeLibrary | none | none | 4 |
| 40 | paths.c 215 | FILE_MKDIR | W, CreateDirectoryW | the static `scratch` (37) | A, own buffer | 5 |
| 41 | paths.c 238 | FILE_RMDIR | W, RemoveDirectoryW | the static `scratch`; the working directory compared before (234) | A, own buffer | 5 |
| 42 | process.c 43 | CLOCK | M1, `counter_now` for QueryPerformanceCounter and every counter reader | none | none | 4 |
| 43 | process.c 71 | FILETIME | M1, `w2m_filetime_now` | none | none | 4 |
| 44 | process.c 74 | FILETIME, the base | W, the first `w2m_filetime_now` on clock mode 0 | the statics `base_filetime` and `base_counter`, set from the answer (73-76) | A: the base pair published once by a compare-and-swap; a caller that loses reads the winner's | 11 |
| 45 | processes.c 64 | KERNEL, PIPE_CREATE | W, CreatePipe | none; both handles published after (65-66), A already | none | 4 |
| 46 | processes.c 75 | KERNEL, PIPE_READ | W, ReadFile on a pipe | the read end's id with its reference dropped (73); the wait's check, the block at 77 | B: the reference kept across; the mark armed before the check, the park under the word | 10 |
| 47 | processes.c 96 | KERNEL, PIPE_WRITE | W, WriteFile on a pipe | the write end's id, its reference dropped (93); the thread's own `progress`; the wait's check, the block at 100 | B, as row 46 | 10 |
| 48 | processes.c 165 | KERNEL, PROCESS_CREATE | W, CreateProcessW | the static `list` (143); its directory entry the address of paths.c's static `cwd` (155-160); the inherited objects' ids read under the table's lock and let go (144) | A: the list and the directory the call's own; B: each inherited object referenced across | 9 |
| 49 | processes.c 180 | KERNEL, PROCESS_STATE | W, OpenProcess | none; the REF and the handle follow the answer, A already | none | 4 |
| 50 | processes.c 184 | KERNEL, REF | W, OpenProcess | none | none | 4 |
| 51 | processes.c 193 | KERNEL, PROCESS_STATE | W, GetExitCodeProcess | a process object's id, its reference dropped (191) | B: the lookup's reference kept across | 9 |
| 52 | runtime.c 22 | any | `host_call`'s body | its five callers' (W1 to W5 below) | as theirs | 12 |
| 53 | runtime.c 26 | the definition | `w2m_host` | the request itself | its text unchanged; it writes this vCPU's page (card 1) and, from card 12, lets this vCPU's hold of the word go across the `hvc #8` | 12 |
| 54 | sync_objects.c 263 | FILE_CLOSE | W, CloseHandle's last handle of a mapping | none; kind and id copied under the table's lock, the slot freed first, A already | none; with row 29 the close moves to the last reference | 4 |
| 55 | sync_objects.c 266 | KERNEL, CLOSE | W, CloseHandle's last handle of a host object | none, A already | none | 4 |
| 56 | sync_objects.c 289 | KERNEL, EVENT_OPEN | C for the page alone (288), a named event's create | none; the handle published after (291), A already | none; the take goes | 4 |
| 57 | sync_objects.c 567 | KERNEL, PROCESS_STATE | C, `host_signaled` in `wait_one_held`, `wait_any_held`, `wait_all_held` | the wait's check, held from the check (872, 993, 1123) to the park (882, 1035, 1170) | B: the mark armed before the check, the park under the word | 10 |
| 58 | sync_objects.c 568 | KERNEL, EVENT_STATE | C, the same | the wait's check | B, as row 57 | 10 |
| 59 | sync_objects.c 575 | KERNEL, EVENT_TAKE | C, `host_take` after the check found the event set | the check's answer, taken by a second request: two waiters of this process could both read one set and both take it | A: one request that takes the event if it is set and answers whether it took | 10 |
| 60 | sync_objects.c 658 | KERNEL, EVENT_SET | C (`take_word` 657), SetEvent on a shared event | none; `w2m_signal(0)` follows (659) | none; the word taken after the request, for the signal | 4 |
| 61 | sync_objects.c 684 | KERNEL, EVENT_RESET | C for the page alone (683), ResetEvent | none | none; the take goes | 4 |
| 62 | threads.c 83 | THREAD | `host`'s body | its eight callers' (W6 to W13 below) | as theirs | 12 |
| 63 | threads.c 385 | THREAD, CREATE | W, CreateThread | none; the TLB flush and the handle follow the answer, A already | none | 4 |
| 64 | virtual_memory.c 20 | VIRTUAL_ALLOC | W, VirtualAlloc | none; the address space is the host's | none | 4 |
| 65 | virtual_memory.c 27 | VIRTUAL_FREE | W, VirtualFree | none | none | 4 |
| 66 | virtual_memory.c 35 | VIRTUAL_PROTECT | W, VirtualProtect | none; `*old` written from the answer | none | 4 |
| 67 | virtual_memory.c 44 | VIRTUAL_QUERY | W, VirtualQuery | none | none | 4 |

The wrappers' callers, which the grep does not count:

| # | Site | Request | Today, and the caller | Held across the `hvc #8` | Form | Card |
|---|---|---|---|---|---|---|
| W1 | runtime.c 139 | UNIMPLEMENTED | the claim (138), any service | none; the run ends | none; the claim goes | 4 |
| W2 | runtime.c 189 | WRITE | W, WriteFile to standard output | none | none | 4 |
| W3 | runtime.c 197 | EXIT | W, ExitProcess | none | none | 4 |
| W4 | runtime.c 335 | RUNTIME_FAULT | E | none; the run ends | none | 4 |
| W5 | runtime.c 384 | EXIT | E, a fault no stack can hold | the exception area, its magic written at 383 | B, as row 5 | 11 |
| W6 | threads.c 167 | THREAD, WAKE | W or C, `readied` after a readying (`w2m_signal` 175-191, `w2m_ready_thread` 198-211) | none; each readying is complete before it | none | 4 |
| W7 | threads.c 397 | THREAD, EXIT_STUB | W, ExitThread | none | none | 4 |
| W8 | threads.c 407 | THREAD, END | W, W2mThreadEnd | none; the mutexes abandoned before (406) | none | 4 |
| W9 | threads.c 414 | THREAD, EXIT_CODE | W, GetExitCodeThread | none; a tid is a value | none | 4 |
| W10 | threads.c 419 | THREAD, EXIT_CODE | C, `w2m_thread_ended` from `host_signaled` (sync_objects.c 564) | the wait's check | B, as row 57 | 10 |
| W11 | threads.c 430 | THREAD, SUSPEND | W, SuspendThread | none | none; the host's kick no longer lets a guest's word go | 4 |
| W12 | threads.c 441 | THREAD, RESUME | W, ResumeThread | none | none | 4 |
| W13 | threads.c 457 | THREAD, CONTEXT | W, GetThreadContext | none; the CONTEXT is the caller's | none | 4 |
| W14 | runtime.c 357, through runtime.h 341 | THREAD, STACK_GROW | E, a touch of the guard pages | none; the state block and save area are the vCPU's own, the area written after (358) | none | 4 |
| W15 | guest_calls.c 32, through runtime.h 341 | THREAD, STACK_GROW | W or E, `w2m_guest_frame` | from `w2m_dispatch_fault` (exceptions.c 508-516), the exception area's record and context, copied into the frame only after `place` (512-513) | A: `w2m_exception` builds them on the vCPU's own runtime stack, the shared area written only by row 5 | 11 |

**What the table finds.** Of 79 request sites, 40 hold nothing (28 direct, 12 through wrappers). The 39 that hold
state (36 direct, 3 through wrappers) hold, by kind, a site holding two counted under each:
- a shared static buffer passed to the host or read back from it: 14 (rows 8, 9, 10, 12, 14, 18, 25, 33, 34, 36, 37,
  40, 41, 48);
- a table slot taken or read across the request: 15, thirteen of `files[]` (rows 9, 15-24, 27, 28; row 9 through
  CreateFileW, row 18 the sharing check DeleteFileW reads) and two of `searches[]` (30, 31); four of the thirteen also
  read a file's position before the request and write it after (19, 20, 21, 24);
- an object's id with its lookup's reference dropped before the request: 5 (29, 46, 47, 48, 51);
- the wait's check, from a host answer to the park: 6 (46, 47, 57, 58, 59, W10);
- the process's one exception area: 3 (5, W5, W15);
- a set-once static set from the answer: 1 (44).
No site holds a heap in the middle of a change: the heap's lock is a leaf that makes no host request (`services.rs`
55-59). No site holds the host's module table during a load: that table is the host's, under the run's lock
(`host_request.rs` 496-546); the guest holds only row 34's buffer. Windows' loader lock, held while DllMain runs
("DllMain is called while the loader-lock is held", Microsoft Learn, Dynamic-Link Library Best Practices,
https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-best-practices, as the host design quotes it),
has no counterpart today with the word or without it: the word is let go when LoadLibrary's bridge returns into the
init stub, before DllMain runs. C2 neither adds that wait nor removes it.

A traced call holds a record of the call trace across its service (`call_trace.c` 110-135): taken and completed under
the word, its own fields between. A request inside the service lets the word go and takes it back before the service
returns, so the record stays whole; another vCPU's traced call between them takes the next record, and the retry's
give-back (133-134) gives the slot back only when it is still the last, as it checks today.

**How a later card proves every row was changed.** Two judges, one of the source and one of the runs.
- A source test, `crates/guest_runtime/tests/host_requests.rs`, made by card 4 in the form of `scoped_locks.rs` (the
  scoped guard's source test, Sprint 359): it lists each request site by file, function and op, the wrappers' callers
  with them, and fails when the C holds a `w2m_host(` site or a wrapper call the list lacks, or the list a site the C
  lacks. Each card moves its rows from the held form to the released one in the list, and the test holds each moved
  row's call to the released request.
- The count of requests made with the word held (judge 3 below), by op, zero after card 12 in the batch run and in
  every family run on one vCPU and on two.

## What the word still covers

After card 12 no request holds the word. The word is still taken, by the guest and by the host, for what follows, each
against Change 1's lock classes (`docs/design-rust-host-2026-10-24.md` 95-153):
- **The run table**, Change 1's "run-table word only for the run table": the park (`w2m_block`), the readyings
  (`w2m_signal`, `w2m_ready_thread`), the switch stub (`Site::Switch`), the woken mark, and on the host the turn's start,
  `hvc #4`, `#12` and a parked vCPU's take (`run.rs` 1773-1776, 2190-2193, 2064-2067, 1660). This is the word's role.
- **The services still of class `RuntimeWord`**: every service outside the five lists, between its requests. Their
  state is what the table names as held: `files[]`, `searches[]`, module.c's `dll_directory`, threads.c's `info[]`,
  processes.c's `handlers[]`, com.c's statics. Change 1 names two classes for these that Sprints 342-359 did not
  build: "the modules' lock for LoadLibrary, GetProcAddress and the module list" and "the call trace's own" (110-120).
  The file table is in no class of Change 1's; the host design's `files` (267) is the host's side of it. These stay on
  the word until a card builds their class; C2 removes the word from a request's span only.
- **The in-VM compiler and the installer** (`Site::Compile`, `monitor_code.rs` 518-566), until C3's per-vCPU regions.
- **The exception entry** (`Site::Exception`), for the per-process fault log (`runtime.c` 376-381) and the exception
  area's last write (row 5).
- **On the host, the requests whose serve writes what the word guards**: THREAD_CREATE (a new run entry, `threads.rs`
  176), a KERNEL 33 answer with a wake (`wake_blocked`), a module load (PEB.Ldr), and the other thread requests that
  write a run entry, which card 2 lists from `threads.rs`. Card 2 makes the host take the word for these itself, before
  the run's lock, as for `hvc #1`, `#4` and `#12` (`take_word`, `run.rs` 830-846). Every other request is served under
  the run's lock alone.

The run's lock (`GuestLock`, `run.rs` 873-934) stays one host lock after C2: every request takes it, and so does every
vCPU's turn start. The host design's locks per part (`modules`, `vm_map`, `files`, `file`, `output`; 262-277) split it.
No card of step 6's order built them (the Decision "G2 met at Sprint 364": C2, C5, C3's per-vCPU parts, 4d2, the
batch run's stop on two vCPUs, 4e, G4, G3, G5, G6); this design makes them its card H, placed by card M's measure of
`GuestLock`'s wait.

## The woken mark's window

Today a wait on another process's object checks the host's answer and blocks inside one holding of the word:
`w2m_pipe_read` asks PIPE_READ (`processes.c` 75) and blocks at 77, and `wait_one_held` holds the word from
`host_signaled` (872) to `w2m_block` (882). The link thread takes that word before it applies a wake
(`apply_wakes`, `run.rs` 1573-1584), so the wake lands before the check or after the block, and the mark that
`wake_from_another_process` sets (`threads.rs` 679-694) is never reached: Sprint 365's plant of it hung 0 of 20 runs
(the Decision "The woken mark in `threads::wake_from_another_process`, a backstop for C2").

**Where C2 opens it.** Card 10 lets the word go across the check's `hvc #8` at the six wait sites (rows 46, 47, 57,
58, 59, W10) and takes it again to park. Between the host's answer ("would block", 259 still running, an event unset)
and `w2m_block`'s exchange of the mark (`threads.c` 124-128), the thread is READY on its own vCPU. Three wakes can land
there:
- another process's, applied by the link thread: `mark_woken_for(guest, 0)` marks every thread a vCPU holds and not
  BLOCKED (`threads.rs` 663-672, 691);
- a KERNEL 33 answer's wake on another vCPU of this process: `wake_blocked`'s `mark_woken`, which passes only the
  served vCPU's thread by (`host_request.rs` 156);
- this process's own readying in the VM: `w2m_signal(0)` after a pipe write (`processes.c` 97) or a shared event's set
  (`sync_objects.c` 659), whose `mark_woken` marks only an ARMED thread (`threads.c` 154-158).
So card 10 arms the mark before the check's request (`w2m_wait_armed`, `threads.c` 136-138), as card 5 of the handles
design arms it before a wait lets `sync_lock` go. `w2m_block` finds WOKEN, clears it, and leaves the thread READY; the
switch retries the call, which asks the host again.

**The plant and its judge.** Card 10's plant is the mark taken out of `wake_from_another_process` (`threads.rs` 691).
Its judge is `tests/data/timed_waits/two_process_pipe_pingpong.c`, Sprint 365's guest (a parent and a child ping-pong
one byte over two pipes 10,000 rounds with INFINITE reads), run 20 times on two vCPUs from `crates/w2m_vm/tests/
run_loop.rs`: with the plant, at least one run hangs to its limit. A plant that hangs no run proves nothing (the kit's
Addendum B4, "verify the verifier"), so the judge also counts the window reached: the wake census
(`w2m_wake_census`, `threads.c` 88) gains the count of parks the mark left READY, and the 20 unplanted runs must show
it above zero. A second plant, the arm left out of `w2m_pipe_read`, is judged by a two-thread pipe ping-pong inside one
process on two vCPUs, which must hang in at least one of 20 runs.

## The judges

1. **Every run as held, on one vCPU and on two**, at every card: the batch run conforming
   (`crates/w2m_vm/tests/batch_run.rs`), the 273 family runs held by `family_runs.rs`'s `state_of` on one vCPU and on
   two, the `twoproc` runs (`scripts/gates.sh` 202-211) and the two-process guests of `tests/data/timed_waits`
   (`two_process_pipe_pingpong`, `spinchild`, `busychild`), the exit ledger's rows unchanged on one vCPU, and G3's
   runs from Sprint 368 (the game's batch run on two vCPUs on the wall clock, several runs, conforming as on one).
2. **A guest whose requests from two vCPUs at once each get their own answer**:
   `tests/data/timed_waits/own_answers.c`. Two threads, each with a region of its own size from VirtualAlloc
   (64 KB and 192 KB), each make 100,000 VirtualQuery calls on their own region and check base and size; each makes
   10,000 GetModuleFileNameW calls on its own module (the program, then kernel32) and checks the path. Every answer
   checked is its asker's; the guest prints the count of wrong answers, 0. Card 1 adds it with VirtualQuery alone;
   card 6 adds GetModuleFileNameW, once its buffer is the call's own.
3. **The count of requests made with the word held**: a new count, `Outcome::requests_under_word`, by op. At each
   `hvc #8` exit the host reads whether the word's holder is the requesting vCPU's owner (one load, as
   `runtime_word_held` reads it, `monitor_code.rs` 179-184) and counts it under the request's op. It is on the run's
   record beside `served_outside_word`. Card M makes it, when it equals the run's `hvc #8` exits; each release card's
   ops fall to zero; after card 12 its sum is zero in the batch run and in every family run on one vCPU and on two.
   Sprint 358's timing of the C's takings is the cross-check: no taking of `Site::Runtime` spans an exit at a
   request.
4. **The woken mark's plant**, at card 10, above.

## The cards that follow

**The sprints before C2's build cards.** By the Decision made at this sprint's close on reviewer-4's thirty-seventh
review, Sprint 367 is the batch run's stop on two vCPUs on the wall clock, and Sprint 368 is G3's runnable part: the
game's batch run on two vCPUs, several runs, conforming as on one; G3 by Sprint 370. Neither is this design's. From
Sprint 368 on, G3's runs are among judge 1's runs at every card below.

**Their order is set by measured waits.** The Decision "Sequence for step 6" (2026-10-30) orders the host design's
changes "in the order those waits name", waits measured on the game. So the cards' sites do not fix their order: a
measuring card does, Sprint 369, and each build card after the prerequisites goes in the order of the measured wait it
removes.

**The bound.** A card is built when the class of requests its rows make, or for card H the run's lock, shows either
of Change 1's bounds for a lock of step 6: "busy more than 5% of the time or a p99 wait above 10 µs"
(`docs/design-rust-host-2026-10-24.md` 136-139). Here "busy" is the word held across that class's requests, as a share
of the run's wall time, and the p99 is over the waits of other vCPUs' takings that overlap such a holding. The goal
gives the two figures their weight: at 37,600 calls a second over four to six threads of play, a 10 µs wait at every
call would be 376 ms of each second (derived: 37,600 × 10 µs), 6.3% to 9.4% of six or four threads' time; 5% of the
word held is 50 ms of each second taken from every other vCPU's park, readying and switch. A card whose class shows
neither is named as not built, and is built when a later measurement shows one. Its rows stay held meanwhile, and judge
3 names their ops.

Each card is one concept; where a card touches more than two files it names them, as the handles design's card 2 did.
Each card's plant is run at that card. A plant whose race the word still serializes shows only once the word is let go
across its rows, so the release card reruns it.

**M. Sprint 369: the measuring card.** On G3's batch run on two vCPUs, three things, with the run's lock timing
(`W2M_TIME_LOCKS` and the runtime word's timing, Sprint 358, `runtime.h` 394-468, `monitor_code.rs` 227-266):
- Judge 3's count, `Outcome::requests_under_word` by op, which this card makes.
- The runtime word's takings, their holds and their waits split by host request class. A taking's entry has eight
  words and the reader reads seven (`run.rs` 266-289); the eighth holds the op of the last request made inside the
  taking, which the host writes into a word of the requesting vCPU's monitor data page at each `hvc #8` and both
  releases (`record_taking`, `monitor_code.rs` 351-391, and `word_released`, `runtime.c` 97-112) copy into the entry.
  A wait whose span overlaps another vCPU's holding of class X counts under X; every vCPU reads the one virtual
  counter.
- `GuestLock`'s hold and wait: the host's clock read before `serve`'s lock, after it and at the guard's drop, by exit
  kind and, for `hvc #8`, by op, on the record beside `turn_start_ns`.
Files: `run.rs`, `monitor_code.rs`, `runtime.c`. Gate: judge 1 with G3's runs; on the batch run on one vCPU, the count equal to its
`hvc #8` exits, every request being under the word today. Plant: the releases leave the eighth word unwritten; the
split then puts no holding under any request class while the count is above zero, and the card's test, which holds the
split's request holdings to the count, names it.

**The prerequisites**, first among the build cards whatever the measurement shows: every release needs its own page,
a child's release needs its own record, and no request may be served with the word free until the host's serve takes
what each request touches. In that order, Sprints 370 to 372: card 1, card 3, card 2. The per-vCPU page and the
request server's record per vCPU thread are prerequisites of every release; card 2 is one too, since
`GuestLock::serve` refuses a served exit with the word free (`run.rs` 915-920).

1. **Each vCPU's transport page.** `VcpuPages::transport` and `private` (`lib.rs`), `private_chain`'s
   19th page (`guest.rs`), and the host's readers (`host_request.rs`, `run.rs`, `cases.rs`, `bin/w2m-vm.rs`): four
   files, one concept, the readers' change mechanical. The word is still held, so no run changes. `runtime.c` 59-62's
   comment names the page as the vCPU's own. Gate: judge 1 with G3's runs; judge 2 with VirtualQuery. Plant: the serve reads
   `VcpuPages::FIRST`'s page for every vCPU; vCPU 1's requests are answered from vCPU 0's stale words, and judge 2
   counts wrong answers above 0.
2. **The host serves a request by what it touches.** `host_request.rs` gives each op its class: the word's
   (the requests of "What the word still covers"), taken by the host's take before the run's lock, passing a guest's
   own hold as today; or the run's lock alone. `GuestLock` gains a serve for a request whose assertion is its class's;
   every other exit keeps `serve`'s. The kick's serve lets its word go before `kick_and_wait`.
   Files: `host_request.rs`, `run.rs`. Gate: judge 1 with G3's runs; judge 3's count, card M's, still equal to the `hvc #8` exits in
   the batch run on one vCPU. Plant: a
   THREAD_CREATE served in the class of the run's lock alone, with the word free: the assertion names it, as
   `a_served_exit_takes_the_lock_only_under_the_word` (`run.rs` 2482-2511) names `serve`'s.
3. **The request server's record per vCPU thread.** Words 910 to 973 of each child's slot, a `Client`
   posting on its vCPU's record, the `Server` serving each posted one (`cross_process.rs`; `host_request.rs`'s
   `child_request` passes the vCPU). It comes before any child KERNEL 33 lets its word go. Gate: Sprint 363's
   two-process test with two writer threads in the second process, 10,000 PIPE_WRITEs each, each writer's bytes
   tagged: every writer's stream read whole and in order; judge 1 with G3's runs. Plant: both threads post on record 0; the order
   check or an answer fails.

**The release cards, by measured wait.** Card 4 comes first of them, since it makes the release every later card
uses and the source test that holds each row; it releases its 40 rows only if their classes show a wait above the
bound, and otherwise builds the release and the test alone. Cards 5 to 11 and card H then go in the order of the wait
card M measured on the class each removes, the largest first, each built only above the bound. Three orders are fixed
whatever the measurement: row 9 is released with card 7, since CreateFileW holds its slot; row 48 with card 9's
references; and rows 46 and 47, card 10's, need card 9's references to the pipe ends, so card 10 carries those two if
card 9 is not yet built. Card 12 is last and is built only once every row is released. Each card's judge 1 includes
G3's runs.

4. **The request with the word let go, for the sites that hold nothing.** Removes: the waits on the classes of its 40
   rows (VirtualAlloc and its kin, the module lookups, GetProcAddress, CreateThread,
   random bytes, the clock on mode 1, the thread requests). `runtime.c` gains
   `w2m_host_without_word`: when this vCPU holds the word, a release store of 0, the `hvc #8`, then the word taken back
   by compare-and-swap of 0 for its owner, acquire, so a guard that took it still lets it go once. The 40 sites
   whose card is 4 in the tables move to it, and the claim, the page's takes (rows 56, 61) and SetEvent's take before
   its request (row 60) go. The source test of "How a later card proves" is made. Files: the twelve that hold those
   rows, one concept, a call swapped where nothing is held. Gate: judges 1 (G3's runs among them) and 2, now unserialized; those ops at 0 in
   judge 3. Plant: card 1's, which now crosses answers with no word between the requests.
5. **A name's buffer the call's own (file.c, paths.c, windows.c).** Removes: the waits on the name requests,
   FILE_STAT, FILE_OPEN by name, FILE_MKDIR and FILE_RMDIR, and LoadImageW's read. `normalize`'s `full` and `path`,
   paths.c's `scratch` and `kind_of`'s `st`, LoadImageW's `file` made frames or heap blocks of the call; rows 8, 10,
   12, 14, 25, 40, 41 released; GetTempFileNameW's EEXIST answer steps to the next number. `lookup` (row 9) waits for
   card 7, its caller CreateFileW holding a slot. Gate: judge 1 with G3's runs; a guest of two threads, each 10,000 CreateDirectoryW
   and RemoveDirectoryW of its own name and GetFileAttributesW of its own file, every answer its own. Plant: paths.c's
   `scratch` static again; the guest counts wrong answers above 0.
6. **Module, search and process buffers the call's own (module.c, file_search.c, processes.c).** Removes: the waits
   on MODULE_PATH, a load by name and FILE_LIST. module.c's
   `path` and `joined`, FindFirstFileExW's folder in the call's buffer and its slot taken after the listings,
   CreateProcessW's list and its directory copy; rows 30, 31, 33, 34, 36, 37 released; row 48 waits for card 9. Gate:
   judges 1 and 2 with GetModuleFileNameW. Plant: module.c's `path` static again; judge 2 counts wrong paths.
7. **The file table's slots across a request (file.c).** Removes: the waits on CreateFileW's lookup, open and
   truncate, CloseHandle's close, DeleteFileW's unlink and the handle queries (FILE_SIZE, FILE_FSTAT, FILE_DUP).
   `struct open_file` gains a state (free, pending,
   open, closing) and a count of uses. CreateFileW looks up, then in one holding of the word checks sharing and takes
   the slot pending with its id, access and share, opens and truncates with the word let go, and fills or frees the
   slot under it. The handle calls take a use under the word and drop it after their request; CloseHandle marks the
   slot closing and the last use's drop makes FILE_CLOSE; DeleteFileW marks the id deleting, which a pending take
   checks. An EEXIST answer to a create sends the call back to its lookup, since the open is `create_new`, which Rust's
   standard library documents as "useful because it is atomic. Otherwise between checking whether a file exists and
   creating a new one, the file may have been created by another process" (`library/std/src/fs.rs` 501-503 of the
   installed toolchain `nightly-2024-12-15-aarch64-apple-darwin`). Rows 9, 15-18, 22, 23, 27, 28 released. Gate: judge
   1 with G3's runs; a guest of two threads, each 10,000 times CreateFileW of one name with share 0, a write and CloseHandle: never two
   handles open on it at once (the second open answers ERROR_SHARING_VIOLATION, 32, while the first is open), every
   close TRUE. Plant: the pending mark not set; two opens succeed at once.
8. **The file position the host's (file.c, file_bridge.rs).** Removes: the waits on ReadFile and WriteFile of a
   file, the asset loads of play (the host design, 274-277), and SetFilePointerEx's FILE_END. FILE_READ and FILE_WRITE at the descriptor's
   position, moved by the host under its per-file lock, the host design's `file`, "that file's position, one read or
   write that uses the position" (262-277); an append-only write at the end in one request; SetFilePointerEx's
   FILE_END a seek the host makes. Rows 19, 20, 21, 24 released. Gate: judge 1 with G3's runs; a guest of two threads reading one
   handle in 4,096-byte pieces of a 40 MB file whose every piece starts with its offset: the pieces read are disjoint
   and cover the file once. Plant: the position read in the guest before the request and written after; two pieces
   read alike.
9. **Objects referenced across a request (sync_objects.c, file.c, processes.c).** Removes: the waits on
   MapViewOfFile, CreateProcessW and GetExitCodeProcess. Kind and mapping lookups
   that keep their reference for the caller to drop after its request; a mapping's FILE_CLOSE and a host object's
   KERNEL_CLOSE at the last reference, where the handles design's rule frees the entry (its "When counts reach 0"),
   as Windows keeps an object while a reference holds it: "Incrementing this count prevents the object from being
   deleted while the pointer is being referenced" (Microsoft Learn, ObReferenceObjectByHandle,
   https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-obreferenceobjectbyhandle, as the handles
   design quotes it). Rows 29, 48, 51 released; `shared_close`'s ledger row still before its "closed" line, since no
   reference is in flight there. Gate: judge 1 with G3's runs; a guest whose one thread maps a view of a mapping 10,000 times while
   another closes and remakes the mapping's handle: every view is the file's bytes. Plant: MapViewOfFile's reference
   dropped before its request; the run stops "a file view of descriptor N, which is not open" (`host_request.rs` 413).
10. **Waits on the host's objects off the word (sync_objects.c, processes.c, kernel.rs).** Removes: the waits behind
    a wait on a pipe, a process, a thread or a shared event, held today from the check to the park. The mark armed
    before the check's request, the check with the word let go, the park under the word; EVENT_TAKE answering whether
    it took. Rows 46, 47, 57, 58, 59, W10 released. Gate: judge 1 with G3's runs; `probes/wait_handoff`, `shared_close` and the
    lost-wakeup guest on two vCPUs. Plants: the woken mark's and the arm's, above.
11. **The exception area and the clock's base (runtime.c, exceptions.c, process.c).** Removes: the waits on a
    fault's stack growth and exit, and on the first system-time read. `w2m_exception`'s
    record and context on the vCPU's own runtime stack, the area taken by its magic word before the last write and the
    EXIT; the base published once. Rows 5, 44, W5, W15 released. Gate: judge 1 with G3's runs, `probes/guest_state`'s exception rows;
    a guest of two threads that fault at once with no handler: the exception area's record is one thread's whole.
    Plant: the area written with no take; the record mixes the two faults' addresses.
12. **`w2m_host` itself (runtime.c, services.rs, run.rs).** `w2m_host` becomes the request with the word
    let go and `w2m_host_without_word` is folded into it; `lock_in`'s clock-mode-1 rules, which exist for the page
    (`services.rs` 127-148), go, but QueryPerformanceCounter's on mode 2, which reads the shared warp word, stays;
    `kick_and_wait`'s release, retake and rewrite go (`run.rs` 1385-1400). Gate: judge 1 with G3's runs; judge 3's sum zero in the
    batch run and every family run on one vCPU and on two; the source test listing every row released. Plant: one
    row's call back to a held request; judge 3 names its op.

H. **The host's locks per part (host_request.rs, run.rs, and the parts' homes).** Removes: the waits on `GuestLock`,
   which every request's serve and every vCPU's turn start take (`run.rs` 873-934, 1773-1776). The host design's
   locks (`docs/design-rust-host-2026-10-24.md` 262-272: `modules`, `vm_map`, `code_page`, `files`, `file`, `output`,
   `trace`, `sched`, `vcpu_handle`) take its place, each request serve taking the lock of what it touches, in the
   design's order, `GuestLock` left for what no part owns. Placed among cards 5 to 11 by card M's measure of
   `GuestLock`'s hold and wait against the same bound. Gate: judge 1 with G3's runs; card M's measure taken
   again, `GuestLock`'s p99 wait below 10 µs; debug builds' check of each thread's lock order (the host design, 293).
   Plant: a request served under a part's lock that its op does not touch, its order check naming it.

After the last card built, the order stands as the Decisions wrote it: C5, C3's per-vCPU parts, 4d2, 4e, G4, G5, G6,
the batch run's stop on two vCPUs and G3 having come before C2's build cards.

## Against the goal

The goal is play at speed: about 37,600 system calls a second in play; 41 million instructions a frame at 112 frames
a second, over six threads; the first scene, 325.4 billion instructions in, at 62 s on the laptop
(`docs/design-rust-host-2026-10-24.md` 26-30).

If the 278 scheduler calls a frame stay inside the VM, about 6,100 calls a second leave it (the host design, 40-41):
files, modules, VirtualAlloc and its kin, the clock and random bytes, the requests the table reads. Each holds the word
today. Sprint 164 measured a bridge's hold of the word across its C service at 2.1 to 4.3 µs at p50, "to 57.5 µs when
the service exits to the host" (the host design, 145-149). At 6,100 requests a second held up to 57.5 µs each, the
word is held across requests up to 0.35 s of each second (derived: 6,100 × 57.5 µs = 350.75 ms), and every other vCPU's
park, readying, switch and `RuntimeWord` service waits behind each one. The six threads' asset loads are file reads
(the host design, 274-277), which today serialize with every scheduler call on the word. After card 12 a request holds
no guest lock; the host's run's lock still serializes the requests' serves, until card H. Which of these waits the
game shows on two vCPUs is not measured yet; card M measures them on G3's runs, and the cards go in that measure's
order.

What C2 costs: card 4's release and retake, one release store and one compare-and-swap around each `hvc #8` that
holds the word, against an exit and back of 0.72 µs (the host design, 42). One page per vCPU, 4 KB, inside a slot
already reserved. Eight request records of 64 bytes in each child's slot of the request page, inside its 8 KB.

## Against the systems that solved it

- **Wine** gives each thread its own request buffer: "Every thread in each Wine process has its own request buffer,
  which is shared with the Wine server" (Wine Developer's Guide, Architecture Overview,
  https://gitlab.winehq.org/wine/wine/-/wikis/Wine-Developer's-Guide/Architecture-Overview, as
  `docs/design-a-process-that-starts-another-2026-11-26.md` quotes it). Taken: a transport page per vCPU, and a
  request record per vCPU thread in the request server's page. Not taken: Wine's server holds every object; here the
  host holds only what the requests reach.
- **Firecracker** runs "one or more vCPU threads (one per guest CPU core)", which "execute synchronous I/O and
  memory-mapped I/O operations on devices models" (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md,
  as the host design quotes it). Taken: each request served on the requesting vCPU's own thread, from its own page.
- **crosvm**: "Each `BusDevice` is reference counted and wrapped in a mutex"
  (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md, as the host design quotes it). Taken: a request takes
  the host's lock of what it touches, the word only for the run table. The host's locks per part are card H.
- **QEMU**: "Each vCPU has its own TCG context and associated TCG region, thereby requiring no locking during
  translation" (https://raw.githubusercontent.com/qemu/qemu/master/docs/devel/multi-thread-tcg.rst, as the host
  design quotes it). Taken: state each vCPU owns needs no lock, which the transport page becomes.
- **Rosetta 2** translates ahead of time and compiles on the fly what it meets at run time (WWDC20 session 10686,
  https://developer.apple.com/videos/play/wwdc2020/10686/, as the host design cites it). Its source is about code;
  nothing of it bears on host requests, and nothing is taken.
- **FEX** keeps its JIT's code on disk and looks there first (https://fex-emu.com/FEX-2609/, as the host design quotes
  it). The source is about code; nothing of FEX's is taken here.

## Revisit if

- Card 2 finds a request whose serve writes the run table, PEB.Ldr or another structure a word-holding service reads
  and that the list of "What the word still covers" lacks: it joins the word's class in that card.
- A card finds held state the table does not name: the table's row is corrected in that card, the source test with it.
- Judge 3's count stays above zero after card 12 in any run: the op it names is a site the table missed.
- The woken mark's window is not reached in 20 runs at card 10 (the census count 0): the judge is widened before the
  plant is believed.
- Card M shows no class and no `GuestLock` wait above the bound on G3's runs: no release card and no card H is built,
  and C5 follows the prerequisites; the measurement is taken again at G5's and G6's runs, whose four and six threads
  G3's batch run does not reach.
- A release card's own measure, taken again at its gate, shows its class's wait not removed: the class's rows are read
  again for state the table missed.
