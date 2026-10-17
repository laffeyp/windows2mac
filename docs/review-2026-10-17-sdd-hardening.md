# SDD hardening review, 2026-10-17

The user pasted this review in chat on 2026-10-17 ("plus this"), after asking for an adherence check of Sprint
107 against SDD, Substrate and A Philosophy of Software Design. It is kept below as pasted. The paste lost text at
several line ends (for example "ary by gen_events.py--check", "whose events do happen"); the gaps are the paste's,
and nothing here fills them. Section 3 holds the adherence check made the same day; section 2 checks the review's
counts against the tree.

## 1. The review, as pasted

SDD hardening review, 2026-10-17

SDD is used correctly on half the project. The half that judges Windows behavior is real SDD. The half that does the current work (speed, the scheduler, vCPUs, the compiler, and now the pivot's system-call kernel) has almost no signals. Its ceremony continues on top of nothing. The kit itself names this failure: a Rubber Duck Pass without check surfaces "is doing comprehension theater".

Is SDD correctly used?

Correct, and strong:
- The guest plane. Each frontier run writes a typed trace. The latest run's trace has 10,546 lines and 28 threads, with 7,906 GUEST_CALL_RETURNED lines carrying the API, an args_sha256, the return value and the last error. It is compared call by call against 11 Windows runs, with first_divergence: null.
- Checked at ingest. src/w2m/trace.py rejects a trace that breaks T5 (first and last line), T6 (per-thread sequence) or S14 (nothing after close).
- Checked at the mouth. Python events are msgspec structs with forbid_unknown_fields, ary by gen_events.py--check. That is commitment 2 (validation at the speaker's mouth), done    right.
- The founding act. The vocabulary session ran: 0.1 through 0.8, each with a rationale doc, proposalgnals/proposals.json. Thevocabulary fills all eleven layers: 22 entities, 5 sessions, 7 temporal nvariants, 14 state ruence constraints.
- The judges. 73 contracts mined from Windows, family and class records, alanted faults. These a in the sense of technique #5.
Incorrect:
1. The signal contract is misread. AGENTS.md says every tag in Emits "mustfire during the sprint"None new" in 14 of 15cases, so the contract has become "vocabulary changes", and nothing grawhat the sprint's code4 (N vCPUs on their ownthreads) and Sprint 106 (one lock around the runtime) changed the      scheduler, and their croves it.
2. Rubber Duck Passes narrate nothing. Seven passes in the current sprint tail begin "Sequence nlowed by prose. Step 1 ofthe kit's pass is to walk every signal in time order, and none are walked.
3. Signal Reports stoppedorts/ holds 26 reports,the last for Sprint 048, while sprints reached 107. Searches of           BLACKBOARD, WORKING_AG found no Decision and nohand-author log entry for the stop. Hard rule 10 says a bypass is logged, never silent.
4. Most invariants are never checked. 3 of 21 are enforced (T5, T6, S14, all
   in trace.py). T1 to T4er checked against the32,241 records in store/runs/. check_replay.py checks stream integrity
   only, and check_vocab.ile's structure, not runs.
5. The vocabulary's map no longer matches the system. Foundation 01 says
   categories map to archion 0.8 has 13 producttags (Steam, install, update; 11 never emitted) and one perf tag. It has
   no category for the sc compiler's run time, orsystem calls. 24 of 62 tags are never emitted, including
   TRANSLATION_INVALIDATEwhose events do happen.
6. The C side is checked only at ingest. The guest runtime writes JSON lines
   by hand and trace.py cNCIPLES.md, commitment 2:"validation at ingest is a filter; validation at the mouth is a contract".

Are the signals usable?

For correctness, yes. The guest trace, the contracts and the comparison
records are the best signrgence names a call, athread and a value.

For the work under way now, no. The latest frontier record holds 29
application events, and 2locks counts, added in0.8. The rest is missing:
- EXECUTION_COMPLETED is ds, exit_code andtimeout_seconds are all null, and exit_status is killed.
- The speed numbers are otions retired, seconds,fps-equivalent, host seconds by exit kind, PAUSE count, wake census, thread
  census and per-vCPU numint 094's table wasrebuilt from logs in build/sprint-094/. The single number milestone P is
  judged by has no tag.
- The pivot's judge has no vocabulary. The laptop's DTrace recordings
  (store/laptop-traces/*.314 system calls and 18win32k calls a frame) are TSV, not signals, and no tag names a system call.
  The new boundary's refeechanically with what w2m will emit.

Hardening, in order

1. Vocabulary 0.9, aligned to the system as it is now, through typed
   proposals.
   - New categories: for the VM (vCPU, exit), the kernel (scheduler, wait,
     system call), the coaphics.
   - Candidate tags:
     - a per-run summary:s-equivalent, host seconds by exit kind, load average;
     - an ambient progresiant (the 30-secondprogress line already is one);
     - a system call servne unserved (the newboundary's SERVICE_UNIMPLEMENTED);
     - per-frame summarieincidents for droppedframes (TECHNIQUES.md, game-development class: frame budget as a
       stratum).
   - Emit TRANSLATION_INVALIDATED and GUEST_FAULT_RAISED.
   - Deprecate the 22 othROPOSED.
2. One schema for both sides of the system-call boundary. Convert the DTrace TSVs into guest-plane he laptop's as thereference and w2m's as the observed. The existing comparator then judgethe new kernel the way
3. Typed C emitters generated from the vocabulary. Extend gen_events.py sothe guest runtime and s and fields thevocabulary declares, checked at compile time.                          A record checker in ga and S1 to S13 over everyrecord, alongside check_replay.py. Fill EXECUTION_COMPLETED's          elapsed_seconds and ex are a payload anomaly inevery frontier record.                                                 Signal contracts list its names the existingtags its code path fires, with expected payload values. A speed sprint states its fps-equivalservation-contract valuesread from the record, not from stderr.
6. A mechanical Rubber Du
   - Narration: a script narrates each sprint's records (Substrate expos  narrate and narratio
   - Observations: two of the six categories come out of the checker. A   broken T4 is a missile is an order violation.
   - Ban: "no new tag" stops counting as a narration.                   Signal Reports resumedion ending them. Thesilent stop at Sprint 048 is logged as a bypass either way.          Data through Substrateict (from the previousreview). Producers read their input from the record, so a replay can re-derive what the com
                                                                           Items 1, 4 and 5 make the-call kernel visible tothe method that already keeps correctness honest. The rest closes the gaps where SDD's form survives

## 2. The review's counts, checked against the tree (2026-10-17, at 4d09f7e7)

| Claim | Found | How |
|---|---|---|
| "None new" in 14 of the last 15 cards' Emits | 14 of 15; Sprint 101's MEASUREMENT_RECORDED the one | first bullet under `### Emits` of the 15 newest cards |
| Rubber Duck Passes in the tail start "Sequence narration" in prose | 10 of the 10 tail entries | `## Sprint tail` of BLACKBOARD.md |
| Signal Reports stop at Sprint 048, 26 of them | 26, the last `signal-reports/sprint-048-report.md` | `ls signal-reports` |
| No logged bypass for the stop | none found in BLACKBOARD.md or WORKING_AGREEMENT.md | search for "signal report" |
| Invariants enforced: T5, T6, S14 in trace.py | T5, T6, S14 in `src/w2m/trace.py`; `scripts/check_replay.py` names S5 | search for each id |
| 32,241 records | 32,228 directories under store/runs (after Sprint 106's) | `find store/runs -maxdepth 1` |
| 62 tags, 13 product (11 never emitted), one perf | 62; 13 product, 11 of them never emitted; perf holds MEASUREMENT_RECORDED alone | `signals/0.8.json` |
| 24 of 62 never emitted, TRANSLATION_INVALIDATED among them | 27 by this scan: no `kind` or `tag` field with the name in any `.jsonl` under store/runs, store/traces, store/game-runs; TRANSLATION_INVALIDATED, GUEST_FAULT_RAISED, EXECUTION_FAILED and TRACE_REJECTED appear only inside schema text | the scan may miss an emit under another field; the difference from 24 is open |
| The latest frontier record: 29 application events; EXECUTION_COMPLETED's elapsed_seconds and exit_code null, exit_status killed | 23 MEASUREMENT_RECORDED + 6 others; `elapsed_seconds` None, `exit_code` None, `exit_status` killed | `frontier-643a6283510c-1792262915456560000` |
| The guest trace: 10,546 lines | `guest_line_count` 10546 | the same record |
| C writes JSON lines by hand | the probes write their guest-trace bodies with `head(...)` and WriteFile (`probes/hello`, `skeleton`, `last_error_values`, `event_auto_reset`) | search for GUEST_THREAD_STARTED |
| The laptop's DTrace recordings are TSV | `store/laptop-traces/frame-calls-hail-to-the-rainbow.tsv`, `process-start-hello.tsv.gz`, one session folder (Sprints 092, 093) | `ls` |

## 3. The adherence check of Sprint 107 (2026-10-17)

Against sdd-kit-3/AGENTS.md:

- **Hard rule 6, as the project overrides it** (WORKING_AGREEMENT, 2026-10-09: one concept, one observation
  contract, one verdict; files counted, not capped): commit 4d09f7e7 holds four concepts (the shared words, the per-vCPU
  current thread, the lock's owner value, the host lock library); the work after it, parked on branch
  `wip/sprint-107-host` (2d254cd4), spans eight files and six more (the RUNNING mark, the host loop, idling, the
  suspend protocol, the wake, a 50 ms cut of every run). Its first judge run hung: the threads probe on two vCPUs
  stopped at its 240 s timer.
- **Hard rule 4** (resolve and record): four choices had no Decision: the 50 ms cut (polling, against the design
  document's SGI in Sprint 108); the runtime lock as the host's only lock; the several-vCPU path skipping the
  oracle's hooks and the frontier's stop conditions; no second process on several vCPUs.
- **Signal contract:** "None new" while the sprint adds idle waits, preemptions and wakes, none on the record.
- **Records from committed trees:** three family records were written from dirty trees during the sprint
  (`family-threads-1792264046883965000`, `family-threads-1792264114628321000`,
  `family-wait_handoff-1792264115417183000`); they are moved to `build/sprint-107-dirty-records/` and the index is
  back at its committed state, as Sprint 099 did with its stopped sweep.
- **Standing rule:** two commands used a shell `&`.

Against Substrate: the several-vCPU judges (Sprints 105, 106, 107) are pytests over `run_in_vm`, outside any record;
a run whose result depends on the vCPU count does not name that count in a record.

Against A Philosophy of Software Design (Ousterhout):

- **Repetition:** `_vcpu_loop` repeated about 100 lines of `steps`' exit handling; each new hvc would be written
  twice.
- **Shallow module, information leakage:** the lock's protocol sat in the guest's words, `runtime_lock.c`,
  `_host_lock`, `_host_unlock`, `_host_took` and a two-way `_release_runtime_lock`, each with a one-vCPU special case.
- **Hidden state:** `VM.serving`, read implicitly at about 70 sites, correct only under the lock.
- **Special cases:** a six-condition `several` flag keeping two run loops.
- **Tactical growth:** vm_backend.py at 5,314 lines, the file plan step 3 splits.
