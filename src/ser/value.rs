use std::fmt;

use serde::ser;

use crate::ser::writer::Formatter;
use crate::ser::{utils, writer, Error, ErrorKind, Result};
#[cfg(feature = "datetime")]
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

/// A serializer for TOML values.
#[derive(Debug)]
pub struct Serializer<'a, W> {
    writer: &'a mut W,
}

impl<'a, W> Serializer<'a, W>
where
    W: fmt::Write,
{
    /// Creates a new serializer that writes to the given writer.
    #[inline]
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<'a, W> ser::Serializer for Serializer<'a, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ArraySerializer<'a, W>;
    type SerializeTuple = ArraySerializer<'a, W>;
    type SerializeTupleStruct = ArraySerializer<'a, W>;
    type SerializeTupleVariant = WrappedArraySerializer<'a, W>;
    type SerializeMap = TableSerializer<'a, W>;
    #[cfg(feature = "datetime")]
    type SerializeStruct = TableOrDatetimeSerializer<'a, W>;
    #[cfg(not(feature = "datetime"))]
    type SerializeStruct = TableSerializer<'a, W>;
    type SerializeStructVariant = WrappedTableSerializer<'a, W>;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok> {
        self.writer
            .write_str(if value { "true" } else { "false" })?;
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok> {
        self.serialize_float(&value)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok> {
        self.serialize_float(&value)
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Self::Ok> {
        self.serialize_str(value.encode_utf8(&mut [0; 4]))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        Formatter::write_string(value, self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        use ser::SerializeSeq as _;

        let mut seq = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        Err(ErrorKind::UnsupportedValue("None").into())
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(ErrorKind::UnsupportedType("()").into())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Self::SerializeSeq::start(self.writer)
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Self::SerializeTuple::start(self.writer)
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Self::SerializeTupleStruct::start(self.writer)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Self::SerializeTupleVariant::start(variant, self.writer)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Self::SerializeMap::start(self.writer)
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        match name {
            #[cfg(feature = "datetime")]
            name => Self::SerializeStruct::start(name, self.writer),
            #[cfg(not(feature = "datetime"))]
            _ => Self::SerializeStruct::start(self.writer),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Self::SerializeStructVariant::start(variant, self.writer)
    }
}

impl<W> Serializer<'_, W>
where
    W: fmt::Write,
{
    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    #[inline]
    fn serialize_integer<T: writer::Integer>(
        self,
        value: &T,
    ) -> Result<<Self as ser::Serializer>::Ok> {
        Formatter::write_integer(value, self.writer)?;
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    #[inline]
    fn serialize_float<T: writer::Float>(self, value: &T) -> Result<<Self as ser::Serializer>::Ok> {
        Formatter::write_float(value, self.writer)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ArraySerializer<'a, W> {
    writer: &'a mut W,
    first: bool,
}

impl<'a, W> ArraySerializer<'a, W>
where
    W: fmt::Write,
{
    pub fn start(writer: &'a mut W) -> Result<Self> {
        writer.write_str("[")?;

        Ok(Self {
            writer,
            first: true,
        })
    }
}

impl<W> ser::SerializeSeq for ArraySerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.writer.write_str(", ")?;
        }
        self.first = false;

        value.serialize(Serializer::new(self.writer))
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.writer.write_str("]")?;
        Ok(())
    }
}

impl<W> ser::SerializeTuple for ArraySerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

impl<W> ser::SerializeTupleStruct for ArraySerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

#[derive(Debug)]
pub struct WrappedArraySerializer<'a, W> {
    writer: &'a mut W,
    first: bool,
}

impl<'a, W> WrappedArraySerializer<'a, W>
where
    W: fmt::Write,
{
    #[inline]
    pub fn start(key: &'static str, writer: &'a mut W) -> Result<Self> {
        use serde::Serializer as _;

        writer.write_str("{ ")?;
        utils::KeySerializer::new(writer).serialize_str(key)?;
        writer.write_str(" = [")?;

        Ok(Self {
            writer,
            first: true,
        })
    }
}

impl<W> ser::SerializeTupleVariant for WrappedArraySerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.writer.write_str(", ")?;
        }
        self.first = false;

        value.serialize(Serializer::new(self.writer))
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.writer.write_str("] }")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TableSerializer<'a, W> {
    writer: &'a mut W,
    first: bool,
}

impl<'a, W> TableSerializer<'a, W>
where
    W: fmt::Write,
{
    #[inline]
    pub fn start(writer: &'a mut W) -> Result<Self> {
        writer.write_str("{ ")?;

        Ok(Self {
            writer,
            first: true,
        })
    }
}

impl<W> ser::SerializeMap for TableSerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.writer.write_str(", ")?;
        }
        self.first = false;

        key.serialize(utils::KeySerializer::new(self.writer))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.writer.write_str(" = ")?;
        value.serialize(Serializer::new(self.writer))
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.writer.write_str(" }")?;
        Ok(())
    }
}

impl<W> ser::SerializeStruct for TableSerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = <Self as ser::SerializeMap>::Ok;
    type Error = <Self as ser::SerializeMap>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

#[cfg(feature = "datetime")]
#[derive(Debug)]
pub enum TableOrDatetimeSerializer<'a, W> {
    OffsetDatetime { writer: &'a mut W, empty: bool },
    LocalDatetime { writer: &'a mut W, empty: bool },
    LocalDate { writer: &'a mut W, empty: bool },
    LocalTime { writer: &'a mut W, empty: bool },
    AnyDatetime { writer: &'a mut W, empty: bool },
    Table(TableSerializer<'a, W>),
}

#[cfg(feature = "datetime")]
impl<'a, W> TableOrDatetimeSerializer<'a, W>
where
    W: fmt::Write,
{
    #[inline]
    pub fn start(name: &'static str, writer: &'a mut W) -> Result<Self> {
        Ok(match name {
            AnyDatetime::WRAPPER_TYPE => Self::AnyDatetime {
                writer,
                empty: true,
            },
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime {
                writer,
                empty: true,
            },
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime {
                writer,
                empty: true,
            },
            LocalDate::WRAPPER_TYPE => Self::LocalDate {
                writer,
                empty: true,
            },
            LocalTime::WRAPPER_TYPE => Self::LocalTime {
                writer,
                empty: true,
            },
            _ => Self::Table(TableSerializer::start(writer)?),
        })
    }
}

#[cfg(feature = "datetime")]
impl<W> ser::SerializeStruct for TableOrDatetimeSerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match *self {
            // For AnyDatetime use the key to determine the type
            Self::AnyDatetime {
                ref mut writer,
                empty: ref mut empty @ true,
            } if matches!(
                key,
                OffsetDatetime::WRAPPER_FIELD
                    | LocalDatetime::WRAPPER_FIELD
                    | LocalDate::WRAPPER_FIELD
                    | LocalTime::WRAPPER_FIELD
            ) =>
            {
                value.serialize(utils::RawStringSerializer { writer })?;
                *empty = false;
                Ok(())
            }
            Self::OffsetDatetime {
                ref mut writer,
                empty: ref mut empty @ true,
            } if key == OffsetDatetime::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer { writer })?;
                *empty = false;
                Ok(())
            }
            Self::LocalDatetime {
                ref mut writer,
                empty: ref mut empty @ true,
            } if key == LocalDatetime::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer { writer })?;
                *empty = false;
                Ok(())
            }
            Self::LocalDate {
                ref mut writer,
                empty: ref mut empty @ true,
            } if key == LocalDate::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer { writer })?;
                *empty = false;
                Ok(())
            }
            Self::LocalTime {
                ref mut writer,
                empty: ref mut empty @ true,
            } if key == LocalTime::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer { writer })?;
                *empty = false;
                Ok(())
            }
            Self::AnyDatetime { empty: false, .. }
            | Self::OffsetDatetime { empty: false, .. }
            | Self::LocalDatetime { empty: false, .. }
            | Self::LocalDate { empty: false, .. }
            | Self::LocalTime { empty: false, .. } => Err(ErrorKind::UnsupportedValue(
                "date-time wrapper with more than one member",
            )
            .into()),
            Self::AnyDatetime { .. }
            | Self::OffsetDatetime { .. }
            | Self::LocalDatetime { .. }
            | Self::LocalDate { .. }
            | Self::LocalTime { .. } => Err(ErrorKind::UnsupportedValue(key).into()),
            Self::Table(ref mut ser) => ser.serialize_field(key, value),
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::AnyDatetime { empty: false, .. }
            | Self::OffsetDatetime { empty: false, .. }
            | Self::LocalDatetime { empty: false, .. }
            | Self::LocalDate { empty: false, .. }
            | Self::LocalTime { empty: false, .. } => Ok(()),
            Self::AnyDatetime { empty: true, .. }
            | Self::OffsetDatetime { empty: true, .. }
            | Self::LocalDatetime { empty: true, .. }
            | Self::LocalDate { empty: true, .. }
            | Self::LocalTime { empty: true, .. } => {
                Err(ErrorKind::UnsupportedValue("empty date-time wrapper").into())
            }
            Self::Table(ser) => ser.end(),
        }
    }
}

#[derive(Debug)]
pub struct WrappedTableSerializer<'a, W> {
    writer: &'a mut W,
    first: bool,
}

impl<'a, W> WrappedTableSerializer<'a, W>
where
    W: fmt::Write,
{
    #[inline]
    pub fn start(key: &'static str, writer: &'a mut W) -> Result<Self> {
        use serde::Serializer as _;

        writer.write_str("{ ")?;
        utils::KeySerializer::new(writer).serialize_str(key)?;
        writer.write_str(" = { ")?;

        Ok(Self {
            writer,
            first: true,
        })
    }
}

impl<W> ser::SerializeStructVariant for WrappedTableSerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use serde::Serializer as _;

        if !self.first {
            self.writer.write_str(", ")?;
        }
        self.first = false;

        utils::KeySerializer::new(self.writer).serialize_str(key)?;

        self.writer.write_str(" = ")?;

        value.serialize(Serializer::new(self.writer))
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.writer.write_str(" } }")?;
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use serde::Serializer as _;
    #[cfg(feature = "datetime")]
    use serde_bytes::Bytes;

    use super::*;

    #[test]
    fn serializer_new() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        assert_eq!(serializer.writer.as_ptr(), buf.as_ptr());
    }

    #[test]
    fn serializer_serialize_bool() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_bool(true).unwrap();
        assert_eq!(buf, "true");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_bool(false).unwrap();
        assert_eq!(buf, "false");
    }

    #[test]
    fn serializer_serialize_i8() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i8(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i8(-12).unwrap();
        assert_eq!(buf, "-12");
    }

    #[test]
    fn serializer_serialize_i16() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i16(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i16(-12).unwrap();
        assert_eq!(buf, "-12");
    }

    #[test]
    fn serializer_serialize_i32() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i32(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i32(-12).unwrap();
        assert_eq!(buf, "-12");
    }

    #[test]
    fn serializer_serialize_i64() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i64(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i64(-12).unwrap();
        assert_eq!(buf, "-12");
    }

    #[test]
    fn serializer_serialize_i128() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i128(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_i128(-12).unwrap();
        assert_eq!(buf, "-12");
    }

    #[test]
    fn serializer_serialize_u8() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u8(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u8(12).unwrap();
        assert_eq!(buf, "12");
    }

    #[test]
    fn serializer_serialize_u16() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u16(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u16(12).unwrap();
        assert_eq!(buf, "12");
    }

    #[test]
    fn serializer_serialize_u32() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u32(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u32(12).unwrap();
        assert_eq!(buf, "12");
    }

    #[test]
    fn serializer_serialize_u64() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u64(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u64(12).unwrap();
        assert_eq!(buf, "12");
    }

    #[test]
    fn serializer_serialize_u128() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u128(42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_u128(12).unwrap();
        assert_eq!(buf, "12");
    }

    #[test]
    fn serializer_serialize_f32() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(42.0).unwrap();
        assert_eq!(buf, "42.0");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(-12.0).unwrap();
        assert_eq!(buf, "-12.0");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(1e28).unwrap();
        assert_eq!(buf, "1e28");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(0.5e-9).unwrap();
        assert_eq!(buf, "5e-10");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(f32::INFINITY).unwrap();
        assert_eq!(buf, "inf");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(f32::NEG_INFINITY).unwrap();
        assert_eq!(buf, "-inf");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(f32::NAN).unwrap();
        assert_eq!(buf, "nan");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f32(-f32::NAN).unwrap();
        assert_eq!(buf, "-nan");
    }

    #[test]
    fn serializer_serialize_f64() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(42.0).unwrap();
        assert_eq!(buf, "42.0");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(-12.0).unwrap();
        assert_eq!(buf, "-12.0");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(1e28).unwrap();
        assert_eq!(buf, "1e28");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(0.5e-9).unwrap();
        assert_eq!(buf, "5e-10");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(f64::INFINITY).unwrap();
        assert_eq!(buf, "inf");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(f64::NEG_INFINITY).unwrap();
        assert_eq!(buf, "-inf");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(f64::NAN).unwrap();
        assert_eq!(buf, "nan");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_f64(-f64::NAN).unwrap();
        assert_eq!(buf, "-nan");
    }

    #[test]
    fn serializer_serialize_char() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_char('a').unwrap();
        assert_eq!(buf, r#""a""#);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_char('😎').unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_char('\n').unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """

                """"#}
        );
    }

    #[test]
    fn serializer_serialize_str() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_str("foo").unwrap();
        assert_eq!(buf, r#""foo""#);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_str("😎").unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_str("abc\ndef\n").unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """
                abc
                def
                """"#}
        );
    }

    #[test]
    fn serializer_serialize_bytes() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_bytes(b"foo").unwrap();
        assert_eq!(buf, "[102, 111, 111]");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_bytes(b"\xF0\x9F\x98\x8E").unwrap();
        assert_eq!(buf, "[240, 159, 152, 142]");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_bytes(b"abc\ndef\n").unwrap();
        assert_eq!(buf, "[97, 98, 99, 10, 100, 101, 102, 10]");
    }

    #[test]
    fn serializer_serialize_none() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        assert_matches!(
            serializer.serialize_none(),
            Err(Error(ErrorKind::UnsupportedValue(..)))
        );
    }

    #[test]
    fn serializer_serialize_some() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_some(&42).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_some("foo").unwrap();
        assert_eq!(buf, r#""foo""#);
    }

    #[test]
    fn serializer_serialize_unit() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        assert_matches!(
            serializer.serialize_unit(),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
    }

    #[test]
    fn serializer_serialize_unit_struct() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        assert_matches!(
            serializer.serialize_unit_struct("name"),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
    }

    #[test]
    fn serializer_serialize_unit_variant() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_unit_variant("name", 0, "foo").unwrap();
        assert_eq!(buf, r#""foo""#);
    }

    #[test]
    fn serializer_serialize_newtype_struct() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer.serialize_newtype_struct("name", &42).unwrap();
        assert_eq!(buf, "42");
    }

    #[test]
    fn serializer_serialize_newtype_variant() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        serializer
            .serialize_newtype_variant("name", 0, "foo", &42)
            .unwrap();
        assert_eq!(buf, "{ foo = 42 }");
    }

    #[test]
    fn serializer_serialize_seq() {
        use ser::SerializeSeq as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer.serialize_seq(Some(2)).unwrap();
        seq.serialize_element(&42).unwrap();
        seq.serialize_element(&"foo").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"[42, "foo"]"#);
    }

    #[test]
    fn serializer_serialize_tuple() {
        use ser::SerializeTuple as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer.serialize_tuple(2).unwrap();
        seq.serialize_element(&42).unwrap();
        seq.serialize_element(&"foo").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"[42, "foo"]"#);
    }

    #[test]
    fn serializer_serialize_tuple_struct() {
        use ser::SerializeTupleStruct as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer.serialize_tuple_struct("name", 2).unwrap();
        seq.serialize_field(&42).unwrap();
        seq.serialize_field(&"foo").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"[42, "foo"]"#);
    }

    #[test]
    fn serializer_serialize_tuple_variant() {
        use ser::SerializeTupleVariant as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer
            .serialize_tuple_variant("name", 0, "foo", 2)
            .unwrap();
        seq.serialize_field(&42).unwrap();
        seq.serialize_field(&"bar").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"{ foo = [42, "bar"] }"#);
    }

    #[test]
    fn serializer_serialize_map() {
        use ser::SerializeMap as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer.serialize_map(Some(2)).unwrap();
        seq.serialize_entry("foo", &42).unwrap();
        seq.serialize_entry("bar", &"baz").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"{ foo = 42, bar = "baz" }"#);
    }

    #[test]
    fn serializer_serialize_struct() {
        use ser::SerializeStruct as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer.serialize_struct("name", 2).unwrap();
        seq.serialize_field("foo", &42).unwrap();
        seq.serialize_field("bar", &"baz").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"{ foo = 42, bar = "baz" }"#);
    }

    #[test]
    fn serializer_serialize_struct_variant() {
        use ser::SerializeStructVariant as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut seq = serializer
            .serialize_struct_variant("name", 0, "foo", 2)
            .unwrap();
        seq.serialize_field("bar", &42).unwrap();
        seq.serialize_field("baz", &"qux").unwrap();
        seq.end().unwrap();

        assert_eq!(buf, r#"{ foo = { bar = 42, baz = "qux" } }"#);
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn serializer_serialize_datetime() {
        use ser::SerializeStruct as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(OffsetDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, OffsetDatetime::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(LocalDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalDatetime::WRAPPER_FIELD,
            Bytes::new(LocalDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, LocalDatetime::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(LocalDate::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalDate::WRAPPER_FIELD,
            Bytes::new(LocalDate::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, LocalDate::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(LocalTime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalTime::WRAPPER_FIELD,
            Bytes::new(LocalTime::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, LocalTime::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, OffsetDatetime::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalDatetime::WRAPPER_FIELD,
            Bytes::new(LocalDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, LocalDatetime::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalDate::WRAPPER_FIELD,
            Bytes::new(LocalDate::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, LocalDate::EXAMPLE_STR);

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        let mut seq = serializer
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalTime::WRAPPER_FIELD,
            Bytes::new(LocalTime::EXAMPLE_BYTES),
        )
        .unwrap();
        seq.end().unwrap();
        assert_eq!(buf, LocalTime::EXAMPLE_STR);
    }

    #[cfg(feature = "datetime")]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn serializer_serialize_datetime_error() {
        use ser::SerializeStruct as _;

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(OffsetDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        assert_matches!(
            seq.serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            ),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(LocalDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalDatetime::WRAPPER_FIELD,
            Bytes::new(LocalDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        assert_matches!(
            seq.serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                Bytes::new(LocalDatetime::EXAMPLE_BYTES),
            ),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(LocalDate::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalDate::WRAPPER_FIELD,
            Bytes::new(LocalDate::EXAMPLE_BYTES),
        )
        .unwrap();
        assert_matches!(
            seq.serialize_field(
                LocalDate::WRAPPER_FIELD,
                Bytes::new(LocalDate::EXAMPLE_BYTES),
            ),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(LocalTime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            LocalTime::WRAPPER_FIELD,
            Bytes::new(LocalTime::EXAMPLE_BYTES),
        )
        .unwrap();
        assert_matches!(
            seq.serialize_field(
                LocalTime::WRAPPER_FIELD,
                Bytes::new(LocalTime::EXAMPLE_BYTES),
            ),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        seq.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
        )
        .unwrap();
        assert_matches!(
            seq.serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            ),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(OffsetDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(
            seq.serialize_field("foo", Bytes::new(b"bar"),),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(LocalDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(
            seq.serialize_field("foo", Bytes::new(b"bar"),),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(LocalDate::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(
            seq.serialize_field("foo", Bytes::new(b"bar"),),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(LocalTime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(
            seq.serialize_field("foo", Bytes::new(b"bar"),),
            Err(Error(..))
        );

        let mut buf = String::new();
        let mut seq = Serializer::new(&mut buf)
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(
            seq.serialize_field("foo", Bytes::new(b"bar"),),
            Err(Error(..))
        );

        let mut buf = String::new();
        let seq = Serializer::new(&mut buf)
            .serialize_struct(OffsetDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(seq.end(), Err(Error(..)));

        let mut buf = String::new();
        let seq = Serializer::new(&mut buf)
            .serialize_struct(LocalDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(seq.end(), Err(Error(..)));

        let mut buf = String::new();
        let seq = Serializer::new(&mut buf)
            .serialize_struct(LocalDate::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(seq.end(), Err(Error(..)));

        let mut buf = String::new();
        let seq = Serializer::new(&mut buf)
            .serialize_struct(LocalTime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(seq.end(), Err(Error(..)));

        let mut buf = String::new();
        let seq = Serializer::new(&mut buf)
            .serialize_struct(AnyDatetime::WRAPPER_TYPE, 1)
            .unwrap();
        assert_matches!(seq.end(), Err(Error(..)));
    }
}
