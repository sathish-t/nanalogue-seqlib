/*  hfile_curl_ca.c -- CA certificate discovery for HTSlib curl backends. */

#include <stdlib.h>

#ifdef __linux__
#include <unistd.h>
#endif

#include "hfile_curl_ca.h"

CURLcode hts_curl_configure_ca(CURL *easy)
{
    const char *bundle = getenv("CURL_CA_BUNDLE");
    CURLcode result;

    // Preserve HTSlib's existing explicit override, including an empty value.
    if (bundle != NULL)
        return curl_easy_setopt(easy, CURLOPT_CAINFO, bundle);

    // OpenSSL interprets these itself.  Do not replace either one with an
    // automatically discovered bundle or alter their independent semantics.
    if (getenv("SSL_CERT_FILE") != NULL || getenv("SSL_CERT_DIR") != NULL)
        return CURLE_OK;

#ifdef __linux__
    {
        static const char * const bundles[] = {
            "/etc/ssl/certs/ca-certificates.crt",
            "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
            "/etc/pki/tls/certs/ca-bundle.crt",
            "/etc/ssl/ca-bundle.pem",
            "/etc/pki/tls/cacert.pem",
            "/etc/ssl/cert.pem",
            "/opt/etc/ssl/certs/ca-certificates.crt",
            "/etc/ssl/certs/cacert.pem"
        };
        size_t i;

        for (i = 0; i < sizeof(bundles) / sizeof(bundles[0]); i++) {
            if (access(bundles[i], R_OK) == 0) {
                result = curl_easy_setopt(easy, CURLOPT_CAINFO, bundles[i]);
                if (result != CURLE_OK)
                    return result;
                return curl_easy_setopt(easy, CURLOPT_PROXY_CAINFO, bundles[i]);
            }
        }
    }
#endif

    // OpenSSL's configured default paths remain the final fallback.  On Apple,
    // curl uses SecTrust when no explicit override was supplied.
    return CURLE_OK;
}
