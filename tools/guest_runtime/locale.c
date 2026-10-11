// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* locale.c — the locale the game asks for (Sprint 049, family "locale"; specs/work_list.json).
 *
 * contracts/windows.locale.locale records Server 2025 and 2022 (runs 38120346754, 38120348019; equal): the runners'
 * locale is en-US (LCID 0x409), as w2m's is. GetThreadLocale answers 0x409. GetLocaleInfoA answers the ISO 639
 * language "en" and the ISO 3166 country "US" for 0x409, LOCALE_USER_DEFAULT and LOCALE_SYSTEM_DEFAULT, in chars
 * with the NUL; a short buffer gets as many chars as fit, no NUL, and ERROR_INSUFFICIENT_BUFFER; size 0 asks the
 * length; a negative size is ERROR_INVALID_PARAMETER. GetUserDefaultLocaleName answers "en-US", 6; a short buffer gets
 * size - 1 units and a NUL, and both it and size 0 answer 0 with error 266. Success leaves the last error. Other
 * LCIDs and LCTYPEs are not in the rows and stop the run.
 */
#include "runtime.h"

#define LCID_EN_US 0x409u
#define LOCALE_USER_DEFAULT 0x400u
#define LOCALE_SYSTEM_DEFAULT 0x800u
#define LOCALE_SISO639LANGNAME 0x59u
#define LOCALE_SISO3166CTRYNAME 0x5Au
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_INSUFFICIENT_BUFFER 122u
#define ERROR_CANNOT_COPY 266u                  /* what GetUserDefaultLocaleName sets for a short buffer (the rows) */

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

u64 k32_GetThreadLocale(void) { return w2m_logged(W2M_API_GetThreadLocale, 0, LCID_EN_US); }

/* GetLocaleInfoA(Locale, LCType, lpLCData, cchData) */
u64 k32_GetLocaleInfoA(u64 lcid, u64 type, u64 data, u64 size) {
    lcid = (u32)lcid;
    type = (u32)type;
    if (lcid != LCID_EN_US && lcid != LOCALE_USER_DEFAULT && lcid != LOCALE_SYSTEM_DEFAULT)
        w2m_unimplemented(W2M_API_GetLocaleInfoA, lcid);
    const char *value = type == LOCALE_SISO639LANGNAME ? "en" : type == LOCALE_SISO3166CTRYNAME ? "US" : 0;
    if (!value) w2m_unimplemented(W2M_API_GetLocaleInfoA, type);
    int cch = (int)(u32)size;
    if (cch < 0) {
        set_error(ERROR_INVALID_PARAMETER);
        return w2m_logged(W2M_API_GetLocaleInfoA, type, 0);
    }
    u32 need = 3;                                   /* both values are two chars and the NUL */
    if (cch == 0) return w2m_logged(W2M_API_GetLocaleInfoA, type, need);
    char *out = (char *)data;
    for (u32 i = 0; i < need && i < (u32)cch; i++) out[i] = value[i];
    if ((u32)cch < need) {
        set_error(ERROR_INSUFFICIENT_BUFFER);
        return w2m_logged(W2M_API_GetLocaleInfoA, type, 0);
    }
    return w2m_logged(W2M_API_GetLocaleInfoA, type, need);
}

/* GetUserDefaultLocaleName(lpLocaleName, cchLocaleName) */
u64 k32_GetUserDefaultLocaleName(u64 name, u64 size) {
    static const unsigned short value[] = {'e', 'n', '-', 'U', 'S', 0};
    u32 need = 6, cch = (u32)size;
    unsigned short *out = (unsigned short *)name;
    if (cch < need) {
        if (cch) {
            for (u32 i = 0; i + 1 < cch; i++) out[i] = value[i];
            out[cch - 1] = 0;
        }
        set_error(ERROR_CANNOT_COPY);
        return w2m_logged(W2M_API_GetUserDefaultLocaleName, cch, 0);
    }
    for (u32 i = 0; i < need; i++) out[i] = value[i];
    return w2m_logged(W2M_API_GetUserDefaultLocaleName, cch, need);
}
