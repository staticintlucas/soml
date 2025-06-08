use std::str;

/// Read from a string
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    line_no: usize,
}

impl<'a> Reader<'a> {
    /// Create a TOML reader from a string slice.
    #[inline]
    pub const fn from_str(str: &'a str) -> Self {
        Self::from_slice(str.as_bytes())
    }

    /// Create a TOML reader from a byte slice.
    #[inline]
    pub const fn from_slice(bytes: &'a [u8]) -> Self {
        Self { bytes, line_no: 0 }
    }

    /// Gets the next line from the source. Returns `Ok(None)` if the end of the source is reached.
    pub fn next_line(&mut self) -> Option<&'a [u8]> {
        match self.bytes.iter().position(|&b| b == b'\n') {
            Some(offset) => {
                let line = &self.bytes[..offset]; // Exclude the \n
                let line = line.strip_suffix(b"\r").unwrap_or(line); // Windows newline

                self.bytes = &self.bytes[offset + 1..]; // Exclude the \n here too
                self.line_no += 1;

                Some(line)
            }
            None => {
                if self.bytes.is_empty() {
                    None
                } else {
                    let line = self.bytes;

                    self.bytes = &self.bytes[self.bytes.len()..];
                    self.line_no += 1;

                    Some(line)
                }
            }
        }
    }

    /// Gets the line number of the line returned by the previous call to [`Self::next_line`] read
    /// from the source.
    #[inline]
    #[allow(unused)] // TODO
    pub fn line_no(&self) -> usize {
        self.line_no
    }
}

#[inline]
pub const fn utf8_len(byte: u8) -> Option<usize> {
    match byte {
        0x00..=0x7F => Some(1),
        0xC0..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF7 => Some(4),
        _ => None,
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;

    use super::*;

    #[test]
    fn slice_reader_from_str() {
        let s = indoc! {r"
            [a]
            b = 1
            c = 2
            d = 3
        "};
        let r = Reader::from_str(s);

        assert_eq!(r.bytes, s.as_bytes());
        assert_eq!(r.line_no, 0);
    }

    #[test]
    fn slice_reader_from_slice() {
        let s = indoc! {b"
            [a]
            b = 1
            c = 2
            d = 3
        "};
        let r = Reader::from_slice(s);

        assert_eq!(r.bytes, s);
        assert_eq!(r.line_no, 0);
    }

    #[test]
    fn slice_reader_next_line() {
        let mut reader = Reader {
            bytes: indoc! {b"
                [a]
                b = 1
                c = 2
                d = 3
            "},
            line_no: 0,
        };

        assert_matches!(reader.next_line(), Some(b"[a]"));
        assert_eq!(reader.line_no, 1);

        assert_matches!(reader.next_line(), Some(b"b = 1"));
        assert_eq!(reader.line_no, 2);

        assert_matches!(reader.next_line(), Some(b"c = 2"));
        assert_eq!(reader.line_no, 3);

        assert_matches!(reader.next_line(), Some(b"d = 3"));
        assert_eq!(reader.line_no, 4);

        assert_matches!(reader.next_line(), None);
        assert_eq!(reader.line_no, 4);
    }

    #[test]
    fn slice_reader_line_no() {
        let mut reader = Reader {
            bytes: indoc! {b"
                [a]
                b = 1
                c = 2
                d = 3
            "},
            line_no: 0,
        };

        assert_eq!(reader.line_no(), 0);

        reader.line_no = 3;
        assert_eq!(reader.line_no(), 3);
    }

    #[test]
    fn test_utf8_len() {
        let mut buf = [0; 4];
        for ch in '\0'..=char::MAX {
            let str = ch.encode_utf8(&mut buf);

            assert_eq!(utf8_len(str.as_bytes()[0]), Some(str.len()));
        }
    }
}
