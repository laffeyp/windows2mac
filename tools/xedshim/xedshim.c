/* xedshim.c — batch front end to Intel XED (third_party/xed, v2026.08.23) for the decoder
 * differential. Reads one hex byte string per line on stdin (up to 15 bytes); writes one
 * tab-separated line per input:
 *   ok <length> <iclass> <iform> <intel-syntax text>
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
        printf("ok\t%u\t%s\t%s\t%s\n", xed_decoded_inst_get_length(&xedd),
               xed_iclass_enum_t2str(xed_decoded_inst_get_iclass(&xedd)),
               xed_iform_enum_t2str(xed_decoded_inst_get_iform_enum(&xedd)), text);
    }
    return 0;
}
