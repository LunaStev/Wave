// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

typedef signed char i8;
typedef unsigned int u32;
typedef signed int i32;
typedef signed long i64;
typedef unsigned long u64;

void *memcpy(void *destination, const void *source, u64 count) {
    unsigned char *out = (unsigned char *)destination;
    const unsigned char *in = (const unsigned char *)source;
    for (u64 index = 0; index < count; ++index)
        out[index] = in[index];
    return destination;
}

struct pair { u64 first; u64 second; };
struct floats { double first; double second; };
struct mixed { double floating; i64 integer; };
struct triple { u64 first; u64 second; u64 third; };

_Static_assert(sizeof(void *) == 8, "x86_64 pointers must be 8 bytes");
_Static_assert(sizeof(struct pair) == 16, "unexpected pair layout");
_Static_assert(sizeof(struct mixed) == 16, "unexpected mixed layout");
_Static_assert(sizeof(struct triple) == 24, "unexpected triple layout");

extern i8 wave_i8(i8);
extern u32 wave_u32(u32);
extern float wave_f32(float);
extern double wave_f64(double);
extern struct pair wave_pair(struct pair);
extern struct floats wave_floats(struct floats);
extern struct mixed wave_mixed(struct mixed);
extern struct triple wave_triple(struct triple);
extern i64 wave_stack_ten(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);

i8 c_i8(i8 value) { return value; }
u32 c_u32(u32 value) { return value; }
float c_f32(float value) { return value; }
double c_f64(double value) { return value; }
struct pair c_pair(struct pair value) { return value; }
struct floats c_floats(struct floats value) { return value; }
struct mixed c_mixed(struct mixed value) { return value; }
struct triple c_triple(struct triple value) { return value; }

i64 c_stack_ten(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4,
                i64 a5, i64 a6, i64 a7, i64 a8, i64 a9) {
    return a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9;
}

i32 c_check_wave_exports(void) {
    struct pair pair = wave_pair((struct pair){11, 22});
    struct floats floats = wave_floats((struct floats){3.5, 4.5});
    struct mixed mixed = wave_mixed((struct mixed){5.5, -6});
    struct triple triple = wave_triple((struct triple){31, 32, 33});

    if (wave_i8(-128) != -128 || wave_u32(4294967295U) != 4294967295U)
        return 1;
    if (wave_f32(1.5f) != 1.5f || wave_f64(2.5) != 2.5)
        return 2;
    if (pair.first != 11 || pair.second != 22)
        return 3;
    if (floats.first != 3.5 || floats.second != 4.5)
        return 4;
    if (mixed.floating != 5.5 || mixed.integer != -6)
        return 5;
    if (triple.first != 31 || triple.second != 32 || triple.third != 33)
        return 6;
    if (wave_stack_ten(1, 2, 3, 4, 5, 6, 7, 8, 9, 10) != 55)
        return 7;
    return 0;
}

extern i32 main(void);

__attribute__((force_align_arg_pointer, noreturn)) void _start(void) {
    register long exit_code __asm__("rdi") = main();
    register long syscall_number __asm__("rax") = 60;
    __asm__ volatile("syscall"
                     :
                     : "r"(exit_code), "r"(syscall_number)
                     : "rcx", "r11", "memory");
    __builtin_unreachable();
}
