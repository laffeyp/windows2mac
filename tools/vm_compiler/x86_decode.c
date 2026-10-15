// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* x86_decode.c — x86dec._decode and x86ops.operands in C (Sprint 075), step for step, over the tables generated from
 * specs/x86_decode.json. decode's CPU-feature fallback is left out: lift calls decode with no feature set, where it
 * never applies. The mnemonic is not kept; its flags (lockable, a REP string, a REPE string) are. Freestanding. */
#include "x86_decode.h"
#include "x86_decode_tables.h"

#define MAX_LEN 15
enum { PFX_NONE, PFX_66, PFX_F2, PFX_F3 };
enum { MAP_1, MAP_0F, MAP_0F38, MAP_0F3A, MAP_X8, MAP_X9, MAP_XA };
enum { IMM_NONE, IMM_B, IMM_W, IMM_D, IMM_Z, IMM_V, IMM_MOFFS, IMM_WB };
enum { FORM_ANY, FORM_REG, FORM_MEM };
enum { MN_LOCKABLE = 1, MN_REP = 2, MN_REPE = 4 };
/* VEX, EVEX and XOP's pp field: none, 66, F3, F2 (x86dec.VEX_PFX) */
static const int PP[4] = {PFX_NONE, PFX_66, PFX_F3, PFX_F2};

/* a byte of the input; past its end the Python's IndexError, which decode reports as Truncated */
#define BYTE(k) ((k) < n ? b[k] : (status = X86_TRUNCATED, 0))
#define CHECK() do { if (status) return status; } while (0)

static int is_legacy_prefix(u8 x) {
    return x == 0xF0 || x == 0xF2 || x == 0xF3 || x == 0x2E || x == 0x36 || x == 0x3E || x == 0x26 || x == 0x64 ||
           x == 0x65 || x == 0x66 || x == 0x67;
}

static int has_prefix(const u8 *pfx, int np, u8 x) {
    for (int k = 0; k < np; k++)
        if (pfx[k] == x) return 1;
    return 0;
}

/* a row fits a ModRM byte by its reg field, its exact byte and its form */
static int fits_modrm(const struct x86_row *r, int m) {
    return (r->reg < 0 || r->reg == (m >> 3 & 7)) && (r->modrm_byte < 0 || r->modrm_byte == m) &&
           (r->form == FORM_ANY || (r->form == FORM_REG) == (m >> 6 == 3));
}

int x86_decode(const u8 *b, int n, struct x86_decoded *out) {
    int status = 0;
    int i = 0, np = 0;
    u8 pfx[MAX_LEN];
    u8 rex = 0;
    /* prefixes; a legacy prefix after a REX cancels it (SDM Vol 2A 2.2.1) */
    for (;;) {
        if (i >= MAX_LEN) break;
        u8 x = BYTE(i); CHECK();
        if (!(is_legacy_prefix(x) || (0x40 <= x && x <= 0x4F))) break;
        if (0x40 <= x && x <= 0x4F) rex = x;
        else { pfx[np++] = x; rex = 0; }
        i++;
    }
    if (i >= MAX_LEN) return X86_INVALID_ENCODING;
    u8 v = BYTE(i); CHECK();
    int xop_like = 0;
    if (v == 0x8F) { u8 nx = BYTE(i + 1); CHECK(); xop_like = (nx & 0x1F) >= 8; }
    if (v == 0x62 || v == 0xC4 || v == 0xC5 || xop_like) {
        for (int k = 0; k < np; k++)
            if (pfx[k] == 0x66 || pfx[k] == 0xF2 || pfx[k] == 0xF3 || pfx[k] == 0xF0) return X86_INVALID_ENCODING;
        if (rex) return X86_INVALID_ENCODING;
    }
    int enc = X86_LEGACY;
    int w = rex >> 3 & 1;
    u8 vex_rex = 0;
    int vl = -1, vvvv = -1;
    int mp;
    int mpfx[3], nmpfx = 0;
    if (v == 0x62 && !rex) {
        u8 p0 = BYTE(i + 1), p1 = BYTE(i + 2), p2 = BYTE(i + 3); CHECK();
        if ((p0 & 7) < 1 || (p0 & 7) > 3) return X86_DECODE_ERROR;
        enc = X86_EVEX; mp = MAP_0F + (p0 & 7) - 1; w = p1 >> 7; vl = p2 >> 5 & 3;
        mpfx[nmpfx++] = PP[p1 & 3];
        i += 4;
    } else if (v == 0x8F && xop_like && !rex) {
        u8 p1 = BYTE(i + 1), p2 = BYTE(i + 2); CHECK();
        if ((p1 & 0x1F) < 8 || (p1 & 0x1F) > 10) return X86_DECODE_ERROR;
        enc = X86_XOP; mp = MAP_X8 + (p1 & 0x1F) - 8; w = p2 >> 7; vl = p2 >> 2 & 1; vvvv = p2 >> 3 & 0xF;
        mpfx[nmpfx++] = PP[p2 & 3];
        i += 3;
    } else if ((v == 0xC4 || v == 0xC5) && !rex) {
        enc = X86_VEX;
        if (v == 0xC5) {
            u8 p = BYTE(i + 1); CHECK();
            mp = MAP_0F; vl = p >> 2 & 1; vvvv = p >> 3 & 0xF;
            vex_rex = (u8)(0x40 | (~p >> 7 & 1) << 2);
            mpfx[nmpfx++] = PP[p & 3];
            i += 2;
        } else {
            u8 p1 = BYTE(i + 1), p2 = BYTE(i + 2); CHECK();
            if ((p1 & 0x1F) < 1 || (p1 & 0x1F) > 3) return X86_DECODE_ERROR;
            mp = MAP_0F + (p1 & 0x1F) - 1; w = p2 >> 7; vl = p2 >> 2 & 1; vvvv = p2 >> 3 & 0xF;
            vex_rex = (u8)(0x40 | w << 3 | (~p1 >> 5 & 7));
            mpfx[nmpfx++] = PP[p2 & 3];
            i += 3;
        }
    } else {
        if (v == 0x0F) {
            u8 x = BYTE(i + 1); CHECK();
            if (x == 0x38 || x == 0x3A) { mp = x == 0x38 ? MAP_0F38 : MAP_0F3A; i += 2; }
            else { mp = MAP_0F; i += 1; }
        } else {
            mp = MAP_1;
        }
        int rep = -1;                                    /* the last F2 or F3 */
        for (int k = 0; k < np; k++)
            if (pfx[k] == 0xF2 || pfx[k] == 0xF3) rep = pfx[k];
        if (rep >= 0) mpfx[nmpfx++] = rep == 0xF2 ? PFX_F2 : PFX_F3;
        if (has_prefix(pfx, np, 0x66)) mpfx[nmpfx++] = PFX_66;
        mpfx[nmpfx++] = PFX_NONE;
    }
    u8 op = BYTE(i); CHECK();
    i++;
    int start = X86_INDEX_START[mp][enc][op], count = X86_INDEX_COUNT[mp][enc][op];
    if (!count) return X86_DECODE_ERROR;
    const struct x86_row *cands = &X86_ROW[start];
    int sse_family = 0;
    if (mp != MAP_1)
        for (int k = 0; k < count; k++)
            if (cands[k].pfx != PFX_NONE) sse_family = 1;
    int chosen_pfx = -1;                                 /* the rows chosen: cands with this prefix */
    int legacy_opcode = -1;
    for (unsigned k = 0; k < sizeof X86_LEGACY_PREFIX_OPCODES / sizeof X86_LEGACY_PREFIX_OPCODES[0]; k++)
        if (X86_LEGACY_PREFIX_OPCODES[k][0] == mp && X86_LEGACY_PREFIX_OPCODES[k][1] == op) legacy_opcode = (int)k;
    if (enc == X86_LEGACY && sse_family && legacy_opcode >= 0) {
        /* a general-purpose opcode with F2/F3 rows: a prefix with no row keeps its legacy meaning only where the
         * spec says so; walk the prefixes in precedence order */
        u8 legacy = X86_LEGACY_PREFIX_OPCODES[legacy_opcode][2];
        for (int q = 0; q < nmpfx; q++) {
            int prefix = mpfx[q], rows = 0, any_modrm = 0;
            for (int k = 0; k < count; k++)
                if (cands[k].pfx == prefix) { rows++; any_modrm |= cands[k].has_modrm; }
            if (rows && any_modrm && i < n) {
                int m = b[i];
                rows = 0;
                for (int k = 0; k < count; k++)
                    if (cands[k].pfx == prefix && fits_modrm(&cands[k], m)) rows++;
            }
            if (rows) { chosen_pfx = prefix; break; }
            if (prefix != PFX_NONE && !(legacy >> prefix & 1)) break;
        }
    } else if (enc == X86_LEGACY && sse_family) {
        chosen_pfx = mpfx[0];                            /* no fallback to a lower prefix's row */
    } else {
        for (int q = 0; q < nmpfx && chosen_pfx < 0; q++)
            for (int k = 0; k < count; k++)
                if (cands[k].pfx == mpfx[q]) { chosen_pfx = mpfx[q]; break; }
    }
    /* the chosen rows, filtered in the Python's order; `keep` marks them */
    u8 keep[256];
    int any = 0, any_modrm = 0;
    for (int k = 0; k < count; k++) {
        keep[k] = chosen_pfx >= 0 && cands[k].pfx == chosen_pfx;
        any |= keep[k];
        if (keep[k]) any_modrm |= cands[k].has_modrm;
    }
    if (!any) return X86_DECODE_ERROR;
    int modrm = -1;
    if (any_modrm) {
        modrm = BYTE(i); CHECK();
        int exact = 0;
        for (int k = 0; k < count; k++) {
            if (keep[k] && !fits_modrm(&cands[k], modrm)) keep[k] = 0;
            if (keep[k] && cands[k].modrm_byte >= 0) exact = 1;
        }
        if (exact)
            for (int k = 0; k < count; k++)
                if (cands[k].modrm_byte < 0) keep[k] = 0;
    }
    const struct x86_row *r = 0;
    for (int k = 0; k < count && !r; k++)
        if (keep[k] && (cands[k].w < 0 || cands[k].w == w) &&
            (cands[k].l < 0 || ((enc == X86_VEX || enc == X86_XOP) && cands[k].l == vl)))
            r = &cands[k];
    if (!r) return X86_DECODE_ERROR;
    if (r->np && enc == X86_LEGACY &&
        (has_prefix(pfx, np, 0xF2) || has_prefix(pfx, np, 0xF3) || (r->np == 1 && has_prefix(pfx, np, 0x66))))
        return X86_INVALID_ENCODING;
    if (r->reg_rule && modrm >= 0 && !(r->reg_valid >> ((rex >> 2 & 1) << 3 | (modrm >> 3 & 7)) & 1))
        return X86_INVALID_ENCODING;
    if (has_prefix(pfx, np, 0xF0) && !(r->lock_ok && modrm >= 0 && modrm >> 6 != 3)) return X86_INVALID_ENCODING;
    if ((enc == X86_VEX || enc == X86_XOP) && r->vvvv_ones && vvvv != 0xF) return X86_INVALID_ENCODING;
    int sib = -1, disp_size = 0;
    if (r->has_modrm) {
        i++;
        int mod = modrm >> 6, rm = modrm & 7;
        if (mod != 3) {
            if (rm == 4) {
                sib = BYTE(i); CHECK();
                i++;
                if ((sib & 7) == 5 && mod == 0) disp_size = 4;
            }
            if (mod == 0 && rm == 5) disp_size = 4;
            else if (mod == 1) disp_size = 1;
            else if (mod == 2) disp_size = 4;
        }
    }
    s64 disp = 0;
    if (disp_size) {                                     /* the bytes present (the Python's slice); short is Truncated below */
        u64 u = 0;
        int got = 0;
        for (int k = 0; k < disp_size && i + k < n; k++, got++) u |= (u64)b[i + k] << 8 * k;
        if (got) disp = got < 8 && u >> (8 * got - 1) & 1 ? (s64)(u | ~0ull << 8 * got) : (s64)u;
    }
    i += disp_size;
    int opsize = w && enc == X86_LEGACY ? 64 : has_prefix(pfx, np, 0x66) ? 16 : 32;
    int imm_size = 0;
    switch (r->imm) {
    case IMM_B: imm_size = 1; break;
    case IMM_W: imm_size = 2; break;
    case IMM_D: imm_size = 4; break;
    case IMM_Z: imm_size = opsize == 16 ? 2 : 4; break;
    case IMM_V: imm_size = opsize == 64 ? 8 : opsize == 16 ? 2 : 4; break;
    case IMM_MOFFS: imm_size = has_prefix(pfx, np, 0x67) ? 4 : 8; break;
    case IMM_WB: imm_size = 3; break;
    }
    struct x86_fields *f = &out->f;
    f->imm_size = 0;
    for (int k = 0; k < imm_size && i + k < n; k++) f->imm[f->imm_size++] = b[i + k];
    i += imm_size;
    if (i > MAX_LEN) return X86_DECODE_ERROR;
    int z = opsize == 16 ? 0 : opsize == 32 ? 1 : 2;
    int iclass = r->iclass[z], flags = r->mn_flags[z];
    if (mp == MAP_1 && op == 0xE3 && has_prefix(pfx, np, 0x67)) { iclass = X86_ICLASS_JECXZ; flags = 0; }
    if (mp == MAP_1 && op == 0x90 && (rex & 1) && !has_prefix(pfx, np, 0xF3)) {
        iclass = X86_ICLASS_XCHG; flags = X86_MN_FLAGS_XCHG;
    }
    if (has_prefix(pfx, np, 0xF0) && (flags & MN_LOCKABLE)) {
        if (iclass != X86_ICLASS_XCHG) iclass = X86_VARIANT[iclass][0];
        flags = 0;                                       /* "lock <mn>" is no REP string */
    }
    if (mp == MAP_1 && (flags & MN_REP)) {
        if (has_prefix(pfx, np, 0xF3)) iclass = X86_VARIANT[iclass][flags & MN_REPE ? 2 : 1];
        else if (has_prefix(pfx, np, 0xF2)) iclass = X86_VARIANT[iclass][flags & MN_REPE ? 3 : 1];
    }
    int eff = r->osz64 == 2 || (r->osz64 == 1 && opsize == 32) ? 64 : opsize;
    if (enc == X86_VEX) eff = vex_rex & 8 ? 64 : 32;
    if (i > n) return X86_TRUNCATED;
    out->length = (u8)i;
    out->iclass = (u16)iclass;
    out->row = (u16)(r - X86_ROW);
    for (int k = 0; k < np; k++) f->prefixes[k] = pfx[k];
    f->nprefixes = (u8)np;
    f->rex = enc == X86_VEX ? vex_rex : rex;
    f->enc = (u8)enc;
    f->opcode = op;
    f->modrm = (s16)modrm;
    f->sib = (s16)sib;
    f->disp = disp;
    f->disp_size = (u8)disp_size;
    f->opsize = (u8)eff;
    f->asize = has_prefix(pfx, np, 0x67) ? 32 : 64;
    f->ops = r->ops;
    f->vvvv = (s8)vvvv;
    f->vl = (u8)(enc == X86_VEX && vl > 0 ? vl : 0);
    return X86_OK;
}

/* ---- the operands (x86ops) ---- */

/* x86ops._reg: the class and number of general register n at `bits`, or 0 where the Python raises */
static int gpr(struct x86_operand *o, int n, int bits, int rex) {
    o->kind = X86_OP_R;
    o->reg = (u8)n;
    if (bits == 8) {
        if (!rex && n >= 8) return 0;
        o->reg_class = rex ? X86_B_REX : X86_B_LEGACY;
    } else if (bits == 64) o->reg_class = X86_Q;
    else if (bits == 32) o->reg_class = X86_D;
    else if (bits == 16) o->reg_class = X86_W;
    else return 0;
    return 1;
}

/* x86ops._bits: a size letter's bits, or 0 where SIZE has no entry */
static int letter_bits(int letter, const struct x86_fields *f) {
    switch (letter) {
    case X86_L_Z: return f->opsize == 16 ? 16 : 32;
    case X86_L_Y: return f->rex >> 3 & 1 ? 64 : 32;
    case X86_L_V: return f->opsize;
    case X86_L_B: return 8;
    case X86_L_W: return 16;
    case X86_L_D: return 32;
    case X86_L_Q: return 64;
    case X86_L_DQ: return 128;
    }
    return 0;
}

/* x86ops._mem: M with `nbytes`, or A for nbytes 0 */
static void mem(struct x86_operand *o, const struct x86_fields *f, int nbytes) {
    int mod = f->modrm >> 6, rm = f->modrm & 7;
    u8 names = f->asize == 64 ? X86_Q : X86_D;
    o->kind = nbytes ? X86_OP_M : X86_OP_A;
    o->base_class = o->index_class = X86_NONE;
    o->base = o->index = 0;
    o->scale = 1;
    if (rm == 4) {
        o->scale = (u8)(1 << (f->sib >> 6));
        int ix = (f->sib >> 3 & 7) | (f->rex >> 1 & 1) << 3;
        if (ix != 4) { o->index_class = names; o->index = (u8)ix; }
        if (!((f->sib & 7) == 5 && mod == 0)) { o->base_class = names; o->base = (u8)((f->sib & 7) | (f->rex & 1) << 3); }
    } else if (mod == 0 && rm == 5) {
        o->base_class = f->asize == 64 ? X86_RIP : X86_EIP;
    } else {
        o->base_class = names; o->base = (u8)(rm | (f->rex & 1) << 3);
    }
    o->seg = 0;
    for (int k = 0; k < f->nprefixes; k++)
        if (f->prefixes[k] == 0x64 || f->prefixes[k] == 0x65) o->seg = f->prefixes[k];
    o->disp = f->disp;
    o->bytes = (u8)nbytes;
}

static void vector(struct x86_operand *o, int cls, int n) { o->kind = X86_OP_R; o->reg_class = (u8)cls; o->reg = (u8)n; }

static u64 imm_value(const struct x86_fields *f) {
    u64 u = 0;
    for (int k = 0; k < f->imm_size; k++) u |= (u64)f->imm[k] << 8 * k;
    return u;
}

int x86_operands(const struct x86_decoded *d, struct x86_operand out[4]) {
    const struct x86_fields *f = &d->f;
    if (f->ops == X86_NO_TEMPLATE) return -1;
    const struct x86_template *t = &X86_TEMPLATE[f->ops];
    int vec = f->vl ? X86_YMM : X86_XMM;
    int modrm = f->modrm, mod = modrm >> 6, rm = modrm & 7;
    int reg_field = (modrm >> 3 & 7) | (f->rex >> 2 & 1) << 3, rm_field = rm | (f->rex & 1) << 3;
    for (int k = 0; k < t->count; k++) {
        struct x86_operand *o = &out[k];
        int arg = t->arg[k];
        switch (t->kind[k]) {
        case X86_T_MF:
            if (modrm < 0 || mod == 3) return -2;
            mem(o, f, arg);
            break;
        case X86_T_STI:
            if (modrm < 0) return -2;
            o->kind = X86_OP_ST; o->reg = (u8)rm;
            break;
        case X86_T_P:
            if (modrm < 0) return -2;
            o->kind = X86_OP_MM; o->reg = (u8)(modrm >> 3 & 7);
            break;
        case X86_T_NQ:
            if (modrm < 0) return -2;
            if (mod == 3) { o->kind = X86_OP_MM; o->reg = (u8)rm; }
            else if (!arg) return -2;
            else mem(o, f, arg);
            break;
        case X86_T_VX:
            if (modrm < 0) return -2;
            vector(o, vec, reg_field);
            break;
        case X86_T_B: {
            if (f->vvvv < 0) return -2;
            int bits = letter_bits(arg, f);
            if (!gpr(o, f->vvvv ^ 0xF, bits, 1)) return -2;
            break;
        }
        case X86_T_H: case X86_T_HX:
            if (f->vvvv < 0) return -2;
            vector(o, t->kind[k] == X86_T_HX ? vec : X86_XMM, f->vvvv ^ 0xF);
            break;
        case X86_T_WX:
            if (modrm < 0) return -2;
            if (mod == 3) vector(o, vec, rm_field);
            else mem(o, f, f->vl ? 32 : 16);
            break;
        case X86_T_V:
            if (modrm < 0) return -2;
            vector(o, X86_XMM, reg_field);
            break;
        case X86_T_EDB:
            if (modrm < 0) return -2;
            if (mod == 3) { if (!gpr(o, rm_field, 32, f->rex)) return -2; }
            else mem(o, f, 1);
            break;
        case X86_T_W:
            if (modrm < 0) return -2;
            if (mod == 3) vector(o, X86_XMM, rm_field);
            else if (!arg) return -2;
            else mem(o, f, arg);
            break;
        case X86_T_E: {
            int bits = letter_bits(arg, f);
            if (modrm < 0) return -2;
            if (mod == 3) { if (!gpr(o, rm_field, bits, f->rex)) return -2; }
            else mem(o, f, bits / 8);
            break;
        }
        case X86_T_G:
            if (modrm < 0) return -2;
            if (!gpr(o, reg_field, letter_bits(arg, f), f->rex)) return -2;
            break;
        case X86_T_Z:
            if (!gpr(o, (f->opcode & 7) | (f->rex & 1) << 3, letter_bits(arg, f), f->rex)) return -2;
            break;
        case X86_T_M: mem(o, f, 0); break;
        case X86_T_MCL: mem(o, f, 64); break;
        case X86_T_MQDQ: mem(o, f, f->opsize == 64 ? 16 : 8); break;
        case X86_T_I:
            o->kind = X86_OP_I; o->imm_len = f->imm_size; o->imm = imm_value(f);
            break;
        case X86_T_J: {
            u64 u = imm_value(f);
            int bits = 8 * f->imm_size;
            o->kind = X86_OP_J;
            o->disp = bits && bits < 64 && u >> (bits - 1) & 1 ? (s64)(u | ~0ull << bits) : (s64)u;
            break;
        }
        case X86_T_ONE: o->kind = X86_OP_I; o->imm_len = 1; o->imm = 1; break;
        case X86_T_AL: o->kind = X86_OP_R; o->reg_class = X86_B_LEGACY; o->reg = 0; break;
        case X86_T_CL: o->kind = X86_OP_R; o->reg_class = X86_B_LEGACY; o->reg = 1; break;
        case X86_T_RAX:
            if (!gpr(o, 0, f->opsize, f->rex)) return -2;
            break;
        default:
            return -2;
        }
    }
    return t->count;
}
