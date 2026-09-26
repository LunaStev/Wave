// SPDX-License-Identifier: MPL-2.0
#include <stdint.h>
#include <stdlib.h>
struct change { uint64_t ident; int16_t filter; uint16_t flags; uint32_t fflags; int64_t data; void *udata; };
static int errors[2], calls, error_value;
void reset(int first, int second) { errors[0]=first; errors[1]=second; calls=0; }
int get_calls(void) { return calls; }
int *__error(void) { return &error_value; }
int kqueue(void) { return 4; }
int kevent(int queue, const struct change *changes, int count, void *events, int capacity, void *timeout) {
    (void)queue; (void)events; (void)capacity; (void)timeout;
    if (calls>=2 || count!=1 || changes->filter != -1-calls || changes->flags!=2) abort();
    error_value=errors[calls++]; return error_value ? -1 : 0;
}
