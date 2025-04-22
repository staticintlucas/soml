use std::{fmt, str};

use serde::de::{self, Error as _, IntoDeserializer as _};

use super::{Datetime, LocalDate, LocalDatetime, LocalTime, Offset, OffsetDatetime};
use crate::de::Error;

impl<'de> de::Deserialize<'de> for Datetime {
    #[inline]
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
                formatter.write_str("a date-time wrapper field")
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
                    _ => Err(de::Error::unknown_field(
                        value,
                        &[
                            OffsetDatetime::WRAPPER_FIELD,
                            LocalDatetime::WRAPPER_FIELD,
                            LocalDate::WRAPPER_FIELD,
                            LocalTime::WRAPPER_FIELD,
                        ],
                    )),
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
                formatter.write_str("a date-time wrapper")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(field) = map.next_key::<Field>()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let value = match field {
                    Field::OffsetDatetime => map.next_value::<OffsetDatetimeFromFields>()?.0.into(),
                    Field::LocalDatetime => map.next_value::<LocalDatetimeFromFields>()?.0.into(),
                    Field::LocalDate => map.next_value::<LocalDateFromFields>()?.0.into(),
                    Field::LocalTime => map.next_value::<LocalTimeFromFields>()?.0.into(),
                };
                Ok(value)
            }
        }

        // The deserializer should accept any of the *::WRAPPER_TYPE/FIELD values, but we can only
        // pass one. So we pass Datetime::* to mean we accept any of the others.
        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

impl<'de> de::Deserialize<'de> for OffsetDatetime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Field;
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an offset date-time wrapper field")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    OffsetDatetime::WRAPPER_FIELD => Ok(Field),
                    _ => Err(de::Error::unknown_field(
                        value,
                        &[OffsetDatetime::WRAPPER_FIELD],
                    )),
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
            type Value = OffsetDatetime;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an offset date-time wrapper")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, OffsetDatetimeFromFields(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(OffsetDatetime::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug)]
pub struct OffsetDatetimeFromFields(pub OffsetDatetime);

impl<'de> de::Deserialize<'de> for OffsetDatetimeFromFields {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = OffsetDatetimeFromFields;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of offset date-time fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(LocalDateFromFields(date)) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let Some(LocalTimeFromFields(time)) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(1, &self));
                };
                let Some(OffsetFromFields(offset)) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(2, &self));
                };
                Ok(OffsetDatetimeFromFields(OffsetDatetime {
                    date,
                    time,
                    offset,
                }))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct OffsetDatetimeAccess(Option<OffsetDatetime>);

impl From<OffsetDatetime> for OffsetDatetimeAccess {
    fn from(datetime: OffsetDatetime) -> Self {
        Self(Some(datetime))
    }
}

impl<'de> de::MapAccess<'de> for OffsetDatetimeAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.0.is_some() {
            seed.deserialize(de::value::BorrowedStrDeserializer::new(
                OffsetDatetime::WRAPPER_FIELD,
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(datetime) => seed.deserialize(de::value::SeqAccessDeserializer::new(OffsetDatetimeInnerAccess::from(datetime))),
            None => panic!(
                "OffsetDatetimeAccess::next_value called without calling OffsetDatetimeAccess::next_key first"
            ),
        }
    }
}

#[derive(Debug)]
struct OffsetDatetimeInnerAccess {
    date: Option<LocalDate>,
    time: Option<LocalTime>,
    offset: Option<Offset>,
}

impl From<OffsetDatetime> for OffsetDatetimeInnerAccess {
    fn from(datetime: OffsetDatetime) -> Self {
        let OffsetDatetime { date, time, offset } = datetime;
        Self {
            date: Some(date),
            time: Some(time),
            offset: Some(offset),
        }
    }
}

impl<'de> de::SeqAccess<'de> for OffsetDatetimeInnerAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if let Some(date) = self.date.take() {
            seed.deserialize(de::value::SeqAccessDeserializer::new(
                LocalDateInnerAccess::from(date),
            ))
            .map(Some)
        } else if let Some(time) = self.time.take() {
            seed.deserialize(de::value::SeqAccessDeserializer::new(
                LocalTimeInnerAccess::from(time),
            ))
            .map(Some)
        } else if let Some(offset) = self.offset.take() {
            seed.deserialize(de::value::SeqAccessDeserializer::new(
                OffsetInnerAccess::from(offset),
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de> de::Deserialize<'de> for LocalDatetime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Field;
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date-time wrapper field")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    LocalDatetime::WRAPPER_FIELD => Ok(Field),
                    _ => Err(de::Error::unknown_field(
                        value,
                        &[LocalDatetime::WRAPPER_FIELD],
                    )),
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
            type Value = LocalDatetime;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date-time wrapper")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, LocalDatetimeFromFields(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(LocalDatetime::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug)]
pub struct LocalDatetimeFromFields(pub LocalDatetime);

impl<'de> de::Deserialize<'de> for LocalDatetimeFromFields {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = LocalDatetimeFromFields;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of local date-time fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(LocalDateFromFields(date)) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let Some(LocalTimeFromFields(time)) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(1, &self));
                };
                Ok(LocalDatetimeFromFields(LocalDatetime { date, time }))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct LocalDatetimeAccess(Option<LocalDatetime>);

impl From<LocalDatetime> for LocalDatetimeAccess {
    fn from(datetime: LocalDatetime) -> Self {
        Self(Some(datetime))
    }
}

impl<'de> de::MapAccess<'de> for LocalDatetimeAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.0.is_some() {
            seed.deserialize(de::value::BorrowedStrDeserializer::new(
                LocalDatetime::WRAPPER_FIELD,
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(datetime) => seed.deserialize(de::value::SeqAccessDeserializer::new(LocalDatetimeInnerAccess::from(datetime))),
            None => panic!(
                "LocalDatetimeAccess::next_value called without calling LocalDatetimeAccess::next_key first"
            ),
        }
    }
}

#[derive(Debug)]
struct LocalDatetimeInnerAccess {
    date: Option<LocalDate>,
    time: Option<LocalTime>,
}

impl From<LocalDatetime> for LocalDatetimeInnerAccess {
    fn from(datetime: LocalDatetime) -> Self {
        let LocalDatetime { date, time } = datetime;
        Self {
            date: Some(date),
            time: Some(time),
        }
    }
}

impl<'de> de::SeqAccess<'de> for LocalDatetimeInnerAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if let Some(date) = self.date.take() {
            seed.deserialize(de::value::SeqAccessDeserializer::new(
                LocalDateInnerAccess::from(date),
            ))
            .map(Some)
        } else if let Some(time) = self.time.take() {
            seed.deserialize(de::value::SeqAccessDeserializer::new(
                LocalTimeInnerAccess::from(time),
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de> de::Deserialize<'de> for LocalDate {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Field;
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date wrapper field")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    LocalDate::WRAPPER_FIELD => Ok(Field),
                    _ => Err(de::Error::unknown_field(value, &[LocalDate::WRAPPER_FIELD])),
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
            type Value = LocalDate;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date wrapper")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, LocalDateFromFields(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(LocalDate::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug)]
pub struct LocalDateFromFields(pub LocalDate);

impl<'de> de::Deserialize<'de> for LocalDateFromFields {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = LocalDateFromFields;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of local date fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(year) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let Some(month) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(1, &self));
                };
                let Some(day) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(2, &self));
                };
                Ok(LocalDateFromFields(LocalDate { year, month, day }))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct LocalDateAccess(Option<LocalDate>);

impl From<LocalDate> for LocalDateAccess {
    fn from(date: LocalDate) -> Self {
        Self(Some(date))
    }
}

impl<'de> de::MapAccess<'de> for LocalDateAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.0.is_some() {
            seed.deserialize(de::value::BorrowedStrDeserializer::new(
                LocalDate::WRAPPER_FIELD,
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(date) => seed.deserialize(de::value::SeqAccessDeserializer::new(
                LocalDateInnerAccess::from(date),
            )),
            None => panic!(
                "LocalDateAccess::next_value called without calling LocalDateAccess::next_key first"
            ),
        }
    }
}

#[derive(Debug)]
struct LocalDateInnerAccess {
    year: Option<u16>,
    month: Option<u8>,
    day: Option<u8>,
}

impl From<LocalDate> for LocalDateInnerAccess {
    fn from(date: LocalDate) -> Self {
        let LocalDate { year, month, day } = date;
        Self {
            year: Some(year),
            month: Some(month),
            day: Some(day),
        }
    }
}

impl<'de> de::SeqAccess<'de> for LocalDateInnerAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if let Some(year) = self.year.take() {
            seed.deserialize(year.into_deserializer()).map(Some)
        } else if let Some(month) = self.month.take() {
            seed.deserialize(month.into_deserializer()).map(Some)
        } else if let Some(day) = self.day.take() {
            seed.deserialize(day.into_deserializer()).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de> de::Deserialize<'de> for LocalTime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Field;
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local time wrapper field")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    LocalTime::WRAPPER_FIELD => Ok(Field),
                    _ => Err(de::Error::unknown_field(value, &[LocalTime::WRAPPER_FIELD])),
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
            type Value = LocalTime;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local time wrapper")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, LocalTimeFromFields(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(LocalTime::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug)]
pub struct LocalTimeFromFields(pub LocalTime);

impl<'de> de::Deserialize<'de> for LocalTimeFromFields {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = LocalTimeFromFields;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of local time fields")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(hour) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let Some(minute) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(1, &self));
                };
                let Some(second) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(2, &self));
                };
                let Some(nanosecond) = seq.next_element()? else {
                    return Err(A::Error::invalid_length(3, &self));
                };
                Ok(LocalTimeFromFields(LocalTime {
                    hour,
                    minute,
                    second,
                    nanosecond,
                }))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct LocalTimeAccess(Option<LocalTime>);

impl From<LocalTime> for LocalTimeAccess {
    fn from(time: LocalTime) -> Self {
        Self(Some(time))
    }
}

impl<'de> de::MapAccess<'de> for LocalTimeAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.0.is_some() {
            seed.deserialize(de::value::BorrowedStrDeserializer::new(
                LocalTime::WRAPPER_FIELD,
            ))
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(time) => seed.deserialize(de::value::SeqAccessDeserializer::new(
                LocalTimeInnerAccess::from(time),
            )),
            None => panic!(
                "LocalTimeAccess::next_value called without calling LocalTimeAccess::next_key first"
            ),
        }
    }
}

#[derive(Debug)]
struct LocalTimeInnerAccess {
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
    nanosecond: Option<u32>,
}

impl From<LocalTime> for LocalTimeInnerAccess {
    fn from(time: LocalTime) -> Self {
        let LocalTime {
            hour,
            minute,
            second,
            nanosecond,
        } = time;
        Self {
            hour: Some(hour),
            minute: Some(minute),
            second: Some(second),
            nanosecond: Some(nanosecond),
        }
    }
}

impl<'de> de::SeqAccess<'de> for LocalTimeInnerAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if let Some(hour) = self.hour.take() {
            seed.deserialize(hour.into_deserializer()).map(Some)
        } else if let Some(minute) = self.minute.take() {
            seed.deserialize(minute.into_deserializer()).map(Some)
        } else if let Some(second) = self.second.take() {
            seed.deserialize(second.into_deserializer()).map(Some)
        } else if let Some(nanosecond) = self.nanosecond.take() {
            seed.deserialize(nanosecond.into_deserializer()).map(Some)
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct OffsetFromFields(pub Offset);

impl<'de> de::Deserialize<'de> for OffsetFromFields {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = OffsetFromFields;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of offset fields")
            }

            #[inline]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Ok(OffsetFromFields(
                    (seq.next_element()?).map_or(Offset::Z, |minutes| Offset::Custom { minutes }),
                ))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

struct OffsetInnerAccess(Option<Offset>);

impl From<Offset> for OffsetInnerAccess {
    fn from(offset: Offset) -> Self {
        Self(Some(offset))
    }
}

impl<'de> de::SeqAccess<'de> for OffsetInnerAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.0.take() {
            Some(Offset::Custom { minutes }) => {
                seed.deserialize(minutes.into_deserializer()).map(Some)
            }
            Some(Offset::Z) | None => Ok(None),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use serde::de::{MapAccess as _, SeqAccess as _};

    use super::*;

    const DATE: LocalDate = LocalDate {
        year: 2023,
        month: 1,
        day: 2,
    };
    const TIME: LocalTime = LocalTime {
        hour: 3,
        minute: 4,
        second: 5,
        nanosecond: 6_000_000,
    };
    const OFFSET: Offset = Offset::Custom { minutes: 428 };

    const OFFSET_DATETIME: OffsetDatetime = OffsetDatetime {
        date: DATE,
        time: TIME,
        offset: OFFSET,
    };
    const LOCAL_DATETIME: LocalDatetime = LocalDatetime {
        date: DATE,
        time: TIME,
    };

    #[test]
    fn deserialize_datetime() {
        let map = indoc! {r#"{
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000], [428]]
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(date, Datetime::from(OFFSET_DATETIME));

        let map = indoc! {r#"{
            "<soml::_impl::LocalDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000]]
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(date, Datetime::from(LOCAL_DATETIME));

        let map = indoc! {r#"{
            "<soml::_impl::LocalDate::Wrapper::Field>": [2023, 1, 2]
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(date, Datetime::from(DATE));

        let map = indoc! {r#"{
            "<soml::_impl::LocalTime::Wrapper::Field>": [3, 4, 5, 6000000]
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(date, Datetime::from(TIME));

        let map = indoc! {r#"{
            "foo": [[2023, 1, 2], [3, 4, 5, 6000000], [428]]
        }"#};
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = indoc! {"{
            2: [[2023, 1, 2], [3, 4, 5, 6000000], [428]]
        }"};
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::Datetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000], [428]],
            "<soml::_impl::LocalDate::Wrapper::Field>": [2023, 1, 2],
            "<soml::_impl::LocalTime::Wrapper::Field>": [3, 4, 5, 6000000]
        }"#};
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<Datetime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_offset_datetime() {
        let map = indoc! {r#"{
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000], [428]]
        }"#};
        let date: OffsetDatetime = serde_json::from_str(map).unwrap();
        assert_eq!(date, OFFSET_DATETIME);

        let map = indoc! {r#"{
            "foo": [[2023, 1, 2], [3, 4, 5, 6000000], [428]]
        }"#};
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000], [428]],
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000], [428]]
        }"#};
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_offset_datetime_from_fields() {
        let date: OffsetDatetimeFromFields =
            serde_json::from_str("[[2023, 1, 2], [3, 4, 5, 6000000], [428]]").unwrap();
        assert_eq!(date.0, OFFSET_DATETIME);

        serde_json::from_str::<OffsetDatetimeFromFields>("[[2023, 1, 2], [3, 4, 5, 6000000]]")
            .unwrap_err();
        serde_json::from_str::<OffsetDatetimeFromFields>("[[2023, 1, 2]]").unwrap_err();
        serde_json::from_str::<OffsetDatetimeFromFields>("[]").unwrap_err();

        serde_json::from_str::<OffsetDatetimeFromFields>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<OffsetDatetimeFromFields>("2").unwrap_err();
    }

    #[test]
    fn offset_datetime_access() {
        let mut access = OffsetDatetimeAccess::from(OFFSET_DATETIME);

        assert_matches!(access, OffsetDatetimeAccess(Some(OFFSET_DATETIME)));

        assert_eq!(
            access.next_key().unwrap(),
            Some(OffsetDatetime::WRAPPER_FIELD)
        );
        assert_eq!(
            access
                .next_value::<Vec<Vec<de::IgnoredAny>>>()
                .unwrap()
                .len(),
            3
        );

        assert!(access.next_key::<&str>().unwrap().is_none());
    }

    #[test]
    #[should_panic = "OffsetDatetimeAccess::next_value called without calling OffsetDatetimeAccess::next_key first"]
    fn offset_datetime_access_empty() {
        let mut access = OffsetDatetimeAccess(None);

        access.next_value::<Vec<Vec<de::IgnoredAny>>>().unwrap();
    }

    #[test]
    fn offset_datetime_inner_access() {
        let mut access = OffsetDatetimeInnerAccess::from(OFFSET_DATETIME);

        assert_matches!(
            access,
            OffsetDatetimeInnerAccess {
                date: Some(DATE),
                time: Some(TIME),
                offset: Some(OFFSET),
            }
        );

        assert_matches!(
            access.next_element::<LocalDateFromFields>().unwrap(),
            Some(LocalDateFromFields(DATE))
        );
        assert_matches!(
            access.next_element::<LocalTimeFromFields>().unwrap(),
            Some(LocalTimeFromFields(TIME))
        );
        assert_matches!(
            access.next_element::<OffsetFromFields>().unwrap(),
            Some(OffsetFromFields(OFFSET))
        );

        assert!(access.next_element::<de::IgnoredAny>().unwrap().is_none());
    }

    #[test]
    fn deserialize_local_datetime() {
        let map = indoc! {r#"{
            "<soml::_impl::LocalDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000]]
        }"#};
        let date: LocalDatetime = serde_json::from_str(map).unwrap();
        assert_eq!(date, LOCAL_DATETIME);

        let map = indoc! {r#"{
            "foo": [[2023, 1, 2], [3, 4, 5, 6000000]]
        }"#};
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::LocalDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000]],
            "<soml::_impl::LocalDatetime::Wrapper::Field>": [[2023, 1, 2], [3, 4, 5, 6000000]]
        }"#};
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_local_datetime_from_fields() {
        let date: LocalDatetimeFromFields =
            serde_json::from_str("[[2023, 1, 2], [3, 4, 5, 6000000]]").unwrap();
        assert_eq!(date.0, LOCAL_DATETIME);

        serde_json::from_str::<LocalDatetimeFromFields>("[[2023, 1, 2]]").unwrap_err();
        serde_json::from_str::<LocalDatetimeFromFields>("[]").unwrap_err();

        serde_json::from_str::<LocalDatetimeFromFields>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<LocalDatetimeFromFields>("2").unwrap_err();
    }

    #[test]
    fn local_datetime_access() {
        let mut access = LocalDatetimeAccess::from(LOCAL_DATETIME);

        assert_matches!(access, LocalDatetimeAccess(Some(LOCAL_DATETIME)));

        assert_eq!(
            access.next_key().unwrap(),
            Some(LocalDatetime::WRAPPER_FIELD)
        );
        assert_eq!(
            access
                .next_value::<Vec<Vec<de::IgnoredAny>>>()
                .unwrap()
                .len(),
            2
        );

        assert!(access.next_key::<&str>().unwrap().is_none());
    }

    #[test]
    #[should_panic = "LocalDatetimeAccess::next_value called without calling LocalDatetimeAccess::next_key first"]
    fn local_datetime_access_empty() {
        let mut access = LocalDatetimeAccess(None);

        access.next_value::<Vec<Vec<de::IgnoredAny>>>().unwrap();
    }

    #[test]
    fn local_datetime_inner_access() {
        let mut access = LocalDatetimeInnerAccess::from(LOCAL_DATETIME);

        assert_matches!(
            access,
            LocalDatetimeInnerAccess {
                date: Some(DATE),
                time: Some(TIME),
            }
        );

        assert_matches!(
            access.next_element::<LocalDateFromFields>().unwrap(),
            Some(LocalDateFromFields(DATE))
        );
        assert_matches!(
            access.next_element::<LocalTimeFromFields>().unwrap(),
            Some(LocalTimeFromFields(TIME))
        );

        assert!(access.next_element::<de::IgnoredAny>().unwrap().is_none());
    }

    #[test]
    fn deserialize_local_date() {
        let map = indoc! {r#"{
            "<soml::_impl::LocalDate::Wrapper::Field>": [2023, 1, 2]
        }"#};
        let date: LocalDate = serde_json::from_str(map).unwrap();
        assert_eq!(date, DATE);

        let map = indoc! {r#"{
            "foo": [2023, 1, 2]
        }"#};
        serde_json::from_str::<LocalDate>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<LocalDate>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::LocalDate::Wrapper::Field>": [2023, 1, 2],
            "<soml::_impl::LocalDate::Wrapper::Field>": [2023, 1, 2]
        }"#};
        serde_json::from_str::<LocalDate>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<LocalDate>(map).unwrap_err();
    }

    #[test]
    fn deserialize_local_date_from_fields() {
        let date: LocalDateFromFields = serde_json::from_str("[2023, 1, 2]").unwrap();
        assert_eq!(date.0, DATE);

        serde_json::from_str::<LocalDateFromFields>("[2023, 1]").unwrap_err();
        serde_json::from_str::<LocalDateFromFields>("[2023]").unwrap_err();
        serde_json::from_str::<LocalDateFromFields>("[]").unwrap_err();

        serde_json::from_str::<LocalDateFromFields>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<LocalDateFromFields>("2").unwrap_err();
    }

    #[test]
    fn local_date_access() {
        let mut access = LocalDateAccess::from(DATE);

        assert_matches!(access, LocalDateAccess(Some(DATE)));

        assert_eq!(access.next_key().unwrap(), Some(LocalDate::WRAPPER_FIELD));
        assert_eq!(access.next_value::<Vec<de::IgnoredAny>>().unwrap().len(), 3);

        assert!(access.next_key::<&str>().unwrap().is_none());
    }

    #[test]
    #[should_panic = "LocalDateAccess::next_value called without calling LocalDateAccess::next_key first"]
    fn local_date_access_empty() {
        let mut access = LocalDateAccess(None);

        access.next_value::<Vec<Vec<de::IgnoredAny>>>().unwrap();
    }

    #[test]
    fn local_date_inner_access() {
        let mut access = LocalDateInnerAccess::from(DATE);

        assert_eq!(access.year, Some(DATE.year));
        assert_eq!(access.month, Some(DATE.month));
        assert_eq!(access.day, Some(DATE.day));

        assert_eq!(access.next_element::<u16>().unwrap(), Some(DATE.year));
        assert_eq!(access.next_element::<u8>().unwrap(), Some(DATE.month));
        assert_eq!(access.next_element::<u8>().unwrap(), Some(DATE.day));

        assert!(access.next_element::<de::IgnoredAny>().unwrap().is_none());
    }

    #[test]
    fn deserialize_local_time() {
        let map = indoc! {r#"{
            "<soml::_impl::LocalTime::Wrapper::Field>": [3, 4, 5, 6000000]
        }"#};
        let time: LocalTime = serde_json::from_str(map).unwrap();
        assert_eq!(time, TIME);

        let map = indoc! {r#"{
            "foo": [3, 4, 5, 6000000]
        }"#};
        serde_json::from_str::<LocalTime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<LocalTime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::LocalTime::Wrapper::Field>": [3, 4, 5, 6000000],
            "<soml::_impl::LocalTime::Wrapper::Field>": [3, 4, 5, 6000000]
        }"#};
        serde_json::from_str::<LocalTime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<LocalTime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_local_time_from_fields() {
        let time: LocalTimeFromFields = serde_json::from_str("[3, 4, 5, 6000000]").unwrap();
        assert_eq!(time.0, TIME);

        serde_json::from_str::<LocalTimeFromFields>("[3, 4, 5]").unwrap_err();
        serde_json::from_str::<LocalTimeFromFields>("[3, 4]").unwrap_err();
        serde_json::from_str::<LocalTimeFromFields>("[3]").unwrap_err();
        serde_json::from_str::<LocalTimeFromFields>("[]").unwrap_err();

        serde_json::from_str::<LocalTimeFromFields>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<LocalTimeFromFields>("2").unwrap_err();
    }

    #[test]
    fn local_time_access() {
        let mut access = LocalTimeAccess::from(TIME);

        assert_matches!(access, LocalTimeAccess(Some(TIME)));

        assert_eq!(access.next_key().unwrap(), Some(LocalTime::WRAPPER_FIELD));
        assert_eq!(access.next_value::<Vec<de::IgnoredAny>>().unwrap().len(), 4);

        assert!(access.next_key::<&str>().unwrap().is_none());
    }

    #[test]
    #[should_panic = "LocalTimeAccess::next_value called without calling LocalTimeAccess::next_key first"]
    fn local_time_access_empty() {
        let mut access = LocalTimeAccess(None);

        access.next_value::<Vec<Vec<de::IgnoredAny>>>().unwrap();
    }

    #[test]
    fn local_time_inner_access() {
        let mut access = LocalTimeInnerAccess::from(TIME);

        assert_eq!(access.hour, Some(TIME.hour));
        assert_eq!(access.minute, Some(TIME.minute));
        assert_eq!(access.second, Some(TIME.second));
        assert_eq!(access.nanosecond, Some(TIME.nanosecond));

        assert_eq!(access.next_element::<u8>().unwrap(), Some(TIME.hour));
        assert_eq!(access.next_element::<u8>().unwrap(), Some(TIME.minute));
        assert_eq!(access.next_element::<u8>().unwrap(), Some(TIME.second));
        assert_eq!(access.next_element::<u32>().unwrap(), Some(TIME.nanosecond));

        assert!(access.next_element::<de::IgnoredAny>().unwrap().is_none());
    }
}
