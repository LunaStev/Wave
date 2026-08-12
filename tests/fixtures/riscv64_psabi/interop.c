// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

typedef signed char i8;
typedef unsigned char u8;
typedef signed short i16;
typedef unsigned short u16;
typedef signed int i32;
typedef unsigned int u32;
typedef signed long i64;
typedef unsigned long u64;

#define U32_MAX 4294967295U

void *memcpy(void *destination, const void *source, u64 count) {
    u8 *destination_bytes = (u8 *)destination;
    const u8 *source_bytes = (const u8 *)source;
    for (u64 index = 0; index < count; ++index)
        destination_bytes[index] = source_bytes[index];
    return destination;
}

struct empty {};

struct one {
    u64 value;
};

struct pair {
    u64 first;
    u64 second;
};

struct floats {
    double first;
    double second;
};

struct float_array {
    float values[2];
};

struct mixed {
    double floating;
    i64 integer;
};

struct mixed_reverse {
    i64 integer;
    double floating;
};

struct triple {
    u64 first;
    u64 second;
    u64 third;
};

struct padded {
    u8 small;
    u64 wide;
};

struct nested {
    u16 head;
    struct padded body;
    u32 tail;
};

struct arrayed {
    u16 values[3];
    u64 tail;
};

struct nine {
    u8 values[9];
};

struct twelve {
    u32 first;
    u32 second;
    u32 third;
};

_Static_assert(sizeof(void *) == 8, "RV64 pointers must be 8 bytes");
_Static_assert(_Alignof(void *) == 8, "RV64 pointers must be 8-byte aligned");
_Static_assert(sizeof(struct padded) == 16, "unexpected padded layout");
_Static_assert(sizeof(struct nested) == 32, "unexpected nested layout");
_Static_assert(sizeof(struct arrayed) == 16, "unexpected array layout");
_Static_assert(sizeof(struct nine) == 9, "unexpected 9-byte layout");
_Static_assert(sizeof(struct twelve) == 12, "unexpected 12-byte layout");

extern i8 wave_i8(i8 value);
extern u8 wave_u8(u8 value);
extern i16 wave_i16(i16 value);
extern u16 wave_u16(u16 value);
extern i32 wave_i32(i32 value);
extern u32 wave_u32(u32 value);
extern struct one wave_one(struct one value);
extern struct pair wave_pair(struct pair value);
extern struct floats wave_floats(struct floats value);
extern struct float_array wave_float_array(struct float_array value);
extern struct mixed wave_mixed(struct mixed value);
extern struct mixed_reverse wave_mixed_reverse(struct mixed_reverse value);
extern struct triple wave_triple(struct triple value);
extern u8 *wave_pointer(u8 *value);
extern struct nested wave_nested(struct nested value);
extern struct arrayed wave_arrayed(struct arrayed value);
extern struct nine wave_nine(struct nine value);
extern struct twelve wave_twelve(struct twelve value);
extern i64 wave_stack_ten(i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
extern struct empty wave_empty(struct empty value);

i8 c_i8(i8 value) { return value; }
u8 c_u8(u8 value) { return value; }
i16 c_i16(i16 value) { return value; }
u16 c_u16(u16 value) { return value; }
i32 c_i32(i32 value) { return value; }
u32 c_u32(u32 value) { return value; }
struct one c_one(struct one value) { return value; }
struct pair c_pair(struct pair value) { return value; }
struct floats c_floats(struct floats value) { return value; }
struct float_array c_float_array(struct float_array value) { return value; }
struct mixed c_mixed(struct mixed value) { return value; }
struct mixed_reverse c_mixed_reverse(struct mixed_reverse value) { return value; }
struct triple c_triple(struct triple value) { return value; }
u8 *c_pointer(u8 *value) { return value; }
struct nested c_nested(struct nested value) { return value; }
struct arrayed c_arrayed(struct arrayed value) { return value; }
struct nine c_nine(struct nine value) { return value; }
struct twelve c_twelve(struct twelve value) { return value; }
struct empty c_empty(struct empty value) { return value; }

i64 c_stack_ten(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4,
                i64 a5, i64 a6, i64 a7, i64 a8, i64 a9) {
    register u64 stack_pointer __asm__("sp");
    if ((stack_pointer & 15) != 0)
        return -1;
    return a0 + a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + a9;
}

i64 c_variadic_sum(i32 count, ...) {
    __builtin_va_list arguments;
    __builtin_va_start(arguments, count);
    i64 sum = 0;
    for (i32 index = 0; index < count; ++index)
        sum += __builtin_va_arg(arguments, i64);
    __builtin_va_end(arguments);
    return sum;
}

double c_variadic_f64(i32 count, ...) {
    __builtin_va_list arguments;
    __builtin_va_start(arguments, count);
    double sum = 0.0;
    for (i32 index = 0; index < count; ++index)
        sum += __builtin_va_arg(arguments, double);
    __builtin_va_end(arguments);
    return sum;
}

i64 c_variadic_promotions(i32 count, ...) {
    __builtin_va_list arguments;
    __builtin_va_start(arguments, count);
    i64 sum = 0;
    for (i32 index = 0; index < count; ++index)
        sum += __builtin_va_arg(arguments, i32);
    __builtin_va_end(arguments);
    return sum;
}

i32 c_check_wave_exports(void) {
    wave_empty((struct empty){});
    struct one one = wave_one((struct one){7});
    struct pair pair = wave_pair((struct pair){11, 22});
    struct floats floats = wave_floats((struct floats){1.5, 2.5});
    struct float_array float_array = wave_float_array((struct float_array){{1.25f, 2.75f}});
    struct mixed mixed = wave_mixed((struct mixed){3.5, -7});
    struct mixed_reverse mixed_reverse = wave_mixed_reverse((struct mixed_reverse){-8, 4.5});
    struct triple triple = wave_triple((struct triple){31, 32, 33});
    struct nested nested = wave_nested(
        (struct nested){41, {42, 43}, 44});
    struct arrayed arrayed = wave_arrayed(
        (struct arrayed){{51, 52, 53}, 54});
    struct nine nine = wave_nine((struct nine){{61, 62, 63, 64, 65, 66, 67, 68, 69}});
    struct twelve twelve = wave_twelve((struct twelve){71, 72, 73});

    if (wave_i8(-128) != -128 || wave_u8(255) != 255)
        return 1;
    if (wave_i16(-32768) != -32768 || wave_u16(65535) != 65535)
        return 2;
    if (wave_i32(-2147483647 - 1) != (-2147483647 - 1))
        return 3;
    if (wave_u32(U32_MAX) != U32_MAX)
        return 4;
    if (one.value != 7)
        return 5;
    if (pair.first != 11 || pair.second != 22)
        return 6;
    if (floats.first != 1.5 || floats.second != 2.5)
        return 7;
    if (float_array.values[0] != 1.25f || float_array.values[1] != 2.75f)
        return 16;
    if (mixed.floating != 3.5 || mixed.integer != -7)
        return 8;
    if (mixed_reverse.integer != -8 || mixed_reverse.floating != 4.5)
        return 17;
    if (triple.first != 31 || triple.second != 32 || triple.third != 33)
        return 9;
    if (wave_pointer((u8 *)0) != (u8 *)0)
        return 10;
    if (nested.head != 41 || nested.body.small != 42 ||
        nested.body.wide != 43 || nested.tail != 44)
        return 11;
    if (arrayed.values[0] != 51 || arrayed.values[1] != 52 ||
        arrayed.values[2] != 53 || arrayed.tail != 54)
        return 12;
    if (wave_stack_ten(1, 2, 3, 4, 5, 6, 7, 8, 9, 10) != 55)
        return 13;
    if (nine.values[0] != 61 || nine.values[8] != 69)
        return 14;
    if (twelve.first != 71 || twelve.second != 72 || twelve.third != 73)
        return 15;
    return 0;
}

extern i32 main(void);

__attribute__((noreturn)) void _start(void) {
    register long exit_code __asm__("a0") = main();
    register long syscall_number __asm__("a7") = 93;
    __asm__ volatile("ecall" : : "r"(exit_code), "r"(syscall_number) : "memory");
    __builtin_unreachable();
}
