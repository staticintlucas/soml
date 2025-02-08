use std::marker::PhantomData;

use serde::{ser, Serialize as _};

use super::error::{Error, ErrorKind, Result};
use super::writer::Writer;
use super::{InlineSerializer, Serializer};
use crate::value::{Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

#[doc(hidden)]
#[derive(Debug)]
pub enum ValueKind {
    // Simple value (int, float, string, etc) or inline array/table/etc
    InlineValue(String),
    // Table or array of tables
    Table(TableKind),
}

#[doc(hidden)]
#[derive(Debug)]
pub enum TableKind {
    // A regular table
    Table(Vec<(String, ValueKind)>),
    // An array of tables
    Array(Vec<Vec<(String, ValueKind)>>),
}

impl ValueKind {
    fn into_inline_value(self) -> Result<String> {
        use ser::{SerializeMap as _, SerializeSeq as _};

        match self {
            Self::InlineValue(value) => Ok(value),
            Self::Table(TableKind::Table(table)) => {
                let mut table_serializer =
                    InlineTableSerializer::<RawStringSerializer>::start(Some(table.len()))?;
                table.into_iter().try_for_each(|(k, v)| {
                    table_serializer.serialize_entry(&k, &v.into_inline_value()?)
                })?;
                table_serializer.end()
            }
            Self::Table(TableKind::Array(array)) => {
                let mut array_serializer =
                    InlineArraySerializer::<RawStringSerializer>::start(Some(array.len()))?;
                array.into_iter().try_for_each(|table| {
                    let mut table_serializer =
                        InlineTableSerializer::<RawStringSerializer>::start(Some(table.len()))?;
                    table.into_iter().try_for_each(|(k, v)| {
                        table_serializer.serialize_entry(&k, &v.into_inline_value()?)
                    })?;
                    array_serializer.serialize_element(&table_serializer.end()?)
                })?;
                array_serializer.end()
            }
        }
    }
}

// Adapted from: https://github.com/serde-rs/serde/blob/04ff3e8/serde/src/private/doc.rs#L47
#[macro_export(local_inner_macros)]
macro_rules! __serialize_unimplemented {
    ($($func:ident)*) => {
        $(
            __serialize_unimplemented_helper!($func);
        )*
    };
}
pub(crate) use __serialize_unimplemented;

#[macro_export(local_inner_macros)]
macro_rules! __serialize_unimplemented_method {
    ($func:ident $(<$t:ident>)* ($($arg:ty),*) -> $ret:ident, $msg:expr) => {
        fn $func $(<$t>)* (self $(, _: $arg)*) -> Result<Self::$ret>
        where
            $($t: ?Sized + ::serde::Serialize,)*
        {
            Err(ErrorKind::UnsupportedType($msg).into())
        }
    };

    ($func:ident $(<$t:ident>)* (name: $name:ty $(, $arg:ty)*) -> $ret:ident) => {
        fn $func $(<$t>)* (self, name: $name $(, _: $arg)*) -> Result<Self::$ret>
        where
            $($t: ?Sized + ::serde::Serialize,)*
        {
            Err(ErrorKind::UnsupportedType(name).into())
        }
    };
}

#[macro_export(local_inner_macros)]
macro_rules! __serialize_unimplemented_helper {
    (bool) => {
        __serialize_unimplemented_method!(serialize_bool(bool) -> Ok, "bool");
    };
    (i8) => {
        __serialize_unimplemented_method!(serialize_i8(i8) -> Ok, "i8");
    };
    (i16) => {
        __serialize_unimplemented_method!(serialize_i16(i16) -> Ok, "i16");
    };
    (i32) => {
        __serialize_unimplemented_method!(serialize_i32(i32) -> Ok, "i32");
    };
    (i64) => {
        __serialize_unimplemented_method!(serialize_i64(i64) -> Ok, "i64");
    };
    (i128) => {
        __serialize_unimplemented_method!(serialize_i128(i128) -> Ok, "i128");
    };
    (u8) => {
        __serialize_unimplemented_method!(serialize_u8(u8) -> Ok, "u8");
    };
    (u16) => {
        __serialize_unimplemented_method!(serialize_u16(u16) -> Ok, "u16");
    };
    (u32) => {
        __serialize_unimplemented_method!(serialize_u32(u32) -> Ok, "u32");
    };
    (u64) => {
        __serialize_unimplemented_method!(serialize_u64(u64) -> Ok, "u64");
    };
    (u128) => {
        __serialize_unimplemented_method!(serialize_u128(u128) -> Ok, "u128");
    };
    (f32) => {
        __serialize_unimplemented_method!(serialize_f32(f32) -> Ok, "f32");
    };
    (f64) => {
        __serialize_unimplemented_method!(serialize_f64(f64) -> Ok, "f64");
    };
    (char) => {
        __serialize_unimplemented_method!(serialize_char(char) -> Ok, "char");
    };
    (str) => {
        __serialize_unimplemented_method!(serialize_str(&str) -> Ok, "str");
    };
    (bytes) => {
        __serialize_unimplemented_method!(serialize_bytes(&[u8]) -> Ok, "[u8]");
    };
    (none) => {
        __serialize_unimplemented_method!(serialize_none() -> Ok, "Option");
    };
    (some) => {
        __serialize_unimplemented_method!(serialize_some<T>(&T) -> Ok, "Option");
    };
    (unit) => {
        __serialize_unimplemented_method!(serialize_unit() -> Ok, "()");
    };
    (unit_struct) => {
        __serialize_unimplemented_method!(serialize_unit_struct(name: &'static str) -> Ok);
    };
    (unit_variant) => {
        __serialize_unimplemented_method!(serialize_unit_variant(name: &'static str, u32, &str) -> Ok);
    };
    (newtype_struct) => {
        __serialize_unimplemented_method!(serialize_newtype_struct<T>(name: &'static str, &T) -> Ok);
    };
    (newtype_variant) => {
        __serialize_unimplemented_method!(serialize_newtype_variant<T>(name: &'static str, u32, &str, &T) -> Ok);
    };
    (seq) => {
        type SerializeSeq = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_seq(Option<usize>) -> SerializeSeq, "slice");
    };
    (tuple) => {
        type SerializeTuple = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_tuple(usize) -> SerializeTuple, "tuple");
    };
    (tuple_struct) => {
        type SerializeTupleStruct = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_tuple_struct(name: &'static str, usize) -> SerializeTupleStruct);
    };
    (tuple_variant) => {
        type SerializeTupleVariant = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_tuple_variant(name: &'static str, u32, &str, usize) -> SerializeTupleVariant);
    };
    (map) => {
        type SerializeMap = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_map(Option<usize>) -> SerializeMap, "map");
    };
    (struct) => {
        type SerializeStruct = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_struct(name: &'static str, usize) -> SerializeStruct);
    };
    (struct_variant) => {
        type SerializeStructVariant = ::serde::ser::Impossible<Self::Ok, Self::Error>;
        __serialize_unimplemented_method!(serialize_struct_variant(name: &'static str, u32, &str, usize) -> SerializeStructVariant);
    };
}

#[derive(Debug)]
#[doc(hidden)]
pub struct WrappedArraySerializer<W> {
    ser: Serializer<W>,
    kind: WrappedArrayKindSerializer,
}

impl<W> WrappedArraySerializer<W> {
    pub fn start(ser: Serializer<W>, len: usize, key: &'static str) -> Result<Self> {
        Ok(Self {
            ser,
            kind: WrappedArrayKindSerializer::start(len, key)?,
        })
    }
}

impl<W> ser::SerializeTupleVariant for WrappedArraySerializer<W>
where
    W: Writer,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_field(value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.write(self.kind.end_inner()?)
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct TableSerializer<W> {
    ser: Serializer<W>,
    kind: TableKindSerializer,
}

impl<W> TableSerializer<W> {
    pub fn start(ser: Serializer<W>, len: Option<usize>) -> Result<Self> {
        Ok(Self {
            ser,
            kind: TableKindSerializer::start(len)?,
        })
    }
}

impl<W> ser::SerializeMap for TableSerializer<W>
where
    W: Writer,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_key(key)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_value(value)
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.kind.serialize_entry(key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.write(self.kind.end_inner())
    }
}

impl<W> ser::SerializeStruct for TableSerializer<W>
where
    W: Writer,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_field(key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.write(self.kind.end_inner())
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct WrappedTableSerializer<W> {
    ser: Serializer<W>,
    kind: WrappedTableKindSerializer,
}

impl<W> WrappedTableSerializer<W> {
    pub fn start(ser: Serializer<W>, len: usize, key: &'static str) -> Result<Self> {
        Ok(Self {
            ser,
            kind: WrappedTableKindSerializer::start(len, key)?,
        })
    }
}

impl<W> ser::SerializeStructVariant for WrappedTableSerializer<W>
where
    W: Writer,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_field(key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.ser.write(self.kind.end_inner()?)
    }
}

#[derive(Debug)]
struct ValueKindSerializer;

impl ser::Serializer for ValueKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    type SerializeSeq = ArrayKindSerializer;
    type SerializeTuple = ArrayKindSerializer;
    type SerializeTupleStruct = ArrayKindSerializer;
    type SerializeTupleVariant = WrappedArrayKindSerializer;
    type SerializeMap = TableKindSerializer;
    type SerializeStruct = TableOrDatetimeKindSerializer;
    type SerializeStructVariant = WrappedTableKindSerializer;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_bool(value)
            .map(ValueKind::InlineValue)
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_char(value)
            .map(ValueKind::InlineValue)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_str(value)
            .map(ValueKind::InlineValue)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_bytes(value)
            .map(ValueKind::InlineValue)
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_none()
            .map(ValueKind::InlineValue)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        InlineSerializer
            .serialize_some(value)
            .map(ValueKind::InlineValue)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_unit()
            .map(ValueKind::InlineValue)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_unit_struct(name)
            .map(ValueKind::InlineValue)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        InlineSerializer
            .serialize_unit_variant(name, variant_index, variant)
            .map(ValueKind::InlineValue)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        InlineSerializer
            .serialize_newtype_struct(name, value)
            .map(ValueKind::InlineValue)
    }

    // TODO is this correct?
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

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Self::SerializeSeq::start(len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Self::SerializeTuple::start(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Self::SerializeTupleStruct::start(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Self::SerializeTupleVariant::start(len, variant)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Self::SerializeMap::start(len)
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Self::SerializeStruct::start(Some(len), name)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Self::SerializeStructVariant::start(len, variant)
    }
}

#[doc(hidden)]
pub trait Integer: lexical::ToLexicalWithOptions<Options = lexical::WriteIntegerOptions> {}

macro_rules! impl_integer {
    ($($t:ident)*) => ($(impl Integer for $t {})*);
}

impl_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

#[doc(hidden)]
pub trait Float: lexical::ToLexicalWithOptions<Options = lexical::WriteFloatOptions> {
    const INFINITY: Self;
    const NEG_INFINITY: Self;
    const NAN: Self;
    const NEG_NAN: Self;
}

macro_rules! impl_float {
    ($($t:ident)*) => ($(impl Float for $t {
        const INFINITY: Self = Self::INFINITY;
        const NEG_INFINITY: Self = Self::NEG_INFINITY;
        const NAN: Self = Self::NAN;
        const NEG_NAN: Self = -Self::NAN;
    })*);
}

impl_float!(f32 f64);

impl ValueKindSerializer {
    #[allow(clippy::unused_self)]
    fn serialize_integer<T: Integer>(self, value: T) -> Result<ValueKind> {
        InlineSerializer
            .serialize_integer(value)
            .map(ValueKind::InlineValue)
    }

    #[allow(clippy::unused_self)]
    fn serialize_float<T: Float>(self, value: T) -> Result<ValueKind> {
        InlineSerializer
            .serialize_float(value)
            .map(ValueKind::InlineValue)
    }
}

#[derive(Debug)]
struct ArrayKindSerializer {
    arr: Vec<ValueKind>,
}

impl ArrayKindSerializer {
    #[allow(clippy::unnecessary_wraps)]
    pub fn start(len: Option<usize>) -> Result<Self> {
        let arr = Vec::with_capacity(len.unwrap_or(0).min(256));
        Ok(Self { arr })
    }
}

impl ser::SerializeSeq for ArrayKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.arr.push(value.serialize(ValueKindSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        if self.arr.is_empty()
            || self
                .arr
                .iter()
                .any(|v| !matches!(*v, ValueKind::Table(TableKind::Table(_))))
        {
            let mut array_serializer =
                InlineArraySerializer::<RawStringSerializer>::start(Some(self.arr.len()))?;
            self.arr.into_iter().try_for_each(|value| {
                array_serializer.serialize_element(&value.into_inline_value()?)
            })?;
            array_serializer.end().map(ValueKind::InlineValue)
        } else {
            Ok(ValueKind::Table(TableKind::Array(
                self.arr
                    .into_iter()
                    .map(|table| match table {
                        ValueKind::Table(TableKind::Table(table)) => table,
                        _ => unreachable!("we just checked they're all tables"),
                    })
                    .collect(),
            )))
        }
    }
}

impl ser::SerializeTuple for ArrayKindSerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for ArrayKindSerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

#[derive(Debug)]
struct WrappedArrayKindSerializer {
    key: String,
    arr: ArrayKindSerializer,
}

impl WrappedArrayKindSerializer {
    pub fn start(len: usize, key: &'static str) -> Result<Self> {
        Ok(Self {
            key: key.to_owned(),
            arr: ArrayKindSerializer::start(Some(len))?,
        })
    }

    fn end_inner(self) -> Result<Vec<(String, ValueKind)>> {
        use ser::SerializeTuple as _;
        Ok(vec![(self.key, self.arr.end()?)])
    }
}

impl ser::SerializeTupleVariant for WrappedArrayKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeTuple as _;
        self.arr.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(ValueKind::Table(TableKind::Table(self.end_inner()?)))
    }
}

#[derive(Debug)]
struct TableKindSerializer {
    key: Option<String>,
    arr: Vec<(String, ValueKind)>,
}

impl TableKindSerializer {
    #[allow(clippy::unnecessary_wraps)]
    pub fn start(len: Option<usize>) -> Result<Self> {
        let arr = Vec::with_capacity(len.unwrap_or(0).min(256));
        Ok(Self { key: None, arr })
    }

    fn end_inner(self) -> Vec<(String, ValueKind)> {
        self.arr
    }
}

impl ser::SerializeMap for TableKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        #[allow(clippy::panic)]
        let Some(key) = self.key.take() else {
            panic!("serialize_value called without calling serialize_key first")
        };

        self.arr.push((key, value.serialize(ValueKindSerializer)?));
        Ok(())
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.arr.push((
            key.serialize(KeySerializer)?,
            value.serialize(ValueKindSerializer)?,
        ));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(ValueKind::Table(TableKind::Table(self.end_inner())))
    }
}

impl ser::SerializeStruct for TableKindSerializer {
    type Ok = <Self as ser::SerializeMap>::Ok;
    type Error = <Self as ser::SerializeMap>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

#[derive(Debug)]
enum TableOrDatetimeKindSerializer {
    Datetime(String),
    OffsetDatetime(String),
    LocalDatetime(String),
    LocalDate(String),
    LocalTime(String),
    Table(TableKindSerializer),
}

impl TableOrDatetimeKindSerializer {
    pub fn start(len: Option<usize>, name: &'static str) -> Result<Self> {
        Ok(match name {
            Datetime::WRAPPER_TYPE => Self::Datetime(String::new()),
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(String::new()),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(String::new()),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(String::new()),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(String::new()),
            _ => Self::Table(TableKindSerializer::start(len)?),
        })
    }
}

impl ser::SerializeStruct for TableOrDatetimeKindSerializer {
    type Ok = <TableKindSerializer as ser::SerializeStruct>::Ok;
    type Error = <TableKindSerializer as ser::SerializeStruct>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match *self {
            Self::Datetime(ref mut buf) if key == Datetime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::OffsetDatetime(ref mut buf) if key == OffsetDatetime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::LocalDatetime(ref mut buf) if key == LocalDatetime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::LocalDate(ref mut buf) if key == LocalDate::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::LocalTime(ref mut buf) if key == LocalTime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::Table(ref mut ser) => ser.serialize_field(key, value),

            // If we don't have the right key for one of the date types
            _ => Err(ErrorKind::UnsupportedValue(key).into()),
        }
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::Datetime(buf)
            | Self::OffsetDatetime(buf)
            | Self::LocalDatetime(buf)
            | Self::LocalDate(buf)
            | Self::LocalTime(buf) => Ok(ValueKind::InlineValue(buf)),
            Self::Table(ser) => ser.end(),
        }
    }
}

#[derive(Debug)]
struct WrappedTableKindSerializer {
    key: String,
    table: TableKindSerializer,
}

impl WrappedTableKindSerializer {
    pub fn start(len: usize, key: &'static str) -> Result<Self> {
        Ok(Self {
            key: key.to_owned(),
            table: TableKindSerializer::start(Some(len))?,
        })
    }

    fn end_inner(self) -> Result<Vec<(String, ValueKind)>> {
        use ser::SerializeMap as _;
        Ok(vec![(self.key, self.table.end()?)])
    }
}

impl ser::SerializeStructVariant for WrappedTableKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;
        self.table.serialize_entry(key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(ValueKind::Table(TableKind::Table(self.end_inner()?)))
    }
}

#[derive(Debug)]
pub struct InlineArraySerializer<S> {
    buf: String,
    first: bool,
    _ser: PhantomData<S>,
}

impl<S> InlineArraySerializer<S> {
    pub fn start(len: Option<usize>) -> Result<Self> {
        let cap = (16 * len.unwrap_or(0)).min(4096); // TODO is there a better estimate?
        let mut buf = String::with_capacity(cap);
        buf.write_str("[")?;

        Ok(Self {
            buf,
            first: true,
            _ser: PhantomData,
        })
    }
}

impl<S> ser::SerializeSeq for InlineArraySerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = String;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.buf.write_str(", ")?;
        }
        self.first = false;
        self.buf.write_str(&value.serialize(S::new())?)
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.buf.write_char(']')?;
        Ok(self.buf)
    }
}

impl<S> ser::SerializeTuple for InlineArraySerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

impl<S> ser::SerializeTupleStruct for InlineArraySerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

/// Inline table containing a single array, not array of inline tables
#[derive(Debug)]
pub struct InlineWrappedArraySerializer<S> {
    buf: String,
    first: bool,
    _ser: PhantomData<S>,
}

impl<S> InlineWrappedArraySerializer<S> {
    pub fn start(len: usize, key: &'static str) -> Result<Self> {
        let cap = (16 * len).min(4096); // TODO is there a better estimate?
        let mut buf = String::with_capacity(cap);
        buf.write_str("{ ")?;
        buf.write_str(&key.serialize(KeySerializer)?)?;
        buf.write_str(" = [")?;

        Ok(Self {
            buf,
            first: true,
            _ser: PhantomData,
        })
    }
}

impl<S> ser::SerializeTupleVariant for InlineWrappedArraySerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = String;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.buf.write_str(", ")?;
        }
        self.first = false;
        self.buf.write_str(&value.serialize(S::new())?)
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.buf.write_str("] }")?;
        Ok(self.buf)
    }
}

#[derive(Debug)]
pub struct InlineTableSerializer<S> {
    buf: String,
    first: bool,
    _ser: PhantomData<S>,
}

impl<S> InlineTableSerializer<S> {
    pub fn start(len: Option<usize>) -> Result<Self> {
        let cap = (32 * len.unwrap_or(0)).min(4096); // TODO is there a better estimate?
        let mut buf = String::with_capacity(cap);
        buf.write_str("{ ")?;

        Ok(Self {
            buf,
            first: true,
            _ser: PhantomData,
        })
    }
}

impl<S> ser::SerializeMap for InlineTableSerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = String;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.buf.write_str(", ")?;
        }
        self.first = false;
        self.buf.write_str(&key.serialize(KeySerializer)?)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.buf.write_str(" = ")?;
        self.buf.write_str(&value.serialize(S::new())?)
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.buf.write_str("}")?;
        Ok(self.buf)
    }
}

impl<S> ser::SerializeStruct for InlineTableSerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = String;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

#[derive(Debug)]
pub enum InlineTableOrDatetimeSerializer<S> {
    Datetime(String),
    OffsetDatetime(String),
    LocalDatetime(String),
    LocalDate(String),
    LocalTime(String),
    Table(InlineTableSerializer<S>),
}

impl<S> InlineTableOrDatetimeSerializer<S> {
    pub fn start(len: Option<usize>, name: &'static str) -> Result<Self> {
        Ok(match name {
            Datetime::WRAPPER_TYPE => Self::Datetime(String::new()),
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(String::new()),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(String::new()),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(String::new()),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(String::new()),
            _ => Self::Table(InlineTableSerializer::start(len)?),
        })
    }
}

impl<S> ser::SerializeStruct for InlineTableOrDatetimeSerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = String;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        // Datetime here is a struct containing the stringified date. So we use RawStringSerializer
        // to avoid serializing as a quoted TOML string
        match *self {
            Self::Datetime(ref mut buf) if key == Datetime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::OffsetDatetime(ref mut buf) if key == OffsetDatetime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::LocalDatetime(ref mut buf) if key == LocalDatetime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::LocalDate(ref mut buf) if key == LocalDate::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }
            Self::LocalTime(ref mut buf) if key == LocalTime::WRAPPER_FIELD => {
                buf.write_str(&value.serialize(RawStringSerializer)?)
            }

            // Not a date, a regular table/struct
            Self::Table(ref mut ser) => ser.serialize_field(key, value),

            // If we don't have the right key for one of the date types
            _ => Err(ErrorKind::UnsupportedValue(key).into()),
        }
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::Datetime(buf)
            | Self::OffsetDatetime(buf)
            | Self::LocalDatetime(buf)
            | Self::LocalDate(buf)
            | Self::LocalTime(buf) => Ok(buf),
            Self::Table(ser) => ser.end(),
        }
    }
}

/// Inline table containing another table
#[derive(Debug)]
pub struct InlineWrappedTableSerializer<S> {
    buf: String,
    first: bool,
    _ser: PhantomData<S>,
}

impl<S> InlineWrappedTableSerializer<S> {
    pub fn start(len: usize, variant: &'static str) -> Result<Self> {
        let cap = (32 * len).min(4096); // TODO is there a better estimate?
        let mut buf = String::with_capacity(cap);
        buf.write_str("{ ")?;
        buf.write_str(&variant.serialize(KeySerializer)?)?;
        buf.write_str(" = { ")?;

        Ok(Self {
            buf,
            first: true,
            _ser: PhantomData,
        })
    }
}

impl<S> ser::SerializeStructVariant for InlineWrappedTableSerializer<S>
where
    S: InlineValueSerializer,
{
    type Ok = String;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.buf.write_str(", ")?;
        }
        self.first = false;
        self.buf.write_str(&key.serialize(KeySerializer)?)?;
        self.buf.write_str(" = ")?;
        self.buf.write_str(&value.serialize(S::new())?)
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.buf.write_str(" } }")?;
        Ok(self.buf)
    }
}

struct KeySerializer;

impl ser::Serializer for KeySerializer {
    type Ok = String;
    type Error = Error;

    __serialize_unimplemented!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 bytes none some
        unit unit_struct unit_variant newtype_struct newtype_variant seq tuple
        tuple_struct tuple_variant map struct struct_variant
    );

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        let is_bare_key = |b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-');

        if value.is_empty() || value.bytes().any(|ch| !is_bare_key(ch)) {
            InlineSerializer.serialize_basic_str(value)
        } else {
            Ok(value.to_owned())
        }
    }
}

struct RawStringSerializer;

impl ser::Serializer for RawStringSerializer {
    type Ok = String;
    type Error = Error;

    __serialize_unimplemented!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        Ok(value.to_owned())
    }
}

pub trait InlineValueSerializer: ser::Serializer<Ok = String, Error = Error> {
    fn new() -> Self;
}

impl InlineValueSerializer for InlineSerializer {
    fn new() -> Self {
        Self
    }
}

impl InlineValueSerializer for RawStringSerializer {
    fn new() -> Self {
        Self
    }
}
