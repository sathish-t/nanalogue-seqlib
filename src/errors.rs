use std::fmt;
use std::path::PathBuf;

/// Generic result type for functions in this crate with
/// a global error class.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, PartialEq)]
pub enum Error {
    // General errors
    FileNotFound { path: PathBuf },
    FileOpen { path: String },
    NonUnicodePath,
    Fetch,
    FileSeek,
    SetThreads,
    ThreadPool,

    WriteRecord,
    WriteHeader,
    WriteClose,

    // Errors for faidx
    FaidxBuildFailed { path: std::path::PathBuf },

    // Errors for BAM
    BamInvalidReferencePath { path: PathBuf },
    BamInvalidCompressionLevel { level: u32 },
    BamOpen { target: String },
    BamHeader,
    BamInvalidIndex { target: String },
    BamInvalidRecord,
    BamTruncatedRecord,
    BamRead,
    BamNotIndexable,
    BamWriteIndex,
    BamBuildIndex,
    BamUnsorted,
    BamHeaderAllocation,
    BamHeaderParse,
    BamVirtualOffsetUnsupported,

    // Errors for BAM auxiliary fields
    BamAux,
    BamAuxStringError,
    BamAuxParsingError,
    BamAuxTagNotFound,
    BamAuxUnknownType,
    BamAuxTagAlreadyPresent,

    HtsSetOpt,
    SlowIdxStats,
    InvalidTid { tid: i32 },
    NoSequencesInReference,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound { path } => write!(f, "file not found: {}", path.display()),
            Self::FileOpen { path } => write!(f, "file could not be opened: {path}"),
            Self::NonUnicodePath => f.write_str("invalid (non-unicode) characters in path"),
            Self::Fetch => f.write_str("failed to fetch region"),
            Self::FileSeek => f.write_str("error seeking to file offset"),
            Self::SetThreads => f.write_str("error setting threads for file reading"),
            Self::ThreadPool => f.write_str("failed to create htslib thread pool"),
            Self::WriteRecord => f.write_str("failed to write BAM/BCF record (out of disk space?)"),
            Self::WriteHeader => f.write_str("failed to write SAM/BAM/CRAM header"),
            Self::WriteClose => f.write_str("failed to close SAM/BAM/CRAM writer"),
            Self::FaidxBuildFailed { path } => {
                write!(f, "failed to build index for fasta file {path:?}")
            }
            Self::BamInvalidReferencePath { path } => {
                write!(f, "invalid path to CRAM-reference {}", path.display())
            }
            Self::BamInvalidCompressionLevel { level } => {
                write!(f, "invalid compression level {level}")
            }
            Self::BamOpen { target } => write!(f, "unable to open SAM/BAM/CRAM file at {target}"),
            Self::BamHeader => f.write_str("failed to initialize BAM header state"),
            Self::BamInvalidIndex { target } => write!(
                f,
                "unable to open SAM/BAM/CRAM index for {target}; please create an index"
            ),
            Self::BamInvalidRecord => f.write_str("invalid record in SAM/BAM/CRAM file"),
            Self::BamTruncatedRecord => f.write_str("truncated record in SAM/BAM/CRAM file"),
            Self::BamRead => f.write_str("failed to read record from SAM/BAM/CRAM file"),
            Self::BamNotIndexable => f.write_str(
                "format not indexable by htslib (format is detected as something else than SAM/BAM/CRAM)",
            ),
            Self::BamWriteIndex => f.write_str("failed to write BAM/CRAM index (out of disk space?)"),
            Self::BamBuildIndex => f.write_str("failed to build BAM/CRAM index"),
            Self::BamUnsorted => f.write_str("file is not sorted by position"),
            Self::BamHeaderAllocation => f.write_str("failed to allocate SAM header text"),
            Self::BamHeaderParse => f.write_str("failed to parse SAM header"),
            Self::BamVirtualOffsetUnsupported => {
                f.write_str("virtual offsets are only supported for BAM files")
            }
            Self::BamAux => f.write_str("failed to add aux field (out of memory?)"),
            Self::BamAuxStringError => f.write_str("provided string contains internal 0 byte(s)"),
            Self::BamAuxParsingError => f.write_str("failed to parse aux data"),
            Self::BamAuxTagNotFound => f.write_str("the specified tag does could not be found"),
            Self::BamAuxUnknownType => f.write_str("data type of aux field is not known"),
            Self::BamAuxTagAlreadyPresent => {
                f.write_str("failed to add aux field, tag is already present")
            }
            Self::HtsSetOpt => f.write_str("failed setting hts reading options"),
            Self::SlowIdxStats => f.write_str("failed calculating slow index statistics"),
            Self::InvalidTid { tid } => write!(f, "invalid tid {tid}"),
            Self::NoSequencesInReference => f.write_str("No sequences in the reference"),
        }
    }
}

impl std::error::Error for Error {}
