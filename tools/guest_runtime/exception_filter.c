// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* exception_filter.c — the process's unhandled-exception filter (Sprint 044, family "exception filter").
 *
 * SetUnhandledExceptionFilter, as contracts/windows.exception_filter.set/0.1.0.json records Windows
 * answering on sixteen runners (32 identical outputs): a fresh process has none (the first call returns
 * NULL); each call returns the filter it replaces, NULL included; the last error is left. Calling the
 * filter on an unhandled exception is exception dispatch (roadmap Sprint 047), not this file.
 */
#include "runtime.h"

static u64 filter;

u64 w2m_unhandled_exception_filter(void) { return filter; }

u64 k32_SetUnhandledExceptionFilter(u64 f) {
    u64 before = filter;
    filter = f;
    return w2m_logged(W2M_API_SetUnhandledExceptionFilter, f, before);
}
