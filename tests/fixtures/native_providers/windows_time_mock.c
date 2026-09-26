// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
static uint64_t ticks;
void set_ticks(uint64_t value) { ticks=value; }
void GetSystemTimeAsFileTime(uint32_t *p) { p[0]=(uint32_t)ticks; p[1]=(uint32_t)(ticks>>32); }
void Sleep(uint32_t ms) { (void)ms; }
int QueryPerformanceCounter(int64_t *p) { *p=0; return 1; }
int QueryPerformanceFrequency(int64_t *p) { *p=1; return 1; }
