// SPDX-License-Identifier: MPL-2.0
// A native host keeps another thread alive while Wave terminates the process.
#include <pthread.h>
#include <unistd.h>

extern void wave_exit(void);
static pthread_barrier_t ready;
static int exit_from_worker;

static void *worker(void *unused) {
    (void)unused;
    pthread_barrier_wait(&ready);
    if (exit_from_worker) {
        wave_exit();
    }
    for (;;) {
        pause();
    }
    return NULL;
}

int main(int argc, char **argv) {
    (void)argv;
    exit_from_worker = argc > 1;
    if (pthread_barrier_init(&ready, NULL, 2) != 0) {
        return 90;
    }
    pthread_t thread;
    if (pthread_create(&thread, NULL, worker, NULL) != 0) {
        return 91;
    }
    pthread_barrier_wait(&ready);
    if (!exit_from_worker) {
        wave_exit();
    }
    for (;;) {
        pause();
    }
}
