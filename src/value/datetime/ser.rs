use serde::ser;
use serde::ser::Error as _;

use super::{Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

impl ser::Serialize for Datetime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match (self.date.clone(), self.time.clone(), self.offset.clone()) {
            (Some(date), Some(time), Some(offset)) => {
                OffsetDatetime { date, time, offset }.serialize(serializer)
            }
            (Some(date), Some(time), None) => LocalDatetime { date, time }.serialize(serializer),
            (Some(date), None, None) => date.serialize(serializer),
            (None, Some(time), None) => time.serialize(serializer),
            _ => Err(S::Error::custom(format_args!(
                "invalid value: {}, expected a valid date-time",
                self.type_str()
            ))),
        }
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
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
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
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
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
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
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
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
        s.end()
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
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
    fn serialize_datetime() {
        let result = serde_json::to_string(&Datetime::from(OFFSET_DATETIME)).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::OffsetDatetime::Wrapper::Field>":"2023-01-02T03:04:05.006+07:08"}"#
        );

        let result = serde_json::to_string(&Datetime::from(LOCAL_DATETIME)).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::LocalDatetime::Wrapper::Field>":"2023-01-02T03:04:05.006"}"#
        );

        let result = serde_json::to_string(&Datetime::from(DATE)).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::LocalDate::Wrapper::Field>":"2023-01-02"}"#
        );

        let result = serde_json::to_string(&Datetime::from(TIME)).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::LocalTime::Wrapper::Field>":"03:04:05.006"}"#
        );

        let result = serde_json::to_string(&Datetime {
            date: None,
            time: None,
            offset: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn serialize_offset_datetime() {
        let result = serde_json::to_string(&OFFSET_DATETIME).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::OffsetDatetime::Wrapper::Field>":"2023-01-02T03:04:05.006+07:08"}"#
        );
    }

    #[test]
    fn serialize_local_datetime() {
        let result = serde_json::to_string(&LOCAL_DATETIME).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::LocalDatetime::Wrapper::Field>":"2023-01-02T03:04:05.006"}"#
        );
    }

    #[test]
    fn serialize_local_date() {
        let result = serde_json::to_string(&DATE).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::LocalDate::Wrapper::Field>":"2023-01-02"}"#
        );
    }

    #[test]
    fn serialize_local_time() {
        let result = serde_json::to_string(&TIME).unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::LocalTime::Wrapper::Field>":"03:04:05.006"}"#
        );
    }
}
