// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
/* xedshim.c — batch front end to Intel XED (third_party/xed, v2026.08.23) for the decoder
 * differential. Reads one hex byte string per line on stdin (up to 15 bytes); writes one
 * tab-separated line per input:
 *   ok <length> <iclass> <iform> <intel-syntax text> <operands>
 * operands: displayed (non-suppressed) operands joined by ';' —
 *   R:<reg>   M:<seg>:<base>:<index>:<scale>:<disp>:<bytes>   A:<base>:<index>:<scale>:<disp>
 *   I:<width bytes>:<raw hex>   J:<displacement>   ?:<operand name> (other kinds)
 *   err <xed error name>
 * Machine mode: 64-bit long mode, 64-bit stack address width.
 * Chip: argv[1] names an XED chip (default ALL). Decoding depends on the chip: F3 0F BC is TZCNT
 * with BMI1 and BSF without it; F3 0F 1E FA is ENDBR64 with CET and NOP without it.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "xed/xed-interface.h"

static int hexval(int c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

int main(int argc, char **argv) {
    xed_tables_init();
    xed_chip_enum_t chip = XED_CHIP_ALL;
    if (argc > 1) {
        chip = str2xed_chip_enum_t(argv[1]);
        if (chip == XED_CHIP_INVALID) {
            fprintf(stderr, "unknown XED chip: %s\n", argv[1]);
            return 2;
        }
    }
    char line[256], text[256];
    unsigned char buf[16];
    while (fgets(line, sizeof line, stdin)) {
        unsigned n = 0;
        for (char *p = line; p[0] && p[1] && n < 15; p += 2) {
            int hi = hexval(p[0]), lo = hexval(p[1]);
            if (hi < 0 || lo < 0) break;
            buf[n++] = (unsigned char)(hi << 4 | lo);
        }
        xed_decoded_inst_t xedd;
        xed_decoded_inst_zero(&xedd);
        xed_decoded_inst_set_mode(&xedd, XED_MACHINE_MODE_LONG_64, XED_ADDRESS_WIDTH_64b);
        xed_decoded_inst_set_input_chip(&xedd, chip);
        xed_error_enum_t err = xed_decode(&xedd, buf, n);
        if (err != XED_ERROR_NONE) {
            printf("err\t%s\n", xed_error_enum_t2str(err));
            continue;
        }
        if (!xed_format_context(XED_SYNTAX_INTEL, &xedd, text, sizeof text, 0, 0, 0))
            strcpy(text, "?");
        char ops[512] = "";
        const xed_inst_t *xi = xed_decoded_inst_inst(&xedd);
        unsigned memi = 0;
        for (unsigned k = 0; k < xed_inst_noperands(xi); k++) {
            const xed_operand_t *op = xed_inst_operand(xi, k);
            if (xed_operand_operand_visibility(op) == XED_OPVIS_SUPPRESSED) continue;
            xed_operand_enum_t name = xed_operand_name(op);
            char one[160];
            if (xed_operand_is_register(name)) {
                snprintf(one, sizeof one, "R:%s", xed_reg_enum_t2str(xed_decoded_inst_get_reg(&xedd, name)));
            } else if (name == XED_OPERAND_MEM0 || name == XED_OPERAND_MEM1 || name == XED_OPERAND_AGEN) {
                unsigned m = name == XED_OPERAND_MEM1 ? 1 : 0;
                if (name != XED_OPERAND_AGEN) memi = m;
                char head[16];
                snprintf(head, sizeof head, name == XED_OPERAND_AGEN ? "A" : "M:%s",
                         xed_reg_enum_t2str(xed_decoded_inst_get_seg_reg(&xedd, m)));
                snprintf(one, sizeof one, "%s:%s:%s:%u:%lld", head,
                         xed_reg_enum_t2str(xed_decoded_inst_get_base_reg(&xedd, m)),
                         xed_reg_enum_t2str(xed_decoded_inst_get_index_reg(&xedd, m)),
                         xed_decoded_inst_get_scale(&xedd, m),
                         (long long)xed_decoded_inst_get_memory_displacement(&xedd, m));
                if (name != XED_OPERAND_AGEN) {
                    size_t n = strlen(one);
                    snprintf(one + n, sizeof one - n, ":%u",
                             xed_decoded_inst_get_memory_operand_length(&xedd, memi));
                }
            } else if (name == XED_OPERAND_IMM0) {
                unsigned w = xed_decoded_inst_get_immediate_width(&xedd);
                unsigned long long v = xed_decoded_inst_get_unsigned_immediate(&xedd);
                if (xed_decoded_inst_get_immediate_is_signed(&xedd))
                    v = (unsigned long long)(long long)xed_decoded_inst_get_signed_immediate(&xedd);
                if (w < 8) v &= (1ULL << (8 * w)) - 1;
                snprintf(one, sizeof one, "I:%u:%llx", w, v);
            } else if (name == XED_OPERAND_RELBR) {
                snprintf(one, sizeof one, "J:%lld", (long long)xed_decoded_inst_get_branch_displacement(&xedd));
            } else {
                snprintf(one, sizeof one, "?:%s", xed_operand_enum_t2str(name));
            }
            if (ops[0]) strncat(ops, ";", sizeof ops - strlen(ops) - 1);
            strncat(ops, one, sizeof ops - strlen(ops) - 1);
        }
        printf("ok\t%u\t%s\t%s\t%s\t%s\n", xed_decoded_inst_get_length(&xedd),
               xed_iclass_enum_t2str(xed_decoded_inst_get_iclass(&xedd)),
               xed_iform_enum_t2str(xed_decoded_inst_get_iform_enum(&xedd)), text, ops);
    }
    return 0;
}
