//! Deserialization error types

use std::fmt;

use serde::{de, ser};

/// Type alias for [`std::result::Result`] using [`Error`] for its error type
pub type Result<T> = std::result::Result<T, Error>;

/// A TOML Deserialization error
pub struct Error(Box<ErrorImpl>);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("type", &self.0.kind)
            .finish_non_exhaustive()
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None // TODO
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ErrorKind::Custom(msg.to_string().into_boxed_str()).into()
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ErrorKind::Custom(msg.to_string().into_boxed_str()).into()
    }
}

// Convenience impl to box the error
impl From<ErrorImpl> for Error {
    fn from(value: ErrorImpl) -> Self {
        Self(Box::new(value))
    }
}

#[derive(Debug)]
struct ErrorImpl {
    pub kind: ErrorKind,
}

impl fmt::Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

#[derive(Debug)]
pub enum ErrorKind {
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

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        ErrorImpl { kind }.into()
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidEncoding => f.write_str("file is not valid UTF-8"),
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
