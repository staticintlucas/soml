use std::borrow::Cow;
use std::collections::VecDeque;
use std::{io, str};

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

    fn end_seq(&mut self) -> Result<Cow<'a, [u8]>> {
        let start = self
            .seq_start
            .take()
            .unwrap_or_else(|| unreachable!("Sequence wasn't started first"));
        Ok(Cow::Borrowed(&self.bytes[start..self.offset]))
    }
}

/// Read from a string
#[derive(Debug)]
pub struct IoReader<R> {
    iter: io::Bytes<R>,
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
            iter: read.bytes(),
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
        let mut result = Vec::with_capacity(n);
        result.extend(self.peek.drain(..n.min(self.peek.len())));

        while result.len() < n {
            match self.iter.next().transpose()? {
                Some(ch) => result.push(ch),
                None => return Ok(None), // return None if we can't get n bytes
            }
        }

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

            let peeked_n = n.min(self.peek.len());
            seq.extend(self.peek.drain(..peeked_n));

            for _ in peeked_n..n {
                match self.iter.next().transpose()? {
                    Some(ch) => seq.push(ch),
                    None => break,
                }
            }
        } else {
            let peeked_n = n.min(self.peek.len());
            self.peek.drain(..peeked_n);

            for _ in peeked_n..n {
                if self.iter.next().transpose()?.is_none() {
                    break;
                }
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

    fn end_seq(&mut self) -> Result<Cow<'a, [u8]>> {
        Ok(Cow::Owned(self.seq.take().unwrap_or_else(|| {
            unreachable!("Sequence wasn't started first")
        })))
    }
}
