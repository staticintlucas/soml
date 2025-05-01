use std::marker::PhantomData;
use std::result::Result as StdResult;

use serde::{ser, Serialize as _};

use super::error::{Error, ErrorKind, Result};
use super::writer::Writer;
use super::{Serializer, ValueSerializer};
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

#[doc(hidden)]
#[derive(Debug, PartialEq, Eq)]
pub enum ValueKind {
    // Simple value (int, float, string, etc) or inline array/table/etc
    InlineValue(String),
    // Table or array of tables
    Table(TableKind),
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Eq)]
pub enum TableKind {
    // A regular table
    Table(Vec<(String, ValueKind)>),
    // An array of tables
    Array(Vec<Vec<(String, ValueKind)>>),
}

impl ValueKind {
    #[inline]
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

// Similar to serde::ser::Impossible, but this implements Debug so it can be unwrapped.
#[derive(Debug)]
pub struct Impossible<O, E> {
    never: Never,
    _phantom: PhantomData<(O, E)>,
}

#[derive(Debug)]
enum Never {}

impl<O, E> ser::SerializeSeq for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_element<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeTuple for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_element<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeTupleStruct for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeTupleVariant for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeMap for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_key<T>(&mut self, _key: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn serialize_value<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeStruct for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeStructVariant for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

// Adapted from: https://github.com/serde-rs/serde/blob/04ff3e8/serde/src/private/doc.rs#L47
#[doc(hidden)]
#[macro_export]
macro_rules! __serialize_unimplemented {
    ($($func:ident)*) => {
        $(
            $crate::__serialize_unimplemented_helper!($func);
        )*
    };
}
pub(crate) use __serialize_unimplemented;

#[doc(hidden)]
#[macro_export]
#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! __serialize_unimplemented_method {
    ($func:ident $(<$t:ident>)* ($($arg:ty),*) -> $ret:ident, $msg:expr) => {
        #[inline]
        fn $func $(<$t>)* (self $(, _: $arg)*) -> $crate::ser::Result<Self::$ret>
        where
            $($t: ?Sized + ::serde::Serialize,)*
        {
            Err($crate::ser::ErrorKind::UnsupportedType($msg).into())
        }
    };

    ($func:ident $(<$t:ident>)* (name: $name:ty $(, $arg:ty)*) -> $ret:ident) => {
        #[inline]
        fn $func $(<$t>)* (self, name: $name $(, _: $arg)*) -> $crate::ser::Result<Self::$ret>
        where
            $($t: ?Sized + ::serde::Serialize,)*
        {
            Err($crate::ser::ErrorKind::UnsupportedType(name).into())
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __serialize_unimplemented_helper {
    (bool) => {
        $crate::__serialize_unimplemented_method!(serialize_bool(bool) -> Ok, "bool");
    };
    (i8) => {
        $crate::__serialize_unimplemented_method!(serialize_i8(i8) -> Ok, "i8");
    };
    (i16) => {
        $crate::__serialize_unimplemented_method!(serialize_i16(i16) -> Ok, "i16");
    };
    (i32) => {
        $crate::__serialize_unimplemented_method!(serialize_i32(i32) -> Ok, "i32");
    };
    (i64) => {
        $crate::__serialize_unimplemented_method!(serialize_i64(i64) -> Ok, "i64");
    };
    (i128) => {
        $crate::__serialize_unimplemented_method!(serialize_i128(i128) -> Ok, "i128");
    };
    (u8) => {
        $crate::__serialize_unimplemented_method!(serialize_u8(u8) -> Ok, "u8");
    };
    (u16) => {
        $crate::__serialize_unimplemented_method!(serialize_u16(u16) -> Ok, "u16");
    };
    (u32) => {
        $crate::__serialize_unimplemented_method!(serialize_u32(u32) -> Ok, "u32");
    };
    (u64) => {
        $crate::__serialize_unimplemented_method!(serialize_u64(u64) -> Ok, "u64");
    };
    (u128) => {
        $crate::__serialize_unimplemented_method!(serialize_u128(u128) -> Ok, "u128");
    };
    (f32) => {
        $crate::__serialize_unimplemented_method!(serialize_f32(f32) -> Ok, "f32");
    };
    (f64) => {
        $crate::__serialize_unimplemented_method!(serialize_f64(f64) -> Ok, "f64");
    };
    (char) => {
        $crate::__serialize_unimplemented_method!(serialize_char(char) -> Ok, "char");
    };
    (str) => {
        $crate::__serialize_unimplemented_method!(serialize_str(&str) -> Ok, "str");
    };
    (bytes) => {
        $crate::__serialize_unimplemented_method!(serialize_bytes(&[u8]) -> Ok, "[u8]");
    };
    (none) => {
        $crate::__serialize_unimplemented_method!(serialize_none() -> Ok, "Option");
    };
    (some) => {
        $crate::__serialize_unimplemented_method!(serialize_some<T>(&T) -> Ok, "Option");
    };
    (unit) => {
        $crate::__serialize_unimplemented_method!(serialize_unit() -> Ok, "()");
    };
    (unit_struct) => {
        $crate::__serialize_unimplemented_method!(serialize_unit_struct(name: &'static str) -> Ok);
    };
    (unit_variant) => {
        $crate::__serialize_unimplemented_method!(serialize_unit_variant(name: &'static str, u32, &str) -> Ok);
    };
    (newtype_struct) => {
        $crate::__serialize_unimplemented_method!(serialize_newtype_struct<T>(name: &'static str, &T) -> Ok);
    };
    (newtype_variant) => {
        $crate::__serialize_unimplemented_method!(serialize_newtype_variant<T>(name: &'static str, u32, &str, &T) -> Ok);
    };
    (seq) => {
        type SerializeSeq = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_seq(Option<usize>) -> SerializeSeq, "slice");
    };
    (tuple) => {
        type SerializeTuple = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_tuple(usize) -> SerializeTuple, "tuple");
    };
    (tuple_struct) => {
        type SerializeTupleStruct = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_tuple_struct(name: &'static str, usize) -> SerializeTupleStruct);
    };
    (tuple_variant) => {
        type SerializeTupleVariant = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_tuple_variant(name: &'static str, u32, &str, usize) -> SerializeTupleVariant);
    };
    (map) => {
        type SerializeMap = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_map(Option<usize>) -> SerializeMap, "map");
    };
    (struct) => {
        type SerializeStruct = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_struct(name: &'static str, usize) -> SerializeStruct);
    };
    (struct_variant) => {
        type SerializeStructVariant = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unimplemented_method!(serialize_struct_variant(name: &'static str, u32, &str, usize) -> SerializeStructVariant);
    };
}

#[derive(Debug)]
#[doc(hidden)]
pub struct WrappedArraySerializer<W> {
    ser: Serializer<W>,
    kind: WrappedArrayKindSerializer,
}

impl<W> WrappedArraySerializer<W> {
    #[inline]
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

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_field(value)
    }

    #[inline]
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
    #[inline]
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

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_key(key)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_value(value)
    }

    #[inline]
    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.kind.serialize_entry(key, value)
    }

    #[inline]
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

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_field(key, value)
    }

    #[inline]
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
    #[inline]
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

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.kind.serialize_field(key, value)
    }

    #[inline]
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

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok> {
        ValueSerializer
            .serialize_bool(value)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Self::Ok> {
        ValueSerializer
            .serialize_char(value)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        ValueSerializer
            .serialize_str(value)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        ValueSerializer
            .serialize_bytes(value)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        ValueSerializer.serialize_none().map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        ValueSerializer
            .serialize_some(value)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        ValueSerializer.serialize_unit().map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        ValueSerializer
            .serialize_unit_struct(name)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        ValueSerializer
            .serialize_unit_variant(name, variant_index, variant)
            .map(ValueKind::InlineValue)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        ValueSerializer
            .serialize_newtype_struct(name, value)
            .map(ValueKind::InlineValue)
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
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Self::SerializeSeq::start(len)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Self::SerializeTuple::start(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Self::SerializeTupleStruct::start(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Self::SerializeTupleVariant::start(len, variant)
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Self::SerializeMap::start(len)
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Self::SerializeStruct::start(Some(len), name)
    }

    #[inline]
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
pub trait Integer: Sized {
    fn to_string(self) -> String;
}

macro_rules! impl_integer {
    ($($t:ident)*) => ($(
        impl Integer for $t {
            #[inline]
            fn to_string(self) -> String {
                <Self as std::string::ToString>::to_string(&self)
            }
        }
    )*);
}

impl_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

#[doc(hidden)]
pub trait Float {
    fn to_string(self) -> String;
}

macro_rules! impl_float {
    ($($t:ident)*) => ($(impl Float for $t {
        #[inline]
        fn to_string(self) -> String {
            if self.is_nan() {
                // Ryu stringifies nan as NaN and never prints the sign, TOML wants lowercase and
                // we want to preserve the sign
                if self.is_sign_positive() { "nan" } else { "-nan" }.into()
            } else {
                let mut buf = ryu::Buffer::new();
                buf.format(self).to_string()
            }
        }
    })*);
}

impl_float!(f32 f64);

impl ValueKindSerializer {
    #[allow(clippy::unused_self)]
    #[inline]
    fn serialize_integer<T: Integer>(self, value: T) -> Result<ValueKind> {
        ValueSerializer
            .serialize_integer(value)
            .map(ValueKind::InlineValue)
    }

    #[allow(clippy::unused_self)]
    #[inline]
    fn serialize_float<T: Float>(self, value: T) -> Result<ValueKind> {
        ValueSerializer
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
    #[inline]
    pub fn start(len: Option<usize>) -> Result<Self> {
        let arr = Vec::with_capacity(len.unwrap_or(0).min(256));
        Ok(Self { arr })
    }
}

impl ser::SerializeSeq for ArrayKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.arr.push(value.serialize(ValueKindSerializer)?);
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        if !self.arr.is_empty()
            && self
                .arr
                .iter()
                .all(|v| matches!(*v, ValueKind::Table(TableKind::Table(_))))
        {
            Ok(ValueKind::Table(TableKind::Array(
                self.arr
                    .into_iter()
                    .map(|table| match table {
                        ValueKind::Table(TableKind::Table(table)) => table,
                        _ => unreachable!("we just checked they're all tables"),
                    })
                    .collect(),
            )))
        } else {
            let mut array_serializer =
                InlineArraySerializer::<RawStringSerializer>::start(Some(self.arr.len()))?;
            self.arr.into_iter().try_for_each(|value| {
                array_serializer.serialize_element(&value.into_inline_value()?)
            })?;
            array_serializer.end().map(ValueKind::InlineValue)
        }
    }
}

impl ser::SerializeTuple for ArrayKindSerializer {
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

impl ser::SerializeTupleStruct for ArrayKindSerializer {
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
struct WrappedArrayKindSerializer {
    key: String,
    arr: ArrayKindSerializer,
}

impl WrappedArrayKindSerializer {
    #[inline]
    pub fn start(len: usize, key: &'static str) -> Result<Self> {
        Ok(Self {
            key: key.to_owned(),
            arr: ArrayKindSerializer::start(Some(len))?,
        })
    }

    #[inline]
    fn end_inner(self) -> Result<Vec<(String, ValueKind)>> {
        use ser::SerializeTuple as _;
        Ok(vec![(self.key, self.arr.end()?)])
    }
}

impl ser::SerializeTupleVariant for WrappedArrayKindSerializer {
    type Ok = ValueKind;
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
    #[inline]
    pub fn start(len: Option<usize>) -> Result<Self> {
        let arr = Vec::with_capacity(len.unwrap_or(0).min(256));
        Ok(Self { key: None, arr })
    }

    #[inline]
    fn end_inner(self) -> Vec<(String, ValueKind)> {
        self.arr
    }
}

impl ser::SerializeMap for TableKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.key = Some(key.serialize(KeySerializer)?);
        Ok(())
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        #[allow(clippy::panic)]
        let Some(key) = self.key.take() else {
            panic!("TableKindSerializer::serialize_value called without calling TableKindSerializer::serialize_key first")
        };

        self.arr.push((key, value.serialize(ValueKindSerializer)?));
        Ok(())
    }

    #[inline]
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

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        Ok(ValueKind::Table(TableKind::Table(self.end_inner())))
    }
}

impl ser::SerializeStruct for TableKindSerializer {
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
enum TableOrDatetimeKindSerializer {
    AnyDatetime, // Used if we see AnyDatetime::WRAPPER_TYPE, use the *::WRAPPER_FIELD to determine which type to use
    OffsetDatetime(Option<Vec<u8>>),
    LocalDatetime(Option<Vec<u8>>),
    LocalDate(Option<Vec<u8>>),
    LocalTime(Option<Vec<u8>>),
    Table(TableKindSerializer),
}

impl TableOrDatetimeKindSerializer {
    #[inline]
    pub fn start(len: Option<usize>, name: &'static str) -> Result<Self> {
        Ok(match name {
            AnyDatetime::WRAPPER_TYPE => Self::AnyDatetime,
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(None),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(None),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(None),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(None),
            _ => Self::Table(TableKindSerializer::start(len)?),
        })
    }
}

impl ser::SerializeStruct for TableOrDatetimeKindSerializer {
    type Ok = <TableKindSerializer as ser::SerializeStruct>::Ok;
    type Error = <TableKindSerializer as ser::SerializeStruct>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match *self {
            // For AnyDatetime use the key to determine the type
            Self::AnyDatetime => match key {
                OffsetDatetime::WRAPPER_FIELD => {
                    *self = Self::OffsetDatetime(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalDatetime::WRAPPER_FIELD => {
                    *self = Self::LocalDatetime(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalDate::WRAPPER_FIELD => {
                    *self = Self::LocalDate(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalTime::WRAPPER_FIELD => {
                    *self = Self::LocalTime(Some(value.serialize(RawBytesSerializer)?));
                }
                _ => return Err(ErrorKind::UnsupportedValue(key).into()),
            },
            Self::OffsetDatetime(ref mut inner @ None) if key == OffsetDatetime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalDatetime(ref mut inner @ None) if key == LocalDatetime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalDate(ref mut inner @ None) if key == LocalDate::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalTime(ref mut inner @ None) if key == LocalTime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::OffsetDatetime(Some(_))
            | Self::LocalDatetime(Some(_))
            | Self::LocalDate(Some(_))
            | Self::LocalTime(Some(_)) => {
                return Err(ErrorKind::UnsupportedValue(
                    "datetime wrapper with more than one member",
                )
                .into())
            }
            Self::OffsetDatetime(_)
            | Self::LocalDatetime(_)
            | Self::LocalDate(_)
            | Self::LocalTime(_) => return Err(ErrorKind::UnsupportedValue(key).into()),
            Self::Table(ref mut ser) => ser.serialize_field(key, value)?,
        }
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::OffsetDatetime(Some(bytes)) => bytes
                .try_into()
                .map(|b| ValueKind::InlineValue(OffsetDatetime::from_encoded(b).to_string()))
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalDatetime(Some(bytes)) => bytes
                .try_into()
                .map(|b| ValueKind::InlineValue(LocalDatetime::from_encoded(b).to_string()))
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalDate(Some(bytes)) => bytes
                .try_into()
                .map(|b| ValueKind::InlineValue(LocalDate::from_encoded(b).to_string()))
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalTime(Some(bytes)) => bytes
                .try_into()
                .map(|b| ValueKind::InlineValue(LocalTime::from_encoded(b).to_string()))
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::AnyDatetime
            | Self::OffsetDatetime(None)
            | Self::LocalDatetime(None)
            | Self::LocalDate(None)
            | Self::LocalTime(None) => {
                Err(ErrorKind::UnsupportedValue("empty date-time wrapper").into())
            }
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
    #[inline]
    pub fn start(len: usize, key: &'static str) -> Result<Self> {
        Ok(Self {
            key: key.to_owned(),
            table: TableKindSerializer::start(Some(len))?,
        })
    }

    #[inline]
    fn end_inner(self) -> Result<Vec<(String, ValueKind)>> {
        use ser::SerializeMap as _;
        Ok(vec![(self.key, self.table.end()?)])
    }
}

impl ser::SerializeStructVariant for WrappedTableKindSerializer {
    type Ok = ValueKind;
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;
        self.table.serialize_entry(key, value)
    }

    #[inline]
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
    #[inline]
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
    S: ValueOrRawStringSerializer,
{
    type Ok = String;
    type Error = Error;

    #[inline]
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
    #[inline]
    fn end(mut self) -> Result<Self::Ok> {
        self.buf.write_char(']')?;
        Ok(self.buf)
    }
}

impl<S> ser::SerializeTuple for InlineArraySerializer<S>
where
    S: ValueOrRawStringSerializer,
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

impl<S> ser::SerializeTupleStruct for InlineArraySerializer<S>
where
    S: ValueOrRawStringSerializer,
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
pub struct InlineWrappedArraySerializer<S> {
    buf: String,
    first: bool,
    _ser: PhantomData<S>,
}

impl<S> InlineWrappedArraySerializer<S> {
    #[inline]
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
    S: ValueOrRawStringSerializer,
{
    type Ok = String;
    type Error = Error;

    #[inline]
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

    #[inline]
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
    #[inline]
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
    S: ValueOrRawStringSerializer,
{
    type Ok = String;
    type Error = Error;

    #[inline]
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

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.buf.write_str(" = ")?;
        self.buf.write_str(&value.serialize(S::new())?)
    }

    #[inline]
    fn end(mut self) -> Result<Self::Ok> {
        self.buf.write_str(" }")?;
        Ok(self.buf)
    }
}

impl<S> ser::SerializeStruct for InlineTableSerializer<S>
where
    S: ValueOrRawStringSerializer,
{
    type Ok = String;
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
pub enum InlineTableOrDatetimeSerializer<S> {
    AnyDatetime, // Used if we see AnyDatetime::WRAPPER_TYPE, use the *::WRAPPER_FIELD to determine which type to use
    OffsetDatetime(Option<Vec<u8>>),
    LocalDatetime(Option<Vec<u8>>),
    LocalDate(Option<Vec<u8>>),
    LocalTime(Option<Vec<u8>>),
    Table(InlineTableSerializer<S>),
}

impl<S> InlineTableOrDatetimeSerializer<S> {
    #[inline]
    pub fn start(len: Option<usize>, name: &'static str) -> Result<Self> {
        Ok(match name {
            AnyDatetime::WRAPPER_TYPE => Self::AnyDatetime,
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(None),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(None),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(None),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(None),
            _ => Self::Table(InlineTableSerializer::start(len)?),
        })
    }
}

impl<S> ser::SerializeStruct for InlineTableOrDatetimeSerializer<S>
where
    S: ValueOrRawStringSerializer,
{
    type Ok = String;
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match *self {
            // For AnyDatetime use the key to determine the type
            Self::AnyDatetime => match key {
                OffsetDatetime::WRAPPER_FIELD => {
                    *self = Self::OffsetDatetime(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalDatetime::WRAPPER_FIELD => {
                    *self = Self::LocalDatetime(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalDate::WRAPPER_FIELD => {
                    *self = Self::LocalDate(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalTime::WRAPPER_FIELD => {
                    *self = Self::LocalTime(Some(value.serialize(RawBytesSerializer)?));
                }
                _ => return Err(ErrorKind::UnsupportedValue(key).into()),
            },
            Self::OffsetDatetime(ref mut inner @ None) if key == OffsetDatetime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalDatetime(ref mut inner @ None) if key == LocalDatetime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalDate(ref mut inner @ None) if key == LocalDate::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalTime(ref mut inner @ None) if key == LocalTime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::OffsetDatetime(Some(_))
            | Self::LocalDatetime(Some(_))
            | Self::LocalDate(Some(_))
            | Self::LocalTime(Some(_)) => {
                return Err(ErrorKind::UnsupportedValue(
                    "datetime wrapper with more than one member",
                )
                .into())
            }
            Self::OffsetDatetime(_)
            | Self::LocalDatetime(_)
            | Self::LocalDate(_)
            | Self::LocalTime(_) => return Err(ErrorKind::UnsupportedValue(key).into()),
            Self::Table(ref mut ser) => ser.serialize_field(key, value)?,
        }
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::OffsetDatetime(Some(bytes)) => bytes
                .try_into()
                .map(|b| OffsetDatetime::from_encoded(b).to_string())
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalDatetime(Some(bytes)) => bytes
                .try_into()
                .map(|b| LocalDatetime::from_encoded(b).to_string())
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalDate(Some(bytes)) => bytes
                .try_into()
                .map(|b| LocalDate::from_encoded(b).to_string())
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalTime(Some(bytes)) => bytes
                .try_into()
                .map(|b| LocalTime::from_encoded(b).to_string())
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::AnyDatetime
            | Self::OffsetDatetime(None)
            | Self::LocalDatetime(None)
            | Self::LocalDate(None)
            | Self::LocalTime(None) => {
                Err(ErrorKind::UnsupportedValue("empty date-time wrapper").into())
            }
            Self::Table(ser) => ser.end(),
        }
    }
}

#[derive(Debug)]
pub struct InlineWrappedTableSerializer<S> {
    buf: String,
    first: bool,
    _ser: PhantomData<S>,
}

impl<S> InlineWrappedTableSerializer<S> {
    #[inline]
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
    S: ValueOrRawStringSerializer,
{
    type Ok = String;
    type Error = Error;

    #[inline]
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

    #[inline]
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

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        let is_bare_key = |b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-');

        if !value.is_empty() && value.bytes().all(is_bare_key) {
            Ok(value.to_owned())
        } else {
            ValueSerializer.serialize_basic_str(value)
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

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        Ok(value.to_owned())
    }
}

pub trait ValueOrRawStringSerializer: ser::Serializer<Ok = String, Error = Error> {
    fn new() -> Self;
}

impl ValueOrRawStringSerializer for ValueSerializer {
    #[inline]
    fn new() -> Self {
        Self
    }
}

impl ValueOrRawStringSerializer for RawStringSerializer {
    #[inline]
    fn new() -> Self {
        Self
    }
}

#[derive(Debug)]
struct RawBytesSerializer;

impl ser::Serializer for RawBytesSerializer {
    type Ok = Vec<u8>;
    type Error = Error;

    __serialize_unimplemented!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        Ok(value.to_vec())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use maplit::hashmap;
    use serde_bytes::Bytes;

    use super::*;

    #[test]
    fn value_kind_into_inline_value() {
        let value = ValueKind::InlineValue("foo".to_owned());
        assert_matches!(value.into_inline_value(), Ok(s) if s == "foo");

        let value = ValueKind::Table(TableKind::Table(vec![(
            "foo".to_owned(),
            ValueKind::InlineValue(r#""bar""#.to_owned()),
        )]));
        assert_matches!(value.into_inline_value(), Ok(s) if s == r#"{ foo = "bar" }"#);

        let value = ValueKind::Table(TableKind::Array(vec![
            vec![(
                "foo".to_owned(),
                ValueKind::InlineValue(r#""bar""#.to_owned()),
            )],
            vec![(
                "foo".to_owned(),
                ValueKind::InlineValue(r#""baz""#.to_owned()),
            )],
        ]));
        assert_matches!(
            value.into_inline_value(),
            Ok(s) if s == r#"[{ foo = "bar" }, { foo = "baz" }]"#
        );
    }

    #[test]
    fn wrapped_array_serializer_serialize_tuple_variant() {
        use serde::ser::SerializeTupleVariant as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut wrapped = WrappedArraySerializer::start(serializer, 1, "array").unwrap();
        wrapped.serialize_field("foo").unwrap();
        wrapped.end().unwrap();

        assert_eq!(
            buf,
            indoc! {r#"
            array = ["foo"]
        "#}
        );
    }

    #[test]
    fn table_serializer_serialize_map() {
        use serde::ser::SerializeMap as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut wrapped = TableSerializer::start(serializer, Some(2)).unwrap();
        wrapped.serialize_entry("foo", "bar").unwrap();
        wrapped.serialize_key("baz").unwrap();
        wrapped.serialize_value("qux").unwrap();
        wrapped.end().unwrap();

        assert_eq!(
            buf,
            indoc! {r#"
            baz = "qux"
            foo = "bar"
        "#}
        );
    }

    #[test]
    fn table_serializer_serialize_struct() {
        use serde::ser::SerializeStruct as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut wrapped = TableSerializer::start(serializer, Some(1)).unwrap();
        wrapped.serialize_field("foo", "bar").unwrap();
        wrapped.end().unwrap();

        assert_eq!(
            buf,
            indoc! {r#"
            foo = "bar"
        "#}
        );
    }

    #[test]
    fn wrapped_table_serializer_serialize_struct_variant() {
        use serde::ser::SerializeStructVariant as _;

        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);

        let mut wrapped = WrappedTableSerializer::start(serializer, 1, "table").unwrap();
        wrapped.serialize_field("foo", "bar").unwrap();
        wrapped.end().unwrap();

        assert_eq!(
            buf,
            indoc! {r#"
            [table]
            foo = "bar"
        "#}
        );
    }

    #[test]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn value_kind_serializer() {
        use serde::ser::Serializer as _;

        assert_matches!(
            ValueKindSerializer.serialize_bool(true),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_i8(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_i16(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_i32(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_i64(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_i128(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_u8(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_u16(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_u32(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_u64(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_u128(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_integer(1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_f32(1.0),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_f64(1.0),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_float(1.0),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_char('a'),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_str("foo"),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_bytes(b"foo"),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_none(),
            Err(Error(ErrorKind::UnsupportedValue(..))) // TOML doesn't have a none/null type
        );
        assert_matches!(
            ValueKindSerializer.serialize_some(&1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_unit(),
            Err(Error(ErrorKind::UnsupportedType(..))) // TOML doesn't have a unit type
        );
        assert_matches!(
            ValueKindSerializer.serialize_unit_struct("foo"),
            Err(Error(ErrorKind::UnsupportedType(..))) // TOML doesn't have a unit type
        );
        assert_matches!(
            ValueKindSerializer.serialize_unit_variant("foo", 1, "bar"),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_newtype_struct("foo", &1),
            Ok(ValueKind::InlineValue(_))
        );
        assert_matches!(
            ValueKindSerializer.serialize_newtype_variant("foo", 1, "bar", &1),
            Ok(ValueKind::Table(TableKind::Table(_)))
        );
        assert_matches!(
            ValueKindSerializer.serialize_seq(Some(1)),
            Ok(ArrayKindSerializer { .. })
        );
        assert_matches!(
            ValueKindSerializer.serialize_tuple(1),
            Ok(ArrayKindSerializer { .. })
        );
        assert_matches!(
            ValueKindSerializer.serialize_tuple_struct("foo", 1),
            Ok(ArrayKindSerializer { .. })
        );
        assert_matches!(
            ValueKindSerializer.serialize_tuple_variant("foo", 1, "bar", 1),
            Ok(WrappedArrayKindSerializer { .. })
        );
        assert_matches!(
            ValueKindSerializer.serialize_map(Some(1)),
            Ok(TableKindSerializer { .. })
        );
        assert_matches!(
            ValueKindSerializer.serialize_struct("foo", 1),
            Ok(TableOrDatetimeKindSerializer::Table(
                TableKindSerializer { .. }
            ))
        );
        assert_matches!(
            ValueKindSerializer.serialize_struct_variant("foo", 1, "bar", 1),
            Ok(WrappedTableKindSerializer { .. })
        );
    }

    #[test]
    fn array_kind_serializer_serialize_seq() {
        use serde::ser::SerializeSeq as _;

        let kind_ser = ArrayKindSerializer::start(Some(0)).unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser = ArrayKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_element(&"foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser = ArrayKindSerializer::start(Some(1)).unwrap();
        kind_ser
            .serialize_element(&hashmap! { "foo" => "bar" })
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Array(_)));
    }

    #[test]
    fn array_kind_serializer_serialize_tuple() {
        use serde::ser::SerializeTuple as _;

        let kind_ser = ArrayKindSerializer::start(Some(0)).unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser = ArrayKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_element(&"foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser = ArrayKindSerializer::start(Some(1)).unwrap();
        kind_ser
            .serialize_element(&hashmap! { "foo" => "bar" })
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Array(_)));
    }

    #[test]
    fn array_kind_serializer_serialize_tuple_struct() {
        use serde::ser::SerializeTupleStruct as _;

        let kind_ser = ArrayKindSerializer::start(Some(0)).unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser = ArrayKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_field(&"foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser = ArrayKindSerializer::start(Some(1)).unwrap();
        kind_ser
            .serialize_field(&hashmap! { "foo" => "bar" })
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Array(_)));
    }

    #[test]
    fn wrapped_array_kind_serializer() {
        use serde::ser::SerializeTupleVariant as _;

        let kind_ser = WrappedArrayKindSerializer::start(0, "foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(inner))
            if matches!(&*inner, &[(ref key, ref value)]
                if key == "foo" && matches!(value, &ValueKind::InlineValue(_))));

        let mut kind_ser = WrappedArrayKindSerializer::start(1, "foo").unwrap();
        kind_ser.serialize_field("foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(inner))
            if matches!(&*inner, &[(ref key, ref value)]
                if key == "foo" && matches!(value, &ValueKind::InlineValue(_))));

        let mut kind_ser = WrappedArrayKindSerializer::start(1, "foo").unwrap();
        kind_ser
            .serialize_field(&hashmap! { "foo" => "bar" })
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(inner))
            if matches!(&*inner, &[(ref key, ref value)]
                if key == "foo" && matches!(value, &ValueKind::Table(TableKind::Array(_)))));
    }

    #[test]
    fn table_kind_serializer_serialize_map() {
        use serde::ser::SerializeMap as _;

        let kind_ser = TableKindSerializer::start(Some(0)).unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));

        let mut kind_ser = TableKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_entry("foo", "bar").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));

        let mut kind_ser = TableKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_key("foo").unwrap();
        kind_ser.serialize_value("bar").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));
    }

    #[test]
    #[should_panic = "TableKindSerializer::serialize_value called without calling TableKindSerializer::serialize_key first"]
    fn table_kind_serializer_serialize_value_without_key() {
        use serde::ser::SerializeMap as _;

        let mut kind_ser = TableKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_value("bar").unwrap();
    }

    #[test]
    fn table_kind_serializer_serialize_struct() {
        use serde::ser::SerializeStruct as _;

        let kind_ser = TableKindSerializer::start(Some(0)).unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));

        let mut kind_ser = TableKindSerializer::start(Some(1)).unwrap();
        kind_ser.serialize_field("foo", "bar").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));
    }

    #[test]
    fn table_or_datetime_kind_serializer() {
        use serde::ser::SerializeStruct as _;

        let kind_ser = TableOrDatetimeKindSerializer::start(Some(0), "foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));

        let mut kind_ser = TableOrDatetimeKindSerializer::start(Some(0), "foo").unwrap();
        kind_ser.serialize_field("foo", "bar").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(_)));

        let mut kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), OffsetDatetime::WRAPPER_TYPE).unwrap();
        kind_ser
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_ENCODED),
            )
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), LocalDatetime::WRAPPER_TYPE).unwrap();
        kind_ser
            .serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                Bytes::new(LocalDatetime::EXAMPLE_ENCODED),
            )
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), LocalDate::WRAPPER_TYPE).unwrap();
        kind_ser
            .serialize_field(
                LocalDate::WRAPPER_FIELD,
                Bytes::new(LocalDate::EXAMPLE_ENCODED),
            )
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        let mut kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), LocalTime::WRAPPER_TYPE).unwrap();
        kind_ser
            .serialize_field(
                LocalTime::WRAPPER_FIELD,
                Bytes::new(LocalTime::EXAMPLE_ENCODED),
            )
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::InlineValue(_));

        // Wrong field name
        let mut kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), OffsetDatetime::WRAPPER_TYPE).unwrap();
        assert_matches!(
            kind_ser.serialize_field("foo", "bar"),
            Err(Error(ErrorKind::UnsupportedValue(..)))
        );

        // More than one field
        let mut kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), OffsetDatetime::WRAPPER_TYPE).unwrap();
        kind_ser
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_ENCODED),
            )
            .unwrap();
        assert_matches!(
            kind_ser.serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_ENCODED)
            ),
            Err(Error(ErrorKind::UnsupportedValue(..)))
        );

        // No field
        let kind_ser =
            TableOrDatetimeKindSerializer::start(Some(1), OffsetDatetime::WRAPPER_TYPE).unwrap();
        assert_matches!(kind_ser.end(), Err(Error(ErrorKind::UnsupportedValue(..))));
    }

    #[test]
    fn wrapped_table_kind_serializer() {
        use serde::ser::SerializeStructVariant as _;

        let kind_ser = WrappedTableKindSerializer::start(0, "foo").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(inner))
            if matches!(&*inner, &[(ref key, ref value)]
                if key == "foo" && matches!(value, &ValueKind::Table(TableKind::Table(_)))));

        let mut kind_ser = WrappedTableKindSerializer::start(1, "foo").unwrap();
        kind_ser.serialize_field("foo", "bar").unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(inner))
            if matches!(&*inner, &[(ref key, ref value)]
                if key == "foo" && matches!(value, &ValueKind::Table(TableKind::Table(_)))));

        let mut kind_ser = WrappedTableKindSerializer::start(1, "foo").unwrap();
        kind_ser
            .serialize_field("foo", &hashmap! { "bar" => "baz" })
            .unwrap();
        let kind = kind_ser.end().unwrap();

        assert_matches!(kind, ValueKind::Table(TableKind::Table(inner))
            if matches!(&*inner, &[(ref key, ref value)]
                if key == "foo" && matches!(value, &ValueKind::Table(TableKind::Table(_)))));
    }

    #[test]
    fn inline_array_serializer_serialize_seq() {
        use serde::ser::SerializeSeq as _;

        let mut ser = InlineArraySerializer::<ValueSerializer>::start(Some(0)).unwrap();
        ser.serialize_element("foo").unwrap();
        ser.serialize_element("bar").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"["foo", "bar"]"#);
    }

    #[test]
    fn inline_array_serializer_serialize_tuple() {
        use serde::ser::SerializeTuple as _;

        let mut ser = InlineArraySerializer::<ValueSerializer>::start(Some(0)).unwrap();
        ser.serialize_element("foo").unwrap();
        ser.serialize_element("bar").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"["foo", "bar"]"#);
    }

    #[test]
    fn inline_array_serializer_serialize_tuple_struct() {
        use serde::ser::SerializeTupleStruct as _;

        let mut ser = InlineArraySerializer::<ValueSerializer>::start(Some(0)).unwrap();
        ser.serialize_field("foo").unwrap();
        ser.serialize_field("bar").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"["foo", "bar"]"#);
    }

    #[test]
    fn inline_wrapped_array_serializer() {
        use serde::ser::SerializeTupleVariant as _;

        let mut ser = InlineWrappedArraySerializer::<ValueSerializer>::start(0, "foo").unwrap();
        ser.serialize_field("foo").unwrap();
        ser.serialize_field("bar").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"{ foo = ["foo", "bar"] }"#);
    }

    #[test]
    fn inline_table_serializer_serialize_map() {
        use serde::ser::SerializeMap as _;

        let mut ser = InlineTableSerializer::<ValueSerializer>::start(Some(0)).unwrap();
        ser.serialize_entry("foo", "bar").unwrap();
        ser.serialize_key("baz").unwrap();
        ser.serialize_value("qux").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"{ foo = "bar", baz = "qux" }"#);
    }

    #[test]
    fn inline_table_serializer_serialize_struct() {
        use serde::ser::SerializeStruct as _;

        let mut ser = InlineTableSerializer::<ValueSerializer>::start(Some(0)).unwrap();
        ser.serialize_field("foo", "bar").unwrap();
        ser.serialize_field("baz", "qux").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"{ foo = "bar", baz = "qux" }"#);
    }

    #[test]
    fn inline_table_or_datetime_serializer() {
        use serde::ser::SerializeStruct as _;

        let mut ser =
            InlineTableOrDatetimeSerializer::<ValueSerializer>::start(Some(0), "foo").unwrap();
        ser.serialize_field("foo", "bar").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"{ foo = "bar" }"#);

        let mut ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            OffsetDatetime::WRAPPER_TYPE,
        )
        .unwrap();
        ser.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_ENCODED),
        )
        .unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, OffsetDatetime::EXAMPLE_STR);

        let mut ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            LocalDatetime::WRAPPER_TYPE,
        )
        .unwrap();
        ser.serialize_field(
            LocalDatetime::WRAPPER_FIELD,
            Bytes::new(LocalDatetime::EXAMPLE_ENCODED),
        )
        .unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, LocalDatetime::EXAMPLE_STR);

        let mut ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            LocalDate::WRAPPER_TYPE,
        )
        .unwrap();
        ser.serialize_field(
            LocalDate::WRAPPER_FIELD,
            Bytes::new(LocalDate::EXAMPLE_ENCODED),
        )
        .unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, LocalDate::EXAMPLE_STR);

        let mut ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            LocalTime::WRAPPER_TYPE,
        )
        .unwrap();
        ser.serialize_field(
            LocalTime::WRAPPER_FIELD,
            Bytes::new(LocalTime::EXAMPLE_ENCODED),
        )
        .unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, LocalTime::EXAMPLE_STR);

        // Wrong field name
        let mut ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            OffsetDatetime::WRAPPER_TYPE,
        )
        .unwrap();
        assert_matches!(
            ser.serialize_field("foo", "bar"),
            Err(Error(ErrorKind::UnsupportedValue(..)))
        );

        // More than one field
        let mut ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            OffsetDatetime::WRAPPER_TYPE,
        )
        .unwrap();
        ser.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_ENCODED),
        )
        .unwrap();
        assert_matches!(
            ser.serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_ENCODED)
            ),
            Err(Error(ErrorKind::UnsupportedValue(..)))
        );

        // No field
        let ser = InlineTableOrDatetimeSerializer::<ValueSerializer>::start(
            Some(0),
            OffsetDatetime::WRAPPER_TYPE,
        )
        .unwrap();
        assert_matches!(ser.end(), Err(Error(ErrorKind::UnsupportedValue(..))));
    }

    #[test]
    fn inline_wrapped_table_serializer() {
        use serde::ser::SerializeStructVariant as _;

        let mut ser = InlineWrappedTableSerializer::<ValueSerializer>::start(1, "foo").unwrap();
        ser.serialize_field("bar", "baz").unwrap();
        ser.serialize_field("qux", "quux").unwrap();
        let value = ser.end().unwrap();

        assert_eq!(value, r#"{ foo = { bar = "baz", qux = "quux" } }"#);
    }

    #[test]
    fn key_serializer() {
        use serde::ser::Serializer as _;

        assert_matches!(KeySerializer.serialize_char('b'), Ok(s) if s == "b");
        assert_matches!(KeySerializer.serialize_str("foo"), Ok(s) if s == "foo");
        assert_matches!(KeySerializer.serialize_str("😎"), Ok(s) if s == r#""😎""#);

        assert_matches!(
            KeySerializer.serialize_i64(1),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
        assert_matches!(
            KeySerializer.serialize_struct("foo", 1),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
    }

    #[test]
    fn raw_string_serializer() {
        use serde::ser::Serializer as _;

        assert_matches!(RawStringSerializer.serialize_str("foo"), Ok(s) if s == "foo");
        assert_matches!(RawStringSerializer.serialize_str("😎"), Ok(s) if s == "😎");

        assert_matches!(
            RawStringSerializer.serialize_i64(1),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
        assert_matches!(
            RawStringSerializer.serialize_struct("foo", 1),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
    }
}
