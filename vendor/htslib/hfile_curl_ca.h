#ifndef HTSLIB_HFILE_CURL_CA_H
#define HTSLIB_HFILE_CURL_CA_H

#include <curl/curl.h>

// Apply explicit and platform-default CA settings to a newly initialized or
// reset easy handle.  Environment variables are read but never modified.
CURLcode hts_curl_configure_ca(CURL *easy);

#endif
