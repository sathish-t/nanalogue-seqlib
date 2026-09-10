const c = @cImport({
    @cInclude("htslib/faidx.h");
});

/// Build the FASTA index beside `path` using the pinned HTSlib source.
///
/// This is Nanalogue's ABI boundary: Rust deliberately does not import any
/// HTSlib types or headers.
export fn nanalogue_fai_build(path: [*:0]const u8) c_int {
    return c.fai_build(path);
}
