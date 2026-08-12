// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

typedef signed int i32;
typedef unsigned int u32;
typedef unsigned long u64;

union f32_bits {
    float value;
    u32 bits;
};

union f64_bits {
    double value;
    u64 bits;
};

extern float wave_roundtrip_f32(float value);
extern double wave_roundtrip_f64(double value);

float c_identity_f32(float value) { return value; }
double c_identity_f64(double value) { return value; }

__attribute__((noreturn)) void _start(void) {
    union f32_bits f32 = {.bits = 0x3fc00000U};
    union f64_bits f64 = {.bits = 0x4004000000000000UL};
    f32.value = wave_roundtrip_f32(f32.value);
    f64.value = wave_roundtrip_f64(f64.value);

    i32 status = 0;
    if (f32.bits != 0x3fc00000U)
        status = 1;
    if (f64.bits != 0x4004000000000000UL)
        status = 2;

    register long exit_code __asm__("a0") = status;
    register long syscall_number __asm__("a7") = 93;
    __asm__ volatile("ecall" : : "r"(exit_code), "r"(syscall_number) : "memory");
    __builtin_unreachable();
}
