/* HTSlib headers used by native/wrapper.c. src/htslib.rs declares the Rust
 * side of the same API by hand; keep the two in step. */
#include "htslib/hts.h"
#include "htslib/hts_log.h"
#include "htslib/sam.h"
#include "htslib/bgzf.h"
#include "htslib/faidx.h"
#include "htslib/thread_pool.h"

/* bgzf_tell is a macro in htslib/bgzf.h, so it needs a real symbol. */
int64_t wrap_bgzf_tell(BGZF *fp);
