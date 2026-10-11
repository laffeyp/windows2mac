// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* messages.c — FormatMessageW from the system message table (Sprint 049, family "format message").
 *
 * contracts/windows.messages.format_message records Server 2025 and 2022 answering mono's form (equal rows):
 * FROM_SYSTEM | IGNORE_INSERTS, with ALLOCATE_BUFFER or a caller's buffer. The text is the table's (specs/
 * system_messages.json, build 26100), inserts such as %1 kept; the answer is its length in units, the NUL after it.
 * An HRESULT of FACILITY_WIN32 (0x8007xxxx) names its low 16 bits; an id the table lacks is ERROR_MR_MID_NOT_FOUND;
 * LANG_NEUTRAL, LANG_USER_DEFAULT and en-US give the text, de-DE ERROR_RESOURCE_LANG_NOT_FOUND; a caller's buffer
 * shorter than the text and its NUL is ERROR_INSUFFICIENT_BUFFER, as many units as fit written first;
 * FORMAT_MESSAGE_MAX_WIDTH_MASK turns the closing CR LF into one space. Success leaves the last error. Other flags,
 * sources, languages and widths are not in the rows and stop the run.
 */
#include "runtime.h"
#include "system_messages.h"

#define ALLOCATE_BUFFER 0x100u
#define IGNORE_INSERTS 0x200u
#define FROM_SYSTEM 0x1000u
#define MAX_WIDTH_MASK 0xFFu
#define ERROR_INSUFFICIENT_BUFFER 122u
#define ERROR_MR_MID_NOT_FOUND 317u
#define ERROR_RESOURCE_LANG_NOT_FOUND 15100u

static int find(u32 id) {
    int lo = 0, hi = W2M_MESSAGE_COUNT - 1;
    while (lo <= hi) {
        int mid = (lo + hi) / 2;
        if (w2m_message_id[mid] == id) return mid;
        if (w2m_message_id[mid] < id) lo = mid + 1;
        else hi = mid - 1;
    }
    return -1;
}

/* FormatMessageW(dwFlags, lpSource, dwMessageId, dwLanguageId, lpBuffer, nSize, Arguments) */
u64 k32_FormatMessageW(u64 flags, u64 source, u64 id, u64 language, u64 rsp) {
    u64 buffer = *(u64 *)(rsp + 0x28);
    u32 size = *(u32 *)(rsp + 0x30);
    flags = (u32)flags;
    u32 width = (u32)flags & MAX_WIDTH_MASK;
    if ((flags & ~(u64)(ALLOCATE_BUFFER | IGNORE_INSERTS | FROM_SYSTEM | MAX_WIDTH_MASK)) || !(flags & FROM_SYSTEM) ||
        !(flags & IGNORE_INSERTS) || (width && width != MAX_WIDTH_MASK) || source)
        w2m_unimplemented(W2M_API_FormatMessageW, flags);
    id = (u32)id;
    if (id >> 16 == 0x8007) id &= 0xFFFF;                     /* HRESULT_FROM_WIN32 (row form.hresult_win32) */
    else if (id >> 16) w2m_unimplemented(W2M_API_FormatMessageW, id);
    language = (u32)language;
    u32 error = 0;
    if (language == 0x407) error = ERROR_RESOURCE_LANG_NOT_FOUND;
    else if (language != 0 && language != 0x400 && language != 0x409) w2m_unimplemented(W2M_API_FormatMessageW, language);
    int k = error ? -1 : find((u32)id);
    if (!error && k < 0) error = ERROR_MR_MID_NOT_FOUND;
    if (error) {
        if (flags & ALLOCATE_BUFFER) *(u64 *)buffer = 0;      /* the rows: the pointer is not left as it was */
        w2m_set_last_error(error);
        return w2m_logged(W2M_API_FormatMessageW, id, 0);
    }
    const char *text = w2m_message_text + w2m_message_offset[k];
    u32 len = w2m_message_offset[k + 1] - w2m_message_offset[k];
    int space = 0;
    if (width) {                                              /* the closing CR LF as one space (row form.max_width) */
        for (u32 i = 0; i + 2 < len; i++)
            if (text[i] == '\r') w2m_unimplemented(W2M_API_FormatMessageW, id);   /* a line break inside: not in the rows */
        if (len < 2 || text[len - 2] != '\r') w2m_unimplemented(W2M_API_FormatMessageW, id);
        len -= 1;
        space = 1;
    }
    unsigned short *out;
    if (flags & ALLOCATE_BUFFER) {
        out = (unsigned short *)w2m_heap_take(((u64)len + 1) * 2);   /* LocalFree frees it (local_memory.c) */
        if (!out) w2m_unimplemented(W2M_API_FormatMessageW, len);
        *(u64 *)buffer = (u64)out;
    } else {
        out = (unsigned short *)buffer;
        if (size < len + 1) {
            for (u32 i = 0; i < size && i < len; i++) out[i] = (unsigned char)text[i];
            w2m_set_last_error(ERROR_INSUFFICIENT_BUFFER);
            return w2m_logged(W2M_API_FormatMessageW, id, 0);
        }
    }
    for (u32 i = 0; i < len; i++) out[i] = (unsigned char)text[i];
    if (space) out[len - 1] = ' ';
    out[len] = 0;
    return w2m_logged(W2M_API_FormatMessageW, id, len);
}
