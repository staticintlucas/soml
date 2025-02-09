//! Deserialization error types

use std::sync::Arc;
use std::{fmt, io};

use serde::de;

/// Type alias for [`std::result::Result`] using [`Error`] for its error type
pub type Result<T> = std::result::Result<T, Error>;

/// A TOML Deserialization error
#[derive(Clone)]
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

    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        ErrorKind::InvalidType(
            unexp.to_string().into_boxed_str(),
            exp.to_string().into_boxed_str(),
        )
        .into()
    }

    fn invalid_value(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        ErrorKind::InvalidValue(
            unexp.to_string().into_boxed_str(),
            exp.to_string().into_boxed_str(),
        )
        .into()
    }

    fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
        ErrorKind::InvalidLength(len, exp.to_string().into_boxed_str()).into()
    }

    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self {
        let expected = match *expected {
            [] => "no variant".into(),
            [variant] => variant.into(),
            [first, last] => format!("{first} or {last}").into(),
            [ref rest @ .., last] => format!("{rest} or, {last}", rest = rest.join(", ")).into(),
        };

        ErrorKind::UnknownVariant(variant.into(), expected).into()
    }

    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        let expected = match *expected {
            [] => "no variant".into(),
            [variant] => variant.into(),
            [first, last] => format!("{first} or {last}").into(),
            [ref rest @ .., last] => format!("{rest} or, {last}", rest = rest.join(", ")).into(),
        };

        ErrorKind::UnknownField(field.into(), expected).into()
    }

    fn missing_field(field: &'static str) -> Self {
        ErrorKind::MissingField(field).into()
    }

    fn duplicate_field(field: &'static str) -> Self {
        ErrorKind::DuplicateField(field).into()
    }
}

// Convenience impl to box the error
impl From<ErrorImpl> for Error {
    fn from(value: ErrorImpl) -> Self {
        Self(Box::new(value))
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        ErrorImpl {
            kind: ErrorKind::Io(Arc::new(value)),
        }
        .into()
    }
}

#[derive(Debug, Clone)]
struct ErrorImpl {
    pub kind: ErrorKind,
}

impl fmt::Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Parser errors
    /// File is not UTF-8 encoded
    InvalidEncoding,
    /// End of file
    UnexpectedEof,
    /// Illegal control character
    IllegalChar(u8),
    /// Unterminated string
    UnterminatedString,
    /// Invalid escape sequence
    InvalidEscape(Box<str>),
    /// Invalid number
    InvalidNumber(Box<str>),
    /// Invalid datetime
    InvalidDatetime,
    /// Unexpected token
    ExpectedToken(Box<str>),
    /// Duplicate key
    DuplicateKey(Box<str>, Box<str>),
    /// Invalid table header
    InvalidTableHeader(Box<str>),
    /// Invalid key path
    InvalidKeyPath(Box<str>, Box<str>),

    // Serde errors
    /// Invalid type (unexpected, expected)
    InvalidType(Box<str>, Box<str>),
    /// Invalid value (unexpected, expected)
    InvalidValue(Box<str>, Box<str>),
    /// Invalid length (length, expected)
    InvalidLength(usize, Box<str>),
    /// Unknown variant (variant, expected)
    UnknownVariant(Box<str>, Box<str>),
    /// Unknown field (field, expected)
    UnknownField(Box<str>, Box<str>),
    /// Missing field (field)
    MissingField(&'static str),
    /// Duplicate field (field)
    DuplicateField(&'static str),

    // Misc
    /// IO Error
    Io(Arc<io::Error>), // Need to use Arc since io::Error is not cloneable
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
        #[allow(clippy::enum_glob_use)] // Just for match
        use ErrorKind::*;

        match *self {
            InvalidEncoding => write!(f, "file contains invalid UTF-8 bytes"),
            UnexpectedEof => write!(f, "unexpected end of file"),
            IllegalChar(ch) => write!(f, "illegal character: {:?}", char::from(ch)),
            UnterminatedString => write!(f, "unterminated string"),
            InvalidEscape(ref seq) => write!(f, "invalid escape sequence: {seq}"),
            InvalidNumber(ref error) => write!(f, "invalid integer: {error}"),
            InvalidDatetime => write!(f, "invalid datetime"),
            ExpectedToken(ref token) => write!(f, "expected {token}"),
            DuplicateKey(ref key, ref table) => write!(f, "duplicate key: {key} in {table}"),
            InvalidTableHeader(ref key) => write!(f, "invalid table header: {key}"),
            InvalidKeyPath(ref key, ref table) => write!(f, "invalid key: {key} in {table}"),
            InvalidType(ref unexp, ref exp) => write!(f, "invalid type: {unexp}, expected {exp}"),
            InvalidValue(ref unexp, ref exp) => write!(f, "invalid value: {unexp}, expected {exp}"),
            InvalidLength(len, ref exp) => write!(f, "invalid length: {len}, expected {exp}"),
            UnknownVariant(ref var, ref exp) => write!(f, "unknown variant: {var}, expected {exp}"),
            UnknownField(ref fld, ref exp) => write!(f, "unknown field: {fld}, expected {exp}"),
            MissingField(fld) => write!(f, "missing field: {fld}"),
            DuplicateField(fld) => write!(f, "duplicate field: {fld}"),
            Io(ref io_error) => write!(f, "IO error: {io_error}"),
            Custom(ref msg) => write!(f, "{msg}"),
        }
    }
}
