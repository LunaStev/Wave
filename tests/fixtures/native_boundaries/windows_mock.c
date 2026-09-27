// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <stddef.h>
#include <string.h>
static int scenario, failed, resized, closed;
static uint32_t protection, allocations;
void choose(int n) { scenario = n; resized = closed = 0; }
int failures(void) { return failed; }
int truncations(void) { return resized; }
int closed_handles(void) { return closed; }
uint32_t last_protection(void) { return protection; }
uint32_t allocation_count(void) { return allocations; }
void *CreateFileW(const uint16_t *name, uint32_t access, uint32_t share,
                 void *security, uint32_t creation, uint32_t attrs, void *template) {
    (void)name; (void)security; (void)attrs; (void)template;
    if (share != 7) failed++;
    if (scenario == 0 && (access != 0x40000000 || creation != 5)) failed++;
    if (scenario == 1 && ((access & (0x40000000 | 2)) || !(access & 4) || creation != 3)) failed++;
    if (scenario >= 2 && scenario <= 4 && (!(access & 0x40000000) || creation != 3)) failed++;
    return (void *)(uintptr_t)4096;
}
void *GetCurrentProcess(void) { return (void *)(intptr_t)-1; }
int DuplicateHandle(void *p, void *h, void *p2, void **out, uint32_t access, int inherit, uint32_t flags) {
    (void)p; (void)h; (void)p2; (void)inherit;
    if (flags || (access & (0x40000000 | 2)) || !(access & 4) || resized) failed++;
    if (scenario == 3) return 0;
    *out = (void *)(uintptr_t)8192;
    return 1;
}
int SetFileInformationByHandle(void *h, int kind, const int64_t *end, uint32_t size) {
    if (h != (void *)(uintptr_t)4096 || kind != 6 || size != 8 || *end) failed++;
    resized++;
    return scenario != 4;
}
int CloseHandle(void *h) { (void)h; closed++; return 1; }
uint32_t GetLastError(void) { return scenario == 12 ? 6 : 5; }
uint32_t GetFileType(void *h) { (void)h; return scenario == 11 ? 3 : scenario == 12 ? 0 : 1; }
struct info { uint32_t attr, cl, ch, al, ah, wl, wh, volume, sh, sl, links, ih, il; };
_Static_assert(sizeof(struct info) == 52, "Windows metadata ABI");
int GetFileInformationByHandle(void *h, struct info *info) {
    (void)h;
    memset(info, 0, sizeof(*info));
    if (scenario == 13) return 0;
    uint64_t ticks = UINT64_C(133444736000000000); // Unix 1700000000 seconds.
    info->attr = scenario == 10 ? 16 : 128;
    info->sh = 2; info->sl = 17;
    info->wl = (uint32_t)ticks; info->wh = (uint32_t)(ticks >> 32);
    return 1;
}
void *VirtualAlloc(void *addr, uint64_t size, uint32_t flags, uint32_t prot) {
    (void)addr;
    if (!size || flags != 12288) failed++;
    allocations++; protection = prot;
    return (void *)(uintptr_t)16384;
}
int VirtualFree(void *addr, uint64_t size, uint32_t flags) {
    if (addr != (void *)(uintptr_t)16384 || size || flags != 32768) failed++;
    return 1;
}
// Unused exported provider functions still have to link at O0.
void *GetStdHandle(uint32_t n) { (void)n; return 0; }
int closesocket(int64_t fd) { (void)fd; return 0; }
int ReadFile(void *h, void *b, uint32_t n, uint32_t *r, void *o) { return 0; }
int WriteFile(void *h, void *b, uint32_t n, uint32_t *r, void *o) { return 0; }
int FlushFileBuffers(void *h) { return 0; }
int SetFilePointerEx(void *h, int64_t d, int64_t *p, uint32_t m) { failed++; return 0; }
uint32_t GetFileAttributesW(const uint16_t *p) { return 0; }
int GetFileAttributesExW(const uint16_t *p, int k, void *out) { return 0; }
uint32_t GetCurrentDirectoryW(uint32_t n, uint16_t *b) { return 0; }
int SetCurrentDirectoryW(const uint16_t *p) { return 0; }
int DeleteFileW(const uint16_t *p) { return 0; }
int CreateDirectoryW(const uint16_t *p, void *s) { return 0; }
int RemoveDirectoryW(const uint16_t *p) { return 0; }
int MoveFileExW(const uint16_t *a, const uint16_t *b, uint32_t f) { return 0; }
int GetFileInformationByHandleEx(void *h, int k, void *out, uint32_t n) { return 0; }
void GetSystemInfo(void *out) { memset(out, 0, 48); }
