// SPDX-License-Identifier: MPL-2.0
#include <windows.h>
#include <stdint.h>
#include <stdio.h>

extern int stack_outer(void);
extern int stack_middle(void);
extern int stack_inner(void);

__declspec(noinline) int stack_bridge(int level, uint64_t *data, int64_t count) {
    if ((uintptr_t)data % _Alignof(uint64_t)) {
        return 20;
    }
    for (int64_t i = 0; i < count; ++i) {
        if (data[i] != (uint64_t)i * 17 + 3) {
            return 21;
        }
    }
    if (level == 2) {
        return stack_middle();
    }
    if (level == 1) {
        return stack_inner();
    }
    void *frames[64];
    void *functions[] = { (void *)stack_inner, (void *)stack_middle, (void *)stack_outer };
    USHORT count_frames = CaptureStackBackTrace(0, 64, frames, NULL);
    unsigned found = 0;
    for (unsigned function = 0; function < 3; ++function) {
        DWORD64 expected_base = 0;
        PRUNTIME_FUNCTION expected = RtlLookupFunctionEntry((DWORD64)functions[function], &expected_base, NULL);
        if (!expected) {
            fprintf(stderr, "missing unwind entry for Wave frame %u\n", function);
            return 22;
        }
        for (USHORT i = 0; i < count_frames; ++i) {
            DWORD64 actual_base = 0;
            PRUNTIME_FUNCTION actual = RtlLookupFunctionEntry((DWORD64)frames[i], &actual_base, NULL);
            if (actual && actual_base == expected_base && actual->BeginAddress == expected->BeginAddress) {
                found |= 1u << function;
                break;
            }
        }
    }
    if (found != 7) {
        fprintf(stderr, "native stack walk recovered Wave frame mask %u, expected 7\n", found);
        return 23;
    }
    return 0;
}
