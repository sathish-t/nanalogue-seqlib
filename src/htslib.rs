//! Raw FFI declarations for the small part of the bundled HTSlib that this
//! crate (and its downstream users) call.
//!
//! These declarations are written by hand from the vendored headers in
//! `vendor/htslib/htslib/*.h`; there is no bindgen step. One file serves every
//! supported target because the declarations use only fixed-width integers,
//! `usize` for `size_t`, and pointers.
//!
//! Every struct layout and constant value below is checked against the C
//! compiler's view of the real headers by `tests/native_stack.rs`, which
//! compares them with the `nanalogue_hts_abi` table compiled from
//! `native/wrapper.c`. When adding anything here:
//!
//! * copy the prototype or definition from the header it is declared in;
//! * keep structs whose fields Rust never reads opaque;
//! * add every new concrete struct field and constant to both the C table and
//!   the Rust test.
#![allow(non_camel_case_types, non_upper_case_globals)]

use libc::{c_char, c_int, c_short, c_uint, c_void};

// ---------------------------------------------------------------------------
// Opaque types: only ever handled through pointers.
// ---------------------------------------------------------------------------

macro_rules! opaque {
    ($($(#[$doc:meta])* $name:ident;)*) => {$(
        $(#[$doc])*
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        pub struct $name {
            _unused: [u8; 0],
        }
    )*};
}

opaque! {
    /// An open SAM/BAM/CRAM file (`htslib/hts.h`). Use [`hts_get_format`] and
    /// [`hts_get_bgzfp`] rather than reading its fields.
    htsFile;
    /// A BGZF stream (`htslib/bgzf.h`).
    BGZF;
    /// A loaded index (`htslib/hts.h`).
    hts_idx_t;
    /// A region iterator (`htslib/hts.h`).
    hts_itr_t;
    /// An HTSlib worker thread pool (`htslib/thread_pool.h`).
    hts_tpool;
    /// Parsed header records, owned by [`sam_hdr_t`] (`htslib/sam.h`).
    sam_hrecs_t;
}

/// `samFile` is a typedef of `htsFile` (`htslib/sam.h`).
pub type samFile = htsFile;

// ---------------------------------------------------------------------------
// Scalar typedefs and enums. C enums are `unsigned int` on every supported
// target, matching the values below.
// ---------------------------------------------------------------------------

/// `hts_pos_t` (`htslib/hts.h`).
pub type hts_pos_t = i64;

/// `enum htsLogLevel` (`htslib/hts_log.h`).
pub type htsLogLevel = c_uint;

/// `enum htsFormatCategory` (`htslib/hts.h`).
pub type htsFormatCategory = c_uint;

/// `enum htsExactFormat` (`htslib/hts.h`).
pub type htsExactFormat = c_uint;
pub const htsExactFormat_sam: htsExactFormat = 3;
pub const htsExactFormat_bam: htsExactFormat = 4;
pub const htsExactFormat_cram: htsExactFormat = 6;

/// `enum htsCompression` (`htslib/hts.h`).
pub type htsCompression = c_uint;

/// `enum hts_fmt_option` (`htslib/hts.h`).
pub type hts_fmt_option = c_uint;
pub const hts_fmt_option_CRAM_OPT_VERSION: hts_fmt_option = 6;
pub const hts_fmt_option_CRAM_OPT_EMBED_REF: hts_fmt_option = 7;
pub const hts_fmt_option_CRAM_OPT_REFERENCE: hts_fmt_option = 9;
pub const hts_fmt_option_CRAM_OPT_NO_REF: hts_fmt_option = 11;
pub const hts_fmt_option_CRAM_OPT_REQUIRED_FIELDS: hts_fmt_option = 18;
pub const hts_fmt_option_HTS_OPT_COMPRESSION_LEVEL: hts_fmt_option = 100;

/// `enum sam_fields` (`htslib/sam.h`).
pub type sam_fields = c_uint;
pub const sam_fields_SAM_FLAG: sam_fields = 2;
pub const sam_fields_SAM_RNAME: sam_fields = 4;

// `hts_features()` bits (`htslib/hts.h`).
pub const HTS_FEATURE_PLUGINS: u32 = 2;
pub const HTS_FEATURE_LIBCURL: u32 = 1024;
pub const HTS_FEATURE_S3: u32 = 2048;
pub const HTS_FEATURE_GCS: u32 = 4096;
pub const HTS_FEATURE_LIBDEFLATE: u32 = 1048576;
pub const HTS_FEATURE_LZMA: u32 = 2097152;
pub const HTS_FEATURE_BZIP2: u32 = 4194304;
pub const HTS_FEATURE_HTSCODECS: u32 = 8388608;

// SAM FLAG bits (`htslib/sam.h`).
pub const BAM_FPAIRED: u32 = 1;
pub const BAM_FPROPER_PAIR: u32 = 2;
pub const BAM_FUNMAP: u32 = 4;
pub const BAM_FMUNMAP: u32 = 8;
pub const BAM_FREVERSE: u32 = 16;
pub const BAM_FMREVERSE: u32 = 32;
pub const BAM_FREAD1: u32 = 64;
pub const BAM_FREAD2: u32 = 128;
pub const BAM_FSECONDARY: u32 = 256;
pub const BAM_FQCFAIL: u32 = 512;
pub const BAM_FDUP: u32 = 1024;
pub const BAM_FSUPPLEMENTARY: u32 = 2048;

// ---------------------------------------------------------------------------
// Concrete structs: Rust reads or writes their fields directly.
// ---------------------------------------------------------------------------

/// `htsFormat.version` (`htslib/hts.h`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct htsFormat_version {
    pub major: c_short,
    pub minor: c_short,
}

/// `htsFormat` (`htslib/hts.h`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct htsFormat {
    pub category: htsFormatCategory,
    pub format: htsExactFormat,
    pub version: htsFormat_version,
    pub compression: htsCompression,
    pub compression_level: c_short,
    pub specific: *mut c_void,
}

/// `htsThreadPool` (`htslib/hts.h`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct htsThreadPool {
    pub pool: *mut hts_tpool,
    pub qsize: c_int,
}

/// `sam_hdr_t` (`htslib/sam.h`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct sam_hdr_t {
    pub n_targets: i32,
    pub ignore_sam_err: i32,
    pub l_text: usize,
    pub target_len: *mut u32,
    pub cigar_tab: *const i8,
    pub target_name: *mut *mut c_char,
    pub text: *mut c_char,
    pub sdict: *mut c_void,
    pub hrecs: *mut sam_hrecs_t,
    pub ref_count: u32,
}

/// `bam_hdr_t` is the older name of `sam_hdr_t` (`htslib/sam.h`).
pub type bam_hdr_t = sam_hdr_t;

/// `bam1_core_t` (`htslib/sam.h`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bam1_core_t {
    pub pos: hts_pos_t,
    pub tid: i32,
    pub bin: u16,
    pub qual: u8,
    pub l_extranul: u8,
    pub flag: u16,
    pub l_qname: u16,
    pub n_cigar: u32,
    pub l_qseq: i32,
    pub mtid: i32,
    pub mpos: hts_pos_t,
    /// The C field `isize` (named `isize_` for compatibility with earlier
    /// bindgen-generated releases).
    pub isize_: hts_pos_t,
}

/// `bam1_t` (`htslib/sam.h`).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bam1_t {
    pub core: bam1_core_t,
    pub id: u64,
    pub data: *mut u8,
    pub l_data: c_int,
    pub m_data: u32,
    /// The C bitfield `uint32_t mempolicy:2, :30`. HTSlib owns these bits;
    /// this crate leaves them zeroed.
    pub mempolicy_bits: u32,
}

// ---------------------------------------------------------------------------
// Functions.
// ---------------------------------------------------------------------------

extern "C" {
    // htslib/hts_log.h
    pub fn hts_set_log_level(level: htsLogLevel);

    // htslib/hts.h
    pub fn hts_features() -> c_uint;
    pub fn hts_open(fn_: *const c_char, mode: *const c_char) -> *mut htsFile;
    pub fn hts_close(fp: *mut htsFile) -> c_int;
    pub fn hts_get_format(fp: *mut htsFile) -> *const htsFormat;
    pub fn hts_set_opt(fp: *mut htsFile, opt: hts_fmt_option, ...) -> c_int;
    pub fn hts_set_threads(fp: *mut htsFile, n: c_int) -> c_int;
    pub fn hts_set_thread_pool(fp: *mut htsFile, p: *mut htsThreadPool) -> c_int;
    pub fn hts_set_fai_filename(fp: *mut htsFile, fn_aux: *const c_char) -> c_int;
    pub fn hts_idx_destroy(idx: *mut hts_idx_t);
    pub fn hts_idx_get_stat(
        idx: *const hts_idx_t,
        tid: c_int,
        mapped: *mut u64,
        unmapped: *mut u64,
    ) -> c_int;
    pub fn hts_idx_get_n_no_coor(idx: *const hts_idx_t) -> u64;
    pub fn hts_itr_destroy(iter: *mut hts_itr_t);
    pub fn hts_itr_next(
        fp: *mut BGZF,
        iter: *mut hts_itr_t,
        r: *mut c_void,
        data: *mut c_void,
    ) -> c_int;

    // htslib/tbx.h declares this accessor; it is defined in hts.c.
    pub fn hts_get_bgzfp(fp: *mut htsFile) -> *mut BGZF;

    // htslib/bgzf.h
    pub fn bgzf_seek(fp: *mut BGZF, pos: i64, whence: c_int) -> i64;

    // htslib/thread_pool.h
    pub fn hts_tpool_init(n: c_int) -> *mut hts_tpool;
    pub fn hts_tpool_destroy(p: *mut hts_tpool);

    // htslib/faidx.h
    pub fn fai_build(fn_: *const c_char) -> c_int;

    // htslib/sam.h: headers
    pub fn sam_hdr_read(fp: *mut samFile) -> *mut sam_hdr_t;
    pub fn sam_hdr_write(fp: *mut samFile, h: *const sam_hdr_t) -> c_int;
    pub fn sam_hdr_destroy(h: *mut sam_hdr_t);
    pub fn sam_hdr_dup(h0: *const sam_hdr_t) -> *mut sam_hdr_t;
    pub fn sam_hdr_parse(l_text: usize, text: *const c_char) -> *mut sam_hdr_t;
    pub fn sam_hdr_str(h: *mut sam_hdr_t) -> *const c_char;
    pub fn sam_hdr_length(h: *mut sam_hdr_t) -> usize;
    pub fn sam_hdr_nref(h: *const sam_hdr_t) -> c_int;
    pub fn sam_hdr_tid2name(h: *const sam_hdr_t, tid: c_int) -> *const c_char;
    pub fn sam_hdr_name2tid(h: *mut sam_hdr_t, ref_: *const c_char) -> c_int;
    pub fn sam_hdr_add_lines(h: *mut sam_hdr_t, lines: *const c_char, len: usize) -> c_int;
    pub fn sam_hdr_count_lines(h: *mut sam_hdr_t, type_: *const c_char) -> c_int;
    pub fn sam_hdr_line_name(bh: *mut sam_hdr_t, type_: *const c_char, pos: c_int)
        -> *const c_char;

    // htslib/sam.h: records
    pub fn sam_read1(fp: *mut samFile, h: *mut sam_hdr_t, b: *mut bam1_t) -> c_int;
    pub fn sam_write1(fp: *mut samFile, h: *const sam_hdr_t, b: *const bam1_t) -> c_int;
    pub fn bam_endpos(b: *const bam1_t) -> hts_pos_t;
    pub fn bam_aux_get(b: *const bam1_t, tag: *const c_char) -> *mut u8;
    pub fn bam_aux_append(
        b: *mut bam1_t,
        tag: *const c_char,
        type_: c_char,
        len: c_int,
        data: *const u8,
    ) -> c_int;
    pub fn bam_aux_del(b: *mut bam1_t, s: *mut u8) -> c_int;
    pub fn bam_aux_update_array(
        b: *mut bam1_t,
        tag: *const c_char,
        type_: u8,
        items: u32,
        data: *mut c_void,
    ) -> c_int;

    // htslib/sam.h: indexes and iterators
    pub fn sam_index_load(fp: *mut htsFile, fn_: *const c_char) -> *mut hts_idx_t;
    pub fn sam_index_load2(
        fp: *mut htsFile,
        fn_: *const c_char,
        fnidx: *const c_char,
    ) -> *mut hts_idx_t;
    pub fn sam_index_build3(
        fn_: *const c_char,
        fnidx: *const c_char,
        min_shift: c_int,
        nthreads: c_int,
    ) -> c_int;
    pub fn sam_idx_init(
        fp: *mut htsFile,
        h: *mut sam_hdr_t,
        min_shift: c_int,
        fnidx: *const c_char,
    ) -> c_int;
    pub fn sam_idx_save(fp: *mut htsFile) -> c_int;
    pub fn sam_itr_queryi(
        idx: *const hts_idx_t,
        tid: c_int,
        beg: hts_pos_t,
        end: hts_pos_t,
    ) -> *mut hts_itr_t;
    pub fn sam_itr_querys(
        idx: *const hts_idx_t,
        hdr: *mut sam_hdr_t,
        region: *const c_char,
    ) -> *mut hts_itr_t;

    // native/wrapper.c: `bgzf_tell` is a macro in htslib/bgzf.h.
    pub fn wrap_bgzf_tell(fp: *mut BGZF) -> i64;
}
