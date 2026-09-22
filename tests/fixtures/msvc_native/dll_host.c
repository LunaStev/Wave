// SPDX-License-Identifier: MPL-2.0
#include <windows.h>
#include <stdint.h>
#include <stdio.h>

typedef struct { int64_t number; double fraction; int32_t marker; } Packet;
typedef Packet (*PacketFn)(Packet);
typedef unsigned char *(*AllocateFn)(uint64_t);
typedef int (*ReleaseFn)(void *);

#ifdef IMPORT_CONSUMER
__declspec(dllimport) Packet wave_packet(Packet);
__declspec(dllimport) unsigned char *wave_allocate(uint64_t);
__declspec(dllimport) int wave_release(void *);
#endif

static int check(PacketFn packet, AllocateFn allocate, ReleaseFn release) {
    Packet value = { 25, 1.25, 0x11 };
    Packet result = packet(value);
    if (result.number != 42 || result.fraction != 2.5 || result.marker != 0x44) return 41;
    unsigned char *data = allocate(65537);
    if (!data) return 42;
    int status = 0;
    for (uint64_t i = 0; i < 65537; ++i) {
        if (data[i] != (unsigned char)(i % 251)) status = 43;
    }
    if (!release(data)) return 44;
    return status;
}

int main(void) {
#ifdef IMPORT_CONSUMER
    int status = check(wave_packet, wave_allocate, wave_release);
    if (status) return status;
#else
    if (LoadLibraryW(L"wave-deliberately-missing.dll") != NULL || GetLastError() != ERROR_MOD_NOT_FOUND) return 45;
    for (int cycle = 0; cycle < 32; ++cycle) {
        HMODULE module = LoadLibraryW(L"wave-library.dll");
        if (!module) return 46;
        if (GetProcAddress(module, "missing_export") || GetLastError() != ERROR_PROC_NOT_FOUND) return 47;
        PacketFn packet = (PacketFn)GetProcAddress(module, "wave_packet");
        AllocateFn allocate = (AllocateFn)GetProcAddress(module, "wave_allocate");
        ReleaseFn release = (ReleaseFn)GetProcAddress(module, "wave_release");
        if (!packet || !allocate || !release) return 48;
        int status = check(packet, allocate, release);
        if (status) return status;
        if (!FreeLibrary(module)) return 49;
    }
#endif
    puts("native DLL host checked");
    return 0;
}
