// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

typedef signed char i8;
typedef unsigned char u8;
typedef signed int i32;
typedef signed long i64;
typedef unsigned long u64;

void *memcpy(void *destination, const void *source, u64 count) {
    u8 *out = (u8 *)destination;
    const u8 *in = (const u8 *)source;
    for (u64 index = 0; index < count; ++index) out[index] = in[index];
    return destination;
}

struct empty {};
struct bytes1 { u8 values[1]; };
struct bytes2 { u8 values[2]; };
struct bytes3 { u8 values[3]; };
struct bytes4 { u8 values[4]; };
struct bytes5 { u8 values[5]; };
struct bytes6 { u8 values[6]; };
struct bytes7 { u8 values[7]; };
struct bytes8 { u8 values[8]; };
struct bytes9 { u8 values[9]; };
struct bytes12 { u8 values[12]; };
struct bytes16 { u8 values[16]; };
struct nested { struct bytes3 head; struct bytes5 tail; };
struct array_member { unsigned short values[3]; };
struct pointer_member { u8 *value; };

#define CHECK_SIZE(name, size) _Static_assert(sizeof(struct name) == size, "bad " #name " size")
CHECK_SIZE(bytes1, 1); CHECK_SIZE(bytes2, 2); CHECK_SIZE(bytes3, 3);
CHECK_SIZE(bytes4, 4); CHECK_SIZE(bytes5, 5); CHECK_SIZE(bytes6, 6);
CHECK_SIZE(bytes7, 7); CHECK_SIZE(bytes8, 8); CHECK_SIZE(bytes9, 9);
CHECK_SIZE(bytes12, 12); CHECK_SIZE(bytes16, 16); CHECK_SIZE(nested, 8);
CHECK_SIZE(array_member, 6); CHECK_SIZE(pointer_member, 8);

#define DECLARE_WAVE(name) extern struct name wave_##name(struct name)
DECLARE_WAVE(empty); DECLARE_WAVE(bytes1); DECLARE_WAVE(bytes2); DECLARE_WAVE(bytes3);
DECLARE_WAVE(bytes4); DECLARE_WAVE(bytes5); DECLARE_WAVE(bytes6); DECLARE_WAVE(bytes7);
DECLARE_WAVE(bytes8); DECLARE_WAVE(bytes9); DECLARE_WAVE(bytes12); DECLARE_WAVE(bytes16);
DECLARE_WAVE(nested); DECLARE_WAVE(array_member); DECLARE_WAVE(pointer_member);

#define DEFINE_ECHO(name) struct name c_##name(struct name value) { return value; }
DEFINE_ECHO(empty) DEFINE_ECHO(bytes1) DEFINE_ECHO(bytes2) DEFINE_ECHO(bytes3)
DEFINE_ECHO(bytes4) DEFINE_ECHO(bytes5) DEFINE_ECHO(bytes6) DEFINE_ECHO(bytes7)
DEFINE_ECHO(bytes8) DEFINE_ECHO(bytes9) DEFINE_ECHO(bytes12) DEFINE_ECHO(bytes16)
DEFINE_ECHO(nested) DEFINE_ECHO(array_member) DEFINE_ECHO(pointer_member)

i8 c_i8(i8 value) { return value; }
double c_f64(double value) { return value; }
u8 *c_pointer(u8 *value) { return value; }

i64 c_promotions(i32 count, ...) {
    __builtin_va_list arguments;
    __builtin_va_start(arguments, count);
    i64 sum = 0;
    for (i32 index = 0; index < count; ++index) sum += __builtin_va_arg(arguments, i32);
    __builtin_va_end(arguments);
    return sum;
}

i64 c_pointer_variadic(i32 count, ...) {
    __builtin_va_list arguments;
    __builtin_va_start(arguments, count);
    i64 nonnull = 0;
    for (i32 index = 0; index < count; ++index)
        nonnull += __builtin_va_arg(arguments, void *) != 0;
    __builtin_va_end(arguments);
    return nonnull;
}

i32 c_check_wave_exports(void) {
    wave_empty((struct empty){});
    struct bytes1 b1 = wave_bytes1((struct bytes1){{1}});
    struct bytes2 b2 = wave_bytes2((struct bytes2){{2, 3}});
    struct bytes3 b3 = wave_bytes3((struct bytes3){{3, 4, 5}});
    struct bytes4 b4 = wave_bytes4((struct bytes4){{4, 5, 6, 7}});
    struct bytes5 b5 = wave_bytes5((struct bytes5){{5, 6, 7, 8, 9}});
    struct bytes6 b6 = wave_bytes6((struct bytes6){{6, 7, 8, 9, 10, 11}});
    struct bytes7 b7 = wave_bytes7((struct bytes7){{7, 8, 9, 10, 11, 12, 13}});
    struct bytes8 b8 = wave_bytes8((struct bytes8){{8, 9, 10, 11, 12, 13, 14, 15}});
    struct bytes9 b9 = wave_bytes9((struct bytes9){{9, 10, 11, 12, 13, 14, 15, 16, 17}});
    struct bytes12 b12 = wave_bytes12((struct bytes12){{12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23}});
    struct bytes16 b16 = wave_bytes16((struct bytes16){{16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31}});
    struct nested nested = wave_nested((struct nested){{{1, 2, 3}}, {{4, 5, 6, 7, 8}}});
    struct array_member array = wave_array_member((struct array_member){{101, 102, 103}});
    struct pointer_member pointer = wave_pointer_member((struct pointer_member){0});
    if (b1.values[0] != 1 || b2.values[1] != 3 || b3.values[2] != 5) return 1;
    if (b4.values[3] != 7 || b5.values[4] != 9 || b6.values[5] != 11) return 2;
    if (b7.values[6] != 13 || b8.values[7] != 15 || b9.values[8] != 17) return 3;
    if (b12.values[11] != 23 || b16.values[15] != 31) return 4;
    if (nested.head.values[2] != 3 || nested.tail.values[4] != 8) return 5;
    if (array.values[2] != 103 || pointer.value != 0) return 6;
    return 0;
}

#if defined(__x86_64__)
__asm__(
    ".global _start\n"
    "_start:\n"
    "andq $-16, %rsp\n"
    "call main\n"
    "movslq %eax, %rdi\n"
    "movq $60, %rax\n"
    "syscall\n"
);
#elif defined(__aarch64__)
__asm__(
    ".global _start\n"
    "_start:\n"
    "bl main\n"
    "mov x8, #93\n"
    "svc #0\n"
);
#elif defined(__riscv)
__asm__(
    ".global _start\n"
    "_start:\n"
    "andi sp, sp, -16\n"
    "call main\n"
    "li a7, 93\n"
    "ecall\n"
);
#else
#error unsupported fixture architecture
#endif
