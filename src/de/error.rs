//! Deserialization error types

use std::{borrow::Borrow, fmt};

use serde::{de, ser};

/// Type alias for [`std::result::Result`] using [`Error`] for its error type
pub type Result<T> = std::result::Result<T, Error>;

/// A TOML Deserialization error
pub struct Error(Box<ErrorImpl>);

impl Error {
    /// The position of the error in the file
    #[must_use]
    pub fn position(&self) -> usize {
        self.0.position
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("type", &self.0.kind)
            .field("position", &self.0.position)
            .finish()
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::custom(msg)
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::custom(msg)
    }
}

// Convenience impl to box the error
impl From<ErrorImpl> for Error {
    fn from(value: ErrorImpl) -> Self {
        Self(Box::new(value))
    }
}

impl Error {
    pub(crate) fn invalid_encoding(_err: std::str::Utf8Error, position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::InvalidEncoding,
            position,
        }
        .into()
    }
    pub(crate) fn eof(position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::Eof,
            position,
        }
        .into()
    }

    pub(crate) fn illegal_char(char: char, position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::IllegalChar(char),
            position,
        }
        .into()
    }

    pub(crate) fn unterminated_string(position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::UnterminatedString,
            position,
        }
        .into()
    }

    pub(crate) fn invalid_escape(seq: impl Into<Box<str>>, position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::InvalidEscape(seq.into()),
            position,
        }
        .into()
    }

    pub(crate) fn invalid_number(err: impl Into<Box<str>>, position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::InvalidNumber(err.into()),
            position,
        }
        .into()
    }

    pub(crate) fn invalid_datetime(position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::InvalidDatetime,
            position,
        }
        .into()
    }

    pub(crate) fn expected(token: impl Into<Box<str>>, position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::Expected(token.into()),
            position,
        }
        .into()
    }

    pub(crate) fn duplicate_key(
        key: &[impl Borrow<str>],
        table: &[impl Borrow<str>],
        position: usize,
    ) -> Self {
        ErrorImpl {
            kind: ErrorKind::DuplicateKey(
                key.join(".").into(),
                Some(table.join(".").into_boxed_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "root table".into()),
            ),
            position,
        }
        .into()
    }

    pub(crate) fn invalid_table_header(key: &[impl Borrow<str>], position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::InvalidTableHeader(key.join(".").into()),
            position,
        }
        .into()
    }

    pub(crate) fn invalid_key_path(
        key: &[impl Borrow<str>],
        table: &[impl Borrow<str>],
        position: usize,
    ) -> Self {
        ErrorImpl {
            kind: ErrorKind::InvalidKeyPath(
                key.join(".").into(),
                Some(table.join(".").into_boxed_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "root table".into()),
            ),
            position,
        }
        .into()
    }

    pub(crate) fn unexpected_char(ch: char, position: usize) -> Self {
        ErrorImpl {
            kind: ErrorKind::UnexpectedChar(ch),
            position,
        }
        .into()
    }

    // pub(crate) fn from_io(err: std::io::Error, position: usize) -> Self {
    //     ErrorImpl {
    //         kind: ErrorKind::Io(err),
    //         position,
    //     }
    //     .into()
    // }

    pub(crate) fn custom(msg: impl fmt::Display) -> Self {
        ErrorImpl {
            kind: ErrorKind::Custom(msg.to_string().into_boxed_str()),
            position: 0,
        }
        .into()
    }
}

#[derive(Debug)]
struct ErrorImpl {
    pub kind: ErrorKind,
    pub position: usize,
}

impl fmt::Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO get line/col from position, or ignore for error where position makes no sense
        write!(f, "{} (at position {})", self.kind, self.position)
    }
}

#[derive(Debug)]
enum ErrorKind {
    /// File is not UTF-8 encoded
    InvalidEncoding,
    /// End of file
    Eof,
    /// Illegal character (in a string)
    IllegalChar(char),
    /// Unterminated string
    UnterminatedString,
    /// Invalid escape sequence
    InvalidEscape(Box<str>),
    /// Invalid number
    InvalidNumber(Box<str>),
    /// Invalid datetime
    InvalidDatetime,
    /// Unexpected token
    Expected(Box<str>),

    /// Duplicate key
    DuplicateKey(Box<str>, Box<str>),
    /// Invalid table header
    InvalidTableHeader(Box<str>),
    /// Invalid key path
    InvalidKeyPath(Box<str>, Box<str>),

    /// Unexpected character
    UnexpectedChar(char),

    // /// IO Error
    // Io(std::io::Error),
    /// Custom error message
    Custom(Box<str>),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidEncoding => f.write_str("TOML file is not valid UTF-8"),
            Self::Eof => f.write_str("unexpected end of file"),
            Self::IllegalChar(char) => write!(f, "illegal character: {char:?}"),
            Self::UnterminatedString => f.write_str("unterminated string"),
            Self::InvalidEscape(ref seq) => write!(f, "invalid escape sequence: {seq}"),
            Self::InvalidNumber(ref error) => write!(f, "invalid integer: {error}"),
            Self::InvalidDatetime => f.write_str("invalid datetime"),
            Self::Expected(ref token) => write!(f, "expected a {token}"),
            Self::DuplicateKey(ref key, ref table) => write!(f, "duplicate key: {key} in {table}"),
            Self::InvalidTableHeader(ref key) => write!(f, "invalid table header: {key}"),
            Self::InvalidKeyPath(ref key, ref table) => write!(f, "invalid key: {key} in {table}"),
            Self::UnexpectedChar(ch) => write!(f, "unexpected character: {ch}"),
            // Self::Io(ref io_error) => write!(f, "IO error: {io_error}"),
            Self::Custom(ref msg) => f.write_str(msg),
        }
    }
}
