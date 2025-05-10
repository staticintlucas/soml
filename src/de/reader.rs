use core::str;

use super::error::{ErrorKind, Result};

/// Read from a string
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    /// Create a TOML reader from a string slice.
    #[doc(hidden)]
    #[inline]
    pub const fn from_str(str: &'a str) -> Self {
        Self::from_slice(str.as_bytes())
    }

    /// Create a TOML reader from a byte slice.
    #[doc(hidden)]
    #[inline]
    pub const fn from_slice(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Gets the next byte from the source. Returns `Ok(None)` if the end of the source is reached.
    #[doc(hidden)]
    #[inline]
    pub fn next(&mut self) -> Option<u8> {
        self.peek().inspect(|_| self.offset += 1)
    }

    /// Gets `n` bytes from the source. Returns `Ok(None)` if the end of the source is reached
    /// before `n` bytes are read.
    #[doc(hidden)]
    #[inline]
    pub fn next_n(&mut self, n: usize) -> Option<Vec<u8>> {
        self.peek_n(n).inspect(|_| self.offset += n)
    }

    /// Gets the next char from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Raises an encoding error if the bytes are not UTF-8 encoded.
    #[doc(hidden)]
    #[inline]
    pub fn next_char(&mut self) -> Result<Option<char>> {
        Ok((self.peek_char()?).inspect(|char| {
            self.discard_n(char.len_utf8());
        }))
    }

    /// Peeks the next byte from the source. Returns `Ok(None)` if the end of the source is reached.
    #[doc(hidden)]
    #[inline]
    pub fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }

    /// Peeks the next `n` bytes from the source. Returns `Ok(None)` if the end of the source is
    /// reached before `n`  bytes are read.
    #[doc(hidden)]
    #[inline]
    pub fn peek_n(&self, n: usize) -> Option<Vec<u8>> {
        self.bytes
            .get(self.offset..self.offset + n)
            .map(<[_]>::to_vec)
    }

    /// Peeks the next char from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Raises an encoding error if the bytes are not UTF-8 encoded.
    #[doc(hidden)]
    #[inline]
    pub fn peek_char(&self) -> Result<Option<char>> {
        let Some(first) = self.peek() else {
            return Ok(None);
        };
        Ok(str::from_utf8(
            self.peek_n(utf8_len(first).ok_or(ErrorKind::InvalidEncoding)?)
                .ok_or(ErrorKind::InvalidEncoding)?
                .as_ref(),
        )
        .map_err(|_| ErrorKind::InvalidEncoding)?
        .chars()
        .next())
    }

    /// Peeks the byte at `pos` bytes from the current location in the source. If the end of the
    /// source is reached, returns `Ok(None)`.
    #[doc(hidden)]
    #[inline]
    pub fn peek_at(&self, pos: usize) -> Option<u8> {
        self.bytes.get(self.offset + pos).copied()
    }

    /// Discards a byte from the source.
    #[doc(hidden)]
    #[inline]
    pub fn discard(&mut self) {
        self.discard_n(1);
    }

    /// Discards `n` bytes from the source.
    #[doc(hidden)]
    #[inline]
    pub fn discard_n(&mut self, n: usize) {
        self.offset = usize::min(self.offset + n, self.bytes.len());
    }

    /// Gets the next byte from the source if the closure `true`. Otherwise returns `Ok(None)`
    #[doc(hidden)]
    #[inline]
    pub fn next_if(&mut self, func: impl FnOnce(&u8) -> bool) -> Option<u8> {
        match self.peek() {
            Some(ch) if func(&ch) => {
                self.discard();
                Some(ch)
            }
            _ => None,
        }
    }

    /// Gets a slice of bytes from the stream where the closure returns `true`. Returns an empty
    /// slice if no bytes matched.
    #[doc(hidden)]
    #[inline]
    pub fn next_while(&mut self, func: impl Fn(&u8) -> bool) -> Vec<u8> {
        let result = self.peek_while(func);
        self.discard_n(result.len());
        result
    }

    /// Gets an string from the stream where the closure returns `true`. Returns an empty slice if
    /// no bytes matched.
    ///
    /// # Errors
    ///
    /// Raises an encoding error if the bytes are not UTF-8 encoded.
    #[doc(hidden)]
    #[inline]
    pub fn next_str_while(&mut self, func: impl Fn(&u8) -> bool) -> Result<String> {
        String::from_utf8(self.next_while(func)).map_err(|_| ErrorKind::InvalidEncoding.into())
    }

    /// Peeks a slice of bytes from the stream where the closure returns `true`. Returns an empty
    /// slice if no bytes matched.
    #[doc(hidden)]
    #[inline]
    pub fn peek_while(&self, func: impl Fn(&u8) -> bool) -> Vec<u8> {
        let off = self.offset;
        let len = self.bytes[off..].iter().copied().take_while(func).count();
        self.bytes[off..off + len].to_vec()
    }

    /// Consumes the next byte if it is equal to the expected value. Returns whether or not the
    /// byte was consumed.
    #[doc(hidden)]
    #[inline]
    pub fn eat_char(&mut self, expected: u8) -> bool {
        self.next_if(|&ch| ch == expected).is_some()
    }

    /// Consumes a slice if it matches the expected value. Returns whether or not the
    /// byte was consumed.
    #[doc(hidden)]
    #[inline]
    pub fn eat_str(&mut self, str: &'_ [u8]) -> bool {
        let result = self.bytes[self.offset..].starts_with(str);
        if result {
            self.offset += str.len();
        }
        result
    }
}

#[inline]
const fn utf8_len(byte: u8) -> Option<usize> {
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

    use super::*;
    use crate::de::Error;

    #[test]
    fn slice_reader_from_str() {
        let s = r"
            [a]
            b = 1
            c = 2
            d = 3
        ";
        let r = Reader::from_str(s);

        assert_eq!(r.bytes, s.as_bytes());
        assert_eq!(r.offset, 0);
    }

    #[test]
    fn slice_reader_from_slice() {
        let s = b"
            [a]
            b = 1
            c = 2
            d = 3
        ";
        let r = Reader::from_slice(s);

        assert_eq!(r.bytes, s);
        assert_eq!(r.offset, 0);
    }

    #[test]
    fn slice_reader_next() {
        let mut reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        assert_matches!(reader.next(), Some(b'f'));
        assert_eq!(reader.offset, 1);

        assert_matches!(reader.next(), Some(b'o'));
        assert_eq!(reader.offset, 2);

        assert_matches!(reader.next(), Some(b'o'));
        assert_eq!(reader.offset, 3);

        assert_matches!(reader.next(), None);
        assert_eq!(reader.offset, 3);

        assert_matches!(reader.next(), None);
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_next_n() {
        let mut reader = Reader {
            bytes: b"foo bar baz",
            offset: 0,
        };

        assert_matches!(reader.next_n(2), Some(b) if &*b == b"fo");
        assert_eq!(reader.offset, 2);

        assert_matches!(reader.next_n(5), Some(b) if &*b == b"o bar");
        assert_eq!(reader.offset, 7);

        assert_matches!(reader.next_n(5), None);
        assert_eq!(reader.offset, 7);

        assert_matches!(reader.next_n(4), Some(b) if &*b == b" baz");
        assert_eq!(reader.offset, 11);
    }

    #[test]
    fn slice_reader_next_char() {
        let mut reader = Reader {
            bytes: b"f",
            offset: 0,
        };

        assert_matches!(reader.next_char(), Ok(Some('f')));
        assert_matches!(reader.next_char(), Ok(None));

        let mut reader = Reader {
            bytes: b"\xff",
            offset: 0,
        };
        assert_matches!(reader.next_char(), Err(Error(ErrorKind::InvalidEncoding)));

        let mut reader = Reader {
            bytes: b"\xcf\xff",
            offset: 0,
        };
        assert_matches!(reader.next_char(), Err(Error(ErrorKind::InvalidEncoding)));

        let mut reader = Reader {
            bytes: b"\xcf",
            offset: 0,
        };
        assert_matches!(reader.next_char(), Err(Error(ErrorKind::InvalidEncoding)));
    }

    #[test]
    fn slice_reader_peek() {
        let reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        assert_matches!(reader.peek(), Some(b'f'));
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek(), Some(b'f'));
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_peek_n() {
        let reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        assert_matches!(reader.peek_n(2), Some(b) if &*b == b"fo");
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_n(4), None);
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_n(3), Some(b) if &*b == b"foo");
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_peek_char() {
        let reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        assert_matches!(reader.peek_char(), Ok(Some('f')));
        assert_eq!(reader.offset, 0);

        let reader = Reader {
            bytes: b"",
            offset: 0,
        };

        assert_matches!(reader.peek_char(), Ok(None));

        let reader = Reader {
            bytes: b"\xff",
            offset: 0,
        };
        assert_matches!(reader.peek_char(), Err(Error(ErrorKind::InvalidEncoding)));

        let reader = Reader {
            bytes: b"\xcf\xff",
            offset: 0,
        };
        assert_matches!(reader.peek_char(), Err(Error(ErrorKind::InvalidEncoding)));

        let reader = Reader {
            bytes: b"\xcf",
            offset: 0,
        };
        assert_matches!(reader.peek_char(), Err(Error(ErrorKind::InvalidEncoding)));
    }

    #[test]
    fn slice_reader_peek_at() {
        let reader = Reader {
            bytes: b"bar",
            offset: 0,
        };

        assert_matches!(reader.peek_at(1), Some(b'a'));
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_at(3), None);
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_at(2), Some(b'r'));
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_discard() {
        let mut reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        reader.discard();
        assert_eq!(reader.offset, 1);

        reader.discard();
        assert_eq!(reader.offset, 2);

        reader.discard();
        assert_eq!(reader.offset, 3);

        reader.discard();
        assert_eq!(reader.offset, 3);

        reader.discard();
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_discard_n() {
        let mut reader = Reader {
            bytes: b"foo bar baz",
            offset: 0,
        };

        reader.discard_n(2);
        assert_eq!(reader.offset, 2);

        reader.discard_n(5);
        assert_eq!(reader.offset, 7);

        reader.discard_n(12);
        assert_eq!(reader.offset, 11);
    }

    #[test]
    fn slice_reader_next_if() {
        let mut reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        assert_matches!(reader.next_if(|&ch| ch == b'f'), Some(b'f'));
        assert_eq!(reader.offset, 1);

        assert_matches!(reader.next_if(|&ch| ch == b'f'), None);
        assert_eq!(reader.offset, 1);

        assert_matches!(reader.next_if(|&ch| ch == b'o'), Some(b'o'));
        assert_eq!(reader.offset, 2);

        assert_matches!(reader.next_if(|&ch| ch == b'o'), Some(b'o'));
        assert_eq!(reader.offset, 3);

        assert_matches!(reader.next_if(|&ch| ch == b'o'), None);
        assert_eq!(reader.offset, 3);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn slice_reader_next_while() {
        let mut reader = Reader {
            bytes: b"bbbbaaaaaaararrrrrrr",
            offset: 0,
        };

        assert_matches!(reader.next_while(|&ch| ch == b'b'), b if &*b == b"bbbb");
        assert_eq!(reader.offset, 4);

        assert_matches!(reader.next_while(|&ch| ch == b'b'), b if b.is_empty());
        assert_eq!(reader.offset, 4);

        assert_matches!(reader.next_while(|&ch| ch == b'a'), b if &*b == b"aaaaaaa");
        assert_eq!(reader.offset, 11);

        assert_matches!(reader.next_while(|&ch| ch == b'a'), b if b.is_empty());
        assert_eq!(reader.offset, 11);

        assert_matches!(reader.next_while(|&ch| ch == b'r'), b if &*b == b"r");
        assert_eq!(reader.offset, 12);

        assert_matches!(reader.next_while(|&ch| ch == b'r'), b if b.is_empty());
        assert_eq!(reader.offset, 12);

        assert_matches!(reader.next_while(|&ch| ch == b'a'), b if &*b == b"a");
        assert_eq!(reader.offset, 13);

        assert_matches!(reader.next_while(|&ch| ch == b'r'), b if &*b == b"rrrrrrr");
        assert_eq!(reader.offset, 20);

        assert_matches!(reader.next_while(|&ch| ch == b'r'), b if b.is_empty());
        assert_eq!(reader.offset, 20);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn slice_reader_next_str_while() {
        let mut reader = Reader {
            bytes: b"bbbbaaaaaaararrrrrrr",
            offset: 0,
        };

        assert_matches!(reader.next_str_while(|&ch| ch == b'b'), Ok(s) if s == "bbbb");
        assert_eq!(reader.offset, 4);

        assert_matches!(reader.next_str_while(|&ch| ch == b'b'), Ok(s) if s.is_empty());
        assert_eq!(reader.offset, 4);

        assert_matches!(reader.next_str_while(|&ch| ch == b'a'), Ok(s) if s == "aaaaaaa");
        assert_eq!(reader.offset, 11);

        assert_matches!(reader.next_str_while(|&ch| ch == b'a'), Ok(s) if s.is_empty());
        assert_eq!(reader.offset, 11);

        assert_matches!(reader.next_str_while(|&ch| ch == b'r'), Ok(s) if s == "r");
        assert_eq!(reader.offset, 12);

        assert_matches!(reader.next_str_while(|&ch| ch == b'r'), Ok(s) if s.is_empty());
        assert_eq!(reader.offset, 12);

        assert_matches!(reader.next_str_while(|&ch| ch == b'a'), Ok(s) if s == "a");
        assert_eq!(reader.offset, 13);

        assert_matches!(reader.next_str_while(|&ch| ch == b'r'), Ok(s) if s == "rrrrrrr");
        assert_eq!(reader.offset, 20);

        assert_matches!(reader.next_str_while(|&ch| ch == b'r'), Ok(s) if s.is_empty());
        assert_eq!(reader.offset, 20);

        let mut reader = Reader {
            bytes: b"\xff\xff\xff\xff",
            offset: 0,
        };
        assert_matches!(
            reader.next_str_while(|&ch| ch == b'\xff'),
            Err(Error(ErrorKind::InvalidEncoding))
        );
    }

    #[test]
    fn slice_reader_peek_while() {
        let reader = Reader {
            bytes: b"foo bar baz",
            offset: 0,
        };

        assert_matches!(reader.peek_while(|&ch| ch == b'f'), b if &*b == b"f");
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_while(|&ch| ch == b'o'), b if &*b == b"");
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_while(|&ch| !ch.is_ascii_whitespace()), b if &*b == b"foo");
        assert_eq!(reader.offset, 0);

        assert_matches!(reader.peek_while(|&ch| ch.is_ascii()), b if &*b == b"foo bar baz");
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_eat_char() {
        let mut reader = Reader {
            bytes: b"foo",
            offset: 0,
        };

        assert_matches!(reader.eat_char(b'f'), true);
        assert_eq!(reader.offset, 1);

        assert_matches!(reader.eat_char(b'f'), false);
        assert_eq!(reader.offset, 1);

        assert_matches!(reader.eat_char(b'o'), true);
        assert_eq!(reader.offset, 2);

        assert_matches!(reader.eat_char(b'o'), true);
        assert_eq!(reader.offset, 3);

        assert_matches!(reader.eat_char(b'o'), false);
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_eat_str() {
        let mut reader = Reader {
            bytes: b"foobar",
            offset: 0,
        };

        assert_matches!(reader.eat_str(b"foo"), true);
        assert_eq!(reader.offset, 3);

        assert_matches!(reader.eat_str(b"foo"), false);
        assert_eq!(reader.offset, 3);

        assert_matches!(reader.eat_str(b"bar"), true);
        assert_eq!(reader.offset, 6);

        assert_matches!(reader.eat_str(b"bar"), false);
        assert_eq!(reader.offset, 6);

        assert_matches!(reader.eat_str(b"baz"), false);
        assert_eq!(reader.offset, 6);
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
