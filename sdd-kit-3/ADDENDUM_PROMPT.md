# ADDENDUM_PROMPT.md — the repeatable prompt for capturing a project's new techniques

*Copy the block below into a fresh LLM session that has read access to a project's repo and to this kit. It drives the LLM to research one project's hard-won technique learnings and land them as a dated, project-stamped entry in `ADDENDUMS.md`. Fill the two bracketed slots first. Everything else is fixed.*

*This is the standing instruction behind Addendum A. Re-run it per project, whenever a project has accumulated a `KIT_DIARY.md` worth of learnings that the numbered `TECHNIQUES.md` catalog does not yet carry.*

---

## The prompt

> **Task: capture this project's new techniques as a dated addendum in the SDD kit.**
>
> Project to research: `[PROJECT_DIR]`
> SDD kit location: `[KIT_DIR]` (contains `TECHNIQUES.md`, `ADDENDUMS.md`, `README.md`)
>
> **1. Research the project — deeply, from the ground.** Read its `process/KIT_DIARY.md` in full (that is where techniques are first observed), then read the actual harness / tooling code the diary refers to, so every technique you capture is grounded in real code, not paraphrase. Read as much as you need; do not skim, sample, or delegate the reads to a subagent that returns excerpts. If the project has no `KIT_DIARY.md`, say so and stop — there is nothing stabilized to capture yet.
>
> **2. Keep ONLY what is genuinely new.** Read `[KIT_DIR]/TECHNIQUES.md` in full. Discard any learning already covered by the numbered catalog. What survives is the delta: techniques this project surfaced that the catalog does not yet carry. If a learning *sharpens* an existing catalog entry rather than replacing it, keep it and name which entry it extends.
>
> **3. Be specific at the tooling level.** The unit of value is a concrete, transferable technique — the "use XCUITest over the IDB bridge" grain, not "test the UI well." Name the exact tool, call, coordinate, failure mode, and fix. Where a technique came from a real bug, state the bug and the root-cause fix (not the symptom patch). Ground each entry in a file path in the project.
>
> **4. Weigh honestly.** Everything from one project is one data point. Mark single-project findings as tentative; do not launder a single observation into kit consensus. Match the hedging in the source diary's hypothesis table.
>
> **5. Write it into `[KIT_DIR]/ADDENDUMS.md`.** Append a new addendum (next free letter). If `ADDENDUMS.md` does not exist, create it top-level in the kit root with the dated-capture framing (it is a holding pen between a project's `KIT_DIARY.md` and the numbered `TECHNIQUES.md`; each entry is a point-in-time capture from ONE named project on ONE dated stretch, not a timeless catalog). Every addendum MUST carry a stamped header: **Captured from** (project + one-line what-it-is), **Dates** (the diary date range + source path), **Weight** (one project, that stretch; not yet reproduced elsewhere; not yet in the catalog), **Harness/tooling**, **Extends** (which `TECHNIQUES.md` sections). Add a row to the "Addendums on file" table at the top of `ADDENDUMS.md`. Close the addendum with a cross-reference block pointing at the source diary and the catalog entries it extends.
>
> **6. Make it visible.** Add or update a one-line pointer to `ADDENDUMS.md` in `[KIT_DIR]/README.md`'s ship-list, naming the newest addendum's source project — so the newest lessons are seen on opening the kit.
>
> **Hard constraints.**
> - Additive only. Never delete or overwrite an existing addendum, table row, or catalog entry. A promotion into `TECHNIQUES.md` keeps the `ADDENDUMS.md` row and notes the promotion. The audit trail is the work.
> - Do NOT back-propagate into the numbered `TECHNIQUES.md` catalog. A technique folds into the catalog only once it has stabilized across a SECOND project; capturing from one project stops at `ADDENDUMS.md`.
> - Tone canon: textual markers only, no emojis; plain register, no LLM-speak; say what the technique is, what bug it kills, and how to apply it.
> - End your turn by reporting: which learnings you kept, which you discarded as already-catalogued, and any single-project finding you flagged as tentative.

---

*ADDENDUM_PROMPT.md — the standing, project-agnostic instruction for producing an `ADDENDUMS.md` entry. Fill `[PROJECT_DIR]` and `[KIT_DIR]`; the rest is fixed. Never delete; the audit trail is the work.*
