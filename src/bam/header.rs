// Copyright 2014 Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

/// A BAM header.
#[derive(Debug, Clone)]
pub struct Header {
    records: Vec<Vec<u8>>,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Header {
    /// Create a new header.
    pub fn new() -> Self {
        Header {
            records: Vec::new(),
        }
    }

    /// Add a record to the header.
    pub fn push_record(&mut self, record: &HeaderRecord<'_>) -> &mut Self {
        self.records.push(record.to_bytes());
        self
    }

    /// Add a comment to the header.
    pub fn push_comment(&mut self, comment: &[u8]) -> &mut Self {
        self.records.push([&b"@CO"[..], comment].join(&b'\t'));
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records.join(&b'\n')
    }
}

/// Header record.
#[derive(Debug, Clone)]
pub struct HeaderRecord<'a> {
    rec_type: Vec<u8>,
    tags: Vec<(&'a [u8], Vec<u8>)>,
}

impl<'a> HeaderRecord<'a> {
    /// Create a new header record.
    /// See SAM format specification for possible record types.
    pub fn new(rec_type: &'a [u8]) -> Self {
        HeaderRecord {
            rec_type: [&b"@"[..], rec_type].concat(),
            tags: Vec::new(),
        }
    }

    /// Add a new tag to the record.
    ///
    /// # Arguments
    ///
    /// * `tag` - the tag identifier
    /// * `value` - the value. Can be any type convertible into a string. Preferably numbers or
    ///   strings.
    pub fn push_tag<V: ToString>(&mut self, tag: &'a [u8], value: V) -> &mut Self {
        self.tags.push((tag, value.to_string().into_bytes()));
        self
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(self.rec_type.iter());
        for &(tag, ref value) in self.tags.iter() {
            out.push(b'\t');
            out.extend(tag.iter());
            out.push(b':');
            out.extend(value.iter());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::HeaderRecord;

    #[test]
    fn test_push_tag() {
        let mut record = HeaderRecord::new(b"HD");
        record.push_tag(b"X1", 0);
        record.push_tag(b"X2", 0);

        let x = "x".to_string();
        record.push_tag(b"X3", x.as_str());
        record.push_tag(b"X4", &x);
        record.push_tag(b"X5", x);

        assert_eq!(record.to_bytes(), b"@HD\tX1:0\tX2:0\tX3:x\tX4:x\tX5:x");
    }
}
