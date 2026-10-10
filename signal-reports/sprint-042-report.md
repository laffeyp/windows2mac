# SIGNAL_REPORT — Sprint 042

## 1. Observed

### scope_confirmation

> One Windows function now runs through the whole pipeline. A no-CRT probe recorded what Windows does with CreateFileW and the five file calls the game makes. A contract was built from those rows. The implementation sits in the guest runtime, over a bridge to the Mac, and a judge runs Windows's own probe binary in the VM. The VM prints all 188 of Windows's rows. As a measurement, a cloud model working in substrate's session topology wrote an implementation the same judge accepted.

### work_performed

- **The probe** (`probes/create_file/`) prints one JSON row per scenario: the five dispositions on absent and existing files, the 144-pair sharing matrix, DeleteFileW on open files, five name forms, and ReadFile, WriteFile and SetFilePointerEx on one handle. Server 2025 and Server 2022 printed the same 188 rows.
- **The contract** `windows.file.create_file.dispositions_sharing@0.1.0` holds those rows, four quoted Microsoft Learn pages, and the four places where Windows goes beyond or against them. Its acceptance form is `exact_rows` and its event relation is `exact`.
- **The implementation:** `tools/guest_runtime/file.c` holds the dispositions, the sharing rule, POSIX delete, the name rules and the last-error behavior. `src/w2m/file_bridge.py` holds POSIX primitives under a per-run root, with names compared ASCII case-folded.
- **The judge:** `scripts/file_run.py` runs an item run on substrate. Its comparisons run in the managed VM, one per runner binary, and decide once.
- **On the way:** a NEG rule and case set 042, judged by a Xeon 8573C; the decide trigger fixed to `Once()` with all 141 classes rerun; substrate used from its clone on branch `w2m`.
- **The measurement:** `scripts/measure_file_generation.py` runs `session_topology` with kimi-k2.7-code:cloud, a workspace, and the tools `build` and `judge`.

### signal_trace

```
store/runs/file-createfilew-1791614609022743000 (indexed in file-runs.json): SESSION_INIT, CANDIDATE_GENERATED (file.c sha256:c2100…), COMPARISON_COMPLETED x2 (exact, conforms), CANDIDATE_ACCEPTED x1, SESSION_COMPLETED
store/runs/vmoracle-1791614555918886000 (set 042, Xeon 8573C): COMPARISON_COMPLETED conforms, 618 cases, 102 faults
store/runs/class-*-1791614599* (141 runs): one CANDIDATE_ACCEPTED each
store/runs/file-generation-session-kimi-k2.7-code-cloud-1791616818397796000: 46 ModelReply, 45 ToolCall, 45 ToolResult, Returned (replied), SessionEnded (user_exit); its 7th judge produced store/runs/file-createfilew-1791617115021734000 (CANDIDATE_ACCEPTED)
```

## 2. Expected

> The card expected Windows's rows to be recorded on two machines, the VM to equal them, planted faults to be caught by the rows alone, and the measurement to be on a record with nothing depending on it. Every expectation held. One card claim was wrong: `rows_equal` as a relation value would have changed the vocabulary.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass. The relation is `exact`, and vocabulary 0.7 is unchanged. **Artifact contract:** `scripts/gates.sh` exits 0 (306 tests; 441 records replay). **Observation contract:**

- **Windows recorded:** 188 rows, equal across two builds and four executions. Pass.
- **The VM equals Windows:** both binaries, all rows. Pass.
- **Planted faults:** each changed exactly the rows its reach named: 35 sharing rows, the 2 ERROR_ALREADY_EXISTS rows, and the 2 trailing-name rows. The case-sensitive plant stopped the run at the case scenario, where its reach said it would. Pass.
- **The measurement:** model, tokens, tool calls and every judgment are on the session record. Pass.

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** the runners printed Windows's rows, and the rows became the contract. The item run's two comparisons each ran a runner binary in the VM over file.c, and the decision followed both. In the session, each model call caused one tool call, and the seventh judge started an item run that accepted.

- *Observation 1 (the documentation is a hypothesis):* POSIX delete, trailing-name stripping and error 131 came from the rows, not the pages. Disposition: resolved-here; the contract's open questions name each one; KIT_DIARY #27.
- *Observation 2 (a record that said "accepted" twice):* every Sprint 041 class record decided twice, and the test only looked for the event. Disposition: resolved-here (`Once()`, counted in tests); KIT_DIARY #26.
- *Observation 3 (the model as a tester):* the model's code found two w2m defects that no test had reached. A runtime missing a service failed late with KeyError. A path containing a NUL crashed the run with ValueError. Disposition: resolved-here, with tests.
- *Observation 4 (the measurement's shape decides its result):* the first hand loop failed 4 of 4, because its retries carried only the error and not the model's own code. Disposition: recorded. The measurement now runs in substrate's session topology, where the model holds its own history.

### status_and_blockers

`status: complete`
