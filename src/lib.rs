// Copyright 2014-2021 Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Rust-Htslib provides a high level API to working with the common HTS file formats.
//!
//! Htslib itself is the *de facto* standard implementation for reading and writing files for
//! HTS alignments (SAM and BAM) as well as variant calls in VCF and BCF format.
//!
//! For example, let's say that we use samtools to view the header of a test file:
//!
//! ```bash
//! samtools view -H test/test.bam
//! @SQ    SN:CHROMOSOME_I    LN:15072423
//! @SQ    SN:CHROMOSOME_II    LN:15279345
//! @SQ    SN:CHROMOSOME_III    LN:13783700
//! @SQ    SN:CHROMOSOME_IV    LN:17493793
//! @SQ    SN:CHROMOSOME_V    LN:20924149
//! ```
//!
//! We can reproduce that with Rust-Htslib. Reading BAM files and printing the header
//! to the screen is as easy as
//!
//! ```
//! use rust_htslib::{bam, bam::Read};
//!
//!
//! let bam = bam::Reader::from_path(&"test/test.bam").unwrap();
//!
//! // print header records to the terminal, akin to samtools
//! print!("{}", std::str::from_utf8(bam.header().as_bytes()).unwrap());
//! ```
//!
//! which results in the following output, equivalent to samtools.
//!
//! ```bash
//! @SQ    SN:CHROMOSOME_I    LN:15072423
//! @SQ    SN:CHROMOSOME_II    LN:15279345
//! @SQ    SN:CHROMOSOME_III    LN:13783700
//! @SQ    SN:CHROMOSOME_IV    LN:17493793
//! @SQ    SN:CHROMOSOME_V    LN:20924149
//! ```
//!
//! We can also read directly from the BAM file and write to an output file,
//! constructing a header containing its reference sequences:
//!
//! ```
//! use rust_htslib::{bam, bam::Read};
//!
//! let mut bam = bam::Reader::from_path(&"test/test.bam").unwrap();
//! let mut header = bam::Header::new();
//! for (tid, name) in bam.header().target_names().iter().enumerate() {
//!     header.push_record(
//!         bam::header::HeaderRecord::new(b"SQ")
//!             .push_tag(b"SN", std::str::from_utf8(name).unwrap())
//!             .push_tag(b"LN", bam.header().target_len(tid as u32).unwrap()),
//!     );
//! }
//! let mut out = bam::Writer::from_path(&"test/out.bam", &header, bam::Format::Bam).unwrap();
//!
//! // copy reverse reads to new BAM file
//! for r in bam.records() {
//!     let record = r.unwrap();
//!     if record.is_reverse() {
//!         out.write(&record).unwrap();
//!     }
//! }
//! ```
//!
//! Indexed BAM files can be seeked for specific regions using [`fetch`](bam/struct.IndexedReader.html#method.fetch), constraining the record iterator:
//!
//! ```
//! use rust_htslib::{bam, bam::Read};
//!
//! let mut bam = bam::IndexedReader::from_path(&"test/test.bam").unwrap();
//!
//! bam.fetch(("CHROMOSOME_I", 0, 20)).unwrap();
//! // afterwards, read records in this region
//! ```
//!
//! See
//! * [`fetch`](bam/struct.IndexedReader.html#method.fetch)
//! * [`records`](bam/struct.IndexedReader.html#method.records)
//! * [`read`](bam/struct.IndexedReader.html#method.read)

#[macro_use]
extern crate custom_derive;

#[macro_use]
extern crate newtype_derive;

#[cfg(feature = "serde_feature")]
extern crate serde;

#[cfg(test)] // <-- not needed in examples + integration tests
#[macro_use]
extern crate pretty_assertions;

pub mod bam;
pub mod errors;
pub mod htslib;
pub mod tpool;
pub mod utils;
