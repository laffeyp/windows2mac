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
struct vmc_choice { u64 value; u32 stencil, first_guard; u16 nguards; };
struct vmc_guard { u8 position; u64 value; };
struct vmc_variant { u8 nzcv_from, live_after, nzcv_out, feature, position; u32 first; u16 count; };
struct vmc_form {
    u64 hash;
    u32 key;
    u16 length;
    u8 ends, falls, setter, nzcv_ok, has_flags;
    u8 branch, target;                                   /* VMC_BRANCH_*; the constant target's value, or VMC_TARGET_* */
    u8 read[6], written[6];
    u32 first;
    u16 count;
};

enum { T_SMALL, T_POOL, T_HOLE, T_SYMBOL, T_EXIT, T_CHECKED };
struct vmc_checked { u8 expression, position; u16 first, count; };
enum { K_ENC, K_ENC32, K_BRANCH };
/* scripts/gen_stencil_table.py branch_facts */
enum { VMC_BRANCH_NONE, VMC_BRANCH_ALWAYS, VMC_BRANCH_OTHER };
enum { VMC_TARGET_NOT_A_VALUE = 254, VMC_TARGET_NONE = 255 };

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

static const struct vmc_form *lookup(const u8 *key, int n) {
    u64 h = fnv1a(key, n);
    int lo = 0, hi = VMC_FORMS;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;
        if (VMC_FORM[mid].hash < h)
            lo = mid + 1;
        else
            hi = mid;
    }
    if (lo == VMC_FORMS || VMC_FORM[lo].hash != h || VMC_FORM[lo].length != n)
        return 0;
    const u8 *k = &VMC_FORM_BYTES[VMC_FORM[lo].key];
    for (int i = 0; i < n; i++)
        if (k[i] != key[i])
            return 0;
    return &VMC_FORM[lo];
}

/* the instruction at code[0..n) and its form, or a vmc_status */
int vmc_decode(const u8 *code, int n, u64 address, struct vmc_insn *out) {
    static const struct vmc_flags none = {0, 0, 0};
    u8 key[VMC_MAX_KEY];
    int kn = vmc_key(code, n, address, &none, key, out->values, &out->nvalues, &out->length);
    if (kn < 0)
        return kn;
    const struct vmc_form *f = lookup(key, kn - 3);       /* the key less its flag arguments */
    if (!f)
        return VMC_NO_STENCIL;
    out->form = f;
    out->address = address;
    return 0;
}

int vmc_emit(const struct vmc_insn *ins, const struct vmc_flags *flags, u64 base, u64 exit_va,
             const u64 *symbols, u32 *out, int max_words) {
    const struct vmc_form *f = ins->form;
    /* the flag arguments as the instruction reads them (stencil_census.canonical_flags) */
    int nzcv_from = f->nzcv_ok >> flags->nzcv_from & 1 ? flags->nzcv_from : 0;
    int live_after = f->has_flags ? flags->live_after : 0;
    int nzcv_out = f->has_flags ? flags->nzcv_out : 0;
    const struct vmc_variant *v = 0;
    for (int i = 0; i < f->count; i++) {
        const struct vmc_variant *c = &VMC_VARIANT[f->first + i];
        if (c->nzcv_from == nzcv_from && c->live_after == live_after && c->nzcv_out == nzcv_out) {
            v = c;                                       /* unique: the generator asserts it */
            break;
        }
    }
    if (!v)
        return VMC_NO_STENCIL;
    const u64 *values = ins->values;
    int nvalues = ins->nvalues;
    const struct vmc_choice *choice = &VMC_CHOICE[v->first];
    if (v->feature != VMC_FEATURE_NONE) {
        if (v->position >= nvalues)
            return VMC_NO_CHOICE;
        u64 fv = vmc_feature(v->feature, values[v->position]);
        choice = 0;
        for (int i = 0; i < v->count; i++)
            if (VMC_CHOICE[v->first + i].value == fv)
                choice = &VMC_CHOICE[v->first + i];
        if (!choice)
            return VMC_NO_CHOICE;
    }
    for (int i = 0; i < choice->nguards; i++) {           /* a value the derivation never varied */
        const struct vmc_guard *g = &VMC_GUARD[choice->first_guard + i];
        if (g->position >= nvalues || values[g->position] != g->value)
            return VMC_GUARDED;
    }
    const struct vmc_stencil *s = &VMC_STENCIL[choice->stencil];
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
            case T_CHECKED: {                            /* every expression the samples allow must agree */
                const struct vmc_checked *h = &VMC_CHECKED[payload];
                if (h->position >= nvalues)
                    return VMC_ENCODER;
                u64 x = values[h->position];
                a[j] = vmc_expression(h->expression, x);
                for (int k = 0; k < h->count; k++)
                    if (vmc_expression(VMC_ALT[h->first + k], x) != a[j])
                        return VMC_GUARDED;
                break;
            }
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

int vmc_compile(const u8 *code, int n, u64 address, u64 base, u64 exit_va, const struct vmc_flags *flags,
                const u64 *symbols, u32 *out, int max_words) {
    struct vmc_insn ins;
    int r = vmc_decode(code, n, address, &ins);
    if (r < 0)
        return r;
    return vmc_emit(&ins, flags, base, exit_va, symbols, out, max_words);
}

/* `address` among the sorted thunks */
static int in_thunks(const u64 *thunks, int nthunks, u64 address) {
    int lo = 0, hi = nthunks;
    while (lo < hi) {
        int mid = lo + (hi - lo) / 2;
        if (thunks[mid] < address)
            lo = mid + 1;
        else
            hi = mid;
    }
    return lo < nthunks && thunks[lo] == address;
}

/* the successor scan (Sprint 082): VM._flags_live_in, line for line */
int vmc_live_in(const struct vmc_memory *m, u64 addr) {
    int live = 0, undecided = VMC_ALL_FLAGS;
    const struct vmc_form *previous = 0;
    for (int i = 0; i < VMC_LIVENESS_SCAN; i++) {
        if (in_thunks(m->thunks, m->nthunks, addr) || addr >= VMC_PROCESS_VA)
            break;
        const u8 *bytes = 0;
        int n = m->code(m->ctx, addr, &bytes);
        if (n <= 0)
            break;
        struct vmc_insn ins;
        /* n stops where the code pages do: an instruction past them does not decode (the Python's pages check);
           one with no form ends the scan with its flags undecided, more live than the Python finds, never fewer */
        if (vmc_decode(bytes, n > 15 ? 15 : n, addr, &ins) < 0)
            break;
        u64 first = addr & ~(VMC_PAGE - 1), last = (addr + (u64)ins.length - 1) & ~(VMC_PAGE - 1);
        m->scanned(m->ctx, first);
        if (last != first)
            m->scanned(m->ctx, last);
        const struct vmc_form *f = ins.form;
        int kind = previous ? previous->setter : 0;
        live |= f->read[kind] & undecided;
        undecided &= ~(f->read[kind] | f->written[kind]);
        if (!undecided)
            return live;
        if (f->branch == VMC_BRANCH_NONE) {
            addr += (u64)ins.length;
            previous = f;
            continue;
        }
        if (f->branch != VMC_BRANCH_ALWAYS || f->target >= ins.nvalues)
            break;
        addr = ins.values[f->target];
        previous = 0;
    }
    return live | undecided;
}

/* VM._flags_live_out: the flags read after a block whose last instruction is `last`, ending at `end` */
int vmc_live_out(const struct vmc_memory *m, const struct vmc_insn *last, u64 end) {
    const struct vmc_form *f = last->form;
    if (f->branch == VMC_BRANCH_NONE)
        return vmc_live_in(m, end);
    if (f->target >= last->nvalues)                      /* not a constant, or one the C cannot follow: all */
        return VMC_ALL_FLAGS;
    int live = vmc_live_in(m, last->values[f->target]);
    return f->branch == VMC_BRANCH_ALWAYS ? live : live | vmc_live_in(m, end);
}

/* the block: Monitor._compile's extent and vm_backend.assemble_block's code */
static int is_thunk(const struct vmc_block_in *in, u64 address) {
    return in_thunks(in->thunks, in->nthunks, address);
}

static u32 subs_budget(int n) {                         /* SUBS x23, x23, #n */
    return 0xF1000000u | (u32)n << 10 | VMC_REG_BUDGET << 5 | VMC_REG_BUDGET;
}

int vmc_block(const struct vmc_block_in *in, struct vmc_block_out *out) {
    struct vmc_insn insns[VMC_MAX_BLOCK_INSNS];
    int count = 0, stopped_short = 0;
    u64 addr = in->start;
    out->cut = 0;
    while (count < VMC_MAX_BLOCK_INSNS) {
        if (count && is_thunk(in, addr))
            break;
        if (addr < in->mem_base || addr - in->mem_base >= in->mem_size) {
            if (!count)
                return VMC_DECODE;
            stopped_short = 1;
            break;
        }
        u64 off = addr - in->mem_base;
        u64 avail = in->mem_size - off;
        int r = vmc_decode(in->mem + off, avail > 15 ? 15 : (int)avail, addr, &insns[count]);
        if (r < 0) {
            if (!count)
                return r;
            out->cut = r;
            stopped_short = 1;
            break;
        }
        addr += insns[count].length;
        count++;
        if (insns[count - 1].form->ends)
            break;
    }
    int natural = stopped_short ? -1 : count;           /* live_out holds where a branch, a thunk or the count
                                                           ends the block; elsewhere every flag is live */
    for (;;) {                                          /* an instruction with no stencil for its flags ends the block */
        u64 end = in->start;
        for (int k = 0; k < count; k++)
            end += insns[k].length;
        int live = count == natural ? in->live_out : VMC_ALL_FLAGS;
        u8 lives[VMC_MAX_BLOCK_INSNS];
        for (int k = count - 1; k >= 0; k--) {
            lives[k] = (u8)live;
            int kind = k ? insns[k - 1].form->setter : 0;
            live = (live & ~insns[k].form->written[kind]) | insns[k].form->read[kind];
        }
        u32 *w = out->words;
        int n = 0, max = out->max_words;
        u64 base = in->base;
        if (max < 2)
            return VMC_FULL;
        out->marks[0].host = base;
        out->marks[0].guest = in->start;
        w[n++] = subs_budget(count);
        int bmi = n++;                                  /* B.MI to the budget's stop, filled at the end */
        int body = n;
        int failed = -1, status = 0;
        for (int k = 0; k < count; k++) {
            struct vmc_flags fl = {
                k ? insns[k - 1].form->setter : 0, lives[k],
                k + 1 < count && insns[k].form->setter && (insns[k + 1].form->nzcv_ok >> insns[k].form->setter & 1)};
            out->marks[1 + k].host = base + 4 * (u64)n;
            out->marks[1 + k].guest = insns[k].address;
            int got = vmc_emit(&insns[k], &fl, base + 4 * (u64)n, in->exit_va, in->symbols, w + n, max - n);
            if (got < 0) {
                failed = k;
                status = got;
                break;
            }
            n += got;
        }
        if (failed >= 0) {
            if (status == VMC_FULL)
                return VMC_FULL;
            if (failed == 0)
                return status;
            count = failed;
            out->cut = status;
            continue;
        }
        if (insns[count - 1].form->falls) {             /* the fall-through exit: x17 = the next address */
            u32 mov[4];
            int m = a64_mov_imm(mov, 17, end);
            if (n + m + 1 > max)
                return VMC_FULL;
            for (int j = 0; j < m; j++)
                w[n++] = mov[j];
            w[n] = a64_b((int)(in->exit_va - (base + 4 * (u64)n)));
            n++;
        }
        if (n + 3 > max)
            return VMC_FULL;
        int stop = n;                                   /* the budget's stop */
        out->marks[1 + count].host = base + 4 * (u64)stop;
        out->marks[1 + count].guest = in->start;
        w[bmi] = a64_bcond(A64_MI, 4 * (stop - bmi));
        w[n] = a64_bl((int)(in->slice_va - (base + 4 * (u64)n)));
        n++;
        w[n++] = a64_hvc(4);
        w[n] = a64_b(4 * (body - n));
        n++;
        out->count = count;
        out->end = end;
        out->nwords = n;
        out->nmarks = count + 2;
        return n;
    }
}
