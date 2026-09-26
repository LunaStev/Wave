// SPDX-License-Identifier: MPL-2.0
struct S { double a,b; };
int c_check(double a0, double a1, double a2, double a3, double a4, double a5, double a6, struct S v) { return (v.a == 8.5 && v.b == 9.5) ? 0 : 11; }
extern int wave_check(double a0, double a1, double a2, double a3, double a4, double a5, double a6, struct S v);
int c_exports(void) { return wave_check(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, (struct S){8.5,9.5}); }
