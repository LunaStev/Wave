// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <string.h>
static int scenario, acquired, released, converted, failed;
static uint16_t block[] = {'=', 'C', ':', '=', 'x', 0, 0xd55c, '=', 0xd83d, 0xde00, 0, 'E', '=', 0, 0};
static const unsigned char bytes[] = {'=', 'C', ':', '=', 'x', 0, 0xed, 0x95, 0x9c, '=', 0xf0, 0x9f, 0x98, 0x80, 0, 'E', '=', 0, 0};
void choose(int n) { scenario = n; acquired = released = converted = 0; }
int failures(void) { return failed; }
int acquisitions(void) { return acquired; }
int releases(void) { return released; }
int conversions(void) { return converted; }
uint16_t *GetEnvironmentStringsW(void) { acquired++; return scenario == 1 ? 0 : block; }
int FreeEnvironmentStringsW(uint16_t *p) { if (p != block) failed++; released++; return scenario != 5; }
uint32_t GetLastError(void) { return 5; }
int WideCharToMultiByte(uint32_t cp, uint32_t flags, const uint16_t *src, int units,
                       unsigned char *dst, int cap, const char *def, int *used) {
    if (cp != 65001 || flags != 128 || src != block || units != 15 || def || used) failed++;
    if (!dst) { if (cap) failed++; return scenario == 2 ? 0 : sizeof bytes; }
    converted++;
    if (cap != sizeof bytes) failed++;
    if (scenario == 3) return 0;
    memcpy(dst, bytes, sizeof bytes); return sizeof bytes;
}
