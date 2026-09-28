// SPDX-License-Identifier: MPL-2.0
// Independently compiled C caller exercises the public native ABI. C may use
// its platform runtime; the Wave object is separately checked for dependencies.
#include <stdint.h>
typedef unsigned __int128 u128;
typedef __int128 i128;
extern u128 wave_unsigned_div(u128, u128), wave_unsigned_rem(u128, u128);
extern i128 wave_signed_div(i128, i128), wave_signed_rem(i128, i128);
extern float wave_to_float(u128);
extern i128 wave_from_double(double);
int main(int argc, char **argv) {
    (void)argv;
    const u128 samples[] = {0, 1, 3, (u128)1 << 64, (u128)1 << 100,
                            ((u128)1 << 127) - 1, (u128)1 << 127, ~(u128)0};
    for (unsigned i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) {
        u128 a = samples[i] + (unsigned)(argc - 1);
        u128 b = ((u128)1 << 65) + (unsigned)argc;
        if (wave_unsigned_div(a, b) != a / b || wave_unsigned_rem(a, b) != a % b) return 1;
        if (wave_signed_div((i128)a, -(i128)b) != (i128)a / -(i128)b) return 2;
        if (wave_signed_rem((i128)a, -(i128)b) != (i128)a % -(i128)b) return 3;
        if (wave_to_float(a) != (float)a) return 4;
    }
    double value = -0x1p100 * argc;
    if (wave_from_double(value) != (i128)value) return 5;
    return 0;
}
