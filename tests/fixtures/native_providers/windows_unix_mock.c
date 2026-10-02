// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <assert.h>
#include <string.h>
static int failure, closed, sockets;
void prepare_failure(int stage) { failure = stage; closed = sockets = 0; }
int closed_count(void) { return closed; }
int socket_count(void) { return sockets; }
int MultiByteToWideChar(uint32_t page, uint32_t flags, const char *text, int bytes, void *out, int cap) {
    assert(page == 65001 && flags == 8 && bytes == 9 && !out && cap == 0);
    assert(memcmp(text, "unit.sock", 9) == 0);
    return bytes;
}
int64_t socket(int domain, int type, int protocol) {
    assert(domain == 1 && type == 1 && protocol == 0);
    sockets++;
    return failure == 1 ? -10047 : 71;
}
struct address { uint16_t family; char path[108]; };
static void check_address(int64_t fd, const struct address *a, int length) {
    assert(fd == 71 && length == 12 && a->family == 1);
    assert(strcmp(a->path, "unit.sock") == 0 && a->path[107] == 0);
}
int64_t bind(int64_t fd, const struct address *a, int length) {
    check_address(fd, a, length); return failure == 2 ? -10048 : 0;
}
int64_t listen(int64_t fd, int backlog) {
    assert(fd == 71 && backlog == 1); return failure == 3 ? -10022 : 0;
}
int64_t connect(int64_t fd, const struct address *a, int length) {
    check_address(fd, a, length); return failure == 4 ? -10061 : 0;
}
int64_t net_close(int64_t fd) {
    assert(fd == 71); closed++;
    // Even a cleanup error must not replace the original operation's error.
    return -10038;
}
