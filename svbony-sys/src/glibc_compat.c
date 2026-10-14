/*
 * glibc_compat.c — shims for glibc-only symbols referenced by the
 * precompiled SVBony SDK (built against glibc with _FORTIFY_SOURCE=2),
 * so it can be linked statically against musl.
 *
 * Covered: __asprintf_chk __ctype_b_loc __fdelt_chk __fprintf_chk
 *          __isoc99_sscanf __longjmp_chk __memcpy_chk __memset_chk
 *          __printf_chk __snprintf_chk __sprintf_chk __stpcpy_chk
 *          __strcat_chk __strcpy_chk __syslog_chk __vsnprintf_chk
 *          __vsprintf_chk
 *
 * Not defined here: __errno_location. musl already exports it with the
 * same ABI (int *(void)); a second definition would clash.
 *
 * On overflow, the *_chk functions behave like glibc: they print
 * "*** buffer overflow detected ***: terminated" and abort().
 *
 * Must be compiled WITHOUT fortification (-U_FORTIFY_SOURCE
 * -D_FORTIFY_SOURCE=0) so Alpine's fortify-headers don't wrap the
 * libc calls made here.
 */
#define _GNU_SOURCE
#undef _FORTIFY_SOURCE

#include <limits.h>
#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <syslog.h>
#include <unistd.h>

#define EXPORT __attribute__((visibility("default"), used))

/* ------------------------------------------------------------------ */
/* Failure path (equivalent to glibc __chk_fail / __fortify_fail)      */
/* ------------------------------------------------------------------ */

__attribute__((noreturn, cold)) static void chk_fail(void)
{
    static const char msg[] = "*** buffer overflow detected ***: terminated\n";
    /* write(2) rather than stdio: the heap/stdio state may be corrupt. */
    (void)!write(STDERR_FILENO, msg, sizeof msg - 1);
    abort();
}

/*
 * musl's vsnprintf() fails with EOVERFLOW when n > INT_MAX. glibc passes
 * slen == (size_t)-1 when the object size is unknown, so clamp it.
 */
static inline size_t clamp_len(size_t n)
{
    return n > (size_t)INT_MAX ? (size_t)INT_MAX : n;
}

/* ------------------------------------------------------------------ */
/* Memory / strings                                                    */
/* ------------------------------------------------------------------ */

EXPORT void *__memcpy_chk(void *dst, const void *src, size_t n, size_t dstlen)
{
    if (n > dstlen)
        chk_fail();
    return memcpy(dst, src, n);
}

EXPORT void *__memset_chk(void *dst, int c, size_t n, size_t dstlen)
{
    if (n > dstlen)
        chk_fail();
    return memset(dst, c, n);
}

EXPORT char *__strcpy_chk(char *dst, const char *src, size_t dstlen)
{
    size_t n = strlen(src);
    if (n >= dstlen)
        chk_fail();
    return memcpy(dst, src, n + 1);
}

/* Returns a pointer to the terminating NUL written in dst. */
EXPORT char *__stpcpy_chk(char *dst, const char *src, size_t dstlen)
{
    size_t n = strlen(src);
    if (n >= dstlen)
        chk_fail();
    memcpy(dst, src, n + 1);
    return dst + n;
}

EXPORT char *__strcat_chk(char *dst, const char *src, size_t dstlen)
{
    size_t d = strnlen(dst, dstlen);
    if (d == dstlen)            /* dst not even terminated inside its buffer */
        chk_fail();
    size_t n = strlen(src);
    if (n >= dstlen - d)
        chk_fail();
    memcpy(dst + d, src, n + 1);
    return dst;
}

/* ------------------------------------------------------------------ */
/* printf family                                                       */
/* The `flag` argument (glibc: %n restriction level) is ignored.       */
/* ------------------------------------------------------------------ */

EXPORT int __vsprintf_chk(char *s, int flag, size_t slen,
                          const char *fmt, va_list ap)
{
    (void)flag;
    if (slen == 0)
        chk_fail();
    if (slen == (size_t)-1)                 /* unknown size: plain vsprintf */
        return vsprintf(s, fmt, ap);

    size_t cap = clamp_len(slen);
    int r = vsnprintf(s, cap, fmt, ap);
    if (r >= 0 && (size_t)r >= cap)
        chk_fail();                         /* output would not fit */
    return r;
}

EXPORT int __sprintf_chk(char *s, int flag, size_t slen, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    int r = __vsprintf_chk(s, flag, slen, fmt, ap);
    va_end(ap);
    return r;
}

EXPORT int __vsnprintf_chk(char *s, size_t maxlen, int flag, size_t slen,
                           const char *fmt, va_list ap)
{
    (void)flag;
    if (maxlen > slen)
        chk_fail();
    return vsnprintf(s, clamp_len(maxlen), fmt, ap);
}

EXPORT int __snprintf_chk(char *s, size_t maxlen, int flag, size_t slen,
                          const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    int r = __vsnprintf_chk(s, maxlen, flag, slen, fmt, ap);
    va_end(ap);
    return r;
}

EXPORT int __printf_chk(int flag, const char *fmt, ...)
{
    (void)flag;
    va_list ap;
    va_start(ap, fmt);
    int r = vprintf(fmt, ap);
    va_end(ap);
    return r;
}

EXPORT int __fprintf_chk(FILE *fp, int flag, const char *fmt, ...)
{
    (void)flag;
    va_list ap;
    va_start(ap, fmt);
    int r = vfprintf(fp, fmt, ap);
    va_end(ap);
    return r;
}

EXPORT int __asprintf_chk(char **strp, int flag, const char *fmt, ...)
{
    (void)flag;
    va_list ap;
    va_start(ap, fmt);
    int r = vasprintf(strp, fmt, ap);
    va_end(ap);
    return r;
}

EXPORT void __syslog_chk(int prio, int flag, const char *fmt, ...)
{
    (void)flag;
    va_list ap;
    va_start(ap, fmt);
    vsyslog(prio, fmt, ap);
    va_end(ap);
}

/* ------------------------------------------------------------------ */
/* scanf: glibc redirects sscanf to its strict C99 variant             */
/* (%a = float, not a GNU allocation modifier). musl is already C99.   */
/* ------------------------------------------------------------------ */

EXPORT int __isoc99_sscanf(const char *str, const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    int r = vsscanf(str, fmt, ap);
    va_end(ap);
    return r;
}

/* ------------------------------------------------------------------ */
/* select(): FD_SET/FD_CLR/FD_ISSET call __fdelt_chk(fd)               */
/* to compute the fds_bits index.                                      */
/* ------------------------------------------------------------------ */

EXPORT long __fdelt_chk(long d)
{
    if (d < 0 || d >= FD_SETSIZE)
        chk_fail();
    return d / (8 * (long)sizeof(long));    /* __NFDBITS on glibc and musl */
}

/*
 * longjmp: the jmp_buf was filled by musl's setjmp (the SDK's setjmp
 * reference resolves to musl), and musl's jmp_buf is 200 bytes on
 * x86_64, the same size as glibc's. Forwarding is therefore safe.
 * glibc's extra stack check (stack only unwinds upward) is skipped.
 */
EXPORT _Noreturn void __longjmp_chk(jmp_buf env, int val)
{
    longjmp(env, val);
}

/* ------------------------------------------------------------------ */
/* __ctype_b_loc: glibc's isalpha()/isdigit()/... macros read          */
/* (*__ctype_b_loc())[c] and test one bit.                             */
/* The table covers c in [-128, 255] (signed char + EOF + unsigned     */
/* char), so the pointer targets index 128 of a 384-entry array.       */
/*                                                                     */
/* Bit layout: glibc x86_64 (little-endian), from <ctype.h>:           */
/*   _ISbit(b) = b < 8 ? (1 << b) << 8 : (1 << b) >> 8                 */
/* Contents: "C"/POSIX locale (ASCII only; bytes >= 0x80 are 0),       */
/* checked bit-for-bit against glibc 2.39.                             */
/* ------------------------------------------------------------------ */

enum {
    U_  = 0x0100,   /* _ISupper  (bit 0)  */
    L_  = 0x0200,   /* _ISlower  (bit 1)  */
    A_  = 0x0400,   /* _ISalpha  (bit 2)  */
    D_  = 0x0800,   /* _ISdigit  (bit 3)  */
    X_  = 0x1000,   /* _ISxdigit (bit 4)  */
    S_  = 0x2000,   /* _ISspace  (bit 5)  */
    P_  = 0x4000,   /* _ISprint  (bit 6)  */
    G_  = 0x8000,   /* _ISgraph  (bit 7)  */
    B_  = 0x0001,   /* _ISblank  (bit 8)  */
    C_  = 0x0002,   /* _IScntrl  (bit 9)  */
    PU_ = 0x0004,   /* _ISpunct  (bit 10) */
    AN_ = 0x0008,   /* _ISalnum  (bit 11) */
};

#define O 128   /* offset of index 0 */

static const unsigned short ctype_b_table[384] = {
    [O + 0x00 ... O + 0x08] = C_,
    [O + '\t']              = C_ | S_ | B_,
    [O + '\n' ... O + '\r'] = C_ | S_,
    [O + 0x0e ... O + 0x1f] = C_,
    [O + ' ']               = P_ | S_ | B_,
    [O + '!'  ... O + '/']  = P_ | G_ | PU_,
    [O + '0'  ... O + '9']  = P_ | G_ | D_ | X_ | AN_,
    [O + ':'  ... O + '@']  = P_ | G_ | PU_,
    [O + 'A'  ... O + 'F']  = P_ | G_ | A_ | U_ | X_ | AN_,
    [O + 'G'  ... O + 'Z']  = P_ | G_ | A_ | U_ | AN_,
    [O + '['  ... O + '`']  = P_ | G_ | PU_,
    [O + 'a'  ... O + 'f']  = P_ | G_ | A_ | L_ | X_ | AN_,
    [O + 'g'  ... O + 'z']  = P_ | G_ | A_ | L_ | AN_,
    [O + '{'  ... O + '~']  = P_ | G_ | PU_,
    [O + 0x7f]              = C_,
    /* [-128..-1] and [128..255]: 0 */
};

#undef O

static const unsigned short *const ctype_b_ptr = ctype_b_table + 128;

/* The table is const and locale-independent, so it's thread-safe. */
EXPORT const unsigned short **__ctype_b_loc(void)
{
    return (const unsigned short **)&ctype_b_ptr;
}
