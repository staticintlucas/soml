use std::str::FromStr as _;
use std::{fmt, str};

use serde::de::{Error as _, Unexpected};

pub use self::de::{
    EncodedLocalDate, EncodedLocalDatetime, EncodedLocalTime, EncodedOffsetDatetime,
    LocalDateAccess, LocalDatetimeAccess, LocalTimeAccess, OffsetDatetimeAccess,
};
use crate::de::{Error, ErrorKind};

mod de;
mod ser;

/// A generic TOML date-time enum.
///
/// This struct can represent any of the TOML date-time types, depending on the variant.
///
/// If compatibility with the [`toml`] crate is required, use the [`Datetime`] struct instead.
///
/// When working with a known date-time type, one of the [`OffsetDatetime`], [`LocalDatetime`],
/// [`LocalDate`], or [`LocalTime`] types can be used.
///
/// [`toml`]: https://crates.io/crates/toml
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AnyDatetime {
    /// A TOML offset date-time value.
    OffsetDatetime(OffsetDatetime),
    /// A TOML local date-time value.
    LocalDatetime(LocalDatetime),
    /// A TOML local date value.
    LocalDate(LocalDate),
    /// A TOML local time value.
    LocalTime(LocalTime),
}

impl AnyDatetime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::AnyDatetime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::AnyDatetime::Wrapper::Field>";

    #[cfg(test)]
    pub(crate) const EXAMPLE_OFFSET_DATETIME: Self = Self::OffsetDatetime(OffsetDatetime::EXAMPLE);
    #[cfg(test)]
    pub(crate) const EXAMPLE_LOCAL_DATETIME: Self = Self::LocalDatetime(LocalDatetime::EXAMPLE);
    #[cfg(test)]
    pub(crate) const EXAMPLE_LOCAL_DATE: Self = Self::LocalDate(LocalDate::EXAMPLE);
    #[cfg(test)]
    pub(crate) const EXAMPLE_LOCAL_TIME: Self = Self::LocalTime(LocalTime::EXAMPLE);

    /// Parses a [`AnyDatetime`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid TOML date-time value.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        if let Some(position) = bytes.iter().position(|b| b"Tt ".contains(b)) {
            let (date, rest) = (&bytes[..position], &bytes[position + 1..]);

            let date = LocalDate::from_slice(date)?;

            if let Some(off_pos) = rest.iter().position(|b| b"Zz+-".contains(b)) {
                let time = LocalTime::from_slice(&rest[..off_pos])?;
                let offset = Offset::from_slice(&rest[off_pos..])?;

                Ok(Self::OffsetDatetime(OffsetDatetime { date, time, offset }))
            } else {
                let time = LocalTime::from_slice(rest)?;

                Ok(Self::LocalDatetime(LocalDatetime { date, time }))
            }
        } else if bytes.contains(&b':') {
            let time = LocalTime::from_slice(bytes)?;

            Ok(Self::LocalTime(time))
        } else {
            let date = LocalDate::from_slice(bytes)?;

            Ok(Self::LocalDate(date))
        }
    }

    #[inline]
    pub(crate) const fn type_str(&self) -> &'static str {
        match *self {
            Self::OffsetDatetime(_) => "offset date-time",
            Self::LocalDatetime(_) => "local date-time",
            Self::LocalDate(_) => "local date",
            Self::LocalTime(_) => "local time",
        }
    }
}

impl str::FromStr for AnyDatetime {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for AnyDatetime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OffsetDatetime(ref datetime) => datetime.fmt(f),
            Self::LocalDatetime(ref datetime) => datetime.fmt(f),
            Self::LocalDate(ref date) => date.fmt(f),
            Self::LocalTime(ref time) => time.fmt(f),
        }
    }
}

/// A TOML date-time value.
///
/// This struct can represent any of the TOML date-time types, depending on which fields are
/// [`Some`]:
///
/// | `date`   | `time`   | `offset` | type             |
/// |:---------|:---------|:---------|:-----------------|
/// | [`Some`] | [`Some`] | [`Some`] | Offset date-time |
/// | [`Some`] | [`Some`] | [`None`] | Local date-time  |
/// | [`Some`] | [`None`] | [`None`] | Local date       |
/// | [`None`] | [`Some`] | [`None`] | Local time       |
///
/// All other combinations are considered invalid
///
/// If compatibility with the [`toml`] crate is not needed, the [`AnyDatetime`] enum is recommended
/// instead. Or when working with a known date-time type, one of the [`OffsetDatetime`],
/// [`LocalDatetime`], [`LocalDate`], or [`LocalTime`] types should be used.
///
/// [`toml`]: https://crates.io/crates/toml
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Datetime {
    /// The date portion of the date-time value.
    pub date: Option<LocalDate>,
    /// The time portion of the date-time value.
    pub time: Option<LocalTime>,
    /// The UTC offset of the date-time value.
    pub offset: Option<Offset>,
}

impl Datetime {
    #[cfg(test)]
    pub(crate) const EXAMPLE_OFFSET_DATETIME: Self = Self {
        date: Some(LocalDate::EXAMPLE),
        time: Some(LocalTime::EXAMPLE),
        offset: Some(Offset::EXAMPLE),
    };
    #[cfg(test)]
    pub(crate) const EXAMPLE_LOCAL_DATETIME: Self = Self {
        date: Some(LocalDate::EXAMPLE),
        time: Some(LocalTime::EXAMPLE),
        offset: None,
    };
    #[cfg(test)]
    pub(crate) const EXAMPLE_LOCAL_DATE: Self = Self {
        date: Some(LocalDate::EXAMPLE),
        time: None,
        offset: None,
    };
    #[cfg(test)]
    pub(crate) const EXAMPLE_LOCAL_TIME: Self = Self {
        date: None,
        time: Some(LocalTime::EXAMPLE),
        offset: None,
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_INVALID_1: Self = Self {
        date: None,
        time: None,
        offset: Some(Offset::EXAMPLE),
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_INVALID_2: Self = Self {
        date: Some(LocalDate::EXAMPLE),
        time: None,
        offset: Some(Offset::EXAMPLE),
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_INVALID_3: Self = Self {
        date: None,
        time: Some(LocalTime::EXAMPLE),
        offset: Some(Offset::EXAMPLE),
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_INVALID_4: Self = Self {
        date: None,
        time: None,
        offset: None,
    };

    /// Parses a [`Datetime`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid TOML date-time value.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        match AnyDatetime::from_slice(bytes)? {
            AnyDatetime::OffsetDatetime(OffsetDatetime { date, time, offset }) => Ok(Self {
                date: Some(date),
                time: Some(time),
                offset: Some(offset),
            }),
            AnyDatetime::LocalDatetime(LocalDatetime { date, time }) => Ok(Self {
                date: Some(date),
                time: Some(time),
                offset: None,
            }),
            AnyDatetime::LocalDate(date) => Ok(Self {
                date: Some(date),
                time: None,
                offset: None,
            }),
            AnyDatetime::LocalTime(time) => Ok(Self {
                date: None,
                time: Some(time),
                offset: None,
            }),
        }
    }

    #[inline]
    pub(crate) const fn type_str(&self) -> &'static str {
        match (self.date.as_ref(), self.time.as_ref(), self.offset.as_ref()) {
            (Some(_), Some(_), Some(_)) => "offset date-time",
            (Some(_), Some(_), None) => "local date-time",
            (Some(_), None, None) => "local date",
            (None, Some(_), None) => "local time",
            // Below are all "invalid" permutations
            (None, None, Some(_)) => "invalid date-time (offset with neither date nor time)",
            (None, Some(_), Some(_)) => "invalid date-time (offset time without date)",
            (Some(_), None, Some(_)) => "invalid date-time (offset date without time)",
            (None, None, None) => "invalid date-time (no date, time, nor offset)",
        }
    }
}

impl str::FromStr for Datetime {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for Datetime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.date.as_ref(), self.time.as_ref(), self.offset.as_ref()) {
            (Some(date), Some(time), Some(offset)) => write!(f, "{date}T{time}{offset}"),
            (Some(date), Some(time), None) => write!(f, "{date}T{time}"),
            (Some(date), None, None) => write!(f, "{date}"),
            (None, Some(time), None) => write!(f, "{time}"),
            _ => write!(f, "<{}>", self.type_str()),
        }
    }
}

impl From<AnyDatetime> for Datetime {
    #[inline]
    fn from(value: AnyDatetime) -> Self {
        match value {
            AnyDatetime::OffsetDatetime(OffsetDatetime { date, time, offset }) => Self {
                date: Some(date),
                time: Some(time),
                offset: Some(offset),
            },
            AnyDatetime::LocalDatetime(LocalDatetime { date, time }) => Self {
                date: Some(date),
                time: Some(time),
                offset: None,
            },
            AnyDatetime::LocalDate(date) => Self {
                date: Some(date),
                time: None,
                offset: None,
            },
            AnyDatetime::LocalTime(time) => Self {
                date: None,
                time: Some(time),
                offset: None,
            },
        }
    }
}

impl TryFrom<Datetime> for AnyDatetime {
    type Error = Error;

    #[inline]
    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        match (value.date, value.time, value.offset) {
            (Some(date), Some(time), Some(offset)) => {
                Ok(Self::OffsetDatetime(OffsetDatetime { date, time, offset }))
            }
            (Some(date), Some(time), None) => Ok(Self::LocalDatetime(LocalDatetime { date, time })),
            (Some(date), None, None) => Ok(Self::LocalDate(date)),
            (None, Some(time), None) => Ok(Self::LocalTime(time)),
            (date, time, offset) => Err(Error::invalid_value(
                Unexpected::Other(Datetime { date, time, offset }.type_str()),
                &"a valid date-time",
            )),
        }
    }
}

/// A TOML offset date-time value.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OffsetDatetime {
    /// The date portion of the date-time value.
    pub date: LocalDate,
    /// The time portion of the date-time value.
    pub time: LocalTime,
    /// The UTC offset of the date-time value.
    pub offset: Offset,
}

impl OffsetDatetime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::OffsetDatetime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::OffsetDatetime::Wrapper::Field>";

    #[cfg(test)]
    pub(crate) const EXAMPLE: Self = Self {
        date: LocalDate::EXAMPLE,
        time: LocalTime::EXAMPLE,
        offset: Offset::EXAMPLE,
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_ENCODED: &[u8; 14] =
        b"\x80\x8d\x5b\x00\x03\x04\x05\x00\xe7\x07\x01\x02\xac\x01";

    #[cfg(test)]
    pub(crate) const EXAMPLE_STR: &str = "2023-01-02T03:04:05.006+07:08";

    /// Parses a [`OffsetDatetime`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid TOML offset date-time value.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let position = bytes
            .iter()
            .position(|b| b"Tt ".contains(b))
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (date, rest) = (&bytes[..position], &bytes[position + 1..]);

        let position = rest
            .iter()
            .position(|b| b"Zz+-".contains(b))
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (time, offset) = (&rest[..position], &rest[position..]);

        let date = LocalDate::from_slice(date)?;
        let time = LocalTime::from_slice(time)?;
        let offset = Offset::from_slice(offset)?;

        Ok(Self { date, time, offset })
    }

    #[inline]
    pub(crate) fn from_encoded(bytes: [u8; 14]) -> Self {
        Self {
            time: LocalTime::from_encoded(
                bytes[0..8].try_into().unwrap_or_else(|_| unreachable!()),
            ),
            date: LocalDate::from_encoded(
                bytes[8..12].try_into().unwrap_or_else(|_| unreachable!()),
            ),
            offset: Offset::from_encoded(
                bytes[12..14].try_into().unwrap_or_else(|_| unreachable!()),
            ),
        }
    }

    #[inline]
    pub(crate) fn to_encoded(&self) -> [u8; 14] {
        let mut bytes = [0; 14];
        bytes[0..8].copy_from_slice(&self.time.to_encoded());
        bytes[8..12].copy_from_slice(&self.date.to_encoded());
        bytes[12..14].copy_from_slice(&self.offset.to_encoded());
        bytes
    }
}

impl str::FromStr for OffsetDatetime {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for OffsetDatetime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ref date,
            ref time,
            ref offset,
        } = *self;
        write!(f, "{date}T{time}{offset}")
    }
}

impl From<OffsetDatetime> for AnyDatetime {
    #[inline]
    fn from(value: OffsetDatetime) -> Self {
        Self::OffsetDatetime(value)
    }
}

impl TryFrom<AnyDatetime> for OffsetDatetime {
    type Error = Error;

    #[inline]
    fn try_from(value: AnyDatetime) -> Result<Self, Self::Error> {
        if let AnyDatetime::OffsetDatetime(datetime) = value {
            Ok(datetime)
        } else {
            Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"an offset date-time",
            ))
        }
    }
}

impl From<OffsetDatetime> for Datetime {
    #[inline]
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

    #[inline]
    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: Some(date),
            time: Some(time),
            offset: Some(offset),
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"an offset date-time",
            ));
        };
        Ok(Self { date, time, offset })
    }
}

/// A TOML local date-time value.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalDatetime {
    /// The date portion of the date-time value.
    pub date: LocalDate,
    /// The time portion of the date-time value.
    pub time: LocalTime,
}

impl LocalDatetime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::LocalDatetime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::LocalDatetime::Wrapper::Field>";

    #[cfg(test)]
    pub(crate) const EXAMPLE: Self = Self {
        date: LocalDate::EXAMPLE,
        time: LocalTime::EXAMPLE,
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_ENCODED: &[u8; 12] =
        b"\x80\x8d\x5b\x00\x03\x04\x05\x00\xe7\x07\x01\x02";

    #[cfg(test)]
    pub(crate) const EXAMPLE_STR: &str = "2023-01-02T03:04:05.006";

    /// Parses a [`LocalDatetime`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid TOML local date-time value.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let position = bytes
            .iter()
            .position(|b| b"Tt ".contains(b))
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (date, time) = (&bytes[..position], &bytes[position + 1..]);

        let date = LocalDate::from_slice(date)?;
        let time = LocalTime::from_slice(time)?;

        Ok(Self { date, time })
    }

    #[inline]
    pub(crate) fn from_encoded(bytes: [u8; 12]) -> Self {
        Self {
            time: LocalTime::from_encoded(
                bytes[0..8].try_into().unwrap_or_else(|_| unreachable!()),
            ),
            date: LocalDate::from_encoded(
                bytes[8..12].try_into().unwrap_or_else(|_| unreachable!()),
            ),
        }
    }

    #[inline]
    pub(crate) fn to_encoded(&self) -> [u8; 12] {
        let mut bytes = [0; 12];
        bytes[0..8].copy_from_slice(&self.time.to_encoded());
        bytes[8..12].copy_from_slice(&self.date.to_encoded());
        bytes
    }
}

impl str::FromStr for LocalDatetime {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for LocalDatetime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { ref date, ref time } = *self;
        write!(f, "{date}T{time}")
    }
}

impl From<LocalDatetime> for AnyDatetime {
    #[inline]
    fn from(value: LocalDatetime) -> Self {
        Self::LocalDatetime(value)
    }
}

impl TryFrom<AnyDatetime> for LocalDatetime {
    type Error = Error;

    #[inline]
    fn try_from(value: AnyDatetime) -> Result<Self, Self::Error> {
        if let AnyDatetime::LocalDatetime(datetime) = value {
            Ok(datetime)
        } else {
            Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date-time",
            ))
        }
    }
}

impl From<LocalDatetime> for Datetime {
    #[inline]
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

    #[inline]
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

/// A TOML local date value.
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalDate {
    /// The year.
    ///
    /// This is always between 0--9999. BCE dates and dates after 9999 CE are not supported by
    /// [RFC 3339].
    ///
    /// [rfc 3339]: https://tools.ietf.org/html/rfc3339
    pub year: u16,
    /// The month of the year.
    ///
    /// This is always between 1--12 (inclusive).
    pub month: u8,
    /// The day of the month.
    ///
    /// This is always between 1--31 (inclusive), and is further restricted by the number of days
    /// in the given month.
    pub day: u8,
}

/// Alias for `LocalDate` for compatibility with [`toml`]
///
/// [`toml`]: https://crates.io/crates/toml
pub type Date = LocalDate;

impl LocalDate {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::LocalDate::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::LocalDate::Wrapper::Field>";

    #[cfg(test)]
    pub(crate) const EXAMPLE: Self = Self {
        year: 2023,
        month: 1,
        day: 2,
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_ENCODED: &[u8; 4] = b"\xe7\x07\x01\x02";

    #[cfg(test)]
    pub(crate) const EXAMPLE_STR: &str = "2023-01-02";

    /// Parses a [`LocalDate`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid TOML date value.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let position = bytes
            .iter()
            .position(|b| *b == b'-')
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (year, rest) = (&bytes[..position], &bytes[position + 1..]);

        let position = rest
            .iter()
            .position(|b| *b == b'-')
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (month, day) = (&rest[..position], &rest[position + 1..]);

        if year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return Err(ErrorKind::InvalidDatetime.into());
        }

        let year = str::from_utf8(year)
            .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
        let year = u16::from_str(year).map_err(|_| ErrorKind::InvalidDatetime)?;
        let month = str::from_utf8(month)
            .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
        let month = u8::from_str(month).map_err(|_| ErrorKind::InvalidDatetime)?;
        let day = str::from_utf8(day)
            .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
        let day = u8::from_str(day).map_err(|_| ErrorKind::InvalidDatetime)?;

        #[cfg(not(feature = "fast"))]
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

    #[inline]
    pub(crate) fn from_encoded(bytes: [u8; 4]) -> Self {
        Self {
            year: u16::from_ne_bytes(bytes[0..2].try_into().unwrap_or_else(|_| unreachable!())),
            month: u8::from_ne_bytes(bytes[2..3].try_into().unwrap_or_else(|_| unreachable!())),
            day: u8::from_ne_bytes(bytes[3..4].try_into().unwrap_or_else(|_| unreachable!())),
        }
    }

    #[inline]
    pub(crate) fn to_encoded(&self) -> [u8; 4] {
        let mut bytes = [0; 4];
        bytes[0..2].copy_from_slice(&self.year.to_ne_bytes());
        bytes[2..3].copy_from_slice(&self.month.to_ne_bytes());
        bytes[3..4].copy_from_slice(&self.day.to_ne_bytes());
        bytes
    }
}

impl str::FromStr for LocalDate {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for LocalDate {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { year, month, day } = *self;
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

impl From<LocalDate> for AnyDatetime {
    #[inline]
    fn from(value: LocalDate) -> Self {
        Self::LocalDate(value)
    }
}

impl TryFrom<AnyDatetime> for LocalDate {
    type Error = Error;

    #[inline]
    fn try_from(value: AnyDatetime) -> Result<Self, Self::Error> {
        if let AnyDatetime::LocalDate(date) = value {
            Ok(date)
        } else {
            Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date",
            ))
        }
    }
}

impl From<LocalDate> for Datetime {
    #[inline]
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

    #[inline]
    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: Some(date),
            time: None,
            offset: None,
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local date",
            ));
        };
        Ok(date)
    }
}

/// A TOML local time value.
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LocalTime {
    /// The hour in 24-hour format.
    ///
    /// This is always between 0--23 (inclusive).
    pub hour: u8,
    /// The minute portion of the time.
    ///
    /// This is always between 0--59 (inclusive).
    pub minute: u8,
    /// The second portion of the time.
    ///
    /// This is always between 0--60 (inclusive), 60 is valid for leap seconds.
    pub second: u8,
    /// The nanosecond portion of the time.
    ///
    /// This is always between 0--999 999 999 (inclusive).
    pub nanosecond: u32,
}

/// Alias for `LocalTime` for compatibility with [`toml`]
///
/// [`toml`]: https://crates.io/crates/toml
pub type Time = LocalTime;

impl LocalTime {
    pub(crate) const WRAPPER_TYPE: &str = "<soml::_impl::LocalTime::Wrapper>";
    pub(crate) const WRAPPER_FIELD: &str = "<soml::_impl::LocalTime::Wrapper::Field>";

    #[cfg(test)]
    pub(crate) const EXAMPLE: Self = Self {
        hour: 3,
        minute: 4,
        second: 5,
        nanosecond: 6_000_000,
    };

    #[cfg(test)]
    pub(crate) const EXAMPLE_ENCODED: &[u8; 8] = b"\x80\x8d\x5b\x00\x03\x04\x05\x00";

    #[cfg(test)]
    pub(crate) const EXAMPLE_STR: &str = "03:04:05.006";

    /// Parses a [`LocalTime`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid TOML time value.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        let position = bytes
            .iter()
            .position(|b| *b == b':')
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (hour, rest) = (&bytes[..position], &bytes[position + 1..]);

        let position = rest
            .iter()
            .position(|b| *b == b':')
            .ok_or(ErrorKind::InvalidDatetime)?;
        let (minute, second) = (&rest[..position], &rest[position + 1..]);

        if hour.len() != 2 || minute.len() != 2 {
            return Err(ErrorKind::InvalidDatetime.into());
        }

        let hour = str::from_utf8(hour)
            .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
        let hour = u8::from_str(hour).map_err(|_| ErrorKind::InvalidDatetime)?;
        let minute = str::from_utf8(minute)
            .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
        let minute = u8::from_str(minute).map_err(|_| ErrorKind::InvalidDatetime)?;

        let (second, fraction) = second
            .iter()
            .position(|b| *b == b'.')
            .map_or((second, None), |position| {
                (&second[..position], Some(&second[position + 1..]))
            });

        if second.len() != 2 {
            return Err(ErrorKind::InvalidDatetime.into());
        }
        let second = str::from_utf8(second)
            .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
        let second = u8::from_str(second).map_err(|_| ErrorKind::InvalidDatetime)?;

        let nanosecond = if let Some(fraction) = fraction {
            if fraction.is_empty() {
                return Err(ErrorKind::InvalidDatetime.into());
            }

            // The TOML spec requires at least milliseconds (6 digits) and truncate additional
            // digits. We support up to nanoseconds (9 digits) here and truncate the rest.
            let fraction = fraction.get(..9).unwrap_or(fraction);

            let fraction = str::from_utf8(fraction)
                .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
            let nanosecond = u32::from_str(fraction).map_err(|_| ErrorKind::InvalidDatetime)?;

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

    #[inline]
    pub(crate) fn from_encoded(bytes: [u8; 8]) -> Self {
        // Note: we put nanosecond first so the field order matches what rustc does. This field
        // order is not guaranteed, but if it does match we get more efficient code here.
        Self {
            nanosecond: u32::from_ne_bytes(
                bytes[0..4].try_into().unwrap_or_else(|_| unreachable!()),
            ),
            hour: u8::from_ne_bytes(bytes[4..5].try_into().unwrap_or_else(|_| unreachable!())),
            minute: u8::from_ne_bytes(bytes[5..6].try_into().unwrap_or_else(|_| unreachable!())),
            second: u8::from_ne_bytes(bytes[6..7].try_into().unwrap_or_else(|_| unreachable!())),
        }
    }

    #[inline]
    pub(crate) fn to_encoded(&self) -> [u8; 8] {
        // Note: we put nanosecond first so the field order matches what rustc does. This field
        // order is not guaranteed, but if it does match we get more efficient code here.
        let mut bytes = [0; 8];
        bytes[0..4].copy_from_slice(&self.nanosecond.to_ne_bytes());
        bytes[4..5].copy_from_slice(&self.hour.to_ne_bytes());
        bytes[5..6].copy_from_slice(&self.minute.to_ne_bytes());
        bytes[6..7].copy_from_slice(&self.second.to_ne_bytes());
        bytes
    }
}

impl str::FromStr for LocalTime {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

impl fmt::Display for LocalTime {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            hour,
            minute,
            second,
            nanosecond,
        } = *self;
        if nanosecond != 0 {
            write!(
                f,
                "{hour:02}:{minute:02}:{second:02}.{}",
                format!("{nanosecond:09}").trim_end_matches('0')
            )
        } else {
            write!(f, "{hour:02}:{minute:02}:{second:02}")
        }
    }
}

impl From<LocalTime> for AnyDatetime {
    #[inline]
    fn from(value: LocalTime) -> Self {
        Self::LocalTime(value)
    }
}

impl TryFrom<AnyDatetime> for LocalTime {
    type Error = Error;

    #[inline]
    fn try_from(value: AnyDatetime) -> Result<Self, Self::Error> {
        if let AnyDatetime::LocalTime(time) = value {
            Ok(time)
        } else {
            Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local time",
            ))
        }
    }
}

impl From<LocalTime> for Datetime {
    #[inline]
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

    #[inline]
    fn try_from(value: Datetime) -> Result<Self, Self::Error> {
        let Datetime {
            date: None,
            time: Some(time),
            offset: None,
        } = value
        else {
            return Err(Error::invalid_value(
                Unexpected::Other(value.type_str()),
                &"a local time",
            ));
        };
        Ok(time)
    }
}

/// A TOML UTC offset value.
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Offset {
    /// UTC zulu offset.
    ///
    /// This is equivalent to `+00:00`.
    Z,
    /// Numeric UTC offset.
    Custom {
        /// The offset in minutes.
        ///
        /// This is always between -1440--+1440 (-24:00--+24:00) (inclusive).
        minutes: i16,
    },
}

impl Offset {
    #[cfg(test)]
    pub(crate) const EXAMPLE: Self = Self::Custom { minutes: 428 };

    /// Parses an [`Offset`] from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not a valid offset portion of a TOML date-time.
    #[inline]
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
        if bytes == b"Z" || bytes == b"z" {
            Ok(Self::Z)
        } else {
            let (sign, bytes) = bytes.split_first().ok_or(ErrorKind::InvalidDatetime)?;
            let sign = match *sign {
                b'+' => 1,
                b'-' => -1,
                _ => return Err(ErrorKind::InvalidDatetime.into()),
            };

            let position = bytes
                .iter()
                .position(|b| *b == b':')
                .ok_or(ErrorKind::InvalidDatetime)?;
            let (hours, minutes) = (&bytes[..position], &bytes[position + 1..]);

            if hours.len() != 2 || minutes.len() != 2 {
                return Err(ErrorKind::InvalidDatetime.into());
            }

            // TODO use int::from_ascii when it's stable
            let hours = str::from_utf8(hours)
                .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
            let hours = i16::from_str(hours).map_err(|_| ErrorKind::InvalidDatetime)?;
            let minutes = str::from_utf8(minutes)
                .unwrap_or_else(|_| unreachable!("we should only have ASCII digits at this point"));
            let minutes = i16::from_str(minutes).map_err(|_| ErrorKind::InvalidDatetime)?;

            #[cfg(not(feature = "fast"))]
            if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
                return Err(ErrorKind::InvalidDatetime.into());
            }

            let minutes = sign * (hours * 60 + minutes);
            Ok(Self::Custom { minutes })
        }
    }

    #[inline]
    pub(crate) fn from_encoded(bytes: [u8; 2]) -> Self {
        let minutes = i16::from_ne_bytes(bytes);
        if minutes == 0 {
            Self::Z
        } else {
            Self::Custom { minutes }
        }
    }

    #[inline]
    pub(crate) fn to_encoded(&self) -> [u8; 2] {
        match *self {
            Self::Z => [0, 0],
            Self::Custom { minutes } => minutes.to_ne_bytes(),
        }
    }
}

impl fmt::Display for Offset {
    #[inline]
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

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_slice(s.as_bytes())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use std::str::FromStr as _;

    use assert_matches::assert_matches;

    use super::*;

    #[test]
    fn any_datetime_from_slice() {
        let result = AnyDatetime::from_slice(b"2023-01-02T03:04:05.006+07:08").unwrap();
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"2023-01-02t03:04:05.006+07:08").unwrap();
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"2023-01-02 03:04:05.006+07:08").unwrap();
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"2023-01-02T03:04:05.006").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"2023-01-02t03:04:05.006").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"2023-01-02 03:04:05.006").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"2023-01-02").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_slice(b"03:04:05.006").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);

        assert_matches!(
            AnyDatetime::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn any_datetime_type_str() {
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(datetime.type_str(), "offset date-time");

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(datetime.type_str(), "local date-time");

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(datetime.type_str(), "local date");

        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(datetime.type_str(), "local time");
    }

    #[test]
    fn any_datetime_from_str() {
        let result = AnyDatetime::from_str("2023-01-02T03:04:05.006+07:08").unwrap();
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_str("2023-01-02T03:04:05.006").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_str("2023-01-02").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);

        let result = AnyDatetime::from_str("03:04:05.006").unwrap();
        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);

        assert_matches!(
            AnyDatetime::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn any_datetime_display() {
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(datetime.to_string(), OffsetDatetime::EXAMPLE_STR);

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(datetime.to_string(), LocalDatetime::EXAMPLE_STR);

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(datetime.to_string(), LocalDate::EXAMPLE_STR);

        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(datetime.to_string(), LocalTime::EXAMPLE_STR);
    }

    #[test]
    fn datetime_from_slice() {
        let result = Datetime::from_slice(b"2023-01-02T03:04:05.006+07:08").unwrap();
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"2023-01-02t03:04:05.006+07:08").unwrap();
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"2023-01-02 03:04:05.006+07:08").unwrap();
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"2023-01-02T03:04:05.006").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"2023-01-02t03:04:05.006").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"2023-01-02 03:04:05.006").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"2023-01-02").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);

        let result = Datetime::from_slice(b"03:04:05.006").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);

        assert_matches!(
            Datetime::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn datetime_type_str() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(datetime.type_str(), "offset date-time");

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(datetime.type_str(), "local date-time");

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(datetime.type_str(), "local date");

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(datetime.type_str(), "local time");

        // Invalid permutations
        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_eq!(
            datetime.type_str(),
            "invalid date-time (offset with neither date nor time)"
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_eq!(
            datetime.type_str(),
            "invalid date-time (offset date without time)"
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_eq!(
            datetime.type_str(),
            "invalid date-time (offset time without date)"
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_eq!(
            datetime.type_str(),
            "invalid date-time (no date, time, nor offset)"
        );
    }

    #[test]
    fn datetime_from_str() {
        let result = Datetime::from_str("2023-01-02T03:04:05.006+07:08").unwrap();
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_str("2023-01-02T03:04:05.006").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from_str("2023-01-02").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);

        let result = Datetime::from_str("03:04:05.006").unwrap();
        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);

        assert_matches!(
            Datetime::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn datetime_display() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(datetime.to_string(), OffsetDatetime::EXAMPLE_STR);

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(datetime.to_string(), LocalDatetime::EXAMPLE_STR);

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(datetime.to_string(), LocalDate::EXAMPLE_STR);

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(datetime.to_string(), LocalTime::EXAMPLE_STR);

        // Invalid permutations
        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_eq!(
            datetime.to_string(),
            "<invalid date-time (offset with neither date nor time)>"
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_eq!(
            datetime.to_string(),
            "<invalid date-time (offset date without time)>"
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_eq!(
            datetime.to_string(),
            "<invalid date-time (offset time without date)>"
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_eq!(
            datetime.to_string(),
            "<invalid date-time (no date, time, nor offset)>"
        );
    }

    #[test]
    fn datetime_from_any_datetime() {
        let result = Datetime::from(AnyDatetime::EXAMPLE_OFFSET_DATETIME);
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from(AnyDatetime::EXAMPLE_LOCAL_DATETIME);
        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);

        let result = Datetime::from(AnyDatetime::EXAMPLE_LOCAL_DATE);
        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);

        let result = Datetime::from(AnyDatetime::EXAMPLE_LOCAL_TIME);
        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn any_datetime_try_from_datetime() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Ok(AnyDatetime::EXAMPLE_OFFSET_DATETIME)
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Ok(AnyDatetime::EXAMPLE_LOCAL_DATETIME)
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Ok(AnyDatetime::EXAMPLE_LOCAL_DATE)
        );

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Ok(AnyDatetime::EXAMPLE_LOCAL_TIME)
        );

        // Invalid permutations
        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_matches!(
            AnyDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn offset_datetime_from_slice() {
        let result = OffsetDatetime::from_slice(b"2023-01-02T03:04:05.006+07:08").unwrap();
        assert_eq!(result, OffsetDatetime::EXAMPLE);

        let result = OffsetDatetime::from_slice(b"2023-01-02t03:04:05.006+07:08").unwrap();
        assert_eq!(result, OffsetDatetime::EXAMPLE);

        let result = OffsetDatetime::from_slice(b"2023-01-02 03:04:05.006+07:08").unwrap();
        assert_eq!(result, OffsetDatetime::EXAMPLE);

        assert_matches!(
            OffsetDatetime::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn offset_datetime_from_str() {
        let result = OffsetDatetime::from_str("2023-01-02T03:04:05.006+07:08").unwrap();
        assert_eq!(result, OffsetDatetime::EXAMPLE);

        assert_matches!(
            OffsetDatetime::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn offset_datetime_display() {
        assert_eq!(
            OffsetDatetime::EXAMPLE.to_string(),
            OffsetDatetime::EXAMPLE_STR
        );
    }

    #[test]
    fn any_datetime_from_offset_datetime() {
        let result = AnyDatetime::from(OffsetDatetime::EXAMPLE);
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn offset_datetime_try_from_any_datetime() {
        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Ok(OffsetDatetime::EXAMPLE)
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn datetime_from_offset_datetime() {
        let result = Datetime::from(OffsetDatetime::EXAMPLE);
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn offset_datetime_try_from_datetime() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Ok(OffsetDatetime::EXAMPLE)
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_matches!(
            OffsetDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn local_datetime_from_slice() {
        let result = LocalDatetime::from_slice(b"2023-01-02T03:04:05.006").unwrap();
        assert_eq!(result, LocalDatetime::EXAMPLE);

        let result = LocalDatetime::from_slice(b"2023-01-02t03:04:05.006").unwrap();
        assert_eq!(result, LocalDatetime::EXAMPLE);

        let result = LocalDatetime::from_slice(b"2023-01-02 03:04:05.006").unwrap();
        assert_eq!(result, LocalDatetime::EXAMPLE);

        assert_matches!(
            LocalDatetime::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn local_datetime_from_str() {
        let result = LocalDatetime::from_str("2023-01-02T03:04:05.006").unwrap();
        assert_eq!(result, LocalDatetime::EXAMPLE);

        assert_matches!(
            LocalDatetime::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn local_datetime_display() {
        assert_eq!(
            LocalDatetime::EXAMPLE.to_string(),
            LocalDatetime::EXAMPLE_STR
        );
    }

    #[test]
    fn any_datetime_from_local_datetime() {
        let result = AnyDatetime::from(LocalDatetime::EXAMPLE);
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn local_datetime_try_from_any_datetime() {
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Ok(LocalDatetime::EXAMPLE)
        );

        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn datetime_from_local_datetime() {
        let result = Datetime::from(LocalDatetime::EXAMPLE);
        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn local_datetime_try_from_datetime() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Ok(LocalDatetime::EXAMPLE)
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_matches!(
            LocalDatetime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn local_date_from_slice() {
        let result = LocalDate::from_slice(b"2023-01-02").unwrap();
        assert_eq!(result, LocalDate::EXAMPLE);

        // Incorrect lengths
        assert_matches!(
            LocalDate::from_slice(b"123-01-02"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalDate::from_slice(b"2023-123-02"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalDate::from_slice(b"2023-01-123"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Invalid numbers
        assert_matches!(
            LocalDate::from_slice(b"abcd-01-02"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalDate::from_slice(b"2023-ef-02"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalDate::from_slice(b"2023-01-gh"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Month in range
        assert_matches!(
            LocalDate::from_slice(b"2023-00-02"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalDate::from_slice(b"2023-13-02"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Day in range
        assert_matches!(LocalDate::from_slice(b"2023-01-31"), Ok(LocalDate { .. }));
        assert_matches!(
            LocalDate::from_slice(b"2023-01-32"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(LocalDate::from_slice(b"2023-04-30"), Ok(LocalDate { .. }));
        assert_matches!(
            LocalDate::from_slice(b"2023-04-31"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(LocalDate::from_slice(b"2023-02-28"), Ok(LocalDate { .. }));
        assert_matches!(
            LocalDate::from_slice(b"2023-02-29"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(LocalDate::from_slice(b"2024-02-29"), Ok(LocalDate { .. }));
        assert_matches!(
            LocalDate::from_slice(b"2024-02-30"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        assert_matches!(
            LocalDate::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn local_date_from_str() {
        let result = LocalDate::from_str("2023-01-02").unwrap();
        assert_eq!(result, LocalDate::EXAMPLE);

        assert_matches!(
            LocalDate::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn local_date_display() {
        assert_eq!(LocalDate::EXAMPLE.to_string(), LocalDate::EXAMPLE_STR);
    }

    #[test]
    fn any_datetime_from_local_date() {
        let result = AnyDatetime::from(LocalDate::EXAMPLE);
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);
    }

    #[test]
    fn local_date_try_from_any_datetime() {
        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(LocalDate::try_from(datetime), Ok(LocalDate::EXAMPLE));

        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn datetime_from_local_date() {
        let result = Datetime::from(LocalDate::EXAMPLE);
        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_eq!(result, datetime);
    }

    #[test]
    fn local_date_try_from_datetime() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(LocalDate::try_from(datetime), Ok(LocalDate::EXAMPLE));

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_matches!(
            LocalDate::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn local_time_from_slice() {
        let time_no_nanos = LocalTime {
            nanosecond: 0,
            ..LocalTime::EXAMPLE
        };

        let result = LocalTime::from_slice(b"03:04:05.006").unwrap();
        assert_eq!(result, LocalTime::EXAMPLE);

        let result = LocalTime::from_slice(b"03:04:05").unwrap();
        assert_eq!(result, time_no_nanos);

        let result = LocalTime::from_slice(b"03:04:05.006000000").unwrap();
        assert_eq!(result, LocalTime::EXAMPLE);

        let result = LocalTime::from_slice(b"03:04:05.006000000999999").unwrap();
        assert_eq!(result, LocalTime::EXAMPLE);

        // Incorrect lengths
        assert_matches!(
            LocalTime::from_slice(b"123:04:05"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalTime::from_slice(b"03:123:05"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalTime::from_slice(b"03:04:123"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalTime::from_slice(b"03:04:05."),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Invalid numbers
        assert_matches!(
            LocalTime::from_slice(b"ab:04:05"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalTime::from_slice(b"03:cd:05"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalTime::from_slice(b"03:04:ef"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            LocalTime::from_slice(b"03:04:05.gh"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Time in range
        assert_matches!(LocalTime::from_slice(b"23:04:05"), Ok(LocalTime { .. }));
        assert_matches!(
            LocalTime::from_slice(b"24:04:05"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(LocalTime::from_slice(b"03:59:05"), Ok(LocalTime { .. }));
        assert_matches!(
            LocalTime::from_slice(b"03:60:05"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(LocalTime::from_slice(b"03:04:60"), Ok(LocalTime { .. })); // Allows for leap second
        assert_matches!(
            LocalTime::from_slice(b"03:04:61"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        assert_matches!(
            LocalTime::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn local_time_from_str() {
        let result = LocalTime::from_str("03:04:05.006").unwrap();
        assert_eq!(result, LocalTime::EXAMPLE);

        assert_matches!(
            Time::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn local_time_display() {
        assert_eq!(LocalTime::EXAMPLE.to_string(), LocalTime::EXAMPLE_STR);

        let time_no_nanos = LocalTime {
            nanosecond: 0,
            ..LocalTime::EXAMPLE
        };
        let str_no_nanos = LocalTime::EXAMPLE_STR.split('.').next().unwrap();
        assert_eq!(time_no_nanos.to_string(), str_no_nanos);
    }

    #[test]
    fn any_datetime_from_local_time() {
        let result = AnyDatetime::from(LocalTime::EXAMPLE);
        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn local_datetime_try_from_any_time() {
        let datetime = AnyDatetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(LocalTime::try_from(datetime), Ok(LocalTime::EXAMPLE));

        let datetime = AnyDatetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = AnyDatetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn datetime_from_local_time() {
        let result = Datetime::from(LocalTime::EXAMPLE);
        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_eq!(result, datetime);
    }

    #[test]
    fn local_time_try_from_datetime() {
        let datetime = Datetime::EXAMPLE_OFFSET_DATETIME;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATETIME;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_DATE;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_LOCAL_TIME;
        assert_matches!(LocalTime::try_from(datetime), Ok(LocalTime::EXAMPLE));

        let datetime = Datetime::EXAMPLE_INVALID_1;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_2;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_3;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );

        let datetime = Datetime::EXAMPLE_INVALID_4;
        assert_matches!(
            LocalTime::try_from(datetime),
            Err(Error(ErrorKind::InvalidValue(..)))
        );
    }

    #[test]
    fn offset_from_slice() {
        let result = Offset::from_slice(b"Z").unwrap();
        assert_eq!(result, Offset::Z);

        let result = Offset::from_slice(b"z").unwrap();
        assert_eq!(result, Offset::Z);

        let result = Offset::from_slice(b"+07:08").unwrap();
        assert_eq!(result, Offset::Custom { minutes: 428 });

        let result = Offset::from_slice(b"-07:08").unwrap();
        assert_eq!(result, Offset::Custom { minutes: -428 });

        assert_matches!(
            Offset::from_slice(b"07:08"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Incorrect lengths
        assert_matches!(
            Offset::from_slice(b"+123:08"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            Offset::from_slice(b"+07:123"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Invalid numbers
        assert_matches!(
            Offset::from_slice(b"+ab:08"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(
            Offset::from_slice(b"+07:cd"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        // Offset in range
        assert_matches!(Offset::from_slice(b"+23:08"), Ok(Offset::Custom { .. }));
        assert_matches!(
            Offset::from_slice(b"+24:08"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(Offset::from_slice(b"-23:08"), Ok(Offset::Custom { .. }));
        assert_matches!(
            Offset::from_slice(b"-24:08"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(Offset::from_slice(b"+07:59"), Ok(Offset::Custom { .. }));
        assert_matches!(
            Offset::from_slice(b"+07:60"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
        assert_matches!(Offset::from_slice(b"-07:59"), Ok(Offset::Custom { .. }));
        assert_matches!(
            Offset::from_slice(b"-07:60"),
            Err(Error(ErrorKind::InvalidDatetime))
        );

        assert_matches!(
            Offset::from_slice(b"invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }

    #[test]
    fn offset_display() {
        assert_eq!(Offset::EXAMPLE.to_string(), "+07:08");

        let offset = Offset::Custom { minutes: -428 };
        assert_eq!(offset.to_string(), "-07:08");

        assert_eq!(Offset::Z.to_string(), "Z");
    }

    #[test]
    fn offset_from_str() {
        let result = Offset::from_str("+07:08").unwrap();
        assert_eq!(result, Offset::EXAMPLE);

        let result = Offset::from_str("-07:08").unwrap();
        assert_eq!(result, Offset::Custom { minutes: -428 });

        assert_matches!(
            Offset::from_str("invalid string"),
            Err(Error(ErrorKind::InvalidDatetime))
        );
    }
}
