// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <stdlib.h>
static int calls, mode;
void reset(int m) { calls=0; mode=m; }
int get_calls(void) { return calls; }
int64_t syscall3(int64_t id, int64_t buffer, int64_t size, int64_t flags) {
    if (id!=563 || flags || buffer<1) abort();
    calls++;
    if (calls==1) { if (buffer!=1 || size!=8) abort(); return -4; }
    if (calls==2) { if (buffer!=1 || size!=8) abort(); return 3; }
    if (calls==3) { if (buffer!=4 || size!=5) abort(); return mode ? -14 : 5; }
    abort();
}
