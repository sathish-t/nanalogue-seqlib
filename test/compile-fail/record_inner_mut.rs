use rust_htslib::bam::Record;

fn main() {
    let mut record = Record::new();
    record.inner_mut().l_data = 1;
}
