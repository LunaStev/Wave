// SPDX-License-Identifier: MPL-2.0
#include <stdarg.h>
typedef unsigned char u8;
typedef signed char i8;
typedef short i16;
typedef unsigned short u16;
typedef int i32;
typedef unsigned int u32;
typedef long long i64;
#define KEEP __declspec(noinline)

typedef struct { u8 data[1]; } Bytes1;
_Static_assert(sizeof(Bytes1) == 1, "byte aggregate size");

extern Bytes1 wave_bytes1(Bytes1 value);
KEEP Bytes1 c_bytes1(Bytes1 value) {
    for (int i = 0; i < 1; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[2]; } Bytes2;
_Static_assert(sizeof(Bytes2) == 2, "byte aggregate size");

extern Bytes2 wave_bytes2(Bytes2 value);
KEEP Bytes2 c_bytes2(Bytes2 value) {
    for (int i = 0; i < 2; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[3]; } Bytes3;
_Static_assert(sizeof(Bytes3) == 3, "byte aggregate size");

extern Bytes3 wave_bytes3(Bytes3 value);
KEEP Bytes3 c_bytes3(Bytes3 value) {
    for (int i = 0; i < 3; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[4]; } Bytes4;
_Static_assert(sizeof(Bytes4) == 4, "byte aggregate size");

extern Bytes4 wave_bytes4(Bytes4 value);
KEEP Bytes4 c_bytes4(Bytes4 value) {
    for (int i = 0; i < 4; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[7]; } Bytes7;
_Static_assert(sizeof(Bytes7) == 7, "byte aggregate size");

extern Bytes7 wave_bytes7(Bytes7 value);
KEEP Bytes7 c_bytes7(Bytes7 value) {
    for (int i = 0; i < 7; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[8]; } Bytes8;
_Static_assert(sizeof(Bytes8) == 8, "byte aggregate size");

extern Bytes8 wave_bytes8(Bytes8 value);
KEEP Bytes8 c_bytes8(Bytes8 value) {
    for (int i = 0; i < 8; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[9]; } Bytes9;
_Static_assert(sizeof(Bytes9) == 9, "byte aggregate size");

extern Bytes9 wave_bytes9(Bytes9 value);
KEEP Bytes9 c_bytes9(Bytes9 value) {
    for (int i = 0; i < 9; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[16]; } Bytes16;
_Static_assert(sizeof(Bytes16) == 16, "byte aggregate size");

extern Bytes16 wave_bytes16(Bytes16 value);
KEEP Bytes16 c_bytes16(Bytes16 value) {
    for (int i = 0; i < 16; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct { u8 data[17]; } Bytes17;
_Static_assert(sizeof(Bytes17) == 17, "byte aggregate size");

extern Bytes17 wave_bytes17(Bytes17 value);
KEEP Bytes17 c_bytes17(Bytes17 value) {
    for (int i = 0; i < 17; ++i) {
        value.data[i] ^= (u8)(i + 37);
    }
    return value;
}

typedef struct {
    i64 left;
    i64 right;
} Pair;

extern Pair wave_pair(Pair value);
KEEP Pair c_pair(Pair value) {
    value.left += 1;
    value.right += 2;
    return value;
}

typedef struct {
    float a;
    float b;
    float c;
    float d;
} Hfa;

extern Hfa wave_hfa(Hfa value);
KEEP Hfa c_hfa(Hfa value) {
    value.a += 1;
    value.b += 2;
    value.c += 3;
    value.d += 4;
    return value;
}

typedef struct {
    double a;
    double b;
    double c;
    double d;
} Hda;

extern Hda wave_hda(Hda value);
KEEP Hda c_hda(Hda value) {
    value.a += 1;
    value.b += 2;
    value.c += 3;
    value.d += 4;
    return value;
}

typedef struct {
    u8 tag;
    double value;
    i16 tail;
} Aligned;

extern Aligned wave_aligned(Aligned value);
KEEP Aligned c_aligned(Aligned value) {
    value.tag += 1;
    value.value += 2;
    value.tail += 3;
    return value;
}

extern i64 wave_narrow(i8, u8, i16, u16, i32, u32);
KEEP i64 c_narrow(i8 a, u8 b, i16 c, u16 d, i32 e, u32 f) {
    return (i64)a + (i64)b * 3 + (i64)c * 5 + (i64)d * 7 + (i64)e * 11 + (i64)f * 13;
}

extern double wave_mixed(
    i64 i1, double d1,
    i64 i2, double d2,
    i64 i3, double d3,
    i64 i4, double d4,
    i64 i5, double d5,
    i64 i6, double d6,
    i64 i7, double d7,
    i64 i8, double d8,
    i64 i9, double d9,
    i64 i10, double d10
);
KEEP double c_mixed(
    i64 i1, double d1,
    i64 i2, double d2,
    i64 i3, double d3,
    i64 i4, double d4,
    i64 i5, double d5,
    i64 i6, double d6,
    i64 i7, double d7,
    i64 i8, double d8,
    i64 i9, double d9,
    i64 i10, double d10
) {
    return i1 * 1.0 + d1 * 1.0
        + i2 * 2.0 + d2 * 2.0
        + i3 * 3.0 + d3 * 3.0
        + i4 * 4.0 + d4 * 4.0
        + i5 * 5.0 + d5 * 5.0
        + i6 * 6.0 + d6 * 6.0
        + i7 * 7.0 + d7 * 7.0
        + i8 * 8.0 + d8 * 8.0
        + i9 * 9.0 + d9 * 9.0
        + i10 * 10.0 + d10 * 10.0;
}

extern float wave_hfa_stack(float, float, float, float, float, float, float, Hfa, float);
KEEP float c_hfa_stack(float a, float b, float c, float d, float e, float f, float g, Hfa value, float tail) {
    return a + b * 2 + c * 3 + d * 4 + e * 5 + f * 6 + g * 7
        + value.a * 8 + value.b * 9 + value.c * 10 + value.d * 11 + tail * 12;
}
KEEP int c_variadic(int tag, ...) {
    va_list args;
    va_start(args, tag);
    int failed = tag != 73;
    failed |= va_arg(args, int) != -128;
    failed |= va_arg(args, int) != 255;
    failed |= va_arg(args, double) != 1.5;
    const u8 *pointer = va_arg(args, const u8 *);
    failed |= !pointer || *pointer != 91;
    for (int i = 1; i <= 9; ++i) {
        failed |= va_arg(args, int) != i;
    }
    for (int i = 1; i <= 9; ++i) {
        failed |= va_arg(args, double) != i * 0.25;
    }
    va_end(args);
    return failed;
}

KEEP int c_aggregate_variadic(Hfa small, Hda large, int tag, ...) {
    va_list args;
    va_start(args, tag);
    int failed = tag != 19 || small.a != 1 || small.b != 2 || small.c != 3 || small.d != 4
        || large.a != 5 || large.b != 6 || large.c != 7 || large.d != 8;
    failed |= va_arg(args, int) != -128;
    failed |= va_arg(args, double) != 1.5;
    va_end(args);
    return failed;
}

extern double wave_nested(i64 seed);

KEEP int c_check_wave(void) {
    Bytes1 b1;
    for (int i = 0; i < 1; ++i) {
        b1.data[i] = (u8)(i + 129);
    }
    Bytes1 r1 = wave_bytes1(b1);
    for (int i = 0; i < 1; ++i) {
        if (r1.data[i] != ((i + 129) ^ (i + 37)) || b1.data[i] != i + 129) {
            return 1;
        }
    }
    Bytes2 b2;
    for (int i = 0; i < 2; ++i) {
        b2.data[i] = (u8)(i + 129);
    }
    Bytes2 r2 = wave_bytes2(b2);
    for (int i = 0; i < 2; ++i) {
        if (r2.data[i] != ((i + 129) ^ (i + 37)) || b2.data[i] != i + 129) {
            return 2;
        }
    }
    Bytes3 b3;
    for (int i = 0; i < 3; ++i) {
        b3.data[i] = (u8)(i + 129);
    }
    Bytes3 r3 = wave_bytes3(b3);
    for (int i = 0; i < 3; ++i) {
        if (r3.data[i] != ((i + 129) ^ (i + 37)) || b3.data[i] != i + 129) {
            return 3;
        }
    }
    Bytes4 b4;
    for (int i = 0; i < 4; ++i) {
        b4.data[i] = (u8)(i + 129);
    }
    Bytes4 r4 = wave_bytes4(b4);
    for (int i = 0; i < 4; ++i) {
        if (r4.data[i] != ((i + 129) ^ (i + 37)) || b4.data[i] != i + 129) {
            return 4;
        }
    }
    Bytes7 b7;
    for (int i = 0; i < 7; ++i) {
        b7.data[i] = (u8)(i + 129);
    }
    Bytes7 r7 = wave_bytes7(b7);
    for (int i = 0; i < 7; ++i) {
        if (r7.data[i] != ((i + 129) ^ (i + 37)) || b7.data[i] != i + 129) {
            return 5;
        }
    }
    Bytes8 b8;
    for (int i = 0; i < 8; ++i) {
        b8.data[i] = (u8)(i + 129);
    }
    Bytes8 r8 = wave_bytes8(b8);
    for (int i = 0; i < 8; ++i) {
        if (r8.data[i] != ((i + 129) ^ (i + 37)) || b8.data[i] != i + 129) {
            return 6;
        }
    }
    Bytes9 b9;
    for (int i = 0; i < 9; ++i) {
        b9.data[i] = (u8)(i + 129);
    }
    Bytes9 r9 = wave_bytes9(b9);
    for (int i = 0; i < 9; ++i) {
        if (r9.data[i] != ((i + 129) ^ (i + 37)) || b9.data[i] != i + 129) {
            return 7;
        }
    }
    Bytes16 b16;
    for (int i = 0; i < 16; ++i) {
        b16.data[i] = (u8)(i + 129);
    }
    Bytes16 r16 = wave_bytes16(b16);
    for (int i = 0; i < 16; ++i) {
        if (r16.data[i] != ((i + 129) ^ (i + 37)) || b16.data[i] != i + 129) {
            return 8;
        }
    }
    Bytes17 b17;
    for (int i = 0; i < 17; ++i) {
        b17.data[i] = (u8)(i + 129);
    }
    Bytes17 r17 = wave_bytes17(b17);
    for (int i = 0; i < 17; ++i) {
        if (r17.data[i] != ((i + 129) ^ (i + 37)) || b17.data[i] != i + 129) {
            return 9;
        }
    }
    Pair pair = wave_pair((Pair) { 41, -73 });
    if (pair.left != 42 || pair.right != -71) {
        return 10;
    }
    Hfa hfa = wave_hfa((Hfa) { 1.0, 2.0, 3.0, 4.0 });
    if (hfa.a != 2.0 || hfa.b != 4.0 || hfa.c != 6.0 || hfa.d != 8.0) {
        return 11;
    }
    Hda hda = wave_hda((Hda) { 1.0, 2.0, 3.0, 4.0 });
    if (hda.a != 2.0 || hda.b != 4.0 || hda.c != 6.0 || hda.d != 8.0) {
        return 12;
    }
    Aligned aligned = wave_aligned((Aligned) { 201, 4.5, -32000 });
    if (aligned.tag != 202 || aligned.value != 6.5 || aligned.tail != -31997) {
        return 13;
    }
    if (wave_narrow(-128, 255, -32768, 65535, -2147483647, 4294967295) != 32212550260) {
        return 14;
    }
    if (wave_mixed(1, 0.5, 2, 1.0, 3, 1.5, 4, 2.0, 5, 2.5, 6, 3.0, 7, 3.5, 8, 4.0, 9, 4.5, 10, 5.0) != 577.5) {
        return 15;
    }
    if (wave_hfa_stack(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, (Hfa) { 8, 9, 10, 11 }, 12.0) != 650.0) {
        return 16;
    }
    for (i64 seed = 1; seed <= 17; ++seed) {
        volatile double live = seed * 3.25;
        volatile i64 canary[4] = { seed, -seed, seed * 7, seed * 11 };
        double result = wave_nested(seed);
        if (result != 601.5 + seed * 19.5 || live != seed * 3.25
                || canary[0] != seed || canary[1] != -seed
                || canary[2] != seed * 7 || canary[3] != seed * 11) {
            return 40;
        }
    }
    return 0;
}
