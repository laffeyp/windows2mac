// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* last_error.c — SetLastError/GetLastError in the guest (Sprint 035). The calling thread's
 * last-error code lives in its TEB at 0x68, where Windows keeps it (Sprint 034, check
 * teb.last_error). A generated candidate replaces this file. */
#include "runtime.h"

void w2m_set_last_error(u32 code) { w2m_teb()->last_error_value = code; }

u32 w2m_get_last_error(void) { return w2m_teb()->last_error_value; }
