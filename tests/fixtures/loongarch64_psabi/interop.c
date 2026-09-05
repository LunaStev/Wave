// This file is part of the Wave language project.
// Copyright (c) 2024-2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

#include <stdarg.h>

typedef signed int i32;
typedef signed long long i64;

struct f1 { double value; };
struct f2 { double first; double second; };
struct fi { double floating; i64 integer; };
struct if_pair { i64 integer; double floating; };
struct fd_padded { float first; double second; };
struct nested { struct f1 first; i32 second; };
struct large { i64 values[3]; };

extern struct f1 wave_f1(struct f1 value);
extern struct f2 wave_f2(struct f2 value);
extern struct fi wave_fi(struct fi value);
extern struct if_pair wave_if_pair(struct if_pair value);
extern struct fd_padded wave_fd_padded(struct fd_padded value);
extern struct nested wave_nested(struct nested value);
extern struct large wave_large(struct large value);
extern struct fi wave_fi_after_gars(
    i64, i64, i64, i64, i64, i64, i64, i64, struct fi);
extern struct f2 wave_f2_after_fars(
    double, double, double, double, double, double, double, double, struct f2);

struct f1 c_f1(struct f1 value) { return value; }
struct f2 c_f2(struct f2 value) { return value; }
struct fi c_fi(struct fi value) { return value; }
struct if_pair c_if_pair(struct if_pair value) { return value; }
struct fd_padded c_fd_padded(struct fd_padded value) { return value; }
struct nested c_nested(struct nested value) { return value; }
struct large c_large(struct large value) { return value; }
struct fi c_fi_after_gars(
    i64 a0, i64 a1, i64 a2, i64 a3, i64 a4, i64 a5, i64 a6, i64 a7,
    struct fi value) {
    if (a0 != 0 || a1 != 1 || a2 != 2 || a3 != 3 ||
        a4 != 4 || a5 != 5 || a6 != 6 || a7 != 7)
        return (struct fi){-1.0, -1};
    return value;
}
struct f2 c_f2_after_fars(
    double f0, double f1, double f2, double f3,
    double f4, double f5, double f6, double f7, struct f2 value) {
    if (f0 != 0.0 || f1 != 1.0 || f2 != 2.0 || f3 != 3.0 ||
        f4 != 4.0 || f5 != 5.0 || f6 != 6.0 || f7 != 7.0)
        return (struct f2){-1.0, -1.0};
    return value;
}
i32 c_check_variadic(i32 count, ...) {
    va_list arguments;
    va_start(arguments, count);
    double floating = va_arg(arguments, double);
    i32 integer = va_arg(arguments, i32);
    va_end(arguments);
    return count == 2 && floating == 15.5 && integer == -16 ? 0 : 1;
}

i32 c_check_wave_exports(void) {
    struct f1 one = wave_f1((struct f1){1.25});
    struct f2 two = wave_f2((struct f2){2.5, 3.75});
    struct fi mixed = wave_fi((struct fi){4.5, 45});
    struct if_pair reverse = wave_if_pair((struct if_pair){56, 5.5});
    struct fd_padded padded = wave_fd_padded((struct fd_padded){6.25f, 7.5});
    struct nested nested = wave_nested((struct nested){{8.5}, 85});
    struct large large = wave_large((struct large){{9, 10, 11}});
    struct fi after_gars = wave_fi_after_gars(
        0, 1, 2, 3, 4, 5, 6, 7, (struct fi){12.5, 125});
    struct f2 after_fars = wave_f2_after_fars(
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0,
        (struct f2){13.5, 14.5});

    if (one.value != 1.25) return 1;
    if (two.first != 2.5 || two.second != 3.75) return 2;
    if (mixed.floating != 4.5 || mixed.integer != 45) return 3;
    if (reverse.integer != 56 || reverse.floating != 5.5) return 4;
    if (padded.first != 6.25f || padded.second != 7.5) return 5;
    if (nested.first.value != 8.5 || nested.second != 85) return 6;
    if (large.values[0] != 9 || large.values[1] != 10 || large.values[2] != 11) return 7;
    if (after_gars.floating != 12.5 || after_gars.integer != 125) return 8;
    if (after_fars.first != 13.5 || after_fars.second != 14.5) return 9;
    return 0;
}

__asm__(
    ".global _start\n"
    "_start:\n"
    "bl main\n"
    "addi.d $a7, $zero, 93\n"
    "syscall 0\n"
);
