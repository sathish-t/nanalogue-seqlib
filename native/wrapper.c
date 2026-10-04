#include <stddef.h>
#include <stdint.h>

#include "wrapper.h"

int64_t wrap_bgzf_tell(BGZF *fp)
{
	return bgzf_tell(fp);
}

/*
 * ABI facts about the real headers, as compiled for the current target.
 * tests/native_stack.rs compares every entry, by name, with the hand-written
 * declarations in src/htslib.rs. Add an entry here for every concrete struct
 * field and constant declared there.
 */
struct nanalogue_abi_entry {
	const char *name;
	uint64_t value;
};

#define ABI_SIZE(t) { "size " #t, sizeof(t) }
#define ABI_ALIGN(t) { "align " #t, _Alignof(t) }
#define ABI_OFFSET(t, f) { "offset " #t "." #f, offsetof(t, f) }
#define ABI_VALUE(c) { #c, (uint64_t)(c) }

static const struct nanalogue_abi_entry nanalogue_hts_abi[] = {
	ABI_SIZE(hts_pos_t),
	ABI_SIZE(enum htsLogLevel),
	ABI_SIZE(enum htsExactFormat),
	ABI_SIZE(enum hts_fmt_option),
	ABI_SIZE(enum sam_fields),

	ABI_SIZE(htsFormat),
	ABI_ALIGN(htsFormat),
	ABI_OFFSET(htsFormat, category),
	ABI_OFFSET(htsFormat, format),
	ABI_OFFSET(htsFormat, version),
	ABI_OFFSET(htsFormat, version.minor),
	ABI_OFFSET(htsFormat, compression),
	ABI_OFFSET(htsFormat, compression_level),
	ABI_OFFSET(htsFormat, specific),

	ABI_SIZE(htsThreadPool),
	ABI_ALIGN(htsThreadPool),
	ABI_OFFSET(htsThreadPool, pool),
	ABI_OFFSET(htsThreadPool, qsize),

	ABI_SIZE(sam_hdr_t),
	ABI_ALIGN(sam_hdr_t),
	ABI_OFFSET(sam_hdr_t, n_targets),
	ABI_OFFSET(sam_hdr_t, ignore_sam_err),
	ABI_OFFSET(sam_hdr_t, l_text),
	ABI_OFFSET(sam_hdr_t, target_len),
	ABI_OFFSET(sam_hdr_t, cigar_tab),
	ABI_OFFSET(sam_hdr_t, target_name),
	ABI_OFFSET(sam_hdr_t, text),
	ABI_OFFSET(sam_hdr_t, sdict),
	ABI_OFFSET(sam_hdr_t, hrecs),
	ABI_OFFSET(sam_hdr_t, ref_count),

	ABI_SIZE(bam1_core_t),
	ABI_ALIGN(bam1_core_t),
	ABI_OFFSET(bam1_core_t, pos),
	ABI_OFFSET(bam1_core_t, tid),
	ABI_OFFSET(bam1_core_t, bin),
	ABI_OFFSET(bam1_core_t, qual),
	ABI_OFFSET(bam1_core_t, l_extranul),
	ABI_OFFSET(bam1_core_t, flag),
	ABI_OFFSET(bam1_core_t, l_qname),
	ABI_OFFSET(bam1_core_t, n_cigar),
	ABI_OFFSET(bam1_core_t, l_qseq),
	ABI_OFFSET(bam1_core_t, mtid),
	ABI_OFFSET(bam1_core_t, mpos),
	ABI_OFFSET(bam1_core_t, isize),

	ABI_SIZE(bam1_t),
	ABI_ALIGN(bam1_t),
	ABI_OFFSET(bam1_t, core),
	ABI_OFFSET(bam1_t, id),
	ABI_OFFSET(bam1_t, data),
	ABI_OFFSET(bam1_t, l_data),
	ABI_OFFSET(bam1_t, m_data),

	ABI_VALUE(sam),
	ABI_VALUE(bam),
	ABI_VALUE(cram),
	ABI_VALUE(CRAM_OPT_VERSION),
	ABI_VALUE(CRAM_OPT_EMBED_REF),
	ABI_VALUE(CRAM_OPT_REFERENCE),
	ABI_VALUE(CRAM_OPT_NO_REF),
	ABI_VALUE(CRAM_OPT_REQUIRED_FIELDS),
	ABI_VALUE(HTS_OPT_COMPRESSION_LEVEL),
	ABI_VALUE(SAM_FLAG),
	ABI_VALUE(SAM_RNAME),
	ABI_VALUE(HTS_FEATURE_PLUGINS),
	ABI_VALUE(HTS_FEATURE_LIBCURL),
	ABI_VALUE(HTS_FEATURE_S3),
	ABI_VALUE(HTS_FEATURE_GCS),
	ABI_VALUE(HTS_FEATURE_LIBDEFLATE),
	ABI_VALUE(HTS_FEATURE_LZMA),
	ABI_VALUE(HTS_FEATURE_BZIP2),
	ABI_VALUE(HTS_FEATURE_HTSCODECS),
	ABI_VALUE(BAM_FPAIRED),
	ABI_VALUE(BAM_FPROPER_PAIR),
	ABI_VALUE(BAM_FUNMAP),
	ABI_VALUE(BAM_FMUNMAP),
	ABI_VALUE(BAM_FREVERSE),
	ABI_VALUE(BAM_FMREVERSE),
	ABI_VALUE(BAM_FREAD1),
	ABI_VALUE(BAM_FREAD2),
	ABI_VALUE(BAM_FSECONDARY),
	ABI_VALUE(BAM_FQCFAIL),
	ABI_VALUE(BAM_FDUP),
	ABI_VALUE(BAM_FSUPPLEMENTARY),
};

const struct nanalogue_abi_entry *nanalogue_hts_abi_table(size_t *len)
{
	*len = sizeof(nanalogue_hts_abi) / sizeof(nanalogue_hts_abi[0]);
	return nanalogue_hts_abi;
}
