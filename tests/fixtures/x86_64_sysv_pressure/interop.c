// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
// SPDX-License-Identifier: MPL-2.0

typedef long i64;
struct Hfa { float a, b, c, d; };
struct Pair { i64 a, b; };
struct Mixed { double a; i64 b; };
struct Triple { i64 a, b, c; };

extern float wave_sse_exact(float a0, float a1, float a2, float a3, float a4, float a5, struct Hfa value, float tail);
float c_sse_exact(float a0, float a1, float a2, float a3, float a4, float a5, struct Hfa value, float tail) {
    return a0 * 1 + a1 * 2 + a2 * 3 + a3 * 4 + a4 * 5 + a5 * 6 + value.a * 7 + value.b * 8 + value.c * 9 + value.d * 10 + tail * 11;
}

extern float wave_sse_spill(float a0, float a1, float a2, float a3, float a4, float a5, float a6, struct Hfa value, float tail);
float c_sse_spill(float a0, float a1, float a2, float a3, float a4, float a5, float a6, struct Hfa value, float tail) {
    return a0 * 1 + a1 * 2 + a2 * 3 + a3 * 4 + a4 * 5 + a5 * 6 + a6 * 7 + value.a * 8 + value.b * 9 + value.c * 10 + value.d * 11 + tail * 12;
}

extern i64 wave_gp_spill(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4, struct Pair value, i64 tail);
i64 c_gp_spill(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4, struct Pair value, i64 tail) {
    return a0 * 1 + a1 * 2 + a2 * 3 + a3 * 4 + a4 * 5 + value.a * 6 + value.b * 7 + tail * 8;
}

extern struct Triple wave_hidden_return(i64 a0, i64 a1, i64 a2, i64 a3, struct Pair value, i64 tail);
struct Triple c_hidden_return(i64 a0, i64 a1, i64 a2, i64 a3, struct Pair value, i64 tail) {
    return (struct Triple){a0 * 1 + a1 * 2 + a2 * 3 + a3 * 4 + value.a * 5 + value.b * 6 + tail * 7, 71, 93};
}

extern double wave_mixed_spill(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4, i64 a5, struct Mixed value, double tail);
double c_mixed_spill(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4, i64 a5, struct Mixed value, double tail) {
    return a0 * 1 + a1 * 2 + a2 * 3 + a3 * 4 + a4 * 5 + a5 * 6 + value.a * 7 + value.b * 8 + tail * 9;
}

int c_check(void) {
    if (wave_sse_exact(1, 2, 3, 4, 5, 6, (struct Hfa){7, 8, 9, 10}, 11) != 506) {
        return 1;
    }
    if (wave_sse_spill(1, 2, 3, 4, 5, 6, 7, (struct Hfa){8, 9, 10, 11}, 12) != 650) {
        return 2;
    }
    if (wave_gp_spill(1, 2, 3, 4, 5, (struct Pair){6, 7}, 8) != 204) {
        return 3;
    }
    struct Triple result = wave_hidden_return(1, 2, 3, 4, (struct Pair){5, 6}, 7);
    if (result.a != 140 || result.b != 71 || result.c != 93) {
        return 4;
    }
    if (wave_mixed_spill(1, 2, 3, 4, 5, 6, (struct Mixed){7, 8}, 9) != 285) {
        return 5;
    }
    return 0;
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
