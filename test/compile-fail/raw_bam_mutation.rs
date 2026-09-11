use rust_htslib::bam::Record;

fn main() {
    let mut record = Record::new();
    // This was safe before the raw state was encapsulated.  Changing either
    // field breaks Record's ownership and slice-length invariants.
    record.inner.data = std::ptr::null_mut();
}
