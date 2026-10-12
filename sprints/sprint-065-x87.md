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
