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
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("type", &self.0.kind)
            .finish_non_exhaustive()
    }
}

impl std::error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.0.kind {
            ErrorKind::Io(ref io_error) => Some(&**io_error),
            _ => None,
        }
    }
}

impl de::Error for Error {
    #[inline]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ErrorKind::Custom(msg.to_string().into_boxed_str()).into()
    }

    #[inline]
    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        ErrorKind::InvalidType(
            unexp.to_string().into_boxed_str(),
            exp.to_string().into_boxed_str(),
        )
        .into()
    }

    #[inline]
    fn invalid_value(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        ErrorKind::InvalidValue(
            unexp.to_string().into_boxed_str(),
            exp.to_string().into_boxed_str(),
        )
        .into()
    }

    #[inline]
    fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
        ErrorKind::InvalidLength(len, exp.to_string().into_boxed_str()).into()
    }

    #[inline]
    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self {
        let expected = match *expected {
            [] => "no variant".into(),
            [variant] => variant.into(),
            [first, last] => format!("{first} or {last}").into(),
            [ref rest @ .., last] => format!("{rest}, or {last}", rest = rest.join(", ")).into(),
        };

        ErrorKind::UnknownVariant(variant.into(), expected).into()
    }

    #[inline]
    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        let expected = match *expected {
            [] => "no field".into(),
            [variant] => variant.into(),
            [first, last] => format!("{first} or {last}").into(),
            [ref rest @ .., last] => format!("{rest}, or {last}", rest = rest.join(", ")).into(),
        };

        ErrorKind::UnknownField(field.into(), expected).into()
    }

    #[inline]
    fn missing_field(field: &'static str) -> Self {
        ErrorKind::MissingField(field).into()
    }

    #[inline]
    fn duplicate_field(field: &'static str) -> Self {
        ErrorKind::DuplicateField(field).into()
    }
}

// Convenience impl to box the error
impl From<ErrorImpl> for Error {
    #[inline]
    fn from(value: ErrorImpl) -> Self {
        Self(Box::new(value))
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        ErrorImpl { kind }.into()
    }
}

impl From<io::Error> for Error {
    #[inline]
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
    #[inline]
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

impl fmt::Display for ErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(clippy::enum_glob_use)] // Just for match
        use ErrorKind::*;

        match *self {
            InvalidEncoding => write!(f, "file contains invalid UTF-8 bytes"),
            UnexpectedEof => write!(f, "unexpected end of file"),
            IllegalChar(ch) => write!(f, "illegal character: {:?}", char::from(ch)),
            UnterminatedString => write!(f, "unterminated string"),
            InvalidEscape(ref seq) => write!(f, "invalid escape sequence: {seq}"),
            InvalidNumber(ref error) => write!(f, "invalid number: {error}"),
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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use std::error::Error as _;

    use assert_matches::assert_matches;
    use serde::de::Error as _;

    use super::*;

    #[test]
    fn error_display() {
        let error = Error(Box::new(ErrorImpl {
            kind: ErrorKind::InvalidType("foo".into(), "bar".into()),
        }));
        assert_eq!(format!("{error}"), "invalid type: foo, expected bar");
    }

    #[test]
    fn error_debug() {
        let error = Error(Box::new(ErrorImpl {
            kind: ErrorKind::InvalidType("foo".into(), "bar".into()),
        }));
        assert_eq!(
            format!("{error:?}"),
            r#"Error { type: InvalidType("foo", "bar"), .. }"#
        );
    }

    #[test]
    fn error_source() {
        let error = Error(Box::new(ErrorImpl {
            kind: ErrorKind::InvalidType("foo".into(), "bar".into()),
        }));
        assert!(error.source().is_none());

        let error = Error(Box::new(ErrorImpl {
            kind: ErrorKind::Io(Arc::new(io::Error::new(io::ErrorKind::NotFound, "foo"))),
        }));
        let source = error.source().unwrap();
        let source = source.downcast_ref::<io::Error>().unwrap();
        assert_eq!(source.kind(), io::ErrorKind::NotFound);
        assert_eq!(format!("{source}"), "foo");
    }

    #[test]
    fn error_custom() {
        let error = Error::custom("foo");
        assert_matches!(error.0.kind, ErrorKind::Custom(msg) if &*msg == "foo");
    }

    #[test]
    fn error_invalid_type() {
        let error = Error::invalid_type(de::Unexpected::Str("foo"), &"bar");
        assert_matches!(error.0.kind, ErrorKind::InvalidType(unexp, exp) if &*unexp == r#"string "foo""# && &*exp == "bar");
    }

    #[test]
    fn error_invalid_value() {
        let error = Error::invalid_value(de::Unexpected::Str("foo"), &"bar");
        assert_matches!(error.0.kind, ErrorKind::InvalidValue(unexp, exp) if &*unexp == r#"string "foo""# && &*exp == "bar");
    }

    #[test]
    fn error_invalid_length() {
        let error = Error::invalid_length(1, &"bar");
        assert_matches!(error.0.kind, ErrorKind::InvalidLength(1, exp) if &*exp == "bar");
    }

    #[test]
    fn error_unknown_variant() {
        let error = Error::unknown_variant("foo", &[]);
        assert_matches!(error.0.kind, ErrorKind::UnknownVariant(var, exp) if &*var == "foo" && &*exp == "no variant");

        let error = Error::unknown_variant("foo", &["bar"]);
        assert_matches!(error.0.kind, ErrorKind::UnknownVariant(var, exp) if &*var == "foo" && &*exp == "bar");

        let error = Error::unknown_variant("foo", &["bar", "baz"]);
        assert_matches!(error.0.kind, ErrorKind::UnknownVariant(var, exp) if &*var == "foo" && &*exp == "bar or baz");

        let error = Error::unknown_variant("foo", &["bar", "baz", "qux"]);
        assert_matches!(error.0.kind, ErrorKind::UnknownVariant(var, exp) if &*var == "foo" && &*exp == "bar, baz, or qux");
    }

    #[test]
    fn error_unknown_field() {
        let error = Error::unknown_field("foo", &[]);
        assert_matches!(error.0.kind, ErrorKind::UnknownField(fld, exp) if &*fld == "foo" && &*exp == "no field");

        let error = Error::unknown_field("foo", &["bar"]);
        assert_matches!(error.0.kind, ErrorKind::UnknownField(fld, exp) if &*fld == "foo" && &*exp == "bar");

        let error = Error::unknown_field("foo", &["bar", "baz"]);
        assert_matches!(error.0.kind, ErrorKind::UnknownField(fld, exp) if &*fld == "foo" && &*exp == "bar or baz");

        let error = Error::unknown_field("foo", &["bar", "baz", "qux"]);
        assert_matches!(error.0.kind, ErrorKind::UnknownField(fld, exp) if &*fld == "foo" && &*exp == "bar, baz, or qux");
    }

    #[test]
    fn error_missing_field() {
        let error = Error::missing_field("foo");
        assert_matches!(error.0.kind, ErrorKind::MissingField(fld) if fld == "foo");
    }

    #[test]
    fn error_duplicate_field() {
        let error = Error::duplicate_field("foo");
        assert_matches!(error.0.kind, ErrorKind::DuplicateField(fld) if fld == "foo");
    }

    #[test]
    fn error_from_error_impl() {
        let err_impl = ErrorImpl {
            kind: ErrorKind::InvalidType("foo".into(), "bar".into()),
        };
        let err = Error::from(err_impl);
        assert_matches!(err.0.kind, ErrorKind::InvalidType(..));
    }

    #[test]
    fn error_from_error_kind() {
        let kind = ErrorKind::InvalidType("foo".into(), "bar".into());
        let err = Error::from(kind);
        assert_matches!(err.0.kind, ErrorKind::InvalidType(..));
    }

    #[test]
    fn error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "foo");
        let err = Error::from(io_err);
        assert_matches!(err.0.kind, ErrorKind::Io(..));
    }

    #[test]
    fn error_impl_display() {
        let err_impl = ErrorImpl {
            kind: ErrorKind::InvalidType("foo".into(), "bar".into()),
        };
        assert_eq!(format!("{err_impl}"), "invalid type: foo, expected bar");
    }

    #[test]
    fn error_kind_display() {
        let kind = ErrorKind::InvalidEncoding;
        assert_eq!(format!("{kind}"), "file contains invalid UTF-8 bytes");

        let kind = ErrorKind::UnexpectedEof;
        assert_eq!(format!("{kind}"), "unexpected end of file");

        let kind = ErrorKind::IllegalChar(b'a');
        assert_eq!(format!("{kind}"), "illegal character: 'a'");

        let kind = ErrorKind::UnterminatedString;
        assert_eq!(format!("{kind}"), "unterminated string");

        let kind = ErrorKind::InvalidEscape("foo".into());
        assert_eq!(format!("{kind}"), "invalid escape sequence: foo");

        let kind = ErrorKind::InvalidNumber("foo".into());
        assert_eq!(format!("{kind}"), "invalid number: foo");

        let kind = ErrorKind::InvalidDatetime;
        assert_eq!(format!("{kind}"), "invalid datetime");

        let kind = ErrorKind::ExpectedToken("foo".into());
        assert_eq!(format!("{kind}"), "expected foo");

        let kind = ErrorKind::DuplicateKey("foo".into(), "bar".into());
        assert_eq!(format!("{kind}"), "duplicate key: foo in bar");

        let kind = ErrorKind::InvalidTableHeader("foo".into());
        assert_eq!(format!("{kind}"), "invalid table header: foo");

        let kind = ErrorKind::InvalidKeyPath("foo".into(), "bar".into());
        assert_eq!(format!("{kind}"), "invalid key: foo in bar");

        let kind = ErrorKind::InvalidType("foo".into(), "bar".into());
        assert_eq!(format!("{kind}"), "invalid type: foo, expected bar");

        let kind = ErrorKind::InvalidValue("foo".into(), "bar".into());
        assert_eq!(format!("{kind}"), "invalid value: foo, expected bar");

        let kind = ErrorKind::InvalidLength(1, "bar".into());
        assert_eq!(format!("{kind}"), "invalid length: 1, expected bar");

        let kind = ErrorKind::UnknownVariant("foo".into(), "bar".into());
        assert_eq!(format!("{kind}"), "unknown variant: foo, expected bar");

        let kind = ErrorKind::UnknownField("foo".into(), "bar".into());
        assert_eq!(format!("{kind}"), "unknown field: foo, expected bar");

        let kind = ErrorKind::MissingField("foo");
        assert_eq!(format!("{kind}"), "missing field: foo");

        let kind = ErrorKind::DuplicateField("foo");
        assert_eq!(format!("{kind}"), "duplicate field: foo");

        let kind = ErrorKind::Io(Arc::new(io::Error::new(io::ErrorKind::NotFound, "foo")));
        assert_eq!(format!("{kind}"), "IO error: foo");

        let kind = ErrorKind::Custom("foo".into());
        assert_eq!(format!("{kind}"), "foo");
    }
}
