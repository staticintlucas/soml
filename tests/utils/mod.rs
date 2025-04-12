#![allow(clippy::panic, clippy::unwrap_used, clippy::fallible_impl_from)]

use std::collections::HashMap;

use chrono::{
    DateTime, Datelike as _, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Offset as _,
    Timelike as _,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EncodedValue {
    #[serde(rename = "type")]
    typ: String,
    value: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum EncodedItem {
    Value(EncodedValue),
    Table(HashMap<String, EncodedItem>),
    Array(Vec<EncodedItem>),
}

#[derive(Debug, PartialEq)]
enum ChronoDatetime {
    OffsetDatetime(DateTime<FixedOffset>),
    LocalDatetime(NaiveDateTime),
    LocalDate(NaiveDate),
    LocalTime(NaiveTime),
}

impl From<EncodedValue> for ChronoDatetime {
    fn from(value: EncodedValue) -> Self {
        match value.typ.as_str() {
            "datetime" => Self::OffsetDatetime(DateTime::parse_from_rfc3339(&value.value).unwrap()),
            "datetime-local" => Self::LocalDatetime(
                NaiveDateTime::parse_from_str(&value.value, "%FT%T%.f").unwrap(),
            ),
            "date-local" => Self::LocalDate(NaiveDate::parse_from_str(&value.value, "%F").unwrap()),
            "time-local" => {
                Self::LocalTime(NaiveTime::parse_from_str(&value.value, "%T%.f").unwrap())
            }
            _ => panic!("not a datetime type"),
        }
    }
}

impl From<soml::value::Datetime> for ChronoDatetime {
    fn from(datetime: soml::value::Datetime) -> Self {
        match (
            datetime.date.as_ref(),
            datetime.time.as_ref(),
            datetime.offset.as_ref(),
        ) {
            (Some(date), Some(time), Some(offset)) => Self::OffsetDatetime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())
                        .unwrap(),
                    NaiveTime::from_hms_nano_opt(
                        time.hour.into(),
                        time.minute.into(),
                        time.second.into(),
                        time.nanosecond,
                    )
                    .unwrap(),
                )
                .and_local_timezone(match *offset {
                    soml::value::Offset::Z => chrono::Utc.fix(),
                    soml::value::Offset::Custom { minutes } => {
                        FixedOffset::east_opt(i32::from(minutes) * 60).unwrap()
                    }
                })
                .unwrap(),
            ),
            (Some(date), Some(time), None) => Self::LocalDatetime(NaiveDateTime::new(
                NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())
                    .unwrap(),
                NaiveTime::from_hms_nano_opt(
                    time.hour.into(),
                    time.minute.into(),
                    time.second.into(),
                    time.nanosecond,
                )
                .unwrap(),
            )),
            (Some(date), None, None) => Self::LocalDate(
                NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())
                    .unwrap(),
            ),
            (None, Some(time), None) => Self::LocalTime(
                NaiveTime::from_hms_nano_opt(
                    time.hour.into(),
                    time.minute.into(),
                    time.second.into(),
                    time.nanosecond,
                )
                .unwrap(),
            ),
            _ => panic!("invalid datetime"),
        }
    }
}

impl From<ChronoDatetime> for soml::value::Datetime {
    fn from(value: ChronoDatetime) -> Self {
        match value {
            ChronoDatetime::OffsetDatetime(datetime) => Self {
                date: Some(soml::value::LocalDate {
                    year: datetime.year().try_into().unwrap(),
                    month: datetime.month().try_into().unwrap(),
                    day: datetime.day().try_into().unwrap(),
                }),
                time: Some(soml::value::LocalTime {
                    hour: datetime.hour().try_into().unwrap(),
                    minute: datetime.minute().try_into().unwrap(),
                    second: datetime.second().try_into().unwrap(),
                    nanosecond: datetime.nanosecond(),
                }),
                offset: {
                    let minutes = (datetime.offset().local_minus_utc() / 60)
                        .try_into()
                        .unwrap();
                    Some(if minutes == 0 {
                        soml::value::Offset::Z
                    } else {
                        soml::value::Offset::Custom { minutes }
                    })
                },
            },
            ChronoDatetime::LocalDatetime(datetime) => Self {
                date: Some(soml::value::LocalDate {
                    year: datetime.year().try_into().unwrap(),
                    month: datetime.month().try_into().unwrap(),
                    day: datetime.day().try_into().unwrap(),
                }),
                time: Some(soml::value::LocalTime {
                    hour: datetime.hour().try_into().unwrap(),
                    minute: datetime.minute().try_into().unwrap(),
                    second: datetime.second().try_into().unwrap(),
                    nanosecond: datetime.nanosecond(),
                }),
                offset: None,
            },
            ChronoDatetime::LocalDate(datetime) => Self {
                date: Some(soml::value::LocalDate {
                    year: datetime.year().try_into().unwrap(),
                    month: datetime.month().try_into().unwrap(),
                    day: datetime.day().try_into().unwrap(),
                }),
                time: None,
                offset: None,
            },
            ChronoDatetime::LocalTime(datetime) => Self {
                date: None,
                time: Some(soml::value::LocalTime {
                    hour: datetime.hour().try_into().unwrap(),
                    minute: datetime.minute().try_into().unwrap(),
                    second: datetime.second().try_into().unwrap(),
                    nanosecond: datetime.nanosecond(),
                }),
                offset: None,
            },
        }
    }
}

impl PartialEq<soml::Value> for EncodedValue {
    fn eq(&self, value: &soml::Value) -> bool {
        match *value {
            soml::Value::String(ref str) => self.typ == "string" && self.value == *str,
            soml::Value::Integer(int) => {
                self.typ == "integer" && self.value.parse::<i64>().is_ok_and(|v| v == int)
            }
            soml::Value::Float(float) => {
                self.typ == "float"
                    && self
                        .value
                        .parse::<f64>()
                        .is_ok_and(|v| (v.is_nan() && float.is_nan()) || (v == float))
            }
            soml::Value::Boolean(bool) => {
                self.typ == "bool" && self.value.parse::<bool>().is_ok_and(|v| v == bool)
            }
            soml::Value::Datetime(ref datetime) => {
                ChronoDatetime::from(datetime.clone()) == ChronoDatetime::from(self.clone())
            }
            soml::Value::Array(_) | soml::Value::Table(_) => false,
        }
    }
}

impl PartialEq<EncodedValue> for soml::Value {
    fn eq(&self, value: &EncodedValue) -> bool {
        value.eq(self)
    }
}

impl PartialEq<soml::Value> for EncodedItem {
    fn eq(&self, value: &soml::Value) -> bool {
        match *self {
            Self::Value(ref enc_value) => enc_value == value,
            Self::Table(ref enc_table) => value.as_table().is_some_and(|table| {
                if table.len() != enc_table.len() {
                    return false;
                }
                table
                    .iter()
                    .all(|(key, value)| enc_table.get(key).is_some_and(|v| *value == *v))
            }),
            Self::Array(ref enc_array) => {
                value.as_array().is_some_and(|array| *enc_array == *array)
            }
        }
    }
}

impl PartialEq<EncodedItem> for soml::Value {
    fn eq(&self, value: &EncodedItem) -> bool {
        value.eq(self)
    }
}

impl From<EncodedValue> for soml::Value {
    fn from(value: EncodedValue) -> Self {
        match value.typ.as_str() {
            "string" => Self::String(value.value),
            "integer" => Self::Integer(value.value.parse().unwrap()),
            "float" => Self::Float(value.value.parse().unwrap()),
            "bool" => Self::Boolean(value.value.parse().unwrap()),
            "datetime" | "datetime-local" | "date-local" | "time-local" => {
                Self::Datetime(ChronoDatetime::from(value).into())
            }
            _ => panic!("not a valid value type"),
        }
    }
}

impl From<EncodedItem> for soml::Value {
    fn from(value: EncodedItem) -> Self {
        match value {
            EncodedItem::Value(value) => value.into(),
            EncodedItem::Table(table) => {
                Self::Table(table.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
            EncodedItem::Array(array) => Self::Array(array.into_iter().map(Into::into).collect()),
        }
    }
}
