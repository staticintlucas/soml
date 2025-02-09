//! Deserialization error types

use std::sync::Arc;
use std::{fmt, io};

use serde::ser;

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

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        ErrorImpl {
            kind: ErrorKind::Io(Arc::new(value)),
        }
        .into()
    }
}

impl From<fmt::Error> for Error {
    fn from(value: fmt::Error) -> Self {
        ErrorImpl {
            kind: ErrorKind::Fmt(value),
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
    // Serialization
    /// Unsupported Rust value
    UnsupportedValue(&'static str),
    /// Unsupported Rust type
    UnsupportedType(&'static str),

    // Misc
    /// IO Error
    Io(Arc<io::Error>), // Need to use Arc since io::Error is not cloneable
    /// Formatting error
    Fmt(fmt::Error),
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
            UnsupportedValue(msg) => write!(f, "unsupported value: {msg}"),
            UnsupportedType(msg) => write!(f, "unsupported type: {msg}"),
            Io(ref io_error) => write!(f, "IO error: {io_error}"),
            Fmt(ref fmt_error) => write!(f, "formatting error: {fmt_error}"),
            Custom(ref msg) => write!(f, "{msg}"),
        }
    }
}
