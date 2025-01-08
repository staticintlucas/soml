use std::borrow::Cow;
use std::fmt;
use std::str;

use serde::de;
use serde::de::{Error as _, IntoDeserializer as _};

use super::Datetime;
use super::LocalDate;
use super::LocalDatetime;
use super::LocalTime;
use super::OffsetDatetime;
use crate::de::Error;

impl<'de> de::Deserialize<'de> for Datetime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        enum Field {
            OffsetDatetime,
            LocalDatetime,
            LocalDate,
            LocalTime,
        }
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid date-time")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    OffsetDatetime::WRAPPER_FIELD => Ok(Self::Value::OffsetDatetime),
                    LocalDatetime::WRAPPER_FIELD => Ok(Self::Value::LocalDatetime),
                    LocalDate::WRAPPER_FIELD => Ok(Self::Value::LocalDate),
                    LocalTime::WRAPPER_FIELD => Ok(Self::Value::LocalTime),
                    _ => Err(de::Error::unknown_field(value, &[Datetime::WRAPPER_FIELD])),
                }
            }
        }

        impl<'de> de::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Datetime;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid date-time")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(key) = map.next_key::<Field>()? else {
                    return Err(A::Error::missing_field(Datetime::WRAPPER_FIELD));
                };
                let value = match key {
                    Field::OffsetDatetime => map.next_value::<OffsetDatetimeFromBytes>()?.0.into(),
                    Field::LocalDatetime => map.next_value::<LocalDatetimeFromBytes>()?.0.into(),
                    Field::LocalDate => map.next_value::<LocalDateFromBytes>()?.0.into(),
                    Field::LocalTime => map.next_value::<LocalTimeFromBytes>()?.0.into(),
                };
                if map.next_key::<Field>()?.is_some() {
                    return Err(A::Error::duplicate_field(Datetime::WRAPPER_FIELD));
                }
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

pub struct DatetimeAccess<'de>(Option<DatetimeAccessInner<'de>>);

enum DatetimeAccessInner<'de> {
    OffsetDatetime(Cow<'de, [u8]>),
    LocalDatetime(Cow<'de, [u8]>),
    LocalDate(Cow<'de, [u8]>),
    LocalTime(Cow<'de, [u8]>),
}

impl<'de> DatetimeAccess<'de> {
    pub fn offset_datetime(value: impl Into<Cow<'de, [u8]>>) -> Self {
        Self(Some(DatetimeAccessInner::OffsetDatetime(value.into())))
    }

    pub fn local_datetime(value: impl Into<Cow<'de, [u8]>>) -> Self {
        Self(Some(DatetimeAccessInner::LocalDatetime(value.into())))
    }

    pub fn local_date(value: impl Into<Cow<'de, [u8]>>) -> Self {
        Self(Some(DatetimeAccessInner::LocalDate(value.into())))
    }

    pub fn local_time(value: impl Into<Cow<'de, [u8]>>) -> Self {
        Self(Some(DatetimeAccessInner::LocalTime(value.into())))
    }
}

impl<'de> de::MapAccess<'de> for DatetimeAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        enum Field {
            OffsetDatetime,
            LocalDatetime,
            LocalDate,
            LocalTime,
        }

        impl<'de> de::Deserializer<'de> for Field {
            type Error = Error;

            fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: de::Visitor<'de>,
            {
                visitor.visit_borrowed_str(match self {
                    Self::OffsetDatetime => OffsetDatetime::WRAPPER_FIELD,
                    Self::LocalDatetime => LocalDatetime::WRAPPER_FIELD,
                    Self::LocalDate => LocalDate::WRAPPER_FIELD,
                    Self::LocalTime => LocalTime::WRAPPER_FIELD,
                })
            }

            serde::forward_to_deserialize_any! {
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
                bytes byte_buf option unit unit_struct newtype_struct seq tuple
                tuple_struct map struct enum identifier ignored_any
            }
        }

        let field = match self.0.as_ref() {
            Some(&DatetimeAccessInner::OffsetDatetime(_)) => Field::OffsetDatetime,
            Some(&DatetimeAccessInner::LocalDatetime(_)) => Field::LocalDatetime,
            Some(&DatetimeAccessInner::LocalDate(_)) => Field::LocalDate,
            Some(&DatetimeAccessInner::LocalTime(_)) => Field::LocalTime,
            None => return Ok(None),
        };

        seed.deserialize(field).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic)]
        let Some(
            DatetimeAccessInner::OffsetDatetime(value)
            | DatetimeAccessInner::LocalDatetime(value)
            | DatetimeAccessInner::LocalDate(value)
            | DatetimeAccessInner::LocalTime(value),
        ) = self.0.take()
        else {
            panic!("next_value called after next_key returned None");
        };

        seed.deserialize(value.into_deserializer())
    }
}

macro_rules! impl_deserialize {
    ($type:ty, $from_bytes:ty, $expecting:literal) => {
        impl<'de> de::Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct Field;
                struct FieldVisitor;

                impl de::Visitor<'_> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str($expecting)
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            <$type>::WRAPPER_FIELD => Ok(Field),
                            _ => Err(de::Error::unknown_field(value, &[<$type>::WRAPPER_FIELD])),
                        }
                    }
                }

                impl<'de> de::Deserialize<'de> for Field {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: de::Deserializer<'de>,
                    {
                        deserializer.deserialize_identifier(FieldVisitor)
                    }
                }

                struct Visitor;

                impl<'de> de::Visitor<'de> for Visitor {
                    type Value = $type;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str($expecting)
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: de::MapAccess<'de>,
                    {
                        let Some((_key, value)) = map.next_entry::<Field, $from_bytes>()? else {
                            return Err(A::Error::missing_field(<$type>::WRAPPER_FIELD));
                        };
                        if map.next_key::<Field>()?.is_some() {
                            return Err(A::Error::duplicate_field(<$type>::WRAPPER_FIELD));
                        }
                        Ok(value.0)
                    }
                }

                deserializer.deserialize_struct(
                    <$type>::WRAPPER_TYPE,
                    &[<$type>::WRAPPER_FIELD],
                    Visitor,
                )
            }
        }

        impl<'de> de::Deserialize<'de> for $from_bytes {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct Visitor;

                impl de::Visitor<'_> for Visitor {
                    type Value = $from_bytes;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str($expecting)
                    }

                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        <$type>::from_slice(v)
                            .map(<$from_bytes>::new)
                            .map_err(de::Error::custom)
                    }
                }

                deserializer.deserialize_bytes(Visitor)
            }
        }
    };
}

pub struct OffsetDatetimeFromBytes(pub OffsetDatetime);

impl OffsetDatetimeFromBytes {
    pub const fn new(value: OffsetDatetime) -> Self {
        Self(value)
    }
}

impl_deserialize!(
    OffsetDatetime,
    OffsetDatetimeFromBytes,
    "a date-time with offset"
);

pub struct LocalDatetimeFromBytes(pub LocalDatetime);

impl LocalDatetimeFromBytes {
    pub const fn new(value: LocalDatetime) -> Self {
        Self(value)
    }
}

impl_deserialize!(LocalDatetime, LocalDatetimeFromBytes, "a local date-time");

pub struct LocalDateFromBytes(pub LocalDate);

impl LocalDateFromBytes {
    pub const fn new(value: LocalDate) -> Self {
        Self(value)
    }
}

impl_deserialize!(LocalDate, LocalDateFromBytes, "a local date");

pub struct LocalTimeFromBytes(pub LocalTime);

impl LocalTimeFromBytes {
    pub const fn new(value: LocalTime) -> Self {
        Self(value)
    }
}

impl_deserialize!(LocalTime, LocalTimeFromBytes, "a local time");
