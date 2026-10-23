#include <stdint.h>

extern int32_t wave_double(int32_t value);

int32_t c_roundtrip(int32_t value) {
    return wave_double(value) + 2;
}
