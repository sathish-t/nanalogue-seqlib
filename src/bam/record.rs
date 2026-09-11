// Copyright 2014 Christopher Schröder, Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::ffi;
use std::fmt;
use std::marker::PhantomData;
use std::mem::{size_of, MaybeUninit};
use std::ops;
use std::os::raw::c_char;
use std::slice;
use std::str;
use std::sync::Arc;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::bam::Error;
use crate::bam::HeaderView;
use crate::errors::Result;
use crate::htslib;
use crate::utils;

use bio_types::genome;

/// A macro creating methods for flag access.
macro_rules! flag {
    ($get:ident, $bit:expr) => {
        pub fn $get(&self) -> bool {
            self.inner().core.flag & $bit != 0
        }
    };
    ($get:ident, $set:ident, $unset:ident, $bit:expr) => {
        flag!($get, $bit);

        pub fn $set(&mut self) {
            self.inner_mut_unchecked().core.flag |= $bit;
        }

        pub fn $unset(&mut self) {
            self.inner_mut_unchecked().core.flag &= !$bit;
        }
    };
}

/// A BAM record.
///
/// Each record owns the allocation referenced by `inner.data` and frees it when dropped.
pub struct Record {
    inner: htslib::bam1_t,
    cigar: Option<CigarStringView>,
    header: Option<Arc<HeaderView>>,
}

impl fmt::Debug for Record {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_fmt(format_args!(
            "Record(tid: {}, pos: {})",
            self.tid(),
            self.pos()
        ))
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Record) -> bool {
        self.inner().core.tid == other.inner().core.tid
            && self.inner().core.pos == other.inner().core.pos
            && self.inner().core.bin == other.inner().core.bin
            && self.inner().core.qual == other.inner().core.qual
            && self.inner().core.flag == other.inner().core.flag
            && self.inner().core.mtid == other.inner().core.mtid
            && self.inner().core.mpos == other.inner().core.mpos
            && self.inner().core.isize_ == other.inner().core.isize_
            && self.data() == other.data()
            && self.inner().core.l_extranul == other.inner().core.l_extranul
    }
}

impl Eq for Record {}

impl Default for Record {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn extranul_from_qname(qname: &[u8]) -> usize {
    let qlen = qname.len() + 1;
    if !qlen.is_multiple_of(4) {
        4 - qlen % 4
    } else {
        0
    }
}

impl Record {
    /// Create an empty BAM record.
    pub fn new() -> Self {
        let mut record = Record {
            inner: unsafe { MaybeUninit::zeroed().assume_init() },
            cigar: None,
            header: None,
        };
        // The read/query name needs to be set as empty to properly initialize
        // the record
        record.set_qname(b"");
        // Developer note: these are needed so the returned record is properly
        // initialized as unmapped.
        record.set_unmapped();
        record.set_tid(-1);
        record.set_pos(-1);
        record.set_mpos(-1);
        record.set_mtid(-1);
        record
    }

    pub fn set_header(&mut self, header: Arc<HeaderView>) {
        self.header = Some(header);
    }

    pub(super) fn data(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.inner().data, self.inner().l_data as usize) }
    }

    /// Returns the underlying HTSlib record for mutation.
    ///
    /// # Safety
    ///
    /// The caller must preserve all `bam1_t` ownership, pointer, length, and
    /// layout invariants. In particular, `data` must remain allocated with the
    /// allocator expected by HTSlib and `l_data` must not exceed `m_data`.
    #[inline]
    pub unsafe fn inner_mut(&mut self) -> &mut htslib::bam1_t {
        &mut self.inner
    }

    #[inline]
    fn inner_mut_unchecked(&mut self) -> &mut htslib::bam1_t {
        &mut self.inner
    }

    #[inline]
    pub(super) fn inner_ptr_mut(&mut self) -> *mut htslib::bam1_t {
        &mut self.inner as *mut htslib::bam1_t
    }

    #[inline]
    pub fn inner(&self) -> &htslib::bam1_t {
        &self.inner
    }

    #[inline]
    pub(super) fn inner_ptr(&self) -> *const htslib::bam1_t {
        &self.inner as *const htslib::bam1_t
    }

    /// Get target id.
    pub fn tid(&self) -> i32 {
        self.inner().core.tid
    }

    /// Set target id.
    pub fn set_tid(&mut self, tid: i32) {
        self.inner_mut_unchecked().core.tid = tid;
    }

    /// Get position (0-based).
    pub fn pos(&self) -> i64 {
        self.inner().core.pos
    }

    /// Set position (0-based).
    pub fn set_pos(&mut self, pos: i64) {
        self.cigar = None;
        self.inner_mut_unchecked().core.pos = pos;
    }

    /// Get MAPQ.
    pub fn mapq(&self) -> u8 {
        self.inner().core.qual
    }

    /// Set MAPQ.
    pub fn set_mapq(&mut self, mapq: u8) {
        self.inner_mut_unchecked().core.qual = mapq;
    }

    /// Get raw flags.
    pub fn flags(&self) -> u16 {
        self.inner().core.flag
    }

    /// Set raw flags.
    pub fn set_flags(&mut self, flags: u16) {
        self.inner_mut_unchecked().core.flag = flags;
    }

    /// Unset all flags.
    pub fn unset_flags(&mut self) {
        self.inner_mut_unchecked().core.flag = 0;
    }

    /// Set target id of mate.
    pub fn set_mtid(&mut self, mtid: i32) {
        self.inner_mut_unchecked().core.mtid = mtid;
    }

    /// Set mate position.
    pub fn set_mpos(&mut self, mpos: i64) {
        self.inner_mut_unchecked().core.mpos = mpos;
    }

    fn qname_capacity(&self) -> usize {
        self.inner().core.l_qname as usize
    }

    fn qname_len(&self) -> usize {
        // discount all trailing zeros (the default one and extra nulls)
        self.qname_capacity() - 1 - self.inner().core.l_extranul as usize
    }

    /// Get qname (read name). Complexity: O(1).
    pub fn qname(&self) -> &[u8] {
        &self.data()[..self.qname_len()]
    }

    /// Set variable length data (qname, cigar, seq, qual).
    /// The aux data is left unchanged.
    /// `qual` is Phred-scaled quality values, without any offset.
    /// NOTE: seq.len() must equal qual.len() or this method
    /// will panic. If you don't have quality values use
    /// `let quals = vec![ 255 as u8; seq.len()];` as a placeholder that will
    /// be recognized as missing QVs by `samtools`.
    pub fn set(&mut self, qname: &[u8], cigar: Option<&CigarString>, seq: &[u8], qual: &[u8]) {
        assert!(qname.len() < 255);
        assert_eq!(seq.len(), qual.len(), "seq.len() must equal qual.len()");
        if let Some(cigar_string) = cigar {
            for cigar_op in cigar_string {
                cigar_op.assert_valid_len();
            }
        }

        self.cigar = None;

        let cigar_width = if let Some(cigar_string) = cigar {
            cigar_string.len()
        } else {
            0
        } * 4;
        let q_len = qname.len() + 1;
        let extranul = extranul_from_qname(qname);

        let orig_aux_offset = self.qname_capacity()
            + 4 * self.cigar_len()
            + self.seq_len().div_ceil(2)
            + self.seq_len();
        let new_aux_offset = q_len + extranul + cigar_width + seq.len().div_ceil(2) + qual.len();
        assert!(orig_aux_offset <= self.inner.l_data as usize);
        let aux_len = self.inner.l_data as usize - orig_aux_offset;
        self.inner_mut_unchecked().l_data = (new_aux_offset + aux_len) as i32;
        if (self.inner().m_data as i32) < self.inner().l_data {
            // Verbosity due to lexical borrowing
            let l_data = self.inner().l_data;
            self.realloc_var_data(l_data as usize);
        }

        // Copy the aux data.
        if aux_len > 0 && orig_aux_offset != new_aux_offset {
            let data =
                unsafe { slice::from_raw_parts_mut(self.inner.data, self.inner().m_data as usize) };
            data.copy_within(orig_aux_offset..orig_aux_offset + aux_len, new_aux_offset);
        }

        let data =
            unsafe { slice::from_raw_parts_mut(self.inner.data, self.inner().l_data as usize) };

        // qname
        utils::copy_memory(qname, data);
        for i in 0..=extranul {
            data[qname.len() + i] = b'\0';
        }
        let mut i = q_len + extranul;
        self.inner_mut_unchecked().core.l_qname = i as u16;
        self.inner_mut_unchecked().core.l_extranul = extranul as u8;

        // cigar
        if let Some(cigar_string) = cigar {
            let cigar_data = unsafe {
                //cigar is always aligned to 4 bytes (see extranul above) - so this is safe
                #[allow(clippy::cast_ptr_alignment)]
                slice::from_raw_parts_mut(data[i..].as_ptr() as *mut u32, cigar_string.len())
            };
            for (i, c) in cigar_string.iter().enumerate() {
                cigar_data[i] = c.encode();
            }
            self.inner_mut_unchecked().core.n_cigar = cigar_string.len() as u32;
            i += cigar_string.len() * 4;
        } else {
            self.inner_mut_unchecked().core.n_cigar = 0;
        };

        // seq
        {
            for j in (0..seq.len()).step_by(2) {
                data[i + j / 2] = (ENCODE_BASE[seq[j] as usize] << 4)
                    | (if j + 1 < seq.len() {
                        ENCODE_BASE[seq[j + 1] as usize]
                    } else {
                        0
                    });
            }
            self.inner_mut_unchecked().core.l_qseq = seq.len() as i32;
            i += seq.len().div_ceil(2);
        }

        // qual
        utils::copy_memory(qual, &mut data[i..]);
    }

    /// Replace current qname with a new one.
    pub fn set_qname(&mut self, new_qname: &[u8]) {
        // 251 + 1NUL is the max 32-bit aligned value that fits in u8
        assert!(new_qname.len() < 252);

        let old_q_len = self.qname_capacity();
        // We're going to add a terminal NUL
        let extranul = extranul_from_qname(new_qname);
        let new_q_len = new_qname.len() + 1 + extranul;

        // Length of data after qname
        let other_len = self.inner_mut_unchecked().l_data - old_q_len as i32;

        if new_q_len < old_q_len && self.inner().l_data > (old_q_len as i32) {
            self.inner_mut_unchecked().l_data -= (old_q_len - new_q_len) as i32;
        } else if new_q_len > old_q_len {
            self.inner_mut_unchecked().l_data += (new_q_len - old_q_len) as i32;

            // Reallocate if necessary
            if (self.inner().m_data as i32) < self.inner().l_data {
                // Verbosity due to lexical borrowing
                let l_data = self.inner().l_data;
                self.realloc_var_data(l_data as usize);
            }
        }

        if new_q_len != old_q_len {
            // Move other data to new location
            unsafe {
                let data = slice::from_raw_parts_mut(self.inner.data, self.inner().l_data as usize);

                ::libc::memmove(
                    data.as_mut_ptr().add(new_q_len) as *mut ::libc::c_void,
                    data.as_mut_ptr().add(old_q_len) as *mut ::libc::c_void,
                    other_len as usize,
                );
            }
        }

        // Copy qname data
        let data =
            unsafe { slice::from_raw_parts_mut(self.inner.data, self.inner().l_data as usize) };
        utils::copy_memory(new_qname, data);
        for i in 0..=extranul {
            data[new_q_len - i - 1] = b'\0';
        }
        self.inner_mut_unchecked().core.l_qname = new_q_len as u16;
        self.inner_mut_unchecked().core.l_extranul = extranul as u8;
    }

    fn realloc_var_data(&mut self, new_len: usize) {
        // pad request
        let new_len = new_len as u32;
        let new_request = new_len + 32 - (new_len % 32);

        let ptr = unsafe {
            ::libc::realloc(
                self.inner().data as *mut ::libc::c_void,
                new_request as usize,
            ) as *mut u8
        };

        if ptr.is_null() {
            panic!("ran out of memory in rust_htslib trying to realloc");
        }

        // don't update m_data until we know we have
        // a successful allocation.
        self.inner_mut_unchecked().m_data = new_request;
        self.inner_mut_unchecked().data = ptr;
    }

    pub fn cigar_len(&self) -> usize {
        self.inner().core.n_cigar as usize
    }

    /// Get reference to raw cigar string representation (as stored in BAM file).
    /// Usually, the method `Record::cigar` should be used instead.
    pub fn raw_cigar(&self) -> &[u32] {
        //cigar is always aligned to 4 bytes - so this is safe
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            slice::from_raw_parts(
                self.data()[self.qname_capacity()..].as_ptr() as *const u32,
                self.cigar_len(),
            )
        }
    }

    /// Return unpacked cigar string. This will create a fresh copy the Cigar data.
    pub fn cigar(&self) -> CigarStringView {
        match self.cigar {
            Some(ref c) => c.clone(),
            None => self.unpack_cigar(),
        }
    }

    // Return unpacked cigar string. This returns None unless you have first called `bam::Record::cache_cigar`.
    pub fn cigar_cached(&self) -> Option<&CigarStringView> {
        self.cigar.as_ref()
    }

    /// Decode the cigar string and cache it inside the `Record`
    pub fn cache_cigar(&mut self) {
        self.cigar = Some(self.unpack_cigar())
    }

    pub(crate) fn clear_cigar_cache(&mut self) {
        self.cigar = None;
    }

    /// Unpack cigar string. Complexity: O(k) with k being the length of the cigar string.
    fn unpack_cigar(&self) -> CigarStringView {
        CigarString(
            self.raw_cigar()
                .iter()
                .map(|&c| {
                    let len = c >> 4;
                    match c & 0b1111 {
                        0 => Cigar::Match(len),
                        1 => Cigar::Ins(len),
                        2 => Cigar::Del(len),
                        3 => Cigar::RefSkip(len),
                        4 => Cigar::SoftClip(len),
                        5 => Cigar::HardClip(len),
                        6 => Cigar::Pad(len),
                        7 => Cigar::Equal(len),
                        8 => Cigar::Diff(len),
                        _ => panic!("Unexpected cigar operation"),
                    }
                })
                .collect(),
        )
        .into_view(self.pos())
    }

    pub fn seq_len(&self) -> usize {
        self.inner().core.l_qseq as usize
    }

    fn seq_data(&self) -> &[u8] {
        let offset = self.qname_capacity() + self.cigar_len() * 4;
        &self.data()[offset..][..self.seq_len().div_ceil(2)]
    }

    /// Get read sequence. Complexity: O(1).
    pub fn seq(&self) -> Seq<'_> {
        Seq {
            encoded: self.seq_data(),
            len: self.seq_len(),
        }
    }

    /// Get base qualities (PHRED-scaled probability that base is wrong).
    /// This does not entail any offsets, hence the qualities can be used directly without
    /// e.g. subtracting 33. Complexity: O(1).
    pub fn qual(&self) -> &[u8] {
        &self.data()[self.qname_capacity() + self.cigar_len() * 4 + self.seq_len().div_ceil(2)..]
            [..self.seq_len()]
    }

    /// Look up an auxiliary field by its tag.
    ///
    /// Only the first two bytes of a given tag are used for the look-up of a field.
    /// See [`Aux`] for more details.
    pub fn aux(&self, tag: &[u8]) -> Result<Aux<'_>> {
        if tag.len() < 2 {
            return Err(Error::BamAuxStringError);
        }
        let mut aux_data = self.aux_data()?;
        while !aux_data.is_empty() {
            let tag_bytes = aux_data.get(..2).ok_or(Error::BamAuxParsingError)?;
            let field_data = aux_data.get(2..).ok_or(Error::BamAuxParsingError)?;
            if tag_bytes == &tag[..2] {
                let (aux, _) = Self::read_aux_field(field_data)?;
                return Ok(aux);
            }
            let length = Self::aux_field_len(field_data)?;
            aux_data = aux_data
                .get(2..)
                .and_then(|field| field.get(length..))
                .ok_or(Error::BamAuxParsingError)?;
        }
        Err(Error::BamAuxTagNotFound)
    }

    fn aux_data(&self) -> Result<&[u8]> {
        let offset = self
            .qname_capacity()
            .checked_add(
                self.cigar_len()
                    .checked_mul(size_of::<u32>())
                    .ok_or(Error::BamAuxParsingError)?,
            )
            .and_then(|offset| offset.checked_add(self.seq_len().div_ceil(2)))
            .and_then(|offset| offset.checked_add(self.seq_len()))
            .ok_or(Error::BamAuxParsingError)?;
        self.data().get(offset..).ok_or(Error::BamAuxParsingError)
    }

    fn aux_field_len(aux: &[u8]) -> Result<usize> {
        const TYPE_ID_LEN: usize = 1;
        const ARRAY_INNER_TYPE_LEN: usize = 1;
        const ARRAY_COUNT_LEN: usize = 4;

        let type_size = match *aux.first().ok_or(Error::BamAuxParsingError)? {
            b'A' | b'c' | b'C' => 1,
            b's' | b'S' => 2,
            b'i' | b'I' | b'f' => 4,
            b'd' => 8,
            b'Z' | b'H' => {
                let string_data = aux.get(TYPE_ID_LEN..).ok_or(Error::BamAuxParsingError)?;
                let nul_offset = string_data
                    .iter()
                    .position(|&byte| byte == 0)
                    .ok_or(Error::BamAuxParsingError)?;
                return Ok(TYPE_ID_LEN + nul_offset + 1);
            }
            b'B' => {
                let array_data_offset = TYPE_ID_LEN + ARRAY_INNER_TYPE_LEN + ARRAY_COUNT_LEN;
                let inner_type = *aux.get(TYPE_ID_LEN).ok_or(Error::BamAuxParsingError)?;
                let count = aux
                    .get(TYPE_ID_LEN + ARRAY_INNER_TYPE_LEN..array_data_offset)
                    .ok_or(Error::BamAuxParsingError)?
                    .read_u32::<LittleEndian>()
                    .map_err(|_| Error::BamAuxParsingError)? as usize;
                let element_size = match inner_type {
                    b'c' | b'C' => 1,
                    b's' | b'S' => 2,
                    b'i' | b'I' | b'f' => 4,
                    _ => return Err(Error::BamAuxUnknownType),
                };
                let field_len = count
                    .checked_mul(element_size)
                    .and_then(|length| array_data_offset.checked_add(length))
                    .ok_or(Error::BamAuxParsingError)?;
                if aux.get(..field_len).is_none() {
                    return Err(Error::BamAuxParsingError);
                }
                return Ok(field_len);
            }
            _ => return Err(Error::BamAuxUnknownType),
        };
        let field_len = TYPE_ID_LEN + type_size;
        if aux.get(..field_len).is_none() {
            return Err(Error::BamAuxParsingError);
        }
        Ok(field_len)
    }

    fn read_aux_field<'a>(aux: &'a [u8]) -> Result<(Aux<'a>, usize)> {
        const TYPE_ID_LEN: usize = 1;
        const ARRAY_INNER_TYPE_LEN: usize = 1;
        const ARRAY_COUNT_LEN: usize = 4;

        let type_id = *aux.first().ok_or(Error::BamAuxParsingError)?;
        let fixed_data = |size| {
            aux.get(TYPE_ID_LEN..TYPE_ID_LEN + size)
                .ok_or(Error::BamAuxParsingError)
        };
        let (data, type_size) = match type_id {
            b'A' => {
                let type_size = size_of::<u8>();
                (Aux::Char(fixed_data(type_size)?[0]), type_size)
            }
            b'c' => {
                let type_size = size_of::<i8>();
                (Aux::I8(fixed_data(type_size)?[0] as i8), type_size)
            }
            b'C' => {
                let type_size = size_of::<u8>();
                (Aux::U8(fixed_data(type_size)?[0]), type_size)
            }
            b's' => {
                let type_size = size_of::<i16>();
                (
                    Aux::I16(
                        fixed_data(type_size)?
                            .read_i16::<LittleEndian>()
                            .map_err(|_| Error::BamAuxParsingError)?,
                    ),
                    type_size,
                )
            }
            b'S' => {
                let type_size = size_of::<u16>();
                (
                    Aux::U16(
                        fixed_data(type_size)?
                            .read_u16::<LittleEndian>()
                            .map_err(|_| Error::BamAuxParsingError)?,
                    ),
                    type_size,
                )
            }
            b'i' => {
                let type_size = size_of::<i32>();
                (
                    Aux::I32(
                        fixed_data(type_size)?
                            .read_i32::<LittleEndian>()
                            .map_err(|_| Error::BamAuxParsingError)?,
                    ),
                    type_size,
                )
            }
            b'I' => {
                let type_size = size_of::<u32>();
                (
                    Aux::U32(
                        fixed_data(type_size)?
                            .read_u32::<LittleEndian>()
                            .map_err(|_| Error::BamAuxParsingError)?,
                    ),
                    type_size,
                )
            }
            b'f' => {
                let type_size = size_of::<f32>();
                (
                    Aux::Float(
                        fixed_data(type_size)?
                            .read_f32::<LittleEndian>()
                            .map_err(|_| Error::BamAuxParsingError)?,
                    ),
                    type_size,
                )
            }
            b'd' => {
                let type_size = size_of::<f64>();
                (
                    Aux::Double(
                        fixed_data(type_size)?
                            .read_f64::<LittleEndian>()
                            .map_err(|_| Error::BamAuxParsingError)?,
                    ),
                    type_size,
                )
            }
            b'Z' | b'H' => {
                let string_data = aux.get(TYPE_ID_LEN..).ok_or(Error::BamAuxParsingError)?;
                let nul_offset = string_data
                    .iter()
                    .position(|&byte| byte == 0)
                    .ok_or(Error::BamAuxParsingError)?;
                let rust_str = str::from_utf8(&string_data[..nul_offset])
                    .map_err(|_| Error::BamAuxParsingError)?;
                let value = if type_id == b'H' {
                    Aux::HexByteArray(rust_str)
                } else {
                    Aux::String(rust_str)
                };
                (value, nul_offset + 1)
            }
            b'B' => {
                let array_data_offset = TYPE_ID_LEN + ARRAY_INNER_TYPE_LEN + ARRAY_COUNT_LEN;
                let inner_type = *aux.get(TYPE_ID_LEN).ok_or(Error::BamAuxParsingError)?;
                let length = aux
                    .get(TYPE_ID_LEN + ARRAY_INNER_TYPE_LEN..array_data_offset)
                    .ok_or(Error::BamAuxParsingError)?
                    .read_u32::<LittleEndian>()
                    .map_err(|_| Error::BamAuxParsingError)? as usize;
                let array_bytes = |element_size| {
                    length
                        .checked_mul(element_size)
                        .and_then(|length| array_data_offset.checked_add(length))
                        .ok_or(Error::BamAuxParsingError)
                };
                let array_slice = |element_size| {
                    aux.get(array_data_offset..array_bytes(element_size)?)
                        .ok_or(Error::BamAuxParsingError)
                };

                // Return tuples of an `Aux` enum and the length of data + metadata in bytes
                let (array_data, array_size) = match inner_type {
                    b'c' => (
                        Aux::ArrayI8(AuxArray::<'a, i8>::from_bytes(
                            array_slice(size_of::<i8>())?,
                        )),
                        length,
                    ),
                    b'C' => (
                        Aux::ArrayU8(AuxArray::<'a, u8>::from_bytes(
                            array_slice(size_of::<u8>())?,
                        )),
                        length,
                    ),
                    b's' => (
                        Aux::ArrayI16(AuxArray::<'a, i16>::from_bytes(array_slice(
                            size_of::<i16>(),
                        )?)),
                        length
                            .checked_mul(size_of::<i16>())
                            .ok_or(Error::BamAuxParsingError)?,
                    ),
                    b'S' => (
                        Aux::ArrayU16(AuxArray::<'a, u16>::from_bytes(array_slice(
                            size_of::<u16>(),
                        )?)),
                        length
                            .checked_mul(size_of::<u16>())
                            .ok_or(Error::BamAuxParsingError)?,
                    ),
                    b'i' => (
                        Aux::ArrayI32(AuxArray::<'a, i32>::from_bytes(array_slice(
                            size_of::<i32>(),
                        )?)),
                        length
                            .checked_mul(size_of::<i32>())
                            .ok_or(Error::BamAuxParsingError)?,
                    ),
                    b'I' => (
                        Aux::ArrayU32(AuxArray::<'a, u32>::from_bytes(array_slice(
                            size_of::<u32>(),
                        )?)),
                        length
                            .checked_mul(size_of::<u32>())
                            .ok_or(Error::BamAuxParsingError)?,
                    ),
                    b'f' => (
                        Aux::ArrayFloat(AuxArray::<f32>::from_bytes(
                            array_slice(size_of::<f32>())?,
                        )),
                        length
                            .checked_mul(size_of::<f32>())
                            .ok_or(Error::BamAuxParsingError)?,
                    ),
                    _ => {
                        return Err(Error::BamAuxUnknownType);
                    }
                };
                (
                    array_data,
                    // Offset: array-specific metadata + array size
                    ARRAY_INNER_TYPE_LEN + ARRAY_COUNT_LEN + array_size,
                )
            }
            _ => {
                return Err(Error::BamAuxUnknownType);
            }
        };

        // Offset: metadata + type size
        Ok((data, TYPE_ID_LEN + type_size))
    }

    /// Add auxiliary data.
    ///
    /// Only the first two bytes of `tag` are used. Returns [`Error::BamAuxStringError`] if `tag`
    /// is shorter than two bytes.
    pub fn push_aux(&mut self, tag: &[u8], value: Aux<'_>) -> Result<()> {
        if tag.len() < 2 {
            return Err(Error::BamAuxStringError);
        }

        // Don't allow pushing aux data when the given tag is already present in the record.
        // `htslib` seems to allow this (for non-array values), which can lead to problems
        // since retrieving aux fields consumes &[u8; 2] and yields one field only.
        if self.aux(tag).is_ok() {
            return Err(Error::BamAuxTagAlreadyPresent);
        }
        self.push_aux_unchecked(tag, value)
    }

    /// Add auxiliary data, without checking if the tag is present.
    ///
    /// The caller should ensure that the same tag is not pushed more than once.
    /// This is provided as a performance optimization.
    fn push_aux_unchecked(&mut self, tag: &[u8], value: Aux<'_>) -> Result<()> {
        let ctag = tag.as_ptr() as *mut c_char;
        let ret = unsafe {
            match value {
                Aux::Char(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'A' as c_char,
                    size_of::<u8>() as i32,
                    [v].as_mut_ptr(),
                ),
                Aux::I8(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'c' as c_char,
                    size_of::<i8>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                Aux::U8(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'C' as c_char,
                    size_of::<u8>() as i32,
                    [v].as_mut_ptr(),
                ),
                Aux::I16(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b's' as c_char,
                    size_of::<i16>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                Aux::U16(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'S' as c_char,
                    size_of::<u16>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                Aux::I32(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'i' as c_char,
                    size_of::<i32>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                Aux::U32(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'I' as c_char,
                    size_of::<u32>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                Aux::Float(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'f' as c_char,
                    size_of::<f32>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                // Not part of specs but implemented in `htslib`:
                Aux::Double(v) => htslib::bam_aux_append(
                    self.inner_ptr_mut(),
                    ctag,
                    b'd' as c_char,
                    size_of::<f64>() as i32,
                    [v].as_mut_ptr() as *mut u8,
                ),
                Aux::String(v) => {
                    let c_str = ffi::CString::new(v).map_err(|_| Error::BamAuxStringError)?;
                    htslib::bam_aux_append(
                        self.inner_ptr_mut(),
                        ctag,
                        b'Z' as c_char,
                        (v.len() + 1) as i32,
                        c_str.as_ptr() as *mut u8,
                    )
                }
                Aux::HexByteArray(v) => {
                    let c_str = ffi::CString::new(v).map_err(|_| Error::BamAuxStringError)?;
                    htslib::bam_aux_append(
                        self.inner_ptr_mut(),
                        ctag,
                        b'H' as c_char,
                        (v.len() + 1) as i32,
                        c_str.as_ptr() as *mut u8,
                    )
                }
                // Not sure it's safe to cast an immutable slice to a mutable pointer in the following branches
                Aux::ArrayI8(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'c',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'c',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
                Aux::ArrayU8(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'C',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'C',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
                Aux::ArrayI16(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b's',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b's',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
                Aux::ArrayU16(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'S',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'S',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
                Aux::ArrayI32(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'i',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'i',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
                Aux::ArrayU32(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'I',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'I',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
                Aux::ArrayFloat(aux_array) => match aux_array {
                    AuxArray::TargetType(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'f',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                    AuxArray::RawLeBytes(inner) => htslib::bam_aux_update_array(
                        self.inner_ptr_mut(),
                        ctag,
                        b'f',
                        inner.len() as u32,
                        inner.slice.as_ptr() as *mut ::libc::c_void,
                    ),
                },
            }
        };

        if ret < 0 {
            Err(Error::BamAux)
        } else {
            Ok(())
        }
    }

    flag!(is_paired, 1u16);
    flag!(is_proper_pair, 2u16);
    flag!(is_unmapped, set_unmapped, unset_unmapped, 4u16);
    flag!(is_mate_unmapped, 8u16);
    flag!(is_reverse, set_reverse, unset_reverse, 16u16);
    flag!(is_mate_reverse, 32u16);
    flag!(is_first_in_template, 64u16);
    flag!(is_last_in_template, 128u16);
    flag!(is_secondary, 256u16);
    flag!(is_quality_check_failed, 512u16);
    flag!(is_duplicate, 1024u16);
    flag!(is_supplementary, 2048u16);
}

impl Drop for Record {
    fn drop(&mut self) {
        unsafe { ::libc::free(self.inner.data as *mut ::libc::c_void) }
    }
}

impl genome::AbstractInterval for Record {
    /// Return contig name. Panics if record does not know its header (which happens if it has not been read from a file).
    fn contig(&self) -> &str {
        let tid = self.tid();
        if tid < 0 {
            panic!("invalid tid, must be at least zero");
        }
        str::from_utf8(
            self.header
                .as_ref()
                .expect(
                    "header must be set (this is the case if the record has been read from a file)",
                )
                .tid2name(tid as u32),
        )
        .expect("unable to interpret contig name as UTF-8")
    }

    /// Return genomic range covered by alignment. Panics if `Record::cache_cigar()` has not been called first or `Record::pos()` is less than zero.
    fn range(&self) -> ops::Range<genome::Position> {
        let end_pos = self
            .cigar_cached()
            .expect("cigar has not been cached yet, call cache_cigar() first")
            .end_pos() as u64;

        if self.pos() < 0 {
            panic!("invalid position, must be positive")
        }

        self.pos() as u64..end_pos
    }
}

/// Auxiliary record data
///
/// The specification allows a wide range of types to be stored as an auxiliary data field of a BAM record.
///
/// Please note that the [`Aux::Double`] variant is _not_ part of the specification, but it is supported by `htslib`.
///
/// # Examples
///
/// ```
/// use rust_htslib::{
///     bam,
///     bam::record::{Aux, AuxArray},
///     errors::Error,
/// };
///
/// //Set up BAM record
/// let mut record = bam::Record::new();
/// record.set(b"ali1", None, b"ACGT", &[37; 4]);
///
/// // Add an integer field
/// let aux_integer_field = Aux::I32(1234);
/// record.push_aux(b"XI", aux_integer_field).unwrap();
///
/// match record.aux(b"XI") {
///     Ok(value) => {
///         // Typically, callers expect an aux field to be of a certain type.
///         // If that's not the case, the value can be `match`ed exhaustively.
///         if let Aux::I32(v) = value {
///             assert_eq!(v, 1234);
///         }
///     }
///     Err(e) => {
///         panic!("Error reading aux field: {}", e);
///     }
/// }
///
/// // Add an array field
/// let array_like_data = vec![0.4, 0.3, 0.2, 0.1];
/// let slice_of_data = &array_like_data;
/// let aux_array: AuxArray<f32> = slice_of_data.into();
/// let aux_array_field = Aux::ArrayFloat(aux_array);
/// record.push_aux(b"XA", aux_array_field).unwrap();
///
/// if let Ok(Aux::ArrayFloat(array)) = record.aux(b"XA") {
///     let read_array = array.iter().collect::<Vec<_>>();
///     assert_eq!(read_array, array_like_data);
/// } else {
///     panic!("Could not read array data");
/// }
/// ```
#[derive(Debug, PartialEq)]
pub enum Aux<'a> {
    Char(u8),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Float(f32),
    Double(f64), // Not part of specs but implemented in `htslib`
    String(&'a str),
    HexByteArray(&'a str),
    ArrayI8(AuxArray<'a, i8>),
    ArrayU8(AuxArray<'a, u8>),
    ArrayI16(AuxArray<'a, i16>),
    ArrayU16(AuxArray<'a, u16>),
    ArrayI32(AuxArray<'a, i32>),
    ArrayU32(AuxArray<'a, u32>),
    ArrayFloat(AuxArray<'a, f32>),
}

/// Types that can be used in aux arrays.
pub trait AuxArrayElement: Copy {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self>;
}

impl AuxArrayElement for i8 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_i8().ok()
    }
}
impl AuxArrayElement for u8 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_u8().ok()
    }
}
impl AuxArrayElement for i16 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_i16::<LittleEndian>().ok()
    }
}
impl AuxArrayElement for u16 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_u16::<LittleEndian>().ok()
    }
}
impl AuxArrayElement for i32 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_i32::<LittleEndian>().ok()
    }
}
impl AuxArrayElement for u32 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_u32::<LittleEndian>().ok()
    }
}
impl AuxArrayElement for f32 {
    fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        std::io::Cursor::new(bytes).read_f32::<LittleEndian>().ok()
    }
}

/// Provides access to aux arrays.
///
/// Provides methods to either retrieve single elements or an iterator over the
/// array.
///
/// This type is used for wrapping both, array data that was read from a
/// BAM record and slices of data that are going to be stored in one.
///
/// In order to be able to add an `AuxArray` field to a BAM record, `AuxArray`s
/// can be constructed via the `From` trait which is implemented for all
/// supported types (see [`AuxArrayElement`] for a list).
///
/// # Examples
///
/// ```
/// use rust_htslib::{
///     bam,
///     bam::record::{Aux, AuxArray},
/// };
///
/// //Set up BAM record
/// let mut record = bam::Record::new();
/// record.set(b"ali1", None, b"ACGT", &[37; 4]);
///
/// let data = vec![0.4, 0.3, 0.2, 0.1];
/// let slice_of_data = &data;
/// let aux_array: AuxArray<f32> = slice_of_data.into();
/// let aux_field = Aux::ArrayFloat(aux_array);
/// record.push_aux(b"XA", aux_field);
///
/// if let Ok(Aux::ArrayFloat(array)) = record.aux(b"XA") {
///     // Retrieve the second element from the array
///     assert_eq!(array.get(1).unwrap(), 0.3);
///     // Iterate over the array and collect it into a `Vec`
///     let read_array = array.iter().collect::<Vec<_>>();
///     assert_eq!(read_array, data);
/// } else {
///     panic!("Could not read array data");
/// }
/// ```
#[derive(Debug)]
pub enum AuxArray<'a, T> {
    TargetType(AuxArrayTargetType<'a, T>),
    RawLeBytes(AuxArrayRawLeBytes<'a, T>),
}

impl<T> PartialEq<AuxArray<'_, T>> for AuxArray<'_, T>
where
    T: AuxArrayElement + PartialEq,
{
    fn eq(&self, other: &AuxArray<'_, T>) -> bool {
        use AuxArray::*;
        match (self, other) {
            (TargetType(v), TargetType(v_other)) => v == v_other,
            (RawLeBytes(v), RawLeBytes(v_other)) => v == v_other,
            (TargetType(_), RawLeBytes(_)) => self.iter().eq(other.iter()),
            (RawLeBytes(_), TargetType(_)) => self.iter().eq(other.iter()),
        }
    }
}

/// Create AuxArrays from slices of allowed target types.
impl<'a, I, T> From<&'a T> for AuxArray<'a, I>
where
    I: AuxArrayElement,
    T: AsRef<[I]> + ?Sized,
{
    fn from(src: &'a T) -> Self {
        AuxArray::TargetType(AuxArrayTargetType {
            slice: src.as_ref(),
        })
    }
}

impl<'a, T> AuxArray<'a, T>
where
    T: AuxArrayElement,
{
    /// Returns the element at a position or None if out of bounds.
    pub fn get(&self, index: usize) -> Option<T> {
        match self {
            AuxArray::TargetType(v) => v.get(index),
            AuxArray::RawLeBytes(v) => v.get(index),
        }
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        match self {
            AuxArray::TargetType(a) => a.len(),
            AuxArray::RawLeBytes(a) => a.len(),
        }
    }

    /// Returns true if the array contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the array.
    pub fn iter(&'_ self) -> AuxArrayIter<'_, T> {
        AuxArrayIter {
            index: 0,
            array: self,
        }
    }

    /// Create AuxArrays from raw byte slices borrowed from `bam::Record`.
    fn from_bytes(bytes: &'a [u8]) -> Self {
        Self::RawLeBytes(AuxArrayRawLeBytes {
            slice: bytes,
            phantom_data: PhantomData,
        })
    }
}

/// Encapsulates slice of target type.
#[doc(hidden)]
#[derive(Debug, PartialEq)]
pub struct AuxArrayTargetType<'a, T> {
    slice: &'a [T],
}

impl<T> AuxArrayTargetType<'_, T>
where
    T: AuxArrayElement,
{
    fn get(&self, index: usize) -> Option<T> {
        self.slice.get(index).copied()
    }

    fn len(&self) -> usize {
        self.slice.len()
    }
}

/// Encapsulates slice of raw bytes to prevent it from being accidentally accessed.
#[doc(hidden)]
#[derive(Debug, PartialEq)]
pub struct AuxArrayRawLeBytes<'a, T> {
    slice: &'a [u8],
    phantom_data: PhantomData<T>,
}

impl<T> AuxArrayRawLeBytes<'_, T>
where
    T: AuxArrayElement,
{
    fn get(&self, index: usize) -> Option<T> {
        let type_size = std::mem::size_of::<T>();
        if index * type_size + type_size > self.slice.len() {
            return None;
        }
        T::from_le_bytes(&self.slice[index * type_size..][..type_size])
    }

    fn len(&self) -> usize {
        self.slice.len() / std::mem::size_of::<T>()
    }
}

/// Aux array iterator
///
/// This struct is created by the [`AuxArray::iter`] method.
pub struct AuxArrayIter<'a, T> {
    index: usize,
    array: &'a AuxArray<'a, T>,
}

impl<T> Iterator for AuxArrayIter<'_, T>
where
    T: AuxArrayElement,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.array.get(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let array_length = self.array.len() - self.index;
        (array_length, Some(array_length))
    }
}

static DECODE_BASE: &[u8] = b"=ACMGRSVTWYHKDBN";
static ENCODE_BASE: [u8; 256] = [
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    1, 2, 4, 8, 15, 15, 15, 15, 15, 15, 15, 15, 15, 0, 15, 15, 15, 1, 14, 2, 13, 15, 15, 4, 11, 15,
    15, 12, 15, 3, 15, 15, 15, 15, 5, 6, 8, 15, 7, 9, 15, 10, 15, 15, 15, 15, 15, 15, 15, 1, 14, 2,
    13, 15, 15, 4, 11, 15, 15, 12, 15, 3, 15, 15, 15, 15, 5, 6, 8, 15, 7, 9, 15, 10, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15,
];

#[inline]
fn encoded_base(encoded_seq: &[u8], i: usize) -> u8 {
    (encoded_seq[i / 2] >> ((!i & 1) << 2)) & 0b1111
}

#[inline]
fn decode_base_unchecked(base: u8) -> &'static u8 {
    unsafe { DECODE_BASE.get_unchecked(base as usize) }
}

/// The sequence of a record.
pub struct Seq<'a> {
    pub encoded: &'a [u8],
    len: usize,
}

impl Seq<'_> {
    /// Return encoded base. Complexity: O(1).
    ///
    /// Panics if `i` is outside the sequence.
    #[inline]
    pub fn encoded_base(&self, i: usize) -> u8 {
        assert!(i < self.len, "sequence index out of bounds");
        encoded_base(self.encoded, i)
    }

    /// Return decoded sequence. Complexity: O(m) with m being the read length.
    pub fn as_bytes(&self) -> Vec<u8> {
        (0..self.len()).map(|i| self[i]).collect()
    }

    /// Return length (in bases) of the sequence.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ops::Index<usize> for Seq<'_> {
    type Output = u8;

    /// Return decoded base at given position within read. Complexity: O(1).
    ///
    /// Panics if `index` is outside the sequence.
    fn index(&self, index: usize) -> &u8 {
        decode_base_unchecked(self.encoded_base(index))
    }
}

#[derive(PartialEq, PartialOrd, Eq, Debug, Clone, Copy, Hash)]
pub enum Cigar {
    Match(u32),    // M
    Ins(u32),      // I
    Del(u32),      // D
    RefSkip(u32),  // N
    SoftClip(u32), // S
    HardClip(u32), // H
    Pad(u32),      // P
    Equal(u32),    // =
    Diff(u32),     // X
}

impl Cigar {
    const MAX_LEN: u32 = 0x0fff_ffff;

    fn encode(self) -> u32 {
        self.assert_valid_len();
        match self {
            Cigar::Match(len) => len << 4, // | 0,
            Cigar::Ins(len) => (len << 4) | 1,
            Cigar::Del(len) => (len << 4) | 2,
            Cigar::RefSkip(len) => (len << 4) | 3,
            Cigar::SoftClip(len) => (len << 4) | 4,
            Cigar::HardClip(len) => (len << 4) | 5,
            Cigar::Pad(len) => (len << 4) | 6,
            Cigar::Equal(len) => (len << 4) | 7,
            Cigar::Diff(len) => (len << 4) | 8,
        }
    }

    fn assert_valid_len(self) {
        assert!(
            self.len() <= Self::MAX_LEN,
            "CIGAR operation length must fit in 28 bits"
        );
    }

    /// Return the length of the CIGAR.
    pub fn len(self) -> u32 {
        match self {
            Cigar::Match(len)
            | Cigar::Ins(len)
            | Cigar::Del(len)
            | Cigar::RefSkip(len)
            | Cigar::SoftClip(len)
            | Cigar::HardClip(len)
            | Cigar::Pad(len)
            | Cigar::Equal(len)
            | Cigar::Diff(len) => len,
        }
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Return the character representing the CIGAR.
    pub fn char(self) -> char {
        match self {
            Cigar::Match(_) => 'M',
            Cigar::Ins(_) => 'I',
            Cigar::Del(_) => 'D',
            Cigar::RefSkip(_) => 'N',
            Cigar::SoftClip(_) => 'S',
            Cigar::HardClip(_) => 'H',
            Cigar::Pad(_) => 'P',
            Cigar::Equal(_) => '=',
            Cigar::Diff(_) => 'X',
        }
    }
}

impl fmt::Display for Cigar {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_fmt(format_args!("{}{}", self.len(), self.char()))
    }
}

custom_derive! {
    /// A CIGAR string. This type wraps around a `Vec<Cigar>`.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_htslib::bam::record::{Cigar, CigarString};
    ///
    /// let cigar = CigarString(vec![Cigar::Match(100), Cigar::SoftClip(10)]);
    ///
    /// // access by index
    /// assert_eq!(cigar[0], Cigar::Match(100));
    /// // format into classical string representation
    /// assert_eq!(format!("{}", cigar), "100M10S");
    /// // iterate
    /// for op in &cigar {
    ///    println!("{}", op);
    /// }
    /// ```
    #[derive(
        NewtypeDeref,
        NewtypeDerefMut,
        NewtypeIndex(usize),
        NewtypeIndexMut(usize),
        NewtypeFrom,
        PartialEq,
        PartialOrd,
        Eq,
        NewtypeDebug,
        Clone,
        Hash
    )]
    pub struct CigarString(pub Vec<Cigar>);
}

impl CigarString {
    /// Create a `CigarStringView` from this CigarString at position `pos`
    pub fn into_view(self, pos: i64) -> CigarStringView {
        CigarStringView::new(self, pos)
    }
}

impl<'a> CigarString {
    pub fn iter(&'a self) -> ::std::slice::Iter<'a, Cigar> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a CigarString {
    type Item = &'a Cigar;
    type IntoIter = ::std::slice::Iter<'a, Cigar>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl fmt::Display for CigarString {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        for op in self {
            fmt.write_fmt(format_args!("{}{}", op.len(), op.char()))?;
        }
        Ok(())
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct CigarStringView {
    inner: CigarString,
    pos: i64,
}

impl CigarStringView {
    /// Construct a new CigarStringView from a CigarString at a position
    pub fn new(c: CigarString, pos: i64) -> CigarStringView {
        CigarStringView { inner: c, pos }
    }

    /// Get (exclusive) end position of alignment.
    pub fn end_pos(&self) -> i64 {
        let mut pos = self.pos;
        for c in self {
            match c {
                Cigar::Match(l)
                | Cigar::RefSkip(l)
                | Cigar::Del(l)
                | Cigar::Equal(l)
                | Cigar::Diff(l) => pos += *l as i64,
                // these don't add to end_pos on reference
                Cigar::Ins(_) | Cigar::SoftClip(_) | Cigar::HardClip(_) | Cigar::Pad(_) => (),
            }
        }
        pos
    }

    /// transfer ownership of the Cigar out of the CigarView
    pub fn take(self) -> CigarString {
        self.inner
    }
}

impl ops::Deref for CigarStringView {
    type Target = CigarString;

    fn deref(&self) -> &CigarString {
        &self.inner
    }
}

impl<'a> CigarStringView {
    pub fn iter(&'a self) -> ::std::slice::Iter<'a, Cigar> {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a CigarStringView {
    type Item = &'a Cigar;
    type IntoIter = ::std::slice::Iter<'a, Cigar>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl fmt::Display for CigarStringView {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.inner.fmt(fmt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_aux_field_rejects_truncated_input() {
        let cases: &[(&str, &[u8])] = &[
            ("u8 scalar", b"C"),
            ("i16 scalar", b"s\x01"),
            ("i32 scalar", b"i\x01\0\0"),
            ("f64 scalar", b"d\0\0\0\0\0\0\0"),
            ("unterminated Z", b"Zvalue"),
            ("unterminated H", b"H0a"),
            ("B type", b"B"),
            ("B subtype", b"Bc"),
            ("B count", b"Bc\x01\0\0"),
            ("B i16 payload", b"Bs\x02\0\0\0\x01\0"),
        ];

        for (name, input) in cases {
            assert!(
                matches!(
                    Record::read_aux_field(input),
                    Err(Error::BamAuxParsingError)
                ),
                "{}",
                name
            );
            assert!(
                matches!(Record::aux_field_len(input), Err(Error::BamAuxParsingError)),
                "{}",
                name
            );
        }
    }

    #[test]
    fn read_aux_field_accepts_zero_length_arrays() {
        let cases: &[&[u8]] = &[b"Bc\0\0\0\0", b"Bs\0\0\0\0", b"Bf\0\0\0\0"];

        for input in cases {
            let (aux, field_len) = Record::read_aux_field(input).unwrap();
            assert_eq!(field_len, 6);
            assert_eq!(Record::aux_field_len(input), Ok(6));
            let array_len = match aux {
                Aux::ArrayI8(array) => array.len(),
                Aux::ArrayI16(array) => array.len(),
                Aux::ArrayFloat(array) => array.len(),
                _ => panic!("unexpected array type"),
            };
            assert_eq!(array_len, 0);
        }
    }

    #[test]
    fn aux_rejects_array_length_past_record_data() {
        let mut record = Record::new();
        record.set(b"read", None, b"A", b"I");
        record
            .push_aux(b"XA", Aux::ArrayI8((&[1_i8][..]).into()))
            .unwrap();

        let aux_offset = record.qname_capacity() + record.seq_len().div_ceil(2) + record.seq_len();
        let data =
            unsafe { slice::from_raw_parts_mut(record.inner.data, record.inner.l_data as usize) };
        data[aux_offset + 4..aux_offset + 8].copy_from_slice(&u32::MAX.to_le_bytes());

        assert_eq!(record.aux(b"XA"), Err(Error::BamAuxParsingError));
        assert_eq!(record.aux(b"ZZ"), Err(Error::BamAuxParsingError));
    }

    #[test]
    fn aux_skips_non_utf8_strings_before_the_requested_tag() {
        let mut record = Record::new();
        record.set(b"read", None, b"A", b"I");
        record.push_aux(b"AA", Aux::String("x")).unwrap();
        record.push_aux(b"NM", Aux::I32(1)).unwrap();

        let aux_offset = record.qname_capacity() + record.seq_len().div_ceil(2) + record.seq_len();
        let data =
            unsafe { slice::from_raw_parts_mut(record.inner.data, record.inner.l_data as usize) };
        data[aux_offset + 3] = 0xff;

        assert_eq!(record.aux(b"NM"), Ok(Aux::I32(1)));
        assert_eq!(record.aux(b"AA"), Err(Error::BamAuxParsingError));
    }

    #[test]
    fn aux_looks_up_mixed_valid_fields() {
        let mut record = Record::new();
        record.set(b"read", None, b"A", b"I");
        record.push_aux(b"AA", Aux::String("value")).unwrap();
        record.push_aux(b"NM", Aux::I32(1)).unwrap();
        record
            .push_aux(b"XC", Aux::ArrayI16((&[2_i16, 3][..]).into()))
            .unwrap();

        assert_eq!(record.aux(b"AA"), Ok(Aux::String("value")));
        assert_eq!(record.aux(b"NM"), Ok(Aux::I32(1)));
        assert!(matches!(
            record.aux(b"XC"),
            Ok(Aux::ArrayI16(array)) if array.iter().eq([2, 3])
        ));
        assert_eq!(record.aux(b"ZZ"), Err(Error::BamAuxTagNotFound));
    }

    #[test]
    fn seq_odd_length_returns_final_base() {
        let mut record = Record::new();
        record.set(b"read", None, b"ACG", &[30; 3]);

        let seq = record.seq();
        assert_eq!(seq.encoded_base(2), 4);
        assert_eq!(seq[2], b'G');
    }

    #[test]
    #[should_panic(expected = "sequence index out of bounds")]
    fn seq_odd_length_rejects_padding_base() {
        let mut record = Record::new();
        record.set(b"read", None, b"ACG", &[30; 3]);

        let seq = record.seq();
        let _ = seq.encoded_base(3);
    }

    #[test]
    #[should_panic(expected = "sequence index out of bounds")]
    fn seq_index_rejects_odd_length_padding_base() {
        let mut record = Record::new();
        record.set(b"read", None, b"ACG", &[30; 3]);

        let seq = record.seq();
        let _ = seq[3];
    }

    #[test]
    fn test_cigar_string() {
        let cigar = CigarString(vec![Cigar::Match(100), Cigar::SoftClip(10)]);

        assert_eq!(cigar[0], Cigar::Match(100));
        assert_eq!(format!("{}", cigar), "100M10S");
        for op in &cigar {
            println!("{}", op);
        }
    }

    #[test]
    fn test_set_pos_invalidates_cached_cigar() {
        let mut record = Record::new();
        let cigar = CigarString(vec![Cigar::Match(1)]);
        record.set(b"read", Some(&cigar), b"A", &[30]);
        record.set_pos(10);
        record.cache_cigar();

        record.set_pos(100);

        assert!(record.cigar_cached().is_none());
        assert_eq!(record.cigar().end_pos(), 101);
    }

    #[test]
    fn cigar_lengths_must_fit_in_bam_encoding() {
        let valid_cigars = [
            Cigar::Match(Cigar::MAX_LEN),
            Cigar::Ins(Cigar::MAX_LEN),
            Cigar::Del(Cigar::MAX_LEN),
            Cigar::RefSkip(Cigar::MAX_LEN),
            Cigar::SoftClip(Cigar::MAX_LEN),
            Cigar::HardClip(Cigar::MAX_LEN),
            Cigar::Pad(Cigar::MAX_LEN),
            Cigar::Equal(Cigar::MAX_LEN),
            Cigar::Diff(Cigar::MAX_LEN),
        ];
        for cigar in valid_cigars {
            assert_eq!(cigar.encode() >> 4, Cigar::MAX_LEN);
        }
        assert!(std::panic::catch_unwind(|| Cigar::Match(Cigar::MAX_LEN + 1).encode()).is_err());

        let mut record = Record::new();
        let cigar = CigarString(vec![Cigar::Match(Cigar::MAX_LEN)]);
        record.set(b"max", Some(&cigar), b"A", &[30]);
        assert_eq!(record.raw_cigar(), &[0xffff_fff0]);
    }

    #[test]
    fn set_rejects_unencodable_cigar_before_mutating_record() {
        let mut record = Record::new();
        let valid_cigar = CigarString(vec![Cigar::Match(1)]);
        record.set(b"original", Some(&valid_cigar), b"A", &[30]);
        record.cache_cigar();
        let raw_cigar = record.raw_cigar().to_vec();

        let invalid_cigar = CigarString(vec![Cigar::Match(1), Cigar::Del(Cigar::MAX_LEN + 1)]);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            record.set(b"replacement", Some(&invalid_cigar), b"C", &[20]);
        }))
        .is_err());

        assert_eq!(record.qname(), b"original");
        assert_eq!(record.raw_cigar(), raw_cigar);
        assert_eq!(record.cigar()[0], Cigar::Match(1));
        assert!(record.cigar_cached().is_some());
        assert_eq!(record.seq().as_bytes(), b"A");
        assert_eq!(record.qual(), &[30]);
    }
}
