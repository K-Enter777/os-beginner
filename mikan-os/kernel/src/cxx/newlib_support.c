#include <errno.h>
#include <sys/types.h>

extern "C" void _exit(void) {
  while (1) __asm__("hlt");
}

extern "C" caddr_t sbrk(int incr) {
  errno = ENOMEM;
  return (caddr_t)-1;
}

extern "C" int getpid(void) {
  return 1;
}

extern "C" int kill(int pid, int sig) {
  errno = EINVAL;
  return -1;
}
