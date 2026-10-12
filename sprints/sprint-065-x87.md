# Sprint 065 — x87

```yaml
---
id: 065
status: open
phase: 5
pass_kind: functional
---
```

## scope

After Sprint 064 the frontier runs 23,371,280,986 instructions and stops at `LiftError: no operand template for FLD`
at 0x1180ccfd2f (`dd 44 24 f8`, FLD m64), in code mono's JIT generated. The census of 2026-10-12
(`scripts/class_census.py`) finds the x87 the game reaches:

| Where | Classes (instances) |
|---|---|
| mono's JIT code (capture of run 38216081440) | FLD 9, FSTP 9, FMUL 5, FSQRT 3, FSIN 3, FCOS 3, FADD 2, FWAIT 2, FISTTP 2 |
| UnityPlayer.dll | FWAIT 4, FIADD 3, FISTP 2, FADD 1, FIDIVR 1, FLDCW 1, FNCLEX 1, FNSTSW 1; FNSTCW lifted |

Nothing else the game folder ships reaches x87. FEMMS (3) is AMD's 3DNow!, behind a CPUID test the guest's Intel
profile fails. The MMX-register forms in UnityPlayer (MOVD 85, MOVQ 444 and the rest) are reached by no run so far,
and keep a plan row of their own.

1. **Cases with x87 state.** The semantics probe's format 4: each case loads an FXSAVE image (FCW, FSW with TOP, the
   abridged tag word, ST0–7) with FXRSTOR before its XMM and MXCSR loads, and each result holds FXSAVE's image after
   the instruction (from the CONTEXT's FltSave at a fault). States: the x87 stack empty, partly full and full;
   operands from 80-bit boundaries (zeros, denormals, pseudo-denormals, infinities, NaNs, unnormals, the largest
   and smallest normals); the control word's precision and rounding fields; the cases run on the Xeon 8573C.
2. **The model.** `x86fp` gains the 80-bit format, and `runtime.py` the x87 register stack and the classes above,
   each against the SDM's page. FSIN and FCOS reduce their argument as the hardware does: Intel's documentation and
   Bruce Dawson's measurements (2014) put that at a 66-bit pi. The model is judged on the cases before the VM.
3. **The VM.** The x87 stack lives in the state block (`specs/guest_state.json`'s st area, the control, status and
   tag words beside it). Each x87 instruction runs as a model step (hvc #11): the game executes few, and every one
   is a model step until a measurement asks for more. Threads switch with their x87 state.
4. **The frontier** passes the FLD and seq 3426 (from 064), its comparison with game run 38216081440 conforming.
5. **What the frontier meets past the FLD** (amended 2026-10-12, before the work; Decision 2026-10-11 user: every
   divergence closed by matching the reference). A scratch run of 50.75 billion instructions writes 49 lines of
   unity.log, equal to Windows' but for two timings. It stops at CreateFileW's form 0x08000080
   (FILE_FLAG_SEQUENTIAL_SCAN | FILE_ATTRIBUTE_NORMAL), and its comparison shows four kinds of divergence:
   - **The system font folder.** Unity lists `C:\Windows\Fonts\*` (seq 2778) and finds files on the runner; w2m's
     drive C: has no such folder. A probe records the runners' listing, and the file bridge holds it.
   - **Hinted VirtualAlloc.** mono asks for the address past its last block and retries without the hint when that
     address is taken (487). Which address is taken depends on the address space: on Windows, ASLR. Those calls are
     left out of the pairing on both sides, with the rule in the contract.
   - **TryEnterCriticalSection's answer** depends on whether another thread holds the lock at that instant, which the
     scheduler decides; it is not compared.
   - **RtlAddGrowableFunctionTable's two calls** pair in opposite orders; its count argument is checked as a pairing
     question.
   The CreateFileW form gets its Windows rows in the file family.
6. **Past the CreateFileW form** (amended 2026-10-12, before the work). With item 5's rows (fonts listed by probe run
   38222954208 and copied by 38226907442; MB_ERR_INVALID_CHARS on code page 1252, run 38226019630; FILE_FLAG_SEQUENTIAL_SCAN,
   run 38228020392) the frontier runs 50,784,721,550 instructions, writes 61 lines of unity.log equal to Windows' but
   for two timings, and stops at LoadLibrary of steam_api64.dll: static TLS in a DLL loaded after the start, which
   Windows supports and w2m refuses. A probe (probes/late_tls) records the index, each thread's block and DllMain's
   view; the loader gives every thread the block and the index, as the rows show. Main-thread seq 3422-3426
   (LoadImageW, GetFileInformationByHandleEx) lie past this load.
   Set 065's judge: 13 of the 15 classes equal the Xeon 8573C on every case (the 8370C agrees bit for bit); FSIN and
   FCOS differ in the last bit on 18 of 1,598 cases near a rounding boundary, where Intel publishes the 66-bit pi and
   not the polynomial. The GC's 0x56000 heap growth happens on the main thread in all four Windows runs and on the
   second thread in w2m (seq 2744): open.

7. **The MMX-register forms** (amended 2026-10-12, after the work had begun: the amendment is late, the work
   followed plan row 065, "MMX and x87"). With items 5 and 6 the frontier runs 53,207,065,370 instructions, writes 79
   lines of unity.log equal to Windows' but for two timings, and stops at `MOVQ mm1, m64` in UnityPlayer.dll. The
   census of 2026-10-12 lists 21 classes with MMX forms unlifted (MOVQ 444, PADDW 202, MOVD 85 ...) and EMMS and
   MOVDQ2Q missing. Decode spec 0.10.0 gives the 30 rows their N, P and Q operands; src/w2m/mmx.py runs them on the
   x87 register file; set 065b (22,083 cases, format 4) judges them on the Xeon 8573C (run 38234338405).
8. **Past the MMX forms** (amended 2026-10-12, after the work: late again). Frontier
   643a6283510c-1791802134771149000 (tree 7ee6c559; commit 4aa8889c's message names 56b88ceb, wrongly) runs
   73,274,486,656 instructions, writes 93 lines of unity.log equal to Windows' but for one timing, and stops at
   `dxgi.dll!W2mOutputGetDisplayModeList`. It runs 9.14M instructions a second over 8,013 s, against 131M before:
   48.7M slow-SSE exits and 19.3M model steps, each followed by a dispatch miss (70.3M hvc #1).
   - **Speed.** PUNPCKHQDQ (ZIP2), STMXCSR and LDMXCSR are lowered in A64 (a9dd06a5); run_frontier counts model exits
     by class, since the site census named 4M of the 48.7M. Frontier 643a6283510c-1791810726153034000's census names
     PACKSSWB, PADDUSB, PCMPGTB, MOVLPS, PMULHW, PSHUFB and PADDB (39.5M slow-SSE exits) and the MMX forms (18.0M model
     steps, MOVQ 11.6M); a3373038 lowers them, the MMX forms on the state block's x87 slots when TOP is 0 and no x87
     exception is pending. PSHUFW enters the registry as an MMX-only class, judged by set 065c.
   - **GetDisplayModeList.** The dxgi probe records the runners' 13 modes (runs 38254500336, 38254503636); the game's
     unity.log prints the same 13. Contract windows.dxgi.dxgi 0.2.0.
   - **HeapReAlloc in place (seq 3265-3267).** Windows refuses UnityPlayer's growth of a 0x417 block to 0x817; w2m
     granted it from a reused block's capacity. Probe heap_in_place (runs 38254640218, 38254643501): a block grows in
     place to at most the larger of its size and 16. Contract windows.heap.in_place 0.1.0.
   - **InterlockedPopEntrySList (seq 3132)** and the GC's heap growth (seq 2744, 2861): the order in which threads
     reach shared lists. In the three Windows runs no pop finds an entry and list +0x80 is never popped; on w2m a
     worker pops +0x80 after the main thread's push. Threads share one vCPU in 200,000-instruction slices; the runners
     run them on several cores. Open, for the threads sprint (plan row 066).
   - **Job pools in the comparison.** Frontier 643a6283510c-1791810726153034000 (tree a9dd06a5) runs 73,496,378,143
     instructions and stops at GetDisplayModeList; its comparison shows nine HeapSize divergences (seq 2548-2575).
     Windows' Job.Worker 1 made the walk, w2m's Job.Worker 0 made it with the same 32 answers, and pairing by name set
     it against Job.Worker 1's later walk. scripts/call_compare.py pairs a pool's workers per function by agreement.
   - **Past GetDisplayModeList.** Frontier 643a6283510c-1791818907808872000 (tree a3373038) runs 73,535,703,658
     instructions at 61.97M a second, writes 158 lines of unity.log and stops at EnumDisplaySettingsW of
     \\\\.\\DISPLAY1: display probe runs 38273510163 and 38273513428, contract windows.display.display 0.2.0. Its
     comparison shows WM_ACTIVATEAPP's lParam and WM_NCACTIVATE's wParam 1 where Windows has 0. Seventeen ways of
     starting probe activation give a thread and 1; game run 38275716702, with a WH_CALLWNDPROC hook in jit_capture,
     shows Windows sent 0 and 0 while a Windows Terminal window kept the foreground. The VM's foreground_held
     (FOREGROUND_MODE) says so for the frontier; the probes take the foreground.
9. **The red team of 2026-10-12** (amended 2026-10-12: items a to d after the work, late; item e before it). The
   reviewer's large review and the frontier records show:
   a. The game-call comparison never reported a Windows call w2m does not make (21 in frontier
      643a6283510c-1791830368398119000). It now reports each as `missing` (d84aada0).
   b. 14 of the 16 Background Job.Worker waits on one semaphore never returned in w2m: a release raised the count and
      the running worker took each unit first. Probe wait_handoff (runs 38304235237, 38304238413) shows Windows hands
      the object to its waiters at the release, first waiter first; sync_objects.c does so (68a5b9a0), family
      wait_handoff 0.1.0.
   c. The frontier's stop: past the reference's 168 lines the run goes on until its main thread has made the Windows
      main thread's calls, at most 20 billion instructions; a harness stop is `killed`; run_frontier exits non-zero on
      a divergence or a broken record; DecodeError is a stop (81755030).
   d. Frontier 643a6283510c-1791838458921720000 (tree 81755030) writes unity.log equal to Windows' 168 lines but for
      timings, line 160 included (16733); run alone, frontier 643a6283510c-1791836968921539000 (tree f9c0bbf3) gave
      16736. A scratch run hashing each translated block's guest bytes at compile and at the stop found 11,365 blocks
      whose bytes had changed, all in mono's JIT region from 0x1180cc0000.
   e. Translated code is invalidated when its guest bytes change (review of 2026-10-12 R3; plan row 067's work,
      brought forward by item d): the pages behind translated blocks are write-protected in the guest's stage-1 tables;
      a write to one drops the blocks over that page and their dispatch slots, unprotects it and resumes; the next
      execution translates afresh. Judged by a probe that writes code, runs it, rewrites it and runs it again, and by
      the frontier: no stale block at the stop, line 160 equal to Windows' in runs alone and alongside.
   f. (After the work.) Frontier 643a6283510c-1791842693323742000 (tree 429d9bf5) is the first full run with
      translations dropped when mono rewrites code: 13,681 writes to translated code, 60,082 blocks dropped, 31 split
      tables (the first try, 643a6283510c-1791841443768764000, stopped on the 64-table area at 50.5 billion
      instructions; 17bbbf55 gives splits 1,024). A model step and a thread switch now go on through the dispatcher's
      table (7c8f72ef): 1,487,645 dispatch misses where the run on f9c0bbf3 had 9,984,088. Its line 160 reads 16736,
      not Windows' 16733; commit 8dc3e19f's message says the log equals Windows' but for timings, which is wrong.
      Line 160 is 16733 in the two runs that shared the machine (81755030, stale code and all) and 16736 in the runs
      alone, stale code or not: the count follows the run's timing, not the translations. The guest clock is the
      host's (process.c, CNTVCT_EL0); v0.2 §3.1 asks for a controlled one.
10. **A clock the run controls** (amended 2026-10-12, before the work; v0.2 §3.1: "A controlled replay can inject
    equivalent clock observations"). Line 160 and the thread-order divergences follow the run's timing (item 9f); the
    guest reads the host's clock while it runs 27 to 600 times slower than Windows. The model is QEMU's `-icount
    shift=N,sleep=off` (qemu-options.hx: "The virtual cpu will execute one instruction every 2^N ns of virtual time";
    with sleep=off "the virtual time will jump to the next timer deadline instantly whenever the virtual cpu goes to
    sleep mode ... This behavior gives deterministic execution times from the guest point of view"), which also warns
    that instruction counts have "little or no correlation with actual performance". Clock mode 2: the 10 MHz counter
    is the VM's executed instructions at a set rate, plus a warp; when no thread can run, the warp moves the counter to
    the earliest deadline and that thread wakes, with no real sleep; the host's random bytes come from a fixed seed.
    The rate is set from one run, so that a duration unity.log prints matches Windows'. Judged: two frontier runs of
    one tree give the same calls and the same log; line 160 against the eight Windows runs; the comparison's
    divergences.

## prerequisites

- 064 (closed partial: CI run 38219168662; main 876c44cf)

## context_files

- `probes/semantics/semantics.c`, `scripts/semantics_differential.py`, `src/w2m/x86fp.py`, `src/w2m/runtime.py`,
  `src/w2m/lift.py`, `specs/x86_decode.json` (the x87 rows), `specs/guest_state.json`, `src/w2m/vm_backend.py`
  (HVC_SLOW_STEP), `scripts/class_run.py`; Intel SDM Vol 1 ch. 8, Vol 2 (each class's page)

## signal contract

### Emits

- The case file's PROBE_AUTHORED; each x87 class's item run (SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED
  for the model and the VM, CANDIDATE_ACCEPTED, SESSION_COMPLETED); the frontier's records.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: the x87 rule module and its tests; the case file format 4
- modified: `probes/semantics/semantics.c`, `scripts/semantics_differential.py`, `src/w2m/x86fp.py`,
  `src/w2m/runtime.py`, `src/w2m/lift.py`, `src/w2m/vm_backend.py`, `specs/guest_state.json`
- gates 0 and CI `success` on the closing commit

## observation contract

- **Every class in the table equals the Xeon 8573C** on its cases, in the model and in the VM.
- **The frontier passes the FLD and seq 3426,** conforming.

## done criteria

The table's classes accepted; the frontier past seq 3426 with a conforming comparison.
