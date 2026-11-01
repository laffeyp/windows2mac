# Each vCPU's own pages (Sprint 312)

The run-loop design's precondition 9 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`): the state a vCPU owns
sits at fixed guest addresses, and a second vCPU would share it. This design lists that state in the Rust host and the
C runtime as they stand at Sprint 311's close (c1919f9be), weighs the two forms Sprint 103 weighed against today's
code, and decides.

## What is known first

- **The C runtime and the layout were built for private pages at shared addresses.** The Python host chose design B
  at Sprint 103 (`docs/design-several-vcpus-2026-10-17.md`, "Choice: B") and built it at 104; at 106 the
  process-wide words that change at run time left the monitor data page for the run table's page: `RECON_COUNT` and
  `CLOCK_WARP` are `RUN_TABLE` + offset (`guest_layout` 17-19). The monitor data page keeps each vCPU's `CURRENT`,
  `CURRENT_ENTRY`, `SLICE_COUNT`, `SLICE_UNTIL`, `LOCK_OWNER` (0xB0-0xE8), and `guest_layout` 90-93 still reserves
  `VCPU_AREA`, 8 slots of `VCPU_STRIDE` (0x20000, 32 pages) in the monitor area for each vCPU's private pages.
- **The Rust host carried the guest side and not the host side.** `compile.rs` 6-7: "One vCPU: no private tables, so
  a descriptor written is written once"; `guest.rs` `page_descriptor`: "One vCPU: no private regions"; one `ttbr`
  (`guest.rs` 640) set on the one vCPU (`monitor.rs` 129-134).
- **The C runtime runs one vCPU at a time.** The runtime word is held around the bridges, the exception entry, the
  switch stub and the miss stub (Sprint 106), and a host request (`w2m_host`, `runtime.c` 25-36: the transport page
  written, `hvc #8`, the result read) is made inside it.
- **The litmus test runs several vCPUs on one monitor data page** (`tests/litmus.rs` 249, x28 the same on each),
  because its threads make no service, fault or switch, Sprint 105's condition. It does not judge this design.

## The inventory

| Page | Address | Holds | Owner | Readers |
|---|---|---|---|---|
| monitor data | `MONITOR_VA + MONITOR_DATA`, one page | the count and limit (0x00, 0x08), the lazy-flag and spill words, `PAUSE_COUNT`, `LOCK_OWNER`, `SLICE_UNTIL`, `RUN_END`, `CURRENT`, `CURRENT_ENTRY`, `SLICE_COUNT`, `SLICE_PAUSES`, `SAVE_AREA`, `SAVE_VECTORS`, `NATIVE_OPERANDS`, the per-thread words `PER_THREAD_MONITOR` saves at a switch | the vCPU, and its running thread | generated code through x28; the monitor code (`monitor_code.rs`, 5 absolute addresses); the C runtime (`W2M_MONITOR_DATA_VA`, 12); the host by offset (97 reads: `threads.rs` 29, `run.rs` 20, `cases.rs` 19, `model_step.rs` 15, `process.rs` 7, `monitor_code.rs` 6, `guest.rs` 1) |
| monitor data, set once | the same page: `CLOCK_MODE` 0x60, `START_MODE` 0x68, `FOREGROUND_MODE` 0x80, `CLOCK_RATE` 0x90, `CLOCK_EPOCH` 0xA0 | the start's choices | the process, written once at load (`process.rs` 283-290) | the C runtime (`process.c`, `threads.c`, `console.c`, `windows.c`, `time_and_error_mode.c`); the host (`threads.rs`) |
| state block | `PROCESS_VA + STATE_OFF`, one page | the guest's registers at a call or fault, rflags, x87 | the running thread, so the vCPU | the translator's lowering (`lower.rs`, 7); the monitor code (4); the C runtime (`W2M_STATE_VA`, 9); the host by offset (26 with the transport page, in 10 files) |
| runtime stack | `MONITOR_VA + RUNTIME_STACK`, 16 pages | the runtime's C frames | the vCPU in the runtime | the C runtime, through SP; the host, a request's buffer in a C frame by guest address (`Layout::offset`; Sprint 327's `statsize`) |
| transport page | `PROCESS_VA + TRANSPORT_OFF` | a host request | the vCPU holding the runtime word | `runtime.c` 27; the host at `hvc #8` |
| exception area | `PROCESS_VA + EXCEPTION_OFF` | a dispatch's records | the vCPU holding the runtime word | `runtime.c` 266, `exceptions.c` 322 |
| run table, compiler log, dispatch and reconstruction tables | the monitor area | threads, the turn, compiles | the process, shared, under the word or atomics | unchanged |

The transport page and the exception area are written only inside the runtime word, so one copy serves every vCPU,
as it did at Sprint 106. The private set is the Python host's: the monitor data page, the state block and the
runtime stack, 18 pages a vCPU. *(Amended 2026-11-01, Sprint 328's card: the table missed the host as a writer and a
reader of the runtime stack. The C runtime passes the host pointers into its own frames, `GetFileAttributesExW`'s
`u64 st[4]` the host writes a stat into and `FindFirstFileExW`'s `u16 full[FULL]` path; the host resolves a guest
address by the layout's maps, which are vCPU 0's, so on vCPU 1 the stat landed in vCPU 0's stack (`statsize` on two
vCPUs, Sprint 327). Card 328 makes the translation take the served vCPU's pages, as Sprint 313 made the host's reads by
offset.)*

## The forms

- **A: each vCPU's state at its own address.** The guest reaches it through a base: x28 already for generated code,
  `TPIDR_EL1` for the C runtime and the monitor code, as Linux arm64 keeps its per-CPU offset. The guest's absolute
  references change: 22 in the C runtime (12 monitor data, 9 state block, 1 save area), 9 in the monitor code, 7 in the
  lowering, 38 in all, with every later stub and C function bound to the rule; the runtime image changes.
- **B: the same address, a different page on each vCPU.** Each vCPU past the first has its own level-0 table and the
  chain of tables down to its 18 pages: `MONITOR_VA` is level-0 entry 0xFE and `PROCESS_VA` 0xFF; the monitor data
  page and the runtime stack lie in two 2 MB ranges of one 1 GB range, the state block in one; so a level-0 table,
  for the monitor area a level-1, a level-2 and two level-3 tables, for the process area a level-1, a level-2 and a
  level-3, 8 tables, each level-3 copying the shared mapping of its 2 MB with the vCPU's pages put in. The 8 tables
  and the 18 pages fit in the vCPU's `VCPU_STRIDE` slot (26 of its 32 pages). No guest code, stub or C function
  changes. `TTBR0_EL1` differs per vCPU, which needs `CnP` = 0: `monitor.rs` writes the table's base with bit 0 clear.
- **Both** need the host's 123 reads of a vCPU's state by offset to take the exiting vCPU's base, as the Python host
  passed a `Vcpu` object with its private pages' offsets (Sprint 103's design, 123); vCPU 0's base is today's offset,
  so one vCPU runs unchanged.

**Decision: B, as Sprint 103 chose it.** It changes no guest code where A changes 38 references and the image, and
the C runtime's offsets already assume it. Its risk is a table on a private chain written after load and not copied:
the tables of the 0xFE and 0xFF chains are written at load (`tables.rs`), and the run-time writers (`page_descriptor`
for the guest's requests, the block compiler's level-3 writes in `block_compiler.c` 85-89 and 237, `protect_code`)
write guest-address tables, shared by every chain; a run-time write into a private chain's range is refused by an
assertion, which the build card puts in the descriptor writer.

## The copied tables after load

*Amended 2026-10-31 (Sprint 314; reviewer-3's Medium 1 at 312):* a private chain's level-0, level-1 and level-2
tables, and its three level-3 tables, are copies of the shared tables made at a vCPU's creation; an entry the shared
tables gain after it never reaches the copy. The tables copied: the level-0 table; the level-1 tables of entries 0xFE
(`MONITOR_VA`) and 0xFF (`PROCESS_VA`, `DLL_BASE`, `TLS_VA`, `LOADER_STUB`); the level-2 tables of the monitor area's
first gigabyte and the process area's (level-1 0x1F8); the level-3 tables of the 2 MB ranges holding the monitor data
page, the runtime stack and the state block. Each run-time writer of a descriptor, read for what it can write in them:

- **`Guest::page_descriptor`** (`guest.rs`) walks from the shared root and creates no table where an entry is invalid
  (it returns 0); its one upper-table write is a split, a block made a table. Under 0xFE and 0xFF no 1 GB block
  exists: `tables.rs` 48-56 maps 1 GB blocks only where the address, IPA and size are 1 GB multiples, and the monitor
  area's IPA (`IPA_BASE`, 0x1000_0000) and the process area's (after it) are not; the loader's regions are mapped a
  page at a time. So no copied level-1 table is written. A 2 MB split in a copied level-2 table would need a split of
  the monitor or process area's own blocks, which no writer asks: the next three keep below `PROCESS_VA` and out of
  the monitor area.
- **`Guest::map_pages`** (`guest.rs`), the guest's mapping requests, writes level-3 descriptors of the addresses the
  guest asks for, which the VirtualAlloc arena places at `VIRTUAL_VA` (0x10_0000_0000, level-0 entries 0 and 1),
  never copied.
- **`HostCompiler::protect_code`** (`compile.rs`) returns for an address at or above `PROCESS_VA`; below it, a guest
  code page in a DLL at `DLL_BASE` is a level-3 write, or a 2 MB split, in tables under level-1 0x1E0, which the
  private level-1 copy points at and does not copy.
- **The block compiler's `protect`** (`block_compiler.c` 89-115) writes only a level-3 descriptor of a page
  `guest_page` allows, below `PROCESS_VA` and outside the monitor area, and leaves a block to the host.
- **A DLL loaded after the start:** LoadLibrary's image goes where the arena places it (`modules.rs` 181), and a system
  image into `PROCESS_VA + SYSTEM_IMAGES_OFF` (`modules.rs` 112, `process.rs` 555), pages mapped at load; neither
  writes a table under 0xFF. Reviewer-3's case, a late system DLL mapped at `DLL_BASE`, does not arise in this code.

So no entry needs making at load: after load no writer writes a copied table, and the count of tables made ahead is 0.
The refusal keeps it so: `page_descriptor` refuses, by name and before any write, a split at level 0 or 1 under 0xFE
or 0xFF, and a split or a leaf's offset inside the monitor area's or the process area's first gigabyte; `map_pages`
and `protect_code` reach their writes through it. The build card's plant: a split asked inside the process area's
gigabyte, which the refusal names.

## Not decided here

- **The instruction clock on two vCPUs.** `process.c` 45 reads the clock from `monitor_word(0)`, the running vCPU's
  own count; with two vCPUs each reads its own, not the process's. The runs that take the clock (`--instruction-clock`,
  the reference and batch runs) stay on one vCPU until a card decides how the counts combine; the refusal is card
  4a's form (Sprint 308), one more reason by name.

## The cards

1. **4c2a, the host's per-vCPU base.** A `VcpuPages` of the exiting vCPU's monitor data and state block offsets,
   passed to the 123 reads; vCPU 0's today's offsets. Judge: every run as held; a plant that gives the run a base one
   page off, which the qpc run's stop names.
2. **4c2b, each vCPU's private chain.** At a vCPU's creation past the first: its 8 tables and 18 pages in its slot,
   the set-once words copied, its `TTBR0_EL1`; the refusal of "The copied tables after load" in `page_descriptor`;
   the instruction clock refused on more than one vCPU, by name. Judge: raw AArch64 on two vCPUs reading the monitor
   data page's first word at one address and seeing each its own (Sprint 104's judge); `TTBR0_EL1.CnP` read 0 on each;
   a page mapped by the guest on vCPU 1 after vCPU 0 touched its range, read on both. Plants: the second vCPU given
   vCPU 0's table, the first judge failing; a split asked inside the process area's gigabyte, the refusal naming it.
3. **4c3, the threads** (the run-loop design's card 4), after both.

## Against the goal

The game makes 37,600 system calls a second in play and retires 41 million instructions a frame at 112 frames a second
over six threads; its first scene is 325.4 billion instructions in. Six threads need several vCPUs, and several vCPUs
need their own monitor data, state block and stack. B adds no instruction to any path the game runs: generated code
keeps its x28-relative loads and the C runtime its constant addresses. A would add a `TPIDR_EL1` read before each of
the 22 C references, on the runtime's service and exception paths, which the 37,600 calls a second enter.

## Against the systems that solved it

- **QEMU**: "Each vCPU has its own TCG context and associated TCG region", and "Structures held on a per-vCPU basis
  won't need locking unless other vCPUs will need to modify them"
  (https://raw.githubusercontent.com/qemu/qemu/master/docs/devel/multi-thread-tcg.rst). The private pages are this.
- **crosvm** runs one thread per vCPU, each owning its vCPU (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md);
  **Firecracker** likewise, "one or more vCPU threads (one per guest CPU core)"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md). Each host thread of card 4c3 owns one
  vCPU and its pages.
- **Wine** reaches a thread's state through a register, `NtCurrentTeb` reading `%gs:0x30` on x86-64 and `x18` on
  aarch64 (https://github.com/wine-mirror/wine/blob/master/include/winnt.h, 2723, 2737): design A's form, which the
  guest's own TEB already uses through GS, and which Linux arm64 uses for per-CPU data through `TPIDR_EL1`
  (https://github.com/torvalds/linux/blob/master/arch/arm64/include/asm/percpu.h, 18-21).
- **Rosetta 2** runs each translated thread as its own native thread
  (https://developer.apple.com/videos/play/wwdc2020/10686/), and **FEX** each guest thread on a host thread
  (https://fex-emu.com/FEX-2609/). Both translate user programs in the host's own process, where a thread's state can
  live in the host thread; w2m runs its guest in a VM under the guest's own page tables, so its per-vCPU state lives
  there.

## Revisit if

A lowering or a C function needs another vCPU's state by address (B hides it; A exposes it), or the private chains'
tables outgrow the slot's 32 pages.
