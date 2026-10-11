// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* token.c — ADVAPI32's OpenProcessToken, GetTokenInformation and GetSidSubAuthority for the process's integrity
 * level (Sprint 048, family "integrity level").
 *
 * As contracts/windows.token.integrity_level records Windows answering (Server 2025 and 2022):
 *   - OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY) answers TRUE and a handle, a multiple of 4; CloseHandle
 *     closes it; both leave the last error
 *   - GetTokenInformation(TokenIntegrityLevel) with no buffer or one byte short answers FALSE, 122 and 28 bytes; with
 *     28 it writes a TOKEN_MANDATORY_LABEL: the SID's address, then SE_GROUP_INTEGRITY | SE_GROUP_INTEGRITY_ENABLED
 *     (0x60); the SID at offset 16: revision 1, one sub-authority, authority 16 (mandatory label), the RID
 *   - GetSidSubAuthority answers the sub-authority's address and sets the last error to 0
 * The RID is SECURITY_MANDATORY_MEDIUM_RID (0x2000): "Standard users receive medium, elevated users receive high"
 * (Microsoft Learn, Mandatory Integrity Control), and a game started from Steam runs as a standard user. A runner's
 * process is elevated (0x3000); the family compares only whether the RID is low (0x1000), which is UnityPlayer's
 * question. Another process, another access or another class stops the run.
 */
#include "runtime.h"

#define CURRENT_PROCESS (~0ull)
#define TOKEN_QUERY 0x8u
#define TOKEN_INTEGRITY_LEVEL 25u
#define ERROR_INSUFFICIENT_BUFFER 122u
#define LABEL_SIZE 28u
#define SID_OFFSET 16u
#define MEDIUM_RID 0x2000u

u64 w2m_object_token(u64 api);       /* sync_objects.c: a token object's handle */
int w2m_object_is_token(u64 h);

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

/* OpenProcessToken(ProcessHandle, DesiredAccess, TokenHandle) */
u64 advapi32_OpenProcessToken(u64 process, u64 access, u64 out) {
    if (process != CURRENT_PROCESS) w2m_unimplemented(W2M_API_OpenProcessToken, process);
    if ((u32)access != TOKEN_QUERY) w2m_unimplemented(W2M_API_OpenProcessToken, access);
    if (!out) w2m_unimplemented(W2M_API_OpenProcessToken, 0);                         /* not in the rows */
    *(u64 *)out = w2m_object_token(W2M_API_OpenProcessToken);
    return w2m_logged(W2M_API_OpenProcessToken, access, 1);
}

/* GetTokenInformation(TokenHandle, TokenInformationClass, TokenInformation, TokenInformationLength, ReturnLength):
 * the fifth argument is on the guest's stack, at RSP + 0x28 on entry. */
u64 advapi32_GetTokenInformation(u64 token, u64 class, u64 info, u64 length, u64 rsp) {
    u64 returned = *(u64 *)(rsp + 0x28);
    if (!w2m_object_is_token(token)) w2m_unimplemented(W2M_API_GetTokenInformation, token);
    if ((u32)class != TOKEN_INTEGRITY_LEVEL) w2m_unimplemented(W2M_API_GetTokenInformation, class);
    if (!returned) w2m_unimplemented(W2M_API_GetTokenInformation, 0);                 /* not in the rows */
    *(u32 *)returned = LABEL_SIZE;
    if ((u32)length < LABEL_SIZE || !info) {
        set_error(ERROR_INSUFFICIENT_BUFFER);
        return w2m_logged(W2M_API_GetTokenInformation, class, 0);
    }
    u8 *l = (u8 *)info;
    *(u64 *)l = info + SID_OFFSET;                                   /* Label.Sid */
    *(u32 *)(l + 8) = 0x60;                                          /* SE_GROUP_INTEGRITY | SE_GROUP_INTEGRITY_ENABLED */
    *(u32 *)(l + 12) = 0;
    u8 *sid = l + SID_OFFSET;
    sid[0] = 1;                                                      /* revision */
    sid[1] = 1;                                                      /* sub-authorities */
    for (int i = 0; i < 5; i++) sid[2 + i] = 0;
    sid[7] = 16;                                                     /* SECURITY_MANDATORY_LABEL_AUTHORITY */
    *(u32 *)(sid + 8) = MEDIUM_RID;
    return w2m_logged(W2M_API_GetTokenInformation, class, 1);
}

/* GetSidSubAuthority(pSid, nSubAuthority) */
u64 advapi32_GetSidSubAuthority(u64 sid, u64 index) {
    if (!sid || (u32)index >= ((const u8 *)sid)[1]) w2m_unimplemented(W2M_API_GetSidSubAuthority, index);
    set_error(0);
    return w2m_logged(W2M_API_GetSidSubAuthority, index, sid + 8 + 4 * (u32)index);
}
