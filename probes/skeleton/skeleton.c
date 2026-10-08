/* skeleton.c — walking-skeleton probe (Sprint 029). No C runtime: entry point `start`, six
 * kernel32 imports. The probe writes its own guest-trace body (GUEST_THREAD_STARTED, two
 * GUEST_CALL_RETURNED, GUEST_THREAD_EXITED) to standard output with WriteFile; the harness wraps
 * the body with GUEST_TRACE_OPENED/CLOSED. The same .exe runs on the Windows runner and in
 * w2m's runtime on the Mac, so both sides report through the same code.
 *
 * Contract: windows.thread.last_error.set_get — GetLastError returns the value SetLastError
 * stored on the same thread (Microsoft Learn, GetLastError and SetLastError).
 * args_sha256 per vocabulary 0.4 note: sha256 of "SetLastError(1234)" and "GetLastError()".
 */
#include <windows.h>

static char out[1024];
static unsigned n;

static void put(const char *s) {
    while (*s) out[n++] = *s++;
}

static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do {
        t[k++] = (char)('0' + v % 10);
        v /= 10;
    } while (v);
    while (k) out[n++] = t[--k];
}

static void head(const char *tag, DWORD tid, unsigned seq) {
    put("{\"tag\":\"");
    put(tag);
    put("\",\"thread_id\":");
    putu(tid);
    put(",\"thread_seq\":");
    putu(seq);
    put(",\"hb_refs\":[],");
}

void start(void) {
    DWORD tid = GetCurrentThreadId();
    head("GUEST_THREAD_STARTED", tid, 0);
    put("\"parent_thread_id\":0,\"start_address\":");
    putu((unsigned long long)(ULONG_PTR)start);
    put(",\"suspended\":false}\n");

    SetLastError(1234);
    head("GUEST_CALL_RETURNED", tid, 1);
    put("\"api\":\"kernel32!SetLastError\",\"args_sha256\":"
        "\"sha256:d3192cf02f36f9617e488563dc2dc21fbad5589955e06319a5910b8564ffed52\","
        "\"return_value\":0,\"last_error\":0}\n");

    DWORD e = GetLastError();
    head("GUEST_CALL_RETURNED", tid, 2);
    put("\"api\":\"kernel32!GetLastError\",\"args_sha256\":"
        "\"sha256:d767c2ccfb34d58d8fbdc63817ba854589de1374265ca5f5ee8860fae59590ed\","
        "\"return_value\":");
    putu(e);
    put(",\"last_error\":0}\n");

    UINT code = e == 1234 ? 0 : 1;
    head("GUEST_THREAD_EXITED", tid, 3);
    put("\"exit_code\":");
    putu(code);
    put("}\n");

    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    ExitProcess(code);
}
