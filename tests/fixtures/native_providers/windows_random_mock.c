// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <stdlib.h>
static int calls, mode;
void reset(int m) { calls=0; mode=m; }
int get_calls(void) { return calls; }
int32_t BCryptGenRandom(void *algorithm, void *buffer, uint32_t size, uint32_t flags) {
    if (algorithm || !buffer || flags!=2) abort();
    calls++;
    if (mode==1) return (int32_t)0xc000000d;
    if (mode==2 || mode==3) {
        if (calls==1) { if (size!=UINT32_MAX || (uintptr_t)buffer!=1) abort(); return 0; }
        if (calls!=2 || size!=9 || (uintptr_t)buffer!=UINT64_C(4294967296)) abort();
        return mode==2 ? (int32_t)0xc0000001 : 0;
    }
    return 0;
}
