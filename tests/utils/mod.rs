#![allow(clippy::panic, clippy::unwrap_used)]

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Offset as _};

#[derive(Debug, serde::Deserialize)]
pub struct EncodedValue {
    #[serde(rename = "type")]
    typ: String,
    value: String,
}

#[derive(Debug, serde::Deserialize)]
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

impl ChronoDatetime {
    fn from_encoded_value(value: &EncodedValue) -> Self {
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

    fn from_datatime(datetime: &soml::value::Datetime) -> Self {
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
                ChronoDatetime::from_datatime(datetime) == ChronoDatetime::from_encoded_value(self)
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
