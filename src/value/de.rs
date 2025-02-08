use std::borrow::Cow;
use std::collections::{hash_map, HashMap};
use std::result::Result as StdResult;
use std::{fmt, slice, vec};

use serde::de;
use serde::de::{Error as _, IntoDeserializer as _};

use super::datetime::{
    DatetimeAccess, LocalDate, LocalDateFromBytes, LocalDatetime, LocalDatetimeFromBytes,
    LocalTime, LocalTimeFromBytes, OffsetDatetime, OffsetDatetimeFromBytes,
};
use super::{Type, Value};
use crate::de::{Error, Result};

impl Value {
    pub fn try_into<'de, T>(self) -> Result<T>
    where
        T: de::Deserialize<'de>,
    {
        T::deserialize(self)
    }
}

impl From<Type> for de::Unexpected<'_> {
    fn from(typ: Type) -> Self {
        de::Unexpected::Other(typ.to_str())
    }
}

impl de::IntoDeserializer<'_, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> de::IntoDeserializer<'de, Error> for &'de Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> de::Deserialize<'de> for Value {
    #[allow(clippy::too_many_lines)]
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        enum MapField<'de> {
            OffsetDatetime,
            LocalDatetime,
            LocalDate,
            LocalTime,
            Other(Cow<'de, str>),
        }
        struct MapFieldVisitor;

        impl MapField<'_> {
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

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a TOML key")
            }

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

            fn visit_str<E>(self, value: &str) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_string(value.to_owned())
            }
        }

        impl<'de> de::Deserialize<'de> for MapField<'de> {
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

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a TOML value")
            }

            fn visit_bool<E>(self, value: bool) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_i64<E>(self, value: i64) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

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

            fn visit_f64<E>(self, value: f64) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_str<E>(self, value: &str) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_string<E>(self, value: String) -> StdResult<Self::Value, E> {
                Ok(value.into())
            }

            fn visit_bytes<E>(self, value: &[u8]) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value.into())
            }

            fn visit_some<D>(self, deserializer: D) -> StdResult<Self::Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }

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

            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(key) = map.next_key::<MapField<'de>>()? else {
                    return Ok(Self::Value::Table(HashMap::new()));
                };

                match key {
                    MapField::OffsetDatetime => {
                        let result = map.next_value::<OffsetDatetimeFromBytes>()?.0.into();
                        match map.next_key::<MapField<'de>>()? {
                            Some(MapField::OffsetDatetime) => {
                                Err(de::Error::duplicate_field(OffsetDatetime::WRAPPER_FIELD))
                            }
                            Some(_) => Err(de::Error::unknown_field(
                                key.as_str(),
                                &[LocalDatetime::WRAPPER_FIELD],
                            )),
                            None => Ok(Self::Value::Datetime(result)),
                        }
                    }
                    MapField::LocalDatetime => {
                        let result = map.next_value::<LocalDatetimeFromBytes>()?.0.into();
                        match map.next_key::<MapField<'de>>()? {
                            Some(MapField::LocalDatetime) => {
                                Err(de::Error::duplicate_field(LocalDatetime::WRAPPER_FIELD))
                            }
                            Some(_) => Err(de::Error::unknown_field(
                                key.as_str(),
                                &[LocalDatetime::WRAPPER_FIELD],
                            )),
                            None => Ok(Self::Value::Datetime(result)),
                        }
                    }
                    MapField::LocalDate => {
                        let result = map.next_value::<LocalDateFromBytes>()?.0.into();
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
                        let result = map.next_value::<LocalTimeFromBytes>()?.0.into();
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
                        let mut result = HashMap::with_capacity(map.size_hint().unwrap_or(0));
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

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self {
            Self::String(str) => visitor.visit_string(str),
            Self::Integer(int) => visitor.visit_i64(int),
            Self::Float(float) => visitor.visit_f64(float),
            Self::Boolean(bool) => visitor.visit_bool(bool),
            Self::Datetime(datetime) => {
                // TODO can we avoid converting to string here?
                match (
                    datetime.date.as_ref(),
                    datetime.time.as_ref(),
                    datetime.offset.as_ref(),
                ) {
                    (Some(_), Some(_), Some(_)) => visitor.visit_map(
                        DatetimeAccess::offset_datetime(datetime.to_string().into_bytes()),
                    ),
                    (Some(_), Some(_), None) => visitor.visit_map(DatetimeAccess::local_datetime(
                        datetime.to_string().into_bytes(),
                    )),
                    (Some(_), None, None) => visitor.visit_map(DatetimeAccess::local_date(
                        datetime.to_string().into_bytes(),
                    )),
                    (None, Some(_), None) => visitor.visit_map(DatetimeAccess::local_time(
                        datetime.to_string().into_bytes(),
                    )),
                    _ => Err(Error::invalid_value(
                        de::Unexpected::Other(datetime.type_str()),
                        &"a valid data-time",
                    )),
                }
            }
            Self::Array(array) => visitor.visit_seq(SeqAccess::new(array)),
            Self::Table(table) => visitor.visit_map(MapAccess::new(table)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
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
    fn new(array: Vec<Value>) -> Self {
        Self {
            values: array.into_iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| seed.deserialize(value))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

struct MapAccess {
    kv_pairs: hash_map::IntoIter<String, Value>,
    next_value: Option<Value>,
}

impl MapAccess {
    fn new(table: HashMap<String, Value>) -> Self {
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
            panic!("next_value_seed called without calling next_key_seed first")
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

    fn size_hint(&self) -> Option<usize> {
        Some(self.kv_pairs.len())
    }
}

struct EnumAccess {
    variant: String,
    value: Value,
}

impl EnumAccess {
    fn new(table: HashMap<String, Value>) -> Result<Self> {
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

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // TODO toml-rs uses maps with integer keys instead of arrays for tuple variants. Do we need
        // to support this too?
        de::Deserializer::deserialize_seq(self.value, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.value, visitor)
    }
}

impl<'de> de::Deserializer<'de> for &'de Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match *self {
            Value::String(ref str) => visitor.visit_borrowed_str(str),
            Value::Integer(int) => visitor.visit_i64(int),
            Value::Float(float) => visitor.visit_f64(float),
            Value::Boolean(bool) => visitor.visit_bool(bool),
            Value::Datetime(ref _datetime) => todo!(), // TODO
            Value::Array(ref array) => visitor.visit_seq(SeqRefAccess::new(array)),
            Value::Table(ref table) => visitor.visit_map(MapRefAccess::new(table)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> StdResult<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
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
    fn new(array: &'de [Value]) -> Self {
        Self {
            values: array.iter(),
        }
    }
}

impl<'de> de::SeqAccess<'de> for SeqRefAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.values
            .next()
            .map(|value| seed.deserialize(value))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

struct MapRefAccess<'de> {
    kv_pairs: hash_map::Iter<'de, String, Value>,
    next_value: Option<&'de Value>,
}

impl<'de> MapRefAccess<'de> {
    fn new(table: &'de HashMap<String, Value>) -> Self {
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
            panic!("next_value_seed called without calling next_key_seed first")
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

    fn size_hint(&self) -> Option<usize> {
        Some(self.kv_pairs.len())
    }
}

struct EnumRefAccess<'de> {
    variant: &'de str,
    value: &'de Value,
}

impl<'de> EnumRefAccess<'de> {
    fn new(table: &'de HashMap<String, Value>) -> Result<Self> {
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

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // TODO toml-rs uses maps with integer keys instead of arrays for tuple variants. Do we need
        // to support this too?
        de::Deserializer::deserialize_seq(self.value, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.value, visitor)
    }
}
