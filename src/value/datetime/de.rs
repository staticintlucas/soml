use std::{fmt, str};

use serde::de::{self, Error as _};

use super::{AnyDatetime, Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};
use crate::de::Error;

impl<'de> de::Deserialize<'de> for AnyDatetime {
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a date-time wrapper field")
            }

            #[inline]
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
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = AnyDatetime;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a date-time wrapper")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some(field) = map.next_key::<Field>()? else {
                    return Err(A::Error::invalid_length(0, &self));
                };
                let value = match field {
                    Field::OffsetDatetime => {
                        Self::Value::OffsetDatetime(map.next_value::<EncodedOffsetDatetime>()?.0)
                    }
                    Field::LocalDatetime => {
                        Self::Value::LocalDatetime(map.next_value::<EncodedLocalDatetime>()?.0)
                    }
                    Field::LocalDate => {
                        Self::Value::LocalDate(map.next_value::<EncodedLocalDate>()?.0)
                    }
                    Field::LocalTime => {
                        Self::Value::LocalTime(map.next_value::<EncodedLocalTime>()?.0)
                    }
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

impl<'de> de::Deserialize<'de> for Datetime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        AnyDatetime::deserialize(deserializer).map(Into::into)
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an offset date-time wrapper field")
            }

            #[inline]
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
            #[inline]
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an offset date-time wrapper")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, EncodedOffsetDatetime(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(OffsetDatetime::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncodedOffsetDatetime(pub OffsetDatetime);

impl<'de> de::Deserialize<'de> for EncodedOffsetDatetime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = EncodedOffsetDatetime;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an encoded offset date-time")
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = v
                    .try_into()
                    .map_err(|_| E::invalid_length(v.len(), &self))?;
                Ok(EncodedOffsetDatetime(OffsetDatetime::from_encoded(bytes)))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct OffsetDatetimeAccess(Option<OffsetDatetime>);

impl From<OffsetDatetime> for OffsetDatetimeAccess {
    #[inline]
    fn from(datetime: OffsetDatetime) -> Self {
        Self(Some(datetime))
    }
}

impl<'de> de::MapAccess<'de> for OffsetDatetimeAccess {
    type Error = Error;

    #[inline]
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

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(datetime) => seed.deserialize(de::value::BytesDeserializer::new(&datetime.to_encoded())),
            None => panic!(
                "OffsetDatetimeAccess::next_value called without calling OffsetDatetimeAccess::next_key first"
            ),
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date-time wrapper field")
            }

            #[inline]
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
            #[inline]
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date-time wrapper")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, EncodedLocalDatetime(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(LocalDatetime::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncodedLocalDatetime(pub LocalDatetime);

impl<'de> de::Deserialize<'de> for EncodedLocalDatetime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = EncodedLocalDatetime;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an encoded local date-time")
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = v
                    .try_into()
                    .map_err(|_| E::invalid_length(v.len(), &self))?;
                Ok(EncodedLocalDatetime(LocalDatetime::from_encoded(bytes)))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct LocalDatetimeAccess(Option<LocalDatetime>);

impl From<LocalDatetime> for LocalDatetimeAccess {
    #[inline]
    fn from(datetime: LocalDatetime) -> Self {
        Self(Some(datetime))
    }
}

impl<'de> de::MapAccess<'de> for LocalDatetimeAccess {
    type Error = Error;

    #[inline]
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

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(datetime) => seed.deserialize(de::value::BytesDeserializer::new(&datetime.to_encoded())),
            None => panic!(
                "LocalDatetimeAccess::next_value called without calling LocalDatetimeAccess::next_key first"
            ),
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date wrapper field")
            }

            #[inline]
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
            #[inline]
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local date wrapper")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, EncodedLocalDate(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(LocalDate::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncodedLocalDate(pub LocalDate);

impl<'de> de::Deserialize<'de> for EncodedLocalDate {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = EncodedLocalDate;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an encoded local date")
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = v
                    .try_into()
                    .map_err(|_| E::invalid_length(v.len(), &self))?;
                Ok(EncodedLocalDate(LocalDate::from_encoded(bytes)))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct LocalDateAccess(Option<LocalDate>);

impl From<LocalDate> for LocalDateAccess {
    #[inline]
    fn from(date: LocalDate) -> Self {
        Self(Some(date))
    }
}

impl<'de> de::MapAccess<'de> for LocalDateAccess {
    type Error = Error;

    #[inline]
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

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(date) => seed.deserialize(de::value::BytesDeserializer::new(&date.to_encoded())),
            None => panic!(
                "LocalDateAccess::next_value called without calling LocalDateAccess::next_key first"
            ),
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local time wrapper field")
            }

            #[inline]
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
            #[inline]
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

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a local time wrapper")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let Some((Field, EncodedLocalTime(value))) = map.next_entry()? else {
                    return Err(A::Error::missing_field(LocalTime::WRAPPER_FIELD));
                };
                Ok(value)
            }
        }

        deserializer.deserialize_struct(Self::WRAPPER_TYPE, &[Self::WRAPPER_FIELD], Visitor)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncodedLocalTime(pub LocalTime);

impl<'de> de::Deserialize<'de> for EncodedLocalTime {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = EncodedLocalTime;

            #[inline]
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an encoded local time")
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let bytes = v
                    .try_into()
                    .map_err(|_| E::invalid_length(v.len(), &self))?;
                Ok(EncodedLocalTime(LocalTime::from_encoded(bytes)))
            }
        }

        deserializer.deserialize_seq(Visitor)
    }
}

#[derive(Debug)]
pub struct LocalTimeAccess(Option<LocalTime>);

impl From<LocalTime> for LocalTimeAccess {
    #[inline]
    fn from(time: LocalTime) -> Self {
        Self(Some(time))
    }
}

impl<'de> de::MapAccess<'de> for LocalTimeAccess {
    type Error = Error;

    #[inline]
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

    #[inline]
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        #[allow(clippy::panic, clippy::option_if_let_else)]
        match self.0.take() {
            Some(time) => seed.deserialize(de::value::BytesDeserializer::new(&time.to_encoded())),
            None => panic!(
                "LocalTimeAccess::next_value called without calling LocalTimeAccess::next_key first"
            ),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use serde::de::MapAccess as _;
    use serde_bytes::ByteBuf;
    use serde_test::{assert_de_tokens, assert_de_tokens_error, Token};

    use super::*;

    #[test]
    fn deserialize_any_datetime() {
        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&AnyDatetime::EXAMPLE_OFFSET_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&AnyDatetime::EXAMPLE_LOCAL_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&AnyDatetime::EXAMPLE_LOCAL_DATE, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&AnyDatetime::EXAMPLE_LOCAL_TIME, tokens);

        let tokens = &[Token::Struct {
            name: "foo",
            len: 1,
        }];
        assert_de_tokens_error::<AnyDatetime>(
            tokens,
            r#"expected Token::Struct { name: "foo", len: 1 } but deserialization wants Token::Struct { name: "<soml::_impl::AnyDatetime::Wrapper>", len: 1 }"#,
        );

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str("bar"),
        ];
        assert_de_tokens_error::<AnyDatetime>(tokens, "unknown field `bar`, expected one of `<soml::_impl::OffsetDatetime::Wrapper::Field>`, `<soml::_impl::LocalDatetime::Wrapper::Field>`, `<soml::_impl::LocalDate::Wrapper::Field>`, `<soml::_impl::LocalTime::Wrapper::Field>`");

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 0,
            },
            Token::StructEnd,
        ];
        assert_de_tokens_error::<AnyDatetime>(
            tokens,
            "invalid length 0, expected a date-time wrapper",
        );

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 3,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED),
            Token::Str(LocalDate::WRAPPER_FIELD),
        ];
        assert_de_tokens_error::<AnyDatetime>(
            tokens,
            r#"expected Token::Str("<soml::_impl::LocalDate::Wrapper::Field>") but deserialization wants Token::StructEnd"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<AnyDatetime>(
            tokens,
            "invalid type: integer `2`, expected a date-time wrapper",
        );
    }

    #[test]
    fn deserialize_datetime() {
        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&Datetime::EXAMPLE_OFFSET_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&Datetime::EXAMPLE_LOCAL_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&Datetime::EXAMPLE_LOCAL_DATE, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&Datetime::EXAMPLE_LOCAL_TIME, tokens);

        let tokens = &[Token::Struct {
            name: "foo",
            len: 1,
        }];
        assert_de_tokens_error::<Datetime>(
            tokens,
            r#"expected Token::Struct { name: "foo", len: 1 } but deserialization wants Token::Struct { name: "<soml::_impl::AnyDatetime::Wrapper>", len: 1 }"#,
        );

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str("bar"),
        ];
        assert_de_tokens_error::<Datetime>(tokens, "unknown field `bar`, expected one of `<soml::_impl::OffsetDatetime::Wrapper::Field>`, `<soml::_impl::LocalDatetime::Wrapper::Field>`, `<soml::_impl::LocalDate::Wrapper::Field>`, `<soml::_impl::LocalTime::Wrapper::Field>`");

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 0,
            },
            Token::StructEnd,
        ];
        assert_de_tokens_error::<Datetime>(
            tokens,
            "invalid length 0, expected a date-time wrapper",
        );

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 3,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED),
            Token::Str(LocalDate::WRAPPER_FIELD),
        ];
        assert_de_tokens_error::<Datetime>(
            tokens,
            r#"expected Token::Str("<soml::_impl::LocalDate::Wrapper::Field>") but deserialization wants Token::StructEnd"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<Datetime>(
            tokens,
            "invalid type: integer `2`, expected a date-time wrapper",
        );
    }

    #[test]
    fn deserialize_offset_datetime() {
        let tokens = &[
            Token::Struct {
                name: OffsetDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&OffsetDatetime::EXAMPLE, tokens);

        let tokens = &[Token::Struct {
            name: "foo",
            len: 1,
        }];
        assert_de_tokens_error::<OffsetDatetime>(
            tokens,
            r#"expected Token::Struct { name: "foo", len: 1 } but deserialization wants Token::Struct { name: "<soml::_impl::OffsetDatetime::Wrapper>", len: 1 }"#,
        );

        let tokens = &[
            Token::Struct {
                name: OffsetDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str("bar"),
        ];
        assert_de_tokens_error::<OffsetDatetime>(
            tokens,
            "unknown field `bar`, expected `<soml::_impl::OffsetDatetime::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: OffsetDatetime::WRAPPER_TYPE,
                len: 0,
            },
            Token::StructEnd,
        ];
        assert_de_tokens_error::<OffsetDatetime>(
            tokens,
            "missing field `<soml::_impl::OffsetDatetime::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: OffsetDatetime::WRAPPER_TYPE,
                len: 3,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED),
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
        ];
        assert_de_tokens_error::<OffsetDatetime>(
            tokens,
            r#"expected Token::Str("<soml::_impl::OffsetDatetime::Wrapper::Field>") but deserialization wants Token::StructEnd"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<OffsetDatetime>(
            tokens,
            "invalid type: integer `2`, expected an offset date-time wrapper",
        );
    }

    #[test]
    fn deserialize_encoded_offset_datetime() {
        let tokens = &[Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens(&EncodedOffsetDatetime(OffsetDatetime::EXAMPLE), tokens);

        let tokens = &[Token::Bytes(LocalDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedOffsetDatetime>(
            tokens,
            "invalid length 12, expected an encoded offset date-time",
        );

        let tokens = &[Token::Bytes(LocalDate::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedOffsetDatetime>(
            tokens,
            "invalid length 4, expected an encoded offset date-time",
        );

        let tokens = &[Token::Bytes(LocalTime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedOffsetDatetime>(
            tokens,
            "invalid length 8, expected an encoded offset date-time",
        );

        let tokens = &[Token::Str("invalid string")];
        assert_de_tokens_error::<EncodedOffsetDatetime>(
            tokens,
            r#"invalid type: string "invalid string", expected an encoded offset date-time"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<EncodedOffsetDatetime>(
            tokens,
            "invalid type: integer `2`, expected an encoded offset date-time",
        );
    }

    #[test]
    fn offset_datetime_access() {
        let mut access = OffsetDatetimeAccess::from(OffsetDatetime::EXAMPLE);

        assert_matches!(access, OffsetDatetimeAccess(Some(OffsetDatetime::EXAMPLE)));

        assert_matches!(access.next_key(), Ok(Some(OffsetDatetime::WRAPPER_FIELD)));
        assert_matches!(access.next_value::<ByteBuf>(), Ok(b) if b == OffsetDatetime::EXAMPLE_ENCODED);

        assert_matches!(access.next_key::<&str>(), Ok(None));
    }

    #[test]
    #[should_panic = "OffsetDatetimeAccess::next_value called without calling OffsetDatetimeAccess::next_key first"]
    fn offset_datetime_access_empty() {
        let mut access = OffsetDatetimeAccess(None);

        let _result = access.next_value::<ByteBuf>();
    }

    #[test]
    fn deserialize_local_datetime() {
        let tokens = &[
            Token::Struct {
                name: LocalDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&LocalDatetime::EXAMPLE, tokens);

        let tokens = &[Token::Struct {
            name: "foo",
            len: 1,
        }];
        assert_de_tokens_error::<LocalDatetime>(
            tokens,
            r#"expected Token::Struct { name: "foo", len: 1 } but deserialization wants Token::Struct { name: "<soml::_impl::LocalDatetime::Wrapper>", len: 1 }"#,
        );

        let tokens = &[
            Token::Struct {
                name: LocalDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str("bar"),
        ];
        assert_de_tokens_error::<LocalDatetime>(
            tokens,
            "unknown field `bar`, expected `<soml::_impl::LocalDatetime::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: LocalDatetime::WRAPPER_TYPE,
                len: 0,
            },
            Token::StructEnd,
        ];
        assert_de_tokens_error::<LocalDatetime>(
            tokens,
            "missing field `<soml::_impl::LocalDatetime::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: LocalDatetime::WRAPPER_TYPE,
                len: 3,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_ENCODED),
            Token::Str(LocalDatetime::WRAPPER_FIELD),
        ];
        assert_de_tokens_error::<LocalDatetime>(
            tokens,
            r#"expected Token::Str("<soml::_impl::LocalDatetime::Wrapper::Field>") but deserialization wants Token::StructEnd"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<LocalDatetime>(
            tokens,
            "invalid type: integer `2`, expected a local date-time wrapper",
        );
    }

    #[test]
    fn deserialize_encoded_local_datetime() {
        let tokens = &[Token::Bytes(LocalDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens(&EncodedLocalDatetime(LocalDatetime::EXAMPLE), tokens);

        let tokens = &[Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalDatetime>(
            tokens,
            "invalid length 14, expected an encoded local date-time",
        );

        let tokens = &[Token::Bytes(LocalDate::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalDatetime>(
            tokens,
            "invalid length 4, expected an encoded local date-time",
        );

        let tokens = &[Token::Bytes(LocalTime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalDatetime>(
            tokens,
            "invalid length 8, expected an encoded local date-time",
        );

        let tokens = &[Token::Str("invalid string")];
        assert_de_tokens_error::<EncodedLocalDatetime>(
            tokens,
            r#"invalid type: string "invalid string", expected an encoded local date-time"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<EncodedLocalDatetime>(
            tokens,
            "invalid type: integer `2`, expected an encoded local date-time",
        );
    }

    #[test]
    fn local_datetime_access() {
        let mut access = LocalDatetimeAccess::from(LocalDatetime::EXAMPLE);

        assert_matches!(access, LocalDatetimeAccess(Some(LocalDatetime::EXAMPLE)));

        assert_matches!(access.next_key(), Ok(Some(LocalDatetime::WRAPPER_FIELD)));
        assert_matches!(access.next_value::<ByteBuf>(), Ok(b) if b == LocalDatetime::EXAMPLE_ENCODED);

        assert_matches!(access.next_key::<&str>(), Ok(None));
    }

    #[test]
    #[should_panic = "LocalDatetimeAccess::next_value called without calling LocalDatetimeAccess::next_key first"]
    fn local_datetime_access_empty() {
        let mut access = LocalDatetimeAccess(None);

        let _result = access.next_value::<ByteBuf>();
    }

    #[test]
    fn deserialize_local_date() {
        let tokens = &[
            Token::Struct {
                name: LocalDate::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&LocalDate::EXAMPLE, tokens);

        let tokens = &[Token::Struct {
            name: "foo",
            len: 1,
        }];
        assert_de_tokens_error::<LocalDate>(
            tokens,
            r#"expected Token::Struct { name: "foo", len: 1 } but deserialization wants Token::Struct { name: "<soml::_impl::LocalDate::Wrapper>", len: 1 }"#,
        );

        let tokens = &[
            Token::Struct {
                name: LocalDate::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str("bar"),
        ];
        assert_de_tokens_error::<LocalDate>(
            tokens,
            "unknown field `bar`, expected `<soml::_impl::LocalDate::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: LocalDate::WRAPPER_TYPE,
                len: 0,
            },
            Token::StructEnd,
        ];
        assert_de_tokens_error::<LocalDate>(
            tokens,
            "missing field `<soml::_impl::LocalDate::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: LocalDate::WRAPPER_TYPE,
                len: 3,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_ENCODED),
            Token::Str(LocalDate::WRAPPER_FIELD),
        ];
        assert_de_tokens_error::<LocalDate>(
            tokens,
            r#"expected Token::Str("<soml::_impl::LocalDate::Wrapper::Field>") but deserialization wants Token::StructEnd"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<LocalDate>(
            tokens,
            "invalid type: integer `2`, expected a local date wrapper",
        );
    }

    #[test]
    fn deserialize_encoded_local_date() {
        let tokens = &[Token::Bytes(LocalDate::EXAMPLE_ENCODED)];
        assert_de_tokens(&EncodedLocalDate(LocalDate::EXAMPLE), tokens);

        let tokens = &[Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalDate>(
            tokens,
            "invalid length 14, expected an encoded local date",
        );

        let tokens = &[Token::Bytes(LocalDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalDate>(
            tokens,
            "invalid length 12, expected an encoded local date",
        );

        let tokens = &[Token::Bytes(LocalTime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalDate>(
            tokens,
            "invalid length 8, expected an encoded local date",
        );

        let tokens = &[Token::Str("invalid string")];
        assert_de_tokens_error::<EncodedLocalDate>(
            tokens,
            r#"invalid type: string "invalid string", expected an encoded local date"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<EncodedLocalDate>(
            tokens,
            "invalid type: integer `2`, expected an encoded local date",
        );
    }

    #[test]
    fn local_date_access() {
        let mut access = LocalDateAccess::from(LocalDate::EXAMPLE);

        assert_matches!(access, LocalDateAccess(Some(LocalDate::EXAMPLE)));

        assert_matches!(access.next_key(), Ok(Some(LocalDate::WRAPPER_FIELD)));
        assert_matches!(access.next_value::<ByteBuf>(), Ok(b) if b == LocalDate::EXAMPLE_ENCODED);

        assert_matches!(access.next_key::<&str>(), Ok(None));
    }

    #[test]
    #[should_panic = "LocalDateAccess::next_value called without calling LocalDateAccess::next_key first"]
    fn local_date_access_empty() {
        let mut access = LocalDateAccess(None);

        let _result = access.next_value::<ByteBuf>();
    }

    #[test]
    fn deserialize_local_time() {
        let tokens = &[
            Token::Struct {
                name: LocalTime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_ENCODED),
            Token::StructEnd,
        ];
        assert_de_tokens(&LocalTime::EXAMPLE, tokens);

        let tokens = &[Token::Struct {
            name: "foo",
            len: 1,
        }];
        assert_de_tokens_error::<LocalTime>(
            tokens,
            r#"expected Token::Struct { name: "foo", len: 1 } but deserialization wants Token::Struct { name: "<soml::_impl::LocalTime::Wrapper>", len: 1 }"#,
        );

        let tokens = &[
            Token::Struct {
                name: LocalTime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str("bar"),
        ];
        assert_de_tokens_error::<LocalTime>(
            tokens,
            "unknown field `bar`, expected `<soml::_impl::LocalTime::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: LocalTime::WRAPPER_TYPE,
                len: 0,
            },
            Token::StructEnd,
        ];
        assert_de_tokens_error::<LocalTime>(
            tokens,
            "missing field `<soml::_impl::LocalTime::Wrapper::Field>`",
        );

        let tokens = &[
            Token::Struct {
                name: LocalTime::WRAPPER_TYPE,
                len: 3,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_ENCODED),
            Token::Str(LocalTime::WRAPPER_FIELD),
        ];
        assert_de_tokens_error::<LocalTime>(
            tokens,
            r#"expected Token::Str("<soml::_impl::LocalTime::Wrapper::Field>") but deserialization wants Token::StructEnd"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<LocalTime>(
            tokens,
            "invalid type: integer `2`, expected a local time wrapper",
        );
    }

    #[test]
    fn deserialize_encoded_local_time() {
        let tokens = &[Token::Bytes(LocalTime::EXAMPLE_ENCODED)];
        assert_de_tokens(&EncodedLocalTime(LocalTime::EXAMPLE), tokens);

        let tokens = &[Token::Bytes(OffsetDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalTime>(
            tokens,
            "invalid length 14, expected an encoded local time",
        );

        let tokens = &[Token::Bytes(LocalDatetime::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalTime>(
            tokens,
            "invalid length 12, expected an encoded local time",
        );

        let tokens = &[Token::Bytes(LocalDate::EXAMPLE_ENCODED)];
        assert_de_tokens_error::<EncodedLocalTime>(
            tokens,
            "invalid length 4, expected an encoded local time",
        );

        let tokens = &[Token::Str("invalid string")];
        assert_de_tokens_error::<EncodedLocalTime>(
            tokens,
            r#"invalid type: string "invalid string", expected an encoded local time"#,
        );

        let tokens = &[Token::I32(2)];
        assert_de_tokens_error::<EncodedLocalTime>(
            tokens,
            "invalid type: integer `2`, expected an encoded local time",
        );
    }

    #[test]
    fn local_time_access() {
        let mut access = LocalTimeAccess::from(LocalTime::EXAMPLE);

        assert_matches!(access, LocalTimeAccess(Some(LocalTime::EXAMPLE)));

        assert_matches!(access.next_key(), Ok(Some(LocalTime::WRAPPER_FIELD)));
        assert_matches!(access.next_value::<ByteBuf>(), Ok(b) if b == LocalTime::EXAMPLE_ENCODED);

        assert_matches!(access.next_key::<&str>(), Ok(None));
    }

    #[test]
    #[should_panic = "LocalTimeAccess::next_value called without calling LocalTimeAccess::next_key first"]
    fn local_time_access_empty() {
        let mut access = LocalTimeAccess(None);

        let _result = access.next_value::<ByteBuf>();
    }
}
