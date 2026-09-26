// SPDX-License-Identifier: MPL-2.0
struct S { void *p; double f; };
int c_check(struct S v) { return (v.p == (void*)16 && v.f == 2.5) ? 0 : 11; }
extern int wave_check(struct S v);
int c_exports(void) { return wave_check((struct S){(void*)16, 2.5}); }
