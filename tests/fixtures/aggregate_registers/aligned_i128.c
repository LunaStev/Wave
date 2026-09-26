// SPDX-License-Identifier: MPL-2.0
struct S { __int128 a; };
int c_check(long long a0, struct S v) { return (v.a == 17) ? 0 : 11; }
extern int wave_check(long long a0, struct S v);
int c_exports(void) { return wave_check(1, (struct S){17}); }
