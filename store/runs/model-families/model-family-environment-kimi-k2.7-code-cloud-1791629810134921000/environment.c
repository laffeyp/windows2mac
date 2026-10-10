#include "runtime.h"

typedef unsigned short u16;

static u64 env_block_length(void) {
    const u16 *p = (const u16 *)W2M_ENVIRONMENT_VA;
    if (!p[0])
        return 1;
    u64 i = 0;
    while (p[i] || p[i + 1])
        i++;
    return i + 2;
}

u64 k32_GetEnvironmentStringsW(void) {
    u64 len = env_block_length();
    u64 bytes = len * sizeof(u16);
    if (bytes < 4)
        bytes = 4;
    u64 block = w2m_heap_take(bytes);
    if (!block)
        return w2m_logged(W2M_API_GetEnvironmentStringsW, 0, 0);

    const u16 *src = (const u16 *)W2M_ENVIRONMENT_VA;
    u16 *dst = (u16 *)block;
    for (u64 i = 0; i < len; i++)
        dst[i] = src[i];

    return w2m_logged(W2M_API_GetEnvironmentStringsW, 0, block);
}

u64 k32_FreeEnvironmentStringsW(u64 block) {
    if (block)
        w2m_heap_release(W2M_API_FreeEnvironmentStringsW, block);
    return w2m_logged(W2M_API_FreeEnvironmentStringsW, block, 1);
}

u64 k32_lstrlenW(u64 string) {
    const u16 *p = (const u16 *)string;
    u64 n = 0;
    while (p[n])
        n++;
    return w2m_logged(W2M_API_lstrlenW, string, n);
}
