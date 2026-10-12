# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""Native harnesses for guest runtime C (Sprint 056; the 2026-10-11 review of Sprint 054).

Each test compiles one runtime source with a small main, natively and under AddressSanitizer, with the runtime's
calls into the rest of w2m stubbed, and runs it: the review reproduced its findings this way, and these keep them
reproduced. A stop by name (w2m_unimplemented) exits 88; AddressSanitizer exits nonzero with its report.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from w2m.guest_runtime import api_header  # noqa: E402

RUNTIME = ROOT / "tools" / "guest_runtime"
CLANG = shutil.which("clang")
STOPPED = 88

PREFIX = """#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static unsigned char test_teb[8192];
#define W2M_TEB_VA ((unsigned long long)test_teb)
static unsigned long long test_peb[8];
#define W2M_PEB_VA ((unsigned long long)test_peb)
"""
COMMON = """
u64 w2m_logged(u64 a, u64 b, u64 c) { (void)a; (void)b; return c; }
void w2m_unimplemented(u64 a, u64 b) { fprintf(stderr, "unimplemented %llu %llu\\n", a, b); exit(88); }
"""


def run_native(sources: list[str], body: str, prefix: str = "") -> subprocess.CompletedProcess[str]:
    """Build runtime files `sources` (the first with `body` appended, each after `prefix`) under AddressSanitizer
    and run the result."""
    if CLANG is None:
        pytest.skip("no clang on this host")
    with tempfile.TemporaryDirectory(prefix="w2m-native-") as d:
        work = Path(d)
        (work / "api_ids.h").write_text(api_header())
        units = []
        for i, name in enumerate(sources):
            text = PREFIX + prefix + f'#include "{RUNTIME / name}"\n' + (COMMON + body if i == 0 else "")
            (work / f"unit{i}.c").write_text(text)
            units.append(str(work / f"unit{i}.c"))
        exe = work / "check"
        built = subprocess.run([CLANG, "-g", "-O1", "-fsanitize=address", "-I", str(work), *units, "-o", str(exe)],
                               capture_output=True, text=True)
        assert built.returncode == 0, built.stderr
        # leaks are not judged: the guest heap is w2m's own, and LeakSanitizer (Linux's AddressSanitizer runs it, macOS's
        # does not) reports the harnesses' test buffers and exits before stdout is flushed (CI run 38182897398)
        env = os.environ | {"ASAN_OPTIONS": "detect_leaks=0"}
        return subprocess.run([str(exe)], capture_output=True, text=True, timeout=60, env=env)


FIND_STUBS = """
u64 w2m_full_path(const u16 *in, u16 *out, long *part) { (void)part; u64 n = 0; while ((out[n] = in[n])) n++; return n; }
u64 w2m_file_stat_full(const u16 *full, u64 out, u64 *attributes) { (void)full; (void)out; (void)attributes; return 2; }
static int heap_fails;
u64 w2m_heap_take(u64 n) { return heap_fails ? 0 : (u64)calloc(1, n); }
void w2m_heap_release(u64 a, u64 p) { (void)a; free((void *)p); }
/* FILE_LIST: the folder's names, from `names`; `grow` names that appear after the first request */
static const char *names[8] = {"a.txt", 0};
static const char *grow;
static int requests;
u64 w2m_host(u64 op, u64 a, u64 b, u64 c, u64 d, u64 *second) {
    (void)op; (void)a; (void)b;
    if (requests++ == 1 && grow) { names[1] = grow; names[2] = 0; }
    u64 need = 2;
    for (int i = 0; names[i]; i++) need += 2 * (strlen(names[i]) + 1);
    *second = 2;
    if (need <= d) {
        u16 *o = (u16 *)c;
        for (int i = 0; names[i]; i++) { for (const char *p = names[i]; *p; p++) *o++ = (u16)*p; *o++ = 0; }
        *o = 0;
    }
    return need;
}
static u16 *wide(const char *s) {
    u16 *w = calloc(strlen(s) + 1, 2);
    for (size_t i = 0; s[i]; i++) w[i] = (u16)s[i];
    return w;
}
"""


def test_find_fill_bounds_a_long_name() -> None:
    """Review finding 1: a 990-unit folder and a 100-unit name overflowed `full`. A name NTFS allows (up to 255
    units) beside a folder w2m accepts (under 1024) is composed in bounds."""
    r = run_native(["file_search.c"], FIND_STUBS + """
int main(void) {
    struct search f = {0};
    f.folder[0] = 'C'; f.folder[1] = ':'; f.folder[2] = '\\\\';
    for (int i = 3; i < 1023; i++) f.folder[i] = 'a';
    u16 name[256]; for (int i = 0; i < 255; i++) name[i] = 'b'; name[255] = 0;
    u8 data[592];
    fill(&f, name, (u64)data);
    puts("ok");
    return 0;
}
""")
    assert r.returncode == 0 and "ok" in r.stdout, r.stderr[-2000:]


def test_find_retries_a_listing_that_grew() -> None:
    """Review finding 2: a directory that grows between the size request and the listing left the buffer unwritten,
    and the guest parsed it. The listing is asked again until it fits, and its names are the second answer's."""
    r = run_native(["file_search.c"], FIND_STUBS + """
int main(void) {
    grow = "a_much_longer_file_name_than_before.txt";
    u8 data[592];
    u64 rsp[16] = {0};
    u64 h = k32_FindFirstFileExW((u64)wide("C:\\\\d\\\\*"), 1, (u64)data, 0, (u64)rsp);
    do {
        for (u16 *c = (u16 *)(data + 44); *c; c++) putchar((char)*c);
        putchar('\\n');
    } while (k32_FindNextFileW(h, (u64)data));
    return 0;
}
""")
    assert r.returncode == 0, r.stderr[-2000:]
    assert r.stdout.split() == [".", "..", "a.txt", "a_much_longer_file_name_than_before.txt"]


def test_find_stops_when_the_heap_is_full() -> None:
    """Review finding 3: w2m_heap_take's 0 was used as an address. A full heap stops the run by name, as HeapAlloc's
    callers do."""
    r = run_native(["file_search.c"], FIND_STUBS + """
int main(void) {
    heap_fails = 1;
    u8 data[592];
    u64 rsp[16] = {0};
    k32_FindFirstFileExW((u64)wide("C:\\\\d\\\\*"), 1, (u64)data, 0, (u64)rsp);
    puts("returned");
    return 0;
}
""")
    assert r.returncode == STOPPED, (r.returncode, r.stdout, r.stderr[-2000:])


def test_registry_keeps_names_utf16() -> None:
    """Review finding 4: U+0173 narrowed to 's' and U+0100 to the end of the name, so both names opened Software.
    Neither names a key w2m holds."""
    r = run_native(["registry.c"], """
int main(void) {
    u16 a[] = {0x173, 'o', 'f', 't', 'w', 'a', 'r', 'e', 0};
    u16 b[] = {'S', 'o', 'f', 't', 'w', 'a', 'r', 'e', 0x100, 'x', 0};
    u64 result = 0, stack[8] = {0};
    stack[5] = (u64)&result;
    printf("%llu %llu\\n", advapi32_RegOpenKeyExW(HKEY_CURRENT_USER, (u64)a, 0, 0x20019, (u64)stack),
           advapi32_RegOpenKeyExW(HKEY_CURRENT_USER, (u64)b, 0, 0x20019, (u64)stack));
    return 0;
}
""")
    assert r.returncode == 0, r.stderr[-2000:]
    assert r.stdout.split() == ["2", "2"]


WINDOW_STUBS = """
u64 w2m_resource(u64 a, u32 b, u32 c, u32 *d, u32 *e) { (void)a; (void)b; (void)c; (void)d; (void)e; return 0; }
void *w2m_guest_frame(u64 a, u64 b, void (*c)(struct w2m_guest_call *, u64)) { (void)a; (void)c; return calloc(1, b); }
void w2m_call_guest(struct w2m_guest_call *c, u64 f, u64 a, u64 b, u64 d, u64 e) {
    (void)c; (void)f; (void)a; (void)b; (void)d; (void)e;
}
void w2m_return_to(u64 a, u64 b) { (void)a; (void)b; }
u32 w2m_file_read_all(const unsigned short *a, u8 *b, u64 c, u64 *d) { (void)a; (void)b; (void)c; (void)d; return 2; }
u64 user32_MonitorFromWindow(u64, u64);
u64 user32_MonitorFromRect(u64, u64);
"""


def test_class_name_of_64_units_is_found() -> None:
    """Review finding 6: a 64-unit class name was cut to 63, then not found by its own name, and a second
    registration took a second atom. It registers once and is found."""
    r = run_native(["windows.c", "display.c"], WINDOW_STUBS + """
int main(void) {
    u16 name[65]; for (int i = 0; i < 64; i++) name[i] = 'a'; name[64] = 0;
    u64 a = register_class(0, 0, 1, 0, 0, 1, 0, 0, 0, (u64)name);
    int found = find_class((u64)name);
    u64 b = register_class(0, 0, 1, 0, 0, 1, 0, 0, 0, (u64)name);
    printf("%llx %d %llx\\n", a, found, b);
    return 0;
}
""")
    assert r.returncode == 0, r.stderr[-2000:]
    first, found, second = r.stdout.split()
    assert first == "c000" and found == "0" and second == "0"


def test_monitor_from_an_off_screen_window() -> None:
    """Review finding 7: MonitorFromWindow answered the monitor for a window that does not touch it. With
    MONITOR_DEFAULTTONULL it answers NULL, as MonitorFromRect does; with MONITOR_DEFAULTTONEAREST the monitor."""
    r = run_native(["windows.c", "display.c"], WINDOW_STUBS + """
int main(void) {
    windows[0].used = 1;
    windows[0].rect[0] = 2000; windows[0].rect[1] = 2000; windows[0].rect[2] = 2320; windows[0].rect[3] = 2240;
    printf("%llx %llx\\n", user32_MonitorFromWindow(WINDOW_BASE, 0), user32_MonitorFromWindow(WINDOW_BASE, 2));
    return 0;
}
""")
    assert r.returncode == 0, r.stderr[-2000:]
    assert r.stdout.split() == ["0", "10001"]


def test_window_title_is_kept_whole() -> None:
    """Finding 8 (found by finding 6's pattern): WM_NCCREATE's DefWindowProcW kept 127 units of the title. A title
    of 300 units is kept whole."""
    r = run_native(["windows.c", "display.c"], WINDOW_STUBS + """
int main(void) {
    windows[0].used = 1;
    u16 title[301]; for (int i = 0; i < 300; i++) title[i] = 't'; title[300] = 0;
    u8 cs[80] = {0};
    *(u64 *)(cs + 56) = (u64)title;
    user32_DefWindowProcW(WINDOW_BASE, 0x81, 0, (u64)cs, 0);    /* answers through w2m_return_to (Sprint 063) */
    int n = 0; while (windows[0].title[n]) n++;
    printf("%d\\n", n);
    return 0;
}
""")
    assert r.returncode == 0, r.stderr[-2000:]
    assert r.stdout.split() == ["300"]


def test_listing_the_mount_parent_names_the_mount(tmp_path: Path) -> None:
    """Review finding 5: C:\\w2m\\app resolves to the mount, while a listing of C:\\w2m left it out (the drive holds no
    app folder of its own). The listing names it, as the folder it resolves to."""
    from w2m.file_bridge import FILE_LIST, FileBridge

    class Memory:
        def __init__(self) -> None:
            self.buf = bytearray(4096)

        def read(self, va: int, n: int) -> bytes:
            return bytes(self.buf[va:va + n])

        def write(self, va: int, data: bytes) -> None:
            self.buf[va:va + len(data)] = data

    files = FileBridge(tmp_path)
    try:
        m = Memory()
        name = "C:\\w2m".encode("utf-16-le")
        m.write(0, name)
        size, kind = files.request(m, FILE_LIST, [0, len(name) // 2, 512, 2048])
        assert kind == 2
        assert m.read(512, size).decode("utf-16-le").split("\0")[:-2] == ["app"]
    finally:
        files.close_all()


# ---- the review of Sprint 055 (Sprint 058) ---------------------------------------------------------------------

INHERIT = """static unsigned long long inherit_page[512];
#define W2M_INHERIT_VA ((unsigned long long)inherit_page)
#define W2M_INHERIT_DIRECTORY_VA ((unsigned long long)inherit_page + 0x800)
"""
PROCESS_STUBS = """
static u64 started = ~0ull, room = 4096;
u64 w2m_host(u64 op, u64 a, u64 b, u64 c, u64 d, u64 *second) {
    (void)op; (void)d;
    if (a == W2M_KERNEL_PROCESS_CREATE) { started = c >> 32; if (second) *second = 8; return 4100; }
    if (a == W2M_KERNEL_PIPE_WRITE) { u64 n = d < room ? d : room; room -= n; return n; }
    (void)b; return 0;
}
static int blocked;
void w2m_block(u64 deadline) { (void)deadline; blocked++; }
u64 w2m_counter_now(void) { return 0; }
u32 w2m_error_mode(void) { return 0; }
int w2m_is_file_handle(u64 h) { (void)h; return 0; }
int w2m_is_std_handle(u64 h) { (void)h; return 0; }
void w2m_signal(void) {}
int w2m_thread_ended(u32 tid) { (void)tid; return 0; }
static const u16 here[] = {'C', ':', '\\\\', 'w', '2', 'm', 0};
const u16 *w2m_working_directory(void) { return here; }
"""


def test_more_than_64_inheritable_handles_stop_the_run() -> None:
    """Defect 4 of the review of Sprint 055: w2m_object_inheritable filled 64 entries and left the rest out, and the
    child started without them. A 65th inheritable handle stops the run."""
    r = run_native(["processes.c", "sync_objects.c"], PROCESS_STUBS + """
int main(void) {
    for (int i = 0; i < 65; i++) w2m_object_new(0, W2M_OBJECT_SHARED_EVENT, i, 1);
    u16 line[] = {'c', 0};
    u8 si[104] = {0}, info[24] = {0};
    *(u32 *)si = 104;
    u64 stack[16] = {0};
    stack[5] = 1; stack[9] = (u64)si; stack[10] = (u64)info;
    k32_CreateProcessW(0, (u64)line, 0, 0, (u64)stack);
    printf("%llu\\n", started);
    return 0;
}
""", INHERIT)
    assert r.returncode == STOPPED, (r.returncode, r.stdout, r.stderr[-2000:])


def test_a_full_pipe_makes_its_writer_wait() -> None:
    """Defect 6: WriteFile reported every byte written when the pipe took 4096 of 10000. The writer blocks and is
    retried; the call returns once the pipe has taken every byte (row pipe.large_write)."""
    r = run_native(["processes.c", "sync_objects.c"], PROCESS_STUBS + """
int main(void) {
    u64 h = w2m_object_new(0, W2M_OBJECT_PIPE_WRITE, 1, 0);
    static u8 data[10000];
    u32 written = 0;
    int first = w2m_pipe_write(h, (u64)data, sizeof data, (u64)&written);
    room = 6000;                                              /* the reader took 6000 */
    int second = w2m_pipe_write(h, (u64)data, sizeof data, (u64)&written);
    printf("%d %d %d %u\\n", first, blocked, second, written);
    return 0;
}
""", INHERIT)
    assert r.returncode == 0, r.stderr[-2000:]
    assert r.stdout.split() == ["-1", "1", "1", "10000"]


PARAMETERS = """static unsigned long long current_directory[0x410 / 8];
static unsigned char test_parameters[0x448];
#define W2M_CURRENT_DIRECTORY_VA ((unsigned long long)current_directory)
"""


def test_the_working_directory_is_the_process_parameters() -> None:
    """Defect 3 of the review of Sprint 055, moved by Sprint 059: a child's GetCurrentDirectoryW answered the
    program's directory. The working directory is ProcessParameters.CurrentDirectory.DosPath, which the host writes
    (a child's: its parent's) with a final backslash; SetCurrentDirectoryW writes the other of its two buffers (rows
    *.current_directory, after.current_directory)."""
    r = run_native(["paths.c"], """
u64 w2m_file_stat_full(const u16 *full, u64 out, u64 *attributes) { (void)full; (void)out; (void)attributes; return 2; }
u64 w2m_host(u64 op, u64 a, u64 b, u64 c, u64 d, u64 *e) { (void)op; (void)a; (void)b; (void)c; (void)d; (void)e; return 0; }
static void show(void) {
    u16 out[64];
    u64 n = k32_GetCurrentDirectoryW(64, (u64)out);
    for (u64 i = 0; i < n; i++) putchar((char)out[i]);
    putchar(' ');
}
int main(void) {
    test_peb[4] = (u64)test_parameters;
    const char *d = "C:\\\\w2m\\\\app\\\\w2m_probe_dir\\\\";
    u16 *to = (u16 *)current_directory;
    u64 n = 0;
    for (; d[n]; n++) to[n] = (u16)d[n];
    *(u16 *)(test_parameters + 0x38) = (u16)(2 * n);
    *(u16 *)(test_parameters + 0x3A) = 0x208;
    *(u16 **)(test_parameters + 0x40) = to;
    show();
    u16 sub[] = {'s', 'u', 'b', 0};
    k32_SetCurrentDirectoryW((u64)sub);
    show();
    const u16 *now = *(const u16 **)(test_parameters + 0x40);
    u64 len = *(const u16 *)(test_parameters + 0x38) / 2;
    printf("%d %c\\n", now == to + 0x104, (char)now[len - 1]);
    return 0;
}
""", PARAMETERS)
    assert r.returncode == 0, r.stderr[-2000:]
    assert r.stdout.split() == ["C:\\w2m\\app\\w2m_probe_dir", "C:\\w2m\\app\\w2m_probe_dir\\sub", "1", "\\"]


def test_standard_handles_without_inheritance_stop_the_run() -> None:
    """The gap Sprint 058 left: STARTF_USESTDHANDLES with bInheritHandles FALSE started the child with handles it
    cannot hold (Microsoft Learn, STARTUPINFOW: they "must be inheritable and the function's bInheritHandles parameter
    must be set to TRUE"). No row has that form: it stops the run (Sprint 059)."""
    r = run_native(["processes.c", "sync_objects.c"], PROCESS_STUBS + """
int main(void) {
    u16 line[] = {'c', 0};
    u8 si[104] = {0}, info[24] = {0};
    *(u32 *)si = 104;
    *(u32 *)(si + 60) = 0x100;                                    /* STARTF_USESTDHANDLES */
    u64 stack[16] = {0};
    stack[9] = (u64)si; stack[10] = (u64)info;                    /* bInheritHandles (stack[5]) FALSE */
    k32_CreateProcessW(0, (u64)line, 0, 0, (u64)stack);
    printf("%llu\\n", started);
    return 0;
}
""", INHERIT)
    assert r.returncode == STOPPED, (r.returncode, r.stdout, r.stderr[-2000:])
