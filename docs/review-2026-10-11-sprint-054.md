# Review 2026-10-11 — Sprint 054

An outside review of Sprint 054 at 3e0dea6d, pasted into the session on 2026-10-11 (22:38 UTC) and acted on by Sprint 056.
The text below is the paste, unchanged except two things: the paste dropped the first letter ("[T]he"), and the
three harness links, which pointed at `/tmp`, now point at the copies in `docs/review-2026-10-11-sprint-054/`.

---

[T]he review found seven defects, including a reproduced stack-buffer overflow. All 12 held Sprint 054 probe binaries still pass their recorded comparisons.
The reviewed checkout is 3e0dea6d, beyond the supplied e2bd4369 snapshot. The review covered the six Sprint 054 families, their probes and comparison code, and the shared path, heap, resource, callback, and host-file mechanisms. Sprint 055’s process implementation received only a supporting read; this is not a full review of that sprint.
High — file enumeration can overwrite the runtime stack.
[file_search.c:67](/Users/peterlaffey/port/tools/guest_runtime/file_search.c:67)
fill() copies the search folder into u16 full[1024], appends a separator, then copies the returned filename without checking the remaining capacity. The length check in FindFirstFileExW covers the search expression. A short wildcard can match a filename much longer than that expression.
A native harness compiled the actual source with AddressSanitizer and called fill() with a 990-unit folder and a 100-unit filename. It failed at line 74:
text
   ERROR: AddressSanitizer: stack-buffer-overflow
   WRITE of size 2
   'full' ... <== Memory access ... overflows this variable
This reproduces an overwrite in the C implementation. It does not establish the exact failure mode inside the VM. The existing probe uses short paths and three directory entries, so every recorded row can pass while this write remains unchecked.
High — directory growth leaves the guest parsing unwritten heap memory.
[file_search.c:119](/Users/peterlaffey/port/tools/guest_runtime/file_search.c:119), [file_bridge.py:196](/Users/peterlaffey/port/src/w2m/file_bridge.py:196)
The guest first asks FILE_LIST for the required size, allocates a buffer, then asks for the listing. The host enumerates the directory separately for each request. When the second listing exceeds the supplied capacity, the host returns its new size without writing the buffer.
The guest discards that return value. It also ignores the second request’s kind, then walks the buffer using NUL terminators without a byte bound.
A direct bridge check reproduced the failed transfer:
text
   first capacity: 6 bytes
   second required size: 38 bytes
   second kind: 2
   output buffer: a5a5a5a5a5a5
The test added longer_filename between the requests. The output retained its sentinel bytes.
w2m_heap_take() does not clear reused blocks. Depending on prior contents, the guest can read stale names, report no match, or scan past the allocation. Directory removal between the requests follows the same unchecked-status path. The current probe changes no directory contents during enumeration.
Medium — all three file-search allocations assume success.
[file_search.c:124](/Users/peterlaffey/port/tools/guest_runtime/file_search.c:124), [heap.c:103](/Users/peterlaffey/port/tools/guest_runtime/heap.c:103)
w2m_heap_take() explicitly returns zero when its arena cannot satisfy an allocation. File search checks none of its three results: the directory listing, copied matching names, and pointer array.
Each failure reaches an invalid access. The listing becomes a host output address and a guest string pointer; the names buffer becomes a copy destination; the pointer array receives order[k] writes.
This is a source-confirmed failure path, not a reproduced arena-exhaustion run. The distinction matters because the runtime already has a defined capacity policy: HeapAlloc stops through w2m_unimplemented when its fixed arena cannot serve a request. File search instead proceeds with address zero.
Medium — UTF-16 registry names alias unrelated ASCII keys.
[registry.c:35](/Users/peterlaffey/port/tools/guest_runtime/registry.c:35)
lower() accepts a 16-bit code unit and returns char. Every non-ASCII code unit loses its upper byte. The subsequent lookup treats those bytes as a NUL-terminated ASCII path.
Two calls against the actual registry implementation reproduced false successes:
Input below HKEY_CURRENT_USER    Return    Handle
U+0173 followed by oftware    ERROR_SUCCESS    0x3000
Software followed by U+0100 and x    ERROR_SUCCESS    0x3004
U+0173 becomes byte 0x73, ASCII s. U+0100 becomes a NUL byte, making the second name appear to end after Software.
Both calls therefore open the existing software record under a different supplied name. The 16 registry rows exercise ASCII spellings, missing ASCII names, and handle lifetime. They cannot detect this conversion defect. These checks reproduced w2m’s aliasing; no new Windows reference run was collected for those names.
Medium — enumeration omits the mounted application directory.
[file_bridge.py:151](/Users/peterlaffey/port/src/w2m/file_bridge.py:151), [file_bridge.py:199](/Users/peterlaffey/port/src/w2m/file_bridge.py:199)
_resolve() gives C:\w2m\app special treatment and redirects it to the mounted application directory. The bridge constructor creates the physical w2m parent but no physical app entry there.
FILE_LIST uses only os.listdir(path). It does not include that virtual mount.
On a fresh bridge, the check produced both of these results:
text
   C:\w2m listing: no names
   C:\w2m\app resolve: kind 2, mounted application directory
Thus FindFirstFileExW(L"C:\\w2m\\app", ...) searches a parent listing that omits the requested directory, while direct lookup resolves that directory successfully. A recursive traversal from C:\ also misses the application subtree.
The probe’s folder_itself case tests a physical directory beneath the mount. It never enumerates the mount’s parent.
Medium — class registration truncates names but reports success.
[windows.c:122](/Users/peterlaffey/port/tools/guest_runtime/windows.c:122)
register_class() stores at most 63 UTF-16 units, appends NUL, and returns a new atom. find_class() later compares the stored name against the complete supplied name.
A native check using a 64-character ASCII name produced:
text
   first registration: 0xc000
   lookup by original name: -1
   second registration: 0xc001
The first successful registration cannot be found by its own name. Registering that name again consumes another slot instead of reporting the existing class. Repeating the call can exhaust the 16-slot class table.
A 64-character name is below the documented Windows maximum of 256. This is silent truncation of a valid name, not a Windows length restriction. [Microsoft’s WNDCLASSEXW reference](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-wndclassexw)
Medium — `MonitorFromWindow` ignores the window’s rectangle.
[display.c:166](/Users/peterlaffey/port/tools/guest_runtime/display.c:166)
Every recognized window returns monitor 0x10001, regardless of its position or dwFlags. MonitorFromRect does perform an intersection test.
A native check placed a window record at (2000, 2000)–(2320, 2240), outside the fixed 1024×768 screen. With MONITOR_DEFAULTTONULL, the two functions disagreed:
text
   MonitorFromWindow: 0x10001
   MonitorFromRect:   0
Windows specifies NULL for a nonintersecting window with that flag. The single-monitor profile does not remove this case. [Microsoft’s MonitorFromWindow reference](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-monitorfromwindow)
The display probe tests three NULL-window calls but no valid off-screen window. The separate window probe therefore leaves this cross-family behavior unchecked.
The verification results are narrower than the sprint card’s phrase “equals Windows.” The fresh VM runs establish equality on the recorded, derived rows:
Family    Rows per binary    Binaries rerun    Unequal rows
Registry    16    2    0
File search    26    2    0
Version    98    2    0
Display    147    2    0
Windows    36    2    0
Time    12    2    0
Total    335    12    0 across 670 comparisons
All 12 executions stopped at ExitProcess. The selected pytest files—test_family_runs.py, test_guest_runtime.py, and test_create_file.py—also passed: 20 tests in 1.55 seconds. The initial command failed before collection because the inherited PYTHONHOME referenced a missing Python standard library; clearing that variable for the test process resolved it. The full suite and mutation plants were not rerun.
Several evidence limits follow directly from the probes and comparison code:
The file-search probe checks creation and write times against GetFileAttributesExW, but never compares access times. Dot entries receive only a check that the write time’s high word is nonzero. The broader comment that “the times are GetFileAttributesExW’s” exceeds those observations.
The directory-order claim describes the observed runner profile. Microsoft explicitly says FindFirstFileExW does no sorting. The implementation’s chosen sort can reproduce these NTFS observations, but “Windows order” is too broad a name for it. [Microsoft’s FindFirstFileExW reference](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-findfirstfileexw)
display_rows() replaces work-area bottoms with a Boolean relation, removes metric 93’s value, and normalizes the adapter GUID. windows_rows() normalizes position and default height. Passing those comparisons establishes the retained relations, not byte-for-byte equality with each runner.
The candidate hash covers each family’s source and manually listed dependencies. File search depends on heap.c, which its dependency list omits; display calls through guest_stubs.py, which its list omits. Their individual acceptance fingerprints therefore do not cover all code exercised by those families.
The display comment calls 0x800700A0 an E_INVALIDARG form. That name conflicts with the repository’s own E_INVALIDARG constant, 0x80070057, in version.c. The numeric observation and its symbolic description are separate claims.
The existing guest/host split keeps Windows matching and result construction in guest C, with filesystem access in Python. The reproduced failures occur at the contracts between those parts: a listing transfer without validated completion, a virtual namespace absent from enumeration, and a composed path without a capacity check. The window findings likewise cross operations that pass separately: registration versus lookup, and window identity versus geometry.
The review harnesses remain in [registry and path checks](review-2026-10-11-sprint-054/w2m_review_checks.py), [bridge checks](review-2026-10-11-sprint-054/w2m_review_bridge.py), and [window checks](review-2026-10-11-sprint-054/w2m_review_windows.py). No repository files changed; the working tree remains clean.
