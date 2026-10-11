// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* network.c — WSAStartup and WSACleanup (Sprint 052, family "network"; specs/work_list.json).
 *
 * No C runtime. The captures hold the forms: mono's WSAStartup(0x0002, &data) and UnityPlayer's WSAStartup(0x0202,
 * &data) (WS2_32 ordinal 115), both answering 0. Rows: the answer, the last error after SetLastError(0xDEAD), and
 * WSADATA's fields (wVersion, wHighVersion, the description and status strings, iMaxSockets, iMaxUdpDg) for the
 * game's versions and the edges: 1.1, 3.0, 0.0, a NULL WSADATA; then WSACleanup's balance.
 */
#include <winsock2.h>
#include <windows.h>

#define SENTINEL 0xDEADu

static char out[8192];
static unsigned n;
static void put(const char *s) { while (*s && n < sizeof out - 64) out[n++] = *s++; }
static void putu(unsigned long long v) {
    char t[24];
    int k = 0;
    do { t[k++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (k) out[n++] = t[--k];
}
static void row(const char *s) { put("{\"s\":\""); put(s); put("\""); }
static void field(const char *k, unsigned long long v) { put(",\""); put(k); put("\":"); putu(v); }
static void text(const char *k, const char *s) {
    put(",\""); put(k); put("\":\"");
    for (; *s; s++) if (*s >= 0x20 && *s < 0x7F && *s != '"' && *s != '\\') out[n++] = *s;
    put("\"");
}
/* Each row is written when it ends, so a fault leaves the rows before it. */
static void end(void) {
    put("}\n");
    DWORD written;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), out, n, &written, NULL);
    n = 0;
}

static void startup(const char *name, WORD version) {
    static WSADATA data;
    for (unsigned i = 0; i < sizeof data; i++) ((unsigned char *)&data)[i] = 0xA5;
    SetLastError(SENTINEL);
    int r = WSAStartup(version, &data);
    DWORD err = GetLastError();
    row(name); field("ret", (unsigned)r); field("err", err);
    if (r == 0) {
        field("version", data.wVersion); field("high_version", data.wHighVersion);
        text("description", data.szDescription); text("status", data.szSystemStatus);
        field("max_sockets", data.iMaxSockets); field("max_udp", data.iMaxUdpDg);
        field("vendor_info_null", data.lpVendorInfo == NULL);
    }
    end();
}

void start(void) {
    SetLastError(SENTINEL);
    int c0 = WSACleanup();
    row("cleanup.before_startup"); field("ret", (unsigned)c0); field("err", GetLastError()); field("wsa_err", (unsigned)WSAGetLastError()); end();
    startup("startup.2_0", 0x0002);
    startup("startup.2_2", 0x0202);
    startup("startup.1_1", 0x0101);
    startup("startup.3_0", 0x0003);
    startup("startup.0_0", 0x0000);
    SetLastError(SENTINEL);
    int rn = WSAStartup(0x0202, NULL);
    row("startup.null_data"); field("ret", (unsigned)rn); field("err", GetLastError()); end();
    for (int i = 0; i < 6; i++) {
        SetLastError(SENTINEL);
        int c = WSACleanup();
        row(i == 0 ? "cleanup.1" : i == 1 ? "cleanup.2" : i == 2 ? "cleanup.3" : i == 3 ? "cleanup.4" : i == 4 ? "cleanup.5" : "cleanup.6");
        field("ret", (unsigned)c); field("err", GetLastError()); end();
    }
    ExitProcess(0);
}
