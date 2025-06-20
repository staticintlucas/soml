//! TOML serialization functions and trait implementations.

use std::{fmt, io};

use serde::ser;

pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, Result};
use self::writer::IoWriter;
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

mod error;
mod tree;
mod utils;
mod writer;

/// Serializes a value to a TOML string.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a TOML document.
#[inline]
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ser::Serialize,
{
    let mut dst = String::new();
    value.serialize(Serializer::from_fmt_writer(&mut dst))?;
    Ok(dst)
}

/// Serializes a value to an [`io::Write`].
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a TOML document.
#[inline]
pub fn to_io_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ser::Serialize,
{
    value.serialize(Serializer::from_io_writer(writer))
}

/// Serializes a value to a [`fmt::Write`].
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a TOML document.
#[inline]
pub fn to_fmt_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: fmt::Write,
    T: ser::Serialize,
{
    value.serialize(Serializer::from_fmt_writer(writer))
}

/// A serializer for a TOML document.
#[derive(Debug)]
pub struct Serializer<W> {
    writer: W,
}

impl<'a> Serializer<&'a mut String> {
    /// Create a new TOML serializer that serializes to the given buffer.
    #[must_use]
    #[inline]
    pub fn new(buf: &'a mut String) -> Self {
        Self::from_fmt_writer(buf)
    }
}

impl<W> Serializer<W>
where
    W: fmt::Write,
{
    /// Create a new TOML serializer that serializes to the given writer.
    #[inline]
    pub fn from_fmt_writer(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> Serializer<IoWriter<W>>
where
    W: io::Write,
{
    /// Create a new TOML serializer that serializes to the given writer.
    #[inline]
    pub fn from_io_writer(writer: W) -> Self {
        Self {
            writer: IoWriter::new(writer),
        }
    }
}

impl<W> ser::Serializer for Serializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeTupleVariant = WrappedArraySerializer<W>;
    type SerializeMap = TableSerializer<W>;
    type SerializeStruct = TableSerializer<W>;
    type SerializeStructVariant = WrappedTableSerializer<W>;

    utils::__serialize_unsupported!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str bytes none
        some unit unit_struct unit_variant newtype_struct seq tuple tuple_struct
    );

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(Self::SerializeTupleVariant::start(
            self.writer,
            variant,
            len,
        ))
    }

    #[inline]
    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        Ok(Self::SerializeMap::start(self.writer, len))
    }

    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        match name {
            AnyDatetime::WRAPPER_TYPE
            | OffsetDatetime::WRAPPER_TYPE
            | LocalDatetime::WRAPPER_TYPE
            | LocalDate::WRAPPER_TYPE
            | LocalTime::WRAPPER_TYPE => Err(ErrorKind::UnsupportedType(name).into()),
            _ => Ok(Self::SerializeStruct::start(self.writer, Some(len))),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        Ok(Self::SerializeStructVariant::start(
            self.writer,
            variant,
            len,
        ))
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct WrappedArraySerializer<W> {
    writer: W,
    key: &'static str,
    arr: tree::ArraySerializer,
}

impl<W> WrappedArraySerializer<W> {
    #[inline]
    fn start(writer: W, key: &'static str, len: usize) -> Self {
        Self {
            writer,
            key,
            arr: tree::ArraySerializer::start(Some(len)),
        }
    }
}

impl<W> ser::SerializeTupleVariant for WrappedArraySerializer<W>
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
        use ser::SerializeTuple as _;

        self.arr.serialize_element(value)
    }

    #[inline]
    fn end(mut self) -> Result<Self::Ok> {
        match self.arr.end_inner() {
            tree::Array::Inline(value) => {
                writeln!(
                    self.writer,
                    "{}",
                    writer::Inlines(&[(&self.key.to_string(), &value)])
                )?;
            }
            tree::Array::Table(array) => {
                write!(
                    self.writer,
                    "{}",
                    writer::ArrayOfTables {
                        array: &array,
                        path: &[&self.key.to_string()]
                    }
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct TableSerializer<W> {
    writer: W,
    table: tree::TableSerializer,
}

impl<W> TableSerializer<W> {
    #[inline]
    fn start(writer: W, len: Option<usize>) -> Self {
        Self {
            writer,
            table: tree::TableSerializer::start(len),
        }
    }
}

impl<W> ser::SerializeMap for TableSerializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.table.serialize_key(key)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.table.serialize_value(value)
    }

    #[inline]
    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.table.serialize_entry(key, value)
    }

    #[inline]
    fn end(mut self) -> Result<Self::Ok> {
        write!(
            self.writer,
            "{}",
            writer::Table {
                table: &self.table.end_inner(),
                path: &[],
            }
        )?;
        Ok(())
    }
}

impl<W> ser::SerializeStruct for TableSerializer<W>
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
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct WrappedTableSerializer<W> {
    writer: W,
    key: &'static str,
    table: tree::TableSerializer,
}

impl<W> WrappedTableSerializer<W> {
    #[inline]
    fn start(writer: W, key: &'static str, len: usize) -> Self {
        Self {
            writer,
            key,
            table: tree::TableSerializer::start(Some(len)),
        }
    }
}

impl<W> ser::SerializeStructVariant for WrappedTableSerializer<W>
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
        use ser::SerializeStruct as _;

        self.table.serialize_field(key, value)
    }

    #[inline]
    fn end(mut self) -> Result<Self::Ok> {
        write!(
            self.writer,
            "{}",
            writer::Table {
                table: &self.table.end_inner(),
                path: &[&self.key.to_owned()],
            }
        )?;
        Ok(())
    }
}
