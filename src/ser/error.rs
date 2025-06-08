//! Deserialization error types

use std::sync::Arc;
use std::{fmt, io};

use serde::ser;

/// Type alias for [`std::result::Result`] using [`Error`] for its error type
pub type Result<T> = std::result::Result<T, Error>;

/// A TOML Deserialization error
#[derive(Clone)]
pub struct Error(pub(crate) ErrorKind);

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("type", &self.0)
            .finish_non_exhaustive()
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.0 {
            ErrorKind::Io(ref io_error) => Some(&**io_error),
            ErrorKind::Fmt(ref fmt_error) => Some(fmt_error),
            _ => None,
        }
    }
}

impl ser::Error for Error {
    #[inline]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ErrorKind::Custom(msg.to_string().into_boxed_str()).into()
    }
}

// Convenience impl to create the error
impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self(kind)
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(value: io::Error) -> Self {
        Self(ErrorKind::Io(Arc::new(value)))
    }
}

impl From<fmt::Error> for Error {
    #[inline]
    fn from(value: fmt::Error) -> Self {
        Self(ErrorKind::Fmt(value))
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Serialization
    /// Unsupported Rust value
    UnsupportedValue(&'static str),
    /// Unsupported Rust type
    UnsupportedType(&'static str),
    /// Duplicate key in table
    DuplicateKey(Box<str>),

    // Misc
    /// IO Error
    Io(Arc<io::Error>), // Need to use Arc since io::Error is not cloneable
    /// Formatting error
    Fmt(fmt::Error),
    /// Custom error message
    Custom(Box<str>),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(clippy::enum_glob_use)] // Just for match
        use ErrorKind::*;

        match *self {
            UnsupportedValue(msg) => write!(f, "unsupported value: {msg}"),
            UnsupportedType(msg) => write!(f, "unsupported type: {msg}"),
            DuplicateKey(ref key) => write!(f, r#"duplicate key "{key}" in table"#),
            Io(ref io_error) => write!(f, "IO error: {io_error}"),
            Fmt(ref fmt_error) => write!(f, "formatting error: {fmt_error}"),
            Custom(ref msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use std::error::Error as _;

    use assert_matches::assert_matches;
    use serde::ser::Error as _;

    use super::*;

    #[test]
    fn error_display() {
        let error = Error(ErrorKind::UnsupportedValue("foo"));
        assert_eq!(error.to_string(), "unsupported value: foo");
    }

    #[test]
    fn error_debug() {
        let error = Error(ErrorKind::UnsupportedValue("foo"));
        assert_eq!(
            format!("{error:?}"),
            r#"Error { type: UnsupportedValue("foo"), .. }"#
        );
    }

    #[test]
    fn error_source() {
        let error = Error(ErrorKind::UnsupportedValue("foo"));
        assert!(error.source().is_none());

        let error = Error(ErrorKind::Io(Arc::new(io::Error::new(
            io::ErrorKind::NotFound,
            "foo",
        ))));
        let source = error.source().unwrap();
        let source = source.downcast_ref::<io::Error>().unwrap();
        assert_eq!(source.kind(), io::ErrorKind::NotFound);
        assert_eq!(source.to_string(), "foo");

        let error = Error(ErrorKind::Fmt(fmt::Error));
        let source = error.source().unwrap();
        assert!(source.downcast_ref::<fmt::Error>().is_some());
        assert_eq!(
            source.to_string(),
            "an error occurred when formatting an argument"
        );
    }

    #[test]
    fn error_custom() {
        let error = Error::custom("foo");
        assert_matches!(error.0, ErrorKind::Custom(msg) if &*msg == "foo");
    }

    #[test]
    fn error_from_error_kind() {
        let kind = ErrorKind::UnsupportedValue("foo");
        let err = Error::from(kind);
        assert_matches!(err.0, ErrorKind::UnsupportedValue(..));
    }

    #[test]
    fn error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "foo");
        let err = Error::from(io_err);
        assert_matches!(err.0, ErrorKind::Io(..));
    }

    #[test]
    fn error_from_fmt_error() {
        let fmt_err = fmt::Error;
        let err = Error::from(fmt_err);
        assert_matches!(err.0, ErrorKind::Fmt(..));
    }

    #[test]
    fn error_kind_display() {
        let kind = ErrorKind::UnsupportedValue("foo");
        assert_eq!(kind.to_string(), "unsupported value: foo");

        let kind = ErrorKind::UnsupportedType("foo");
        assert_eq!(kind.to_string(), "unsupported type: foo");

        let kind = ErrorKind::DuplicateKey("foo".into());
        assert_eq!(format!("{kind}"), r#"duplicate key "foo" in table"#);

        let kind = ErrorKind::Io(Arc::new(io::Error::new(io::ErrorKind::NotFound, "foo")));
        assert_eq!(kind.to_string(), "IO error: foo");

        let kind = ErrorKind::Fmt(fmt::Error);
        assert_eq!(
            kind.to_string(),
            "formatting error: an error occurred when formatting an argument"
        );

        let kind = ErrorKind::Custom("foo".into());
        assert_eq!(kind.to_string(), "foo");
    }
}
