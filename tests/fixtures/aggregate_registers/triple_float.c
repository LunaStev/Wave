// SPDX-License-Identifier: MPL-2.0
struct S { float a,b,c; };
int c_check(struct S v) { return (v.a == 1.5 && v.b == 2.5 && v.c == 3.5) ? 0 : 11; }
extern int wave_check(struct S v);
int c_exports(void) { return wave_check((struct S){1.5,2.5,3.5}); }
