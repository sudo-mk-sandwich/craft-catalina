/* Compiled by install.sh and linked into Craft Anything on macOS 10.15.
 *
 * CGPreflightScreenCaptureAccess exists only from macOS 11. On newer systems the real function is
 * called; on 10.15 the answer is "allowed" (the permission cannot be checked there this way). */
#include <dlfcn.h>
#include <stdbool.h>

bool CGPreflightScreenCaptureAccess(void) {
  bool (*real)(void) = (bool (*)(void))dlsym(RTLD_NEXT, "CGPreflightScreenCaptureAccess");
  return real ? real() : true;
}
