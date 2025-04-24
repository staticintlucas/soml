use std::borrow::Cow;
use std::result::Result as StdResult;
use std::{fmt, slice, vec};

use serde::de;
use serde::de::{Error as _, IntoDeserializer as _};

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
        #[derive(Debug)]
        enum MapField<'de> {
            OffsetDatetime,
            LocalDatetime,
            LocalDate,
            LocalTime,
            Other(Cow<'de, str>),
        }
        struct MapFieldVisitor;

        impl MapField<'_> {
            #[inline]
            fn as_str(&self) -> &str {
                match *self {
                    Self::OffsetDatetime => OffsetDatetime::WRAPPER_FIELD,
                    Self::LocalDatetime => LocalDatetime::WRAPPER_FIELD,
                    Self::LocalDate => LocalDate::WRAPPER_FIELD,
                    Self::LocalTime => LocalTime::WRAPPER_FIELD,
                    Self::Other(ref field) => field.as_ref(),
                }
            }
        }

        impl<'de> de::Visitor<'de> for MapFieldVisitor {
            type Value = MapField<'de>;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a TOML key")
            }

            #[inline]
            fn visit_borrowed_str<E>(self, value: &'de str) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    OffsetDatetime::WRAPPER_FIELD => Ok(Self::Value::OffsetDatetime),
                    LocalDatetime::WRAPPER_FIELD => Ok(Self::Value::LocalDatetime),
                    LocalDate::WRAPPER_FIELD => Ok(Self::Value::LocalDate),
                    LocalTime::WRAPPER_FIELD => Ok(Self::Value::LocalTime),
                    _ => Ok(Self::Value::Other(Cow::Borrowed(value))),
                }
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                match value.as_str() {
                    OffsetDatetime::WRAPPER_FIELD => Ok(Self::Value::OffsetDatetime),
                    LocalDatetime::WRAPPER_FIELD => Ok(Self::Value::LocalDatetime),
                    LocalDate::WRAPPER_FIELD => Ok(Self::Value::LocalDate),
                    LocalTime::WRAPPER_FIELD => Ok(Self::Value::LocalTime),
                    _ => Ok(Self::Value::Other(Cow::Owned(value))),
                }
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_string(value.to_owned())
            }
        }

        impl<'de> de::Deserialize<'de> for MapField<'de> {
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
                self.visit_i64(
                    value
                        .try_into()
                        .map_err(|_| de::Error::custom("integer out of range of i64"))?,
                )
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i64(
                    value
                        .try_into()
                        .map_err(|_| de::Error::custom("integer out of range of i64"))?,
                )
            }

            #[inline]
            fn visit_u128<E>(self, value: u128) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_i64(
                    value
                        .try_into()
                        .map_err(|_| de::Error::custom("integer out of range of i64"))?,
                )
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
                let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(element) = seq.next_element()? {
                    result.push(element);
                }
                Ok(Self::Value::Array(result))
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(key) = map.next_key::<MapField<'de>>()? else {
                    return Ok(Self::Value::Table(Table::new()));
                };

                match key {
                    MapField::OffsetDatetime => {
                        let result = map.next_value::<EncodedOffsetDatetime>()?.0.into();
                        match map.next_key::<MapField<'de>>()? {
                            Some(MapField::OffsetDatetime) => {
                                Err(de::Error::duplicate_field(OffsetDatetime::WRAPPER_FIELD))
                            }
                            Some(key) => Err(de::Error::unknown_field(
                                key.as_str(),
                                &[OffsetDatetime::WRAPPER_FIELD],
                            )),
                            None => Ok(Self::Value::Datetime(result)),
                        }
                    }
                    MapField::LocalDatetime => {
                        let result = map.next_value::<EncodedLocalDatetime>()?.0.into();
                        match map.next_key::<MapField<'de>>()? {
                            Some(MapField::LocalDatetime) => {
                                Err(de::Error::duplicate_field(LocalDatetime::WRAPPER_FIELD))
                            }
                            Some(key) => Err(de::Error::unknown_field(
                                key.as_str(),
                                &[LocalDatetime::WRAPPER_FIELD],
                            )),
                            None => Ok(Self::Value::Datetime(result)),
                        }
                    }
                    MapField::LocalDate => {
                        let result = map.next_value::<EncodedLocalDate>()?.0.into();
                        match map.next_key::<MapField<'de>>()? {
                            Some(MapField::LocalDate) => {
                                Err(de::Error::duplicate_field(LocalDate::WRAPPER_FIELD))
                            }
                            Some(key) => Err(de::Error::unknown_field(
                                key.as_str(),
                                &[LocalDate::WRAPPER_FIELD],
                            )),
                            None => Ok(Self::Value::Datetime(result)),
                        }
                    }
                    MapField::LocalTime => {
                        let result = map.next_value::<EncodedLocalTime>()?.0.into();
                        match map.next_key::<MapField<'de>>()? {
                            Some(MapField::LocalTime) => {
                                Err(de::Error::duplicate_field(LocalTime::WRAPPER_FIELD))
                            }
                            Some(key) => Err(de::Error::unknown_field(
                                key.as_str(),
                                &[LocalTime::WRAPPER_FIELD],
                            )),
                            None => Ok(Self::Value::Datetime(result)),
                        }
                    }
                    MapField::Other(key) => {
                        let mut result = Table::new();
                        result.insert(key.into_owned(), map.next_value()?);
                        while let Some((key, value)) = map.next_entry()? {
                            result.insert(key, value);
                        }
                        Ok(Self::Value::Table(result))
                    }
                }
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
            Self::Datetime(datetime) => match datetime.try_into()? {
                AnyDatetime::OffsetDatetime(datetime) => {
                    visitor.visit_map(OffsetDatetimeAccess::from(datetime))
                }
                AnyDatetime::LocalDatetime(datetime) => {
                    visitor.visit_map(LocalDatetimeAccess::from(datetime))
                }
                AnyDatetime::LocalDate(date) => visitor.visit_map(LocalDateAccess::from(date)),
                AnyDatetime::LocalTime(time) => visitor.visit_map(LocalTimeAccess::from(time)),
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
        seed.deserialize(value)
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

struct EnumAccess {
    variant: String,
    value: Value,
}

impl EnumAccess {
    #[inline]
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
            Value::Datetime(ref datetime) => {
                // Unfortunately we have to convert to a string here before re-parsing it in the
                // deserialize impl because serde doesn't have a way to pass the datetime struct
                // through directly
                match (
                    datetime.date.clone(),
                    datetime.time.clone(),
                    datetime.offset.clone(),
                ) {
                    (Some(date), Some(time), Some(offset)) => {
                        visitor.visit_map(OffsetDatetimeAccess::from(OffsetDatetime {
                            date,
                            time,
                            offset,
                        }))
                    }
                    (Some(date), Some(time), None) => {
                        visitor.visit_map(LocalDatetimeAccess::from(LocalDatetime { date, time }))
                    }
                    (Some(date), None, None) => visitor.visit_map(LocalDateAccess::from(date)),
                    (None, Some(time), None) => visitor.visit_map(LocalTimeAccess::from(time)),
                    _ => Err(Error::invalid_value(
                        de::Unexpected::Other(datetime.type_str()),
                        &"a valid data-time",
                    )),
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

struct EnumRefAccess<'de> {
    variant: &'de str,
    value: &'de Value,
}

impl<'de> EnumRefAccess<'de> {
    #[inline]
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

    use maplit::{btreemap, hashmap};
    use serde::de::{EnumAccess as _, MapAccess as _, SeqAccess as _, VariantAccess as _};
    use serde::Deserialize;

    use super::*;
    use crate::value::{Datetime, Offset};

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
        assert!(result.is_err());

        let value = Value::deserialize(de::value::U64Deserializer::<Error>::new(123)).unwrap();
        assert_eq!(value, Value::Integer(123));
        let result = Value::deserialize(de::value::U64Deserializer::<Error>::new(u64::MAX));
        assert!(result.is_err());

        let value = Value::deserialize(de::value::U128Deserializer::<Error>::new(123)).unwrap();
        assert_eq!(value, Value::Integer(123));
        let result = Value::deserialize(de::value::U128Deserializer::<Error>::new(u128::MAX));
        assert!(result.is_err());

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
        assert!(result.is_err());

        let result = Value::deserialize(de::value::UnitDeserializer::<Error>::new());
        assert!(result.is_err());
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
        assert!(result.is_err());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn value_deserialize_datetime() {
        let date = || LocalDate {
            year: 2023,
            month: 1,
            day: 2,
        };
        let time = || LocalTime {
            hour: 3,
            minute: 4,
            second: 5,
            nanosecond: 6_000_000,
        };
        let offset = || Offset::Custom { minutes: 428 };

        let tests = [
            (
                OffsetDatetime::WRAPPER_FIELD,
                &b"\x80\x8D\x5B\x00\x03\x04\x05\x00\xE7\x07\x01\x02\xAC\x01"[..],
                Datetime {
                    date: Some(date()),
                    time: Some(time()),
                    offset: Some(offset()),
                },
            ),
            (
                LocalDatetime::WRAPPER_FIELD,
                &b"\x80\x8D\x5B\x00\x03\x04\x05\x00\xE7\x07\x01\x02"[..],
                Datetime {
                    date: Some(date()),
                    time: Some(time()),
                    offset: None,
                },
            ),
            (
                LocalDate::WRAPPER_FIELD,
                &b"\xE7\x07\x01\x02"[..],
                Datetime {
                    date: Some(date()),
                    time: None,
                    offset: None,
                },
            ),
            (
                LocalTime::WRAPPER_FIELD,
                &b"\x80\x8D\x5B\x00\x03\x04\x05\x00"[..],
                Datetime {
                    date: None,
                    time: Some(time()),
                    offset: None,
                },
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

            let value =
                Value::deserialize(de::value::MapDeserializer::<_, Error>::new(iter::once((
                    de::value::BorrowedStrDeserializer::new(field),
                    de::value::BytesDeserializer::new(bytes),
                ))))
                .unwrap();
            assert_eq!(value, expected);
        }

        let tests = [
            (
                OffsetDatetime::WRAPPER_FIELD,
                &b"\x80\x8D\x5B\x00\x03\x04\x05\x00\xE7\x07\x01\x02\xAC\x01"[..],
            ),
            (
                LocalDatetime::WRAPPER_FIELD,
                &b"\x80\x8D\x5B\x00\x03\x04\x05\x00\xE7\x07\x01\x02"[..],
            ),
            (LocalDate::WRAPPER_FIELD, &b"\xE7\x07\x01\x02"[..]),
            (
                LocalTime::WRAPPER_FIELD,
                &b"\x80\x8D\x5B\x00\x03\x04\x05\x00"[..],
            ),
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
            assert!(result.is_err());

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
            assert!(result.is_err());

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
            assert!(result.is_err());

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
            assert!(result.is_err());
        }
    }

    #[test]
    fn value_deserializer() {
        String::deserialize(Value::String("Hello".to_string())).unwrap();

        i32::deserialize(Value::Integer(42)).unwrap();

        f64::deserialize(Value::Float(42.0)).unwrap();

        bool::deserialize(Value::Boolean(true)).unwrap();

        Vec::<i32>::deserialize(Value::Array(vec![
            Value::Integer(123),
            Value::Integer(456),
            Value::Integer(789),
        ]))
        .unwrap();

        HashMap::<String, i32>::deserialize(Value::Table(btreemap! {
            "abc".into() => Value::Integer(123),
            "def".into() => Value::Integer(456),
            "ghi".into() => Value::Integer(789),
        }))
        .unwrap();

        Option::<i32>::deserialize(Value::Integer(123)).unwrap();
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

        Enum::deserialize(Value::Integer(123)).unwrap_err();
    }

    #[test]
    fn value_deserializer_datetime() {
        let date = || LocalDate {
            year: 2023,
            month: 1,
            day: 2,
        };
        let time = || LocalTime {
            hour: 3,
            minute: 4,
            second: 5,
            nanosecond: 6_000_000,
        };
        let offset = || Offset::Custom { minutes: 428 };

        let datetime = Datetime {
            date: Some(date()),
            time: Some(time()),
            offset: Some(offset()),
        };
        let result = Datetime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = OffsetDatetime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: Some(date()),
            time: Some(time()),
            offset: None,
        };
        let result = Datetime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = LocalDatetime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: Some(date()),
            time: None,
            offset: None,
        };
        let result = Datetime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = LocalDate::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: None,
            time: Some(time()),
            offset: None,
        };
        let result = Datetime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = LocalTime::deserialize(Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: None,
            time: None,
            offset: Some(offset()),
        };
        Datetime::deserialize(Value::Datetime(datetime)).unwrap_err();

        let datetime = Datetime {
            date: None,
            time: Some(time()),
            offset: Some(offset()),
        };
        Datetime::deserialize(Value::Datetime(datetime)).unwrap_err();

        let datetime = Datetime {
            date: Some(date()),
            time: None,
            offset: Some(offset()),
        };
        Datetime::deserialize(Value::Datetime(datetime)).unwrap_err();

        let datetime = Datetime {
            date: None,
            time: None,
            offset: None,
        };
        Datetime::deserialize(Value::Datetime(datetime)).unwrap_err();
    }

    #[test]
    fn seq_access() {
        let mut seq_access = SeqAccess::new(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);

        assert_eq!(seq_access.size_hint(), Some(3));

        assert_eq!(seq_access.next_element::<i32>().unwrap(), Some(1));
        assert_eq!(seq_access.next_element::<i32>().unwrap(), Some(2));
        assert_eq!(seq_access.next_element::<i32>().unwrap(), Some(3));

        assert_eq!(seq_access.size_hint(), Some(0));

        assert_eq!(seq_access.next_element::<i32>().unwrap(), None);

        assert_eq!(seq_access.size_hint(), Some(0));
    }

    #[test]
    fn map_access() {
        let mut map_access = MapAccess::new(btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });

        assert_eq!(map_access.size_hint().unwrap(), 3);

        assert_eq!(map_access.next_key::<String>().unwrap().unwrap(), "one");
        assert_eq!(map_access.next_value::<i32>().unwrap(), 1);
        assert_eq!(map_access.next_key::<String>().unwrap().unwrap(), "three");
        assert_eq!(map_access.next_value::<i32>().unwrap(), 3);
        assert_eq!(map_access.next_key::<String>().unwrap().unwrap(), "two");
        assert_eq!(map_access.next_value::<i32>().unwrap(), 2);

        assert_eq!(map_access.size_hint().unwrap(), 0);

        assert!(map_access.next_key::<String>().unwrap().is_none());

        assert_eq!(map_access.size_hint().unwrap(), 0);

        let mut map_access = MapAccess::new(btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });

        assert_eq!(map_access.size_hint().unwrap(), 3);

        assert_eq!(
            map_access.next_entry::<String, i32>().unwrap().unwrap(),
            ("one".to_string(), 1)
        );
        assert_eq!(
            map_access.next_entry::<String, i32>().unwrap().unwrap(),
            ("three".to_string(), 3)
        );
        assert_eq!(
            map_access.next_entry::<String, i32>().unwrap().unwrap(),
            ("two".to_string(), 2)
        );

        assert_eq!(map_access.size_hint().unwrap(), 0);

        assert!(map_access.next_entry::<String, i32>().unwrap().is_none());

        assert_eq!(map_access.size_hint().unwrap(), 0);
    }

    #[test]
    #[should_panic = "MapAccess::next_value called without calling MapAccess::next_key first"]
    fn map_access_empty() {
        let mut map_access = MapAccess::new(btreemap! {});

        map_access.next_value::<i32>().unwrap();
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
        assert!(value.unit_variant().is_err());

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Table(btreemap! {
                "foo".to_string() => Value::Integer(42),
            }),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert!(value.unit_variant().is_err());
    }

    #[test]
    fn enum_access_newtype() {
        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Integer(42),
        })
        .unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_eq!(value.newtype_variant::<i32>().unwrap(), 42);
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
        assert_eq!(value.tuple_variant(3, Visitor).unwrap(), vec![1, 2, 3]);
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
        assert_eq!(
            value.struct_variant(&["one", "two"], Visitor).unwrap(),
            hashmap! {
                "one".to_string() => 1,
                "two".to_string() => 2,
            }
        );
    }

    #[test]
    fn enum_access_error() {
        let enum_access = EnumAccess::new(btreemap! {});
        assert!(enum_access.is_err());

        let enum_access = EnumAccess::new(btreemap! {
            "variant".to_string() => Value::Integer(1),
            "variant2".to_string() => Value::Integer(2),
        });
        assert!(enum_access.is_err());
    }

    #[test]
    fn value_ref_deserializer() {
        String::deserialize(&Value::String("Hello".to_string())).unwrap();

        i32::deserialize(&Value::Integer(42)).unwrap();

        f64::deserialize(&Value::Float(42.0)).unwrap();

        bool::deserialize(&Value::Boolean(true)).unwrap();

        Vec::<i32>::deserialize(&Value::Array(vec![
            Value::Integer(123),
            Value::Integer(456),
            Value::Integer(789),
        ]))
        .unwrap();

        HashMap::<String, i32>::deserialize(&Value::Table(btreemap! {
            "abc".into() => Value::Integer(123),
            "def".into() => Value::Integer(456),
            "ghi".into() => Value::Integer(789),
        }))
        .unwrap();

        Option::<i32>::deserialize(&Value::Integer(123)).unwrap();
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

        Enum::deserialize(&Value::Integer(123)).unwrap_err();
    }

    #[test]
    fn value_ref_deserializer_datetime() {
        let date = || LocalDate {
            year: 2023,
            month: 1,
            day: 2,
        };
        let time = || LocalTime {
            hour: 3,
            minute: 4,
            second: 5,
            nanosecond: 6_000_000,
        };
        let offset = || Offset::Custom { minutes: 428 };

        let datetime = Datetime {
            date: Some(date()),
            time: Some(time()),
            offset: Some(offset()),
        };
        let result = Datetime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = OffsetDatetime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: Some(date()),
            time: Some(time()),
            offset: None,
        };
        let result = Datetime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = LocalDatetime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: Some(date()),
            time: None,
            offset: None,
        };
        let result = Datetime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = LocalDate::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: None,
            time: Some(time()),
            offset: None,
        };
        let result = Datetime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime);

        let result = LocalTime::deserialize(&Value::Datetime(datetime.clone())).unwrap();
        assert_eq!(result, datetime.try_into().unwrap());

        let datetime = Datetime {
            date: None,
            time: None,
            offset: Some(offset()),
        };
        Datetime::deserialize(&Value::Datetime(datetime)).unwrap_err();

        let datetime = Datetime {
            date: None,
            time: Some(time()),
            offset: Some(offset()),
        };
        Datetime::deserialize(&Value::Datetime(datetime)).unwrap_err();

        let datetime = Datetime {
            date: Some(date()),
            time: None,
            offset: Some(offset()),
        };
        Datetime::deserialize(&Value::Datetime(datetime)).unwrap_err();

        let datetime = Datetime {
            date: None,
            time: None,
            offset: None,
        };
        Datetime::deserialize(&Value::Datetime(datetime)).unwrap_err();
    }

    #[test]
    fn seq_ref_access() {
        let array = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        let mut seq_access = SeqRefAccess::new(&array);

        assert_eq!(seq_access.size_hint(), Some(3));

        assert_eq!(seq_access.next_element::<i32>().unwrap(), Some(1));
        assert_eq!(seq_access.next_element::<i32>().unwrap(), Some(2));
        assert_eq!(seq_access.next_element::<i32>().unwrap(), Some(3));

        assert_eq!(seq_access.size_hint(), Some(0));

        assert_eq!(seq_access.next_element::<i32>().unwrap(), None);

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

        assert_eq!(map_access.size_hint().unwrap(), 3);

        assert_eq!(map_access.next_key::<String>().unwrap().unwrap(), "one");
        assert_eq!(map_access.next_value::<i32>().unwrap(), 1);
        assert_eq!(map_access.next_key::<String>().unwrap().unwrap(), "three");
        assert_eq!(map_access.next_value::<i32>().unwrap(), 3);
        assert_eq!(map_access.next_key::<String>().unwrap().unwrap(), "two");
        assert_eq!(map_access.next_value::<i32>().unwrap(), 2);

        assert_eq!(map_access.size_hint().unwrap(), 0);

        assert!(map_access.next_key::<String>().unwrap().is_none());

        assert_eq!(map_access.size_hint().unwrap(), 0);

        let table = btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        };
        let mut map_access = MapRefAccess::new(&table);

        assert_eq!(map_access.size_hint().unwrap(), 3);

        assert_eq!(
            map_access.next_entry::<String, i32>().unwrap().unwrap(),
            ("one".to_string(), 1)
        );
        assert_eq!(
            map_access.next_entry::<String, i32>().unwrap().unwrap(),
            ("three".to_string(), 3)
        );
        assert_eq!(
            map_access.next_entry::<String, i32>().unwrap().unwrap(),
            ("two".to_string(), 2)
        );

        assert_eq!(map_access.size_hint().unwrap(), 0);

        assert!(map_access.next_entry::<String, i32>().unwrap().is_none());

        assert_eq!(map_access.size_hint().unwrap(), 0);
    }

    #[test]
    #[should_panic = "MapRefAccess::next_value called without calling MapRefAccess::next_key first"]
    fn map_ref_access_empty() {
        let table = btreemap! {};
        let mut map_access = MapRefAccess::new(&table);

        map_access.next_value::<i32>().unwrap();
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
        assert!(value.unit_variant().is_err());

        let table = btreemap! {
            "variant".to_string() => Value::Table(btreemap! {
                "foo".to_string() => Value::Integer(42),
            }),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert!(value.unit_variant().is_err());
    }

    #[test]
    fn enum_ref_access_newtype() {
        let table = btreemap! {
            "variant".to_string() => Value::Integer(42),
        };
        let enum_access = EnumRefAccess::new(&table).unwrap();

        let (variant, value) = enum_access.variant::<String>().unwrap();
        assert_eq!(variant, "variant".to_string());
        assert_eq!(value.newtype_variant::<i32>().unwrap(), 42);
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
        assert_eq!(value.tuple_variant(3, Visitor).unwrap(), vec![1, 2, 3]);
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
        assert_eq!(variant, "variant".to_string());
        assert_eq!(
            value.struct_variant(&["one", "two"], Visitor).unwrap(),
            hashmap! {
                "one".to_string() => 1,
                "two".to_string() => 2,
            }
        );
    }

    #[test]
    fn enum_ref_access_error() {
        let table = btreemap! {};
        let enum_access = EnumRefAccess::new(&table);
        assert!(enum_access.is_err());

        let table = btreemap! {
            "variant".to_string() => Value::Integer(1),
            "variant2".to_string() => Value::Integer(2),
        };
        let enum_access = EnumRefAccess::new(&table);
        assert!(enum_access.is_err());
    }
}
