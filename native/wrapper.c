#include "wrapper.h"

/* Independently check the ABI of the checked-in bindings in integration tests. */
const size_t nanalogue_hts_abi[12] = {
    sizeof(bam1_t), _Alignof(bam1_t), offsetof(bam1_t, data),
    offsetof(bam1_t, l_data), sizeof(bam1_core_t), offsetof(bam1_core_t, pos),
    offsetof(bam1_core_t, n_cigar), sizeof(sam_hdr_t),
    offsetof(sam_hdr_t, target_len), sizeof(htsFile), offsetof(htsFile, fp),
    sizeof(htsFormat)
};

kbitset_t *wrap_kbs_init2(size_t ni, int fill)
{
	return kbs_init2(ni, fill);
}

kbitset_t *wrap_kbs_init(size_t ni)
{
	return wrap_kbs_init2(ni, 0);
}

void wrap_kbs_insert(kbitset_t *bs, int i)
{
  kbs_insert(bs, i);
}

void wrap_kbs_destroy(kbitset_t *bs)
{
  kbs_destroy(bs);
}
