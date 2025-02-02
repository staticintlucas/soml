use std::fmt;
use std::str;

use lexical::{FromLexicalWithOptions, NumberFormatBuilder, ParseIntegerOptions};
use serde::de::Error as _;
use serde::de::Unexpected;

pub use self::de::{
    DatetimeAccess, LocalDateFromBytes, LocalDatetimeFromBytes, LocalTimeFromBytes,
    OffsetDatetimeFromBytes,
};
use crate::de::{Error, ErrorKind};

mod de;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Datetime {
    pub date: Option<LocalDate>,
    pub time: Option<LocalTime>,
    pub offset: Option<Offset>,
}

impl Datetime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::Datetime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::Datetime::Wrapper::Field>";

    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        if let Some((date, rest)) = split_once(bytes, |b| *b == b'T' || *b == b't' || *b == b' ') {
            let date = Some(LocalDate::from_slice(date)?);

            let (time, offset) =
                if let Some(off_pos) = rest.iter().position(|b| b"Zz+-".contains(b)) {
                    let time = LocalTime::from_slice(&rest[..off_pos])?;
                    let offset = Offset::from_slice(&rest[off_pos..])?;

                    (Some(time), Some(offset))
                } else {
                    let time = LocalTime::from_slice(rest)?;

                    (Some(time), None)
                };

            Ok(Self { date, time, offset })
        } else if bytes.contains(&b':') {
            let time = Some(LocalTime::from_slice(bytes)?);

            Ok(Self {
                date: None,
                time,
                offset: None,
            })
        } else {
            let date = Some(LocalDate::from_slice(bytes)?);

            Ok(Self {
                date,
                time: None,
                offset: None,
            })
        }
    }

    pub(crate) const fn type_str(&self) -> &'static str {
        match (self.date.as_ref(), self.time.as_ref(), self.offset.as_ref()) {
            (Some(_), Some(_), Some(_)) => "offset date-time",
            (Some(_), Some(_), None) => "local date-time",
            (Some(_), None, None) => "local date",
            (None, Some(_), None) => "local time",
            // Below are all "invalid" permutations
            (None, None, Some(_)) => "invalid date-time (offset without date or time)",
            (None, Some(_), Some(_)) => "invalid date-time (offset without date)",
            (Some(_), None, Some(_)) => "invalid date-time (offset without time)",
            (None, None, None) => "invalid date-time (no date, time, or offset)",
        }
    }
}

impl str::FromStr for Datetime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for Datetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.date.as_ref(), self.time.as_ref()) {
            (Some(date), Some(time)) => write!(f, "{date}T{time}"),
            (Some(date), None) => write!(f, "{date}"),
            (None, Some(time)) => write!(f, "{time}"),
            _ => Ok(()),
        }?;
        if let Some(offset) = self.offset.as_ref() {
            write!(f, "{offset}")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OffsetDatetime {
    pub date: LocalDate,
    pub time: LocalTime,
    pub offset: Offset,
}

impl OffsetDatetime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::OffsetDatetime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::OffsetDatetime::Wrapper::Field>";

    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let Some((date, rest)) = split_once(bytes, |b| b"Tt ".contains(b)) else {
            return Err(ErrorKind::InvalidDatetime.into());
        };
        let Some(off_pos) = rest.iter().position(|b| b"Zz+-".contains(b)) else {
            return Err(ErrorKind::InvalidDatetime.into());
        };

        let date = LocalDate::from_slice(date)?;
        let time = LocalTime::from_slice(&rest[..off_pos])?;
        let offset = Offset::from_slice(&rest[off_pos..])?;

        Ok(Self { date, time, offset })
    }
}

impl str::FromStr for OffsetDatetime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for OffsetDatetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ref date,
            ref time,
            ref offset,
        } = *self;
        write!(f, "{date}T{time}{offset}")
    }
}

impl From<OffsetDatetime> for Datetime {
    fn from(value: OffsetDatetime) -> Self {
        Self {
            date: Some(value.date),
            time: Some(value.time),
            offset: Some(value.offset),
        }
    }
}

impl TryFrom<Datetime> for OffsetDatetime {
    type Error = Error;

    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: Some(date),
            time: Some(time),
            offset: Some(offset),
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date-time",
            ));
        };
        Ok(Self { date, time, offset })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalDatetime {
    pub date: LocalDate,
    pub time: LocalTime,
}

impl LocalDatetime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::LocalDatetime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::LocalDatetime::Wrapper::Field>";

    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let Some((date, time)) = split_once(bytes, |b| *b == b'T' || *b == b' ') else {
            return Err(ErrorKind::InvalidDatetime.into());
        };

        let date = LocalDate::from_slice(date)?;
        let time = LocalTime::from_slice(time)?;

        Ok(Self { date, time })
    }
}

impl str::FromStr for LocalDatetime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for LocalDatetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { ref date, ref time } = *self;
        write!(f, "{date}T{time}")
    }
}

impl From<LocalDatetime> for Datetime {
    fn from(value: LocalDatetime) -> Self {
        Self {
            date: Some(value.date),
            time: Some(value.time),
            offset: None,
        }
    }
}

impl TryFrom<Datetime> for LocalDatetime {
    type Error = Error;

    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: Some(date),
            time: Some(time),
            offset: None,
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date-time",
            ));
        };
        Ok(Self { date, time })
    }
}

#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// Alias for `LocalDate` for compatibility with [`toml`]
///
/// [`toml`]: https://crates.io/crates/toml
pub type Date = LocalDate;

impl LocalDate {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::LocalDate::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::LocalDate::Wrapper::Field>";

    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        const FORMAT: u128 = NumberFormatBuilder::new()
            .no_positive_mantissa_sign(true)
            .build();
        const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

        let (year, (month, day)) = split_once(bytes, |b| *b == b'-')
            .and_then(|(year, rest)| Some((year, split_once(rest, |b| *b == b'-')?)))
            .ok_or(ErrorKind::InvalidDatetime)?;

        if year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return Err(ErrorKind::InvalidDatetime.into());
        }

        let year = u16::from_lexical_with_options::<FORMAT>(year, &OPTIONS)
            .map_err(|_| ErrorKind::InvalidDatetime)?;
        let month = u8::from_lexical_with_options::<FORMAT>(month, &OPTIONS)
            .map_err(|_| ErrorKind::InvalidDatetime)?;
        let day = u8::from_lexical_with_options::<FORMAT>(day, &OPTIONS)
            .map_err(|_| ErrorKind::InvalidDatetime)?;

        // #[cfg(not(feature = "fast"))]
        {
            let is_valid = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => (1..=31).contains(&day),
                4 | 6 | 9 | 11 => (1..=30).contains(&day),
                // Check for leap year
                2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => {
                    (1..=29).contains(&day)
                }
                2 => (1..=28).contains(&day),
                _ => false,
            };
            if !is_valid {
                return Err(ErrorKind::InvalidDatetime.into());
            }
        }

        Ok(Self { year, month, day })
    }
}

impl str::FromStr for LocalDate {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { year, month, day } = *self;
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

impl From<LocalDate> for Datetime {
    fn from(value: LocalDate) -> Self {
        Self {
            date: Some(value),
            time: None,
            offset: None,
        }
    }
}

impl TryFrom<Datetime> for LocalDate {
    type Error = Error;

    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: Some(date),
            time: None,
            offset: None,
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date-time",
            ));
        };
        Ok(date)
    }
}

#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
}

/// Alias for `LocalTime` for compatibility with [`toml`]
///
/// [`toml`]: https://crates.io/crates/toml
pub type Time = LocalTime;

impl LocalTime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::LocalTime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::LocalTime::Wrapper::Field>";

    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        const FORMAT: u128 = NumberFormatBuilder::new()
            .no_positive_mantissa_sign(true)
            .build();
        const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

        let (hour, (minute, second)) = split_once(bytes, |b| *b == b':')
            .and_then(|(hour, rest)| Some((hour, split_once(rest, |b| *b == b':')?)))
            .ok_or(ErrorKind::InvalidDatetime)?;

        if hour.len() != 2 || minute.len() != 2 {
            return Err(ErrorKind::InvalidDatetime.into());
        }
        let hour = u8::from_lexical_with_options::<FORMAT>(hour, &OPTIONS)
            .map_err(|_| ErrorKind::InvalidDatetime)?;
        let minute = u8::from_lexical_with_options::<FORMAT>(minute, &OPTIONS)
            .map_err(|_| ErrorKind::InvalidDatetime)?;

        let (second, fraction) =
            if let Some((second, fraction)) = split_once(second, |b| *b == b'.') {
                // The TOML spec requires at least milliseconds (6 digits) and truncate additional
                // digits. We support up to nanoseconds (9 digits) here and truncate the rest.
                (second, Some(fraction.get(..9).unwrap_or(fraction)))
            } else {
                (second, None)
            };

        if second.len() != 2 {
            return Err(ErrorKind::InvalidDatetime.into());
        }
        let second = u8::from_lexical_with_options::<FORMAT>(second, &OPTIONS)
            .map_err(|_| ErrorKind::InvalidDatetime)?;

        let nanosecond = if let Some(fraction) = fraction {
            if fraction.is_empty() {
                return Err(ErrorKind::InvalidDatetime.into());
            }
            let nanosecond = u32::from_lexical_with_options::<FORMAT>(fraction, &OPTIONS)
                .map_err(|_| ErrorKind::InvalidDatetime)?;

            // If we parsed <9 digits, we need to multiply by 10 for each digit we're short
            let extra_zeros = 9 - u32::try_from(fraction.len())
                .unwrap_or_else(|_| unreachable!("fraction <= 9 digits"));
            nanosecond * 10_u32.pow(extra_zeros)
        } else {
            0
        };

        #[cfg(not(feature = "fast"))]
        if hour >= 24 || minute >= 60 || second >= 61 {
            // second == 60 is valid for a leap second
            return Err(ErrorKind::InvalidDatetime.into());
        }

        Ok(Self {
            hour,
            minute,
            second,
            nanosecond,
        })
    }
}

impl str::FromStr for LocalTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for LocalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            hour,
            minute,
            second,
            nanosecond,
        } = *self;
        if nanosecond == 0 {
            write!(f, "{hour:02}:{minute:02}:{second:02}")
        } else {
            write!(
                f,
                "{hour:02}:{minute:02}:{second:02}.{}",
                format!("{nanosecond:09}").trim_end_matches('0')
            )
        }
    }
}

impl From<LocalTime> for Datetime {
    fn from(value: LocalTime) -> Self {
        Self {
            date: None,
            time: Some(value),
            offset: None,
        }
    }
}

impl TryFrom<Datetime> for LocalTime {
    type Error = Error;

    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: None,
            time: Some(time),
            offset: None,
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date-time",
            ));
        };
        Ok(time)
    }
}

#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Offset {
    Z,
    Custom { minutes: i16 },
}

impl Offset {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        const FORMAT: u128 = NumberFormatBuilder::new()
            .no_positive_mantissa_sign(true)
            .build();
        const OPTIONS: ParseIntegerOptions = ParseIntegerOptions::new();

        if bytes == b"Z" || bytes == b"z" {
            Ok(Self::Z)
        } else {
            let (sign, bytes) = bytes.split_first().ok_or(ErrorKind::InvalidDatetime)?;
            let sign = match *sign {
                b'+' => 1,
                b'-' => -1,
                _ => return Err(ErrorKind::InvalidDatetime.into()),
            };

            let (hours, minutes) =
                split_once(bytes, |b| *b == b':').ok_or(ErrorKind::InvalidDatetime)?;
            if hours.len() != 2 && minutes.len() != 2 {
                return Err(ErrorKind::InvalidDatetime.into());
            }
            let hours = i16::from_lexical_with_options::<FORMAT>(hours, &OPTIONS)
                .map_err(|_| ErrorKind::InvalidDatetime)?;
            let minutes = i16::from_lexical_with_options::<FORMAT>(minutes, &OPTIONS)
                .map_err(|_| ErrorKind::InvalidDatetime)?;

            #[cfg(not(feature = "fast"))]
            if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
                return Err(ErrorKind::InvalidDatetime.into());
            }

            let minutes = sign * (hours * 60 + minutes);
            Ok(Self::Custom { minutes })
        }
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Z => write!(f, "Z"),
            Self::Custom { minutes } => {
                let sign = if minutes < 0 { "-" } else { "+" };
                let minutes = minutes.abs();
                let (hours, minutes) = (minutes / 60, minutes % 60);
                write!(f, "{sign}{hours:02}:{minutes:02}")
            }
        }
    }
}

impl str::FromStr for Offset {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

// TODO use <[T]>::split_once when it's stable
fn split_once<F>(bytes: &[u8], predicate: F) -> Option<(&[u8], &[u8])>
where
    F: FnMut(&u8) -> bool,
{
    let position = bytes.iter().position(predicate)?;
    Some((&bytes[..position], &bytes[position + 1..]))
}
