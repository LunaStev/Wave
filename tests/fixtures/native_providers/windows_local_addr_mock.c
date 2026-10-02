// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <assert.h>
int MultiByteToWideChar(uint32_t page, uint32_t flags, const unsigned char *s,
                       int bytes, uint16_t *output, int capacity) {
    assert(page == 65001 && flags == 8 && bytes > 0 && bytes <= 107);
    assert(output == 0 && capacity == 0);
    // Boundary mock: native Windows tests validate the actual UTF-8 decoder.
    return s[0] == 192 ? 0 : bytes;
}
