// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* vm_compiler.c — the compiler inside the VM, for one instruction (Sprint 077). See vm_compiler.h.
 *
 * The key is scripts/stencil_census.py's decoded_form written as bytes (scripts/gen_stencil_table.py key_bytes): the
 * spec row, iclass, operand and address size, VEX.L; per operand its kind and shape (a register's class letter, or
 * its name for the stack pointer and RIP; a memory operand's segment, base, index, scale and size; an immediate's
 * width); which registers are equal; which equal the row's implicit registers; each value's class; the flag
 * arguments. Freestanding: no library calls.
 */
#include "vm_compiler.h"
#include "a64_encoder.h"

struct vmc_item { u8 encoder, kind, count; u32 first; };
struct vmc_stencil { u32 first; u16 count; };
struct vmc_choice { u64 value; u32 stencil; };
struct vmc_entry { u64 hash; u32 key; u16 length; u8 feature, position; u32 first; u16 count; };

enum { T_SMALL, T_POOL, T_HOLE, T_SYMBOL, T_EXIT };
enum { K_ENC, K_ENC32, K_BRANCH };

#include "stencil_table.h"

/* stencil_census.value_class, as bits in key_bytes' order */
static u32 value_class(u64 m) {
    s64 sv = (s64)m;
    u32 c = 0;
    int i = 0;
    c |= (u32)(sv == 0) << i++;
    c |= (u32)(sv < 0) << i++;
    c |= (u32)(sv >= 0 && sv < 4096) << i++;
    c |= (u32)(sv > -4096 && sv < 4096) << i++;
    c |= (u32)(sv >= -256 && sv <= 255) << i++;
    for (s64 size = 1; size <= 16; size *= 2)
        c |= (u32)(sv % size == 0 && sv / size >= 0 && sv / size < 4096) << i++;
    for (int h = 0; h < 4; h++)
        c |= (u32)((m >> (16 * h) & 0xFFFF) != 0) << i++;
    c |= (u32)((m & 31) == 1) << i++;
    c |= (u32)((m & 63) == 1) << i++;
    c |= (u32)((m & 31) == 0) << i++;
    c |= (u32)((m & 63) == 0) << i++;
    return c;
}

struct key_builder {
    u8 *key;
    int k;
    u64 regs[VMC_MAX_VALUES];
    int nregs;
    u64 vals[VMC_MAX_VALUES];
    int nvals;
    int overflow;
};

static void put(struct key_builder *b, int byte) {
    if (b->k < VMC_MAX_KEY)
        b->key[b->k++] = (u8)byte;
    else
        b->overflow = 1;
}

static void value(struct key_builder *b, u64 v) {
    if (b->nvals < VMC_MAX_VALUES)
        b->vals[b->nvals++] = v;
    else
        b->overflow = 1;
}

/* a register named by the operand: its code in the key; a register with a class letter joins the values */
static int reg(struct key_builder *b, int cls, int n) {
    if (cls == X86_NONE)
        return 0;
    int code = VMC_REG_CODE[cls][n & 31];
    if (code >= 1 && code <= 7) {
        if (b->nregs < VMC_MAX_VALUES)
            b->regs[b->nregs++] = VMC_REG_NUMBER[cls][n & 31];
        else
            b->overflow = 1;
    }
    return code;
}

static int implicit_bit(u64 r) {
    if (r < 16)
        return (int)r;
    if (r >= 1000 && r < 1016)
        return 16 + (int)(r - 1000);
    return -1;
}

int vmc_key(const u8 *code, int n, u64 address, const struct vmc_flags *flags, u8 *key, u64 *values,
            int *nvalues, int *length) {
    struct x86_decoded d;
    if (x86_decode(code, n, &d) != X86_OK)
        return VMC_DECODE;
    struct x86_operand ops[4];
    int nops = 0;
    if (d.f.ops != X86_NO_TEMPLATE) {
        nops = x86_operands(&d, ops);
        if (nops < 0)
            return VMC_DECODE;
    }
    *length = d.length;
    u64 nxt = address + d.length;
    struct key_builder b = {.key = key};
    value(&b, nxt);
    put(&b, d.row & 0xFF);
    put(&b, d.row >> 8);
    put(&b, d.iclass & 0xFF);
    put(&b, d.iclass >> 8);
    put(&b, d.f.opsize);
    put(&b, d.f.asize);
    put(&b, d.f.vl);
    put(&b, nops);
    for (int i = 0; i < nops; i++) {
        const struct x86_operand *o = &ops[i];
        int f[6] = {0};
        switch (o->kind) {
        case X86_OP_R:
            f[0] = 1;
            f[1] = reg(&b, o->reg_class, o->reg);
            break;
        case X86_OP_M:
        case X86_OP_A:
            f[0] = o->kind == X86_OP_M ? 2 : 3;
            f[1] = o->kind == X86_OP_M ? (o->seg == 0x64 ? 1 : o->seg == 0x65 ? 2 : 0) : 0;
            f[2] = reg(&b, o->base_class, o->base);
            f[3] = reg(&b, o->index_class, o->index);
            f[4] = o->scale;
            f[5] = o->kind == X86_OP_M ? o->bytes : 0;
            value(&b, (u64)o->disp);
            if (o->base_class == X86_RIP)
                value(&b, nxt + (u64)o->disp);
            break;
        case X86_OP_I: {
            f[0] = 4;
            f[1] = o->imm_len;
            u64 raw = o->imm;
            int bits = 8 * o->imm_len;
            u64 sext = bits < 64 && raw >> (bits - 1) & 1 ? raw - ((u64)1 << bits) : raw;
            value(&b, raw);
            value(&b, sext);
            break;
        }
        case X86_OP_J:
            f[0] = 5;
            value(&b, (u64)o->disp);
            value(&b, nxt + (u64)o->disp);
            break;
        default:                                         /* MM:n, ST:n */
            f[0] = o->kind == X86_OP_MM ? 6 : 7;
            f[1] = o->reg;
            break;
        }
        for (int j = 0; j < 6; j++)
            put(&b, f[j]);
    }
    put(&b, b.nregs);
    for (int i = 0; i < b.nregs; i++) {
        int first = 0;
        while (b.regs[first] != b.regs[i])
            first++;
        put(&b, first);
    }
    u32 mask = d.row < VMC_ROWS ? VMC_IMPLICIT[d.row] : 0;
    for (int i = 0; i < b.nregs; i++) {
        int bit = implicit_bit(b.regs[i]);
        u64 named = bit >= 0 && (mask >> bit & 1) ? b.regs[i] : 0xFFFF;
        put(&b, (int)(named & 0xFF));
        put(&b, (int)(named >> 8));
    }
    put(&b, b.nvals);
    for (int i = 0; i < b.nvals; i++) {
        u32 c = value_class(b.vals[i]);
        put(&b, c & 0xFF);
        put(&b, c >> 8 & 0xFF);
        put(&b, c >> 16);
    }
    put(&b, flags->nzcv_from);
    put(&b, flags->live_after);
    put(&b, flags->nzcv_out);
    if (b.overflow || b.nregs + b.nvals > VMC_MAX_VALUES)
        return VMC_KEY_SIZE;
    for (int i = 0; i < b.nregs; i++)
        values[i] = b.regs[i];
    for (int i = 0; i < b.nvals; i++)
        values[b.nregs + i] = b.vals[i];
    *nvalues = b.nregs + b.nvals;
    return b.k;
}

static u64 fnv1a(const u8 *p, int n) {
    u64 h = 0xCBF29CE484222325ull;
    for (int i = 0; i < n; i++)
        h = (h ^ p[i]) * 0x100000001B3ull;
    return h;
}

static const struct vmc_entry *lookup(const u8 *key, int n) {
    u64 h = fnv1a(key, n);
    int lo = 0, hi = VMC_KEYS;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;
        if (VMC_ENTRY[mid].hash < h)
            lo = mid + 1;
        else
            hi = mid;
    }
    if (lo == VMC_KEYS || VMC_ENTRY[lo].hash != h || VMC_ENTRY[lo].length != n)
        return 0;
    const u8 *k = &VMC_KEY_BYTES[VMC_ENTRY[lo].key];
    for (int i = 0; i < n; i++)
        if (k[i] != key[i])
            return 0;
    return &VMC_ENTRY[lo];
}

int vmc_compile(const u8 *code, int n, u64 address, u64 base, u64 exit_va, const struct vmc_flags *flags,
                const u64 *symbols, u32 *out, int max_words) {
    u8 key[VMC_MAX_KEY];
    u64 values[VMC_MAX_VALUES];
    int nvalues, length;
    int kn = vmc_key(code, n, address, flags, key, values, &nvalues, &length);
    if (kn < 0)
        return kn;
    const struct vmc_entry *e = lookup(key, kn);
    if (!e)
        return VMC_NO_STENCIL;
    u32 stencil = VMC_CHOICE[e->first].stencil;
    if (e->feature != VMC_FEATURE_NONE) {
        if (e->position >= nvalues)
            return VMC_NO_CHOICE;
        u64 fv = vmc_feature(e->feature, values[e->position]);
        int found = 0;
        for (int i = 0; i < e->count; i++)
            if (VMC_CHOICE[e->first + i].value == fv) {
                stencil = VMC_CHOICE[e->first + i].stencil;
                found = 1;
            }
        if (!found)
            return VMC_NO_CHOICE;
    }
    const struct vmc_stencil *s = &VMC_STENCIL[stencil];
    int w = 0;
    for (int i = 0; i < s->count; i++) {
        const struct vmc_item *it = &VMC_ITEM[VMC_STENCIL_ITEM[s->first + i]];
        u64 a[8];
        for (int j = 0; j < it->count; j++) {
            u32 arg = VMC_ARG[it->first + j];
            u32 payload = arg & 0x0FFFFFFF;
            switch (arg >> 28) {
            case T_SMALL:
                a[j] = (u64)((s64)payload - (1 << 27));
                break;
            case T_POOL:
                a[j] = VMC_POOL[payload];
                break;
            case T_HOLE:
                if ((int)(payload & 0xFF) >= nvalues)
                    return VMC_ENCODER;
                a[j] = vmc_expression((int)(payload >> 8), values[payload & 0xFF]);
                break;
            case T_SYMBOL:
                a[j] = symbols ? symbols[payload] : 0;
                break;
            default:                                     /* T_EXIT: the exit's offset from this branch */
                a[j] = exit_va - (base + 4 * (u64)w);
                break;
            }
        }
        u32 words[4];
        int count = vmc_encode(it->encoder, a, words);
        if (count <= 0)
            return VMC_ENCODER;
        if (w + count > max_words)
            return VMC_FULL;
        if (it->kind == K_ENC32)
            words[0] &= ~(1u << 31);
        for (int j = 0; j < count; j++)
            out[w++] = words[j];
    }
    return w;
}
