// SPDX-License-Identifier: MPL-2.0
#include <stdlib.h>
#include <stdint.h>
#include <windows.h>

typedef struct { int64_t number; double fraction; int32_t marker; } Packet;
static unsigned initialized;

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved) {
    (void)instance;
    (void)reserved;
    if (reason == DLL_PROCESS_ATTACH) initialized = 1;
    return TRUE;
}

__declspec(dllexport) Packet peer_packet(Packet value) {
    value.number += 17;
    value.fraction *= 2;
    value.marker ^= 0x55;
    if (!initialized) value.marker = -1;
    return value;
}

__declspec(dllexport) unsigned char *peer_allocate(uint64_t size) {
    unsigned char *data = malloc((size_t)size);
    if (data) {
        for (uint64_t i = 0; i < size; ++i) data[i] = (unsigned char)(i % 251);
    }
    return data;
}

__declspec(dllexport) void peer_release(void *data) {
    free(data);
}
