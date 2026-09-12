// Copyright 2019 Johannes Köster and Florian Finkernagel.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

//! Extensions for BAM records beyond htslib

use crate::bam;
use crate::bam::record::Cigar;
use crate::htslib;

pub struct IterAlignedPairsFull {
    genome_pos: i64,
    read_pos: i64,
    cigar: Vec<Cigar>,
    remaining_match_bp: u32,
    remaining_ins_bp: u32,
    remaining_del_bp: u32,
    cigar_index: usize,
}

impl Iterator for IterAlignedPairsFull {
    type Item = [Option<i64>; 2];
    fn next(&mut self) -> Option<Self::Item> {
        if self.cigar.is_empty() {
            return None;
        }
        assert!(self.genome_pos >= 0, "negative position detected!");
        assert!(self.read_pos >= 0, "negative position detected!");
        assert!(
            self.genome_pos < i64::MAX,
            "overflow detected in positions!"
        );
        assert!(self.read_pos < i64::MAX, "overflow detected in positions!");
        if self.remaining_match_bp > 0 {
            self.remaining_match_bp -= 1;
            self.genome_pos += 1;
            self.read_pos += 1;
            return Some([Some(self.read_pos - 1), Some(self.genome_pos - 1)]);
        }
        if self.remaining_ins_bp > 0 {
            self.remaining_ins_bp -= 1;
            self.read_pos += 1;
            return Some([Some(self.read_pos - 1), None]);
        }
        if self.remaining_del_bp > 0 {
            self.remaining_del_bp -= 1;
            self.genome_pos += 1;
            return Some([None, Some(self.genome_pos - 1)]);
        }

        while self.cigar_index < self.cigar.len() {
            let entry = self.cigar[self.cigar_index];
            match entry {
                Cigar::Match(len) | Cigar::Equal(len) | Cigar::Diff(len) => {
                    assert!(
                        self.genome_pos < i64::MAX,
                        "overflow detected in positions!"
                    );
                    assert!(self.read_pos < i64::MAX, "overflow detected in positions!");
                    assert!(len > 0, "malformed cigar detected!");
                    self.genome_pos += 1;
                    self.read_pos += 1;
                    self.remaining_match_bp = len - 1;
                    self.cigar_index += 1;
                    return Some([Some(self.read_pos - 1), Some(self.genome_pos - 1)]);
                }
                Cigar::Ins(len) | Cigar::SoftClip(len) => {
                    assert!(self.read_pos < i64::MAX, "overflow detected in positions!");
                    assert!(len > 0, "malformed cigar detected!");
                    self.read_pos += 1;
                    self.remaining_ins_bp = len - 1;
                    self.cigar_index += 1;
                    return Some([Some(self.read_pos - 1), None]);
                }
                Cigar::Del(len) | Cigar::RefSkip(len) => {
                    assert!(
                        self.genome_pos < i64::MAX,
                        "overflow detected in positions!"
                    );
                    assert!(len > 0, "malformed cigar detected!");
                    self.genome_pos += 1;
                    self.remaining_del_bp = len - 1;
                    self.cigar_index += 1;
                    return Some([None, Some(self.genome_pos - 1)]);
                }
                Cigar::HardClip(_) | Cigar::Pad(_) => {
                    // SAM CIGAR H and P consume neither query nor reference.
                    // We've not run into padded BAMs in our normal workflow.
                    // We may have to deal with this in the future if we run
                    // into one such file.
                }
            }
            self.cigar_index += 1;
        }
        None
    }
}

/// Extra functionality for BAM records
///
/// Inspired by pysam
pub trait BamRecordExtensions {
    /// iter list of read and reference positions on a basepair level.
    ///
    /// Returns None in either the read position or the reference position
    /// for insertions, deletions or skipped pairs. Hard clips and padding consume
    /// neither coordinate and therefore produce no pair.
    ///
    /// pysam: aligned_pairs(matches_only = False)
    fn aligned_pairs_full(&self) -> IterAlignedPairsFull;

    /// right most aligned absolute reference position of the read on the reference genome.
    fn reference_end(&self) -> i64;
}

impl BamRecordExtensions for bam::Record {
    fn aligned_pairs_full(&self) -> IterAlignedPairsFull {
        IterAlignedPairsFull {
            genome_pos: self.pos(),
            read_pos: 0,
            cigar: self.cigar().take().0,
            remaining_match_bp: 0,
            remaining_ins_bp: 0,
            remaining_del_bp: 0,
            cigar_index: 0,
        }
    }

    /// Calculate the rightmost absolute reference base position of an alignment on the reference genome.
    /// Returns the coordinate of the first base after the alignment (0-based).
    fn reference_end(&self) -> i64 {
        unsafe { htslib::bam_endpos(self.inner_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use crate::bam;
    use crate::bam::ext::BamRecordExtensions;
    use crate::bam::record::{Cigar, CigarString};
    use crate::bam::Read;

    #[test]
    fn test_aligned_pairs_full_ignores_padding() {
        let cigar = CigarString(vec![Cigar::Match(1), Cigar::Pad(2), Cigar::Match(1)]);
        let mut record = bam::Record::new();
        record.set(b"padded", Some(&cigar), b"AC", &[30, 30]);
        record.set_pos(100);

        assert_eq!(
            record.aligned_pairs_full().collect::<Vec<_>>(),
            vec![[Some(0), Some(100)], [Some(1), Some(101)]]
        );
    }

    #[test]
    fn test_aligned_pairs_full() {
        let mut bam = bam::Reader::from_path("./test/test_spliced_reads.bam").unwrap();
        let mut it = bam.records();

        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(
            pairs,
            vec![
                [Some(0), None],
                [Some(1), None],
                [Some(2), None],
                [Some(3), None],
                [Some(4), None],
                [Some(5), None],
                [Some(6), Some(16050676)],
                [Some(7), Some(16050677)],
                [Some(8), Some(16050678)],
                [Some(9), Some(16050679)],
                [Some(10), Some(16050680)],
                [Some(11), Some(16050681)],
                [Some(12), Some(16050682)],
                [Some(13), Some(16050683)],
                [Some(14), Some(16050684)],
                [Some(15), Some(16050685)],
                [Some(16), Some(16050686)],
                [Some(17), Some(16050687)],
                [Some(18), Some(16050688)],
                [Some(19), Some(16050689)],
                [Some(20), Some(16050690)],
                [Some(21), Some(16050691)],
                [Some(22), Some(16050692)],
                [Some(23), Some(16050693)],
                [Some(24), Some(16050694)],
                [Some(25), Some(16050695)],
                [Some(26), Some(16050696)],
                [Some(27), Some(16050697)],
                [Some(28), Some(16050698)],
                [Some(29), Some(16050699)],
                [Some(30), Some(16050700)],
                [Some(31), Some(16050701)],
                [Some(32), Some(16050702)],
                [Some(33), Some(16050703)],
                [Some(34), Some(16050704)],
                [Some(35), Some(16050705)],
                [Some(36), Some(16050706)],
                [Some(37), Some(16050707)],
                [Some(38), Some(16050708)],
                [Some(39), Some(16050709)],
                [Some(40), Some(16050710)],
                [Some(41), Some(16050711)],
                [Some(42), Some(16050712)],
                [Some(43), Some(16050713)],
                [Some(44), Some(16050714)],
                [Some(45), Some(16050715)],
                [Some(46), Some(16050716)],
                [Some(47), Some(16050717)],
                [Some(48), Some(16050718)],
                [Some(49), Some(16050719)],
                [Some(50), Some(16050720)]
            ]
        );
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(
            pairs,
            vec![
                [Some(0), Some(16096878)],
                [Some(1), Some(16096879)],
                [Some(2), Some(16096880)],
                [Some(3), Some(16096881)],
                [Some(4), Some(16096882)],
                [Some(5), Some(16096883)],
                [Some(6), Some(16096884)],
                [None, Some(16096885)],
                [None, Some(16096886)],
                [Some(7), Some(16096887)],
                [Some(8), Some(16096888)],
                [Some(9), Some(16096889)],
                [Some(10), Some(16096890)],
                [Some(11), Some(16096891)],
                [Some(12), Some(16096892)],
                [Some(13), Some(16096893)],
                [Some(14), Some(16096894)],
                [Some(15), Some(16096895)],
                [Some(16), Some(16096896)],
                [Some(17), Some(16096897)],
                [Some(18), Some(16096898)],
                [Some(19), Some(16096899)],
                [Some(20), Some(16096900)],
                [Some(21), Some(16096901)],
                [Some(22), Some(16096902)],
                [Some(23), Some(16096903)],
                [Some(24), Some(16096904)],
                [Some(25), Some(16096905)],
                [Some(26), Some(16096906)],
                [Some(27), Some(16096907)],
                [Some(28), Some(16096908)],
                [Some(29), Some(16096909)],
                [Some(30), Some(16096910)],
                [Some(31), Some(16096911)],
                [Some(32), Some(16096912)],
                [Some(33), Some(16096913)],
                [Some(34), Some(16096914)],
                [Some(35), Some(16096915)],
                [Some(36), Some(16096916)],
                [Some(37), Some(16096917)],
                [Some(38), Some(16096918)],
                [Some(39), Some(16096919)],
                [Some(40), Some(16096920)],
                [Some(41), Some(16096921)],
                [Some(42), Some(16096922)],
                [Some(43), Some(16096923)],
                [Some(44), Some(16096924)],
                [Some(45), Some(16096925)],
                [Some(46), Some(16096926)],
                [Some(47), Some(16096927)],
                [Some(48), Some(16096928)],
                [Some(49), Some(16096929)],
                [Some(50), Some(16096930)]
            ]
        );
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(
            pairs,
            vec![
                [Some(0), Some(16097145)],
                [Some(1), Some(16097146)],
                [Some(2), Some(16097147)],
                [Some(3), Some(16097148)],
                [Some(4), Some(16097149)],
                [Some(5), Some(16097150)],
                [Some(6), Some(16097151)],
                [Some(7), Some(16097152)],
                [Some(8), Some(16097153)],
                [Some(9), Some(16097154)],
                [Some(10), Some(16097155)],
                [Some(11), Some(16097156)],
                [Some(12), Some(16097157)],
                [Some(13), Some(16097158)],
                [Some(14), Some(16097159)],
                [Some(15), Some(16097160)],
                [Some(16), Some(16097161)],
                [Some(17), Some(16097162)],
                [Some(18), Some(16097163)],
                [Some(19), Some(16097164)],
                [Some(20), Some(16097165)],
                [Some(21), Some(16097166)],
                [Some(22), Some(16097167)],
                [Some(23), Some(16097168)],
                [Some(24), Some(16097169)],
                [Some(25), Some(16097170)],
                [Some(26), Some(16097171)],
                [Some(27), Some(16097172)],
                [Some(28), Some(16097173)],
                [None, Some(16097174)],
                [None, Some(16097175)],
                [Some(29), Some(16097176)],
                [Some(30), Some(16097177)],
                [Some(31), Some(16097178)],
                [Some(32), Some(16097179)],
                [Some(33), Some(16097180)],
                [Some(34), Some(16097181)],
                [Some(35), Some(16097182)],
                [Some(36), Some(16097183)],
                [Some(37), Some(16097184)],
                [Some(38), Some(16097185)],
                [Some(39), Some(16097186)],
                [Some(40), Some(16097187)],
                [Some(41), Some(16097188)],
                [Some(42), Some(16097189)],
                [Some(43), Some(16097190)],
                [Some(44), Some(16097191)],
                [Some(45), Some(16097192)],
                [Some(46), Some(16097193)],
                [Some(47), Some(16097194)],
                [Some(48), Some(16097195)],
                [Some(49), Some(16097196)],
                [Some(50), Some(16097197)]
            ]
        );
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(
            pairs,
            vec![
                [Some(0), Some(16117350)],
                [Some(1), Some(16117351)],
                [Some(2), Some(16117352)],
                [Some(3), Some(16117353)],
                [Some(4), Some(16117354)],
                [Some(5), Some(16117355)],
                [Some(6), Some(16117356)],
                [Some(7), Some(16117357)],
                [Some(8), Some(16117358)],
                [Some(9), Some(16117359)],
                [Some(10), Some(16117360)],
                [Some(11), Some(16117361)],
                [Some(12), Some(16117362)],
                [Some(13), Some(16117363)],
                [Some(14), Some(16117364)],
                [Some(15), Some(16117365)],
                [Some(16), Some(16117366)],
                [Some(17), Some(16117367)],
                [Some(18), Some(16117368)],
                [Some(19), Some(16117369)],
                [Some(20), Some(16117370)],
                [Some(21), Some(16117371)],
                [Some(22), Some(16117372)],
                [Some(23), Some(16117373)],
                [Some(24), Some(16117374)],
                [Some(25), Some(16117375)],
                [Some(26), Some(16117376)],
                [Some(27), Some(16117377)],
                [Some(28), Some(16117378)],
                [Some(29), Some(16117379)],
                [Some(30), Some(16117380)],
                [Some(31), Some(16117381)],
                [Some(32), Some(16117382)],
                [Some(33), Some(16117383)],
                [Some(34), Some(16117384)],
                [Some(35), Some(16117385)],
                [Some(36), Some(16117386)],
                [Some(37), Some(16117387)],
                [Some(38), Some(16117388)],
                [Some(39), Some(16117389)],
                [Some(40), Some(16117390)],
                [Some(41), Some(16117391)],
                [Some(42), Some(16117392)],
                [Some(43), Some(16117393)],
                [Some(44), Some(16117394)],
                [Some(45), Some(16117395)],
                [Some(46), Some(16117396)],
                [Some(47), Some(16117397)],
                [Some(48), Some(16117398)],
                [Some(49), Some(16117399)],
                [Some(50), Some(16117400)]
            ]
        );
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(
            pairs,
            vec![
                [Some(0), Some(16118483)],
                [Some(1), Some(16118484)],
                [Some(2), Some(16118485)],
                [Some(3), Some(16118486)],
                [Some(4), Some(16118487)],
                [Some(5), Some(16118488)],
                [Some(6), Some(16118489)],
                [Some(7), Some(16118490)],
                [Some(8), Some(16118491)],
                [Some(9), Some(16118492)],
                [Some(10), Some(16118493)],
                [Some(11), Some(16118494)],
                [Some(12), Some(16118495)],
                [Some(13), Some(16118496)],
                [Some(14), Some(16118497)],
                [Some(15), Some(16118498)],
                [Some(16), Some(16118499)],
                [Some(17), Some(16118500)],
                [Some(18), Some(16118501)],
                [Some(19), Some(16118502)],
                [Some(20), Some(16118503)],
                [Some(21), Some(16118504)],
                [Some(22), Some(16118505)],
                [Some(23), Some(16118506)],
                [Some(24), Some(16118507)],
                [Some(25), Some(16118508)],
                [Some(26), Some(16118509)],
                [Some(27), Some(16118510)],
                [Some(28), Some(16118511)],
                [Some(29), Some(16118512)],
                [Some(30), Some(16118513)],
                [Some(31), Some(16118514)],
                [Some(32), Some(16118515)],
                [Some(33), Some(16118516)],
                [Some(34), Some(16118517)],
                [Some(35), Some(16118518)],
                [Some(36), Some(16118519)],
                [Some(37), Some(16118520)],
                [Some(38), Some(16118521)],
                [Some(39), Some(16118522)],
                [Some(40), Some(16118523)],
                [Some(41), Some(16118524)],
                [Some(42), Some(16118525)],
                [Some(43), Some(16118526)],
                [Some(44), Some(16118527)],
                [Some(45), Some(16118528)],
                [Some(46), Some(16118529)],
                [Some(47), Some(16118530)],
                [Some(48), Some(16118531)],
                [Some(49), Some(16118532)],
                [Some(50), Some(16118533)]
            ]
        );

        //
        //for the rest, we just verify that they have the expected amount of None in each position
        fn some_count(pairs: &[[Option<i64>; 2]], pos: usize) -> i64 {
            pairs.iter().filter(|x| x[pos].is_some()).count() as i64
        }
        fn none_count(pairs: &[[Option<i64>; 2]], pos: usize) -> i64 {
            pairs.iter().filter(|x| x[pos].is_none()).count() as i64
        }

        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 45);
        assert_eq!(none_count(&pairs, 1), 6);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 41);
        assert_eq!(none_count(&pairs, 1), 10);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 42);
        assert_eq!(none_count(&pairs, 1), 9);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 50);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 45);
        assert_eq!(none_count(&pairs, 1), 6);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 48);
        assert_eq!(none_count(&pairs, 1), 3);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 42);
        assert_eq!(none_count(&pairs, 1), 9);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 38);
        assert_eq!(none_count(&pairs, 1), 13);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 44);
        assert_eq!(none_count(&pairs, 1), 7);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 46);
        assert_eq!(none_count(&pairs, 1), 5);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 47);
        assert_eq!(none_count(&pairs, 1), 4);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 50);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 2183);
        assert_eq!(some_count(&pairs, 1), 2234);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 2183);
        assert_eq!(some_count(&pairs, 1), 2234);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 2183);
        assert_eq!(some_count(&pairs, 1), 2234);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 33);
        assert_eq!(none_count(&pairs, 1), 18);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 48);
        assert_eq!(none_count(&pairs, 1), 3);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 45);
        assert_eq!(none_count(&pairs, 1), 6);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 11832);
        assert_eq!(some_count(&pairs, 1), 11883);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 12542);
        assert_eq!(some_count(&pairs, 1), 12593);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1467);
        assert_eq!(some_count(&pairs, 1), 1517);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1609);
        assert_eq!(some_count(&pairs, 1), 1660);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1609);
        assert_eq!(some_count(&pairs, 1), 1660);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1609);
        assert_eq!(some_count(&pairs, 1), 1660);
        assert_eq!(none_count(&pairs, 1), 0);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 95);
        assert_eq!(some_count(&pairs, 1), 145);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 538);
        assert_eq!(some_count(&pairs, 1), 588);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 538);
        assert_eq!(some_count(&pairs, 1), 588);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 538);
        assert_eq!(some_count(&pairs, 1), 588);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1);
        assert_eq!(some_count(&pairs, 1), 51);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1);
        assert_eq!(some_count(&pairs, 1), 49);
        assert_eq!(none_count(&pairs, 1), 3);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1);
        assert_eq!(some_count(&pairs, 1), 45);
        assert_eq!(none_count(&pairs, 1), 7);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 0);
        assert_eq!(some_count(&pairs, 1), 39);
        assert_eq!(none_count(&pairs, 1), 12);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1892);
        assert_eq!(some_count(&pairs, 1), 1938);
        assert_eq!(none_count(&pairs, 1), 5);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 1892);
        assert_eq!(some_count(&pairs, 1), 1941);
        assert_eq!(none_count(&pairs, 1), 2);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 2288);
        assert_eq!(some_count(&pairs, 1), 2338);
        assert_eq!(none_count(&pairs, 1), 1);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 14680);
        assert_eq!(some_count(&pairs, 1), 14729);
        assert_eq!(none_count(&pairs, 1), 2);
        let pairs: Vec<_> = it.next().unwrap().unwrap().aligned_pairs_full().collect();
        assert_eq!(some_count(&pairs, 0), 51);
        assert_eq!(none_count(&pairs, 0), 4027);
        assert_eq!(some_count(&pairs, 1), 4074);
        assert_eq!(none_count(&pairs, 1), 4);
    }

    #[test]
    fn test_reference_end() {
        let mut bam = bam::Reader::from_path("./test/test_spliced_reads.bam").unwrap();
        let mut it = bam.records();
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16050721);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16096931);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16097198);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16117401);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16118534);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16118550);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16118550);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16118550);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16123462);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16123462);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16165901);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16180922);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16189756);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16231322);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16237708);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16255054);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16255442);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16255442);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16256129);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16256272);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16325241);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16352903);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16353012);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 16415044);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17031638);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17057432);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17095000);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17095016);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17095016);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17137320);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17306286);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17561913);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17577961);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578701);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578704);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578704);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578704);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578704);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578705);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578706);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17578706);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17581250);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17583029);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17583030);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17583030);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17583056);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17589209);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17589209);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17589209);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17591821);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17593904);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17593908);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17596515);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17625950);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 17625953);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 31799038);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 36737421);
        let read = it.next().unwrap().unwrap();
        assert_eq!(read.reference_end(), 44592037);
    }
}
