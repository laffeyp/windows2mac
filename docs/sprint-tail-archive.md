# Sprint tail archive

Entries rolled out of `BLACKBOARD.md` `## Sprint tail` on 2026-10-08 (Sprint 027) and later, verbatim, newest first. The tail keeps the 10 most recent (`sdd-kit-3/templates/BLACKBOARD.md`); each sprint's summary also stands in `## Built`.

### Sprint 023 (2026-10-08, closed)
- **Scope:** decoder phase close; review items 4.3, 4.4, 4.2.
- **Rubber Duck Pass:**
  - *Observation 1 (a measuring tool that hid its own blind spot):* `encgen` never put a prefix before a VEX/XOP/EVEX escape, so 1,170 accepted-but-invalid cases were invisible to every differential since Sprint 020. The review found the class by reading SDM §2.3.2. Disposition: resolved-here; KIT_DIARY finding 10.
  - *Observation 2 (a review claim holds only in part):* zero-fill between raw and virtual size is the loader's behavior and stays; bytes past the section end were invented and are gone. Disposition: resolved-here.
  - *Observation 3 (hashing found a spec defect):* keying confirmations by row hash exposed three duplicate rows. Disposition: resolved-here; generator refuses duplicates.
- **Closed:** clean.

### Sprint 019 (2026-10-08, closed)
- **Scope:** EVEX and XOP prefix decoding; last corpus gaps.
- **Rubber Duck Pass:**
  - *Observation 1:* EVEX and XOP decoded correctly on the first run; one missing row (EVEX vpmaddwd) found by the differential and added. Disposition: resolved-here.
  - *Observation 2 (what this does not show):* full agreement on 2.44 M compiler-produced instructions. The literature (Paleari 2010; Woodruff et al. 2021) places decoder bugs in unusual encodings this corpus does not contain. Disposition: deferred to Sprint 020, generated encodings per spec row.
- **Files:** specs/x86_decode.json, scripts/gen_decoder.py, src/w2m/x86dec.py, src/w2m/x86dec_tables.py, tests/test_x86dec.py
- **Closed:** clean.
### Sprint 018 (2026-10-08, closed)
- **Scope:** XED-style iclass identity, corpus gaps closed except EVEX/XOP, CPU-feature dependence.
- **Rubber Duck Pass:**
  - *Observation 1 (a reference artifact that was a fact):* the four iclass differences on first run came from XED decoding for an unspecified chip. The bytes F3 0F BC are TZCNT on BMI1 CPUs and BSF on older ones; the results differ for a zero input. Disposition: recorded — Decision 2026-10-08 (decoder takes the CPU feature set); this is a semantic, not cosmetic, dependence.
  - *Observation 2 (independence held):* iclass names were assigned by rule before comparison; 99.997% matched XED on first run, and the residue was the chip effect, not naming errors.
  - *Observation 3 (remaining gaps):* 343 positions, 25 kinds, all EVEX or XOP. Disposition: deferred — EVEX/XOP prefix parsing is the next decoder sprint (implementation repair first, then rows).
  - *Observation 4 (vocabulary gap):* the generated-encoding test (mishegos-style, decision 2026-10-08 item 3) has not run; corpus agreement still says nothing about unusual encodings. Disposition: deferred — next after EVEX/XOP.
- **Files:** specs/x86_decode.json, scripts/gen_decoder.py, src/w2m/x86dec.py, src/w2m/x86dec_tables.py, tools/xedshim/xedshim.c, src/w2m/xedref.py, tests/test_x86dec.py, tests/test_xedref.py, scripts/decoder_differential.py
- **Closed:** clean.
### Sprint 017 (2026-10-08, closed)
- **Scope:** XED built from source and made the judge of length in the differential.
- **Rubber Duck Pass:**
  - *Observation 1 (what agreement means):* on 2.44 M compiler-produced instructions no decoder disagrees with XED on length. Disposition: recorded — consistent with the literature (decoder bugs live in unusual encodings); generated encodings are the next test, not more corpus.
  - *Observation 2 (traversal, not decoder):* the 15 Capstone failures are ASCII credit strings inside libcrypto's `.text`. Disposition: deferred — the traversal needs data detection for hand-written assembly (stop at bytes all decoders refuse; mark the region data); Deferred entry.
  - *Observation 3 (identity):* XED's iclass names separate what mnemonics conflate (MOVSD vs MOVSD_XMM). Disposition: deferred to Sprint 018 — the spec adopts XED iclass as each row's identity (a contract revision of `specs/x86_decode.json`).
- **Files:** tools/xedshim/xedshim.c, src/w2m/xedref.py, tests/test_xedref.py, scripts/decoder_differential.py
- **Closed:** clean.

### Sprint 016 (2026-10-08, closed)
- **Scope:** generalization test of the spec-generated decoder on 33 binaries it was not written from.
- **Rubber Duck Pass:**
  - *Observation 1 (vocabulary gap → worklist):* 203 gap kinds, all "no entry for opcode", none a structural failure; `specs/x86_decode_worklist.json` lists each with count and example bytes. Disposition: deferred — next sprint adds rows from the SDM tables, highest counts first, and reruns.
  - *Observation 2 (payload anomaly, real bug):* VEX.L is not modeled; 0F 77 with L=1 is vzeroall. Disposition: deferred to the same sprint (spec schema gains an `l` constraint).
  - *Observation 3 (reference uncertainty):* `repz ret` is Capstone's spelling of F3 C3; Intel's manuals describe REP with RET as having no defined effect. Disposition: recorded — mnemonic spelling for prefix-on-non-string instructions follows the reference; the spec gains a rule rather than a guess.
  - *Observation 4 (reference failure):* Capstone cannot decode 15 reachable locations. Disposition: deferred — check each with LLVM; where LLVM decodes them, the reference is wrong there.
- **Files:** scripts/decoder_differential.py, specs/x86_decode_worklist.json, corpus/python-3.14.8.manifest.json
- **Closed:** clean.

### Sprint 015 (2026-10-08, closed)
- **Scope:** spec-generated x86-64 decoder, compared with Capstone at every reachable instruction.
- **Rubber Duck Pass:**
  - *Observation 1 (the differential's first finding):* 94 instructions, one class: F2 0F 10/11 (SSE scalar-double move, mnemonic `movsd`) was printed `repne movsd` because `movsd` also names the string move A5, for which F2 is a repeat prefix. Disposition: resolved-here — REP fusion is limited to the one-byte map. The two-decoder plan found a name collision on its first run.
  - *Observation 2 (selection effect):* the spec was authored from the observed mnemonic list, so 100% here is not evidence of generality. Disposition: deferred — next sprint runs both decoders on binaries neither the spec nor its author has seen; DecodeError and disagreement counts become the worklist.
  - *Observation 3 (vocabulary gap):* operands are not compared yet; the translator needs operand semantics, not mnemonics. Disposition: deferred — operand comparison is a later layer of the same differential.
- **Files:** specs/x86_decode.json, scripts/gen_decoder.py, src/w2m/x86dec.py, src/w2m/x86dec_tables.py, tests/test_x86dec.py
- **Closed:** clean.

### Sprint 014 (2026-10-08, closed)
- **Scope:** Capstone reference traversal; Capstone graded against LLVM.
- **Rubber Duck Pass:**
  - *Observation 1 (payload anomaly, resolved-here):* the first Capstone/LLVM comparison showed 42-47 length disagreements per binary; every one was LLVM printing `lock` as a 1-byte line. A second pass mis-folded `rep` (LLVM prints `rep` with its instruction on one line). After normalizing the printing, agreement is total. Lesson recorded: disagreements are first checked for presentation before they count as decoder facts.
  - *Observation 2 (vocabulary gap → translator input):* two imports in event_auto_reset.exe are never called through their IAT slot: the compiler loads the slot into a callee-saved register (`mov r12, [rip+slot]` … `call r12` for CreateThread; `mov rsi, [rip+slot]` … `call rsi` for RegGetValueW) and calls through it. The traversal counted these as unresolved indirect calls. Disposition: recorded — import resolution needs one step of register data flow; the agent's first explanation ("passed as a value") was a guess and was replaced by the instruction listing.
  - *Observation 3:* 42 and 47 indirect transfers remain unresolved by static traversal (jump tables, function pointers). Disposition: deferred to the dynamic discovery path (v0.2 §5.1).
- **Files:** src/w2m/x86ref.py, tests/test_x86ref.py, src/w2m/pe.py, pyproject.toml
- **Closed:** clean.

### Sprint 013 (2026-10-08, closed)
- **Scope:** PE32+ reader graded against llvm-objdump on two runner-built binaries.
- **Rubber Duck Pass:**
  - *Observation 1 (vocabulary gap → contract inventory):* event_auto_reset.exe's source calls about a dozen Windows functions; the image imports 81 (KERNEL32 79, ADVAPI32 1, bcrypt 1). The statically linked C runtime contributes the rest. v0.2 §7: import scans are a coverage view; the first real inventory is dominated by CRT startup and I/O, not program logic. Disposition: recorded — the contract inventory sprint takes CRT entry points as first-class members.
  - *Observation 2 (resolved-here):* the first download loop fetched one run's artifact under both names; the probes were re-fetched by run id and their sha256 compared to the runner's.
  - *Observation 3 (resolved-here):* mypy flagged the data-directory list as variable-length tuples; typed explicitly.
- **Files:** src/w2m/pe.py, tests/test_pe.py, store/binaries/
- **Closed:** clean.

### Sprint 012 (2026-10-08, closed)
- **Scope:** v0.2 §8's two-page example on this Mac: correctness on both backends, then execution time per access.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass.
- **Rubber Duck Pass:**
  - *Sequence narration:* SESSION_INIT (mac-Mac17,7-macos-25F80, capability). CAPABILITY_PROBED native_mprotect_4k absent (EINVAL); native_page_check_4k present; guest_page_protection_4k present (EC 0x25, FAR 0x20001000). 10 MEASUREMENT_RECORDED. SESSION_COMPLETED.
  - *Observation 1 (timing surprise, resolved-here):* the first check_page_cross ran at 1.9x — as much as checking every access — because its common path took a branch on 1023 of 1024 elements, doubling taken branches per iteration. Moving the page-entry check out of line made it 1.04x. The first number measured code layout, not the check.
  - *Observation 2 (vocabulary gap, deferred):* check_page_cross is valid only while no other thread changes protection during the loop. Windows lets any thread call VirtualProtect at any time, so a hoisted check needs an invalidation rule (a protection epoch the loop re-checks, or re-translation on change) — the v0.2 §5.1 code-validity machinery applied to data protection. Deferred entry added.
  - *Observation 3 (vocabulary, resolved-here):* the three capabilities had no enum values; P10 ratified into 0.3 within the sprint.
- **Files:** probes/page_geometry/page_geometry.c, scripts/page_geometry.py, signals/0.3.json, signals/0.3-rationale.md
- **Closed:** clean.

### Sprint 011 (2026-10-07, closed)
- **Scope:** execution time of x86-TSO ordering lowerings: native lowerings vs VM hardware TSO, L1 and DRAM-sized working sets.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass, after two corrections the observation itself forced.
- **Rubber Duck Pass:**
  - *Observation 1 (payload anomaly, resolved-here):* first full run showed the VM 6.2x slower than native on the DRAM-sized set. The VM path had no warm pass while the native path did; the number included first-touch stage-2 faults on 128 MiB. With a warm pass the VM is 1.03-1.10x. Disposition: resolved-here; the 6.2x figure was never recorded as a measurement.
  - *Observation 2 (missing pair, resolved-here):* the native MP precondition passed with plain 0/10,000,000 — a check with no power. The driver now requires plain > 0 in the same attempt (attempt 7 had 9).
  - *Observation 3 (timing surprise):* absolute ns/access moved up to 50% between runs (L1 plain 0.087 then 0.131), ratios held within 0.01. Cause not established; likely core placement or clock state of an unpinned thread. Disposition: deferred — MEASUREMENT_RECORDED values are only comparable within one record; Drift watchlist entry.
  - *Observation 4 (vocabulary gap):* VM entso_on vs entso_off on the DRAM set differs by 7% at the median but only 1.4% at the minimum; not separable from noise with 5 repeats. Disposition: recorded — TSO hardware cost reported as "under 10% on this kernel", not as a number.
- **Files:** probes/ordering_cost/ordering_cost.c, scripts/ordering_cost.py
- **Closed:** clean.

### Sprint 010 (2026-10-07, closed)
- **Scope:** vocabulary 0.1 → 0.2 by proposal (P8, P9); version wiring through generator, gate, ingest.
- **Dual contract:** signal vacuous; artifact pass (see Built).
- **Rubber Duck Pass:**
  - *Observation 1 (order violation, hard rule 6):* the sprint touched 12 files. Disposition: recorded — a version bump must land whole, or the gate fails between sprints; stated in the card before execution.
  - *Observation 2 (payload anomaly):* TRACE_INGESTED.vocab_version previously echoed the harness version, not the trace's; under one version they coincided and the error was invisible. Disposition: resolved-here (records the trace's own version; tested).
- **Closed:** clean.

### Sprint 009 (2026-10-07, closed)
- **Scope:** demonstrate that ACTLR_EL1.EnTSO changes guest memory ordering; MP litmus on two vCPUs, rounds alternating off/on.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass — the "effect demonstrated" branch.
- **Rubber Duck Pass:**
  - *Sequence narration:* SESSION_INIT (mac-Mac17,7-macos-25F80, capability). CAPABILITY_PROBED guest_tsoen present: off 2245/50,000,000, on 0/50,000,000, bound 6.0e-08, effect demonstrated. CAPABILITY_PROBED guest_tsoen_context_switch present: 200 exits, 0 mismatches. SESSION_COMPLETED.
  - *Observation 1 (timing surprise):* the off-mode rate was 1.2e-03 in a 4-round trial (233/200,000) and 4.5e-05 in the 100-round run. Disposition: recorded — the rate depends on timing and round length; the claim rests on off > 0 and on = 0 in alternating rounds, not on the rate.
  - *Observation 2 (vocabulary gap):* MP checks load-load and store-store order only. x86-TSO also *allows* store-load reordering (SB), and a backend must not be stronger than it claims to measure nor weaker than TSO elsewhere (LB, 2+2W, IRIW). Disposition: deferred — the herdtools7 x86-TSO suite on two to four vCPUs is a later sprint; Deferred entry added.
  - *Observation 3 (payload anomaly):* the guest MMU had to be on: with SCTLR_EL1.M=0, EL1 data accesses are Device-nGnRnE, which would have produced a false "TSO" result in the off mode. Disposition: resolved-here (identity map, Normal WB ISH).
- **Files:** probes/mac_hv_litmus/mp_litmus.c, scripts/litmus_mp.py
- **Closed:** clean.

### Sprint 008 (2026-10-07, closed)
- **Scope:** Hypervisor.framework capability probe on this Mac; CAPABILITY_PROBED on a substrate record.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass.
- **Rubber Duck Pass:**
  - *Sequence narration:* SESSION_INIT (profile mac-Mac17,7-macos-25F80, purpose capability). CAPABILITY_PROBED ipa_granule_4k present; guest_tsoen present; guest_tsoen_context_switch present; host_page_16k present; native_process_tso absent. SESSION_COMPLETED (completed).
  - *Observation 1 (tone trace):* a detail string began with "ACTLR_EL1", against E12 (lowercase first word). Disposition: resolved-here.
  - *Observation 2 (vocabulary gap):* `guest_tsoen_context_switch` was observed across one vCPU exit, not across host thread migration or many exits; the capability name claims more than one exit shows. Disposition: deferred — the litmus sprint runs thousands of exits on two vCPUs and re-records it.
  - *Observation 3 (timing):* the probe and the driver ran first time; Hypervisor headers in the installed SDK were the bridge mapping (TECHNIQUES.md #46), no symbol was guessed.
- **Files:** probes/mac_hv_capability/hv_capability.c, probes/mac_hv_capability/entitlements.plist, scripts/probe_capabilities.py
- **Closed:** clean.

### Sprint 007 (2026-10-07, closed)
- **Scope:** the event contract's probe on real x64 Windows, 10 executions, judged on one substrate record.
- **Dual contract:** signal pass; artifact pass (see Built).
- **Observation contract:** pass. Judge record: 106 events, 21 producers started, 0 failed; 10 of 10 conforms. profile_id `gha-windows-2025-x64-10.0.26100.33438-Server`.
- **Rubber Duck Pass:**
  - *Sequence narration (evt-1):* main 6680 started; SetEvent returned 1; join of waiter 5880 returned 0 after 5880:2; join of 4328 returned 0 after 4328:2; WaitForSingleObject(event,0) returned 258. Waiter a's wait returned 0 citing main's SetEvent line; waiter b's returned 258.
  - *Observation 1 (timing surprise):* waiter a released in 10 of 10 executions; the profile looks FIFO. Disposition: recorded — documented evidence keeps {b} allowed (v0.2 §11.1: repeated observations are witnesses, not rules); open question added to 0.2.0 for a wait-order variant.
  - *Observation 2 (payload anomaly, resolved):* the Sprint 006 watchlist item: profile_id now carries UBR 33438 and `Server`. Disposition: resolved-here; watchlist item closed.
  - *Observation 3 (missing pair):* the imported executions carry `elapsed_seconds=0.0` on EXECUTION_COMPLETED and timeout 600 s (job limit) on EXECUTION_STARTED, not measured values, because the probe ran on the runner, not under the harness clock. Disposition: deferred — the probe writes its own elapsed time in a later probe-format revision.
- **Files:** probes/event_auto_reset/event_auto_reset.c, scripts/judge_traces.py, .github/workflows/windows-probe.yml, contracts/.../0.2.0.json
- **Closed:** clean.

### Sprint 006 (2026-10-07, closed)
- **Scope:** private repo; hello probe built and run on the `windows-2025` hosted runner; trace ingested on the Mac.
- **Dual contract:** signal pass; artifact pass (run 37718681787 conclusion success; ingest TRACE_INGESTED line_count=8 thread_count=2; `uv run pytest` 38 passed).
- **Observation contract:** pass. profile_id `gha-windows-2025-x64-10.0.26100`; runner Get-FileHash FD933867...555A equals Mac shasum.
- **Rubber Duck Pass:**
  - *Sequence narration (guest plane):* OPENED hello-001 on 10.0.26100. Thread 6588 started, read QPC 2267788232. Thread 8820 started after 6588:1, read QPC 2267788974, exited. Thread 6588 read QPC 2267789575 after 8820:2. CLOSED with 8 lines, 2 threads, 0 dropped.
  - *Observation 1 (order):* QPC values rise along both hb edges (…232 < …974 < …575), as the edges require. Disposition: resolved-here (no fault; first observed evidence that QPC is consistent across threads on this profile — one run, a witness, not a rule).
  - *Observation 2 (payload anomaly):* profile_id carries 10.0.26100 but not the update build revision (the runner README lists Build 33438) nor that the OS is Windows Server, not a client SKU. v0.2 §11.1 asks for the Windows build. Disposition: deferred — the next probe reads UBR and product type from the registry; Drift watchlist.
  - *Observation 3 (timing):* first dispatch (run 37718622857) failed at build: MSVC CRT deprecates `fopen` and `-Werror` promoted it. Disposition: resolved-here (`_CRT_SECURE_NO_WARNINGS`, commit after 052383a).
- **Files:** probes/hello/hello.c, .github/workflows/windows-probe.yml, .gitignore
- **Closed:** clean.

### Sprint 005 (2026-10-05, closed)
- **Scope:** comparator for allowed_outcome_set; substrate Producer triggered by TRACE_INGESTED.
- **Dual contract:** signal pass; artifact pass (`uv run pytest` 37 passed; ruff, mypy exit 0).
- **Observation contract:** pass. {b} trace → `COMPARISON_COMPLETED(verdict=conforms, relation=allowed_outcome_set)`; {a,b} trace → `verdict=discrepancy`, first_divergence "forbidden outcome both_waits_complete_from_the_single_signal: ['a', 'b'] at ['2:1', '3:1']".
- **Rubber Duck Pass:**
  - *Sequence narration:* EXECUTION_COMPLETED (e1, exited) → a probe trace exists. TRACE_INGESTED (same hash) → the trace is evidence-grade. COMPARISON_COMPLETED (cmp1, contract 0.1.0, verdict) → the trace was judged against the proposed contract.
  - *Observation 1 (payload anomaly):* `reference_ref="ref-e0"` names no reference execution; constraint E13 cannot resolve it. Disposition: deferred — no reference run exists until the Windows reference instance does; Drift watchlist entry added.
  - *Observation 2 (order):* the comparator re-decodes the trace; a malformed trace would raise inside the Producer. Disposition: resolved-here by construction — the comparator is triggered only by TRACE_INGESTED (S4 chain), so it only reads validated traces.
- **Files:** src/w2m/compare.py, tests/test_compare.py
- **Closed:** clean.

### Sprint 004 (2026-10-05, closed)
- **Scope:** contract format; first contract from v0.2 §11.2.
- **Dual contract:** signal vacuous; artifact pass (27 passed; ruff, mypy exit 0).
- **Rubber Duck Pass:** *Observation 1 (vocabulary gap):* the postcondition `signaled_after_consumption: false` is observable through GUEST_CALL_RETURNED (main's zero-timeout wait returning 258, WAIT_TIMEOUT per Microsoft Learn) — P4 not needed for this contract. Disposition: resolved-here. *Observation 2 (drift):* contract layout differs from the rationale's audit table. Disposition: recorded (Decision 2026-10-05).
- **Files:** src/w2m/contract.py, contracts/windows.event.auto_reset.single_signal/0.1.0.json, tests/test_contract.py
- **Closed:** clean.

### Sprint 003 (2026-10-05, closed)
- **Scope:** guest-trace ingest with hash, strict decode, T5, T6, S14, and hb_refs checks; substrate Producer wrapper.
- **Dual contract:** signal pass; artifact pass (`uv run pytest` 18 passed exit 0; ruff, mypy, `gen_events.py --check`, `check_vocab.py` all exit 0).
- **Observation contract:** pass. Record of `test_observation_ingested_on_the_record` narrated by `substrate narrate`: 12 events, 2 producers started, 2 completed, 0 failed.
- **Rubber Duck Pass:**
  - *Sequence narration:* seq 3 EXECUTION_STARTED (e1, windows_reference, timeout 60 s, grace 5 s) → reference_runner began a probe. seq 4 EXECUTION_COMPLETED (exited, 8 lines, trace sha256:fccec9fb...) → the probe ended and its trace exists. seq 5 trigger ingest-on-complete started trace_ingester. seq 8 TRACE_INGESTED (same hash, 8 lines, 2 threads, vocab 0.1) → T4 pairing satisfied.
  - *Observation 1 (payload anomaly):* `kill_grace_seconds` written as 5.0 is recorded as 5 (RFC 8785). Disposition: resolved-here — msgspec reads it back as 5.0; the round-trip is now asserted in the observation test.
  - *Observation 2 (timing surprise):* none; whole suite 0.13 s.
- **Files:** src/w2m/trace.py, tests/test_trace.py
- **Closed:** clean.

### Sprint 002 (2026-10-05, closed)
- **Scope:** generated Structs, `validate`, strict guest decoding.
- **Dual contract:** signal pass; artifact pass (7 tests; `--check`, ruff, mypy exit 0).
- **Rubber Duck Pass:**
  - *Observation 1 (payload anomaly):* msgspec cannot express a `le` above int64; the first `u64` alias crashed decoder construction. Disposition: resolved-here (u64 is `ge=0` only).
  - *Observation 2 (payload anomaly):* substrate refuses harness-plane integers beyond 2^53-1. Disposition: recorded — generated bound, Decision 2026-10-05, proposal P8.
  - *Observation 3 (vocabulary gap):* `types.version` text says major.minor.patch, but the vocabulary's own version is "0.1". Disposition: deferred — P8.
- **Files:** scripts/gen_events.py, src/w2m/events.py, tests/test_events.py
- **Closed:** clean.

### Sprint 001 (2026-10-05, closed)
- **Scope:** package scaffold and dependencies. **Dual contract:** signal vacuous; artifact pass. **Rubber Duck Pass:** vacuous (no runtime signals). **Closed:** clean.

### Sprint 000 (2026-10-05, closed)
- **Scope:** Vocabulary Session for w2m against `windows_to_macos_research_direction_v0_2.md`.
- **Dual contract:** signal vacuous (no runtime emission); artifact pass — `json.load` on `signals/0.1.json` exit 0, on `signals/proposals.json` exit 0; `python3 scripts/check_vocab.py` exit 0; assertions: SESSION_INIT carries vocab_version/profile/contract_version; DISCREPANCY_REPORTED is incident with contract_id/mismatch_class/minimized_program_sha256; mismatch_class enum is the six v0.2 §3 relations; rationale has `Dual-contract audit` and `Sign-off` sections.
- **Observation contract:** not applicable (architecture, content-only).
- **Rubber Duck Pass:**
  - *Sequence narration:* no runtime signals; the narrated trace is the session's own steps: Step 0 orientation, Layers 0-7 written, Step 9 audit, Steps 10-12 sign-off, gate run.
  - *Observation 1 (payload anomaly):* the rationale said "37 enums"; the file has 34. Disposition: resolved-here (rationale corrected; Built entry uses 34).
  - *Observation 2 (order violation, card scope):* `scripts/check_vocab.py` was authored though the card listed three files. The first gate run caught a false failure (READ as a non-past verb) that only a written allowlist fixes, and WORKING_AGREEMENT had no vocabulary gate command. Disposition: resolved-here — the file is the project's gate, registered in WORKING_AGREEMENT; one concept (the lock) kept.
  - *Observation 3 (vocabulary gap):* APPLICATION_LAUNCHED has no view-side counterpart. Disposition: deferred — proposal P2 WINDOW_PRESENTED; Drift watchlist.
  - *Observation 4 (citation slip):* the reference-machine Decision cited the wrong addendum item. Disposition: recorded — correction entry in `## Decisions`.
- **Files:** signals/0.1.json, signals/0.1-rationale.md, signals/proposals.json, scripts/check_vocab.py
- **Closed:** clean.

### Sprint 020 (2026-10-08, closed)
- **Scope:** generated encodings per spec row vs XED.
- **Rubber Duck Pass:**
  - *Observation 1 (the literature's prediction held):* full agreement on 2.44 M corpus instructions coexisted with 31% disagreement on generated encodings. Disposition: recorded — corpus agreement is not decoder correctness.
  - *Observation 2 (vocabulary gap → spec schema):* the spec has no validity layer; invalid encodings decode instead of producing #UD, which v0.2 §5.1 requires ("or produces the fault prescribed by the guest contract"). Disposition: deferred to Sprint 021 — LOCK rule, VEX/EVEX L/W/vvvv constraints from the SDM opcode column.
  - *Observation 3 (structure held):* length_differs 0 across all generated cases.
- **Closed:** clean.

### Sprint 021 (2026-10-08, closed)
- **Scope:** validity layer (#UD) and iclass edge rules from the generated differential.
- **Rubber Duck Pass:**
  - *Observation 1 (the agent's own test was wrong):* a test asserted `f0 01 03` is invalid; XED decodes it as `lock add dword ptr [rbx], eax` (opcode 01 has a memory destination). The intended case was `f0 03 03` (BAD_LOCK_PREFIX). Disposition: resolved-here — the test was checked against XED before being changed, and the decoder was right.
  - *Observation 2 (direction of error):* the strict-prefix rule turned silent misdecodes into 73 refusals (ours has no row where XED knows a newer instruction). Disposition: recorded — a refusal is honest; the rows are worklist items.
  - *Observation 3 (residue):* 11.38% accepted-but-invalid, mostly EVEX broadcast/zeroing/opmask rules and VEX/EVEX W constraints. Disposition: deferred — next decoder sprint; then operand comparison.
- **Closed:** clean.

### Sprint 022 (2026-10-08, closed)
- **Scope:** explicit operands, w2m (SDM Appendix A templates) vs XED, corpus and generated encodings.
- **Rubber Duck Pass:**
  - *Observation 1 (zero is a claim that needs a test):* the first corpus run reported 0 differences in 2.26 M comparisons. A planted fault (REX byte-register names without REX) produced 224 differences, so the comparison has power. Disposition: resolved-here; the planted-fault result is in the card.
  - *Observation 2 (the literature's prediction held again):* both template errors (f64 on FF /2, /4; MOVSXD `Ez`) appeared only in generated encodings, never in 2.44 M compiler-emitted instructions. Disposition: resolved-here — spec 0.6.1.
  - *Observation 3 (a recorded number looked wrong):* this run reported 2,391,671 positions against the 2,444,456 recorded since Sprint 017. The recorded figure includes the 52,785 probe instructions; both are now reported separately. Disposition: resolved-here.
  - *Observation 4 (coverage):* 5.4% of corpus positions have no template (no-operand rows, SSE/AVX). Disposition: deferred — next decoder sprint, with the AVX-512 validity tail.
- **Closed:** clean.
