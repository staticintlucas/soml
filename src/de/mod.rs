use std::borrow::Cow;
use std::io;
use std::num::NonZero;

use lexical::{
    FromLexicalWithOptions, NumberFormatBuilder, ParseFloatOptions, ParseIntegerOptions,
};
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeOwned, Error as _, IntoDeserializer as _};
use serde::{de, forward_to_deserialize_any, Deserialize};

pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, Result};
use self::parser::{Parser, SpecialFloat, Table as ParsedTable, Value as ParsedValue};
use self::reader::{IoReader, Reader, SliceReader};
use crate::value::datetime::DatetimeAccess;

mod error;
mod parser;
mod reader;

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

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    T::deserialize(&mut deserializer)
}

pub fn from_slice<'a, T>(bytes: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_slice(bytes);
    T::deserialize(&mut deserializer)
}

pub fn from_reader<R, T>(read: R) -> Result<T>
where
    R: io::Read,
    T: DeserializeOwned,
{
    let mut deserializer = Deserializer::from_reader(read);
    T::deserialize(&mut deserializer)
}

impl<'de, R> de::Deserializer<'de> for &mut Deserializer<'de, R>
where
    R: Reader<'de>,
{
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        ValueDeserializer::new(self.parser.parse()?).deserialize_any(visitor)
    }

    forward_to_deserialize_any! {
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
            ParsedValue::Array(array) => visitor.visit_seq(SeqAccess::new(array)),
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
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ParsedValue::String(str) => visitor.visit_enum(str.into_deserializer()),
            ParsedValue::Table(table) => visitor.visit_enum(EnumAccess::new(table)?),
            _ => Err(Error::invalid_type(self.value.typ().into(), &visitor)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
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
