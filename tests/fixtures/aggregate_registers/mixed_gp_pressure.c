// SPDX-License-Identifier: MPL-2.0
struct S { double a; long long b; };
int c_check(long long a0, long long a1, long long a2, long long a3, long long a4, long long a5, long long a6, long long a7, struct S v) { return (v.a == 8.5 && v.b == 9) ? 0 : 11; }
extern int wave_check(long long a0, long long a1, long long a2, long long a3, long long a4, long long a5, long long a6, long long a7, struct S v);
int c_exports(void) { return wave_check(1, 2, 3, 4, 5, 6, 7, 8, (struct S){8.5,9}); }
