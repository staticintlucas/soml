//! TOML deserialization functions and trait implementations.

use core::str;
use std::io;
use std::result::Result as StdResult;

use serde::de::value::StrDeserializer;
use serde::de::{DeserializeOwned, Error as _, IntoDeserializer as _};
use serde::{de, Deserialize};

pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, Result};
use self::parser::{Parser, SpecialFloat, Table as ParsedTable, Value as ParsedValue};
use self::reader::Reader;
#[cfg(feature = "datetime")]
use crate::value::datetime::{
    LocalDateAccess, LocalDatetimeAccess, LocalTimeAccess, OffsetDatetimeAccess,
};
#[cfg(feature = "datetime")]
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

mod error;
mod parser;
mod reader;

/// Deserialize a value of type `T` from a TOML string slice.
///
/// # Errors
///
/// This function will return an error if the input slice is not valid TOML.
#[inline]
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    T::deserialize(Deserializer::from_str(s))
}

/// Deserialize a value of type `T` from a TOML byte slice.
///
/// # Errors
///
/// This function will return an error if the input slice is not valid TOML.
#[inline]
pub fn from_slice<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    T::deserialize(Deserializer::from_slice(bytes))
}

/// Deserialize a value of type `T` from an [`io::Read`] source.
///
/// # Errors
///
/// This function will return an error if the source is not valid TOML.
#[inline]
pub fn from_reader<R, T>(mut read: R) -> Result<T>
where
    R: io::Read,
    T: DeserializeOwned,
{
    let mut bytes = Vec::new();
    read.read_to_end(&mut bytes)?;
    T::deserialize(Deserializer::from_slice(&bytes))
}

/// A deserializer for a TOML document.
#[derive(Debug)]
pub struct Deserializer<'de> {
    parser: Parser<'de>,
}

impl<'de> Deserializer<'de> {
    /// Create a new deserializer from a string slice.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    #[inline]
    pub fn from_str(str: &'de str) -> Self {
        Self {
            parser: Parser::from_str(str),
        }
    }

    /// Create a new deserializer from a byte slice.
    #[must_use]
    #[inline]
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        Self {
            parser: Parser::from_slice(bytes),
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        ValueDeserializer::new(self.parser.parse()?).deserialize_any(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

#[derive(Debug)]
struct ValueDeserializer {
    value: ParsedValue,
}

impl ValueDeserializer {
    #[inline]
    const fn new(value: ParsedValue) -> Self {
        Self { value }
    }
}

impl<'de> de::Deserializer<'de> for ValueDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(str) => str.into_deserializer().deserialize_any(visitor),
            ParsedValue::Integer(bytes) => visitor.visit_i64(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_i64(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_i64(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_i64(parse_hexadecimal(&bytes)?),
            ParsedValue::Float(bytes) => visitor.visit_f64(parse_float(&bytes)?),
            ParsedValue::SpecialFloat(special) => visitor.visit_f64(parse_special(special)),
            ParsedValue::Boolean(bool) => visitor.visit_bool(bool),
            #[cfg(feature = "datetime")]
            ParsedValue::OffsetDatetime(datetime) => {
                visitor.visit_map(OffsetDatetimeAccess::new(datetime))
            }
            #[cfg(feature = "datetime")]
            ParsedValue::LocalDatetime(datetime) => {
                visitor.visit_map(LocalDatetimeAccess::new(datetime))
            }
            #[cfg(feature = "datetime")]
            ParsedValue::LocalDate(date) => visitor.visit_map(LocalDateAccess::new(date)),
            #[cfg(feature = "datetime")]
            ParsedValue::LocalTime(time) => visitor.visit_map(LocalTimeAccess::new(time)),
            ParsedValue::Array(array) => visitor.visit_seq(SeqAccess::new(array)),
            ParsedValue::ArrayOfTables(array) => visitor.visit_seq(SeqAccess::new(array)),
            ParsedValue::Table(table)
            | ParsedValue::UndefinedTable(table)
            | ParsedValue::InlineTable(table)
            | ParsedValue::DottedKeyTable(table) => visitor.visit_map(MapAccess::new(table)),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Boolean(bool) => visitor.visit_bool(bool),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_i8(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_i8(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_i8(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_i8(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_i16(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_i16(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_i16(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_i16(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_i32(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_i32(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_i32(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_i32(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_i64(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_i64(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_i64(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_i64(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_i128(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_i128(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_i128(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_i128(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_u8(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_u8(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_u8(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_u8(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_u16(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_u16(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_u16(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_u16(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_u32(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_u32(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_u32(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_u32(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_u64(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_u64(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_u64(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_u64(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Integer(bytes) => visitor.visit_u128(parse_integer(&bytes)?),
            ParsedValue::BinaryInt(bytes) => visitor.visit_u128(parse_binary(&bytes)?),
            ParsedValue::OctalInt(bytes) => visitor.visit_u128(parse_octal(&bytes)?),
            ParsedValue::HexInt(bytes) => visitor.visit_u128(parse_hexadecimal(&bytes)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Float(bytes) | ParsedValue::Integer(bytes) => {
                visitor.visit_f32(parse_float(&bytes)?)
            }
            ParsedValue::SpecialFloat(special) => visitor.visit_f32(parse_special(special)),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Float(bytes) | ParsedValue::Integer(bytes) => {
                visitor.visit_f64(parse_float(&bytes)?)
            }
            ParsedValue::SpecialFloat(special) => visitor.visit_f64(parse_special(special)),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(string) => visitor.visit_string(string),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(string) => visitor.visit_byte_buf(string.into_bytes()),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::invalid_type(self.value.typ().into(), &visitor))
    }

    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Array(array) => visitor.visit_seq(SeqAccess::new(array)),
            ParsedValue::ArrayOfTables(array) => visitor.visit_seq(SeqAccess::new(array)),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Table(table)
            | ParsedValue::UndefinedTable(table)
            | ParsedValue::DottedKeyTable(table)
            | ParsedValue::InlineTable(table) => visitor.visit_map(MapAccess::new(table)),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    // name and fields are only used when date-time is enabled
    #[cfg_attr(not(feature = "datetime"), allow(unused_variables))]
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            #[cfg(feature = "datetime")]
            ParsedValue::OffsetDatetime(datetime)
                if matches!(
                    name,
                    AnyDatetime::WRAPPER_TYPE | OffsetDatetime::WRAPPER_TYPE
                ) && matches!(
                    *fields,
                    [AnyDatetime::WRAPPER_FIELD | OffsetDatetime::WRAPPER_FIELD]
                ) =>
            {
                visitor.visit_map(OffsetDatetimeAccess::new(datetime))
            }
            #[cfg(feature = "datetime")]
            ParsedValue::LocalDatetime(datetime)
                if matches!(
                    name,
                    AnyDatetime::WRAPPER_TYPE | LocalDatetime::WRAPPER_TYPE
                ) && matches!(
                    *fields,
                    [AnyDatetime::WRAPPER_FIELD | LocalDatetime::WRAPPER_FIELD]
                ) =>
            {
                visitor.visit_map(LocalDatetimeAccess::new(datetime))
            }
            #[cfg(feature = "datetime")]
            ParsedValue::LocalDate(date)
                if matches!(name, AnyDatetime::WRAPPER_TYPE | LocalDate::WRAPPER_TYPE)
                    && matches!(
                        *fields,
                        [AnyDatetime::WRAPPER_FIELD | LocalDate::WRAPPER_FIELD]
                    ) =>
            {
                visitor.visit_map(LocalDateAccess::new(date))
            }
            #[cfg(feature = "datetime")]
            ParsedValue::LocalTime(time)
                if matches!(name, AnyDatetime::WRAPPER_TYPE | LocalTime::WRAPPER_TYPE)
                    && matches!(
                        *fields,
                        [AnyDatetime::WRAPPER_FIELD | LocalTime::WRAPPER_FIELD]
                    ) =>
            {
                visitor.visit_map(LocalTimeAccess::new(time))
            }
            ParsedValue::Table(table)
            | ParsedValue::UndefinedTable(table)
            | ParsedValue::DottedKeyTable(table)
            | ParsedValue::InlineTable(table) => visitor.visit_map(MapAccess::new(table)),
            value => Err(Error::invalid_type(value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(str) => visitor.visit_enum(str.into_deserializer()),
            ParsedValue::Table(table)
            | ParsedValue::UndefinedTable(table)
            | ParsedValue::DottedKeyTable(table)
            | ParsedValue::InlineTable(table) => visitor.visit_enum(EnumAccess::new(table)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    #[inline]
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    #[inline]
    fn deserialize_ignored_any<V>(self, visitor: V) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct SeqAccess<T> {
    values: <Vec<T> as IntoIterator>::IntoIter,
}

impl<T> SeqAccess<T> {
    #[inline]
    fn new(array: Vec<T>) -> Self {
        Self {
            values: array.into_iter(),
        }
    }
}

// For regular arrays
impl<'de> de::SeqAccess<'de> for SeqAccess<ParsedValue> {
    type Error = Error;

    #[inline]
    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| seed.deserialize(ValueDeserializer::new(value)))
            .transpose()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

// Used for array of tables
impl<'de> de::SeqAccess<'de> for SeqAccess<ParsedTable> {
    type Error = Error;

    #[inline]
    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| {
                seed.deserialize(de::value::MapAccessDeserializer::new(MapAccess::new(value)))
            })
            .transpose()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

struct MapAccess {
    kv_pairs: <ParsedTable as IntoIterator>::IntoIter,
    next_value: Option<ParsedValue>,
}

impl MapAccess {
    #[inline]
    fn new(table: ParsedTable) -> Self {
        Self {
            kv_pairs: table.into_iter(),
            next_value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess {
    type Error = Error;

    #[inline]
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.kv_pairs
            .next()
            .map(|(key, value)| {
                self.next_value = Some(value);
                seed.deserialize(key.into_deserializer())
            })
            .transpose()
    }

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic)]
        let Some(value) = self.next_value.take() else {
            panic!("MapAccess::next_value called without calling MapAccess::next_key first")
        };
        seed.deserialize(ValueDeserializer::new(value))
    }

    #[inline]
    fn next_entry_seed<K, V>(&mut self, kseed: K, vseed: V) -> Result<Option<(K::Value, V::Value)>>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        self.kv_pairs
            .next()
            .map(|(key, value)| {
                kseed.deserialize(key.into_deserializer()).and_then(|k| {
                    vseed
                        .deserialize(ValueDeserializer::new(value))
                        .map(|v| (k, v))
                })
            })
            .transpose()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.kv_pairs.len())
    }
}

#[derive(Debug)]
struct EnumAccess {
    variant: String,
    value: ParsedValue,
}

impl EnumAccess {
    fn new(table: ParsedTable) -> Result<Self> {
        let mut table = table.into_iter();
        let (variant, value) = table.next().ok_or_else(|| {
            Error::invalid_value(
                de::Unexpected::Other("empty table"),
                &"exactly one key/value pair",
            )
        })?;
        if table.next().is_some() {
            return Err(Error::invalid_value(
                de::Unexpected::Other("multiple entries in table"),
                &"exactly one key/value pair",
            ));
        }
        Ok(Self { variant, value })
    }
}

impl<'de> de::EnumAccess<'de> for EnumAccess {
    type Error = Error;
    type Variant = Self;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(StrDeserializer::<Error>::new(&self.variant))?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumAccess {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<()> {
        // We allow unit variants to be represented by `x = { variant = {} }` in addition to the
        // normal `x = "variant"`. toml-rs seems to do the same.
        match self.value {
            ParsedValue::Table(table) if table.is_empty() => Ok(()),
            _ => Err(Error::invalid_type(self.value.typ().into(), &"empty table")),
        }
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(ValueDeserializer::new(self.value))
    }

    #[inline]
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(ValueDeserializer::new(self.value), visitor)
    }

    #[inline]
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(ValueDeserializer::new(self.value), visitor)
    }
}

trait Integer: Sized {
    fn from_str_radix(src: &[u8], radix: u32) -> Result<Self>;

    fn from_str(src: &[u8]) -> Result<Self>;
}

macro_rules! impl_integer {
    ($($t:ident)*) => ($(
        impl Integer for $t {
            fn from_str_radix(bytes: &[u8], radix: u32) -> Result<Self> {
                let str = str::from_utf8(bytes)
                    .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
                <Self>::from_str_radix(str, radix)
                    .map_err(|err| $crate::de::ErrorKind::InvalidInteger(err).into())
            }

            fn from_str(bytes: &[u8]) -> Result<Self> {
                let str = str::from_utf8(bytes)
                    .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
                <Self as std::str::FromStr>::from_str(str)
                    .map_err(|err| $crate::de::ErrorKind::InvalidInteger(err).into())
            }
        }
    )*);
}

impl_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

trait Float: Sized {
    const INFINITY: Self;
    const NEG_INFINITY: Self;
    const NAN: Self;
    const NEG_NAN: Self;

    fn from_str(src: &[u8]) -> Result<Self>;
}

macro_rules! impl_float {
    ($($t:ident)*) => ($(impl Float for $t {
        const INFINITY: Self = Self::INFINITY;
        const NEG_INFINITY: Self = Self::NEG_INFINITY;
        const NAN: Self = Self::NAN;
        const NEG_NAN: Self = -Self::NAN;

        fn from_str(bytes: &[u8]) -> Result<Self> {
            let str = str::from_utf8(bytes)
                .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
            <Self as std::str::FromStr>::from_str(str)
                .map_err(|err| $crate::de::ErrorKind::InvalidFloat(err).into())
        }
    })*);
}

impl_float!(f32 f64);

#[inline]
fn parse_integer<T: Integer>(bytes: &[u8]) -> Result<T> {
    T::from_str(bytes)
}

#[inline]
fn parse_binary<T: Integer>(bytes: &[u8]) -> Result<T> {
    T::from_str_radix(bytes, 2)
}

#[inline]
fn parse_octal<T: Integer>(bytes: &[u8]) -> Result<T> {
    T::from_str_radix(bytes, 8)
}

#[inline]
fn parse_hexadecimal<T: Integer>(bytes: &[u8]) -> Result<T> {
    T::from_str_radix(bytes, 16)
}

#[inline]
fn parse_float<T: Float>(bytes: &[u8]) -> Result<T> {
    T::from_str(bytes)
}

#[inline]
const fn parse_special<T: Float>(special: SpecialFloat) -> T {
    match special {
        SpecialFloat::Nan => T::NAN,
        SpecialFloat::Infinity => T::INFINITY,
        SpecialFloat::NegNan => T::NEG_NAN,
        SpecialFloat::NegInfinity => T::NEG_INFINITY,
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use std::collections::HashMap;
    use std::{fmt, iter};

    use assert_matches::assert_matches;
    use indoc::indoc;
    use maplit::{btreemap, hashmap};
    use serde::de::{EnumAccess as _, MapAccess as _, SeqAccess as _, VariantAccess as _};
    use serde_bytes::ByteBuf;

    use super::*;
    #[cfg(feature = "datetime")]
    use crate::value::{Datetime, Offset};
    use crate::Value;

    mod example {
        use std::collections::HashMap;

        #[cfg(feature = "datetime")]
        use crate::value::OffsetDatetime;

        #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
        pub struct Struct {
            pub title: String,
            pub owner: Owner,
            pub database: Database,
            pub servers: HashMap<String, Server>,
            pub clients: Clients,
        }

        #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
        pub struct Owner {
            pub name: String,
            #[cfg(feature = "datetime")]
            pub dob: OffsetDatetime,
        }

        #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
        pub struct Database {
            pub server: String,
            pub ports: Vec<u16>,
            pub connection_max: usize,
            pub enabled: bool,
        }

        #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
        pub struct Server {
            pub ip: String,
            pub dc: String,
        }

        #[derive(Debug, PartialEq, Eq, serde::Deserialize)]
        pub struct Clients {
            pub hosts: Vec<String>,
            pub data: HashMap<String, usize>,
        }
    }

    #[test]
    fn test_from_str() {
        let result: example::Struct = from_str(
            &[
                indoc! {r#"
                    # This is a TOML document.

                    title = "TOML Example"

                    [owner]
                    name = "Tom Preston-Werner"
                "#},
                if cfg!(feature = "datetime") {
                    "dob = 1979-05-27T07:32:00-08:00 # First class dates\n"
                } else {
                    ""
                },
                indoc! {r#"
                    [database]
                    server = "192.168.1.1"
                    ports = [ 8000, 8001, 8002 ]
                    connection_max = 5000
                    enabled = true

                    [servers]

                    # Indentation (tabs and/or spaces) is allowed but not required
                    [servers.alpha]
                    ip = "10.0.0.1"
                    dc = "eqdc10"

                    [servers.beta]
                    ip = "10.0.0.2"
                    dc = "eqdc10"

                    [clients]
                    data = { "gamma" = 1, "delta" = 2 }

                    # Line breaks are OK when inside arrays
                    hosts = [
                    "alpha",
                    "omega"
                    ]
                "#},
            ]
            .join(""),
        )
        .unwrap();

        assert_eq!(
            result,
            example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
                    #[cfg(feature = "datetime")]
                    dob: OffsetDatetime {
                        date: LocalDate {
                            year: 1979,
                            month: 5,
                            day: 27,
                        },
                        time: LocalTime {
                            hour: 7,
                            minute: 32,
                            second: 0,
                            nanosecond: 0,
                        },
                        offset: Offset::Custom { minutes: -480 }
                    },
                },
                database: example::Database {
                    server: "192.168.1.1".into(),
                    ports: vec![8000, 8001, 8002],
                    connection_max: 5000,
                    enabled: true,
                },
                servers: hashmap! {
                    "alpha".into() => example::Server {
                        ip: "10.0.0.1".into(),
                        dc: "eqdc10".into(),
                    },
                    "beta".into() => example::Server {
                        ip: "10.0.0.2".into(),
                        dc: "eqdc10".into(),
                    },
                },
                clients: example::Clients {
                    hosts: vec!["alpha".into(), "omega".into()],
                    data: hashmap! {
                        "gamma".into() => 1,
                        "delta".into() => 2,
                    },
                },
            }
        );
    }

    #[test]
    fn test_from_slice() {
        let result: example::Struct = from_slice(
            [
                indoc! {r#"
                    # This is a TOML document.

                    title = "TOML Example"

                    [owner]
                    name = "Tom Preston-Werner"
                "#},
                if cfg!(feature = "datetime") {
                    "dob = 1979-05-27T07:32:00-08:00 # First class dates\n"
                } else {
                    ""
                },
                indoc! {r#"
                    [database]
                    server = "192.168.1.1"
                    ports = [ 8000, 8001, 8002 ]
                    connection_max = 5000
                    enabled = true

                    [servers]

                    # Indentation (tabs and/or spaces) is allowed but not required
                    [servers.alpha]
                    ip = "10.0.0.1"
                    dc = "eqdc10"

                    [servers.beta]
                    ip = "10.0.0.2"
                    dc = "eqdc10"

                    [clients]
                    data = { "gamma" = 1, "delta" = 2 }

                    # Line breaks are OK when inside arrays
                    hosts = [
                    "alpha",
                    "omega"
                    ]
                "#},
            ]
            .join("")
            .as_bytes(),
        )
        .unwrap();

        assert_eq!(
            result,
            example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
                    #[cfg(feature = "datetime")]
                    dob: OffsetDatetime {
                        date: LocalDate {
                            year: 1979,
                            month: 5,
                            day: 27,
                        },
                        time: LocalTime {
                            hour: 7,
                            minute: 32,
                            second: 0,
                            nanosecond: 0,
                        },
                        offset: Offset::Custom { minutes: -480 }
                    },
                },
                database: example::Database {
                    server: "192.168.1.1".into(),
                    ports: vec![8000, 8001, 8002],
                    connection_max: 5000,
                    enabled: true,
                },
                servers: hashmap! {
                    "alpha".into() => example::Server {
                        ip: "10.0.0.1".into(),
                        dc: "eqdc10".into(),
                    },
                    "beta".into() => example::Server {
                        ip: "10.0.0.2".into(),
                        dc: "eqdc10".into(),
                    },
                },
                clients: example::Clients {
                    hosts: vec!["alpha".into(), "omega".into()],
                    data: hashmap! {
                        "gamma".into() => 1,
                        "delta".into() => 2,
                    },
                },
            }
        );
    }

    #[test]
    fn test_from_reader() {
        let result: example::Struct = from_reader(
            [
                indoc! {r#"
                    # This is a TOML document.

                    title = "TOML Example"

                    [owner]
                    name = "Tom Preston-Werner"
                "#},
                if cfg!(feature = "datetime") {
                    "dob = 1979-05-27T07:32:00-08:00 # First class dates\n"
                } else {
                    ""
                },
                indoc! {r#"
                    [database]
                    server = "192.168.1.1"
                    ports = [ 8000, 8001, 8002 ]
                    connection_max = 5000
                    enabled = true

                    [servers]

                    # Indentation (tabs and/or spaces) is allowed but not required
                    [servers.alpha]
                    ip = "10.0.0.1"
                    dc = "eqdc10"

                    [servers.beta]
                    ip = "10.0.0.2"
                    dc = "eqdc10"

                    [clients]
                    data = { "gamma" = 1, "delta" = 2 }

                    # Line breaks are OK when inside arrays
                    hosts = [
                    "alpha",
                    "omega"
                    ]
                "#},
            ]
            .join("")
            .as_bytes(),
        )
        .unwrap();

        assert_eq!(
            result,
            example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
                    #[cfg(feature = "datetime")]
                    dob: OffsetDatetime {
                        date: LocalDate {
                            year: 1979,
                            month: 5,
                            day: 27,
                        },
                        time: LocalTime {
                            hour: 7,
                            minute: 32,
                            second: 0,
                            nanosecond: 0,
                        },
                        offset: Offset::Custom { minutes: -480 }
                    },
                },
                database: example::Database {
                    server: "192.168.1.1".into(),
                    ports: vec![8000, 8001, 8002],
                    connection_max: 5000,
                    enabled: true,
                },
                servers: hashmap! {
                    "alpha".into() => example::Server {
                        ip: "10.0.0.1".into(),
                        dc: "eqdc10".into(),
                    },
                    "beta".into() => example::Server {
                        ip: "10.0.0.2".into(),
                        dc: "eqdc10".into(),
                    },
                },
                clients: example::Clients {
                    hosts: vec!["alpha".into(), "omega".into()],
                    data: hashmap! {
                        "gamma".into() => 1,
                        "delta".into() => 2,
                    },
                },
            }
        );
    }

    #[test]
    fn deserializer_from_str() {
        let mut deserializer = Deserializer::from_str("abc = 123");

        assert_matches!(
            deserializer.parser.parse(),
            Ok(ParsedValue::Table(t)) if t == hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".to_vec())
            }
        );
    }

    #[test]
    fn deserializer_from_slice() {
        let mut deserializer = Deserializer::from_slice(b"abc = 123");

        assert_matches!(
            deserializer.parser.parse(),
            Ok(ParsedValue::Table(t)) if t == hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            }
        );
    }

    #[test]
    fn deserializer_deserialize_any() {
        let deserializer = Deserializer::from_str("abc = 123");

        assert_matches!(
            HashMap::deserialize(deserializer),
            Ok(t) if t == hashmap! {
                "abc".to_owned() => 123_i32,
            }
        );
    }

    #[test]
    fn value_deserializer_new() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));

        assert_eq!(deserializer.value, ParsedValue::String("hello".into()));
    }

    #[test]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn value_deserializer_deserialize_any() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::String(s)) if &*s == "hello");

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Integer(123)));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Integer(10)));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Integer(83)));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"123".to_vec()));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Integer(291)));

        let deserializer = ValueDeserializer::new(ParsedValue::Float(b"123.0".to_vec()));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Float(123.0)));

        let deserializer =
            ValueDeserializer::new(ParsedValue::SpecialFloat(SpecialFloat::Infinity));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Float(v)) if v.is_infinite());

        let deserializer = ValueDeserializer::new(ParsedValue::Boolean(true));
        assert_matches!(Value::deserialize(deserializer), Ok(Value::Boolean(true)));

        #[cfg(feature = "datetime")]
        {
            let deserializer = ValueDeserializer::new(ParsedValue::OffsetDatetime(
                OffsetDatetime::EXAMPLE_BYTES.to_vec(),
            ));
            assert_matches!(
                Value::deserialize(deserializer),
                Ok(Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME))
            );

            let deserializer = ValueDeserializer::new(ParsedValue::LocalDatetime(
                LocalDatetime::EXAMPLE_BYTES.to_vec(),
            ));
            assert_matches!(
                Value::deserialize(deserializer),
                Ok(Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME))
            );

            let deserializer =
                ValueDeserializer::new(ParsedValue::LocalDate(LocalDate::EXAMPLE_BYTES.to_vec()));
            assert_matches!(
                Value::deserialize(deserializer),
                Ok(Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE))
            );

            let deserializer =
                ValueDeserializer::new(ParsedValue::LocalTime(LocalTime::EXAMPLE_BYTES.to_vec()));
            assert_matches!(
                Value::deserialize(deserializer),
                Ok(Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME))
            );
        };

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".to_vec()),
            ParsedValue::Integer(b"456".to_vec()),
            ParsedValue::Integer(b"789".to_vec()),
        ]));
        assert_matches!(
            Value::deserialize(deserializer),
            Ok(Value::Array(a)) if a == vec![
                Value::Integer(123),
                Value::Integer(456),
                Value::Integer(789),
            ]
        );

        let deserializer = ValueDeserializer::new(ParsedValue::ArrayOfTables(vec![
            hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            },
            hashmap! {
                "def".into() => ParsedValue::Integer(b"456".to_vec()),
            },
            hashmap! {
                "ghi".into() => ParsedValue::Integer(b"789".to_vec()),
            },
        ]));
        assert_matches!(
            Value::deserialize(deserializer),
            Ok(Value::Array(a)) if a == [
                Value::Table(btreemap! {
                    "abc".into() => Value::Integer(123),
                }),
                Value::Table(btreemap! {
                    "def".into() => Value::Integer(456),
                }),
                Value::Table(btreemap! {
                    "ghi".into() => Value::Integer(789),
                }),
            ]
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            Value::deserialize(deserializer),
            Ok(Value::Table(t)) if t == btreemap! {
                "abc".into() => Value::Integer(123),
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            Value::deserialize(deserializer),
            Ok(Value::Table(t)) if t == btreemap! {
                "abc".into() => Value::Integer(123),
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            Value::deserialize(deserializer),
            Ok(Value::Table(t)) if t == btreemap! {
                "abc".into() => Value::Integer(123),
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            Value::deserialize(deserializer),
            Ok(Value::Table(t)) if t == btreemap! {
                "abc".into() => Value::Integer(123),
            }
        );
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn value_deserializer_deserialize_bool() {
        let deserializer = ValueDeserializer::new(ParsedValue::Boolean(true));
        assert_matches!(bool::deserialize(deserializer), Ok(true));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            bool::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_i8() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(i8::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(i8::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(i8::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(i8::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            i8::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_i16() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(i16::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(i16::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(i16::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(i16::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            i16::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_i32() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(i32::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(i32::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(i32::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(i32::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            i32::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_i64() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(i64::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(i64::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(i64::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(i64::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            i64::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_i128() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(i128::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(i128::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(i128::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(i128::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            i128::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_u8() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(u8::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(u8::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(u8::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(u8::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            u8::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_u16() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(u16::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(u16::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(u16::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(u16::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            u16::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_u32() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(u32::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(u32::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(u32::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(u32::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            u32::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_u64() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(u64::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(u64::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(u64::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(u64::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            u64::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_u128() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(u128::deserialize(deserializer), Ok(123));

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".to_vec()));
        assert_matches!(u128::deserialize(deserializer), Ok(10));

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".to_vec()));
        assert_matches!(u128::deserialize(deserializer), Ok(83));

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".to_vec()));
        assert_matches!(u128::deserialize(deserializer), Ok(90));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            u128::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_f32() {
        let deserializer = ValueDeserializer::new(ParsedValue::Float(b"123.0".to_vec()));
        assert_matches!(f32::deserialize(deserializer), Ok(123.0));

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(f32::deserialize(deserializer), Ok(123.0));

        let deserializer =
            ValueDeserializer::new(ParsedValue::SpecialFloat(SpecialFloat::Infinity));
        assert_matches!(f32::deserialize(deserializer), Ok(f) if f.is_infinite());

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            f32::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_f64() {
        let deserializer = ValueDeserializer::new(ParsedValue::Float(b"123.0".to_vec()));
        assert_matches!(f64::deserialize(deserializer), Ok(123.0));

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(f64::deserialize(deserializer), Ok(123.0));

        let deserializer =
            ValueDeserializer::new(ParsedValue::SpecialFloat(SpecialFloat::Infinity));
        assert_matches!(f64::deserialize(deserializer), Ok(f) if f.is_infinite());

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            f64::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_char() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("A".to_string()));
        assert_matches!(char::deserialize(deserializer), Ok('A'));

        let deserializer = ValueDeserializer::new(ParsedValue::String("A".to_string()));
        assert_matches!(char::deserialize(deserializer), Ok('A'));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            char::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(
            char::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_str() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".to_string()));
        // Can't actually deserialize to a borrowed &str
        assert_matches!(
            <&str>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(
            <&str>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_string() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".to_string()));
        assert_matches!(String::deserialize(deserializer), Ok(s) if s == "hello");

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(
            String::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_bytes() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".to_string()));
        // Can't actually deserialize to a borrowed &[u8]
        assert_matches!(
            <&[u8]>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(
            <&[u8]>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_byte_buf() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".to_string()));
        assert_matches!(ByteBuf::deserialize(deserializer), Ok(b) if &*b == b"hello");

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(
            ByteBuf::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_option() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".to_string()));
        assert_matches!(Option::<String>::deserialize(deserializer), Ok(Some(s)) if s == "hello");
    }

    #[test]
    fn value_deserializer_deserialize_unit() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".to_string()));
        assert_matches!(
            <()>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_unit_struct() {
        #[derive(Debug, Deserialize)]
        struct Unit;

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            Unit::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_newtype_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Newtype(i32);

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(Newtype::deserialize(deserializer), Ok(Newtype(123)));
    }

    #[test]
    fn value_deserializer_deserialize_seq() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Struct {
            val: i32,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".to_vec()),
            ParsedValue::Integer(b"456".to_vec()),
            ParsedValue::Integer(b"789".to_vec()),
        ]));
        assert_matches!(
            <Vec<i32>>::deserialize(deserializer),
            Ok(a) if a == [123, 456, 789]
        );

        let deserializer = ValueDeserializer::new(ParsedValue::ArrayOfTables(vec![
            hashmap! { "val".into() => ParsedValue::Integer(b"123".to_vec()) },
            hashmap! { "val".into() => ParsedValue::Integer(b"456".to_vec()) },
            hashmap! { "val".into() => ParsedValue::Integer(b"789".to_vec()) },
        ]));
        assert_matches!(
            <Vec<Struct>>::deserialize(deserializer),
            Ok(a) if a == [
                Struct { val: 123 },
                Struct { val: 456 },
                Struct { val: 789 },
            ]
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            <Vec<i32>>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_tuple() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Struct<T> {
            val: T,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".to_vec()),
            ParsedValue::String("hello".into()),
            ParsedValue::Array(vec![]),
        ]));
        assert_matches!(
            <(i32, String, Vec<i64>)>::deserialize(deserializer),
            Ok((i, s, d)) if i == 123 && s == "hello" && d.is_empty()
        );

        let deserializer = ValueDeserializer::new(ParsedValue::ArrayOfTables(vec![
            hashmap! { "val".into() => ParsedValue::Integer(b"123".to_vec()) },
            hashmap! { "val".into() => ParsedValue::String("hello".into()) },
            hashmap! { "val".into() => ParsedValue::Array(vec![]) },
        ]));
        assert_matches!(
            <(Struct<i32>, Struct<String>, Struct<Vec<i64>>)>::deserialize(deserializer),
            Ok((a, b, c)) if a == Struct { val: 123 }
                && b == Struct { val: "hello".into() }
                && c == Struct {
                    val: vec![]
                }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            <(i32, i32, i32)>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_tuple_struct_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct TupleStruct(i32, String, Vec<i64>);

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".to_vec()),
            ParsedValue::String("hello".into()),
            ParsedValue::Array(vec![]),
        ]));
        assert_matches!(
            TupleStruct::deserialize(deserializer),
            Ok(TupleStruct(i, s, d)) if i == 123
                && s == "hello"
                && d.is_empty()
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            TupleStruct::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_map() {
        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            HashMap::<String, i32>::deserialize(deserializer),
            Ok(m) if m == hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            HashMap::<String, i32>::deserialize(deserializer),
            Ok(m) if m == hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            HashMap::<String, i32>::deserialize(deserializer),
            Ok(m) if m == hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            HashMap::<String, i32>::deserialize(deserializer),
            Ok(m) if m == hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            HashMap::<String, i32>::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Struct {
            abc: i32,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Struct::deserialize(deserializer), Ok(Struct { abc: 123 }));

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Struct::deserialize(deserializer), Ok(Struct { abc: 123 }));

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Struct::deserialize(deserializer), Ok(Struct { abc: 123 }));

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Struct::deserialize(deserializer), Ok(Struct { abc: 123 }));

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_matches!(
            Struct::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[cfg(feature = "datetime")]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn value_deserializer_deserialize_struct_datetime() {
        let deserializer = ValueDeserializer::new(ParsedValue::OffsetDatetime(
            OffsetDatetime::EXAMPLE_BYTES.to_vec(),
        ));
        assert_matches!(
            OffsetDatetime::deserialize(deserializer),
            Ok(OffsetDatetime::EXAMPLE)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalDatetime(
            LocalDatetime::EXAMPLE_BYTES.to_vec(),
        ));
        assert_matches!(
            LocalDatetime::deserialize(deserializer),
            Ok(LocalDatetime::EXAMPLE)
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalDate(LocalDate::EXAMPLE_BYTES.to_vec()));
        assert_matches!(LocalDate::deserialize(deserializer), Ok(LocalDate::EXAMPLE));

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalTime(LocalTime::EXAMPLE_BYTES.to_vec()));
        assert_matches!(LocalTime::deserialize(deserializer), Ok(LocalTime::EXAMPLE));

        let deserializer = ValueDeserializer::new(ParsedValue::OffsetDatetime(
            OffsetDatetime::EXAMPLE_BYTES.to_vec(),
        ));
        assert_matches!(
            Datetime::deserialize(deserializer),
            Ok(Datetime::EXAMPLE_OFFSET_DATETIME)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalDatetime(
            LocalDatetime::EXAMPLE_BYTES.to_vec(),
        ));
        assert_matches!(
            Datetime::deserialize(deserializer),
            Ok(Datetime::EXAMPLE_LOCAL_DATETIME)
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalDate(LocalDate::EXAMPLE_BYTES.to_vec()));
        assert_matches!(
            Datetime::deserialize(deserializer),
            Ok(Datetime::EXAMPLE_LOCAL_DATE)
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalTime(LocalTime::EXAMPLE_BYTES.to_vec()));
        assert_matches!(
            Datetime::deserialize(deserializer),
            Ok(Datetime::EXAMPLE_LOCAL_TIME)
        );
    }

    #[test]
    fn value_deserializer_deserialize_enum() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        enum Enum {
            VariantA,
            VariantB(i32),
            VariantC { a: i32, b: i32 },
        }

        let deserializer = ValueDeserializer::new(ParsedValue::String("VariantA".into()));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantA));

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantB(123)));

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantB(123)));

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantB(123)));

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantB(123)));

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        }));
        assert_matches!(
            Enum::deserialize(deserializer),
            Ok(Enum::VariantC { a: 123, b: 456 })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        }));
        assert_matches!(
            Enum::deserialize(deserializer),
            Ok(Enum::VariantC { a: 123, b: 456 })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        }));
        assert_matches!(
            Enum::deserialize(deserializer),
            Ok(Enum::VariantC { a: 123, b: 456 })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        }));
        assert_matches!(
            Enum::deserialize(deserializer),
            Ok(Enum::VariantC { a: 123, b: 456 })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantA".into() => ParsedValue::Integer(b"123".to_vec()),
        }));
        assert_matches!(
            Enum::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantB".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        }));
        assert_matches!(
            Enum::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("VariantC".into()));
        assert_matches!(
            Enum::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".to_vec()));
        assert_matches!(
            Enum::deserialize(deserializer),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn value_deserializer_deserialize_identifier() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        #[serde(variant_identifier)]
        enum Enum {
            VariantA,
            VariantB,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::String("VariantA".into()));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantA));

        let deserializer = ValueDeserializer::new(ParsedValue::String("VariantB".into()));
        assert_matches!(Enum::deserialize(deserializer), Ok(Enum::VariantB));
    }

    #[test]
    fn value_deserializer_deserialize_ignored_any() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert!(de::IgnoredAny::deserialize(deserializer).is_ok());
    }

    #[test]
    fn seq_access_new() {
        let array = vec!["123", "456", "789"];
        let seq = SeqAccess::new(array.clone());

        assert_eq!(seq.values.collect::<Vec<_>>(), array);
    }

    #[test]
    fn seq_access_next_element() {
        let array = vec![
            ParsedValue::Integer(b"123".to_vec()),
            ParsedValue::Integer(b"456".to_vec()),
            ParsedValue::Integer(b"789".to_vec()),
        ];
        let mut seq = SeqAccess::new(array);

        assert_matches!(seq.next_element(), Ok(Some(123)));
        assert_matches!(seq.next_element(), Ok(Some(456)));
        assert_matches!(seq.next_element(), Ok(Some(789)));
        assert_matches!(seq.next_element(), Ok(None::<i32>));

        let array = vec![
            hashmap! { "abc".into() => ParsedValue::Integer(b"123".to_vec()) },
            hashmap! { "def".into() => ParsedValue::Integer(b"456".to_vec()) },
            hashmap! { "ghi".into() => ParsedValue::Integer(b"789".to_vec()) },
        ];
        let mut seq = SeqAccess::new(array);

        assert_matches!(
            seq.next_element::<HashMap<String, i32>>(),
            Ok(Some(m)) if m == hashmap! { "abc".to_owned() => 123 }
        );
        assert_matches!(
            seq.next_element::<HashMap<String, i32>>(),
            Ok(Some(m)) if m == hashmap! { "def".to_owned() => 456 }
        );
        assert_matches!(
            seq.next_element::<HashMap<String, i32>>(),
            Ok(Some(m)) if m == hashmap! { "ghi".to_owned() => 789 }
        );
        assert_matches!(seq.next_element::<HashMap<String, i32>>(), Ok(None));
    }

    #[test]
    fn seq_access_size_hint() {
        let array = vec![
            ParsedValue::Integer(b"123".to_vec()),
            ParsedValue::Integer(b"456".to_vec()),
            ParsedValue::Integer(b"789".to_vec()),
        ];
        let seq = SeqAccess::new(array);

        assert_eq!(seq.size_hint(), Some(3));

        let array = vec![
            hashmap! { "abc".into() => ParsedValue::Integer(b"123".to_vec()) },
            hashmap! { "def".into() => ParsedValue::Integer(b"456".to_vec()) },
            hashmap! { "ghi".into() => ParsedValue::Integer(b"789".to_vec()) },
        ];
        let seq = SeqAccess::new(array);

        assert_eq!(seq.size_hint(), Some(3));
    }

    #[test]
    fn map_access_new() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            "def".into() => ParsedValue::Integer(b"456".to_vec()),
            "ghi".into() => ParsedValue::Integer(b"789".to_vec()),
        };
        let map = MapAccess::new(table.clone());

        assert_eq!(
            map.kv_pairs.collect::<Vec<_>>(),
            table.into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn map_access_next_key_value() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            "def".into() => ParsedValue::Integer(b"456".to_vec()),
            "ghi".into() => ParsedValue::Integer(b"789".to_vec()),
        };
        let mut map = MapAccess::new(table);

        let mut entries: Vec<(String, i32)> = iter::from_fn(|| {
            map.next_key().unwrap().map(|key| {
                let value = map.next_value().unwrap();
                (key, value)
            })
        })
        .collect();
        entries.sort();

        assert_eq!(
            entries,
            vec![
                ("abc".to_owned(), 123),
                ("def".to_owned(), 456),
                ("ghi".to_owned(), 789)
            ]
        );

        assert_matches!(map.next_key::<String>(), Ok(None));
    }

    #[test]
    fn map_access_next_entry() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            "def".into() => ParsedValue::Integer(b"456".to_vec()),
            "ghi".into() => ParsedValue::Integer(b"789".to_vec()),
        };
        let mut map = MapAccess::new(table);

        let mut entries: Vec<(String, i32)> = iter::from_fn(|| map.next_entry().unwrap()).collect();
        entries.sort();

        assert_eq!(
            entries,
            vec![
                ("abc".to_owned(), 123),
                ("def".to_owned(), 456),
                ("ghi".to_owned(), 789)
            ]
        );

        assert_matches!(map.next_key::<String>(), Ok(None));
    }

    #[test]
    fn map_access_size_hint() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            "def".into() => ParsedValue::Integer(b"456".to_vec()),
            "ghi".into() => ParsedValue::Integer(b"789".to_vec()),
        };
        let map = MapAccess::new(table);
        assert_eq!(map.size_hint(), Some(3));
    }

    #[test]
    fn enum_access_new() {
        let table = hashmap! {
            "Variant".into() => ParsedValue::Integer(b"123".to_vec()),
        };
        let enum_ = EnumAccess::new(table).unwrap();
        assert_eq!(enum_.variant, "Variant");
        assert_eq!(enum_.value, ParsedValue::Integer(b"123".to_vec()));

        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".to_vec()),
            "def".into() => ParsedValue::Integer(b"456".to_vec()),
            "ghi".into() => ParsedValue::Integer(b"789".to_vec()),
        };
        assert_matches!(
            EnumAccess::new(table),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let table = hashmap! {};
        assert_matches!(
            EnumAccess::new(table),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn enum_access_variant_seed() {
        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(access.variant::<String>(), Ok((v, _)) if v == "VariantA");

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(access.variant::<String>(), Ok((v, _)) if v == "VariantB");

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(access.variant::<String>(), Ok((v, _)) if v == "VariantC");
    }

    #[test]
    fn enum_access_unit_variant() {
        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        access.unit_variant().unwrap();

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.unit_variant(),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".to_vec()),
                ParsedValue::Integer(b"456".to_vec()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.unit_variant(),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.unit_variant(),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn enum_access_newtype_variant() {
        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.newtype_variant::<i32>(),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(access.newtype_variant::<i32>(), Ok(123));

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".to_vec()),
                ParsedValue::Integer(b"456".to_vec()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.newtype_variant::<i32>(),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.newtype_variant::<i32>(),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn enum_access_tuple_variant() {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Vec<i32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a tuple")
            }

            fn visit_seq<A>(self, mut seq: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(value) = seq.next_element::<i32>()? {
                    result.push(value);
                }
                Ok(result)
            }
        }

        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.tuple_variant(0, Visitor),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.tuple_variant(1, Visitor),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".to_vec()),
                ParsedValue::Integer(b"456".to_vec()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(access.tuple_variant(2, Visitor), Ok(s) if s == [123, 456]);

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.tuple_variant(2, Visitor),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[test]
    fn enum_access_struct_variant() {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = HashMap<String, i32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a table")
            }

            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut result = HashMap::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<String, i32>()? {
                    result.insert(key, value);
                }
                Ok(result)
            }
        }

        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(access.struct_variant(&[], Visitor), Ok(m) if m.is_empty());

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".to_vec()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.struct_variant(&["a"], Visitor),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".to_vec()),
                ParsedValue::Integer(b"456".to_vec()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.struct_variant(&["a", "b"], Visitor),
            Err(Error(ErrorKind::InvalidType(..)))
        );

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".to_vec()),
                "b".into() => ParsedValue::Integer(b"456".to_vec()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_matches!(
            access.struct_variant(&["a", "b"], Visitor),
            Ok(m) if m == hashmap! {
                "a".into() => 123,
                "b".into() => 456,
            }
        );
    }

    #[test]
    fn test_parse_integer() {
        let bytes = b"123";
        assert_matches!(parse_integer::<i32>(bytes), Ok(123));

        let bytes = b"0123"; // Leading zeros are handled in the parser
        assert_matches!(parse_integer::<i32>(bytes), Ok(123));

        let bytes = b"1_2_3"; // Underscores are stripped in the parser
        assert_matches!(
            parse_integer::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );

        let bytes = b"123.0";
        assert_matches!(
            parse_integer::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );
    }

    #[test]
    fn test_parse_binary() {
        let bytes = b"1010";
        assert_matches!(parse_binary::<i32>(bytes), Ok(10));

        let bytes = b"01010"; // Leading zeros are ok because we already have a leading 0b
        assert_matches!(parse_binary::<i32>(bytes), Ok(10));

        let bytes = b"1_0_1_0"; // Underscores are stripped in the parser
        assert_matches!(
            parse_binary::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );

        let bytes = b"1010.0";
        assert_matches!(
            parse_binary::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );
    }

    #[test]
    fn test_parse_octal() {
        let bytes = b"123";
        assert_matches!(parse_octal::<i32>(bytes), Ok(83));

        let bytes = b"0123"; // Leading zeros are ok because we already have a leading 0o
        assert_matches!(parse_octal::<i32>(bytes), Ok(83));

        let bytes = b"1_2_3"; // Underscores are stripped in the parser
        assert_matches!(
            parse_octal::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );

        let bytes = b"123.0";
        assert_matches!(
            parse_octal::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );
    }

    #[test]
    fn test_parse_hexadecimal() {
        let bytes = b"123";
        assert_matches!(parse_hexadecimal::<i32>(bytes), Ok(291));

        let bytes = b"0123"; // Leading zeros are ok because we already have a leading 0x
        assert_matches!(parse_hexadecimal::<i32>(bytes), Ok(291));

        let bytes = b"1_2_3"; // Underscores are stripped in the parser
        assert_matches!(
            parse_hexadecimal::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );

        let bytes = b"123.0";
        assert_matches!(
            parse_hexadecimal::<i32>(bytes),
            Err(Error(ErrorKind::InvalidInteger(..)))
        );
    }

    #[test]
    fn test_parse_float() {
        let bytes = b"123.456";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.456));

        let bytes = b"123e0";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.0));

        let bytes = b"123e+0";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.0));

        let bytes = b"123e-0";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.0));

        let bytes = b"123.456e123";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.456e123));

        let bytes = b"123.456e+123";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.456e123));

        let bytes = b"123.456e-123";
        assert_matches!(parse_float::<f64>(bytes), Ok(123.456e-123));

        let bytes = b"1_2_3.4_5_6";
        assert_matches!(
            parse_float::<f64>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );

        let bytes = b"1_2_3e1_2_3";
        assert_matches!(
            parse_float::<f64>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );

        let bytes = b"1_2_3e+1_2_3";
        assert_matches!(
            parse_float::<f64>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );

        let bytes = b"1_2_3e-1_2_3";
        assert_matches!(
            parse_float::<f64>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );

        let bytes = b"123.0_";
        assert_matches!(
            parse_float::<f32>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );

        let bytes = b"_123.0";
        assert_matches!(
            parse_float::<f32>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );

        let bytes = b"123.0.0";
        assert_matches!(
            parse_float::<f32>(bytes),
            Err(Error(ErrorKind::InvalidFloat(..)))
        );
    }

    #[test]
    fn test_parse_special() {
        assert!(parse_special::<f64>(SpecialFloat::Infinity).is_infinite());
        assert!(parse_special::<f64>(SpecialFloat::Infinity).is_sign_positive());

        assert!(parse_special::<f64>(SpecialFloat::Nan).is_nan());
        assert!(parse_special::<f64>(SpecialFloat::Nan).is_sign_positive());

        assert!(parse_special::<f64>(SpecialFloat::NegInfinity).is_infinite());
        assert!(parse_special::<f64>(SpecialFloat::NegInfinity).is_sign_negative());

        assert!(parse_special::<f64>(SpecialFloat::NegNan).is_nan());
        assert!(parse_special::<f64>(SpecialFloat::NegNan).is_sign_negative());
    }
}
