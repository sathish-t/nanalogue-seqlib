/* The browser viewer is single-threaded; WASI has no pthread signals. */
#include <pthread.h>
#include <errno.h>
static inline int pthread_kill(pthread_t thread, int signal) {
    (void)thread;
    (void)signal;
    return ENOSYS;
}
