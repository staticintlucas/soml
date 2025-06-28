use std::fmt;

use serde::ser;

use crate::ser::writer::Formatter;
use crate::ser::{utils, writer, Error, ErrorKind, Result};
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
    type SerializeStruct = TableOrDatetimeSerializer<'a, W>;
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
        Self::SerializeStruct::start(name, self.writer)
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

#[derive(Debug)]
pub enum TableOrDatetimeSerializer<'a, W> {
    OffsetDatetime { writer: &'a mut W, empty: bool },
    LocalDatetime { writer: &'a mut W, empty: bool },
    LocalDate { writer: &'a mut W, empty: bool },
    LocalTime { writer: &'a mut W, empty: bool },
    AnyDatetime { writer: &'a mut W, empty: bool },
    Table(TableSerializer<'a, W>),
}

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
                empty: ref mut first @ true,
            } if matches!(
                key,
                OffsetDatetime::WRAPPER_FIELD
                    | LocalDatetime::WRAPPER_FIELD
                    | LocalDate::WRAPPER_FIELD
                    | LocalTime::WRAPPER_FIELD
            ) =>
            {
                value.serialize(utils::RawStringSerializer::new(writer))?;
                *first = false;
                Ok(())
            }
            Self::OffsetDatetime {
                ref mut writer,
                empty: ref mut first @ true,
            } if key == OffsetDatetime::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer::new(writer))?;
                *first = false;
                Ok(())
            }
            Self::LocalDatetime {
                ref mut writer,
                empty: ref mut first @ true,
            } if key == LocalDatetime::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer::new(writer))?;
                *first = false;
                Ok(())
            }
            Self::LocalDate {
                ref mut writer,
                empty: ref mut first @ true,
            } if key == LocalDate::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer::new(writer))?;
                *first = false;
                Ok(())
            }
            Self::LocalTime {
                ref mut writer,
                empty: ref mut first @ true,
            } if key == LocalTime::WRAPPER_FIELD => {
                value.serialize(utils::RawStringSerializer::new(writer))?;
                *first = false;
                Ok(())
            }
            Self::AnyDatetime { empty: false, .. }
            | Self::OffsetDatetime { empty: false, .. }
            | Self::LocalDatetime { empty: false, .. }
            | Self::LocalDate { empty: false, .. }
            | Self::LocalTime { empty: false, .. } => Err(ErrorKind::UnsupportedValue(
                "datetime wrapper with more than one member",
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
