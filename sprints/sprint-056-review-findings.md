# Sprint 056 — the review's findings

```yaml
---
id: 056
status: closed
phase: 5
pass_kind: functional
---
```

## scope

An outside review of Sprint 054 (2026-10-11, on 3e0dea6d) found seven defects; each reproduced on 243b8ad9 with the review's harnesses, and a search for the class-name pattern found an eighth. BLACKBOARD Decision 2026-10-11 (user) puts them before the frontier goes on. KIT_DIARY #47 names the cause: the probes fed the game's inputs, and the code accepted wider ones without a row or a stop.

| # | Defect | Where |
|---|---|---|
| 1 | `fill()` copies folder and name into `u16 full[1024]` unchecked: a short pattern matching a long name overflows the stack (AddressSanitizer: stack-buffer-overflow at line 74) | `tools/guest_runtime/file_search.c` |
| 2 | FILE_LIST is asked twice; if the directory grew between, the host writes nothing and the guest parses the buffer anyway, without a byte bound | `file_search.c`, `src/w2m/file_bridge.py` |
| 3 | three `w2m_heap_take` results used unchecked (the runtime's other ten callers check) | `file_search.c` |
| 4 | `lower()` narrows UTF-16 to `char`: U+0173 reads as `s` and U+0100 as the end of the name, so `ųoftware` and `Software\u0100x` open Software | `registry.c` |
| 5 | listing `C:\w2m` omits the mounted `app` directory that resolving it finds | `file_bridge.py` |
| 6 | class names cut at 63 units with success reported; the class is then not found by its own name | `windows.c` |
| 7 | MonitorFromWindow answers the monitor for any window, off the screen too | `display.c` |
| 8 | window titles cut at 127 units without a stop | `windows.c` |

1. **A failing check first, for each:** native harnesses that compile the runtime's C under AddressSanitizer (`tests/test_runtime_native.py`), and a bridge test, each failing on 243b8ad9's code.
2. **Windows rows where Windows decides the answer:** registry (a missing key with non-ASCII letters, one equal to Software but for a letter above ASCII, `ſoftware` with U+017F), file_search (a long name under a deep folder, a long name matched by a short pattern), windows (class names of 64, 256 and 257 units, a window created at 2000, 2000 asked with each MonitorFrom flag, a long title).
3. **The fixes**, judged by the harnesses and the rows.
4. **The audit:** 054's and 055's runtime code searched for the three patterns (a fixed buffer filled from guest text without a bound, a narrowing of UTF-16, a host or heap answer used unchecked); a gate test refuses an unchecked `w2m_heap_take` and a function narrowing `u16` to `char`.
5. **The claims:** the display comment's 0x800700A0 (HRESULT_FROM_WIN32(ERROR_BAD_ARGUMENTS), not E_INVALIDARG), file_search's "times are GetFileAttributesExW's" (creation and write times are compared), its order (NTFS's as the runners showed; FindFirstFileEx "does no sorting", Microsoft Learn); each contract names its derive function.

## prerequisites

- 055 (closed: CI run 38180806466 on 3e0dea6d; main 243b8ad9)

## context_files

- the review (chat 2026-10-11) and its harnesses (`/tmp/w2m_review_checks.py`, `_bridge.py`, `_windows.py`); `tools/guest_runtime/file_search.c`, `registry.c`, `windows.c`, `display.c`, `heap.c`; `src/w2m/file_bridge.py`; `scripts/family_run.py`; Microsoft Learn: FindFirstFileExW, WNDCLASSEXW, MonitorFromWindow

## signal contract

### Emits

- Per family: the item run. The frontier is not run: no service on its path changes but these.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `tests/test_runtime_native.py`, `tests/test_runtime_patterns.py`
- modified: the files in the table; the registry, file_search and windows probes and contracts (new versions); `scripts/family_run.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **Each harness fails on 243b8ad9's code and passes after the fix**, under AddressSanitizer.
- **registry, file_search and windows equal Windows** with their new rows on Server 2025 and 2022; display, version and time stay accepted.
- **The pattern gate** fails on a planted unchecked `w2m_heap_take` and a planted narrowing.

## done criteria

All eight fixed and checked; the three families re-accepted with the new rows; the gate in place.

## observed

- **All eight fixed, each checked first.** `tests/test_runtime_native.py` holds a native harness per finding (the
  runtime's C under AddressSanitizer, its calls into w2m stubbed) and a bridge test for the mount; all eight failed on
  243b8ad9's code for the reason each names (the listing check first failed on a wrong expectation of its own: "." and
  ".." come first below a drive's root) and pass now.
- **Windows decided four answers** (runs 38182330078, 38182331638; 38182330015, 38182331619; 38182422811,
  38182424645): a key name with letters above ASCII is a missing key, `ſoftware` (U+017F) included, so ASCII-only folding
  is Windows's here; a 200-unit name is found by a 1-unit pattern and by itself; a window at (2000, 2000) is on no
  monitor for MONITOR_DEFAULTTONULL and on the primary for the other flags; a class name of 255 units registers and
  one of 256 or 257 answers 0 with ERROR_INVALID_PARAMETER: Microsoft Learn's "maximum length ... is 256" counts the NUL.
- **registry, file_search and windows 0.2.0 equal Windows** with the new rows; their plants (five, six and nine) are
  caught as named, the new ones among them (narrowed_again, name_cut_short, class_limit_256, monitor_ignores_window).
- **The audit** of the runtime for the three patterns found a ninth defect, GetCommandLineA's 1,024-byte buffer cutting
  a longer line silently (now 2,048, the process page's bound); the other `w2m_heap_take` callers and `(char)` casts
  are sound; the host now refuses a child's inherited list longer than its page. `tests/test_runtime_patterns.py`
  refuses an unchecked `w2m_heap_take` and a u16-to-char narrowing, and found all four of 243b8ad9's instances.
- **The claims:** the three comments corrected; `ExactRows` gains `derivation`, and the 24 contracts whose rows are
  derived name the function and what it keeps (rows unchanged); a derive function without a description is refused;
  file_search and display list heap.c and guest_stubs.py (derived lists are Sprint 057).
- **Not done here:** a review of Sprint 055 (Sprint 058); the pipe's buffer, which grows without bound where Windows's
  blocks a writer, is left to it.
