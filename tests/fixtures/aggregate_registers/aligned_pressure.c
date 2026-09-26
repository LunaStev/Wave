// SPDX-License-Identifier: MPL-2.0
struct S { __int128 a; };
int c_check0(struct S v, long tail) { return (v.a == (((__int128)17 << 64) + 23) && tail == 91) ? 0 : 10; }
extern int wave_check0(struct S v, long tail);
int c_check1(long a0, struct S v, long tail) { return (a0 == 1 && v.a == (((__int128)17 << 64) + 23) && tail == 91) ? 0 : 11; }
extern int wave_check1(long a0, struct S v, long tail);
int c_check2(long a0, long a1, struct S v, long tail) { return (a0 == 1 && a1 == 2 && v.a == (((__int128)17 << 64) + 23) && tail == 91) ? 0 : 12; }
extern int wave_check2(long a0, long a1, struct S v, long tail);
int c_check7(long a0, long a1, long a2, long a3, long a4, long a5, long a6, struct S v, long tail) { return (a0 == 1 && a1 == 2 && a2 == 3 && a3 == 4 && a4 == 5 && a5 == 6 && a6 == 7 && v.a == (((__int128)17 << 64) + 23) && tail == 91) ? 0 : 17; }
extern int wave_check7(long a0, long a1, long a2, long a3, long a4, long a5, long a6, struct S v, long tail);
int c_check8(long a0, long a1, long a2, long a3, long a4, long a5, long a6, long a7, struct S v, long tail) { return (a0 == 1 && a1 == 2 && a2 == 3 && a3 == 4 && a4 == 5 && a5 == 6 && a6 == 7 && a7 == 8 && v.a == (((__int128)17 << 64) + 23) && tail == 91) ? 0 : 18; }
extern int wave_check8(long a0, long a1, long a2, long a3, long a4, long a5, long a6, long a7, struct S v, long tail);
int c_exports(void) { return wave_check0((struct S){((__int128)17 << 64) + 23}, 91) | wave_check1(1, (struct S){((__int128)17 << 64) + 23}, 91) | wave_check2(1, 2, (struct S){((__int128)17 << 64) + 23}, 91) | wave_check7(1, 2, 3, 4, 5, 6, 7, (struct S){((__int128)17 << 64) + 23}, 91) | wave_check8(1, 2, 3, 4, 5, 6, 7, 8, (struct S){((__int128)17 << 64) + 23}, 91); }
