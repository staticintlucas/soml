use std::result::Result as StdResult;
use std::{fmt, slice, vec};

use serde::de;
use serde::de::{Error as _, IntoDeserializer as _};

#[cfg(feature = "datetime")]
use super::datetime::{
    AnyDatetime, EncodedLocalDate, EncodedLocalDatetime, EncodedLocalTime, EncodedOffsetDatetime,
    LocalDate, LocalDateAccess, LocalDatetime, LocalDatetimeAccess, LocalTime, LocalTimeAccess,
    OffsetDatetime, OffsetDatetimeAccess,
};
use super::{Type, Value};
use crate::de::{Error, Result};
use crate::{map, Table};

impl Value {
    /// Try to convert the value into type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to type `T`.
    #[inline]
    pub fn try_into<'de, T>(self) -> Result<T>
    where
        T: de::Deserialize<'de>,
    {
        T::deserialize(self)
    }
}

impl From<Type> for de::Unexpected<'_> {
    #[inline]
    fn from(typ: Type) -> Self {
        de::Unexpected::Other(typ.to_str())
    }
}

impl de::IntoDeserializer<'_, Error> for Value {
    type Deserializer = Self;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> de::IntoDeserializer<'de, Error> for &'de Value {
    type Deserializer = Self;

    #[inline]
    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> de::Deserialize<'de> for Value {
    #[allow(clippy::too_many_lines)]
    #[inline]
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[cfg(feature = "datetime")]
        #[derive(Debug)]
        enum MapField {
            Datetime(DatetimeField),
            Other(String),
        }

        #[cfg(feature = "datetime")]
        #[derive(Debug, PartialEq, Eq)]
        enum DatetimeField {
            OffsetDatetime,
            LocalDatetime,
            LocalDate,
            LocalTime,
        }

        #[cfg(feature = "datetime")]
        struct MapFieldVisitor;

        #[cfg(feature = "datetime")]
        impl MapField {
            #[inline]
            fn as_str(&self) -> &str {
                match *self {
                    #[cfg(feature = "datetime")]
                    Self::Datetime(ref field) => field.as_str(),
                    Self::Other(ref field) => field.as_str(),
                }
            }
        }
        #[cfg(feature = "datetime")]
        impl DatetimeField {
            #[inline]
            fn as_str(&self) -> &'static str {
                match *self {
                    Self::OffsetDatetime => OffsetDatetime::WRAPPER_FIELD,
                    Self::LocalDatetime => LocalDatetime::WRAPPER_FIELD,
                    Self::LocalDate => LocalDate::WRAPPER_FIELD,
                    Self::LocalTime => LocalTime::WRAPPER_FIELD,
                }
            }
            #[inline]
            fn expected(&self) -> &'static [&'static str] {
                match *self {
                    Self::OffsetDatetime => &[OffsetDatetime::WRAPPER_FIELD],
                    Self::LocalDatetime => &[LocalDatetime::WRAPPER_FIELD],
                    Self::LocalDate => &[LocalDate::WRAPPER_FIELD],
                    Self::LocalTime => &[LocalTime::WRAPPER_FIELD],
                }
            }
        }

        #[cfg(feature = "datetime")]
        impl de::Visitor<'_> for MapFieldVisitor {
            type Value = MapField;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a TOML key")
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                match value.as_str() {
                    #[cfg(feature = "datetime")]
                    OffsetDatetime::WRAPPER_FIELD => {
                        Ok(Self::Value::Datetime(DatetimeField::OffsetDatetime))
                    }
                    #[cfg(feature = "datetime")]
                    LocalDatetime::WRAPPER_FIELD => {
                        Ok(Self::Value::Datetime(DatetimeField::LocalDatetime))
                    }
                    #[cfg(feature = "datetime")]
                    LocalDate::WRAPPER_FIELD => Ok(Self::Value::Datetime(DatetimeField::LocalDate)),
                    #[cfg(feature = "datetime")]
                    LocalTime::WRAPPER_FIELD => Ok(Self::Value::Datetime(DatetimeField::LocalTime)),
                    _ => Ok(Self::Value::Other(value)),
                }
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    #[cfg(feature = "datetime")]
                    OffsetDatetime::WRAPPER_FIELD => {
                        Ok(Self::Value::Datetime(DatetimeField::OffsetDatetime))
                    }
                    #[cfg(feature = "datetime")]
                    LocalDatetime::WRAPPER_FIELD => {
                        Ok(Self::Value::Datetime(DatetimeField::LocalDatetime))
                    }
                    #[cfg(feature = "datetime")]
                    LocalDate::WRAPPER_FIELD => Ok(Self::Value::Datetime(DatetimeField::LocalDate)),
                    #[cfg(feature = "datetime")]
                    LocalTime::WRAPPER_FIELD => Ok(Self::Value::Datetime(DatetimeField::LocalTime)),
                    _ => Ok(Self::Value::Other(value.to_string())),
                }
            }
        }

        #[cfg(feature = "datetime")]
        impl<'de> de::Deserialize<'de> for MapField {
            #[inline]
            fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(MapFieldVisitor)
            }
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Value;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a TOML value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_i128<E>(self, value: i128) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i64(value.try_into().map_err(|_| {
                    E::invalid_value(de::Unexpected::Other("value out of range"), &"an i64")
                })?)
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i64(value.try_into().map_err(|_| {
                    E::invalid_value(de::Unexpected::Other("value out of range"), &"an i64")
                })?)
            }

            #[inline]
            fn visit_u128<E>(self, value: u128) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i64(value.try_into().map_err(|_| {
                    E::invalid_value(de::Unexpected::Other("value out of range"), &"an i64")
                })?)
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            #[inline]
            fn visit_bytes<E>(self, value: &[u8]) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value.into())
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> StdResult<Self::Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_seq<A>(self, mut seq: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(256));
                while let Some(element) = seq.next_element()? {
                    result.push(element);
                }
                Ok(Self::Value::Array(result))
            }

            #[cfg(feature = "datetime")]
            #[inline]
            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(key) = map.next_key::<MapField>()? else {
                    return Ok(Self::Value::Table(Table::new()));
                };

                match key {
                    MapField::Datetime(key) => {
                        let result = match key {
                            DatetimeField::OffsetDatetime => Self::Value::Datetime(
                                map.next_value::<EncodedOffsetDatetime>()?.0.into(),
                            ),
                            DatetimeField::LocalDatetime => Self::Value::Datetime(
                                map.next_value::<EncodedLocalDatetime>()?.0.into(),
                            ),
                            DatetimeField::LocalDate => Self::Value::Datetime(
                                map.next_value::<EncodedLocalDate>()?.0.into(),
                            ),
                            DatetimeField::LocalTime => Self::Value::Datetime(
                                map.next_value::<EncodedLocalTime>()?.0.into(),
                            ),
                        };

                        match map.next_key::<MapField>()? {
                            Some(MapField::Datetime(k)) if k == key => {
                                Err(de::Error::duplicate_field(key.as_str()))
                            }
                            Some(k) => Err(de::Error::unknown_field(k.as_str(), key.expected())),
                            None => Ok(result),
                        }
                    }
                    MapField::Other(key) => {
                        let mut result = Table::new();
                        result.insert(key, map.next_value()?);
                        while let Some((key, value)) = map.next_entry()? {
                            result.insert(key, value);
                        }
                        Ok(Self::Value::Table(result))
                    }
                }
            }

            #[cfg(not(feature = "datetime"))]
            #[inline]
            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut result = Table::new();
                while let Some((key, value)) = map.next_entry()? {
                    result.insert(key, value);
                }
                Ok(Self::Value::Table(result))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> de::Deserializer<'de> for Value {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Self::String(str) => visitor.visit_string(str),
            Self::Integer(int) => visitor.visit_i64(int),
            Self::Float(float) => visitor.visit_f64(float),
            Self::Boolean(bool) => visitor.visit_bool(bool),
            #[cfg(feature = "datetime")]
            Self::Datetime(datetime) => match datetime.try_into()? {
                AnyDatetime::OffsetDatetime(datetime) => {
                    visitor.visit_map(OffsetDatetimeAccess::new(datetime.to_bytes()))
                }
                AnyDatetime::LocalDatetime(datetime) => {
                    visitor.visit_map(LocalDatetimeAccess::new(datetime.to_bytes()))
                }
                AnyDatetime::LocalDate(date) => {
                    visitor.visit_map(LocalDateAccess::new(date.to_bytes()))
                }
                AnyDatetime::LocalTime(time) => {
                    visitor.visit_map(LocalTimeAccess::new(time.to_bytes()))
                }
            },
            Self::Array(array) => visitor.visit_seq(SeqAccess::new(array)),
            Self::Table(table) => visitor.visit_map(MapAccess::new(table)),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
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
        match self {
            Self::String(str) => visitor.visit_enum(str.into_deserializer()),
            Self::Table(table) => visitor.visit_enum(EnumAccess::new(table)?),
            _ => Err(Error::invalid_type(self.typ().into(), &visitor)),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit
        unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

#[derive(Debug)]
struct SeqAccess {
    values: vec::IntoIter<Value>,
}

impl SeqAccess {
    #[inline]
    fn new(array: Vec<Value>) -> Self {
        Self {
            values: array.into_iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess {
    type Error = Error;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| seed.deserialize(value))
            .transpose()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

#[derive(Debug)]
struct MapAccess {
    kv_pairs: map::IntoIter,
    next_value: Option<Value>,
}

impl MapAccess {
    #[inline]
    fn new(table: Table) -> Self {
        Self {
            kv_pairs: table.into_iter(),
            next_value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess {
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
            panic!("MapAccess::next_value called without calling MapAccess::next_key first")
        };
        seed.deserialize(value)
    }

    fn next_entry_seed<K, V>(&mut self, kseed: K, vseed: V) -> Result<Option<(K::Value, V::Value)>>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        self.kv_pairs
            .next()
            .map(|(key, value)| {
                Ok((
                    kseed.deserialize(de::value::StrDeserializer::<Error>::new(&key))?,
                    vseed.deserialize(value)?,
                ))
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
    value: Value,
}

impl EnumAccess {
    fn new(table: Table) -> Result<Self> {
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
        let variant = seed.deserialize(de::value::StrDeserializer::<Error>::new(&self.variant))?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumAccess {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<()> {
        match self.value {
            Value::Table(table) if table.is_empty() => Ok(()),
            Value::Table(_) => Err(Error::invalid_value(
                de::Unexpected::Other("non-empty table"),
                &"empty table",
            )),
            _ => Err(Error::invalid_type(self.value.typ().into(), &"empty table")),
        }
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    #[inline]
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.value, visitor)
    }

    #[inline]
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.value, visitor)
    }
}

impl<'de> de::Deserializer<'de> for &'de Value {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match *self {
            Value::String(ref str) => visitor.visit_borrowed_str(str),
            Value::Integer(int) => visitor.visit_i64(int),
            Value::Float(float) => visitor.visit_f64(float),
            Value::Boolean(bool) => visitor.visit_bool(bool),
            #[cfg(feature = "datetime")]
            Value::Datetime(ref datetime) => {
                // Note: a datetime clone here is very cheap, Datetime should probably impl Copy
                // but we don't for toml-rs compatibility.
                match datetime.clone().try_into()? {
                    AnyDatetime::OffsetDatetime(datetime) => {
                        visitor.visit_map(OffsetDatetimeAccess::new(datetime.to_bytes()))
                    }
                    AnyDatetime::LocalDatetime(datetime) => {
                        visitor.visit_map(LocalDatetimeAccess::new(datetime.to_bytes()))
                    }
                    AnyDatetime::LocalDate(date) => {
                        visitor.visit_map(LocalDateAccess::new(date.to_bytes()))
                    }
                    AnyDatetime::LocalTime(time) => {
                        visitor.visit_map(LocalTimeAccess::new(time.to_bytes()))
                    }
                }
            }
            Value::Array(ref array) => visitor.visit_seq(SeqRefAccess::new(array)),
            Value::Table(ref table) => visitor.visit_map(MapRefAccess::new(table)),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
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
        match *self {
            Value::String(ref str) => visitor.visit_enum(str.as_str().into_deserializer()),
            Value::Table(ref table) => visitor.visit_enum(EnumRefAccess::new(table)?),
            _ => Err(Error::invalid_type(self.typ().into(), &visitor)),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit
        unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}

#[derive(Debug)]
struct SeqRefAccess<'de> {
    values: slice::Iter<'de, Value>,
}

impl<'de> SeqRefAccess<'de> {
    #[inline]
    fn new(array: &'de [Value]) -> Self {
        Self {
            values: array.iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqRefAccess<'de> {
    type Error = Error;

    #[inline]
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| seed.deserialize(value))
            .transpose()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

#[derive(Debug)]
struct MapRefAccess<'de> {
    kv_pairs: map::Iter<'de>,
    next_value: Option<&'de Value>,
}

impl<'de> MapRefAccess<'de> {
    #[inline]
    fn new(table: &'de Table) -> Self {
        Self {
            kv_pairs: table.iter(),
            next_value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapRefAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.kv_pairs
            .next()
            .map(|(key, value)| {
                self.next_value = Some(value);
                seed.deserialize(key.as_str().into_deserializer())
            })
            .transpose()
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic)]
        let Some(value) = self.next_value.take() else {
            panic!("MapRefAccess::next_value called without calling MapRefAccess::next_key first")
        };
        seed.deserialize(value)
    }

    fn next_entry_seed<K, V>(&mut self, kseed: K, vseed: V) -> Result<Option<(K::Value, V::Value)>>
    where
        K: de::DeserializeSeed<'de>,
        V: de::DeserializeSeed<'de>,
    {
        self.kv_pairs
            .next()
            .map(|(key, value)| {
                Ok((
                    kseed.deserialize(de::value::StrDeserializer::<Error>::new(key))?,
                    vseed.deserialize(value)?,
                ))
            })
            .transpose()
    }

    #[inline]
    fn size_hint(&self) -> Option<usize> {
        Some(self.kv_pairs.len())
    }
}

#[derive(Debug)]
struct EnumRefAccess<'de> {
    variant: &'de str,
    value: &'de Value,
}

impl<'de> EnumRefAccess<'de> {
    fn new(table: &'de Table) -> Result<Self> {
        let mut table = table.iter();
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

impl<'de> de::EnumAccess<'de> for EnumRefAccess<'de> {
    type Error = Error;
    type Variant = Self;

    #[inline]
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(de::value::StrDeserializer::<Error>::new(self.variant))?;
        Ok((variant, self))
    }
}

impl<'de> de::VariantAccess<'de> for EnumRefAccess<'de> {
    type Error = Error;

    #[inline]
    fn unit_variant(self) -> Result<()> {
        match *self.value {
            Value::Table(ref table) if table.is_empty() => Ok(()),
            Value::Table(_) => Err(de::Error::invalid_value(
                de::Unexpected::Other("non-empty table"),
                &"empty table",
            )),
            _ => Err(Error::invalid_type(self.value.typ().into(), &"empty table")),
        }
    }

    #[inline]
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    #[inline]
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.value, visitor)
    }

    #[inline]
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.value, visitor)
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use std::collections::HashMap;
    use std::iter;
    use std::marker::PhantomData;

    use assert_matches::assert_matches;
    use maplit::{btreemap, hashmap};
    use serde::de::{EnumAccess as _, MapAccess as _, SeqAccess as _, VariantAccess as _};
    use serde::Deserialize;

    use super::*;
    use crate::de::ErrorKind;
    #[cfg(feature = "datetime")]
    use crate::value::Datetime;

    struct OptionDeserializer<T, E> {
        value: Option<T>,
        marker: PhantomData<E>,
    }

    impl<T, E> OptionDeserializer<T, E> {
        fn new(value: Option<T>) -> Self {
            Self {
                value,
                marker: PhantomData,
            }
        }
    }

    impl<'de, T, E> de::Deserializer<'de> for OptionDeserializer<T, E>
    where
        T: de::IntoDeserializer<'de, E>,
        E: de::Error,
    {
        type Error = E;

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
            string bytes byte_buf option unit unit_struct newtype_struct seq
            tuple tuple_struct map struct enum identifier ignored_any
        }

        fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            match self.value {
                Some(value) => visitor.visit_some(value.into_deserializer()),
                None => visitor.visit_none(),
            }
        }
    }

    #[test]
    fn value_try_into() {
        Value::Integer(2).try_into::<i32>().unwrap();
        Value::String("hi".to_string())
            .try_into::<i32>()
            .unwrap_err();
    }

    #[test]
    fn unexpected_from_type() {
        let string = Type::String;
        let unexp = de::Unexpected::from(string);

        assert_eq!(unexp.to_string(), "string");
    }

    #[test]
    fn value_into_deserializer() {
        let value = Value::String("foo".to_string());
        let deserializer = value.clone().into_deserializer();

        assert_eq!(deserializer, value);
    }

    #[test]
    fn value_ref_into_deserializer() {
        let value = Value::String("foo".to_string());
        let deserializer = (&value).into_deserializer();

        assert_eq!(deserializer, &value);
    }

    #[test]
    fn value_deserialize_primitive() {
        let value = Value::deserialize(de::value::BoolDeserializer::<Error>::new(true)).unwrap();
        assert_eq!(value, Value::Boolean(true));

        let value = Value::deserialize(de::value::I64Deserializer::<Error>::new(123)).unwrap();
        assert_eq!(value, Value::Integer(123));

        let value = Value::deserialize(de::value::I128Deserializer::<Error>::new(123)).unwrap();
        assert_eq!(value, Value::Integer(123));
        let result = Value::deserialize(de::value::I128Deserializer::<Error>::new(i128::MIN));
        assert_matches!(result, Err(Error(ErrorKind::InvalidValue(..))));

        let value = Value::deserialize(de::value::U64Deserializer::<Error>::new(123)).unwrap();
        assert_eq!(value, Value::Integer(123));
        let result = Value::deserialize(de::value::U64Deserializer::<Error>::new(u64::MAX));
        assert_matches!(result, Err(Error(ErrorKind::InvalidValue(..))));

        let value = Value::deserialize(de::value::U128Deserializer::<Error>::new(123)).unwrap();
        assert_eq!(value, Value::Integer(123));
        let result = Value::deserialize(de::value::U128Deserializer::<Error>::new(u128::MAX));
        assert_matches!(result, Err(Error(ErrorKind::InvalidValue(..))));

        let value = Value::deserialize(de::value::F64Deserializer::<Error>::new(123.0)).unwrap();
        assert_eq!(value, Value::Float(123.0));

        let value = Value::deserialize(de::value::StrDeserializer::<Error>::new(
            "Does it smell like updog in here?",
        ))
        .unwrap();
        assert_eq!(
            value,
            Value::String("Does it smell like updog in here?".to_string())
        );

        let value = Value::deserialize(de::value::StringDeserializer::<Error>::new(
            "No, what's updog?".into(),
        ))
        .unwrap();
        assert_eq!(value, Value::String("No, what's updog?".to_string()));

        let value = Value::deserialize(OptionDeserializer::<_, Error>::new(Some(123))).unwrap();
        assert_eq!(value, Value::Integer(123));

        let result = Value::deserialize(OptionDeserializer::<i32, Error>::new(None));
        assert_matches!(result, Err(Error(ErrorKind::InvalidType(..))));

        let result = Value::deserialize(de::value::UnitDeserializer::<Error>::new());
        assert_matches!(result, Err(Error(ErrorKind::InvalidType(..))));
    }

    #[test]
    fn value_deserialize_array() {
        let value =
            Value::deserialize(de::value::BytesDeserializer::<Error>::new(&[1, 2, 3])).unwrap();
        assert_eq!(
            value,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );

        let value = Value::deserialize(de::value::SeqDeserializer::<_, Error>::new(
            [1, 2, 3].into_iter().map(de::value::I64Deserializer::new),
        ))
        .unwrap();
        assert_eq!(
            value,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
    }

    #[test]
    fn value_deserialize_table() {
        let value = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
            [("one", 1), ("two", 2), ("three", 3)]
                .into_iter()
                .map(|(k, v)| {
                    (
                        de::value::StrDeserializer::new(k),
                        de::value::I64Deserializer::new(v),
                    )
                }),
        ))
        .unwrap();
        assert_eq!(
            value,
            Value::Table(btreemap! {
                "one".to_string() => Value::Integer(1),
                "two".to_string() => Value::Integer(2),
                "three".to_string() => Value::Integer(3),
            })
        );

        let value = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
            [("one", 1), ("two", 2), ("three", 3)]
                .into_iter()
                .map(|(k, v)| {
                    (
                        de::value::BorrowedStrDeserializer::new(k),
                        de::value::I64Deserializer::new(v),
                    )
                }),
        ))
        .unwrap();
        assert_eq!(
            value,
            Value::Table(btreemap! {
                "one".to_string() => Value::Integer(1),
                "two".to_string() => Value::Integer(2),
                "three".to_string() => Value::Integer(3),
            })
        );

        let value = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
            iter::empty::<(de::value::StrDeserializer<_>, de::value::I64Deserializer<_>)>(),
        ))
        .unwrap();
        assert_eq!(value, Value::Table(btreemap! {}));

        let result = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(iter::once((
            de::value::I64Deserializer::new(123),
            de::value::StrDeserializer::new("foo"),
        ))));
        assert_matches!(result, Err(Error(ErrorKind::InvalidType(..))));
    }

    #[cfg(feature = "datetime")]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn value_deserialize_datetime() {
        let tests = [
            (
                OffsetDatetime::WRAPPER_FIELD,
                OffsetDatetime::EXAMPLE_BYTES,
                Datetime::EXAMPLE_OFFSET_DATETIME,
            ),
            (
                LocalDatetime::WRAPPER_FIELD,
                LocalDatetime::EXAMPLE_BYTES,
                Datetime::EXAMPLE_LOCAL_DATETIME,
            ),
            (
                LocalDate::WRAPPER_FIELD,
                LocalDate::EXAMPLE_BYTES,
                Datetime::EXAMPLE_LOCAL_DATE,
            ),
            (
                LocalTime::WRAPPER_FIELD,
                LocalTime::EXAMPLE_BYTES,
                Datetime::EXAMPLE_LOCAL_TIME,
            ),
        ];

        for (field, bytes, expected) in tests {
            let value =
                Value::deserialize(de::value::MapDeserializer::<_, Error>::new(iter::once((
                    de::value::StrDeserializer::new(field),
                    de::value::BytesDeserializer::new(bytes),
                ))))
                .unwrap();
            assert_eq!(value, expected);

            let value = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
                iter::once((field.to_string(), de::value::BytesDeserializer::new(bytes))),
            ))
            .unwrap();
            assert_eq!(value, expected);

            let value =
                Value::deserialize(de::value::MapDeserializer::<_, Error>::new(iter::once((
                    de::value::BorrowedStrDeserializer::new(field),
                    de::value::BytesDeserializer::new(bytes),
                ))))
                .unwrap();
            assert_eq!(value, expected);
        }

        let tests = [
            (OffsetDatetime::WRAPPER_FIELD, OffsetDatetime::EXAMPLE_BYTES),
            (LocalDatetime::WRAPPER_FIELD, LocalDatetime::EXAMPLE_BYTES),
            (LocalDate::WRAPPER_FIELD, LocalDate::EXAMPLE_BYTES),
            (LocalTime::WRAPPER_FIELD, LocalTime::EXAMPLE_BYTES),
        ];

        for (i, (field, bytes)) in tests.into_iter().enumerate() {
            let result = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
                [
                    (
                        de::value::BorrowedStrDeserializer::new(field),
                        de::value::BytesDeserializer::new(bytes),
                    ),
                    (
                        de::value::BorrowedStrDeserializer::new(field),
                        de::value::BytesDeserializer::new(bytes),
                    ),
                ]
                .into_iter(),
            ));
            assert_matches!(result, Err(Error(ErrorKind::DuplicateField(..))));

            let result = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
                [
                    (
                        de::value::StrDeserializer::new(field),
                        de::value::BytesDeserializer::new(bytes),
                    ),
                    (
                        de::value::StrDeserializer::new("foo"),
                        de::value::BytesDeserializer::new(b"bar"),
                    ),
                ]
                .into_iter(),
            ));
            assert_matches!(result, Err(Error(ErrorKind::UnknownField(..))));

            let result = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
                [
                    (
                        de::value::BorrowedStrDeserializer::new(field),
                        de::value::BytesDeserializer::new(bytes),
                    ),
                    (
                        de::value::BorrowedStrDeserializer::new("foo"),
                        de::value::BytesDeserializer::new(b"bar"),
                    ),
                ]
                .into_iter(),
            ));
            assert_matches!(result, Err(Error(ErrorKind::UnknownField(..))));

            let other_field = tests[(i + 1) % tests.len()].0;
            let result = Value::deserialize(de::value::MapDeserializer::<_, Error>::new(
                [
                    (
                        de::value::StrDeserializer::new(field),
                        de::value::BytesDeserializer::new(bytes),
                    ),
                    (
                        de::value::StrDeserializer::new(other_field),
                        de::value::BytesDeserializer::new(b"bar"),
                    ),
                ]
                .into_iter(),
            ));
            assert_matches!(result, Err(Error(ErrorKind::UnknownField(..))));
        }
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn value_deserializer() {
        assert_matches!(String::deserialize(Value::String("Hello".to_string())), Ok(s) if s == "Hello");

        assert_matches!(i32::deserialize(Value::Integer(42)), Ok(42));

        assert_matches!(f64::deserialize(Value::Float(42.0)), Ok(42.0));

        assert_matches!(bool::deserialize(Value::Boolean(true)), Ok(true));

        assert_matches!(
            Vec::<i32>::deserialize(Value::Array(vec![
                Value::Integer(123),
                Value::Integer(456),
                Value::Integer(789),
            ])),
            Ok(a) if a == [123, 456, 789]
        );

        assert_matches!(
            HashMap::<String, i32>::deserialize(Value::Table(btreemap! {
                "abc".into() => Value::Integer(123),
                "def".into() => Value::Integer(456),
                "ghi".into() => Value::Integer(789),
            })),
            Ok(t) if t == hashmap! { "abc".into() => 123, "def".into() => 456, "ghi".into() => 789 }
        );

        assert_matches!(
            Option::<i32>::deserialize(Value::Integer(123)),
            Ok(Some(123))
        );
    }

    #[test]
    fn value_deserializer_newtype() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Newtype(i32);

        let result = Newtype::deserialize(Value::Integer(123)).unwrap();
        assert_eq!(result, Newtype(123));
    }

    #[test]
    fn value_deserializer_enum() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        enum Enum {
            A,
            B(i32),
        }

        let result = Enum::deserialize(Value::String("A".to_string())).unwrap();
        assert_eq!(result, Enum::A);

        let result = Enum::deserialize(Value::Table(btreemap! {
            "B".into() => Value::Integer(123),
        }))
        .unwrap();
        assert_eq!(result, Enum::B(123));

        assert_matches!(
            Enum::deserialize(Value::Integer(123)),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn value_deserializer_datetime() {
        let result =
            Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_OFFSET_DATETIME);

        let result =
            OffsetDatetime::deserialize(Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME))
                .unwrap();
        assert_eq!(result, OffsetDatetime::EXAMPLE);

        let result =
            Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_LOCAL_DATETIME);

        let result =
            LocalDatetime::deserialize(Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME)).unwrap();
        assert_eq!(result, LocalDatetime::EXAMPLE);

        let result = Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_LOCAL_DATE);

        let result = LocalDate::deserialize(Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE)).unwrap();
        assert_eq!(result, LocalDate::EXAMPLE);

        let result = Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_LOCAL_TIME);

        let result = LocalTime::deserialize(Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME)).unwrap();
        assert_eq!(result, LocalTime::EXAMPLE);

        assert_matches!(
            Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_INVALID_1)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        assert_matches!(
            Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_INVALID_2)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        assert_matches!(
            Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_INVALID_3)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        assert_matches!(
            Datetime::deserialize(Value::Datetime(Datetime::EXAMPLE_INVALID_4)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn seq_access() {
        let mut seq_access = SeqAccess::new(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);

        assert_eq!(seq_access.size_hint(), Some(3));

        assert_matches!(seq_access.next_element::<i32>(), Ok(Some(1)));
        assert_matches!(seq_access.next_element::<i32>(), Ok(Some(2)));
        assert_matches!(seq_access.next_element::<i32>(), Ok(Some(3)));

        assert_eq!(seq_access.size_hint(), Some(0));

        assert_matches!(seq_access.next_element::<i32>(), Ok(None));

        assert_eq!(seq_access.size_hint(), Some(0));
    }

    #[test]
    fn map_access() {
        let mut map_access = MapAccess::new(btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });

        assert_eq!(map_access.size_hint(), Some(3));

        assert_matches!(map_access.next_key::<String>(), Ok(Some(k)) if k == "one");
        assert_matches!(map_access.next_value::<i32>(), Ok(1));
        assert_matches!(map_access.next_key::<String>(), Ok(Some(k)) if k == "three");
        assert_matches!(map_access.next_value::<i32>(), Ok(3));
        assert_matches!(map_access.next_key::<String>(), Ok(Some(k)) if k == "two");
        assert_matches!(map_access.next_value::<i32>(), Ok(2));

        assert_eq!(map_access.size_hint(), Some(0));

        assert_matches!(map_access.next_key::<String>(), Ok(None));

        assert_eq!(map_access.size_hint(), Some(0));

        let mut map_access = MapAccess::new(btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });

        assert_eq!(map_access.size_hint(), Some(3));

        assert_matches!(
            map_access.next_entry::<String, i32>(),
            Ok(Some((k, v))) if k == "one" && v == 1
        );
        assert_matches!(
            map_access.next_entry::<String, i32>(),
            Ok(Some((k, v))) if k == "three" && v == 3
        );
        assert_matches!(
            map_access.next_entry::<String, i32>(),
            Ok(Some((k, v))) if k == "two" && v == 2
        );

        assert_eq!(map_access.size_hint(), Some(0));

        assert_matches!(map_access.next_entry::<String, i32>(), Ok(None));

        assert_eq!(map_access.size_hint(), Some(0));
    }

    #[test]
    #[should_panic = "MapAccess::next_value called without calling MapAccess::next_key first"]
    fn map_access_empty() {
        let mut map_access = MapAccess::new(btreemap! {});

        let _result = map_access.next_value::<i32>();
    }

    #[test]
    fn enum_access_unit() {
        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Table(btreemap! {}),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert!(value.unit_variant().is_ok());

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Integer(42),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(value.unit_variant(), Err(Error(ErrorKind::InvalidType(..))));

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Table(btreemap! {
                "foo".to_string() => Value::Integer(42),
            }),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(
            value.unit_variant(),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn enum_access_newtype() {
        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Integer(42),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(value.newtype_variant::<i32>(), Ok(42));
    }

    #[test]
    fn enum_access_tuple() {
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

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ]),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(value.tuple_variant(3, Visitor), Ok(s) if s == [1, 2, 3]);
    }

    #[test]
    fn enum_access_struct() {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = HashMap<String, i32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a struct")
            }

            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut result = HashMap::new();
                while let Some((key, value)) = map.next_entry::<String, i32>()? {
                    result.insert(key, value);
                }
                Ok(result)
            }
        }

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Table(btreemap! {
                "one".to_string() => Value::Integer(1),
                "two".to_string() => Value::Integer(2),
            }),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(
            value.struct_variant(&["one", "two"], Visitor),
            Ok(t) if t == hashmap! {
                "one".to_string() => 1,
                "two".to_string() => 2,
            }
        );
    }

    #[test]
    fn enum_access_error() {
        let enum_access = EnumAccess::new(btreemap! {});
        assert_matches!(enum_access, Err(Error(ErrorKind::InvalidValue(..))));

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Integer(1),
            "variant2".to_string() => Value::Integer(2),
        });
        assert_matches!(enum_access, Err(Error(ErrorKind::InvalidValue(..))));
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn value_ref_deserializer() {
        assert_matches!(String::deserialize(&Value::String("Hello".to_string())), Ok(s) if s == "Hello");

        assert_matches!(i32::deserialize(&Value::Integer(42)), Ok(42));

        assert_matches!(f64::deserialize(&Value::Float(42.0)), Ok(42.0));

        assert_matches!(bool::deserialize(&Value::Boolean(true)), Ok(true));

        assert_matches!(
            Vec::<i32>::deserialize(&Value::Array(vec![
                Value::Integer(123),
                Value::Integer(456),
                Value::Integer(789),
            ])),
            Ok(a) if a == [123, 456, 789]
        );

        assert_matches!(
            HashMap::<String, i32>::deserialize(&Value::Table(btreemap! {
                "abc".into() => Value::Integer(123),
                "def".into() => Value::Integer(456),
                "ghi".into() => Value::Integer(789),
            })),
            Ok(a) if a == hashmap! { "abc".into() => 123, "def".into() => 456, "ghi".into() => 789 }
        );

        assert_matches!(
            Option::<i32>::deserialize(&Value::Integer(123)),
            Ok(Some(123))
        );
    }

    #[test]
    fn value_ref_deserializer_newtype() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Newtype(i32);

        let result = Newtype::deserialize(&Value::Integer(123)).unwrap();
        assert_eq!(result, Newtype(123));
    }

    #[test]
    fn value_ref_deserializer_enum() {
        #[derive(Debug, Deserialize, PartialEq, Eq)]
        enum Enum {
            A,
            B(i32),
        }

        let result = Enum::deserialize(&Value::String("A".to_string())).unwrap();
        assert_eq!(result, Enum::A);

        let result = Enum::deserialize(&Value::Table(btreemap! {
            "B".into() => Value::Integer(123),
        }))
        .unwrap();
        assert_eq!(result, Enum::B(123));

        assert_matches!(
            Enum::deserialize(&Value::Integer(123)),
            Err(Error(ErrorKind::InvalidType(..)))
        );
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn value_ref_deserializer_datetime() {
        let result =
            Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_OFFSET_DATETIME);

        let result =
            OffsetDatetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME))
                .unwrap();
        assert_eq!(result, OffsetDatetime::EXAMPLE);

        let result =
            Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_LOCAL_DATETIME);

        let result =
            LocalDatetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME)).unwrap();
        assert_eq!(result, LocalDatetime::EXAMPLE);

        let result = Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_LOCAL_DATE);

        let result =
            LocalDate::deserialize(&Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE)).unwrap();
        assert_eq!(result, LocalDate::EXAMPLE);

        let result = Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME)).unwrap();
        assert_eq!(result, Datetime::EXAMPLE_LOCAL_TIME);

        let result =
            LocalTime::deserialize(&Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME)).unwrap();
        assert_eq!(result, LocalTime::EXAMPLE);

        assert_matches!(
            Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_INVALID_1)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        assert_matches!(
            Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_INVALID_2)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        assert_matches!(
            Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_INVALID_3)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        assert_matches!(
            Datetime::deserialize(&Value::Datetime(Datetime::EXAMPLE_INVALID_4)),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn seq_ref_access() {
        let array = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        let mut seq_access = SeqRefAccess::new(&array);

        assert_eq!(seq_access.size_hint(), Some(3));

        assert_matches!(seq_access.next_element::<i32>(), Ok(Some(1)));
        assert_matches!(seq_access.next_element::<i32>(), Ok(Some(2)));
        assert_matches!(seq_access.next_element::<i32>(), Ok(Some(3)));

        assert_eq!(seq_access.size_hint(), Some(0));

        assert_matches!(seq_access.next_element::<i32>(), Ok(None));

        assert_eq!(seq_access.size_hint(), Some(0));
    }

    #[test]
    fn map_ref_access() {
        let table = btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        };
        let mut map_access = MapRefAccess::new(&table);

        assert_eq!(map_access.size_hint(), Some(3));

        assert_matches!(map_access.next_key::<String>(), Ok(Some(k)) if k == "one");
        assert_matches!(map_access.next_value::<i32>(), Ok(1));
        assert_matches!(map_access.next_key::<String>(), Ok(Some(k)) if k == "three");
        assert_matches!(map_access.next_value::<i32>(), Ok(3));
        assert_matches!(map_access.next_key::<String>(), Ok(Some(k)) if k == "two");
        assert_matches!(map_access.next_value::<i32>(), Ok(2));

        assert_eq!(map_access.size_hint(), Some(0));

        assert_matches!(map_access.next_key::<String>(), Ok(None));

        assert_eq!(map_access.size_hint(), Some(0));

        let table = btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        };
        let mut map_access = MapRefAccess::new(&table);

        assert_eq!(map_access.size_hint(), Some(3));

        assert_matches!(
            map_access.next_entry::<String, i32>(),
            Ok(Some((k, v))) if k == "one" && v == 1
        );
        assert_matches!(
            map_access.next_entry::<String, i32>(),
            Ok(Some((k, v))) if k == "three" && v == 3
        );
        assert_matches!(
            map_access.next_entry::<String, i32>(),
            Ok(Some((k, v))) if k == "two" && v == 2
        );

        assert_eq!(map_access.size_hint(), Some(0));

        assert_matches!(map_access.next_entry::<String, i32>(), Ok(None));

        assert_eq!(map_access.size_hint(), Some(0));
    }

    #[test]
    #[should_panic = "MapRefAccess::next_value called without calling MapRefAccess::next_key first"]
    fn map_ref_access_empty() {
        let table = btreemap! {};
        let mut map_access = MapRefAccess::new(&table);

        let _result = map_access.next_value::<i32>();
    }

    #[test]
    fn enum_ref_access_unit() {
        let table = btreemap! {
            "variant".to_string() => Value::Table(btreemap! {}),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert!(value.unit_variant().is_ok());

        let table = btreemap! {
            "variant".to_string() => Value::Integer(42),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(value.unit_variant(), Err(Error(ErrorKind::InvalidType(..))));

        let table = btreemap! {
            "variant".to_string() => Value::Table(btreemap! {
                "foo".to_string() => Value::Integer(42),
            }),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(
            value.unit_variant(),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn enum_ref_access_newtype() {
        let table = btreemap! {
            "variant".to_string() => Value::Integer(42),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(value.newtype_variant::<i32>(), Ok(42));
    }

    #[test]
    fn enum_ref_access_tuple() {
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

        let table = btreemap! {
            "variant".to_string() => Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ]),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_matches!(value.tuple_variant(3, Visitor), Ok(s) if s == [1, 2, 3]);
    }

    #[test]
    fn enum_ref_access_struct() {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = HashMap<String, i32>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a struct")
            }

            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut result = HashMap::new();
                while let Some((key, value)) = map.next_entry::<String, i32>()? {
                    result.insert(key, value);
                }
                Ok(result)
            }
        }

        let table = btreemap! {
            "variant".to_string() => Value::Table(btreemap! {
                "one".to_string() => Value::Integer(1),
                "two".to_string() => Value::Integer(2),
            }),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant");
        assert_matches!(
            value.struct_variant(&["one", "two"], Visitor),
            Ok(t) if t == hashmap! {
                "one".to_string() => 1,
                "two".to_string() => 2,
            }
        );
    }

    #[test]
    fn enum_ref_access_error() {
        let table = btreemap! {};
        let enum_access = EnumRefAccess::new(&table);
        assert_matches!(enum_access, Err(Error(ErrorKind::InvalidValue(..))));

        let table = btreemap! {
            "variant".to_string() => Value::Integer(1),
            "variant2".to_string() => Value::Integer(2),
        };
        let enum_access = EnumRefAccess::new(&table);
        assert_matches!(enum_access, Err(Error(ErrorKind::InvalidValue(..))));
    }
}
