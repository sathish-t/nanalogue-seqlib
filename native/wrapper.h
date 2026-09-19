#include "htslib/hts.h"
#include "htslib/vcf.h"
#include "htslib/sam.h"
#include "htslib/hfile.h"
#include "htslib/cram.h"
#include "htslib/bgzf.h"
#include "htslib/vcfutils.h"
#include "htslib/tbx.h"
#include "htslib/synced_bcf_reader.h"
#include "htslib/kbitset.h"
#include "htslib/faidx.h"
#include "htslib/thread_pool.h"

// The following functions have to be wrapped here because they are inline in htslib.

/**
 * <div rustbindgen replaces="kbs_init2"></div>
 */
kbitset_t *wrap_kbs_init2(size_t ni, int fill);

/**
 * <div rustbindgen replaces="kbs_init"></div>
 */
kbitset_t *wrap_kbs_init(size_t ni);


/**
 * <div rustbindgen replaces="kbs_insert"></div>
 */
void wrap_kbs_insert(kbitset_t *bs, int i);


/**
 * <div rustbindgen replaces="kbs_destroy"></div>
 */
void wrap_kbs_destroy(kbitset_t *bs);
