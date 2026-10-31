# The translation cache: UnityPlayer.dll translated ahead of time (Sprint 251)

Plan step 5 (`docs/rust-port-plan-2026-10-24.md` 158-160): "The PE images translated ahead of time. UnityPlayer.dll
first … Gate: the batch run conforms with the cache, and compiles at run time, counted on the record, fall by the
share the cache covers." The Decision "Sequence for step 5" states the gate in counts: the batch run conforming,
`vm_compile_blocks` "compiled in UnityPlayer.dll" falling from 149,308 by the blocks the cache holds, and "compiled"
from 274,837 by as many.

## What the run does now

Measured on the batch run (Sprints 246-250, `store/runs/w2m-vm-game-17933*`), and on one run from 9622bb579 with
temporary lines at the stop, removed after (Sprint 251's card):

- 274,837 compiles inside the VM a run, 5.17% to 5.19% of the run's seconds run alone; the most compiling in one 60 Hz
  frame's span 10.9 to 12.0 ms of 16.7.
- UnityPlayer.dll at its preferred base, 0x180000000, size 0x19db000, its bytes the file's (no relocation applied).
  mono-2.0-bdwgc.dll, which asks the same base, lands at 0x1180010000; steam_api64.dll at 0x1222ff0000.
- In UnityPlayer.dll: 149,308 compiles of 132,110 distinct block starts; 3,098,064 guest bytes, 23.5 a block; their
  code 14,340,616 bytes, 108.5 a block, 4.6 bytes of code a byte of x86.
- The code area, 64 MiB at `MONITOR_VA + 0x60000` and shared by both compilers, ends the run 29.3 MB full for 275,559
  blocks; nothing in it is ever freed.
- The 17,198 repeated compiles of a start: the in-VM lookup is the dispatch table alone, two ways of 1<<20 entries,
  and a block evicted from it, or dropped by a write to its page, is compiled again.
- The static census of 2026-10-15 counts 4,996,593 instructions reachable in UnityPlayer.dll and 649 jump tables
  (`store/census/static/instructions-hail-to-the-rainbow.json`): the run reaches about 3.1 MB of the image's code.

## The choices

*(Amended 2026-10-30, Sprint 256: B, B', C and E are superseded by "Revised: the cache installed into the code
area" below; they stand as written, for the record of why.)*

**A. Which blocks: the image's, found statically.** The cache holds every block a walk of UnityPlayer.dll finds from
its entry, exports, TLS callbacks and `.pdata` starts (`pe_image::code_targets`), following direct branches, calls'
fall-throughs and jump tables, as the frozen census's walk did. A cache of the blocks one batch run compiled would
meet the batch run's gate by construction and miss the play path the cache exists for (UnityPlayer.dll retires 76.9%
of the game's instructions in play, Decision 2026-10-14); the walk's cover is measured against the batch run's 132,110
starts, and a start the walk misses is compiled at run time as today.

**B. Where the code lives: its own region at a fixed address, just below the monitor area.** Translated code is
position-dependent (exits are `B`/`BL` to the dispatcher, runtime symbols absolute), so the cache is placed where it
was made. Two bounds fix where. Inside the monitor area there is no room: `layout.rs` packs `CODE_CACHE` (0x60000,
64 MiB), `RUNTIME_STACK` and the vCPU areas up to `MONITOR_SIZE`, 0xAA0_0000, and the runtime image links against
those addresses (`W2M_*`, `layout.rs` 167-200) while the exit ledger hashes the process area from `MONITOR_SIZE`
(`run.rs` 392), so a region there would move judged bytes, KIT_DIARY #103's fault (reviewer-3's Medium 1 at 251).
And a `B` or `BL` reaches ±128 MiB, so a cached block must lie within that of the dispatcher's `exit_va`
(`MONITOR_VA + 0x1800`) and the slice stub, the code area's first stub after the bridges (`guest.rs` 926). The region
is therefore `[MONITOR_VA − 0x780_0000, MONITOR_VA)`, 120 MiB, whose lowest byte lies about 120.4 MiB below the slice
stub; no constant of `layout.rs`, `pe_loader` (`DLL_BASE` 0x7FF8_0000_0000) or the arena (to 0x90_0000_0000) names an
address in it (2026-10-30). It is mapped only when a cache loads, so the monitor area, the runtime image and the
process area are the frozen host's whether or not a cache is used; card 3's gate holds that by a test that every
`W2M_*` value and `MONITOR_SIZE` is unchanged and that the region overlaps nothing the loader or the arena maps. The
walk's code at 4.6 bytes a guest byte may exceed 120 MiB; card 3 measures it, and if it does, the cache holds the
walk's blocks in order of their reach from the image's entries up to the region's end (Revisit if). The run's
compilers keep the 64 MiB area to themselves. The frozen host relocated each entry instead (`vm_backend.py` at
`python-final` 4662-4752); a fixed place needs no relocation code to judge.

**B'. The cache's data in a region of its own** (reviewer-3's Medium 1 at 253). The walk gives 1,004,561 blocks; at
the run's 108.5 code bytes a block they take about 109 MB of B's 120 MiB, so nothing else may share that region. The
table of C (held half full, at least 2,009,122 slots of 16 bytes, about 32 MB), E's page bitmap and the page-to-starts
index are read by absolute address (`mov_imm` and a load), not branched to, so the ±128 MiB bound does not reach them;
they live in a second region directly below the first, `[MONITOR_VA − 0x780_0000 − 0x400_0000, MONITOR_VA −
0x780_0000)`, 64 MiB, mapped only when a cache loads and held by the same layout test. The code's true size is
measured from the cache file at card 3 (108.5 bytes a block is the batch run's mix, not the cold code's), and both
regions' use is on its record.

**C. The lookup: a second level the miss stub reads before the compiler.** The dispatch table stays the first level.
On a miss whose target lies in UnityPlayer.dll's range, the miss stub probes the cache's own table, open-addressed,
held to half full, written once at load; a hit is inserted into the dispatch table as a compile's result is, and the
run goes on with no compile. A miss in the cache's table goes to `w2m_vm_compile` as today. The probe is Rust-written
monitor code, outside the C runtime, so the runtime image stays the frozen host's (KIT_DIARY #103), and off unless a
cache is loaded, so the gates' frozen-word tests hold the default words.

**D. Flag liveness and the key.** A block's words depend on its successors' flag use, which at run time depends on
which pages were compiled before (`block_compiler.c` 131-140). The walk knows every successor in the image, so the
cache's liveness is the walk's, the same on every run; a successor outside the image or unknown is taken as reading
every flag, as the run-time scan takes an unscanned page. The key is the sha256 of the image file, its base, the
translator's source tree id, and the addresses the words embed: the dispatcher, the slice stub, the state block and
the runtime's functions (amended 2026-10-30, Sprint 254: the draft named the monitor code's words and the runtime image whole,
more than a word depends on); the cache file lives under `build/translation-cache/<key>.w2mcache`, made by
`w2m-vm --make-cache` with a game's setup, as Rosetta 2 translates at install.

**E. A write to the image.** The run-time path sees a write because the in-VM compiler makes read-only each page it
compiles from (17,086 pages a run), and the fault reaches the host as `hvc 13` (`monitor_code.rs` 404, `compile.rs`
304-330); cached blocks are never compiled at run time, so that path alone would not see a write to their pages
(reviewer-3's Medium 2 at 251). So at load the host makes read-only, by the same stage-1 descriptor bit, every 4 KB
page of UnityPlayer.dll that holds a cached block, splitting a 2 MB block descriptor where one covers it, as the
compiler's "page needs a split" does; a write then faults to the same `hvc 13`, whose handler sets the page's bit in a
bitmap the probe reads, empties the dispatch slots of the page's cached starts, and makes the page writable. A marked
page's starts fall through to the run-time compiler, whose blocks the existing write handling drops (Sprint 250). The
cache file is never changed by a run. Card 4's gate carries a plant: a guest that writes into a cached page and then
calls the written code must run the new code, not the cached; card 4's lookup searches `tests/data`'s guests for one
that writes its own code first.

**F. The compiler and its judge.** The Rust translator makes the cache's words. Today its words equal the C
compiler's instruction by instruction over 1,186,511 calls (`crates/translator/tests/c_words.rs`); no judge compares
whole blocks, so the first building card adds one: every block the batch run's in-VM compiler made, compiled again
by the Rust translator from the same bytes and the same liveness, word for word, or each difference named.

Ruled out: a dispatch table enlarged to hold the image's blocks (it would still evict, and every entry costs the
first level); the C runtime holding the probe (it changes the judged image); the frozen host's per-entry relocation
(B).

## The order, a gate each

1. **The whole-block judge.** The Rust translator compiles each block the batch run's in-VM compiler made, from the
   logged bytes and liveness, and its words equal the C's, or each class of difference is named. Gate: the judge on
   the record, 0 unnamed differences.
2. **The walk.** UnityPlayer.dll's blocks found in Rust from its code targets, branches and jump tables. Gate: its
   instruction count equals the census's 4,996,593 or each difference is named, and its cover of the batch run's
   132,110 starts is on the record.
3. **The cache file.** The walk's blocks translated into a file under its key, at the region's addresses, with the
   walk's liveness. Gate: every block's words equal the judge's (1) at the same liveness; for every cached block, the
   flags its words leave live cover every flag any of its walked successors reads, computed from the successors' own
   lifted operations, so a dropped flag write is caught on blocks no run reaches (reviewer-3's Low 1 at 251); the
   layout test of B; its size and the time to make it on the record.
4. *(Superseded 2026-10-30, Sprint 256, by "The order from card 4" below.)* **The probe.** The second-level lookup and
   the write bitmap in the monitor code, the cache loaded at start, its
   pages made read-only (E). Gate: the plan's, the batch run conforming with the cache and "compiled in
   UnityPlayer.dll" falling from 149,308 by the blocks the cache holds, "compiled" by as many; E's plant; the
   frozen-word tests unchanged with no cache. Its expected fall is stated before it runs (reviewer-3's Low 1 at 253):
   of the batch run's 132,359 distinct starts in UnityPlayer.dll, 3,081 are not walked and 755 fall inside a walk
   block, so at most 128,523 distinct starts are served, and the repeats of those starts with them. The fall has a
   ceiling: 82,022 compiles a run lie outside any image,
   Mono's run-time code, so "compiled" can fall by at most 192,815 of 274,837 with every image cached, and by
   149,308 with UnityPlayer.dll alone.

## Revised: the cache installed into the code area

*(Amended 2026-10-30, Sprint 256.)* Card 4 could not be built as B, C and E have it. Every reader that turns a code
address into a guest address knows only the 64 MiB code area: the C runtime's exception path (`runtime.c` 245) and
in-VM slice switch (`threads.c` 258) map any other address to x86 address 0, and the reconstruction table, 4,194,304
entries kept in cursor order and searched by bisection, cannot hold the cache's 7,828,283 marks; the host's
give-backs and `code_written` read the same area (KIT_DIARY #108). So a cached block does not run where it was made.

**G. A cached block installed on its first use.** On a miss at a cached start, the block's words are copied to the
code area's cursor and relocated: a `B` or `BL` whose target lies outside the block is re-encoded for its new address,
every other word kept. Measured at Sprint 256 on all 998,222 blocks of UnityPlayer.dll's cache, and on every block of
the 280 fixture images, the relocated words equal the block assembled at its new address, and no block holds another
kind of address-dependent word (`cache_file::relocate`, `relocation_census`). The install then does what a host
compile does (`compile.rs` 70-140): the marks appended to the reconstruction table, the tails noted, the pages
protected, the block recorded, the dispatch slot written; the C runtime, the give-backs, the write path and every
judge see an ordinary block in the code area. An install is counted apart from a compile, so the plan's gate reads
"compiled in UnityPlayer.dll" falling. The code area holds the run's own blocks as today; the cache file holds the
image's.

**H. Who installs, in two steps.** A simple host service takes 4.1 µs a call (`hvc 13`, 56.1 ms over 13,639 on the
batch run), an in-VM compile 4.93 µs (1,355 ms over 274,834). An install by the host, through an exit, saves a
compile's count and little of its time; an install inside the VM, a copy and a re-encoding, saves most of both. The
host's install comes first, since it reuses the host compile's bookkeeping whole and every judge then sees cached
words in the real run; the VM's follows once the host's conforms.

**The order from card 4** (cards 1 to 3 done):

4. **The install by the host.** The cache file loaded at the start; the miss stub sends a start in UnityPlayer.dll's
   range to the host before the in-VM compiler, and the host installs it from the cache, or compiles it as today when
   the cache does not hold it. Gate: the plan's, the batch run conforming with the cache, "compiled in UnityPlayer.dll"
   falling by the starts installed, at most 128,523 distinct starts and their repeats (reviewer-3's Low 1 at 253); the
   installs counted on the record; and the time beside the counts (amended 2026-10-30, reviewer-3's Medium 1 at 256):
   the VM's compile time and the host's seconds on `hvc 1` on timed records run alone, against Sprint 248's 1,351 to
   1,362 ms, a slower run named, not hidden by a falling count; the whole-block judge and the frozen-word tests
   unchanged with no cache. A guest write to the image's code stops the cache's use for the rest of the run, so no
   cached word or liveness outlives the bytes it came from. The batch run makes none (Sprint 257): the card's first
   count, taken in the `hvc 13` handler, saw only pages already behind code; the run with the cache, every executable
   page of the image read-only from the load (`HostCompiler::use_cache`), stopped the cache on no write. *(Amended
   2026-10-30, Sprint 262, reviewer-3's Medium 1 at 261.)* Card 4's time: on six timed records alike with neither the
   whole-block judge nor the exit ledger, the host's install is faster, the rate 3.174 to 3.185 billion instructions a
   second against 3.065 to 3.117 without the cache (3.0% on the means), the VM's run seconds 22.72 to 22.79 against
   23.41 to 23.71, the host's `hvc 1` seconds 0.206 to 0.212 against 0.042. Sprint 257's "1.1% slower" came from
   records that wrote the ledger, whose hashing at each of 150,065 exits weighed on the cached run alone.
5. **The install inside the VM.** The copy and the re-encoding in the miss stub's path, no exit. Gate: card 4's counts
   unchanged, and the run's compile time falling below Sprint 248's 1,351 to 1,362 ms and its most in a 60 Hz frame
   below 248's 10.9 to 12.0 ms, on timed records run alone, without the judge or the exit ledger (Sprint 262); its
   `ledger_parting` check on separate untimed runs that write the ledger. Step 5 is met at this card's close.
   *(Amended 2026-10-30, Sprint 258.)* A cached run is another schedule of the same guest, conforming: a time slice ends at a block's stop
   (`threads.c` 17-18), each block charged whole on entry, and the cache cuts blocks where the in-VM compiler does not
   (a page boundary among them), so the same 14 switches fall at other instructions and the threads interleave
   otherwise from the batch run's 5,539th host request on, 20,683,462,044 instructions in; the run retires
   80,307,872,496 instructions with the cache against 80,306,813,574 without, each deterministic, and both reach the
   reference's end. An install inside the VM installs the same blocks, so card 5's runs retire card 4's cached count
   exactly, and `ledger_parting` against a card 4 ledger finds no parting; its rate and times are read against card
   4's cached records and the uncached ones alike. *(Amended 2026-10-30, Sprint 259.)* Run without the exit ledger
   and the whole-block judge, the VM's run seconds fall with the host's install: 23.06 to 23.67 against 23.90 to 24.34
   without the cache, the compile time 606 to 613 ms against 1,278 to 1,329, at load 2.5 to 3.8; the cached words cost
   no VM time against the host's own compiles of the same starts (23.23 to 23.63). The miss stub's global invalidation
   after each `hvc 1` (`IC IALLUIS; TLBI VMALLE1IS`) costs about 0.7 µs of VM time a use, measured by adding it after
   each in-VM compile (+0.19 s over 274,837), so about 0.1 s over the cached run's 150,065. Card 5's install makes its
   words visible by range and invalidates by page, as the in-VM compiler does (`block_compiler.c` 113-118, 166-176);
   a change to the miss stub's words is an option off by default, as `Timing` is, so the gates' frozen-word tests hold
   the default stub (reviewer-3's Low 2 at 259). *(Amended 2026-10-30, Sprint 260: how it is built.)*
   - **I. The file in the guest, read only.** The cache file's bytes, 211,190,784 for UnityPlayer.dll, are mapped
     read-only at a fixed address below the monitor area, replacing `CACHE_CODE_VA` and `CACHE_DATA_VA` (184 MiB,
     too small), in the layout the file already has: the blocks in start order, their words, their marks. Nothing
     there runs and the C runtime never reads it, so its addresses outside `W2M_CODE_START..END` meet none of
     Sprint 255's limits; the installer only copies from it. Weighed: copying only the 128,456 blocks the batch run
     installs, about 14.5 MB at the cache's 112.8 bytes a block, and not the rest. Not taken: which blocks a run reaches is known only after it, and the cache
     serves any run. The file's table is written once and only read after, so several vCPUs read it with no lock,
     as QEMU's lookups are (below). The guest's memory today (`Layout::new`, `guest.rs` 229-262) is the monitor area
     (0xAA0_0000, 170 MiB), the process area, the loaded images, the heap (256 MiB) and the VirtualAlloc arena
     (0x80_0000_0000, 512 GiB, sparse, 1 GiB aligned): a little over 513 GiB of the 40-bit IPA's 1 TiB (`Vm::new(40)`).
     The file's 211 MB is one more entry in `Layout::maps` and one more part of the guest's allocation, made only
     when a cache is in use; the two regions it replaces had no backing, and `tests/cache_layout.rs`, which holds
     them, is rewritten for the one region and its IPA bound.
   - **II. Each block's fix-ups in the file.** For each block the file lists the word indices `relocate` re-encodes
     (a `B` or `BL` whose target leaves the block), so the installer copies and patches them without decoding a word.
     Version 2 of the file. Weighed: decoding in the VM as `relocate` does. Not taken: the rule would then live in two
     places, Rust and the VM, which Sprint 254's Observation 1 (one rule, one home) refuses.
   - **III. The installer in monitor code, not the C runtime.** The miss stub, with the cache option on, sends a
     target in the image's range to an installer written by `monitor_code` (Rust assembling A64), not to `hvc 1`. It
     finds the start by bisection over the file's block table (998,222 blocks, 20 probes); copies the words to the
     code cursor; patches the fix-ups by the move's distance; appends the marks rebased to the cursor; makes the words
     visible by range (`dc cvau`, `ic ivau`, as `block_compiler.c` 166-176 does); writes the dispatch pair; logs the
     install; and branches to the code. A start the file lacks, a full log, a full code area or a full reconstruction
     table go to `hvc 1` as card 4's do. Weighed: the installer in `block_compiler.c`. Not taken: any change there
     moves the runtime image's linked addresses and with them the frozen-image and call-stream tests (Sprint 247,
     Observation 1). With the option off the stub's words are the frozen host's.
   - **IV. No page protected inside the VM.** Card 4 makes every executable page of the image read-only at load
     (`HostCompiler::use_cache`), so an install changes no descriptor and needs no TLB invalidation; the host notes
     the pages behind each installed block, and the code-page set, when it replays the log.
   - **V. The log tells the host.** An install writes a compile log entry (`block_compiler.c` 31-42) with its `pad`
     word set to 1: the host's replay (`vm_compiled`) notes its tails, pages and start as for a compile and counts it
     as an install, apart from "compiled". The log's 4,096 entries and its replay at every exit serve both. The C
     compiler never writes `pad`, and the log's slots are reused after each replay, so the replay clears `pad` after
     reading each entry; else a compile written into an install's old slot would read as an install.
   - **VI. A write stops the installer.** `code_written` (card 4's write rule) sets a monitor data word the installer
     reads first; set, it sends every target to `hvc 1`, where the host compiles. The log is replayed before the
     write's exit is served, so every block installed before the write is dropped with the others.
   - **VII. The judges.** Card 4's counts unchanged; instructions retired equal to card 4's cached runs',
     80,307,872,496; `ledger_parting` against a card 4 ledger finds no parting; a unit test of the installer's words
     on a small cache in a VM (`run_loop.rs`'s guests), its marks rebased and its fix-ups patched; the plants: a
     fix-up left unpatched, the marks not rebased, the stop word unread.

Each card's census rows: row 13 (flag liveness on composed blocks) at 1 and 3; row 2 (the loader's tests) at 4,
where the cache loads with the image; row 15c (a full code area) at 3, the cache taking the image's code out of it.
Deferred (Sprint 247), the counters on several vCPUs, holds the probe's counts as it holds the compiler's.

## Against the goal

The goal is play at speed: about 37,600 system calls a second in play, 41 million instructions a frame at 112 frames
a second on six threads, and the first scene, 325.4 billion instructions in 62 s on the laptop
(`docs/rust-port-plan-2026-10-24.md` 32-40). The cache moves none of the call figures; it moves what compiling takes
from the frames. On the batch run compiling is 5.2% of the run's seconds, so the cache raises the rate by at most
about 5.5% there; its larger effect is the bursts, 10.9 to 12.0 ms of compiling inside one 60 Hz frame's span, which
on the play path would be late frames, and UnityPlayer.dll's share of play, 76.9% of the instructions, is where those
compiles come from. Six threads compiling at once (step 6) would contend for the one code area and its cursor; a
cache made before the run takes the image's code out of that contention. The first scene is the end of milestone B's
path, and its new code is the play path's; the walk reaches it without a run having gone there first.

## Against the systems that solved it

- **Rosetta 2** translates ahead of time and compiles what it meets at run time: it "sets to work right from the moment
  your application is being installed", and "If Rosetta newly encounters code that haven't been translated at install
  time, then we'll compile it on the fly" (Apple, WWDC20 session 10686,
  https://developer.apple.com/videos/play/wwdc2020/10686/, as the plan quotes it). Taken: the image ahead of time,
  the run-time compiler for what the walk misses and for Mono's generated code.
- **FEX** keeps the code its JIT makes on disk and looks there first: "when FEX needs to JIT code subsequently, it will
  attempt to look up code from that FOZ database first" (FEX-2609, https://fex-emu.com/FEX-2609/, as the plan quotes
  it). Taken: a file under a key, looked up before the compiler; not taken: filling it from what a run compiled, which
  would cover the batch run alone.
- **QEMU** looks translations up without a global lock: lookups go through QHT, "designed for lockless lookups"
  (https://gitlab.com/qemu-project/qemu/-/blob/master/docs/devel/multi-thread-tcg.rst, as the plan quotes it). Taken:
  the cache's table is written once at load and only read after, so several vCPUs probe it with no lock (step 6).
- **Wine** loads PE images itself, its server holding the process's objects ("the clients must send requests to the
  Wine server", Wine Developer's Guide, Architecture Overview,
  https://gitlab.winehq.org/wine/wine/-/wikis/Wine-Developer's-Guide/Architecture-Overview, as the plan quotes it).
  Not taken here: on an x86 host Wine runs the image's code natively and translates none, so it has no cache of
  translations to borrow; the key's base is w2m's own loader's (`pe_loader` 367-372).
- **Firecracker** runs "one or more vCPU threads (one per guest CPU core)"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md, as the plan quotes it), and **crosvm**
  locks each device, not the machine: "Each `BusDevice` is reference counted and wrapped in a mutex"
  (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md, as the plan quotes it). Neither translates code; both
  bear on step 6, where each vCPU's thread probes the cache's read-only table with no shared lock, as each serves its
  own exits.

## Revisit if

- The walk covers under half of the batch run's 132,110 starts in UnityPlayer.dll: the static cover is then the wrong
  tool for this image, and a cover from recorded runs, FEX's, is weighed again.
- The whole-block judge finds differences it cannot name: the cache's words are then not the run-time compiler's, and
  the cache waits on them.
- The walk's code exceeds the 120 MiB within a branch's reach of the dispatcher: the cache then holds part of the
  image, and exits that reach farther (a veneer, or a long branch through a register) are weighed against the
  words they would change.
