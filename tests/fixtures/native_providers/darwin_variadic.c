// SPDX-License-Identifier: MPL-2.0

double c_variadic(int count, ...) {
    __builtin_va_list args;
    __builtin_va_start(args, count);
    double sum = 0;
    for (int i=0; i<count; i++) sum += __builtin_va_arg(args, double);
    sum += __builtin_va_arg(args, int);
    sum += __builtin_va_arg(args, int);
    __builtin_va_end(args);
    return sum;
}
