# CLAUDE.md — sdd-kit-3 (Claude Code shim)

**Read `AGENTS.md` first.** It is the load-bearing working agreement. This file adds Claude-Code-specific notes only.

---

## Claude-Code-specific notes

**Auto-load behavior.** Claude Code auto-loads CLAUDE.md at session start. This shim ensures Claude Code reads AGENTS.md. The methodology is in AGENTS.md and the foundations; tool-specific notes are here.

**File tools.** Use `Read` for files you'll cite; use `Edit` for files you'll modify in place (preserves accreted detail per hard rule 7); use `Write` only for new files or full rewrites the sprint card explicitly authorizes. The Edit tool's SEARCH/REPLACE semantics match the SDD discipline of preserving prior-sprint detail.

**Shell access.** You have Bash access. Run the project's build and test commands yourself at every sprint close, as the dual contract requires. Put the exit code and the last 200 lines of output in the Signal Report. Diagnose a failed build before re-running it; never silently retry.

**Relative paths.** Your tool calls resolve from the project root. When AGENTS.md says "read `sdd-kit/foundations/01-...`", that's the path from project root.

**TodoWrite for sprint progress.** Optional. If a sprint has more than 3 discrete steps, using TodoWrite to track them is fine; it doesn't replace the Signal Report, but it helps the human see progress mid-sprint.

**Memory.** If you have access to memory tools (e.g., the SDK's memory system), record cross-session learnings about *this project* there. Do NOT record kit-level findings in project memory; those belong in `KIT_DIARY.md` in the project (and back-propagated to sdd-kit upstream by the maintainer).

**Plan mode.** sdd-kit-3 has no plan-mode wait. Compose the card, self-review it, state its scope in one line, and execute. Use Claude Code's plan mode only when the human turns it on.

**Questions.** Settle questions on the three grounds in AGENTS.md "Resolve-and-record" and record them. Use the AskUserQuestion tool only for the closed list of user-only questions, and keep working on independent work while the question is open.

**No emojis in committed files.** Hard rule from AGENTS.md but stating here too because Claude Code's default tendency is to use them. The kit's tone canon: textual markers only.

**No deletions.** Hard rule from AGENTS.md. The audit trail is the work. New thinking goes into new files / new folders / round-N versions; do not `rm` or `rmdir` anything in this project. If a restructure is needed, build it additively.

---

## What this file does NOT add

Anything substantive about the methodology. That's all in AGENTS.md and the foundations. If you find yourself wanting to add a discipline rule here, add it to AGENTS.md instead so other tools' shims (cursor.md, copilot-instructions.md, cline.md, etc.) inherit it.

---

*CLAUDE.md is a shim. The working agreement is AGENTS.md.*
