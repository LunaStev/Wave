// SPDX-License-Identifier: MPL-2.0
// Repeated EINTR must consume one deadline, without changing an alias's flags.
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

extern int wave_timed_read(int64_t fd);
static volatile sig_atomic_t interruptions;
static void interrupt_read(int signal_number) {
    (void)signal_number;
    ++interruptions;
}

int main(void) {
    int sockets[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, sockets) != 0) {
        return 10;
    }
    int alias = dup(sockets[0]);
    if (alias < 0) {
        return 11;
    }
    struct sigaction action = {0};
    action.sa_handler = interrupt_read;
    sigemptyset(&action.sa_mask);
    if (sigaction(SIGALRM, &action, NULL) != 0) {
        return 12;
    }
    for (int nonblocking = 0; nonblocking < 2; ++nonblocking) {
        int flags = fcntl(alias, F_GETFL);
        if (flags < 0 || fcntl(alias, F_SETFL, flags | (nonblocking ? O_NONBLOCK : 0)) < 0) {
            return 13;
        }
        flags = fcntl(alias, F_GETFL);
        struct itimerval timer = {{0, 5000}, {0, 5000}};
        struct timespec start, end;
        clock_gettime(CLOCK_MONOTONIC, &start);
        if (setitimer(ITIMER_REAL, &timer, NULL) != 0) {
            return 14;
        }
        int result = wave_timed_read(sockets[0]);
        timer.it_interval.tv_usec = timer.it_value.tv_usec = 0;
        setitimer(ITIMER_REAL, &timer, NULL);
        clock_gettime(CLOCK_MONOTONIC, &end);
        int64_t elapsed = (end.tv_sec - start.tv_sec) * INT64_C(1000000000)
                        + end.tv_nsec - start.tv_nsec;
        if (result != 0 || elapsed < 30000000 || elapsed > 2000000000 || interruptions < 2) {
            return 15;
        }
        if (fcntl(alias, F_GETFL) != flags || fcntl(sockets[0], F_GETFL) != flags) {
            return 16;
        }
    }
    close(alias);
    close(sockets[0]);
    close(sockets[1]);
    return 0;
}
