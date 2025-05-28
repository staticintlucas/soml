use serde::ser;
use serde::ser::Error as _;
use serde_bytes::ByteBuf;

use super::{AnyDatetime, Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

impl ser::Serialize for AnyDatetime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        match *self {
            Self::OffsetDatetime(ref datetime) => s.serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                &ByteBuf::from(datetime.to_bytes()),
            ),
            Self::LocalDatetime(ref datetime) => s.serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                &ByteBuf::from(datetime.to_bytes()),
            ),
            Self::LocalDate(ref date) => {
                s.serialize_field(LocalDate::WRAPPER_FIELD, &ByteBuf::from(date.to_bytes()))
            }
            Self::LocalTime(ref time) => {
                s.serialize_field(LocalTime::WRAPPER_FIELD, &ByteBuf::from(time.to_bytes()))
            }
        }?;
        s.end()
    }
}

impl ser::Serialize for Datetime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        AnyDatetime::try_from(self.clone())
            .map_err(S::Error::custom)
            .and_then(|dt| dt.serialize(serializer))
    }
}

impl ser::Serialize for OffsetDatetime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, &ByteBuf::from(self.to_bytes()))?;
        s.end()
    }
}

impl ser::Serialize for LocalDatetime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, &ByteBuf::from(self.to_bytes()))?;
        s.end()
    }
}

impl ser::Serialize for LocalDate {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, &ByteBuf::from(self.to_bytes()))?;
        s.end()
    }
}

impl ser::Serialize for LocalTime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, &ByteBuf::from(self.to_bytes()))?;
        s.end()
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use serde_test::{assert_ser_tokens, assert_ser_tokens_error, Token};

    use super::*;

    #[test]
    fn serialize_any_datetime() {
        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&AnyDatetime::EXAMPLE_OFFSET_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&AnyDatetime::EXAMPLE_LOCAL_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&AnyDatetime::EXAMPLE_LOCAL_DATE, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&AnyDatetime::EXAMPLE_LOCAL_TIME, tokens);
    }

    #[test]
    fn serialize_datetime() {
        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&Datetime::EXAMPLE_OFFSET_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&Datetime::EXAMPLE_LOCAL_DATETIME, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&Datetime::EXAMPLE_LOCAL_DATE, tokens);

        let tokens = &[
            Token::Struct {
                name: AnyDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&Datetime::EXAMPLE_LOCAL_TIME, tokens);

        let tokens = &[];
        assert_ser_tokens_error(
            &Datetime::EXAMPLE_INVALID_1,
            tokens,
            "invalid value: invalid date-time (offset with neither date nor time), expected a valid date-time",
        );

        let tokens = &[];
        assert_ser_tokens_error(
            &Datetime::EXAMPLE_INVALID_2,
            tokens,
            "invalid value: invalid date-time (offset date without time), expected a valid date-time",
        );

        let tokens = &[];
        assert_ser_tokens_error(
            &Datetime::EXAMPLE_INVALID_3,
            tokens,
            "invalid value: invalid date-time (offset time without date), expected a valid date-time",
        );

        let tokens = &[];
        assert_ser_tokens_error(
            &Datetime::EXAMPLE_INVALID_4,
            tokens,
            "invalid value: invalid date-time (no date, time, nor offset), expected a valid date-time",
        );
    }

    #[test]
    fn serialize_offset_datetime() {
        let tokens = &[
            Token::Struct {
                name: OffsetDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(OffsetDatetime::WRAPPER_FIELD),
            Token::Bytes(OffsetDatetime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&OffsetDatetime::EXAMPLE, tokens);
    }

    #[test]
    fn serialize_local_datetime() {
        let tokens = &[
            Token::Struct {
                name: LocalDatetime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDatetime::WRAPPER_FIELD),
            Token::Bytes(LocalDatetime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&LocalDatetime::EXAMPLE, tokens);
    }

    #[test]
    fn serialize_local_date() {
        let tokens = &[
            Token::Struct {
                name: LocalDate::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalDate::WRAPPER_FIELD),
            Token::Bytes(LocalDate::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&LocalDate::EXAMPLE, tokens);
    }

    #[test]
    fn serialize_local_time() {
        let tokens = &[
            Token::Struct {
                name: LocalTime::WRAPPER_TYPE,
                len: 1,
            },
            Token::Str(LocalTime::WRAPPER_FIELD),
            Token::Bytes(LocalTime::EXAMPLE_BYTES),
            Token::StructEnd,
        ];
        assert_ser_tokens(&LocalTime::EXAMPLE, tokens);
    }
}
