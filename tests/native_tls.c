/* Client for native_tls.py: exercise the actual HTSlib backend, not system curl. */
#undef NDEBUG
#include <assert.h>
#include <string.h>
#include <htslib/sam.h>
#include <openssl/pem.h>
#include <openssl/x509.h>
int main(int argc, char **argv)
{
    const char *names[] = {"I", "II.14978392", "III", "IV", "V", "VI"};
    assert(argc == 3);
    /* Cover generated PEM read/write adapters and both ASN.1 callback paths. */
    BIO *input = BIO_new_file(argv[2], "r");
    X509 *cert = PEM_read_bio_X509_AUX(input, NULL, NULL, NULL);
    BIO *encoded = BIO_new(BIO_s_mem());
    BIO *printed = BIO_new(BIO_s_mem());
    assert(cert && encoded && printed);
    assert(PEM_write_bio_X509_AUX(encoded, cert));
    X509 *copy = PEM_read_bio_X509_AUX(encoded, NULL, NULL, NULL);
    assert(copy && X509_cmp(cert, copy) == 0);
    assert(ASN1_item_print(printed, (const ASN1_VALUE *)copy, 0, ASN1_ITEM_rptr(X509), NULL));
    X509_free(copy);
    X509_free(cert);
    BIO_free(input);
    BIO_free(encoded);
    BIO_free(printed);
    samFile *fp = sam_open(argv[1], "r");
    if (fp == NULL) return 1;
    sam_hdr_t *header = sam_hdr_read(fp);
    bam1_t *record = bam_init1();
    assert(header && record);
    for (unsigned i = 0; i < sizeof(names) / sizeof(*names); i++) {
        assert(sam_read1(fp, header, record) >= 0);
        assert(strcmp(bam_get_qname(record), names[i]) == 0);
    }
    assert(sam_read1(fp, header, record) == -1);
    bam_destroy1(record);
    sam_hdr_destroy(header);
    return sam_close(fp) != 0;
}
