// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <assert.h>
static int sleeps, polls;
static uint32_t milliseconds;
void reset_calls(void) { sleeps = polls = 0; milliseconds = 0; }
bool calls_ok(int s, uint32_t ms, int p) { return sleeps == s && milliseconds == ms && polls == p; }
void Sleep(uint32_t ms) { sleeps++; milliseconds = ms; }
void *sys_alloc(int64_t size) { return calloc(1, (size_t)size); }
int64_t sys_free(void *p, int64_t size) { (void)size; free(p); return 0; }
struct pollfd { int64_t fd; int16_t events, revents; };
int64_t poll(struct pollfd *fds, int64_t count, int32_t timeout) {
    polls++;
    assert(count == 1 && timeout == 12 && fds[0].fd == 17 && fds[0].events == 256);
    fds[0].revents = 256;
    return 1;
}
