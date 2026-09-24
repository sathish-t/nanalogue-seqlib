/* Local adapters for the generic modes API. Public AES signatures stay intact. */
#ifndef OSSL_AES_CALLBACKS_H
#define OSSL_AES_CALLBACKS_H
#include <openssl/aes.h>
#include <openssl/e_os2.h>

static ossl_inline void ossl_aes_encrypt_block(const unsigned char *in,
    unsigned char *out, const void *key)
{
    AES_encrypt(in, out, (const AES_KEY *)key);
}

static ossl_inline void ossl_aes_decrypt_block(const unsigned char *in,
    unsigned char *out, const void *key)
{
    AES_decrypt(in, out, (const AES_KEY *)key);
}

static ossl_inline void ossl_aes_cbc(const unsigned char *in, unsigned char *out,
    size_t len, const void *key, unsigned char *iv, int enc)
{
    AES_cbc_encrypt(in, out, len, (const AES_KEY *)key, iv, enc);
}
#endif
