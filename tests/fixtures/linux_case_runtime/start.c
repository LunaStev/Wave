// SPDX-License-Identifier: MPL-2.0
// Minimal Linux process entry and memory helpers for libc-independent language
// workloads. All assertions and algorithms live in the shared Wave cases.
typedef __SIZE_TYPE__ size_t;
void *memcpy(void *destination, const void *source, size_t count) {
    unsigned char *out = destination;
    const unsigned char *in = source;
    for (size_t i = 0; i < count; ++i) out[i] = in[i];
    return destination;
}
void *memmove(void *destination, const void *source, size_t count) {
    unsigned char *out = destination;
    const unsigned char *in = source;
    if ((__UINTPTR_TYPE__)out < (__UINTPTR_TYPE__)in) {
        for (size_t i = 0; i < count; ++i) out[i] = in[i];
    } else {
        while (count) { --count; out[count] = in[count]; }
    }
    return destination;
}
void *memset(void *destination, int value, size_t count) {
    unsigned char *out = destination;
    for (size_t i = 0; i < count; ++i) out[i] = (unsigned char)value;
    return destination;
}
#if defined(__loongarch64)
__asm__(".global _start\n_start:\nbl main\naddi.d $a7, $zero, 93\nsyscall 0\n");
#elif defined(__aarch64__)
__asm__(".global _start\n_start:\nbl main\nmov x8, #93\nsvc #0\n");
#elif defined(__riscv) && __riscv_xlen == 64
__asm__(".global _start\n_start:\ncall main\nli a7, 93\necall\n");
#else
#error Unsupported runtime test architecture
#endif
