// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* bcrypt.c — bcrypt's BCryptGenRandom (Sprint 047, family "generate random").
 *
 * As contracts/windows.bcrypt.gen_random records Windows answering (Server 2025 and 2022, identical rows):
 *   - a NULL handle with BCRYPT_USE_SYSTEM_PREFERRED_RNG fills the buffer and returns STATUS_SUCCESS, the
 *     length 0 included; BCRYPT_RNG_USE_ENTROPY_IN_BUFFER beside it is accepted
 *   - a NULL buffer, any other flag bit, or a handle with the system flag: STATUS_INVALID_PARAMETER
 *   - a NULL handle without the system flag: STATUS_INVALID_HANDLE
 *   - bytes past the asked length stay as they were; the last error is never touched
 * The bytes are the host's CSPRNG (os.urandom), W2M_HOST_RANDOM. An algorithm handle from
 * BCryptOpenAlgorithmProvider is not in the rows.
 */
#include "runtime.h"

#define STATUS_SUCCESS 0u
#define STATUS_INVALID_HANDLE 0xC0000008u
#define STATUS_INVALID_PARAMETER 0xC000000Du
#define SYSTEM_PREFERRED_RNG 0x2u
#define USE_ENTROPY_IN_BUFFER 0x1u

u64 bcrypt_BCryptGenRandom(u64 handle, u64 buffer, u64 length, u64 flags) {
    u32 status;
    flags &= 0xFFFFFFFFu;
    if (flags & ~(SYSTEM_PREFERRED_RNG | USE_ENTROPY_IN_BUFFER)) status = STATUS_INVALID_PARAMETER;
    else if (flags & SYSTEM_PREFERRED_RNG) status = handle ? STATUS_INVALID_PARAMETER : STATUS_SUCCESS;
    else if (!handle) status = STATUS_INVALID_HANDLE;
    else { w2m_unimplemented(W2M_API_BCryptGenRandom, handle); status = 0; }    /* a provider handle */
    if (status == STATUS_SUCCESS && !buffer) status = STATUS_INVALID_PARAMETER;
    if (status == STATUS_SUCCESS && (length & 0xFFFFFFFFu)) w2m_host(W2M_HOST_RANDOM, length & 0xFFFFFFFFu, buffer, 0, 0, 0);
    return w2m_logged(W2M_API_BCryptGenRandom, length, status);
}
