// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

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
extern int main(void);

__attribute__((force_align_arg_pointer, noreturn)) void _start(void) {
    register long exit_code __asm__("rdi") = main();
    register long syscall_number __asm__("rax") = 60;
    __asm__ volatile("syscall"
                     :
                     : "r"(exit_code), "r"(syscall_number)
                     : "rcx", "r11", "memory");
    __builtin_unreachable();
}
