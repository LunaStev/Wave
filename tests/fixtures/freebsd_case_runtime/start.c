// SPDX-License-Identifier: MPL-2.0
// Minimal FreeBSD process entry and memory helpers for libc-independent language
// workloads. All assertions and algorithms live in the FreeBSD Wave cases.
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
#if defined(__x86_64__)
__asm__(".global _start\n_start:\nxor %rbp,%rbp\nand $-16,%rsp\ncall main\nmov %eax,%edi\nmov $1,%eax\nsyscall\nud2\n");
#elif defined(__aarch64__)
__asm__(".global _start\n_start:\nbl main\nmov x8, #1\nsvc #0\nbrk #0\n");
#elif defined(__riscv) && __riscv_xlen == 64
__asm__(".global _start\n_start:\ncall main\nli t0, 1\necall\nunimp\n");
#else
#error Unsupported FreeBSD test architecture
#endif
// FreeBSD ABI note for static binaries without the system crt.
__asm__(".section .note.tag,\"a\",@note\n.balign 4\n.long 8,4,1\n.asciz \"FreeBSD\"\n.balign 4\n.long 1403000\n.previous\n");
