// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* code_pages.c — code pages and character data (Sprint 044, family "code pages").
 *
 * GetACP, GetOEMCP, IsValidCodePage, GetCPInfo, MultiByteToWideChar, WideCharToMultiByte,
 * GetStringTypeW, LCMapStringW and LCMapStringEx, as contracts/windows.code_pages.ansi_1252/0.2.0.json records Windows
 * answering on sixteen runners (Server 2025 and 2022; AMD EPYC 7763, 9V45, 9V74; Intel Xeon 8370C;
 * store/probe-runs/380440* to 380442*, 32 identical outputs):
 *   - the ANSI code page is 1252 and the OEM code page 437; IsValidCodePage(CP_ACP) and
 *     IsValidCodePage(CP_OEMCP) are FALSE
 *   - code page 1252's bytes, character types and case come from code_page_tables.h, generated from
 *     the rows (src/w2m/code_pages.py); a UTF-16 unit outside the table converts to 1252 by best fit
 *     (452 units) or else to '?' with *lpUsedDefaultChar set
 *   - malformed UTF-8 becomes U+FFFD by the Unicode Standard's Table 3-7 with one difference the rows
 *     show (3,072 malformed inputs): after E0, ED, F0 or F4, a second byte in 80..BF outside the
 *     table's narrower range is taken into the U+FFFD, where WHATWG's maximal subpart leaves it
 *   - MB_ERR_INVALID_CHARS refuses exactly the inputs that would make a U+FFFD:
 *     ERROR_NO_UNICODE_TRANSLATION
 *   - MB_PRECOMPOSED changes nothing on code page 1252 (row forms.m2w.precomposed)
 *   - a destination too small: 0, ERROR_INSUFFICIENT_BUFFER; a length of 0 or an unknown code page:
 *     0, ERROR_INVALID_PARAMETER; success leaves the last error, GetStringTypeW's and LCMapStringEx's too
 * Anything the rows do not show stops the run (w2m_unimplemented): another code page, other flags,
 * a default character, a character outside the table given to GetStringTypeW or LCMapStringW.
 */
#include "runtime.h"
#include "code_page_tables.h"

typedef unsigned short u16;

#define CP_ACP 0u
#define CP_OEMCP 1u
#define CP_UTF8 65001u
#define ANSI_CP 1252u
#define OEM_CP 437u
#define UNKNOWN_CP 12345u                /* the rows' code page that Windows does not know */
#define MB_PRECOMPOSED 0x1u
#define MB_ERR_INVALID_CHARS 0x8u
#define CT_CTYPE1 1u
#define LOCALE_INVARIANT 0x7Fu
#define LOCALE_USER_DEFAULT 0x400u
#define LCMAP_LOWERCASE 0x100u
#define LCMAP_UPPERCASE 0x200u
#define ERROR_INVALID_PARAMETER 87u
#define ERROR_INSUFFICIENT_BUFFER 122u
#define ERROR_NO_UNICODE_TRANSLATION 1113u
#define REPLACEMENT 0xFFFDu
#define MALFORMED 0xFFFFFFFFu            /* utf8_next: input that becomes REPLACEMENT */
#define DEFAULT_CHAR '?'

static void set_error(u32 code) { w2m_teb()->last_error_value = code; }

u64 k32_GetACP(void) { return w2m_logged(W2M_API_GetACP, 0, ANSI_CP); }
u64 k32_GetOEMCP(void) { return w2m_logged(W2M_API_GetOEMCP, 0, OEM_CP); }

u64 k32_IsValidCodePage(u64 cp) {
    cp = (u32)cp;
    if (cp == ANSI_CP || cp == OEM_CP || cp == CP_UTF8 || cp == 932) return w2m_logged(W2M_API_IsValidCodePage, cp, 1);
    if (cp == CP_ACP || cp == CP_OEMCP || cp == UNKNOWN_CP) return w2m_logged(W2M_API_IsValidCodePage, cp, 0);
    w2m_unimplemented(W2M_API_IsValidCodePage, cp);
    return 0;
}

/* CPINFO: MaxCharSize, DefaultChar[2], LeadByte[12]. The rows show the first four lead bytes, all zero;
 * the other eight are written zero too (a single-byte page has no lead-byte ranges). */
u64 k32_GetCPInfo(u64 cp, u64 info) {
    cp = (u32)cp;
    u32 max;
    if (cp == CP_ACP || cp == ANSI_CP || cp == OEM_CP) max = 1;
    else if (cp == CP_UTF8) max = 4;
    else if (cp == UNKNOWN_CP) { set_error(ERROR_INVALID_PARAMETER); return w2m_logged(W2M_API_GetCPInfo, cp, 0); }
    else { w2m_unimplemented(W2M_API_GetCPInfo, cp); return 0; }
    u8 *p = (u8 *)info;
    *(u32 *)p = max;
    p[4] = DEFAULT_CHAR;
    for (int i = 5; i < 18; i++) p[i] = 0;
    return w2m_logged(W2M_API_GetCPInfo, cp, 1);
}

static int ansi_index(u32 unit) {                 /* the byte whose character is `unit`, or -1 */
    if (unit < 0x80) return (int)unit;
    for (int b = 0x80; b < 256; b++)
        if (w2m_ansi_unit[b] == unit) return b;
    return -1;
}

static int best_fit(u32 unit) {
    int lo = 0, hi = W2M_BEST_FIT_COUNT - 1;
    while (lo <= hi) {
        int mid = (lo + hi) / 2;
        if (w2m_best_fit[mid].unit == unit) return w2m_best_fit[mid].byte;
        if (w2m_best_fit[mid].unit < unit) lo = mid + 1;
        else hi = mid - 1;
    }
    return -1;
}

/* One UTF-8 character from s[*i..n): its code point, or MALFORMED; *i moves past what it took. */
static u32 utf8_next(const u8 *s, u64 n, u64 *i) {
    u32 c = s[*i];
    (*i)++;
    if (c < 0x80) return c;
    u32 len, lo = 0x80, hi = 0xBF;
    if (c >= 0xC2 && c <= 0xDF) len = 2;
    else if (c == 0xE0) { len = 3; lo = 0xA0; }
    else if (c == 0xED) { len = 3; hi = 0x9F; }
    else if (c >= 0xE1 && c <= 0xEF) len = 3;
    else if (c == 0xF0) { len = 4; lo = 0x90; }
    else if (c >= 0xF1 && c <= 0xF3) len = 4;
    else if (c == 0xF4) { len = 4; hi = 0x8F; }
    else return MALFORMED;                      /* 80..C1, F5..FF */
    u32 cp = c & (0x7Fu >> len);
    for (u32 k = 1; k < len; k++) {
        if (*i >= n || s[*i] < 0x80 || s[*i] > 0xBF) return MALFORMED;
        if (k == 1 && (s[*i] < lo || s[*i] > hi)) { (*i)++; return MALFORMED; }   /* taken: the rows */
        cp = cp << 6 | (s[*i] & 0x3Fu);
        (*i)++;
    }
    return cp;
}

u64 k32_MultiByteToWideChar(u64 cp, u64 flags, u64 src, u64 src_len, u64 rsp) {
    u64 dst = *(u64 *)(rsp + 0x28);
    int cap = *(int *)(rsp + 0x30);
    cp = (u32)cp;
    flags = (u32)flags;
    int len = (int)src_len;
    int utf8 = cp == CP_UTF8;
    if (cp == UNKNOWN_CP) { set_error(ERROR_INVALID_PARAMETER); return w2m_logged(W2M_API_MultiByteToWideChar, cp, 0); }
    if (!(cp == CP_ACP || cp == ANSI_CP || utf8)) w2m_unimplemented(W2M_API_MultiByteToWideChar, cp);
    if (utf8 ? (flags & ~(MB_PRECOMPOSED | MB_ERR_INVALID_CHARS)) != 0 : (flags & ~MB_PRECOMPOSED) != 0)
        w2m_unimplemented(W2M_API_MultiByteToWideChar, flags);
    if (len == 0) { set_error(ERROR_INVALID_PARAMETER); return w2m_logged(W2M_API_MultiByteToWideChar, cp, 0); }
    if (len < -1 || cap < 0 || (cap > 0 && dst == 0)) w2m_unimplemented(W2M_API_MultiByteToWideChar, (u64)len);
    const u8 *s = (const u8 *)src;
    u64 n = (u64)len;
    if (len == -1) { n = 0; while (s[n]) n++; n++; }
    u16 *out = (u16 *)dst;
    u64 count = 0;
    for (u64 i = 0; i < n;) {
        u32 c = utf8 ? utf8_next(s, n, &i) : w2m_ansi_unit[s[i++]];
        if (c == MALFORMED) {
            if (flags & MB_ERR_INVALID_CHARS) {
                set_error(ERROR_NO_UNICODE_TRANSLATION);
                return w2m_logged(W2M_API_MultiByteToWideChar, cp, 0);
            }
            c = REPLACEMENT;
        }
        u64 units = c > 0xFFFF ? 2 : 1;
        if (cap) {
            if (count + units > (u64)cap) { set_error(ERROR_INSUFFICIENT_BUFFER); return w2m_logged(W2M_API_MultiByteToWideChar, cp, 0); }
            if (units == 2) { out[count] = (u16)(0xD800 + ((c - 0x10000) >> 10)); out[count + 1] = (u16)(0xDC00 + (c & 0x3FF)); }
            else out[count] = (u16)c;
        }
        count += units;
    }
    return w2m_logged(W2M_API_MultiByteToWideChar, cp, count);
}

u64 k32_WideCharToMultiByte(u64 cp, u64 flags, u64 src, u64 src_len, u64 rsp) {
    u64 dst = *(u64 *)(rsp + 0x28);
    int cap = *(int *)(rsp + 0x30);
    u64 default_char = *(u64 *)(rsp + 0x38);
    u32 *used_default = *(u32 **)(rsp + 0x40);
    cp = (u32)cp;
    int len = (int)src_len;
    int utf8 = cp == CP_UTF8;
    if (!(cp == CP_ACP || cp == ANSI_CP || utf8)) w2m_unimplemented(W2M_API_WideCharToMultiByte, cp);
    if ((u32)flags != 0 || default_char != 0) w2m_unimplemented(W2M_API_WideCharToMultiByte, (u32)flags);
    if (len == 0 || len < -1 || cap < 0 || (cap > 0 && dst == 0)) w2m_unimplemented(W2M_API_WideCharToMultiByte, (u64)len);
    const u16 *s = (const u16 *)src;
    u64 n = (u64)len;
    if (len == -1) { n = 0; while (s[n]) n++; n++; }
    u8 *out = (u8 *)dst;
    u64 count = 0;
    u32 defaulted = 0;
    for (u64 i = 0; i < n; i++) {
        u32 c = s[i];
        u8 bytes[4];
        u32 k = 0;
        if (utf8) {
            if (c >= 0xD800 && c <= 0xDBFF && i + 1 < n && s[i + 1] >= 0xDC00 && s[i + 1] <= 0xDFFF) {
                c = 0x10000 + ((c - 0xD800) << 10) + (s[i + 1] - 0xDC00);
                i++;
            } else if (c >= 0xD800 && c <= 0xDFFF) {
                if (used_default) w2m_unimplemented(W2M_API_WideCharToMultiByte, c);   /* not in the rows */
                c = REPLACEMENT;                   /* row w2m.utf8.lone_surrogate */
            }
            if (c < 0x80) bytes[k++] = (u8)c;
            else if (c < 0x800) { bytes[k++] = (u8)(0xC0 | c >> 6); bytes[k++] = (u8)(0x80 | (c & 0x3F)); }
            else if (c < 0x10000) { bytes[k++] = (u8)(0xE0 | c >> 12); bytes[k++] = (u8)(0x80 | (c >> 6 & 0x3F)); bytes[k++] = (u8)(0x80 | (c & 0x3F)); }
            else { bytes[k++] = (u8)(0xF0 | c >> 18); bytes[k++] = (u8)(0x80 | (c >> 12 & 0x3F));
                   bytes[k++] = (u8)(0x80 | (c >> 6 & 0x3F)); bytes[k++] = (u8)(0x80 | (c & 0x3F)); }
        } else {
            if (c >= 0xD800 && c <= 0xDBFF && i + 1 < n && s[i + 1] >= 0xDC00 && s[i + 1] <= 0xDFFF)
                w2m_unimplemented(W2M_API_WideCharToMultiByte, c);   /* a pair into 1252: not in the rows */
            int b = ansi_index(c);
            if (b < 0) b = best_fit(c);
            if (b < 0) { b = DEFAULT_CHAR; defaulted = 1; }
            bytes[k++] = (u8)b;
        }
        if (cap) {
            if (count + k > (u64)cap) { set_error(ERROR_INSUFFICIENT_BUFFER); return w2m_logged(W2M_API_WideCharToMultiByte, cp, 0); }
            for (u32 j = 0; j < k; j++) out[count + j] = bytes[j];
        }
        count += k;
    }
    if (used_default) *used_default = defaulted;
    return w2m_logged(W2M_API_WideCharToMultiByte, cp, count);
}

/* The rows give CT_CTYPE1 and case for code page 1252's 256 characters; any other unit stops the run. */
static int table_index(u64 api, u32 unit) {
    int b = ansi_index(unit);
    if (b < 0) w2m_unimplemented(api, unit);
    return b;
}

u64 k32_GetStringTypeW(u64 type, u64 src, u64 len, u64 out) {
    int n = (int)len;
    if ((u32)type != CT_CTYPE1 || n < 1) w2m_unimplemented(W2M_API_GetStringTypeW, (u32)type);
    const u16 *s = (const u16 *)src;
    u16 *types = (u16 *)out;
    for (int i = 0; i < n; i++) types[i] = w2m_ansi_ctype1[table_index(W2M_API_GetStringTypeW, s[i])];
    return w2m_logged(W2M_API_GetStringTypeW, (u32)type, 1);
}

u64 k32_LCMapStringW(u64 locale, u64 flags, u64 src, u64 len, u64 rsp) {
    u64 dst = *(u64 *)(rsp + 0x28);
    int cap = *(int *)(rsp + 0x30);
    int n = (int)len;
    locale = (u32)locale;
    flags = (u32)flags;
    if ((locale != LOCALE_USER_DEFAULT && locale != LOCALE_INVARIANT) ||
        (flags != LCMAP_UPPERCASE && flags != LCMAP_LOWERCASE))
        w2m_unimplemented(W2M_API_LCMapStringW, flags);
    if (n < 1 || cap < n || dst == 0) w2m_unimplemented(W2M_API_LCMapStringW, (u64)n);   /* size queries: not in the rows */
    const u16 *s = (const u16 *)src;
    u16 *out = (u16 *)dst;
    const unsigned short *map = flags == LCMAP_UPPERCASE ? w2m_ansi_upper : w2m_ansi_lower;
    for (int i = 0; i < n; i++) out[i] = map[table_index(W2M_API_LCMapStringW, s[i])];
    return w2m_logged(W2M_API_LCMapStringW, flags, (u64)n);
}

/* LCMapStringEx(lpLocaleName, dwMapFlags, lpSrcStr, cchSrc, lpDestStr, cchDest, lpVersionInformation,
 * lpReserved, sortHandle): the C runtime reaches it through GetProcAddress (frontier 2026-10-10). A NULL
 * locale name maps as LCMapStringW's user default (rows forms.lcmap_ex.*); the size query answers the
 * source's length. */
u64 k32_LCMapStringEx(u64 locale_name, u64 flags, u64 src, u64 len, u64 rsp) {
    u64 dst = *(u64 *)(rsp + 0x28);
    int cap = *(int *)(rsp + 0x30);
    u64 version = *(u64 *)(rsp + 0x38), reserved = *(u64 *)(rsp + 0x40), sort = *(u64 *)(rsp + 0x48);
    int n = (int)len;
    flags = (u32)flags;
    if (locale_name != 0 || version != 0 || reserved != 0 || sort != 0 ||
        (flags != LCMAP_UPPERCASE && flags != LCMAP_LOWERCASE))
        w2m_unimplemented(W2M_API_LCMapStringEx, flags);
    if (n < 1 || (cap != 0 && (cap < n || dst == 0))) w2m_unimplemented(W2M_API_LCMapStringEx, (u64)n);
    const u16 *s = (const u16 *)src;
    if (cap == 0) {                               /* the size query: row forms.lcmap_ex.*.size */
        for (int i = 0; i < n; i++) (void)table_index(W2M_API_LCMapStringEx, s[i]);
        return w2m_logged(W2M_API_LCMapStringEx, flags, (u64)n);
    }
    u16 *out = (u16 *)dst;
    const unsigned short *map = flags == LCMAP_UPPERCASE ? w2m_ansi_upper : w2m_ansi_lower;
    for (int i = 0; i < n; i++) out[i] = map[table_index(W2M_API_LCMapStringEx, s[i])];
    return w2m_logged(W2M_API_LCMapStringEx, flags, (u64)n);
}

/* AreFileApisANSI: TRUE, the file functions use the ANSI code page (contracts/windows.loader.static_tls row
 * file_apis_ansi; Microsoft Learn: the default until SetFileApisToOEM, which w2m does not serve). */
u64 k32_AreFileApisANSI(void) { return w2m_logged(W2M_API_AreFileApisANSI, 0, 1); }
