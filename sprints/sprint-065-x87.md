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
     by class, since the site census named 4M of the 48.7M.
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
