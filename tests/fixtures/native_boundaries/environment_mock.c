// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <string.h>
static int mode, reads, allocated, freed;
static unsigned char pool[131072];
void choose(int n) { mode = n; reads = allocated = freed = 0; }
int live_allocations(void) { return allocated - freed; }
void *mock_alloc(int64_t size) {
    if (mode == 3 || size > (int64_t)sizeof(pool)) return 0;
    allocated++;
    return pool;
}
int64_t mock_free(void *p, int64_t size) {
    (void)size;
    if (p == pool) freed++;
    return 0;
}
int64_t mock_env_read(unsigned char *buf, int64_t cap) {
    reads++;
    if (mode == 0) return -5;
    if (mode == 1) { memcpy(buf, "KEY=x", 5); return 5; }
    if (mode == 2) return cap + 1;
    if (mode == 3) return -28;
    if (mode == 4) return reads == 1 ? -28 : -5;
    if (mode == 5 && reads < 3) return -28;
    if (mode == 6) return 0;
    memcpy(buf, "KEY=value", 10);
    return 10;
}
static int offset, interrupted, closed;
void select_read(int n) { mode = n; offset = interrupted = closed = 0; }
int close_count(void) { return closed; }
int64_t mock_open(const char *p, int flags, int perms) {
    (void)p; (void)flags; (void)perms; return 10;
}
int64_t mock_close(int64_t fd) { (void)fd; closed++; return 0; }
int64_t mock_read(int64_t fd, unsigned char *dst, int64_t cap) {
    (void)fd;
    if (!interrupted++) return -4;
    if (mode == 2 && offset == 2) return -5;
    int length = mode == 1 ? 5 : 4;
    if (offset == length) return 0;
    if (cap > 2) cap = 2;
    if (cap > length - offset) cap = length - offset;
    memcpy(dst, "K=x\0y" + offset, (size_t)cap);
    offset += (int)cap;
    return cap;
}
