// SPDX-License-Identifier: MPL-2.0
// LP64S register/stack transport. Floating values are inspected as bits so this
// freestanding fixture does not require a soft-float arithmetic runtime.
#include <stdarg.h>
typedef unsigned long long u64;
typedef signed long long i64;
struct pair { double value; i64 tag; };
struct large { struct pair value; i64 guard; };
union bits { double floating; u64 integer; };
extern double wave_double(double);
extern struct pair wave_pair(struct pair);
extern struct large wave_large(struct large);
extern i64 wave_stack(i64,i64,i64,i64,i64,i64,i64,i64,struct pair);
extern int main(void);

u64 c_double(double value) { union bits b = {.floating=value}; return b.integer; }
struct pair c_pair(struct pair value) { value.tag += 11; return value; }
struct large c_large(struct large value) { value.guard += 23; return value; }
i64 c_stack(i64 a,i64 b,i64 c,i64 d,i64 e,i64 f,i64 g,i64 h,struct pair p) {
    if (a!=1 || b!=2 || c!=3 || d!=4 || e!=5 || f!=6 || g!=7 || h!=8) return -1;
    if (c_double(p.value)!=0x402f000000000000ULL) return -2;
    return p.tag;
}
int c_variadic(int count, ...) {
    va_list args; va_start(args,count);
    double value=va_arg(args,double); int narrow=va_arg(args,int);
    i64 tag=va_arg(args,i64); va_end(args);
    return count==3 && c_double(value)==0x8000000000000000ULL && narrow==-123 &&
        tag==9876543210LL ? 0 : 1;
}
int c_exports(void) {
    if (c_double(wave_double(-0.0))!=0x8000000000000000ULL) return 1;
    struct pair input={15.5,9876543210LL};
    struct pair p=wave_pair(input);
    if (p.tag!=9876543217LL || c_double(p.value)!=0x402f000000000000ULL) return 2;
    struct large l=wave_large((struct large){input,123456789});
    if (l.guard!=123456808 || l.value.tag!=9876543210LL ||
        c_double(l.value.value)!=0x402f000000000000ULL) return 3;
    if (wave_stack(1,2,3,4,5,6,7,8,input)!=9876543210LL) return 4;
    return 0;
}
__asm__(".global _start\n_start:\nbl main\naddi.d $a7, $zero, 93\nsyscall 0\n");
