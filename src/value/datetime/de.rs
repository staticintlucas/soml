use std::borrow::Cow;
use std::{fmt, str};

use serde::de::{self, Error as _, IgnoredAny, IntoDeserializer as _};

use super::{Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};
use crate::de::Error;

impl<'de> de::Deserialize<'de> for Datetime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(PartialEq, Eq)]
        enum Field {
            Datetime,
            OffsetDatetime,
            LocalDatetime,
            LocalDate,
            LocalTime,
        }
        struct FieldVisitor;

        impl de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid date-time field")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    Datetime::WRAPPER_FIELD => Ok(Self::Value::Datetime),
                    OffsetDatetime::WRAPPER_FIELD => Ok(Self::Value::OffsetDatetime),
                    LocalDatetime::WRAPPER_FIELD => Ok(Self::Value::LocalDatetime),
                    LocalDate::WRAPPER_FIELD => Ok(Self::Value::LocalDate),
                    LocalTime::WRAPPER_FIELD => Ok(Self::Value::LocalTime),
                    _ => Err(de::Error::unknown_field(
                        value,
                        &[
                            Datetime::WRAPPER_FIELD,
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
                formatter.write_str("map with one field")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(key) = map.next_key::<Field>()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let value = match key {
                    Field::Datetime => map.next_value::<DatetimeFromBytes>()?.0,
                    Field::OffsetDatetime => map.next_value::<OffsetDatetimeFromBytes>()?.0.into(),
                    Field::LocalDatetime => map.next_value::<LocalDatetimeFromBytes>()?.0.into(),
                    Field::LocalDate => map.next_value::<LocalDateFromBytes>()?.0.into(),
                    Field::LocalTime => map.next_value::<LocalTimeFromBytes>()?.0.into(),
                };
                // Need to use next_entry here to skip the value too, with next_key (and no
                // corresponding next_value) some deserializers will return bogus results
                if map.next_entry::<Field, IgnoredAny>()?.is_some() {
                    let mut len = 2;
                    while map.next_entry::<Field, IgnoredAny>()?.is_some() {
                        len += 1;
                    }
                    return Err(A::Error::invalid_length(len, &self));
                }
                Ok(value)
            }
        }

        // The deserializer should accept any of the *::WRAPPER_FIELD values, but we can only pass
        // one. In practice it always ignores the fields anyway, so we just pass Self::WRAPPER_FIELD
        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug)]
pub struct DatetimeFromBytes(pub Datetime);

impl<'de> de::Deserialize<'de> for DatetimeFromBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = DatetimeFromBytes;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid date-time")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Datetime::from_slice(v).map(DatetimeFromBytes).map_err(|e| {
                    de::Error::invalid_value(de::Unexpected::Other(&e.to_string()), &self)
                })
            }
        }

        deserializer.deserialize_bytes(Visitor)
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
                        formatter.write_str(concat!("a valid ", $expecting, " field"))
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
                        formatter.write_str("map with one field")
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
                        formatter.write_str(concat!("a valid ", $expecting))
                    }

                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        <$type>::from_slice(v).map(<$from_bytes>::new).map_err(|e| {
                            de::Error::invalid_value(de::Unexpected::Other(&e.to_string()), &self)
                        })
                    }
                }

                deserializer.deserialize_bytes(Visitor)
            }
        }
    };
}

#[derive(Debug)]
pub struct OffsetDatetimeFromBytes(pub OffsetDatetime);

impl OffsetDatetimeFromBytes {
    pub const fn new(value: OffsetDatetime) -> Self {
        Self(value)
    }
}

impl_deserialize!(OffsetDatetime, OffsetDatetimeFromBytes, "offset date-time");

#[derive(Debug)]
pub struct LocalDatetimeFromBytes(pub LocalDatetime);

impl LocalDatetimeFromBytes {
    pub const fn new(value: LocalDatetime) -> Self {
        Self(value)
    }
}

impl_deserialize!(LocalDatetime, LocalDatetimeFromBytes, "local date-time");

#[derive(Debug)]
pub struct LocalDateFromBytes(pub LocalDate);

impl LocalDateFromBytes {
    pub const fn new(value: LocalDate) -> Self {
        Self(value)
    }
}

impl_deserialize!(LocalDate, LocalDateFromBytes, "local date");

#[derive(Debug)]
pub struct LocalTimeFromBytes(pub LocalTime);

impl LocalTimeFromBytes {
    pub const fn new(value: LocalTime) -> Self {
        Self(value)
    }
}

impl_deserialize!(LocalTime, LocalTimeFromBytes, "local time");

#[derive(Debug)]
pub struct DatetimeAccess<'de>(Option<DatetimeAccessInner<'de>>);

#[derive(Debug)]
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
            panic!("next_value_seed called without calling next_key_seed first")
        };

        seed.deserialize(value.into_deserializer())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use serde::de::MapAccess;
    use serde_bytes::ByteBuf;

    use super::*;
    use crate::value::Offset;

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

    #[test]
    fn deserialize_datetime() {
        let map = indoc! {r#"{
            "<soml::_impl::Datetime::Wrapper::Field>": "2023-01-02T03:04:05.006+07:08"
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            Datetime {
                date: Some(DATE),
                time: Some(TIME),
                offset: Some(OFFSET),
            }
        );

        let map = indoc! {r#"{
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006+07:08"
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            Datetime {
                date: Some(DATE),
                time: Some(TIME),
                offset: Some(OFFSET),
            }
        );

        let map = indoc! {r#"{
            "<soml::_impl::LocalDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006"
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            Datetime {
                date: Some(DATE),
                time: Some(TIME),
                offset: None,
            }
        );

        let map = indoc! {r#"{
            "<soml::_impl::LocalDate::Wrapper::Field>": "2023-01-02"
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            Datetime {
                date: Some(DATE),
                time: None,
                offset: None,
            }
        );

        let map = indoc! {r#"{
            "<soml::_impl::LocalTime::Wrapper::Field>": "03:04:05.006"
        }"#};
        let date: Datetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            Datetime {
                date: None,
                time: Some(TIME),
                offset: None,
            }
        );

        let map = indoc! {r#"{
            "foo": "2023-01-02T03:04:05.006+07:08"
        }"#};
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = indoc! {r#"{
            2: "2023-01-02T03:04:05.006+07:08"
        }"#};
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::Datetime::Wrapper::Field>": "2023-01-02T03:04:05.006+07:08",
            "<soml::_impl::LocalDate::Wrapper::Field>": "2023-01-02",
            "<soml::_impl::LocalTime::Wrapper::Field>": "03:04:05.006"
        }"#};
        serde_json::from_str::<Datetime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<Datetime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_datetime_from_bytes() {
        let date: DatetimeFromBytes =
            serde_json::from_str(r#""2023-01-02T03:04:05.006+07:08""#).unwrap();
        assert_eq!(
            date.0,
            Datetime {
                date: Some(DATE),
                time: Some(TIME),
                offset: Some(OFFSET),
            }
        );

        serde_json::from_str::<DatetimeFromBytes>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<DatetimeFromBytes>("2").unwrap_err();
    }

    #[test]
    fn deserialize_offset_datetime() {
        let map = indoc! {r#"{
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006+07:08"
        }"#};
        let date: OffsetDatetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            OffsetDatetime {
                date: DATE,
                time: TIME,
                offset: OFFSET,
            }
        );

        let map = indoc! {r#"{
            "foo": "2023-01-02T03:04:05.006+07:08"
        }"#};
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006+07:08",
            "<soml::_impl::OffsetDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006+07:08"
        }"#};
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<OffsetDatetime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_offset_datetime_from_bytes() {
        let date: OffsetDatetimeFromBytes =
            serde_json::from_str(r#""2023-01-02T03:04:05.006+07:08""#).unwrap();
        assert_eq!(
            date.0,
            OffsetDatetime {
                date: DATE,
                time: TIME,
                offset: OFFSET,
            }
        );

        serde_json::from_str::<OffsetDatetimeFromBytes>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<OffsetDatetimeFromBytes>("2").unwrap_err();
    }

    #[test]
    fn deserialize_local_datetime() {
        let map = indoc! {r#"{
            "<soml::_impl::LocalDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006"
        }"#};
        let date: LocalDatetime = serde_json::from_str(map).unwrap();
        assert_eq!(
            date,
            LocalDatetime {
                date: DATE,
                time: TIME,
            }
        );

        let map = indoc! {r#"{
            "foo": "2023-01-02T03:04:05.006"
        }"#};
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::LocalDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006",
            "<soml::_impl::LocalDatetime::Wrapper::Field>": "2023-01-02T03:04:05.006"
        }"#};
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<LocalDatetime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_local_datetime_from_bytes() {
        let date: LocalDatetimeFromBytes =
            serde_json::from_str(r#""2023-01-02T03:04:05.006""#).unwrap();
        assert_eq!(
            date.0,
            LocalDatetime {
                date: DATE,
                time: TIME,
            }
        );

        serde_json::from_str::<LocalDatetimeFromBytes>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<LocalDatetimeFromBytes>("2").unwrap_err();
    }

    #[test]
    fn deserialize_local_date() {
        let map = indoc! {r#"{
            "<soml::_impl::LocalDate::Wrapper::Field>": "2023-01-02"
        }"#};
        let date: LocalDate = serde_json::from_str(map).unwrap();
        assert_eq!(date, DATE);

        let map = indoc! {r#"{
            "foo": "2023-01-02"
        }"#};
        serde_json::from_str::<LocalDate>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<LocalDate>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::LocalDate::Wrapper::Field>": "2023-01-02",
            "<soml::_impl::LocalDate::Wrapper::Field>": "2023-01-02"
        }"#};
        serde_json::from_str::<LocalDate>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<LocalDate>(map).unwrap_err();
    }

    #[test]
    fn deserialize_local_date_from_bytes() {
        let date: LocalDateFromBytes = serde_json::from_str(r#""2023-01-02""#).unwrap();
        assert_eq!(date.0, DATE);

        serde_json::from_str::<LocalDateFromBytes>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<LocalDateFromBytes>("2").unwrap_err();
    }

    #[test]
    fn deserialize_local_time() {
        let map = indoc! {r#"{
            "<soml::_impl::LocalTime::Wrapper::Field>": "03:04:05.006"
        }"#};
        let time: LocalTime = serde_json::from_str(map).unwrap();
        assert_eq!(time, TIME);

        let map = indoc! {r#"{
            "foo": "03:04:05.006"
        }"#};
        serde_json::from_str::<LocalTime>(map).unwrap_err();

        let map = "{}";
        serde_json::from_str::<LocalTime>(map).unwrap_err();

        let map = indoc! {r#"{
            "<soml::_impl::LocalTime::Wrapper::Field>": "03:04:05.006",
            "<soml::_impl::LocalTime::Wrapper::Field>": "03:04:05.006"
        }"#};
        serde_json::from_str::<LocalTime>(map).unwrap_err();

        let map = "2";
        serde_json::from_str::<LocalTime>(map).unwrap_err();
    }

    #[test]
    fn deserialize_local_time_from_bytes() {
        let time: LocalTimeFromBytes = serde_json::from_str(r#""03:04:05.006""#).unwrap();
        assert_eq!(time.0, TIME);

        serde_json::from_str::<LocalTimeFromBytes>(r#""invalid string""#).unwrap_err();

        serde_json::from_str::<LocalTimeFromBytes>("2").unwrap_err();
    }

    #[test]
    fn datetime_access_new() {
        let value = b"test string";

        assert_matches!(
            DatetimeAccess::offset_datetime(value),
            DatetimeAccess(Some(DatetimeAccessInner::OffsetDatetime(_)))
        );
        assert_matches!(
            DatetimeAccess::local_datetime(value),
            DatetimeAccess(Some(DatetimeAccessInner::LocalDatetime(_)))
        );
        assert_matches!(
            DatetimeAccess::local_date(value),
            DatetimeAccess(Some(DatetimeAccessInner::LocalDate(_)))
        );
        assert_matches!(
            DatetimeAccess::local_time(value),
            DatetimeAccess(Some(DatetimeAccessInner::LocalTime(_)))
        );
    }

    #[test]
    fn datetime_access_map_access() {
        let mut access = DatetimeAccess(Some(DatetimeAccessInner::OffsetDatetime(
            b"2023-01-02T03:04:05.006+07:08".into(),
        )));

        assert_eq!(
            access.next_entry().unwrap(),
            Some((
                OffsetDatetime::WRAPPER_FIELD,
                ByteBuf::from(b"2023-01-02T03:04:05.006+07:08")
            ))
        );
        assert!(access.next_entry::<&str, IgnoredAny>().unwrap().is_none());

        let mut access = DatetimeAccess(Some(DatetimeAccessInner::LocalDatetime(
            b"2023-01-02T03:04:05.006".into(),
        )));

        assert_eq!(
            access.next_entry().unwrap(),
            Some((
                LocalDatetime::WRAPPER_FIELD,
                ByteBuf::from(b"2023-01-02T03:04:05.006")
            ))
        );
        assert!(access.next_entry::<&str, IgnoredAny>().unwrap().is_none());

        let mut access = DatetimeAccess(Some(DatetimeAccessInner::LocalDate(b"2023-01-02".into())));

        assert_eq!(
            access.next_entry().unwrap(),
            Some((LocalDate::WRAPPER_FIELD, ByteBuf::from(b"2023-01-02")))
        );
        assert!(access.next_entry::<&str, IgnoredAny>().unwrap().is_none());

        let mut access =
            DatetimeAccess(Some(DatetimeAccessInner::LocalTime(b"03:04:05.006".into())));

        assert_eq!(
            access.next_entry().unwrap(),
            Some((LocalTime::WRAPPER_FIELD, ByteBuf::from(b"03:04:05.006")))
        );
        assert!(access.next_entry::<&str, IgnoredAny>().unwrap().is_none());
    }
}
