use std::borrow::Cow;
use std::str;

use super::error::{Error, Result};

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
    fn next(&mut self) -> Result<Option<u8>> {
        Ok(self.next_array::<1>()?.map(|bytes| bytes[0]))
    }

    /// Gets `N` bytes from the source. If the end of the source is reached, returns `Ok(None)`.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    fn next_array<const N: usize>(&mut self) -> Result<Option<Cow<'a, [u8; N]>>> {
        Ok(if let result @ Some(_) = self.peek_array::<N>()? {
            self.discard_array::<N>()?;
            result
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
    fn peek(&mut self) -> Result<Option<u8>> {
        Ok(self.peek_array::<1>()?.map(|bytes| bytes[0]))
    }

    /// Peeks the next `N` bytes from the source. Returns `Ok(None)` if the end of the source is reached.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn peek_array<const N: usize>(&mut self) -> Result<Option<Cow<'a, [u8; N]>>>;

    /// Peeks the byte at position `pos` from the current position of the source. If the end of the
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
        self.discard_array::<1>()
    }

    /// Discards `N` bytes from the source.
    ///
    /// # Errors
    ///
    /// Propagates any IO errors that occurred while reading from the source.
    #[doc(hidden)]
    fn discard_array<const N: usize>(&mut self) -> Result<()> {
        self.discard_n(N)
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
        Ok(if let result @ Some(_) = self.peek()?.filter(func) {
            self.discard()?;
            result
        } else {
            None
        })
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
        let position = self.position();
        match self.next_while(func)? {
            Cow::Borrowed(bytes) => str::from_utf8(bytes)
                .map(Cow::Borrowed)
                .map_err(|err| Error::invalid_encoding(err, position + err.valid_up_to())),
            Cow::Owned(vec) => String::from_utf8(vec).map(Cow::Owned).map_err(|err| {
                Error::invalid_encoding(err.utf8_error(), position + err.utf8_error().valid_up_to())
            }),
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

    /// Returns the current position in the source.
    #[doc(hidden)]
    fn position(&self) -> usize;
}

/// Read from a string
#[derive(Debug, Clone)]
pub struct StrReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    seq_start: Option<usize>,
}

impl<'a> StrReader<'a> {
    /// Create a JSON input source to read from a string.
    pub const fn from_str(str: &'a str) -> Self {
        Self::from_slice(str.as_bytes())
    }

    /// Create a JSON input source to read from a slice of bytes.
    pub const fn from_slice(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            seq_start: None,
        }
    }
}

impl private::Sealed for StrReader<'_> {}

impl<'a> Reader<'a> for StrReader<'a> {
    fn peek(&mut self) -> Result<Option<u8>> {
        Ok(self.bytes.get(self.offset).copied())
    }

    fn peek_array<const N: usize>(&mut self) -> Result<Option<Cow<'a, [u8; N]>>> {
        Ok(self.bytes.get(self.offset..self.offset + N).map(|bytes| {
            Cow::Borrowed(
                bytes
                    .try_into()
                    .unwrap_or_else(|_| unreachable!("length is always N")),
            )
        }))
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

    fn position(&self) -> usize {
        self.offset
    }
}
