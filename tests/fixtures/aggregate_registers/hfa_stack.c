// SPDX-License-Identifier: MPL-2.0
struct S { float a,b,c; };
int c_check(double a0, double a1, double a2, double a3, double a4, double a5, double a6, double a7, struct S v, struct S w) { return (v.a == 1.5 && v.b == 2.5 && v.c == 3.5 && w.a == 4.5 && w.b == 5.5 && w.c == 6.5) ? 0 : 11; }
extern int wave_check(double a0, double a1, double a2, double a3, double a4, double a5, double a6, double a7, struct S v, struct S w);
int c_exports(void) { return wave_check(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, (struct S){1.5,2.5,3.5}, (struct S){4.5,5.5,6.5}); }
