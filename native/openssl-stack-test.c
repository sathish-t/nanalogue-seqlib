/* Local regression test for OpenSSL's typed callback boundary. */
#undef NDEBUG
#include <assert.h>
#include <pthread.h>
#include <stdlib.h>
#include <string.h>
#include <openssl/safestack.h>
#include <openssl/provider.h>
#include <openssl/crypto.h>
#include <openssl/evp.h>
#include <openssl/kdf.h>
#include <openssl/ssl.h>

#define CHECK_ENUMERATION(type) \
    static void visit_##type(type *method, void *arg) \
    { \
        assert(method != NULL && type##_get0_name(method) != NULL); \
        (*(unsigned *)arg)++; \
    } \
    static void check_##type(void) \
    { \
        unsigned count = 0; \
        type##_do_all_provided(NULL, visit_##type, &count); \
        assert(count > 0); \
    }
CHECK_ENUMERATION(EVP_MD)
CHECK_ENUMERATION(EVP_CIPHER)
CHECK_ENUMERATION(EVP_MAC)
CHECK_ENUMERATION(EVP_KDF)
CHECK_ENUMERATION(EVP_RAND)
CHECK_ENUMERATION(EVP_KEYMGMT)
CHECK_ENUMERATION(EVP_KEYEXCH)
CHECK_ENUMERATION(EVP_SIGNATURE)
CHECK_ENUMERATION(EVP_ASYM_CIPHER)
CHECK_ENUMERATION(EVP_KEM)
CHECK_ENUMERATION(EVP_SKEYMGMT)
#undef CHECK_ENUMERATION

typedef struct { int value; } Item;
DEFINE_STACK_OF(Item)

static int ascending(const Item *const *a, const Item *const *b)
{
    return ((*a)->value > (*b)->value) - ((*a)->value < (*b)->value);
}
static int descending(const Item *const *a, const Item *const *b)
{
    return -ascending(a, b);
}
static int raw_compare(const void *a, const void *b)
{
    const Item *x = *(const void *const *)a, *y = *(const void *const *)b;
    return (x->value > y->value) - (x->value < y->value);
}

static STACK_OF(Item) *nested;
static int nested_compare(const Item *const *a, const Item *const *b)
{
    sk_Item_sort(nested);
    return ascending(a, b);
}

/* Enumerate sizes, duplicates and both ordering directions. Expected values
 * come from a histogram, not from the sorting/search implementation. */
static void *exercise_sort(void *unused)
{
    (void)unused;
    for (int n = 0; n < 65; n++) {
        Item items[64];
        int counts[11] = {0}, offset = 0;
        STACK_OF(Item) *s = sk_Item_new_reserve(ascending, n);
        assert(s != NULL);
        for (int i = 0; i < n; i++) {
            items[i].value = (i * 7 + n * 3) % 11;
            counts[items[i].value]++;
            assert(sk_Item_push(s, &items[i]) == i + 1);
        }
        /* Unsorted search must retain insertion order. */
        if (n > 0) assert(sk_Item_find(s, &items[0]) == 0);
        sk_Item_sort(s);
        assert(sk_Item_is_sorted(s));
        for (int k = 0; k < 11; k++) {
            Item key = {k};
            int count = 0;
            assert(sk_Item_find_all(s, &key, &count) == (counts[k] ? offset : -1));
            assert(count == counts[k]);
            for (int j = 0; j < counts[k]; j++)
                assert(sk_Item_value(s, offset + j)->value == k);
            offset += counts[k];
        }
        STACK_OF(Item) *dup = sk_Item_dup(s);
        assert(dup != NULL);
        assert(sk_Item_set_cmp_func(dup, descending) == ascending);
        sk_Item_sort(dup);
        for (int i = 0; i < n; i++)
            assert(sk_Item_value(dup, i)->value == sk_Item_value(s, n - 1 - i)->value);
        sk_Item_free(dup);
        sk_Item_free(s);
    }
    return NULL;
}

static int copies, frees, fail_at;
static Item *copy_item(const Item *p)
{
    if (++copies == fail_at) return NULL;
    Item *out = malloc(sizeof(*out));
    assert(out != NULL);
    *out = *p;
    return out;
}
static void free_item(Item *p)
{
    assert(p != NULL);
    frees++;
    free(p);
}
static int string_compare(const char *const *a, const char *const *b)
{
    return strcmp(*a, *b);
}
static char *copy_string(const char *p)
{
    if (++copies == fail_at) return NULL;
    char *out = malloc(strlen(p) + 1);
    assert(out != NULL);
    return strcpy(out, p);
}
static void free_string(char *p)
{
    frees++;
    free(p);
}

int main(void)
{
    Item items[] = {{9}, {2}, {7}, {2}};
    STACK_OF(Item) *s = sk_Item_new_null();
    assert(s != NULL);
    for (int i = 0; i < 4; i++) assert(sk_Item_push(s, &items[i]));
    assert(sk_Item_insert(s, NULL, 1));
    /* NULL slots must not call copy/free. Every partial failure frees exactly
     * the already copied objects; the borrowed original remains untouched. */
    for (fail_at = 1; fail_at <= 5; fail_at++) {
        copies = frees = 0;
        STACK_OF(Item) *copy = sk_Item_deep_copy(s, copy_item, free_item);
        if (fail_at <= 4) {
            assert(copy == NULL && copies == fail_at && frees == fail_at - 1);
        } else {
            assert(copy && copies == 4 && frees == 0 && sk_Item_value(copy, 1) == NULL);
            assert(sk_Item_value(copy, 0) != &items[0]);
            assert(sk_Item_value(copy, 0)->value == 9);
            sk_Item_pop_free(copy, free_item);
            assert(frees == 4);
        }
        assert(sk_Item_value(s, 0) == &items[0] && items[0].value == 9);
    }
    sk_Item_delete(s, 1);
    nested = sk_Item_new(descending);
    for (int i = 0; i < 4; i++) assert(sk_Item_push(nested, &items[i]));
    assert(sk_Item_set_cmp_func(s, nested_compare) == NULL);
    sk_Item_sort(s);
    assert(sk_Item_value(s, 0)->value == 2 && sk_Item_value(nested, 0)->value == 9);
    sk_Item_free(nested);
    /* Raw comparator replacement must clear the typed adapter. */
    OPENSSL_sk_set_cmp_func((OPENSSL_STACK *)s, raw_compare);
    sk_Item_sort(s);
    assert(sk_Item_find(s, &items[0]) == 3);
    sk_Item_free(s);

    /* The generated string macros are a distinct wrapper family. */
    STACK_OF(OPENSSL_STRING) *strings = sk_OPENSSL_STRING_new_reserve(string_compare, 4);
    assert(strings);
    assert(sk_OPENSSL_STRING_push(strings, "zulu"));
    assert(sk_OPENSSL_STRING_push(strings, "alpha"));
    assert(sk_OPENSSL_STRING_push(strings, "echo"));
    sk_OPENSSL_STRING_sort(strings);
    assert(sk_OPENSSL_STRING_find(strings, "echo") == 1);
    assert(sk_OPENSSL_STRING_find_ex(strings, "beta") == 0);
    assert(sk_OPENSSL_STRING_find_ex(strings, "foxtrot") == 2);
    assert(sk_OPENSSL_STRING_find_ex(strings, "aardvark") == 0);
    assert(sk_OPENSSL_STRING_find_ex(strings, "zz") == 2);
    assert(sk_OPENSSL_STRING_set_cmp_func(strings, string_compare) == string_compare);
    for (fail_at = 1; fail_at <= 4; fail_at++) {
        copies = frees = 0;
        STACK_OF(OPENSSL_STRING) *copy = sk_OPENSSL_STRING_deep_copy(strings, copy_string, free_string);
        if (fail_at <= 3) assert(copy == NULL && frees == fail_at - 1);
        else {
            assert(copy && strcmp(sk_OPENSSL_STRING_value(copy, 1), "echo") == 0);
            sk_OPENSSL_STRING_pop_free(copy, free_string);
            assert(frees == 3);
        }
    }
    sk_OPENSSL_STRING_free(strings);
    sk_Item_pop_free(sk_Item_dup(NULL), free_item);
    sk_Item_pop_free(sk_Item_deep_copy(NULL, copy_item, free_item), free_item);
    sk_Item_pop_free(NULL, free_item);
    pthread_t threads[2];
    for (int i = 0; i < 2; i++) assert(pthread_create(&threads[i], NULL, exercise_sort, NULL) == 0);
    for (int i = 0; i < 2; i++) assert(pthread_join(threads[i], NULL) == 0);
    assert(OPENSSL_version_major() == 3 && OPENSSL_version_minor() == 6 && OPENSSL_version_patch() == 4);
    OSSL_PROVIDER *def = OSSL_PROVIDER_load(NULL, "default");
    OSSL_PROVIDER *base = OSSL_PROVIDER_load(NULL, "base");
    assert(def && base);
    SSL_CTX *tls = SSL_CTX_new(TLS_client_method());
    assert(tls != NULL);
    SSL_CTX_free(tls);
    check_EVP_MD();
    check_EVP_CIPHER();
    check_EVP_MAC();
    check_EVP_KDF();
    check_EVP_RAND();
    check_EVP_KEYMGMT();
    check_EVP_KEYEXCH();
    check_EVP_SIGNATURE();
    check_EVP_ASYM_CIPHER();
    check_EVP_KEM();
    check_EVP_SKEYMGMT();
    assert(OSSL_PROVIDER_unload(base));
    assert(OSSL_PROVIDER_unload(def));
    return 0;
}
