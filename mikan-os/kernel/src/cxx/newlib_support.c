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

extern "C" int close(int fd) {
  return -1;
}

extern "C" off_t lseek(int fd, off_t offset, int whence) {
  errno = EINVAL;
  return (off_t)-1;
}

extern "C" ssize_t read(int fd, void *buf, size_t count) {
  errno = EINVAL;
  return -1;
}

extern "C" ssize_t write(int fd, const void *buf, size_t count) {
  errno = EINVAL;
  return -1;
}

extern "C" int fstat(int fd, struct stat *buf) {
  errno = EINVAL;
  return 0;
}

extern "C" int isatty(int fd) {
  errno = EINVAL;
  return 0;
}
