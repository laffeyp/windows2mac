# A process that starts another, on several vCPUs (Sprint 361)

Card 4f of the run-loop design (`docs/design-run-loop-on-several-vcpus-2026-10-31.md` 204-220) reads "the Machine
driven from its own thread (Sprint 122)". The Decision "Step 6 restored to the user's fix (b)" (BLACKBOARD, 2026-11-24)
asks that card to be read again against the host design's Change 4 (`docs/design-rust-host-2026-10-24.md` 212-232),
"each process its own vCPUs", with "the Python host's one Machine thread not carried by default". Six family runs wait
on it (`store/family_runs/two-vcpus-refused-2026-11-26.tsv`): `process_parameters` 38194049597 and 38194051809,
`processes` 38189162864 and 38189164581, `time_and_error_mode` 38206225386 and 38206227300. Each stops on two vCPUs at
`MACHINE_ON_ONE` (`crates/w2m_vm/src/host_request.rs` 79, refused at 532-534). This document maps how a child process
runs today, places each thing that crosses between processes under Change 4, states the form on several vCPUs, names
its judges, and orders the cards. Line numbers are those of commit ae82b92ba.

The reading's result, first: **on several vCPUs no Machine runs.** Processes run at once, each on its own vCPUs, and a
request server in the first `w2m-vm` answers what crosses processes, one request at a time. One vCPU keeps today's
Machine and its turns unchanged, because the instruction clock needs them. Card 4f does not need C2 or C3 first
(below, "Why C2 and C3 need not come first").

## The state, as it is

**CreateProcessW.** The guest runtime's `k32_CreateProcessW` (`tools/guest_runtime/processes.c` 130-173) refuses
every form the rows do not hold (134-142). It builds the inherited list from the handle table (`w2m_object_inheritable`,
144), the three standard handles under STARTF_USESTDHANDLES (145-150), the error mode (151-154) and the working
directory (155-161). It then asks KERNEL 33 PROCESS_CREATE (165) and makes a PROCESS handle and a REMOTE_THREAD handle
from the answer (167-168). Like every host request, it does so with the runtime word held: `w2m_host` writes the one
transport page and executes `hvc #8` (`runtime.c` 26-36), and "the transport page is every vCPU's" (`runtime.c` 60).

**The kernel objects.** `kernel.rs`'s `Objects` (90-101) holds what processes share: named events (77-82), pipes
(83-87, `PIPE_CAPACITY` 4,096 at 29), and a record per process (42-46: its exit code, the handles others hold on it,
the handles it holds). PROCESS_CREATE (281-325) allocates the child's ids, references its inherited objects, gives the
parent a handle on the child, and returns `Spawn`. The objects live in the first process's `Guest`
(`served.kernel`).

**The spawner.** `host_request.rs` 516-559 serves KERNEL 33. A child's request goes to `child_request` (519, 162-187).
The supervisor's own request takes the objects out of `served.kernel`, serves them against its memory, and puts them
back (522-527). On a spawn it refuses above one vCPU (532-534). It then builds the child's start (`child_start`,
222-246: command line, ids, inherited handles, directory, the environment as the guest holds it now, process group,
clock, drive, read-only setting), makes the link (`turns::Supervisor::new`, 537-542) and spawns (`Supervisor::spawn`,
543-545). `turns.rs` 434-475 starts a second `w2m-vm` with `--child <turn page fd> <memory fd> <slot>` and writes the
start as one JSON line on its standard input. Last, the spawner sets time slices (`set_budget`, 547): "a parent shares
the vCPU with its children from here".

**The child.** `w2m-vm --child` (`crates/w2m_vm/src/bin/w2m-vm.rs` 373-378, 717-814) opens its link (724), arms
`end_with_parent` (725; `turns.rs` 165-198: kqueue EVFILT_PROC, NOTE_EXIT on the parent), and waits for its first turn
before it loads anything (726). It builds its guest from the start line. Its memory is the anonymous file the
supervisor made (`start.memory_fd`, 779), which the supervisor maps too. It is caught up to the machine's clock (801)
and runs on one vCPU (`on_one_vcpu`, 804). Its end posts `link.end(code)`, and its stop posts `link.stop(why)`
(806-811). The option comment says it outright: "a child process, started `--child` alone, never has the option and
runs on one" (325-326).

**The link.** One 64 KB page of an unlinked file (`turns.rs` 19-51) holds a turn word and a slot per process: pid,
message, a request's four arguments, result and second word, wake, readiness, deadline, exit code, stop text, the
child's maps, and its clock. Words are waited on with `os_sync_wait_on_address` and OS_SYNC_WAIT_ON_ADDRESS_SHARED
(`shared.rs` 3-7, 110-150). `Link` is `Supervisor` or `Child` (`turns.rs` 319-322).

**KERNEL 33's wait.** A child's KERNEL 33 posts its maps and arguments and hands the turn back with
`Message::Request` (`ChildLink::request`, 259-273). It then waits for the turn to come back (`hand_back` 216-229,
`wait_for_turn` 232-245), all with its guest's runtime word held. The supervisor serves the request only inside its
own `hand_over` (486-604), which its process enters only when its own turn ends. There it reads the slot (566-567),
serves it against the child's shared memory (`serve` 608-638, `ChildMemory` 387-414, which writes through
`as_mut_slice`), writes the answer, and gives the turn back (551). This is the one host request that waits
(run-loop design 49-57): the answer depends on another process's guest code reaching an idle or a slice end.

**The Machine.** `machine.rs` is kernel.py's `Machine`: "the processes of a run take turns, one at a time, on one
machine clock" (3-7). `next` (36-52) picks the next process in rotation, the first not yet started or ready, else
a timed waiter, else a deadlock. `machine_now` and `catch_up` (56-64) keep the instruction clock one clock over all
processes. No thread of its own drives it. The first process's vCPU drives it from inside `turn` (`run.rs` 934-980),
which `run.rs` enters at two places: a switch with no thread ready in a process with a link (`threads.rs` 1035-1036
returns `Switch::Idle`; `run.rs` 1808-1820), and a slice's end while another process is ready (`run.rs` 1934-1960,
`others_ready`). `turn` takes the link and the kernel objects out of `Guest` (948-949) and lets the lock go, then the
word (951-957). It then runs `Supervisor::hand_over` or `ChildLink::yield_turn` (959-968), takes the word and the lock
again, and puts both back (969-972).

**The turns.** A process runs only in its turn. At each turn's start the run loop asks `others_due` and sets the run
table's `in_vm` (`run.rs` 1524-1529), so the VM's own switch stays off while another process is due. A slice's end
with `others_ready` hands the turn over (1937-1960). `run.rs` 805-810 names the one take of the word that reads no
run's stop, "`turn`'s take after the Machine's wait, a process with a machine link running on one vCPU
(`MACHINE_ON_ONE`, Sprint 325)": a link today runs on one vCPU only.

**What crosses between them.**

| Crossing | Today | Where |
|---|---|---|
| A pipe | an `Objects::Pipe` in the first process; a read of an empty pipe answers "would block" (second word 1, or 2 once no writer is left) and the guest blocks INFINITE and retries; a write takes what fits in 4,096 bytes and wakes | `kernel.rs` 241-277; `processes.c` 71-107 |
| A handle on the other's process | PROCESS_CREATE gives the parent one; OpenProcess asks PROCESS_STATE and REF; a wait on it asks PROCESS_STATE until the answer is not 259 | `kernel.rs` 279, 236-240, 313-314; `processes.c` 176-196; `sync_objects.c` 565-567 |
| The child's exit | `Message::Ended` with the code; `Objects::end` keeps the code, closes what it held (a pipe's last writer breaks it) and wakes every process | `turns.rs` 282-286, 590-598; `kernel.rs` 164-176, 352-363 |
| Named objects | EVENT_OPEN by name finds or makes the event; SET wakes | `kernel.rs` 190-224 |
| A wake | a requester's own blocked threads are readied at once (`wake_blocked`); the others' WAKE words are set and read at their next turn | `host_request.rs` 143-160, 549-554; `turns.rs` 477-482, 226-227 |
| The child's start | command line, ids, inherited handles, directory, environment, drive, clock: one JSON line | `host_request.rs` 222-246; `w2m-vm.rs` 717-798 |
| The child's memory | one anonymous file both map; the supervisor reads and writes a requester's buffers through the child's posted maps | `turns.rs` 387-414, 615-632 |
| The clock | each slot's clock and the machine's, `catch_up` on the instruction clock | `turns.rs` 44-51, 498-502; `threads.rs` 807-819 |
| A stop | the child's stop text in its slot; the supervisor's run stops naming the child | `turns.rs` 288-292, 599-600 |

## Against Change 4

Change 4 (`docs/design-rust-host-2026-10-24.md` 212-232) keeps each process's objects and waits in its own VM:
"Objects of one process live in its VM: events, semaphores, mutants, timers, critical sections and the threads that
wait on them". "The supervisor holds only what crosses processes: the process table, a process's start and end, names
one process opens in another, handles on another process's objects, pipes between processes. A request reaches it as in
the old design (a request page per vCPU thread, `os_sync_wait_on_address`, a median 0.58 to 0.71 µs)." Its locks
table gives the supervisor one lock: "`kernel`, for processes, cross-process names, handles and pipes, one request at a
time" (289). It also says why a process per guest process: "a process per guest process is what lets several vCPUs run
while it exists (the Python host refused that, Sprint 120)" (228-229).

Each crossing, its home, and what of today's form goes with it:

| Crossing | Home under Change 4 | Carried, changed or dropped |
|---|---|---|
| A pipe | the supervisor's `Objects`, under `kernel` | carried: the 4,096-byte buffer, "would block" and the guest's own wait, the broken pipe at the last writer's close; changed: a write's wake reaches the other process at once, not at its next turn |
| A handle on the other's process | the supervisor's process records, under `kernel` | carried: PROCESS_STATE, REF, the 259 rule; changed: a wait's check is answered at once, not in a turn |
| The child's exit | the supervisor's process table, under `kernel` | carried: the code kept, the handles closed, every process woken; changed: posted to the server, not handed back with the turn |
| Named objects | the supervisor's names, under `kernel` | carried whole |
| A wake | each process's own link thread, which readies its own blocked threads under its own word | changed: from "read at the next turn" to "applied when posted" |
| The child's start | the spawn, made by the first process's vCPU under `kernel` | carried, with the run's vCPU count added |
| The child's memory | a view (`ConcurrentGuestMemory`) of the child's mapping in the server | changed: `as_mut_slice` cannot stay while the child's other vCPUs run (the census row D7 rule: every host access to guest memory while a vCPU may run goes through the view) |
| The clock | each process's own | dropped on several vCPUs: they run on the wall clock (Decision "The instruction clock on several vCPUs", 2026-11-26), and each process reads the host's counter; carried on one |
| The turn and the Machine | none on several vCPUs | dropped on several vCPUs; carried on one (below) |
| A stop | the run's stop (`RunStop`), set by the server | changed: the server stops the first process's run through its `RunStop` |

**The Python host's one Machine thread is not carried.** At Sprint 122 the Python host drove the Machine from a
thread of its own, so that vCPU 0 need not wait (run-loop design 204-209). That keeps the Machine: processes still take
turns, one at a time. Under Change 4 processes do not take turns. Windows does not run them in turns either: "The
system assigns time slices in a round-robin fashion to all threads with the highest priority" (Microsoft Learn,
Scheduling Priorities, https://learn.microsoft.com/en-us/windows/win32/procthread/scheduling-priorities), threads, not
processes, and "on multiprocessor systems ... the executable threads are distributed among the available processors"
(Multitasking, https://learn.microsoft.com/en-us/windows/win32/procthread/multitasking). On several vCPUs the Machine
is therefore dropped, not moved to a thread.

**What is carried, and why: the Machine on one vCPU.** One vCPU keeps `machine.rs`, `turns.rs`'s turn form and
`run.rs`'s `turn` unchanged. The instruction clock runs on one vCPU only (Decision "The instruction clock on several
vCPUs": "the instruction clock stays the deterministic clock for tests"). A multi-process run stays deterministic on
it only if one machine clock orders every process, which is what the turns give (`machine.rs` 3-7, `catch_up`). QEMU
makes the same choice: "System emulation will fall back to the original round robin approach if: ... enabling
--icount mode" (https://raw.githubusercontent.com/qemu/qemu/master/docs/devel/multi-thread-tcg.rst, 35-39). The run's
vCPU count, fixed before the run, picks the form, and a child is started with its parent's count.

**What is carried on both forms: the child ends with the run.** Windows does not end a child with its parent: "When
the system is terminating a process, it does not terminate any child processes that the process has created"
(Microsoft Learn, Terminating a Process,
https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process). w2m's run is the first process's
run: `Supervisor::end` (`turns.rs` 641-647) ends every child at the run's end, and `end_with_parent` ends a child whose
supervisor is gone. No row holds a child that outlives its parent, and the game's crash handler is ended with the game.
This stays as it is.

## The form on several vCPUs

**What Windows asks.**
- CreateProcessW: "Creates a new process and its primary thread", and "CreateProcess returns without waiting for the
  new process to finish its initialization"; with bInheritHandles TRUE "each inheritable handle in the calling process
  is inherited by the new process", and "inherited handles have the same value and access rights as the original
  handles" (https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw).
  The parent runs on at once. No turn is due to the child first.
- CreatePipe: with a size of zero "the system uses the default buffer size"; "If the pipe buffer is full before all
  bytes are written, WriteFile does not return until another process or thread uses ReadFile to make more buffer space
  available"; "ReadFile returns when one of the following is true: a write operation completes on the write end of the
  pipe, the number of bytes requested has been read, or an error occurs"
  (https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createpipe).
- WaitForSingleObject on a process handle: a process is among the objects it waits for, and "If the object's state is
  nonsignaled, the calling thread enters the wait state until the object is signaled or the time-out interval elapses"
  (https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject); "When a process
  terminates, the state of the process object becomes signaled, releasing any threads that had been waiting for the
  process to terminate" (Terminating a Process, above).
- GetExitCodeProcess: "If the process has not terminated and the function succeeds, the status returned is
  STILL_ACTIVE" (https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getexitcodeprocess).

None of these orders the parent and the child in turns. Each asks that a write, an exit or a set reach a waiter in
the other process when it happens.

**What macOS gives.**
- One VM per process: `hv_vm_create` "Creates a VM instance for the current process" (Hypervisor.framework `hv_vm.h`
  28-33), and `hv_vcpu_create` "Creates a vCPU instance for the current thread ... Each thread can only have one vCPU
  associated at a time" (`hv_vcpu.h` 20-28). A child stays a `w2m-vm` of its own, as today, with a thread per vCPU.
- A word two processes wait on: `os_sync_wait_on_address` "reads a value from @addr, compares it to expected @value
  and blocks the calling thread if they are equal. This sequence of operations is done atomically with respect to other
  concurrent operations"; OS_SYNC_WAIT_ON_ADDRESS_SHARED "should be used when synchronizing among multiple processes by
  placing the @addr ... in a shared memory region", and "should not be used when synchronizing among multiple threads
  of a single process"; `os_sync_wake_by_address_any` "wakes up one waiter out of all those blocked"
  (`usr/include/os/os_sync_wait_on_address.h` 49-51, 57-59, 74-78, 305-306, in this Mac's SDK). `shared.rs` already
  uses them; its test measured a round trip between two processes held under 1 ms, and between two threads a median of
  6,292 ns (`shared.rs` 173-176).
- A child's end: kqueue EVFILT_PROC with NOTE_EXIT, "process exited" (`sys/event.h` 72, 260), as `end_with_parent`
  uses it; or the `try_wait` the supervisor's wait makes every 100 ms today (`shared.rs` 132-150, `turns.rs` 554-556).

**Which thread or process runs the child, and on how many vCPUs.** The child is a second `w2m-vm`, as today, started
by the same spawn with one change: its start line carries the run's vCPU count, and `w2m-vm --child` runs the
several-vCPU `run` (`run.rs` 1001) with that many threads in place of `on_one_vcpu`. Each process runs at once, each on
its own vCPUs, none waiting for a turn. On several vCPUs a link never makes `Switch::Idle`. A process with a link and
no thread ready parks the vCPU on its `Sched` condition (`Switch::Parked`, `threads.rs` 1030-1033), whether or not
another of its vCPUs holds a thread. Today that case returns `Switch::Idle` (1035-1036), or, with no link, a timed
waiter or the deadlock at the switch (1040-1053). The park's wait ends at the earliest deadline of a blocked thread,
as every park's does (run-loop design 188-190). `others_due` and `others_ready` are false, so the VM's own switch stays
on and a slice's end hands nothing over.

**The request server.** One thread in the first `w2m-vm`, started at its first spawn on several vCPUs and ended at the
run's end. It is Change 4's supervisor, and holds no guest word and no guest lock. Wine's server is the precedent: it
"is a single and separate Unix process and does not have its own threading - instead, it is built on top of a large
`poll()` loop" (below). It is a thread here, not a process. The first process already holds the objects
(`served.kernel`), and its own requests (UnityPlayer's side: CreatePipe, three named events, PROCESS_CREATE, the
framed writes; `processes.c` 6-12) then take the lock directly with no round trip. A separate process would add a
process to the run and a round trip to each of those.
- **The page.** The same anonymous 64 KB file, a second layout: a doorbell word the server waits on, then a slot per
  process holding a request state (idle, posted, answered), four arguments, the result and second word, a message
  (request, ended, stopped), the exit code, the stop text, the maps posted once at the child's start (the layout is
  fixed after load: `layout` is a Load field, run-loop design 80), and a wake count.
- **A request.** A child's KERNEL 33 writes its arguments, marks its slot posted, raises the doorbell
  (`os_sync_wake_by_address_any`, SHARED) and waits on its slot's state. The server takes `kernel`, serves the slot
  against the child's memory through a view of its mapping, writes the result and second word, then the state,
  answered (a release store after the answer's words), and wakes the slot. One request per process is in flight at a
  time, since the child's runtime word covers its `hvc #8`. When C2 takes the word off host requests, the slot becomes
  a slot per vCPU thread, Change 4's "a request page per vCPU thread".
- **A child gone without a word** is found as today: the server's doorbell wait times out every 100 ms and asks each
  child's `try_wait` (`shared.rs` 132-150's form); the run stops naming the child.

**Which lock or request each crossing takes.**
- The first process's own KERNEL 33: the runtime word (its guest's), then the run's lock (`GuestLock`), then `kernel`,
  a leaf held for one request. A spawn is made under `kernel` (the children's table is under it too): `Command::spawn`
  is held there once per child, a stated cost, bounded and rare.
- A child's KERNEL 33: its own runtime word, held by its guest as for every host request, then the round trip to the
  server; the server takes `kernel` alone. No KERNEL 33 answer waits for guest code to run in any process, so the word
  is held for a bounded time, as for a file request.
- The wake: the server, after a waking answer (`Answer::wake`: a pipe written or read, an event set, a close that
  broke a pipe, an end), raises every live process's wake count. A child's wake count lives on the page, waited on
  with SHARED. The first process's own wake is an in-process count and condition the server and its own requests
  raise, since the header says SHARED "should not be used when synchronizing among multiple threads of a single
  process". The requester's own threads are readied in its own service as today (`host_request.rs` 169-171, 549-550).
- Each process's **link thread** waits on its wake count. When it rises, the thread takes its process's runtime word
  (the host's take, `host_take_runtime_word`), then the run's lock, and runs `wake_blocked` (`host_request.rs`
  143-160). That readies every BLOCKED thread (`set_status`, which notifies `Sched`) and marks every running thread
  woken. The word closes the window between a wait's check and its park. A pipe read or a wait on a process checks the
  host's answer and blocks (`w2m_block`, `threads.c` 97-129) inside one holding of its word, so the link thread's wake
  lands before the check or after the block, never between. A thread past its check and not yet parked takes the mark
  and retries (`threads.c` 117-126). The link thread is not a Machine: it chooses no turn and runs no other process.
- The deadlock rule: a process alone stops when every vCPU is parked, no thread has a deadline, and a second passes
  with no change (run-loop design 197-200). A process with a link on several vCPUs waits there with no verdict, since a
  wake can come from another process. The `processes` child blocks INFINITE on its pipe read (`processes.c` 76-79)
  until its parent writes, and would otherwise be stopped as a deadlock. The verdict moves to the server instead. Each
  process posts in its slot whether every one of its vCPUs is parked with no blocked thread holding a deadline, and
  clears the post when a vCPU takes a thread. The server stops the run as a deadlock, with today's `DEADLOCK` words
  (`threads.rs` 650), when every live process has posted it and a whole second passes with no request, no wake count
  raised and no post cleared. That is the one-process rule (run-loop design 197-200) taken over all processes, as
  `machine::next`'s `Next::Deadlock` takes it on one vCPU (`machine.rs` 48-51). Card 4f3 builds it.
- The stop: the server, on a child's stop or a child gone, stops the first process's run through its `RunStop`,
  handed to the spawn by the serving vCPU, as a time limit stops it.

Lock order. In a vCPU thread: the runtime word, the run's lock, then `kernel`, a leaf. In the server: `kernel` alone.
In a link thread: the runtime word, then the run's lock; never `kernel`. No thread holding `kernel` waits for a word
or the run's lock, so no cycle forms.

**What one vCPU keeps unchanged.** A run of one vCPU makes the turn form's link, as today: `Supervisor`, `ChildLink`,
`hand_over`, `machine::next`, the instruction clock's `catch_up` and warps, the 1 ms timed waiter on the wall clock
(`turns.rs` 521-524), and the child's KERNEL 33 served in the supervisor's turn. The child of a one-vCPU run runs on one.
The batch run (one vCPU, the instruction clock, UnityCrashHandler64.exe its child), the family runs on one vCPU and the
exit ledger see no change of form. The `kernel` lock (card 4f1) is the only change one vCPU takes, and it takes it
uncontended.

### Why C2 and C3 need not come first

The run-loop design kept a child on one vCPU "until KERNEL 33 waits with no guest lock held, the host design's change
2" (55-56), because KERNEL 33 was the one request that waited. It waited for a turn, while the child's word was held,
on the supervisor's guest code reaching an idle. The form above removes the wait itself, not the word. A child's
request is answered by a thread that runs no guest code and waits for none, and the word is held across a round
trip, as for every host request today: a file read, a VirtualAlloc. C2 removes the word from every host request,
KERNEL 33 among them, and stays in its place in the order. C3 (each vCPU's own cursor, save area and runtime stack)
concerns translation inside one process; a child on two vCPUs is the same run loop as the 267 runs that conform on
two vCPUs today. The Decision's revisit clause ("card 4f's re-reading of Change 4 needs C2 or C3 first") is therefore
not met. Revisit if a card finds a KERNEL 33 answer that must wait for another process's guest code: then that
request waits with no word held, and C2 comes first.

## The judges

**The six held runs on two vCPUs, held to Windows' rows.** The six runs of `two-vcpus-refused-2026-11-26.tsv`, each
run on two vCPUs, parent and children, held by `crates/w2m_vm/tests/family_runs.rs`'s `state_of` (1320-1348) to their
contracts' `exact_rows`: `windows.process.processes@0.2.0`, `windows.process.parameters@0.1.0`,
`windows.time.local_and_error_mode@0.3.0`. The refused list then holds no run, and its test
(`the_refused_runs_are_held_to_the_hosts_refusals`, 1366-1377) becomes "holds none". Each of the six is run 20 times on
two vCPUs in the card that lifts the refusal, all 120 runs conforming, since the six exercise a wake across processes
that a lost wake would show only sometimes.

**The `processes` runs' pipe.capacity row, held exactly at 4,096.** The row (`processes.c` 168-179) counts a writer
thread's one-byte writes during the reader's Sleep(500) on the wall clock. Windows' rows read 4,096 on both runners
(runs 38189162864, windows-2025, and 38189164581, windows-2022; the contract's row
`{"s":"pipe.capacity","written_before_read":4096,"read":65536,"writer_done":0}`). The 1,816 and 3,634 the card
carries in were the Python host's on the wall clock under load (commit 9a7515bb9, 2026-10-13), which moved the family
to the instruction clock. On two vCPUs the family's start runs on the wall clock (Decision "The instruction clock on
several vCPUs"). The Rust host was measured on the wall clock here, on one vCPU, since two are refused
(2026-11-26, this Mac, an Apple M5 Max of 18 cores, `w2m-vm` built at 15:18 PST). Every one of 50 runs read 4,096:
- 2 runs, one per binary, at a load average of 2.3; each run's every row equals the contract's after its derivation;
- 36 runs at once;
- 12 runs at once beside 54 busy loops (`yes`), three for each core, at 23:39:40-23:39:43 UTC.

So on the Rust host the wall clock's cost that Sprint 360 carried into card 4f did not show, quiet or loaded. No run
has read the row on two vCPUs yet, since all two-vCPU runs are refused. On two vCPUs it is held by card 4f3's gate,
then 4f4's 20 runs of each, and then every gates run's two-vCPU family runs (`family_runs.rs` `state_of`, exact
rows). The row is held to Windows' value, not to a range. Windows' own rows give no range (both read 4,096), and a range
below it would pass a pipe that holds less than 4,096 bytes, the fault the row exists to catch. No quiet host is
needed: three busy loops a core did not move it. Revisit if a two-vCPU run reads less than 4,096. Then the writer's
one-byte write is measured on two vCPUs before any range is weighed.

**One vCPU, unchanged.** The batch run conforming (`crates/w2m_vm/tests/batch_run.rs`), the 273 family runs on one
vCPU as held, the multi-process guest (`scripts/gates.sh` 202-211's `twoproc` run on the instruction clock), and the
exit ledger's rows unchanged, at each of the cards below.

## The cards that follow

Each is one concept. Where a card touches three files, it says so, as the handles design's card 2 did.

1. **4f1, Sprint 362: the supervisor's objects behind `kernel`.** `Objects` and the turn form's `Supervisor` move out
   of `Guest.served` into one mutex beside `GuestLock`, Change 4's `kernel`, which a KERNEL 33 service takes under the
   word and the run's lock as a leaf. `turn` no longer moves the objects out (`run.rs` 949, 971). One vCPU, turns
   unchanged. Files: `host_request.rs`, `run.rs`, and the field in `guest.rs`. Gate: every run as held on one vCPU and
   on two (the batch run conforming, the 273 family runs as held, the six still refused by name, the `twoproc` guests,
   the exit ledger); `kernel`'s guard asserts that its taker holds a runtime word or is the server, in release builds
   too, as `GuestLock::serve` asserts the word. Plant: a KERNEL 33 that takes `kernel` without the word; the assertion
   names it.
2. **4f2, Sprint 363: the request server.** A new file, `crates/w2m_vm/src/cross_process.rs`, holds the page's second
   layout (doorbell; per process: state, arguments, answer, message, exit code, stop text, maps, wake count), `Server`
   (its thread, the doorbell wait with its 100 ms look at each child, a request served under `kernel` against a
   `ConcurrentGuestMemory` view of the child's mapping) and `Client` (post and wait). `kernel.rs`'s `Memory` gains the
   view. No run reaches it yet. Gate: a test that runs its own binary as a second process, as `shared.rs` 211-250
   does. The second process makes 10,000 PIPE_WRITE requests of one to 100 bytes; a thread of the first makes the
   PIPE_READs under `kernel`. The bytes read equal the bytes written, in order, and the round trip's median is printed
   and held under 1 ms. Plant: the state stored before the answer's words; the order check fails.
3. **4f3, Sprint 364: processes at once, the child on one vCPU.** `MACHINE_ON_ONE` goes. A spawn on several vCPUs
   makes the `Server` link; a child, still on one vCPU, makes the `Client` link. Each process gets its link thread and
   wake count. On several vCPUs a link makes no `Switch::Idle` and no turn, and `others_due` and `others_ready` are
   false. A process with a link parks with no deadlock verdict of its own, and the server gives the verdict across
   processes (above, "The deadlock rule"). Files: `host_request.rs`, `threads.rs`, `cross_process.rs`. Gate:
   - the six runs on two vCPUs (parent on two, children on one) held to Windows' rows, pipe.capacity at 4,096, and one
     vCPU unchanged;
   - a two-process guest beside `tests/data/timed_waits`', on two vCPUs: a parent and its child each wait INFINITE on
     a named event that only the other would set. It stops within two seconds as a deadlock, by `DEADLOCK`'s words.
   Plants:
   - the child's end raises no wake count: the `processes` parent's WaitForMultipleObjectsEx on its child
     (`processes.c` 297) runs to its 10,000 ms timeout, and the row `process.exited` differs;
   - the server's verdict skipped: the deadlock guest runs to GUEST_SECONDS and stops by the time limit, not as a
     deadlock.
4. **4f4, Sprint 365: the child on the run's vCPUs.** `child_start` carries the vCPU count, and `w2m-vm --child` runs
   `run` with it. `vcpus_allowed`'s refusal of a child (`run.rs` 668-670) goes for a child started by a several-vCPU
   run. Files: `host_request.rs`, `w2m-vm.rs`. Gate: the six runs on two vCPUs, children on two, 20 runs each, all
   conforming; a two-process guest beside `tests/data/timed_waits`' (a parent and a child ping-pong one byte over two
   pipes 10,000 rounds with INFINITE reads), ending inside its instruction limit in 20 of 20 runs on two vCPUs.
   Plant: the link thread's wake without the woken mark; the guest must hang to its limit in at least one run.
5. **G2's judge, Sprint 366.** The 273 family runs on two vCPUs, the six among them, conforming; the refused list
   holding none; G2 met by Sprint 366, the budget of the Decision "Step 6's order weighed again at Sprint 360". After
   G2 the order stands as that Decision wrote it: C2, C5, C3's per-vCPU parts, 4d2, the batch run's stop on two vCPUs,
   4e, G4, G3, G5, G6. G3 gains from this design: the batch run's child, UnityCrashHandler64.exe, is no longer refused
   on two vCPUs.

## Against the goal

The goal is play at speed: about 37,600 system calls a second in play, 41 million instructions a frame at 112 frames
a second over six threads, and the first scene, 325.4 billion instructions in, at 62 s on the laptop
(`docs/design-rust-host-2026-10-24.md` 26-30). The game starts UnityCrashHandler64.exe at every batch and play run
(run-loop design 208-209), so while the refusal stands no run of the game gets a second vCPU, and with one vCPU the
six threads cannot run at once (2.87 to 2.93 billion instructions a second measured on one, against the game's 4.6
billion; run-loop design 360-362).

What the form costs the game. UnityPlayer's crossings are CreatePipe, three named events, CreateProcessW, three framed
writes and zero-timeout waits on the process (`processes.c` 6-12, from game run 38176899195). Each is one request
under `kernel` in the first process, or one round trip for the handler, a median of 0.58 to 0.71 µs (Change 4). Change
4's revisit bound, 376 requests a second in play (1% of 37,600), stands. Removed with the turns: the game's process
handing the machine to the handler at every slice's end while the handler is due (`run.rs` 1937-1960), and every slice
cut to TIME_SLICE once a child exists (`run.rs` 1040-1044).

## Against the systems that solved it

- **Wine** keeps every process's threads native and concurrent, and serves what crosses processes from one server:
  "Every thread in each Wine process has its own request buffer, which is shared with the Wine server", and "The Wine
  server itself is a single and separate Unix process and does not have its own threading - instead, it is built on
  top of a large `poll()` loop" (Wine Developer's Guide, Architecture Overview,
  https://gitlab.winehq.org/wine/wine/-/wikis/Wine-Developer's-Guide/Architecture-Overview). Taken: one server, one
  request at a time, a request buffer per client. Not taken: Wine's server is a separate process and holds every
  process's objects. Here it is a thread of the first process and holds only what crosses, Change 4's lesson from esync
  and ntsync (the host design, 74-82).
- **Firecracker**: "Each Firecracker process encapsulates one and only one microVM", with "one or more vCPU threads
  (one per guest CPU core)" (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md, 71-77).
  Taken: a `w2m-vm` per guest process, each with its vCPU threads, as Hypervisor.framework's one VM per process asks.
- **crosvm** runs "A process per virtual device", whose "only interaction ... with the main process is via the proxied
  trait methods of `BusDevice`, shared memory mappings such as the guest memory, and file descriptors that were
  specifically allowed", and "Each `BusDevice` is reference counted and wrapped in a mutex"
  (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md, 5, 32-34, 113). Taken: a child reaches the first
  process only through the page, its memory's mapping and the descriptors it inherits; `kernel` is one device's mutex.
- **QEMU** runs every vCPU at once, and falls back "to the original round robin approach if: ... enabling --icount
  mode" (https://raw.githubusercontent.com/qemu/qemu/master/docs/devel/multi-thread-tcg.rst, 35-39). Taken: the turns
  stay with the instruction clock on one vCPU, and every run of several vCPUs drops them.
- **Rosetta 2** runs each translated thread natively, its own thread (WWDC20 session 10686,
  https://developer.apple.com/videos/play/wwdc2020/10686/, as the run-loop design cites it). Its source is about
  threads in one process; nothing of it bears on processes, and nothing is taken.
- **FEX** keeps its JIT's code on disk and looks there first (https://fex-emu.com/FEX-2609/, as the host design quotes
  it). The source is about code; nothing of FEX's is taken here.

## Revisit if

- A card finds a KERNEL 33 answer that waits for another process's guest code: that request waits with no word held,
  and C2 comes before the card that needs it.
- A two-vCPU run's pipe.capacity row reads less than 4,096: the writer's one-byte write is measured on two vCPUs before
  any allowed range.
- A play stretch sends the server more than 376 requests a second (Change 4's bound): cross-process objects move to
  pages both processes map, waited on with `os_sync_wait_on_address` (shared).
- A run of several vCPUs hangs to its time limit with every process parked and the server's verdict not given: the
  posts or the second's window miss a change, which is fixed in the sprint that finds it.
- A row needs a child that outlives its parent: the run's end stops waiting on the first process alone.
