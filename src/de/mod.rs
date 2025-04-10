use std::borrow::Cow;
use std::io;
use std::num::NonZero;
use std::result::Result as StdResult;

use lexical::{
    FromLexicalWithOptions, NumberFormatBuilder, ParseFloatOptions, ParseIntegerOptions,
};
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeOwned, Error as _, IntoDeserializer as _};
use serde::{de, Deserialize};

pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, Result};
use self::parser::{Parser, SpecialFloat, Table as ParsedTable, Value as ParsedValue};
use self::reader::{IoReader, Reader, SliceReader};
use crate::value::datetime::DatetimeAccess;
use crate::value::{Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

mod error;
mod parser;
mod reader;

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    T::deserialize(Deserializer::from_str(s))
}

pub fn from_slice<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    T::deserialize(Deserializer::from_slice(bytes))
}

pub fn from_reader<R, T>(read: R) -> Result<T>
where
    R: io::Read,
    T: DeserializeOwned,
{
    T::deserialize(Deserializer::from_reader(read))
}

#[derive(Debug)]
pub struct Deserializer<'de, R>
where
    R: Reader<'de>,
{
    parser: Parser<'de, R>,
}

impl<'de> Deserializer<'de, SliceReader<'de>> {
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(str: &'de str) -> Self {
        Self {
            parser: Parser::from_str(str),
        }
    }

    #[must_use]
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        Self {
            parser: Parser::from_slice(bytes),
        }
    }
}

impl<R> Deserializer<'_, IoReader<R>>
where
    R: io::Read,
{
    #[must_use]
    pub fn from_reader(read: R) -> Self {
        Self {
            parser: Parser::from_reader(read),
        }
    }
}

impl<'de, R> de::Deserializer<'de> for Deserializer<'de, R>
where
    R: Reader<'de>,
{
    type Error = Error;

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
struct ValueDeserializer<'de> {
    value: ParsedValue<'de>,
}

impl<'de> ValueDeserializer<'de> {
    const fn new(value: ParsedValue<'de>) -> Self {
        Self { value }
    }
}

impl<'de> de::Deserializer<'de> for ValueDeserializer<'de> {
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
            ParsedValue::OffsetDatetime(bytes) => {
                visitor.visit_map(DatetimeAccess::offset_datetime(bytes))
            }
            ParsedValue::LocalDatetime(bytes) => {
                visitor.visit_map(DatetimeAccess::local_datetime(bytes))
            }
            ParsedValue::LocalDate(bytes) => visitor.visit_map(DatetimeAccess::local_date(bytes)),
            ParsedValue::LocalTime(bytes) => visitor.visit_map(DatetimeAccess::local_time(bytes)),
            ParsedValue::Array(array) => visitor.visit_seq(SeqAccess::new(array)),
            ParsedValue::ArrayOfTables(array) => visitor.visit_seq(SeqAccess::new(array)),
            ParsedValue::Table(table)
            | ParsedValue::UndefinedTable(table)
            | ParsedValue::InlineTable(table)
            | ParsedValue::DottedKeyTable(table) => visitor.visit_map(MapAccess::new(table)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Boolean(bool) => visitor.visit_bool(bool),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

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

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Float(bytes) => visitor.visit_f32(parse_float(&bytes)?),
            ParsedValue::SpecialFloat(special) => visitor.visit_f32(parse_special(special)),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::Float(bytes) => visitor.visit_f64(parse_float(&bytes)?),
            ParsedValue::SpecialFloat(special) => visitor.visit_f64(parse_special(special)),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(Cow::Borrowed(str)) => visitor.visit_borrowed_str(str),
            ParsedValue::String(Cow::Owned(string)) => visitor.visit_string(string),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(Cow::Borrowed(str)) => visitor.visit_borrowed_bytes(str.as_bytes()),
            ParsedValue::String(Cow::Owned(string)) => visitor.visit_byte_buf(string.into_bytes()),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::invalid_type(self.value.typ().into(), &visitor))
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

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

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

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

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match (self.value, name, fields) {
            (
                ParsedValue::OffsetDatetime(bytes),
                OffsetDatetime::WRAPPER_TYPE | Datetime::WRAPPER_TYPE,
                &[OffsetDatetime::WRAPPER_FIELD | Datetime::WRAPPER_FIELD],
            ) => visitor.visit_map(DatetimeAccess::offset_datetime(bytes)),
            (
                ParsedValue::LocalDatetime(bytes),
                LocalDatetime::WRAPPER_TYPE | Datetime::WRAPPER_TYPE,
                &[LocalDatetime::WRAPPER_FIELD | Datetime::WRAPPER_FIELD],
            ) => visitor.visit_map(DatetimeAccess::local_datetime(bytes)),
            (
                ParsedValue::LocalDate(bytes),
                LocalDate::WRAPPER_TYPE | Datetime::WRAPPER_TYPE,
                &[LocalDate::WRAPPER_FIELD | Datetime::WRAPPER_FIELD],
            ) => visitor.visit_map(DatetimeAccess::local_date(bytes)),
            (
                ParsedValue::LocalTime(bytes),
                LocalTime::WRAPPER_TYPE | Datetime::WRAPPER_TYPE,
                &[LocalTime::WRAPPER_FIELD | Datetime::WRAPPER_FIELD],
            ) => visitor.visit_map(DatetimeAccess::local_time(bytes)),
            (
                ParsedValue::Table(table)
                | ParsedValue::UndefinedTable(table)
                | ParsedValue::DottedKeyTable(table)
                | ParsedValue::InlineTable(table),
                _,
                _,
            ) => visitor.visit_map(MapAccess::new(table)),
            (value, _, _) => Err(Error::invalid_type(value.typ().into(), &visitor)),
        }
    }

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

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

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
    fn new(array: Vec<T>) -> Self {
        Self {
            values: array.into_iter(),
        }
    }
}

// For regular arrays
impl<'de> de::SeqAccess<'de> for SeqAccess<ParsedValue<'de>> {
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| seed.deserialize(ValueDeserializer::new(value)))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

// Used for array of tables
impl<'de> de::SeqAccess<'de> for SeqAccess<ParsedTable<'de>> {
    type Error = Error;

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

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

struct MapAccess<'de> {
    kv_pairs: <ParsedTable<'de> as IntoIterator>::IntoIter,
    next_value: Option<ParsedValue<'de>>,
}

impl<'de> MapAccess<'de> {
    fn new(table: ParsedTable<'de>) -> Self {
        Self {
            kv_pairs: table.into_iter(),
            next_value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess<'de> {
    type Error = Error;

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

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic)]
        let Some(value) = self.next_value.take() else {
            panic!("next_value_seed called without calling next_key_seed first")
        };
        seed.deserialize(ValueDeserializer::new(value))
    }

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

    fn size_hint(&self) -> Option<usize> {
        Some(self.kv_pairs.len())
    }
}

#[derive(Debug)]
struct EnumAccess<'de> {
    variant: Cow<'de, str>,
    value: ParsedValue<'de>,
}

impl<'de> EnumAccess<'de> {
    fn new(table: ParsedTable<'de>) -> Result<Self> {
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

impl<'de> de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(StrDeserializer::<Error>::new(&self.variant))?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        // We allow unit variants to be represented by `x = { variant = {} }` in addition to the
        // normal `x = "variant"`. toml-rs seems to do the same.
        match self.value {
            ParsedValue::Table(table) if table.is_empty() => Ok(()),
            _ => Err(Error::invalid_type(self.value.typ().into(), &"empty table")),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(ValueDeserializer::new(self.value))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(ValueDeserializer::new(self.value), visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(ValueDeserializer::new(self.value), visitor)
    }
}

trait Integer: FromLexicalWithOptions<Options = ParseIntegerOptions> {}

macro_rules! impl_integer {
    ($($t:ident)*) => ($(impl Integer for $t {})*);
}

impl_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

trait Float: FromLexicalWithOptions<Options = ParseFloatOptions> {
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

fn parse_integer<T: Integer>(bytes: &[u8]) -> Result<T> {
    const FORMAT: u128 = NumberFormatBuilder::new()
        .digit_separator(NonZero::new(b'_'))
        .no_integer_leading_zeros(true)
        .internal_digit_separator(true)
        .build();
    const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

    T::from_lexical_with_options::<FORMAT>(bytes, &OPTIONS)
        .map_err(|err| ErrorKind::InvalidNumber(err.to_string().into()).into())
}

fn parse_binary<T: Integer>(bytes: &[u8]) -> Result<T> {
    const FORMAT: u128 = NumberFormatBuilder::new()
        .digit_separator(NonZero::new(b'_'))
        .radix(2)
        .internal_digit_separator(true)
        .build();
    const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

    T::from_lexical_with_options::<FORMAT>(bytes, &OPTIONS)
        .map_err(|err| ErrorKind::InvalidNumber(err.to_string().into()).into())
}

fn parse_octal<T: Integer>(bytes: &[u8]) -> Result<T> {
    const FORMAT: u128 = NumberFormatBuilder::new()
        .digit_separator(NonZero::new(b'_'))
        .radix(8)
        .internal_digit_separator(true)
        .build();
    const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

    T::from_lexical_with_options::<FORMAT>(bytes, &OPTIONS)
        .map_err(|err| ErrorKind::InvalidNumber(err.to_string().into()).into())
}

fn parse_hexadecimal<T: Integer>(bytes: &[u8]) -> Result<T> {
    const FORMAT: u128 = NumberFormatBuilder::new()
        .digit_separator(NonZero::new(b'_'))
        .radix(16)
        .internal_digit_separator(true)
        .build();
    const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

    T::from_lexical_with_options::<FORMAT>(bytes, &OPTIONS)
        .map_err(|err| ErrorKind::InvalidNumber(err.to_string().into()).into())
}

fn parse_float<T: Float>(bytes: &[u8]) -> Result<T> {
    const FORMAT: u128 = NumberFormatBuilder::new()
        .digit_separator(NonZero::new(b'_'))
        .required_digits(true)
        .no_special(true) // Handled in deserialize_special_float
        .no_integer_leading_zeros(true)
        .no_float_leading_zeros(true)
        .internal_digit_separator(true)
        .build();
    const OPTIONS: ParseFloatOptions = ParseFloatOptions::new();

    T::from_lexical_with_options::<FORMAT>(bytes, &OPTIONS)
        .map_err(|err| ErrorKind::InvalidNumber(err.to_string().into()).into())
}

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
    use isclose::{assert_is_close, IsClose as _};
    use maplit::hashmap;
    use serde::de::{EnumAccess as _, MapAccess as _, SeqAccess as _, VariantAccess as _};
    use serde_bytes::ByteBuf;

    use super::*;
    use crate::value::{Datetime, Offset};
    use crate::Value;

    mod example {
        use std::collections::HashMap;

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
        let result: example::Struct = from_str(indoc! {r#"
            # This is a TOML document.

            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates

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
        "#})
        .unwrap();

        assert_eq!(
            result,
            example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
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
        let result: example::Struct = from_slice(indoc! {br#"
            # This is a TOML document.

            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates

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
        "#})
        .unwrap();

        assert_eq!(
            result,
            example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
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
            indoc! {br#"
            # This is a TOML document.

            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates

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
        "#}
            .as_ref(),
        )
        .unwrap();

        assert_eq!(
            result,
            example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
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

        assert_eq!(
            deserializer.parser.parse().unwrap(),
            ParsedValue::Table(hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".into()),
            }),
        );
    }

    #[test]
    fn deserializer_from_slice() {
        let mut deserializer = Deserializer::from_slice(b"abc = 123");

        assert_eq!(
            deserializer.parser.parse().unwrap(),
            ParsedValue::Table(hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".into()),
            }),
        );
    }

    #[test]
    fn deserializer_from_reader() {
        let mut deserializer = Deserializer::from_reader(b"abc = 123".as_ref());

        assert_eq!(
            deserializer.parser.parse().unwrap(),
            ParsedValue::Table(hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".into()),
            }),
        );
    }

    #[test]
    fn deserializer_deserialize_any() {
        let deserializer = Deserializer::from_str("abc = 123");

        assert_eq!(
            HashMap::deserialize(deserializer).unwrap(),
            hashmap! {
                "abc".to_owned() => 123_i32,
            },
        );
    }

    #[test]
    fn value_deserializer_new() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));

        assert_eq!(deserializer.value, ParsedValue::String("hello".into()));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn value_deserializer_deserialize_any() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::String("hello".into()),
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Integer(123)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Integer(10)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Integer(83)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"123".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Integer(291)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Float(b"123.0".into()));
        assert_matches!(
            Value::deserialize(deserializer).unwrap(),
            Value::Float(v) if v.is_close(123.0)
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::SpecialFloat(SpecialFloat::Infinity));
        assert_matches!(
            Value::deserialize(deserializer).unwrap(),
            Value::Float(v) if v.is_infinite()
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Boolean(true));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Boolean(true)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::OffsetDatetime(
            b"1979-05-27T07:32:00-08:00".into(),
        ));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Datetime(Datetime {
                date: Some(LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27,
                }),
                time: Some(LocalTime {
                    hour: 7,
                    minute: 32,
                    second: 0,
                    nanosecond: 0,
                }),
                offset: Some(Offset::Custom { minutes: -480 }),
            })
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalDatetime(b"1979-05-27T07:32:00".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Datetime(Datetime {
                date: Some(LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27,
                }),
                time: Some(LocalTime {
                    hour: 7,
                    minute: 32,
                    second: 0,
                    nanosecond: 0,
                }),
                offset: None,
            })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalDate(b"1979-05-27".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Datetime(Datetime {
                date: Some(LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27
                }),
                time: None,
                offset: None,
            })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalTime(b"07:32:00".into()));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Datetime(Datetime {
                date: None,
                time: Some(LocalTime {
                    hour: 7,
                    minute: 32,
                    second: 0,
                    nanosecond: 0
                }),
                offset: None,
            })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".into()),
            ParsedValue::Integer(b"456".into()),
            ParsedValue::Integer(b"789".into()),
        ]));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Array(vec![
                Value::Integer(123),
                Value::Integer(456),
                Value::Integer(789),
            ])
        );

        let deserializer = ValueDeserializer::new(ParsedValue::ArrayOfTables(vec![
            hashmap! {
                "abc".into() => ParsedValue::Integer(b"123".into()),
            },
            hashmap! {
                "def".into() => ParsedValue::Integer(b"456".into()),
            },
            hashmap! {
                "ghi".into() => ParsedValue::Integer(b"789".into()),
            },
        ]));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Array(vec![
                Value::Table(hashmap! {
                    "abc".into() => Value::Integer(123),
                }),
                Value::Table(hashmap! {
                    "def".into() => Value::Integer(456),
                }),
                Value::Table(hashmap! {
                    "ghi".into() => Value::Integer(789),
                }),
            ])
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Table(hashmap! {
                "abc".into() => Value::Integer(123),
            })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Table(hashmap! {
                "abc".into() => Value::Integer(123),
            })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Table(hashmap! {
                "abc".into() => Value::Integer(123),
            })
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Value::deserialize(deserializer).unwrap(),
            Value::Table(hashmap! {
                "abc".into() => Value::Integer(123),
            })
        );
    }

    #[test]
    fn value_deserializer_deserialize_bool() {
        let deserializer = ValueDeserializer::new(ParsedValue::Boolean(true));
        assert!(bool::deserialize(deserializer).unwrap());

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        bool::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_i8() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(i8::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(i8::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(i8::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(i8::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        i8::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_i16() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(i16::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(i16::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(i16::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(i16::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        i16::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_i32() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(i32::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(i32::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(i32::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(i32::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        i32::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_i64() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(i64::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(i64::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(i64::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(i64::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        i64::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_i128() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(i128::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(i128::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(i128::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(i128::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        i128::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_u8() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(u8::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(u8::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(u8::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(u8::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        u8::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_u16() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(u16::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(u16::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(u16::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(u16::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        u16::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_u32() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(u32::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(u32::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(u32::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(u32::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        u32::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_u64() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(u64::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(u64::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(u64::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(u64::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        u64::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_u128() {
        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(u128::deserialize(deserializer).unwrap(), 123);

        let deserializer = ValueDeserializer::new(ParsedValue::BinaryInt(b"1010".into()));
        assert_eq!(u128::deserialize(deserializer).unwrap(), 10);

        let deserializer = ValueDeserializer::new(ParsedValue::OctalInt(b"123".into()));
        assert_eq!(u128::deserialize(deserializer).unwrap(), 83);

        let deserializer = ValueDeserializer::new(ParsedValue::HexInt(b"5a".into()));
        assert_eq!(u128::deserialize(deserializer).unwrap(), 90);

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        u128::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_f32() {
        let deserializer = ValueDeserializer::new(ParsedValue::Float(b"123.0".into()));
        assert_is_close!(f32::deserialize(deserializer).unwrap(), 123.0);

        let deserializer =
            ValueDeserializer::new(ParsedValue::SpecialFloat(SpecialFloat::Infinity));
        assert!(f32::deserialize(deserializer).unwrap().is_infinite());

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        f32::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_f64() {
        let deserializer = ValueDeserializer::new(ParsedValue::Float(b"123.0".into()));
        assert_is_close!(f64::deserialize(deserializer).unwrap(), 123.0);

        let deserializer =
            ValueDeserializer::new(ParsedValue::SpecialFloat(SpecialFloat::Infinity));
        assert!(f64::deserialize(deserializer).unwrap().is_infinite());

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        f64::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_char() {
        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Borrowed("A")));
        assert_eq!(char::deserialize(deserializer).unwrap(), 'A');

        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Owned("A".into())));
        assert_eq!(char::deserialize(deserializer).unwrap(), 'A');

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        char::deserialize(deserializer).unwrap_err();

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        char::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_str() {
        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Borrowed("hello")));
        assert_eq!(<&str>::deserialize(deserializer).unwrap(), "hello");

        // TODO
        // let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Owned("hello".into())));
        // assert_eq!(<&str>::deserialize(deserializer).unwrap(), "hello");

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        <&str>::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_string() {
        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Borrowed("hello")));
        assert_eq!(String::deserialize(deserializer).unwrap(), "hello");

        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Owned("hello".into())));
        assert_eq!(String::deserialize(deserializer).unwrap(), "hello");

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        String::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_bytes() {
        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Borrowed("hello")));
        assert_eq!(<&[u8]>::deserialize(deserializer).unwrap(), b"hello");

        // let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Owned("hello".into())));
        // assert_eq!(<&[u8]>::deserialize(deserializer).unwrap(), b"hello");

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        <&[u8]>::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_byte_buf() {
        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Borrowed("hello")));
        assert_eq!(
            ByteBuf::deserialize(deserializer).unwrap(),
            b"hello".to_owned()
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String(Cow::Owned("hello".into())));
        assert_eq!(
            ByteBuf::deserialize(deserializer).unwrap(),
            b"hello".to_owned()
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        ByteBuf::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_option() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        assert_eq!(
            Option::<&str>::deserialize(deserializer).unwrap(),
            Some("hello")
        );
    }

    #[test]
    fn value_deserializer_deserialize_unit() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        <()>::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_unit_struct() {
        #[derive(Debug, Deserialize)]
        struct Unit;

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        Unit::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_newtype_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Newtype(i32);

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        assert_eq!(Newtype::deserialize(deserializer).unwrap(), Newtype(123));
    }

    #[test]
    fn value_deserializer_deserialize_seq() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Struct {
            val: i32,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".into()),
            ParsedValue::Integer(b"456".into()),
            ParsedValue::Integer(b"789".into()),
        ]));
        assert_eq!(
            <Vec<i32>>::deserialize(deserializer).unwrap(),
            vec![123, 456, 789]
        );

        let deserializer = ValueDeserializer::new(ParsedValue::ArrayOfTables(vec![
            hashmap! { "val".into() => ParsedValue::Integer(b"123".into()) },
            hashmap! { "val".into() => ParsedValue::Integer(b"456".into()) },
            hashmap! { "val".into() => ParsedValue::Integer(b"789".into()) },
        ]));
        assert_eq!(
            <Vec<Struct>>::deserialize(deserializer).unwrap(),
            vec![
                Struct { val: 123 },
                Struct { val: 456 },
                Struct { val: 789 },
            ]
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        <Vec<i32>>::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_tuple() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Struct<T> {
            val: T,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".into()),
            ParsedValue::String("hello".into()),
            ParsedValue::LocalDate(b"1979-05-27".into()),
        ]));
        assert_eq!(
            <(i32, String, LocalDate)>::deserialize(deserializer).unwrap(),
            (
                123,
                "hello".into(),
                LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27,
                }
            )
        );

        let deserializer = ValueDeserializer::new(ParsedValue::ArrayOfTables(vec![
            hashmap! { "val".into() => ParsedValue::Integer(b"123".into()) },
            hashmap! { "val".into() => ParsedValue::String("hello".into()) },
            hashmap! { "val".into() => ParsedValue::LocalDate(b"1979-05-27".into()) },
        ]));
        assert_eq!(
            <(Struct<i32>, Struct<String>, Struct<LocalDate>)>::deserialize(deserializer).unwrap(),
            (
                Struct { val: 123 },
                Struct {
                    val: "hello".into()
                },
                Struct {
                    val: LocalDate {
                        year: 1979,
                        month: 5,
                        day: 27,
                    }
                },
            )
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        <(i32, i32, i32)>::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_tuple_struct_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct TupleStruct(i32, String, LocalDate);

        let deserializer = ValueDeserializer::new(ParsedValue::Array(vec![
            ParsedValue::Integer(b"123".into()),
            ParsedValue::String("hello".into()),
            ParsedValue::LocalDate(b"1979-05-27".into()),
        ]));
        assert_eq!(
            TupleStruct::deserialize(deserializer).unwrap(),
            TupleStruct(
                123,
                "hello".into(),
                LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27,
                }
            )
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        TupleStruct::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_map() {
        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            HashMap::<String, i32>::deserialize(deserializer).unwrap(),
            hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            HashMap::<String, i32>::deserialize(deserializer).unwrap(),
            hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            HashMap::<String, i32>::deserialize(deserializer).unwrap(),
            hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            HashMap::<String, i32>::deserialize(deserializer).unwrap(),
            hashmap! {
                "abc".into() => 123,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        HashMap::<String, i32>::deserialize(deserializer).unwrap_err();
    }

    #[test]
    fn value_deserializer_deserialize_struct() {
        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct Struct {
            abc: i32,
        }

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Struct::deserialize(deserializer).unwrap(),
            Struct { abc: 123 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Struct::deserialize(deserializer).unwrap(),
            Struct { abc: 123 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Struct::deserialize(deserializer).unwrap(),
            Struct { abc: 123 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Struct::deserialize(deserializer).unwrap(),
            Struct { abc: 123 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        Struct::deserialize(deserializer).unwrap_err();
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn value_deserializer_deserialize_struct_datetime() {
        let deserializer = ValueDeserializer::new(ParsedValue::OffsetDatetime(
            b"1979-05-27T07:32:00-08:00".into(),
        ));
        assert_eq!(
            OffsetDatetime::deserialize(deserializer).unwrap(),
            OffsetDatetime {
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
            }
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalDatetime(b"1979-05-27T07:32:00".into()));
        assert_eq!(
            LocalDatetime::deserialize(deserializer).unwrap(),
            LocalDatetime {
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
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalDate(b"1979-05-27".into()));
        assert_eq!(
            LocalDate::deserialize(deserializer).unwrap(),
            LocalDate {
                year: 1979,
                month: 5,
                day: 27,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalTime(b"07:32:00".into()));
        assert_eq!(
            LocalTime::deserialize(deserializer).unwrap(),
            LocalTime {
                hour: 7,
                minute: 32,
                second: 0,
                nanosecond: 0,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::OffsetDatetime(
            b"1979-05-27T07:32:00-08:00".into(),
        ));
        assert_eq!(
            Datetime::deserialize(deserializer).unwrap(),
            Datetime {
                date: Some(LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27,
                }),
                time: Some(LocalTime {
                    hour: 7,
                    minute: 32,
                    second: 0,
                    nanosecond: 0,
                }),
                offset: Some(Offset::Custom { minutes: -480 }),
            }
        );

        let deserializer =
            ValueDeserializer::new(ParsedValue::LocalDatetime(b"1979-05-27T07:32:00".into()));
        assert_eq!(
            Datetime::deserialize(deserializer).unwrap(),
            Datetime {
                date: Some(LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27,
                }),
                time: Some(LocalTime {
                    hour: 7,
                    minute: 32,
                    second: 0,
                    nanosecond: 0,
                }),
                offset: None,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalDate(b"1979-05-27".into()));
        assert_eq!(
            Datetime::deserialize(deserializer).unwrap(),
            Datetime {
                date: Some(LocalDate {
                    year: 1979,
                    month: 5,
                    day: 27
                }),
                time: None,
                offset: None,
            }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::LocalTime(b"07:32:00".into()));
        assert_eq!(
            Datetime::deserialize(deserializer).unwrap(),
            Datetime {
                date: None,
                time: Some(LocalTime {
                    hour: 7,
                    minute: 32,
                    second: 0,
                    nanosecond: 0
                }),
                offset: None,
            }
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
        assert_eq!(Enum::deserialize(deserializer).unwrap(), Enum::VariantA);

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantB(123)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantB(123)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantB(123)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantB(123)
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantC { a: 123, b: 456 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::UndefinedTable(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantC { a: 123, b: 456 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::DottedKeyTable(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantC { a: 123, b: 456 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::InlineTable(hashmap! {
            "VariantC".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        }));
        assert_eq!(
            Enum::deserialize(deserializer).unwrap(),
            Enum::VariantC { a: 123, b: 456 }
        );

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantA".into() => ParsedValue::Integer(b"123".into()),
        }));
        Enum::deserialize(deserializer).unwrap_err();

        let deserializer = ValueDeserializer::new(ParsedValue::Table(hashmap! {
            "VariantB".into() => ParsedValue::InlineTable(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        }));
        Enum::deserialize(deserializer).unwrap_err();

        let deserializer = ValueDeserializer::new(ParsedValue::String("VariantC".into()));
        Enum::deserialize(deserializer).unwrap_err();

        let deserializer = ValueDeserializer::new(ParsedValue::Integer(b"123".into()));
        Enum::deserialize(deserializer).unwrap_err();
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
        assert_eq!(Enum::deserialize(deserializer).unwrap(), Enum::VariantA);

        let deserializer = ValueDeserializer::new(ParsedValue::String("VariantB".into()));
        assert_eq!(Enum::deserialize(deserializer).unwrap(), Enum::VariantB);
    }

    #[test]
    fn value_deserializer_deserialize_ignored_any() {
        let deserializer = ValueDeserializer::new(ParsedValue::String("hello".into()));
        de::IgnoredAny::deserialize(deserializer).unwrap();
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
            ParsedValue::Integer(b"123".into()),
            ParsedValue::Integer(b"456".into()),
            ParsedValue::Integer(b"789".into()),
        ];
        let mut seq = SeqAccess::new(array.clone());

        assert_eq!(seq.next_element().unwrap(), Some(123));
        assert_eq!(seq.next_element().unwrap(), Some(456));
        assert_eq!(seq.next_element().unwrap(), Some(789));
        assert_eq!(seq.next_element().unwrap(), None::<i32>);

        let array = vec![
            hashmap! { "abc".into() => ParsedValue::Integer(b"123".into()) },
            hashmap! { "def".into() => ParsedValue::Integer(b"456".into()) },
            hashmap! { "ghi".into() => ParsedValue::Integer(b"789".into()) },
        ];
        let mut seq = SeqAccess::new(array.clone());

        assert_eq!(
            seq.next_element().unwrap(),
            Some(hashmap! { "abc".to_owned() => 123 })
        );
        assert_eq!(
            seq.next_element().unwrap(),
            Some(hashmap! { "def".to_owned() => 456 })
        );
        assert_eq!(
            seq.next_element().unwrap(),
            Some(hashmap! { "ghi".to_owned() => 789 })
        );
        assert_eq!(seq.next_element().unwrap(), None::<HashMap<String, i32>>);
    }

    #[test]
    fn seq_access_size_hint() {
        let array = vec![
            ParsedValue::Integer(b"123".into()),
            ParsedValue::Integer(b"456".into()),
            ParsedValue::Integer(b"789".into()),
        ];
        let seq = SeqAccess::new(array);

        assert_eq!(seq.size_hint(), Some(3));

        let array = vec![
            hashmap! { "abc".into() => ParsedValue::Integer(b"123".into()) },
            hashmap! { "def".into() => ParsedValue::Integer(b"456".into()) },
            hashmap! { "ghi".into() => ParsedValue::Integer(b"789".into()) },
        ];
        let seq = SeqAccess::new(array);

        assert_eq!(seq.size_hint(), Some(3));
    }

    #[test]
    fn map_access_new() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
            "def".into() => ParsedValue::Integer(b"456".into()),
            "ghi".into() => ParsedValue::Integer(b"789".into()),
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
            "abc".into() => ParsedValue::Integer(b"123".into()),
            "def".into() => ParsedValue::Integer(b"456".into()),
            "ghi".into() => ParsedValue::Integer(b"789".into()),
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

        assert_eq!(map.next_key().unwrap(), None::<String>);
    }

    #[test]
    fn map_access_next_entry() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
            "def".into() => ParsedValue::Integer(b"456".into()),
            "ghi".into() => ParsedValue::Integer(b"789".into()),
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

        assert_eq!(map.next_key().unwrap(), None::<String>);
    }

    #[test]
    fn map_access_size_hint() {
        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
            "def".into() => ParsedValue::Integer(b"456".into()),
            "ghi".into() => ParsedValue::Integer(b"789".into()),
        };
        let map = MapAccess::new(table);
        assert_eq!(map.size_hint(), Some(3));
    }

    #[test]
    fn enum_access_new() {
        let table = hashmap! {
            "Variant".into() => ParsedValue::Integer(b"123".into()),
        };
        let enum_ = EnumAccess::new(table).unwrap();
        assert_eq!(enum_.variant, "Variant");
        assert_eq!(enum_.value, ParsedValue::Integer(b"123".into()));

        let table = hashmap! {
            "abc".into() => ParsedValue::Integer(b"123".into()),
            "def".into() => ParsedValue::Integer(b"456".into()),
            "ghi".into() => ParsedValue::Integer(b"789".into()),
        };
        EnumAccess::new(table).unwrap_err();

        let table = hashmap! {};
        EnumAccess::new(table).unwrap_err();
    }

    #[test]
    fn enum_access_variant_seed() {
        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_eq!(access.variant::<String>().unwrap().0, "VariantA");

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_eq!(access.variant::<String>().unwrap().0, "VariantB");

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_eq!(access.variant::<String>().unwrap().0, "VariantC");
    }

    #[test]
    fn enum_access_unit_variant() {
        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        access.unit_variant().unwrap();

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        };
        let access = EnumAccess::new(table).unwrap();
        access.unit_variant().unwrap_err();

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".into()),
                ParsedValue::Integer(b"456".into()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        access.unit_variant().unwrap_err();

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        access.unit_variant().unwrap_err();
    }

    #[test]
    fn enum_access_newtype_variant() {
        let table = hashmap! {
            "VariantA".into() => ParsedValue::Table(hashmap! {}),
        };
        let access = EnumAccess::new(table).unwrap();
        access.newtype_variant::<i32>().unwrap_err();

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_eq!(access.newtype_variant::<i32>().unwrap(), 123);

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".into()),
                ParsedValue::Integer(b"456".into()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        access.newtype_variant::<i32>().unwrap_err();

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        access.newtype_variant::<i32>().unwrap_err();
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
        access.tuple_variant(0, Visitor).unwrap_err();

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        };
        let access = EnumAccess::new(table).unwrap();
        access.tuple_variant(1, Visitor).unwrap_err();

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".into()),
                ParsedValue::Integer(b"456".into()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_eq!(access.tuple_variant(2, Visitor).unwrap(), vec![123, 456]);

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        access.tuple_variant(2, Visitor).unwrap_err();
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
        assert_eq!(access.struct_variant(&[], Visitor).unwrap(), hashmap! {});

        let table = hashmap! {
            "VariantB".into() => ParsedValue::Integer(b"123".into()),
        };
        let access = EnumAccess::new(table).unwrap();
        access.struct_variant(&["a"], Visitor).unwrap_err();

        let table = hashmap! {
            "VariantC".into() => ParsedValue::Array(vec![
                ParsedValue::Integer(b"123".into()),
                ParsedValue::Integer(b"456".into()),
            ]),
        };
        let access = EnumAccess::new(table).unwrap();
        access.struct_variant(&["a", "b"], Visitor).unwrap_err();

        let table = hashmap! {
            "VariantD".into() => ParsedValue::Table(hashmap! {
                "a".into() => ParsedValue::Integer(b"123".into()),
                "b".into() => ParsedValue::Integer(b"456".into()),
            }),
        };
        let access = EnumAccess::new(table).unwrap();
        assert_eq!(
            access.struct_variant(&["a", "b"], Visitor).unwrap(),
            hashmap! {
                "a".into() => 123,
                "b".into() => 456,
            }
        );
    }

    #[test]
    fn test_parse_integer() {
        let bytes = b"123";
        assert_eq!(parse_integer::<i32>(bytes).unwrap(), 123);

        let bytes = b"1_2_3";
        assert_eq!(parse_integer::<i32>(bytes).unwrap(), 123);

        let bytes = b"0123";
        parse_integer::<i32>(bytes).unwrap_err();

        let bytes = b"123_";
        parse_integer::<i32>(bytes).unwrap_err();

        let bytes = b"_123";
        parse_integer::<i32>(bytes).unwrap_err();

        let bytes = b"123.0";
        parse_integer::<i32>(bytes).unwrap_err();
    }

    #[test]
    fn test_parse_binary() {
        let bytes = b"1010";
        assert_eq!(parse_binary::<i32>(bytes).unwrap(), 10);

        let bytes = b"1_0_1_0";
        assert_eq!(parse_binary::<i32>(bytes).unwrap(), 10);

        let bytes = b"01010"; // Leading zeros are ok because we already have a leading 0b
        assert_eq!(parse_binary::<i32>(bytes).unwrap(), 10);

        let bytes = b"1010_";
        parse_binary::<i32>(bytes).unwrap_err();

        let bytes = b"_1010";
        parse_binary::<i32>(bytes).unwrap_err();

        let bytes = b"1010.0";
        parse_binary::<i32>(bytes).unwrap_err();
    }

    #[test]
    fn test_parse_octal() {
        let bytes = b"123";
        assert_eq!(parse_octal::<i32>(bytes).unwrap(), 83);

        let bytes = b"1_2_3";
        assert_eq!(parse_octal::<i32>(bytes).unwrap(), 83);

        let bytes = b"0123"; // Leading zeros are ok because we already have a leading 0o
        assert_eq!(parse_octal::<i32>(bytes).unwrap(), 83);

        let bytes = b"123_";
        parse_octal::<i32>(bytes).unwrap_err();

        let bytes = b"_123";
        parse_octal::<i32>(bytes).unwrap_err();

        let bytes = b"123.0";
        parse_octal::<i32>(bytes).unwrap_err();
    }

    #[test]
    fn test_parse_hexadecimal() {
        let bytes = b"123";
        assert_eq!(parse_hexadecimal::<i32>(bytes).unwrap(), 291);

        let bytes = b"1_2_3";
        assert_eq!(parse_hexadecimal::<i32>(bytes).unwrap(), 291);

        let bytes = b"0123"; // Leading zeros are ok because we already have a leading 0x
        assert_eq!(parse_hexadecimal::<i32>(bytes).unwrap(), 291);

        let bytes = b"123_";
        parse_hexadecimal::<i32>(bytes).unwrap_err();

        let bytes = b"_123";
        parse_hexadecimal::<i32>(bytes).unwrap_err();

        let bytes = b"123.0";
        parse_hexadecimal::<i32>(bytes).unwrap_err();
    }

    #[test]
    fn test_parse_float() {
        let bytes = b"123.456";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.456);

        let bytes = b"1_2_3.4_5_6";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.456);

        let bytes = b"123e0";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.0);

        let bytes = b"123e+0";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.0);

        let bytes = b"123e-0";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.0);

        let bytes = b"1_2_3e1_2_3";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123e123);

        let bytes = b"1_2_3e+1_2_3";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123e123);

        let bytes = b"1_2_3e-1_2_3";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123e-123);

        let bytes = b"123.456e123";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.456e123);

        let bytes = b"123.456e+123";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.456e123);

        let bytes = b"123.456e-123";
        assert_is_close!(parse_float::<f64>(bytes).unwrap(), 123.456e-123);

        let bytes = b"0123.0";
        parse_float::<f32>(bytes).unwrap_err();

        let bytes = b"123.0_";
        parse_float::<f32>(bytes).unwrap_err();

        let bytes = b"_123.0";
        parse_float::<f32>(bytes).unwrap_err();

        let bytes = b"123.0.0";
        parse_float::<f32>(bytes).unwrap_err();

        let bytes = b"inf";
        parse_float::<f64>(bytes).unwrap_err();

        let bytes = b"nan";
        parse_float::<f64>(bytes).unwrap_err();

        let bytes = b"+inf";
        parse_float::<f32>(bytes).unwrap_err();

        let bytes = b"-nan";
        parse_float::<f32>(bytes).unwrap_err();
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
