// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* network.c — WSAStartup and WSACleanup (Sprint 052, family "network"; specs/work_list.json).
 *
 * contracts/windows.network.network records Windows (Server 2025 and 2022, runs 38127406287, 38127407661; equal):
 *   - WSAStartup(wVersionRequested, lpWSAData): 0, and one more WSACleanup owed; the version granted is the one
 *     asked, at most 2.2; wHighVersion 2.2; "WinSock 2.0" and "Running"; iMaxSockets and iMaxUdpDg 0 from 2.0, 32767
 *     and 65467 for 1.x; lpVendorInfo non-NULL; 0.0 is WSAVERNOTSUPPORTED (10092) and owes nothing; a NULL lpWSAData
 *     is WSAEFAULT (10014) and sets the last error to 0; the first successful call sets it to 0, later ones leave it
 *   - WSACleanup: 0 and the last error 0 while one is owed; SOCKET_ERROR (-1) and WSANOTINITIALISED (10093) after
 *   - WSAGetLastError is the thread's last error
 * WSADATA on x64 (winsock2.h): wVersion 0, wHighVersion 2, iMaxSockets 4, iMaxUdpDg 6, lpVendorInfo 8,
 * szDescription 16 (257 bytes), szSystemStatus 273 (129 bytes).
 */
#include "runtime.h"

#define WSAVERNOTSUPPORTED 10092u
#define WSAEFAULT 10014u
#define WSANOTINITIALISED 10093u

static u64 owed;                                 /* WSAStartup calls not yet balanced by WSACleanup */
static u64 started;                              /* any WSAStartup has succeeded */
static char vendor_info;                         /* what lpVendorInfo points to (ignored from 2.0) */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

static void copy(char *to, const char *s) { while ((*to++ = *s++)) {} }

u64 ws2_32_WSAStartup(u64 requested, u64 data) {
    u32 major = (u32)requested & 0xFF, minor = (u32)requested >> 8 & 0xFF;
    if (!data) {
        set_error(0);
        return w2m_logged(W2M_API_WSAStartup, requested, WSAEFAULT);
    }
    if (major == 0) return w2m_logged(W2M_API_WSAStartup, requested, WSAVERNOTSUPPORTED);
    u32 version = major > 2 || (major == 2 && minor > 2) ? 0x0202u : ((u32)requested & 0xFFFF);
    u8 *d = (u8 *)data;
    for (int i = 0; i < 16 + 257 + 129; i++) d[i] = 0;
    *(unsigned short *)(d + 0) = (unsigned short)version;
    *(unsigned short *)(d + 2) = 0x0202;
    *(unsigned short *)(d + 4) = major < 2 ? 32767 : 0;
    *(unsigned short *)(d + 6) = major < 2 ? 65467 : 0;
    *(u64 *)(d + 8) = (u64)&vendor_info;
    copy((char *)d + 16, "WinSock 2.0");
    copy((char *)d + 273, "Running");
    if (!started) set_error(0);
    started = 1;
    owed++;
    return w2m_logged(W2M_API_WSAStartup, requested, 0);
}

u64 ws2_32_WSACleanup(void) {
    if (!owed) {
        set_error(WSANOTINITIALISED);
        return w2m_logged(W2M_API_WSACleanup, 0, 0xFFFFFFFFu);
    }
    owed--;
    set_error(0);
    return w2m_logged(W2M_API_WSACleanup, owed, 0);
}

u64 ws2_32_WSAGetLastError(void) { return w2m_logged(W2M_API_WSAGetLastError, 0, w2m_teb()->last_error_value); }
