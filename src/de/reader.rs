use core::str;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::io;
use std::io::Read as _;

use super::error::{ErrorKind, Result};

mod private {
    pub trait Sealed {}
}

pub trait Reader<'a>: private::Sealed {
    /// Gets the next byte from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn next(&mut self) -> Result<Option<u8>>;

    /// Gets `n` bytes from the source. Returns `Ok(None)` if the end of the source is reached
    /// before `n` bytes are read.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    fn next_n(&mut self, n: usize) -> Result<Option<Cow<'a, [u8]>>>;

    /// Gets the next char from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Raises an encoding error if the bytes are not UTF-8 encoded.
    /// Propagates any IO errors that occurred while reading from the source.
    fn next_char(&mut self) -> Result<Option<char>> {
        Ok(if let Some(char) = self.peek_char()? {
            self.discard_n(char.len_utf8())?;
            Some(char)
        } else {
            None
        })
    }

    /// Peeks the next byte from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn peek(&mut self) -> Result<Option<u8>>;

    /// Peeks the next `n` bytes from the source. Returns `Ok(None)` if the end of the source is
    /// reached before `n`  bytes are read.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn peek_n(&mut self, n: usize) -> Result<Option<Cow<'a, [u8]>>>;

    /// Peeks the next char from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Raises an encoding error if the bytes are not UTF-8 encoded.
    /// Propagates any IO errors that occurred while reading from the source.
    fn peek_char(&mut self) -> Result<Option<char>> {
        let Some(first) = self.peek()? else {
            return Ok(None);
        };
        Ok(str::from_utf8(
            self.peek_n(utf8_len(first).ok_or(ErrorKind::InvalidEncoding)?)?
                .ok_or(ErrorKind::InvalidEncoding)?
                .as_ref(),
        )
        .map_err(|_| ErrorKind::InvalidEncoding)?
        .chars()
        .next())
    }

    /// Peeks the byte at `pos` bytes from the current location in the source. If the end of the
    /// source is reached, returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn peek_at(&mut self, pos: usize) -> Result<Option<u8>>;

    /// Discards a byte from the source.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn discard(&mut self) -> Result<()> {
        self.discard_n(1)
    }

    /// Discards `n` bytes from the source.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn discard_n(&mut self, n: usize) -> Result<()>;

    /// Gets the next byte from the source if the closure `true`. Otherwise returns `Ok(None)`
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn next_if(&mut self, func: impl FnOnce(&u8) -> bool) -> Result<Option<u8>> {
        match self.peek()? {
            Some(ch) if func(&ch) => self.discard().map(|()| Some(ch)),
            _ => Ok(None),
        }
    }

    /// Gets a slice of bytes from the stream where the closure returns `true`. Returns an empty
    /// slice if no bytes matched.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn next_while(&mut self, func: impl Fn(&u8) -> bool) -> Result<Cow<'a, [u8]>> {
        let result = self.peek_while(func)?;
        self.discard_n(result.len())?;
        Ok(result)
    }

    /// Gets an string from the stream where the closure returns `true`. Returns an empty slice if
    /// no bytes matched.
    ///
    /// # Errors
    ///
    /// Raises an encoding error if the bytes are not UTF-8 encoded.
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn next_str_while(&mut self, func: impl Fn(&u8) -> bool) -> Result<Cow<'a, str>> {
        match self.next_while(func)? {
            Cow::Borrowed(bytes) => str::from_utf8(bytes)
                .map(Cow::Borrowed)
                .map_err(|_| ErrorKind::InvalidEncoding.into()),
            Cow::Owned(vec) => String::from_utf8(vec)
                .map(Cow::Owned)
                .map_err(|_| ErrorKind::InvalidEncoding.into()),
        }
    }

    /// Peeks a slice of bytes from the stream where the closure returns `true`. Returns an empty
    /// slice if no bytes matched.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn peek_while(&mut self, func: impl Fn(&u8) -> bool) -> Result<Cow<'a, [u8]>>;

    /// Consumes the next byte if it is equal to the expected value. Returns whether or not the
    /// byte was consumed.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn eat_char(&mut self, expected: u8) -> Result<bool> {
        Ok(self.next_if(|&ch| ch == expected)?.is_some())
    }

    /// Consumes a slice if it matches the expected value. Returns whether or not the
    /// byte was consumed.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn eat_str(&mut self, str: &'_ [u8]) -> Result<bool>;

    /// Start collecting consumed bytes as they are parsed.
    fn start_seq(&mut self);

    /// Stop collecting bytes and returns the collected sequence.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    fn end_seq(&mut self) -> Result<Cow<'a, [u8]>>;
}

/// Read from a string
#[derive(Debug, Clone)]
pub struct SliceReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    seq_start: Option<usize>,
}

impl<'a> SliceReader<'a> {
    /// Create a TOML reader from a string slice.
    pub const fn from_str(str: &'a str) -> Self {
        Self::from_slice(str.as_bytes())
    }

    /// Create a TOML reader from a byte slice.
    pub const fn from_slice(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            seq_start: None,
        }
    }
}

impl private::Sealed for SliceReader<'_> {}

impl<'a> Reader<'a> for SliceReader<'a> {
    fn next(&mut self) -> Result<Option<u8>> {
        Ok(self.peek()?.inspect(|_| self.offset += 1))
    }

    fn next_n(&mut self, n: usize) -> Result<Option<Cow<'a, [u8]>>> {
        Ok(self.peek_n(n)?.inspect(|_| self.offset += n))
    }

    fn peek(&mut self) -> Result<Option<u8>> {
        Ok(self.bytes.get(self.offset).copied())
    }

    fn peek_n(&mut self, n: usize) -> Result<Option<Cow<'a, [u8]>>> {
        Ok(self
            .bytes
            .get(self.offset..self.offset + n)
            .map(Cow::Borrowed))
    }

    fn peek_at(&mut self, pos: usize) -> Result<Option<u8>> {
        Ok(self.bytes.get(self.offset + pos).copied())
    }

    fn discard_n(&mut self, n: usize) -> Result<()> {
        self.offset = usize::min(self.offset + n, self.bytes.len());
        Ok(())
    }

    fn peek_while(&mut self, func: impl Fn(&u8) -> bool) -> Result<Cow<'a, [u8]>> {
        let off = self.offset;
        let len = self.bytes[off..].iter().copied().take_while(func).count();
        Ok(Cow::Borrowed(&self.bytes[off..off + len]))
    }

    fn eat_str(&mut self, str: &'_ [u8]) -> Result<bool> {
        let result = self.bytes[self.offset..].starts_with(str);
        if result {
            self.offset += str.len();
        }
        Ok(result)
    }

    fn start_seq(&mut self) {
        self.seq_start = Some(self.offset);
    }

    #[allow(clippy::panic)]
    fn end_seq(&mut self) -> Result<Cow<'a, [u8]>> {
        let Some(start) = self.seq_start.take() else {
            panic!("Reader::end_seq called without calling Reader::start_seq first")
        };
        Ok(Cow::Borrowed(&self.bytes[start..self.offset]))
    }
}

/// Read from a string
#[derive(Debug)]
pub struct IoReader<R> {
    iter: io::Bytes<io::BufReader<R>>,
    peek: VecDeque<u8>,
    seq: Option<Vec<u8>>,
}

impl<R> IoReader<R>
where
    R: io::Read,
{
    /// Create a JSON reader from a [`io::Read`].
    pub fn from_reader(read: R) -> Self {
        Self {
            iter: io::BufReader::new(read).bytes(),
            peek: VecDeque::with_capacity(16),
            seq: None,
        }
    }
}

impl<R> private::Sealed for IoReader<R> {}

impl<'a, R> Reader<'a> for IoReader<R>
where
    R: io::Read,
{
    fn next(&mut self) -> Result<Option<u8>> {
        let result = match self.peek.pop_front() {
            Some(ch) => Some(ch),
            None => self.iter.next().transpose()?,
        };

        if let Some((ch, seq)) = result.zip(self.seq.as_mut()) {
            seq.push(ch);
        }

        Ok(result)
    }

    fn next_n(&mut self, n: usize) -> Result<Option<Cow<'a, [u8]>>> {
        while self.peek.len() < n {
            match self.iter.next().transpose()? {
                Some(ch) => self.peek.push_back(ch),
                None => return Ok(None), // return None if we can't get n bytes
            }
        }

        let result: Vec<_> = self.peek.drain(..n).collect();

        if let Some(seq) = self.seq.as_mut() {
            seq.extend_from_slice(result.as_ref());
        }

        Ok(Some(Cow::Owned(result)))
    }

    fn peek(&mut self) -> Result<Option<u8>> {
        if let Some(ch) = self.peek.front() {
            Ok(Some(*ch))
        } else {
            let Some(ch) = self.iter.next().transpose()? else {
                return Ok(None);
            };
            self.peek.push_back(ch);
            Ok(Some(ch))
        }
    }

    fn peek_n(&mut self, n: usize) -> Result<Option<Cow<'a, [u8]>>> {
        while self.peek.len() < n {
            match self.iter.next().transpose()? {
                Some(ch) => self.peek.push_back(ch),
                None => return Ok(None),
            }
        }

        let result = self.peek.range(..n).copied().collect();

        Ok(Some(Cow::Owned(result)))
    }

    fn peek_at(&mut self, pos: usize) -> Result<Option<u8>> {
        while self.peek.len() < pos + 1 {
            match self.iter.next().transpose()? {
                Some(ch) => self.peek.push_back(ch),
                None => break,
            }
        }

        Ok(self.peek.get(pos).copied())
    }

    fn discard_n(&mut self, n: usize) -> Result<()> {
        if let Some(seq) = self.seq.as_mut() {
            seq.reserve(n);
        }

        let peeked_n = n.min(self.peek.len());
        let peeked = self.peek.drain(..peeked_n);
        if let Some(seq) = self.seq.as_mut() {
            seq.extend(peeked);
        }

        for _ in peeked_n..n {
            match self.iter.next().transpose()? {
                Some(ch) => {
                    if let Some(seq) = self.seq.as_mut() {
                        seq.push(ch);
                    }
                }
                None => break,
            }
        }

        Ok(())
    }

    fn peek_while(&mut self, func: impl Fn(&u8) -> bool) -> Result<Cow<'a, [u8]>> {
        if let Some(i) = self.peek.iter().position(|ch| !func(ch)) {
            Ok(self.peek.range(..i).copied().collect())
        } else {
            loop {
                match self.iter.next().transpose()? {
                    Some(ch) if func(&ch) => {
                        self.peek.push_back(ch);
                    }
                    Some(ch) => {
                        // Collect before pushing the non-matching char
                        let result = self.peek.iter().copied().collect();
                        // But make sure to push it after so we don't lose a char
                        self.peek.push_back(ch);

                        break Ok(result);
                    }
                    None => break Ok(self.peek.iter().copied().collect()),
                }
            }
        }
    }

    fn eat_str(&mut self, str: &'_ [u8]) -> Result<bool> {
        while self.peek.len() < str.len() {
            match self.iter.next().transpose()? {
                Some(ch) => self.peek.push_back(ch),
                None => return Ok(false),
            }
        }

        let result = str.iter().zip(self.peek.iter()).all(|(a, b)| a == b);

        if result {
            self.peek.drain(..str.len());

            if let Some(seq) = self.seq.as_mut() {
                seq.extend_from_slice(str);
            }
        }

        Ok(result)
    }

    fn start_seq(&mut self) {
        self.seq = Some(Vec::with_capacity(16));
    }

    #[allow(clippy::panic)]
    fn end_seq(&mut self) -> Result<Cow<'a, [u8]>> {
        let Some(seq) = self.seq.take() else {
            panic!("Reader::end_seq called without calling Reader::start_seq first")
        };
        Ok(Cow::Owned(seq))
    }
}

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
    use std::io::Read as _;

    use super::*;

    #[test]
    fn slice_reader_from_str() {
        let s = r"
            [a]
            b = 1
            c = 2
            d = 3
        ";
        let r = SliceReader::from_str(s);

        assert_eq!(r.bytes, s.as_bytes());
        assert_eq!(r.offset, 0);
        assert_eq!(r.seq_start, None);
    }

    #[test]
    fn slice_reader_from_slice() {
        let s = b"
            [a]
            b = 1
            c = 2
            d = 3
        ";
        let r = SliceReader::from_slice(s);

        assert_eq!(r.bytes, s);
        assert_eq!(r.offset, 0);
        assert_eq!(r.seq_start, None);
    }

    #[test]
    fn slice_reader_next() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.next().unwrap(), Some(b'f'));
        assert_eq!(reader.offset, 1);

        assert_eq!(reader.next().unwrap(), Some(b'o'));
        assert_eq!(reader.offset, 2);

        assert_eq!(reader.next().unwrap(), Some(b'o'));
        assert_eq!(reader.offset, 3);

        assert_eq!(reader.next().unwrap(), None);
        assert_eq!(reader.offset, 3);

        assert_eq!(reader.next().unwrap(), None);
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_next_n() {
        let mut reader = SliceReader {
            bytes: b"foo bar baz",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.next_n(2).unwrap(), Some(b"fo".into()));
        assert_eq!(reader.offset, 2);

        assert_eq!(reader.next_n(5).unwrap(), Some(b"o bar".into()));
        assert_eq!(reader.offset, 7);

        assert_eq!(reader.next_n(5).unwrap(), None);
        assert_eq!(reader.offset, 7);

        assert_eq!(reader.next_n(4).unwrap(), Some(b" baz".into()));
        assert_eq!(reader.offset, 11);
    }

    #[test]
    fn slice_reader_next_char() {
        let mut reader = SliceReader {
            bytes: b"f",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.next_char().unwrap(), Some('f'));
        assert_eq!(reader.next_char().unwrap(), None);

        let mut reader = SliceReader {
            bytes: b"\xff",
            offset: 0,
            seq_start: None,
        };

        reader.next_char().unwrap_err();

        let mut reader = SliceReader {
            bytes: b"\xcf\xff",
            offset: 0,
            seq_start: None,
        };

        reader.next_char().unwrap_err();

        let mut reader = SliceReader {
            bytes: b"\xcf",
            offset: 0,
            seq_start: None,
        };

        reader.next_char().unwrap_err();
    }

    #[test]
    fn slice_reader_peek() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.peek().unwrap(), Some(b'f'));
        assert_eq!(reader.offset, 0);

        assert_eq!(reader.peek().unwrap(), Some(b'f'));
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_peek_n() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.peek_n(2).unwrap(), Some(b"fo".into()));
        assert_eq!(reader.offset, 0);

        assert_eq!(reader.peek_n(4).unwrap(), None);
        assert_eq!(reader.offset, 0);

        assert_eq!(reader.peek_n(3).unwrap(), Some(b"foo".into()));
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_peek_char() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.peek_char().unwrap(), Some('f'));
        assert_eq!(reader.offset, 0);

        let mut reader = SliceReader {
            bytes: b"",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.peek_char().unwrap(), None);

        let mut reader = SliceReader {
            bytes: b"\xff",
            offset: 0,
            seq_start: None,
        };

        reader.peek_char().unwrap_err();

        let mut reader = SliceReader {
            bytes: b"\xcf\xff",
            offset: 0,
            seq_start: None,
        };

        reader.peek_char().unwrap_err();

        let mut reader = SliceReader {
            bytes: b"\xcf",
            offset: 0,
            seq_start: None,
        };

        reader.peek_char().unwrap_err();
    }

    #[test]
    fn slice_reader_peek_at() {
        let mut reader = SliceReader {
            bytes: b"bar",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.peek_at(1).unwrap(), Some(b'a'));
        assert_eq!(reader.offset, 0);

        assert_eq!(reader.peek_at(3).unwrap(), None);
        assert_eq!(reader.offset, 0);

        assert_eq!(reader.peek_at(2).unwrap(), Some(b'r'));
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_discard() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        reader.discard().unwrap();
        assert_eq!(reader.offset, 1);

        reader.discard().unwrap();
        assert_eq!(reader.offset, 2);

        reader.discard().unwrap();
        assert_eq!(reader.offset, 3);

        reader.discard().unwrap();
        assert_eq!(reader.offset, 3);

        reader.discard().unwrap();
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_discard_n() {
        let mut reader = SliceReader {
            bytes: b"foo bar baz",
            offset: 0,
            seq_start: None,
        };

        reader.discard_n(2).unwrap();
        assert_eq!(reader.offset, 2);

        reader.discard_n(5).unwrap();
        assert_eq!(reader.offset, 7);

        reader.discard_n(12).unwrap();
        assert_eq!(reader.offset, 11);
    }

    #[test]
    fn slice_reader_next_if() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.next_if(|&ch| ch == b'f').unwrap(), Some(b'f'));
        assert_eq!(reader.offset, 1);

        assert_eq!(reader.next_if(|&ch| ch == b'f').unwrap(), None);
        assert_eq!(reader.offset, 1);

        assert_eq!(reader.next_if(|&ch| ch == b'o').unwrap(), Some(b'o'));
        assert_eq!(reader.offset, 2);

        assert_eq!(reader.next_if(|&ch| ch == b'o').unwrap(), Some(b'o'));
        assert_eq!(reader.offset, 3);

        assert_eq!(reader.next_if(|&ch| ch == b'o').unwrap(), None);
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_next_while() {
        let mut reader = SliceReader {
            bytes: b"bbbbaaaaaaararrrrrrr",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(
            reader.next_while(|&ch| ch == b'b').unwrap(),
            b"bbbb".as_slice()
        );
        assert_eq!(reader.offset, 4);

        assert_eq!(reader.next_while(|&ch| ch == b'b').unwrap(), b"".as_slice());
        assert_eq!(reader.offset, 4);

        assert_eq!(
            reader.next_while(|&ch| ch == b'a').unwrap(),
            b"aaaaaaa".as_slice()
        );
        assert_eq!(reader.offset, 11);

        assert_eq!(reader.next_while(|&ch| ch == b'a').unwrap(), b"".as_slice());
        assert_eq!(reader.offset, 11);

        assert_eq!(
            reader.next_while(|&ch| ch == b'r').unwrap(),
            b"r".as_slice()
        );
        assert_eq!(reader.offset, 12);

        assert_eq!(reader.next_while(|&ch| ch == b'r').unwrap(), b"".as_slice());
        assert_eq!(reader.offset, 12);

        assert_eq!(
            reader.next_while(|&ch| ch == b'a').unwrap(),
            b"a".as_slice()
        );
        assert_eq!(reader.offset, 13);

        assert_eq!(
            reader.next_while(|&ch| ch == b'r').unwrap(),
            b"rrrrrrr".as_slice()
        );
        assert_eq!(reader.offset, 20);

        assert_eq!(reader.next_while(|&ch| ch == b'r').unwrap(), b"".as_slice());
        assert_eq!(reader.offset, 20);
    }

    #[test]
    fn slice_reader_next_str_while() {
        let mut reader = SliceReader {
            bytes: b"bbbbaaaaaaararrrrrrr",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(reader.next_str_while(|&ch| ch == b'b').unwrap(), "bbbb");
        assert_eq!(reader.offset, 4);

        assert_eq!(reader.next_str_while(|&ch| ch == b'b').unwrap(), "");
        assert_eq!(reader.offset, 4);

        assert_eq!(reader.next_str_while(|&ch| ch == b'a').unwrap(), "aaaaaaa");
        assert_eq!(reader.offset, 11);

        assert_eq!(reader.next_str_while(|&ch| ch == b'a').unwrap(), "");
        assert_eq!(reader.offset, 11);

        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "r");
        assert_eq!(reader.offset, 12);

        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "");
        assert_eq!(reader.offset, 12);

        assert_eq!(reader.next_str_while(|&ch| ch == b'a').unwrap(), "a");
        assert_eq!(reader.offset, 13);

        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "rrrrrrr");
        assert_eq!(reader.offset, 20);

        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "");
        assert_eq!(reader.offset, 20);

        let mut reader = SliceReader {
            bytes: b"\xff\xff\xff\xff",
            offset: 0,
            seq_start: None,
        };

        reader.next_str_while(|&ch| ch == b'\xff').unwrap_err();
    }

    #[test]
    fn slice_reader_peek_while() {
        let mut reader = SliceReader {
            bytes: b"foo bar baz",
            offset: 0,
            seq_start: None,
        };

        assert_eq!(
            reader.peek_while(|&ch| ch == b'f').unwrap(),
            b"f".as_slice()
        );
        assert_eq!(reader.offset, 0);

        assert_eq!(reader.peek_while(|&ch| ch == b'o').unwrap(), b"".as_slice());
        assert_eq!(reader.offset, 0);

        assert_eq!(
            reader.peek_while(|&ch| !ch.is_ascii_whitespace()).unwrap(),
            b"foo".as_slice()
        );
        assert_eq!(reader.offset, 0);

        assert_eq!(
            reader.peek_while(|&ch| ch.is_ascii()).unwrap(),
            b"foo bar baz".as_slice()
        );
        assert_eq!(reader.offset, 0);
    }

    #[test]
    fn slice_reader_eat_char() {
        let mut reader = SliceReader {
            bytes: b"foo",
            offset: 0,
            seq_start: None,
        };

        assert!(reader.eat_char(b'f').unwrap());
        assert_eq!(reader.offset, 1);

        assert!(!reader.eat_char(b'f').unwrap());
        assert_eq!(reader.offset, 1);

        assert!(reader.eat_char(b'o').unwrap());
        assert_eq!(reader.offset, 2);

        assert!(reader.eat_char(b'o').unwrap());
        assert_eq!(reader.offset, 3);

        assert!(!reader.eat_char(b'o').unwrap());
        assert_eq!(reader.offset, 3);
    }

    #[test]
    fn slice_reader_eat_str() {
        let mut reader = SliceReader {
            bytes: b"foobar",
            offset: 0,
            seq_start: None,
        };

        assert!(reader.eat_str(b"foo").unwrap());
        assert_eq!(reader.offset, 3);

        assert!(!reader.eat_str(b"foo").unwrap());
        assert_eq!(reader.offset, 3);

        assert!(reader.eat_str(b"bar").unwrap());
        assert_eq!(reader.offset, 6);

        assert!(!reader.eat_str(b"bar").unwrap());
        assert_eq!(reader.offset, 6);

        assert!(!reader.eat_str(b"baz").unwrap());
        assert_eq!(reader.offset, 6);
    }

    #[test]
    fn slice_reader_seq() {
        let mut reader = SliceReader {
            bytes: b"foo bar baz",
            offset: 0,
            seq_start: None,
        };

        let _f = reader.next().unwrap().unwrap();

        reader.start_seq();

        let _oo = reader.next_while(|ch| *ch == b'o').unwrap();
        let _space = reader.next_if(u8::is_ascii_whitespace).unwrap().unwrap();
        let _bar = reader.next_while(|ch| !ch.is_ascii_whitespace()).unwrap();
        reader.discard().unwrap();
        let _ba = reader.next_n(2).unwrap();

        assert_eq!(reader.end_seq().unwrap(), b"oo bar ba".as_slice());
    }

    #[test]
    #[should_panic = "Reader::end_seq called without calling Reader::start_seq first"]
    fn slice_reader_end_seq_without_starting() {
        let mut reader = SliceReader {
            bytes: b"foo bar baz",
            offset: 0,
            seq_start: None,
        };

        let _result = reader.end_seq();
    }

    #[test]
    fn io_reader_from_reader() {
        let s = br"
            [a]
            b = 1
            c = 2
            d = 3
        ";
        let r = IoReader::from_reader(s.as_slice());

        assert_eq!(r.peek.len(), 0);
        assert_eq!(r.seq, None);
    }

    #[test]
    fn io_reader_next() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.next().unwrap(), Some(b'f'));
        assert_eq!(reader.next().unwrap(), Some(b'o'));

        reader.peek().unwrap();
        assert_eq!(reader.next().unwrap(), Some(b'o'));

        assert_eq!(reader.next().unwrap(), None);
        assert_eq!(reader.next().unwrap(), None);
    }

    #[test]
    fn io_reader_next_n() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo bar baz".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.next_n(2).unwrap(), Some(b"fo".into()));
        assert_eq!(reader.next_n(5).unwrap(), Some(b"o bar".into()));
        assert_eq!(reader.next_n(5).unwrap(), None);
        assert_eq!(reader.next_n(4).unwrap(), Some(b" baz".into()));
    }

    #[test]
    fn io_reader_next_char() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"f".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.next_char().unwrap(), Some('f'));
        assert_eq!(reader.next_char().unwrap(), None);

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xff".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.next_char().unwrap_err();

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xcf\xff".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.next_char().unwrap_err();

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xcf".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.next_char().unwrap_err();
    }

    #[test]
    fn io_reader_peek() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.peek.len(), 0);
        assert_eq!(reader.peek().unwrap(), Some(b'f'));
        assert_eq!(reader.peek.len(), 1);
        assert_eq!(reader.peek().unwrap(), Some(b'f'));
        assert_eq!(reader.peek.len(), 1);
    }

    #[test]
    fn io_reader_peek_n() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.peek.len(), 0);
        assert_eq!(reader.peek_n(2).unwrap(), Some(b"fo".into()));
        assert_eq!(reader.peek.len(), 2);
        assert_eq!(reader.peek_n(4).unwrap(), None);
        assert_eq!(reader.peek.len(), 3);
        assert_eq!(reader.peek_n(3).unwrap(), Some(b"foo".into()));
        assert_eq!(reader.peek.len(), 3);
    }

    #[test]
    fn io_reader_peek_char() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.peek_char().unwrap(), Some('f'));

        let mut reader = IoReader {
            iter: io::BufReader::new(b"".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.peek_char().unwrap(), None);

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xff".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.peek_char().unwrap_err();

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xcf\xff".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.peek_char().unwrap_err();

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xcf".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.peek_char().unwrap_err();
    }

    #[test]
    fn io_reader_peek_at() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"bar".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.peek.len(), 0);
        assert_eq!(reader.peek_at(1).unwrap(), Some(b'a'));
        assert_eq!(reader.peek.len(), 2);
        assert_eq!(reader.peek_at(3).unwrap(), None);
        assert_eq!(reader.peek.len(), 3);
        assert_eq!(reader.peek_at(2).unwrap(), Some(b'r'));
        assert_eq!(reader.peek.len(), 3);
    }

    #[test]
    fn io_reader_discard() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.discard().unwrap();
        assert_eq!(reader.peek.len(), 0);
        assert_eq!(reader.peek().unwrap(), Some(b'o'));
        assert_eq!(reader.peek.len(), 1);
        reader.discard().unwrap();
        assert_eq!(reader.peek.len(), 0);
        reader.discard().unwrap();
        assert_eq!(reader.peek.len(), 0);
        assert_eq!(reader.peek().unwrap(), None);
        assert_eq!(reader.peek.len(), 0);
        reader.discard().unwrap();
        assert_eq!(reader.peek.len(), 0);
        reader.discard().unwrap();
    }

    #[test]
    fn io_reader_discard_n() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo bar baz".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.discard_n(2).unwrap();
        assert_eq!(reader.peek.len(), 0);
        assert_eq!(reader.peek_n(7).unwrap(), Some(b"o bar b".into()));
        assert_eq!(reader.peek.len(), 7);
        reader.discard_n(5).unwrap();
        assert_eq!(reader.peek.len(), 2);
        reader.discard_n(12).unwrap();
        assert_eq!(reader.peek.len(), 0);
    }

    #[test]
    fn io_reader_next_if() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.next_if(|&ch| ch == b'f').unwrap(), Some(b'f'));
        assert_eq!(reader.next_if(|&ch| ch == b'f').unwrap(), None);
        assert_eq!(reader.next_if(|&ch| ch == b'o').unwrap(), Some(b'o'));
        assert_eq!(reader.next_if(|&ch| ch == b'o').unwrap(), Some(b'o'));
        assert_eq!(reader.next_if(|&ch| ch == b'o').unwrap(), None);
    }

    #[test]
    fn io_reader_next_while() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"bbbbaaaaaaararrrrrrr".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(
            reader.next_while(|&ch| ch == b'b').unwrap(),
            b"bbbb".as_slice()
        );
        assert_eq!(reader.next_while(|&ch| ch == b'b').unwrap(), b"".as_slice());
        assert_eq!(
            reader.next_while(|&ch| ch == b'a').unwrap(),
            b"aaaaaaa".as_slice()
        );
        assert_eq!(reader.next_while(|&ch| ch == b'a').unwrap(), b"".as_slice());
        assert_eq!(
            reader.next_while(|&ch| ch == b'r').unwrap(),
            b"r".as_slice()
        );
        assert_eq!(reader.next_while(|&ch| ch == b'r').unwrap(), b"".as_slice());
        assert_eq!(
            reader.next_while(|&ch| ch == b'a').unwrap(),
            b"a".as_slice()
        );
        assert_eq!(
            reader.next_while(|&ch| ch == b'r').unwrap(),
            b"rrrrrrr".as_slice()
        );
        assert_eq!(reader.next_while(|&ch| ch == b'r').unwrap(), b"".as_slice());
    }

    #[test]
    fn io_reader_next_str_while() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"bbbbaaaaaaararrrrrrr".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(reader.next_str_while(|&ch| ch == b'b').unwrap(), "bbbb");
        assert_eq!(reader.next_str_while(|&ch| ch == b'b').unwrap(), "");
        assert_eq!(reader.next_str_while(|&ch| ch == b'a').unwrap(), "aaaaaaa");
        assert_eq!(reader.next_str_while(|&ch| ch == b'a').unwrap(), "");
        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "r");
        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "");
        assert_eq!(reader.next_str_while(|&ch| ch == b'a').unwrap(), "a");
        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "rrrrrrr");
        assert_eq!(reader.next_str_while(|&ch| ch == b'r').unwrap(), "");

        let mut reader = IoReader {
            iter: io::BufReader::new(b"\xff\xff\xff\xff".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.next_str_while(|&ch| ch == b'\xff').unwrap_err();
    }

    #[test]
    fn io_reader_peek_while() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo bar baz".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert_eq!(
            reader.peek_while(|&ch| ch == b'f').unwrap(),
            b"f".as_slice()
        );
        assert_eq!(reader.peek_while(|&ch| ch == b'o').unwrap(), b"".as_slice());
        assert_eq!(
            reader.peek_while(|&ch| !ch.is_ascii_whitespace()).unwrap(),
            b"foo".as_slice()
        );
        assert_eq!(
            reader.peek_while(|&ch| ch.is_ascii()).unwrap(),
            b"foo bar baz".as_slice()
        );
    }

    #[test]
    fn io_reader_eat_char() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert!(reader.eat_char(b'f').unwrap());
        assert!(!reader.eat_char(b'f').unwrap());
        assert!(reader.eat_char(b'o').unwrap());
        assert!(reader.eat_char(b'o').unwrap());
        assert!(!reader.eat_char(b'o').unwrap());
    }

    #[test]
    fn io_reader_eat_str() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foobar".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        assert!(reader.eat_str(b"foo").unwrap());
        assert!(!reader.eat_str(b"foo").unwrap());
        reader.peek_n(3).unwrap();
        assert_eq!(reader.peek.len(), 3);
        assert!(reader.eat_str(b"bar").unwrap());
        assert!(!reader.eat_str(b"bar").unwrap());
        assert!(!reader.eat_str(b"baz").unwrap());
    }

    #[test]
    fn io_reader_seq() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo bar baz".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        let _f = reader.next().unwrap().unwrap();

        reader.start_seq();

        let _oo = reader.next_while(|ch| *ch == b'o').unwrap();
        let _space = reader.next_if(u8::is_ascii_whitespace).unwrap().unwrap();
        let _bar = reader.next_while(|ch| !ch.is_ascii_whitespace()).unwrap();
        reader.discard().unwrap();
        let _ba = reader.next_n(2).unwrap();

        assert_eq!(reader.end_seq().unwrap(), b"oo bar ba".as_slice());

        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo bar baz".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        let _f = reader.next().unwrap().unwrap();

        reader.start_seq();

        let _o = reader.next().unwrap();
        reader.discard_n(3).unwrap();
        let _ar = reader.eat_str(b"ar").unwrap();

        assert_eq!(reader.end_seq().unwrap(), b"oo bar".as_slice());
    }

    #[test]
    #[should_panic = "Reader::end_seq called without calling Reader::start_seq first"]
    fn io_reader_end_seq_without_starting() {
        let mut reader = IoReader {
            iter: io::BufReader::new(b"foo bar baz".as_slice()).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        let _result = reader.end_seq();
    }

    #[test]
    fn io_reader_error() {
        struct ErrReader;

        impl io::Read for ErrReader {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("foo"))
            }
        }

        let mut reader = IoReader {
            iter: io::BufReader::new(ErrReader).bytes(),
            peek: VecDeque::new(),
            seq: None,
        };

        reader.next().unwrap_err();
        reader.next_n(4).unwrap_err();
        reader.peek().unwrap_err();
        reader.peek_n(4).unwrap_err();
        reader.peek_at(1).unwrap_err();
        reader.discard().unwrap_err();
        reader.discard_n(4).unwrap_err();
        reader.next_if(|_| true).unwrap_err();
        reader.next_while(|_| true).unwrap_err();
        reader.next_str_while(|_| true).unwrap_err();
        reader.peek_while(|_| true).unwrap_err();
        reader.eat_char(b'a').unwrap_err();
        reader.eat_str(b"foo").unwrap_err();
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
