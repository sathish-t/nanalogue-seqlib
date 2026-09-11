use rust_htslib::bam::HeaderView;

fn main() {
    let mut header = HeaderView::from_bytes(b"@HD\tVN:1.6\n");
    header.inner_mut().n_targets = 0;
}
