/* Compile/link contract probe and local transfer client. No system libcurl. */
#include "curl_config.h"
#undef NDEBUG
#include <assert.h>
#include <errno.h>
#include <netdb.h>
#include <poll.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <time.h>
#include <curl/curl.h>
#include <openssl/opensslv.h>
#include <zlib.h>
#ifdef CHECK_HTSLIB
#include <htslib/hts.h>
#endif

_Static_assert(sizeof(long) == SIZEOF_LONG, "long ABI");
_Static_assert(sizeof(size_t) == SIZEOF_SIZE_T, "size_t ABI");
_Static_assert(sizeof(off_t) == SIZEOF_OFF_T, "off_t ABI");
_Static_assert(sizeof(time_t) == SIZEOF_TIME_T, "time_t ABI");
_Static_assert(sizeof(curl_off_t) == SIZEOF_CURL_OFF_T, "curl_off_t ABI");
_Static_assert(sizeof(curl_socket_t) == SIZEOF_CURL_SOCKET_T, "socket ABI");
#ifdef HAVE_GLIBC_STRERROR_R
_Static_assert(_Generic(&strerror_r, char *(*)(int, char *, size_t): 1, default: 0), "GNU strerror_r ABI");
#else
_Static_assert(_Generic(&strerror_r, int (*)(int, char *, size_t): 1, default: 0), "POSIX strerror_r ABI");
#endif
#ifdef HAVE_GETHOSTBYNAME_R_6
_Static_assert(_Generic(&gethostbyname_r, int (*)(const char *, struct hostent *, char *, size_t, struct hostent **, int *): 1, default: 0), "gethostbyname_r ABI");
#endif
extern const char *curlx_strerror(int, char *, size_t);

static char body[8192];
static size_t used;
static size_t receive(char *p, size_t size, size_t n, void *unused)
{
    size_t len = size * n;
    (void)unused;
    assert(len < sizeof(body) - used);
    memcpy(body + used, p, len);
    used += len;
    return len;
}

static void *wake(void *multi)
{
    struct timespec delay = {0, 50000000};
    nanosleep(&delay, NULL);
    assert(curl_multi_wakeup(multi) == CURLM_OK);
    return NULL;
}

int main(int argc, char **argv)
{
    char error[256];
#ifdef CHECK_HTSLIB
    assert(!strcmp(hts_version(), "1.24"));
    const unsigned hts_required = HTS_FEATURE_LIBCURL | HTS_FEATURE_S3 | HTS_FEATURE_GCS |
        HTS_FEATURE_LIBDEFLATE | HTS_FEATURE_LZMA | HTS_FEATURE_BZIP2;
    assert((hts_features() & hts_required) == hts_required);
#endif
    assert(curl_global_init(CURL_GLOBAL_DEFAULT) == CURLE_OK);
    const curl_version_info_data *v = curl_version_info(CURLVERSION_NOW);
    const char *protocols[] = {"ftp", "ftps", "http", "https", NULL};
    assert(!strcmp(v->version, "8.22.0-DEV") && v->version_num == 0x081600);
    assert(!strcmp(v->ssl_version, "OpenSSL/" OPENSSL_VERSION_STR));
    assert(!strcmp(v->libz_version, ZLIB_VERSION));
    assert(v->ares == NULL && v->libidn == NULL && v->libssh_version == NULL);
    for (size_t i = 0; protocols[i]; ++i) assert(v->protocols[i] && !strcmp(protocols[i], v->protocols[i]));
    assert(v->protocols[4] == NULL);
    const unsigned required = CURL_VERSION_IPV6 | CURL_VERSION_SSL | CURL_VERSION_LIBZ |
        CURL_VERSION_ASYNCHDNS | CURL_VERSION_LARGEFILE | CURL_VERSION_THREADSAFE |
        CURL_VERSION_UNIX_SOCKETS | CURL_VERSION_HTTPS_PROXY;
    assert((v->features & required) == required);
    const unsigned disabled = CURL_VERSION_HTTP2 | CURL_VERSION_HTTP3 | CURL_VERSION_BROTLI |
        CURL_VERSION_ZSTD | CURL_VERSION_PSL | CURL_VERSION_IDN | CURL_VERSION_GSSAPI |
        CURL_VERSION_SPNEGO | CURL_VERSION_NTLM;
    assert(!(v->features & disabled));
    assert(!strcmp(curlx_strerror(EACCES, error, sizeof(error)), strerror(EACCES)));
    CURLM *multi = curl_multi_init();
    pthread_t thread;
    struct timespec start, end;
    int numfds = -1;
    assert(multi && !clock_gettime(CLOCK_MONOTONIC, &start));
    assert(!pthread_create(&thread, NULL, wake, multi));
    assert(curl_multi_poll(multi, NULL, 0, 10000, &numfds) == CURLM_OK);
    assert(!clock_gettime(CLOCK_MONOTONIC, &end));
    assert(!pthread_join(thread, NULL));
    assert(numfds == 0 && end.tv_sec - start.tv_sec < 5);
    assert(curl_multi_cleanup(multi) == CURLM_OK);
    puts(curl_version());
    for (const char *const *f = v->feature_names; *f; ++f) printf("%s ", *f);
    puts("");
    CURL *defaults = curl_easy_init();
    char *ca = NULL;
    assert(defaults);
    assert(!curl_easy_getinfo(defaults, CURLINFO_CAINFO, &ca) && ca == NULL);
    assert(!curl_easy_getinfo(defaults, CURLINFO_CAPATH, &ca) && ca == NULL);
    curl_easy_cleanup(defaults);
    if (argc > 1) {
        assert(argc == 6);
        CURL *easy = curl_easy_init();
        assert(easy);
        assert(!curl_easy_setopt(easy, CURLOPT_URL, argv[1]));
        assert(!curl_easy_setopt(easy, CURLOPT_PROXY, ""));
        assert(!curl_easy_setopt(easy, CURLOPT_TIMEOUT, 10L));
        assert(!curl_easy_setopt(easy, CURLOPT_WRITEFUNCTION, receive));
        assert(!curl_easy_setopt(easy, CURLOPT_FOLLOWLOCATION, 1L));
        assert(!curl_easy_setopt(easy, CURLOPT_COOKIEFILE, ""));
        assert(!curl_easy_setopt(easy, CURLOPT_ACCEPT_ENCODING, ""));
        assert(!curl_easy_setopt(easy, CURLOPT_USERNAME, "user"));
        assert(!curl_easy_setopt(easy, CURLOPT_PASSWORD, "password"));
        if (strcmp(argv[3], "-")) assert(!curl_easy_setopt(easy, CURLOPT_CAINFO, argv[3]));
        if (strcmp(argv[4], "-")) assert(!curl_easy_setopt(easy, CURLOPT_RANGE, argv[4]));
        if (!strcmp(argv[5], "tls")) assert(!curl_easy_setopt(easy, CURLOPT_USE_SSL, (long)CURLUSESSL_ALL));
        CURLcode result = curl_easy_perform(easy);
        if (result) fprintf(stderr, "curl result %d: %s\n", result, curl_easy_strerror(result));
        curl_easy_cleanup(easy);
        if (result) return (int)result;
        assert(used == strlen(argv[2]) && !memcmp(body, argv[2], used));
    }
    curl_global_cleanup();
    return 0;
}
