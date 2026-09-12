use std::path::PathBuf;
use thiserror::Error;

/// Generic result type for functions in this crate with
/// a global error class.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    // General errors
    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf },
    #[error("file could not be opened: {path}")]
    FileOpen { path: String },
    #[error("invalid (non-unicode) characters in path")]
    NonUnicodePath,
    #[error("failed to fetch region")]
    Fetch,
    #[error("error seeking to file offset")]
    FileSeek,
    #[error("error setting threads for file reading")]
    SetThreads,
    #[error("failed to create htslib thread pool")]
    ThreadPool,

    #[error("failed to write BAM/BCF record (out of disk space?)")]
    WriteRecord,
    #[error("failed to write SAM/BAM/CRAM header")]
    WriteHeader,
    #[error("failed to close SAM/BAM/CRAM writer")]
    WriteClose,

    // Errors for faidx
    #[error("failed to build index for fasta file {path:?}")]
    FaidxBuildFailed { path: std::path::PathBuf },

    // Errors for BAM
    #[error("invalid path to CRAM-reference {path}")]
    BamInvalidReferencePath { path: PathBuf },
    #[error("invalid compression level {level}")]
    BamInvalidCompressionLevel { level: u32 },
    #[error("unable to open SAM/BAM/CRAM file at {target}")]
    BamOpen { target: String },
    #[error("failed to initialize BAM header state")]
    BamHeader,
    #[error("unable to open SAM/BAM/CRAM index for {target}; please create an index")]
    BamInvalidIndex { target: String },
    #[error("invalid record in SAM/BAM/CRAM file")]
    BamInvalidRecord,
    #[error("truncated record in SAM/BAM/CRAM file")]
    BamTruncatedRecord,
    #[error("failed to read record from SAM/BAM/CRAM file")]
    BamRead,
    #[error(
        "format not indexable by htslib (format is detected as something else than SAM/BAM/CRAM)"
    )]
    BamNotIndexable,
    #[error("failed to write BAM/CRAM index (out of disk space?)")]
    BamWriteIndex,
    #[error("failed to build BAM/CRAM index")]
    BamBuildIndex,
    #[error("file is not sorted by position")]
    BamUnsorted,
    #[error("failed to allocate SAM header text")]
    BamHeaderAllocation,
    #[error("failed to parse SAM header")]
    BamHeaderParse,
    #[error("virtual offsets are only supported for BAM files")]
    BamVirtualOffsetUnsupported,

    // Errors for BAM auxiliary fields
    #[error("failed to add aux field (out of memory?)")]
    BamAux,
    #[error("provided string contains internal 0 byte(s)")]
    BamAuxStringError,
    #[error("failed to parse aux data")]
    BamAuxParsingError,
    #[error("the specified tag does could not be found")]
    BamAuxTagNotFound,
    #[error("data type of aux field is not known")]
    BamAuxUnknownType,
    #[error("failed to add aux field, tag is already present")]
    BamAuxTagAlreadyPresent,

    #[error("failed setting hts reading options")]
    HtsSetOpt,
    #[error("failed calculating slow index statistics")]
    SlowIdxStats,
    #[error("invalid tid {tid}")]
    InvalidTid { tid: i32 },
    #[error("No sequences in the reference")]
    NoSequencesInReference,
}
