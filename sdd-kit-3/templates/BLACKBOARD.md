# BLACKBOARD — {Project Name}

*The project's working record. Seven sections. Single-writer per section (discipline, not code-enforced). The Architect (human) does not need to read this: everything the Agent writes here that matters to them is also said in chat in the same turn. The BLACKBOARD is bookkeeping for the next session and for audit. Copy this template to the project root as `BLACKBOARD.md`; do not edit this template.*

---

## Surfaced for review

*Agent + Architect. The record of what the Agent told the user: comprehension affirmations, user-only questions and their answers, partial closes, Rubber Duck observations marked `recorded`. Not a request queue: each item was also said in chat.*

*(empty on project start)*

---

## Decisions

*Append-only. Two labeled kinds of entry. **user** entries record what the Architect said (quote or cite where). **agent-resolved** entries record a question the Agent settled on the three grounds (AGENTS.md "Resolve-and-record"). Nobody edits or removes an entry; a later entry supersedes an earlier one on the same question, and a user entry supersedes any agent-resolved one.*

- **{YYYY-MM-DD} user** — Project scope: {one-paragraph statement of what this project is, who it's for, what success looks like, what's out of scope, with a pointer to where the Architect stated it. The Agent reads this at session start as ground truth.}
- **{YYYY-MM-DD} agent-resolved** — {question, one line}. Decision: {answer}. Grounds: project — {path or Decision}; SDD — {file, rule or technique}; practice — {source}. Revisit if: {condition}.

---

## Built

*Agent appends one entry per sprint close. Append-only. One short paragraph per sprint: sprint id, files authored, dual-contract outcome.*

*(empty until first sprint closes)*

---

## Deferred

*Anyone may append. Items deferred from a sprint, with the re-visit condition.*

*(empty on project start)*

---

## Open questions

*Anyone may append. Questions not yet answered.*

*(empty on project start)*

---

## Drift watchlist

*Agent maintains. Patterns to monitor across sprints. When the same observation recurs in three consecutive sprints, resolve it: fix the cause or make it a project invariant in `WORKING_AGREEMENT.md`, record the decision, and say so in chat.*

*(empty on project start)*

---

## Sprint tail

*Agent maintains. Rolling log of recent sprint closes. Last 10 sprint summaries; older entries roll into `## Built` as compressed paragraphs.*

*(empty until first sprint closes)*

---

## Single-writer-per-section discipline

Per `sdd-kit/AGENTS.md` hard rule and `sdd-kit/foundations/03-sdd-team-model.md`'s communication-protocol-inversion principle.

| Section | Who writes | What they write |
|---|---|---|
| `## Surfaced for review` | Agent + Architect | User-only questions and answers (Agent), comprehension affirmations (Agent at session start), partial closes (Agent), specific feedback (Architect) |
| `## Decisions` | Agent + Architect (append-only, labeled) | Project scope at start (user); agent-resolved decisions with grounds; user decisions, which supersede |
| `## Built` | Agent (append-only) | One paragraph per sprint close |
| `## Deferred` | Anyone | Items deferred with re-visit condition |
| `## Open questions` | Anyone | Questions not yet answered |
| `## Drift watchlist` | Agent maintains | Observations to track across sprints |
| `## Sprint tail` | Agent maintains | Last 10 sprint closes; older roll into `## Built` |

When in doubt: the Agent settles the question on the three grounds, appends an agent-resolved entry to `## Decisions`, and says so in chat. It asks the Architect only the user-only questions, in chat.

---

*BLACKBOARD.md — template. Copy to project root as `BLACKBOARD.md`; fill `## Decisions` with project scope (a user entry); leave other sections empty for the first sprint to populate. Single-writer per section is discipline, not code-enforced.*
