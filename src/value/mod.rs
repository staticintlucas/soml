//! Generic TOML value (de-)serialization.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::result::Result as StdResult;
use std::str::FromStr;
use std::{fmt, ops};

use serde::Serialize as _;

pub use self::datetime::{
    Date, Datetime, LocalDate, LocalDatetime, LocalTime, Offset, OffsetDatetime, Time,
};
use self::ser::ToValueSerializer;

pub(crate) mod datetime;
mod de;
mod ser;

mod private {
    pub trait Sealed {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type {
    String,
    Integer,
    Float,
    Boolean,
    Datetime,
    Array,
    Table,
}

impl Type {
    #[inline]
    pub fn to_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Datetime => "datetime",
            Self::Array => "array",
            Self::Table => "table",
        }
    }
}

impl fmt::Display for Type {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

/// A generic TOML value type.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string.
    String(String),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Boolean(bool),
    /// A datetime.
    Datetime(Datetime),
    /// An array of values.
    Array(Vec<Self>),
    /// A table of key-value pairs.
    Table(HashMap<String, Self>),
}

impl Value {
    /// Try to construct a [`Value`] from type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be represented as a [`Value`].
    #[inline]
    pub fn try_from<T>(value: T) -> Result<Self, crate::ser::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(ToValueSerializer)
    }

    /// Return an element of a TOML array or table, depending on the type of the index.
    ///
    /// Returns `None` if the index is a [`usize`] and `self` is not an array, or if the index is
    /// a string and `self` is not a table.
    #[inline]
    pub fn get(&self, index: impl Index) -> Option<&Self> {
        index.get(self)
    }

    /// Return an element of a TOML array or table, depending on the type of the index.
    ///
    /// Returns `None` if the index is a [`usize`] and `self` is not an array, or if the index is
    /// a string and `self` is not a table.
    #[inline]
    pub fn get_mut(&mut self, index: impl Index) -> Option<&mut Self> {
        index.get_mut(self)
    }

    /// Returns `true` if `self` is a string.
    #[must_use]
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(*self, Self::String(_))
    }

    /// Alias for [`Self::is_string`]. Included for compatibility with the [toml] crate.
    ///
    /// [toml]: https://github.com/toml-rs/toml
    #[must_use]
    #[inline]
    pub fn is_str(&self) -> bool {
        self.is_string()
    }

    /// Returns `true` if `self` is an integer.
    #[must_use]
    #[inline]
    pub fn is_integer(&self) -> bool {
        matches!(*self, Self::Integer(_))
    }

    /// Returns `true` if `self` is a float.
    #[must_use]
    #[inline]
    pub fn is_float(&self) -> bool {
        matches!(*self, Self::Float(_))
    }

    /// Returns `true` if `self` is a boolean.
    #[must_use]
    #[inline]
    pub fn is_boolean(&self) -> bool {
        matches!(*self, Self::Boolean(_))
    }

    /// Alias for [`Self::is_boolean`]. Included for compatibility with the [toml] crate.
    ///
    /// [toml]: https://github.com/toml-rs/toml
    #[must_use]
    #[inline]
    pub fn is_bool(&self) -> bool {
        self.is_boolean()
    }

    /// Returns `true` if `self` is a datetime.
    #[must_use]
    #[inline]
    pub fn is_datetime(&self) -> bool {
        matches!(*self, Self::Datetime(_))
    }

    /// Returns `true` if `self` is an array.
    #[must_use]
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(*self, Self::Array(_))
    }

    /// Returns `true` if `self` is a table.
    #[must_use]
    #[inline]
    pub fn is_table(&self) -> bool {
        matches!(*self, Self::Table(_))
    }

    /// If `self` is a string, returns it as a `&str`.
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Self::String(ref str) => Some(str),
            _ => None,
        }
    }

    /// If `self` is an integer, returns it as an `i64`.
    #[must_use]
    #[inline]
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Self::Integer(int) => Some(int),
            _ => None,
        }
    }

    /// If `self` is a float, returns it as an `f64`.
    #[must_use]
    #[inline]
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Self::Float(float) => Some(float),
            _ => None,
        }
    }

    /// If `self` is a boolean, returns it as a `bool`.
    #[must_use]
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Boolean(bool) => Some(bool),
            _ => None,
        }
    }

    /// If `self` is a datetime, returns it as a [`Datetime`].
    #[must_use]
    #[inline]
    pub fn as_datetime(&self) -> Option<&Datetime> {
        match *self {
            Self::Datetime(ref datetime) => Some(datetime),
            _ => None,
        }
    }

    /// If `self` is an array, returns it as a [`Vec<Value>`].
    #[must_use]
    #[inline]
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        match *self {
            Self::Array(ref array) => Some(array),
            _ => None,
        }
    }

    /// If `self` is an array, returns a mutable reference as a [`Vec<Value>`].
    #[must_use]
    #[inline]
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match *self {
            Self::Array(ref mut array) => Some(array),
            _ => None,
        }
    }

    /// If `self` is a table, returns it as a [`HashMap<String, Self>`].
    #[must_use]
    #[inline]
    pub fn as_table(&self) -> Option<&HashMap<String, Self>> {
        match *self {
            Self::Table(ref table) => Some(table),
            _ => None,
        }
    }

    /// If `self` is a table, returns a mutable reference as a [`HashMap<String, Self>`].
    #[must_use]
    #[inline]
    pub fn as_table_mut(&mut self) -> Option<&mut HashMap<String, Self>> {
        match *self {
            Self::Table(ref mut table) => Some(table),
            _ => None,
        }
    }

    /// Returns `true` if two values have the same type.
    #[must_use]
    #[inline]
    pub fn same_type(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    #[must_use]
    #[inline]
    const fn typ(&self) -> Type {
        match *self {
            Self::String(_) => Type::String,
            Self::Integer(_) => Type::Integer,
            Self::Float(_) => Type::Float,
            Self::Boolean(_) => Type::Boolean,
            Self::Datetime(_) => Type::Datetime,
            Self::Array(_) => Type::Array,
            Self::Table(_) => Type::Table,
        }
    }

    /// Returns the type of `self` as a `&str`.
    #[must_use]
    #[inline]
    pub fn type_str(&self) -> &'static str {
        self.typ().to_str()
    }
}

impl fmt::Display for Value {
    #[allow(clippy::panic)]
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.serialize(crate::ser::ValueSerializer) {
            Ok(s) => s.fmt(f),
            Err(e) => panic!("{e}"), // This should never happen
        }
    }
}

/// A trait for indexing into TOML values.
pub trait Index: private::Sealed {
    #[doc(hidden)]
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value>;

    #[doc(hidden)]
    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value>;

    #[doc(hidden)]
    fn index<'a>(&self, value: &'a Value) -> &'a Value;

    #[doc(hidden)]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value;
}

impl private::Sealed for usize {}

impl Index for usize {
    #[inline]
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        match *value {
            Value::Array(ref array) => array.get(*self),
            _ => None,
        }
    }

    #[inline]
    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        match *value {
            Value::Array(ref mut array) => array.get_mut(*self),
            _ => None,
        }
    }

    #[allow(clippy::panic)]
    #[inline]
    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        match *value {
            Value::Array(ref array) => array
                .get(*self)
                .unwrap_or_else(|| panic!("index `{self}` is out of bounds of TOML array")),
            _ => panic!("cannot index TOML {} with `usize`", value.type_str()),
        }
    }

    #[allow(clippy::panic)]
    #[inline]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        match *value {
            Value::Array(ref mut array) => array
                .get_mut(*self)
                .unwrap_or_else(|| panic!("index `{self}` is out of bounds of TOML array")),
            _ => panic!("cannot index TOML {} with `usize`", value.type_str()),
        }
    }
}

impl private::Sealed for str {}

impl Index for str {
    #[inline]
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        match *value {
            Value::Table(ref table) => table.get(self),
            _ => None,
        }
    }

    #[inline]
    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        match *value {
            Value::Table(ref mut table) => table.get_mut(self),
            _ => None,
        }
    }

    #[allow(clippy::panic)]
    #[inline]
    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        match *value {
            Value::Table(ref table) => table
                .get(self)
                .unwrap_or_else(|| panic!("key {self:?} is not present in TOML table")),
            _ => panic!("cannot index TOML {} with `str`", value.type_str()),
        }
    }

    #[allow(clippy::panic)]
    #[inline]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        match *value {
            Value::Table(ref mut table) => table
                .get_mut(self)
                .unwrap_or_else(|| panic!("key {self:?} is not present in TOML table")),
            _ => panic!("cannot index TOML {} with `str`", value.type_str()),
        }
    }
}

impl private::Sealed for String {}

impl Index for String {
    #[inline]
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        <str as Index>::get(self, value)
    }

    #[inline]
    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        <str as Index>::get_mut(self, value)
    }

    #[inline]
    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        <str as Index>::index(self, value)
    }

    #[inline]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        <str as Index>::index_mut(self, value)
    }
}

impl<T> private::Sealed for &T where T: Index + ?Sized {}

impl<T> Index for &T
where
    T: Index + ?Sized,
{
    #[inline]
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        T::get(self, value)
    }

    #[inline]
    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        T::get_mut(self, value)
    }

    #[inline]
    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        T::index(self, value)
    }

    #[inline]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        T::index_mut(self, value)
    }
}

impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Self;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        index.index(self)
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.index_mut(self)
    }
}

impl From<String> for Value {
    #[inline]
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    #[inline]
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<Cow<'_, str>> for Value {
    #[inline]
    fn from(value: Cow<'_, str>) -> Self {
        Self::String(value.into_owned())
    }
}

impl From<i8> for Value {
    #[inline]
    fn from(value: i8) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i16> for Value {
    #[inline]
    fn from(value: i16) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i32> for Value {
    #[inline]
    fn from(value: i32) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i64> for Value {
    #[inline]
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl TryFrom<i128> for Value {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        value.try_into().map(Self::Integer)
    }
}

impl From<u8> for Value {
    #[inline]
    fn from(value: u8) -> Self {
        Self::Integer(value.into())
    }
}

impl From<u16> for Value {
    #[inline]
    fn from(value: u16) -> Self {
        Self::Integer(value.into())
    }
}

impl From<u32> for Value {
    #[inline]
    fn from(value: u32) -> Self {
        Self::Integer(value.into())
    }
}

impl TryFrom<u64> for Value {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        value.try_into().map(Self::Integer)
    }
}

impl TryFrom<u128> for Value {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        value.try_into().map(Self::Integer)
    }
}

impl From<f32> for Value {
    #[inline]
    fn from(value: f32) -> Self {
        Self::Float(value.into())
    }
}

impl From<f64> for Value {
    #[inline]
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for Value {
    #[inline]
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<Datetime> for Value {
    #[inline]
    fn from(value: Datetime) -> Self {
        Self::Datetime(value)
    }
}

impl<V> From<Vec<V>> for Value
where
    V: Into<Self>,
{
    #[inline]
    fn from(value: Vec<V>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<V> From<&[V]> for Value
where
    V: Into<Self> + Clone,
{
    #[inline]
    fn from(value: &[V]) -> Self {
        Self::Array(value.iter().cloned().map(Into::into).collect())
    }
}

impl<V, const N: usize> From<[V; N]> for Value
where
    V: Into<Self>,
{
    #[inline]
    fn from(value: [V; N]) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<K, V> From<HashMap<K, V>> for Value
where
    K: Into<String>,
    V: Into<Self>,
{
    #[inline]
    fn from(value: HashMap<K, V>) -> Self {
        Self::Table(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl<K, V> From<BTreeMap<K, V>> for Value
where
    K: Into<String>,
    V: Into<Self>,
{
    #[inline]
    fn from(value: BTreeMap<K, V>) -> Self {
        Self::Table(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl FromStr for Value {
    type Err = crate::de::Error;

    #[inline]
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        crate::from_str(s)
    }
}

impl<V> FromIterator<V> for Value
where
    V: Into<Self>,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        Self::Array(iter.into_iter().map(Into::into).collect())
    }
}

impl<K, V> FromIterator<(K, V)> for Value
where
    K: Into<String>,
    V: Into<Self>,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::Table(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl PartialEq<&str> for Value {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        match *self {
            Self::String(ref str) => str == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for &str {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::String(ref str) => self == str,
            _ => false,
        }
    }
}

impl PartialEq<String> for Value {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        match *self {
            Self::String(ref str) => str == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for String {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::String(ref str) => self == str,
            _ => false,
        }
    }
}

impl PartialEq<Cow<'_, str>> for Value {
    #[inline]
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        match *self {
            Self::String(ref str) => str == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for Cow<'_, str> {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::String(ref str) => self == str,
            _ => false,
        }
    }
}

impl PartialEq<i64> for Value {
    #[inline]
    fn eq(&self, other: &i64) -> bool {
        match *self {
            Self::Integer(int) => int == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for i64 {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Integer(int) => *self == int,
            _ => false,
        }
    }
}

impl PartialEq<f64> for Value {
    #[inline]
    fn eq(&self, other: &f64) -> bool {
        match *self {
            Self::Float(float) => float == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for f64 {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Float(float) => *self == float,
            _ => false,
        }
    }
}

impl PartialEq<bool> for Value {
    #[inline]
    fn eq(&self, other: &bool) -> bool {
        match *self {
            Self::Boolean(bool) => bool == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for bool {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Boolean(bool) => *self == bool,
            _ => false,
        }
    }
}

impl PartialEq<Datetime> for Value {
    #[inline]
    fn eq(&self, other: &Datetime) -> bool {
        match *self {
            Self::Datetime(ref datetime) => datetime == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for Datetime {
    #[inline]
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Datetime(ref datetime) => self == datetime,
            _ => false,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use std::ops::{Index as _, IndexMut as _};

    use indoc::indoc;
    use isclose::assert_is_close;
    use maplit::{btreemap, hashmap};

    use super::*;

    #[test]
    fn type_to_str() {
        assert_eq!(Type::String.to_str(), "string");
        assert_eq!(Type::Integer.to_str(), "integer");
        assert_eq!(Type::Float.to_str(), "float");
        assert_eq!(Type::Boolean.to_str(), "boolean");
        assert_eq!(Type::Datetime.to_str(), "datetime");
        assert_eq!(Type::Array.to_str(), "array");
        assert_eq!(Type::Table.to_str(), "table");
    }

    #[test]
    fn type_display() {
        assert_eq!(Type::String.to_string(), "string");
        assert_eq!(Type::Integer.to_string(), "integer");
        assert_eq!(Type::Float.to_string(), "float");
        assert_eq!(Type::Boolean.to_string(), "boolean");
        assert_eq!(Type::Datetime.to_string(), "datetime");
        assert_eq!(Type::Array.to_string(), "array");
        assert_eq!(Type::Table.to_string(), "table");
    }

    #[test]
    fn value_try_from() {
        let value: Value = Value::try_from("hello").unwrap();
        assert_eq!(value, Value::String("hello".to_string()));

        let value: Value = Value::try_from(42).unwrap();
        assert_eq!(value, Value::Integer(42));

        let value: Value = Value::try_from(42.0).unwrap();
        assert_eq!(value, Value::Float(42.0));

        let value: Value = Value::try_from(true).unwrap();
        assert_eq!(value, Value::Boolean(true));

        let value: Value = Value::try_from(OffsetDatetime {
            date: LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            },
            time: LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            },
            offset: Offset::Custom { minutes: 428 },
        })
        .unwrap();
        assert_eq!(
            value,
            Value::Datetime(Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            })
        );

        let value: Value = Value::try_from(vec![1, 2, 3]).unwrap();
        assert_eq!(
            value,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );

        let value: Value = Value::try_from(hashmap! {
            "a" => 1,
            "b" => 2,
            "c" => 3,
        })
        .unwrap();
        assert_eq!(
            value,
            Value::Table(hashmap! {
                "a".to_string() => Value::Integer(1),
                "b".to_string() => Value::Integer(2),
                "c".to_string() => Value::Integer(3),
            })
        );
    }

    #[test]
    fn value_get() {
        let value = Value::Table(hashmap! {
            "a".to_string() => Value::Integer(1),
            "b".to_string() => Value::String("Hello!".to_string()),
        });

        assert_eq!(value.get("a").unwrap(), &Value::Integer(1));
        assert_eq!(
            value.get("b").unwrap(),
            &Value::String("Hello!".to_string())
        );
        assert!(value.get("c").is_none());
    }

    #[test]
    fn value_get_mut() {
        let mut value = Value::Table(hashmap! {
            "a".to_string() => Value::Integer(1),
            "b".to_string() => Value::String("Hello!".to_string()),
        });

        assert_eq!(value.get_mut("a").unwrap(), &Value::Integer(1));
        assert_eq!(
            value.get_mut("b").unwrap(),
            &mut Value::String("Hello!".to_string())
        );
        assert!(value.get_mut("c").is_none());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn value_is() {
        let value = Value::String("Hello!".to_string());
        assert!(value.is_string());
        assert!(value.is_str());
        assert!(!value.is_integer());
        assert!(!value.is_float());
        assert!(!value.is_boolean());
        assert!(!value.is_bool());
        assert!(!value.is_datetime());
        assert!(!value.is_array());
        assert!(!value.is_table());

        let value = Value::Integer(42);
        assert!(!value.is_string());
        assert!(!value.is_str());
        assert!(value.is_integer());
        assert!(!value.is_float());
        assert!(!value.is_boolean());
        assert!(!value.is_bool());
        assert!(!value.is_datetime());
        assert!(!value.is_array());
        assert!(!value.is_table());

        let value = Value::Float(42.0);
        assert!(!value.is_string());
        assert!(!value.is_str());
        assert!(!value.is_integer());
        assert!(value.is_float());
        assert!(!value.is_boolean());
        assert!(!value.is_bool());
        assert!(!value.is_datetime());
        assert!(!value.is_array());
        assert!(!value.is_table());

        let value = Value::Boolean(true);
        assert!(!value.is_string());
        assert!(!value.is_str());
        assert!(!value.is_integer());
        assert!(!value.is_float());
        assert!(value.is_boolean());
        assert!(value.is_bool());
        assert!(!value.is_datetime());
        assert!(!value.is_array());
        assert!(!value.is_table());

        let value = Value::Datetime(Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        });
        assert!(!value.is_string());
        assert!(!value.is_str());
        assert!(!value.is_integer());
        assert!(!value.is_float());
        assert!(!value.is_boolean());
        assert!(!value.is_bool());
        assert!(value.is_datetime());
        assert!(!value.is_array());
        assert!(!value.is_table());

        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert!(!value.is_string());
        assert!(!value.is_str());
        assert!(!value.is_integer());
        assert!(!value.is_float());
        assert!(!value.is_boolean());
        assert!(!value.is_bool());
        assert!(!value.is_datetime());
        assert!(value.is_array());
        assert!(!value.is_table());

        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert!(!value.is_string());
        assert!(!value.is_str());
        assert!(!value.is_integer());
        assert!(!value.is_float());
        assert!(!value.is_boolean());
        assert!(!value.is_bool());
        assert!(!value.is_datetime());
        assert!(!value.is_array());
        assert!(value.is_table());
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn value_as() {
        let mut value = Value::String("Hello!".to_string());
        assert_eq!(value.as_str().unwrap(), "Hello!");
        assert!(value.as_integer().is_none());
        assert!(value.as_float().is_none());
        assert!(value.as_bool().is_none());
        assert!(value.as_datetime().is_none());
        assert!(value.as_array().is_none());
        assert!(value.as_array_mut().is_none());
        assert!(value.as_table().is_none());
        assert!(value.as_table_mut().is_none());

        let mut value = Value::Integer(42);
        assert!(value.as_str().is_none());
        assert_eq!(value.as_integer().unwrap(), 42);
        assert!(value.as_float().is_none());
        assert!(value.as_bool().is_none());
        assert!(value.as_datetime().is_none());
        assert!(value.as_array().is_none());
        assert!(value.as_array_mut().is_none());
        assert!(value.as_table().is_none());
        assert!(value.as_table_mut().is_none());

        let mut value = Value::Float(42.0);
        assert!(value.as_str().is_none());
        assert!(value.as_integer().is_none());
        assert_is_close!(value.as_float().unwrap(), 42.0);
        assert!(value.as_bool().is_none());
        assert!(value.as_datetime().is_none());
        assert!(value.as_array().is_none());
        assert!(value.as_array_mut().is_none());
        assert!(value.as_table().is_none());
        assert!(value.as_table_mut().is_none());

        let mut value = Value::Boolean(true);
        assert!(value.as_str().is_none());
        assert!(value.as_integer().is_none());
        assert!(value.as_float().is_none());
        assert!(value.as_bool().unwrap());
        assert!(value.as_datetime().is_none());
        assert!(value.as_array().is_none());
        assert!(value.as_array_mut().is_none());
        assert!(value.as_table().is_none());
        assert!(value.as_table_mut().is_none());

        let datetime = Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        };
        let mut value = Value::Datetime(datetime.clone());
        assert!(value.as_str().is_none());
        assert!(value.as_integer().is_none());
        assert!(value.as_float().is_none());
        assert!(value.as_bool().is_none());
        assert_eq!(value.as_datetime().unwrap(), &datetime);
        assert!(value.as_array().is_none());
        assert!(value.as_array_mut().is_none());
        assert!(value.as_table().is_none());
        assert!(value.as_table_mut().is_none());

        let array = vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        let mut value = Value::Array(array.clone());
        assert!(value.as_str().is_none());
        assert!(value.as_integer().is_none());
        assert!(value.as_float().is_none());
        assert!(value.as_bool().is_none());
        assert!(value.as_datetime().is_none());
        assert_eq!(value.as_array().unwrap(), &array);
        assert_eq!(value.as_array_mut().unwrap(), &array);
        assert!(value.as_table().is_none());
        assert!(value.as_table_mut().is_none());

        let table = hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        };
        let mut value = Value::Table(table.clone());
        assert!(value.as_str().is_none());
        assert!(value.as_integer().is_none());
        assert!(value.as_float().is_none());
        assert!(value.as_bool().is_none());
        assert!(value.as_datetime().is_none());
        assert!(value.as_array().is_none());
        assert!(value.as_array_mut().is_none());
        assert_eq!(value.as_table().unwrap(), &table);
        assert_eq!(value.as_table_mut().unwrap(), &table);
    }

    #[test]
    fn value_same_type() {
        let values1 = [
            Value::String("Hello!".to_string()),
            Value::Integer(42),
            Value::Float(42.0),
            Value::Boolean(true),
            Value::Datetime(Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            }),
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ]),
            Value::Table(hashmap! {
                "one".to_string() => Value::Integer(1),
                "two".to_string() => Value::Integer(2),
                "three".to_string() => Value::Integer(3),
            }),
        ];

        let values2 = [
            Value::String("World!".to_string()),
            Value::Integer(123),
            Value::Float(123.4),
            Value::Boolean(false),
            Value::Datetime(Datetime {
                date: None,
                time: Some(LocalTime {
                    hour: 1,
                    minute: 2,
                    second: 3,
                    nanosecond: 4_000_000,
                }),
                offset: None,
            }),
            Value::Array(vec![
                Value::Integer(4),
                Value::Integer(5),
                Value::Integer(6),
            ]),
            Value::Table(hashmap! {
                "four".to_string() => Value::Integer(4),
                "five".to_string() => Value::Integer(5),
                "six".to_string() => Value::Integer(6),
            }),
        ];

        for (i, val1) in values1.iter().enumerate() {
            for (j, val2) in values2.iter().enumerate() {
                if i == j {
                    assert!(val1.same_type(val2));
                } else {
                    assert!(!val1.same_type(val2));
                }
            }
        }
    }

    #[test]
    fn value_typ() {
        let value = Value::String("Hello!".to_string());
        assert_eq!(value.typ(), Type::String);

        let value = Value::Integer(42);
        assert_eq!(value.typ(), Type::Integer);

        let value = Value::Float(42.0);
        assert_eq!(value.typ(), Type::Float);

        let value = Value::Boolean(true);
        assert_eq!(value.typ(), Type::Boolean);

        let value = Value::Datetime(Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        });
        assert_eq!(value.typ(), Type::Datetime);

        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(value.typ(), Type::Array);

        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(value.typ(), Type::Table);
    }

    #[test]
    fn value_typ_str() {
        let value = Value::String("Hello!".to_string());
        assert_eq!(value.type_str(), "string");

        let value = Value::Integer(42);
        assert_eq!(value.type_str(), "integer");

        let value = Value::Float(42.0);
        assert_eq!(value.type_str(), "float");

        let value = Value::Boolean(true);
        assert_eq!(value.type_str(), "boolean");

        let value = Value::Datetime(Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        });
        assert_eq!(value.type_str(), "datetime");

        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(value.type_str(), "array");

        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(value.type_str(), "table");
    }

    #[test]
    fn value_display() {
        let value = Value::String("Hello!".to_string());
        assert_eq!(value.to_string(), r#""Hello!""#);

        let value = Value::Integer(42);
        assert_eq!(value.to_string(), "42");

        let value = Value::Float(42.0);
        assert_eq!(value.to_string(), "42.0");

        let value = Value::Boolean(true);
        assert_eq!(value.to_string(), "true");

        let value = Value::Datetime(Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        });
        assert_eq!(value.to_string(), "2023-01-02T03:04:05.006+07:08");

        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(value.to_string(), "[1, 2, 3]");

        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(value.to_string(), "{ one = 1, three = 3, two = 2 }");
    }

    #[test]
    fn usize_index() {
        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(0.get(&value).unwrap(), &Value::Integer(1));
        assert_eq!(1.get(&value).unwrap(), &Value::Integer(2));
        assert_eq!(2.get(&value).unwrap(), &Value::Integer(3));
        assert!(3.get(&value).is_none());

        assert_eq!(0.get_mut(&mut value).unwrap(), &Value::Integer(1));
        assert_eq!(1.get_mut(&mut value).unwrap(), &Value::Integer(2));
        assert_eq!(2.get_mut(&mut value).unwrap(), &Value::Integer(3));
        assert!(3.get_mut(&mut value).is_none());

        assert_eq!(0.index(&value), &Value::Integer(1));
        assert_eq!(1.index(&value), &Value::Integer(2));
        assert_eq!(2.index(&value), &Value::Integer(3));

        assert_eq!(0.index_mut(&mut value), &Value::Integer(1));
        assert_eq!(1.index_mut(&mut value), &Value::Integer(2));
        assert_eq!(2.index_mut(&mut value), &Value::Integer(3));

        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert!(0.get(&value).is_none());
        assert!(1.get(&value).is_none());
        assert!(2.get(&value).is_none());

        assert!(0.get_mut(&mut value).is_none());
        assert!(1.get_mut(&mut value).is_none());
        assert!(2.get_mut(&mut value).is_none());
    }

    #[test]
    #[should_panic = "index `3` is out of bounds of TOML array"]
    fn usize_index_bounds_error() {
        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        3.index(&value);
    }

    #[test]
    #[should_panic = "index `3` is out of bounds of TOML array"]
    fn usize_index_mut_bounds_error() {
        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        3.index_mut(&mut value);
    }

    #[test]
    #[should_panic = "cannot index TOML table with `usize`"]
    fn usize_index_type_error() {
        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        1.index(&value);
    }

    #[test]
    #[should_panic = "cannot index TOML table with `usize`"]
    fn usize_index_mut_type_error() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        1.index_mut(&mut value);
    }

    #[test]
    fn str_index() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(Index::get("one", &value).unwrap(), &Value::Integer(1));
        assert_eq!(Index::get("two", &value).unwrap(), &Value::Integer(2));
        assert_eq!(Index::get("three", &value).unwrap(), &Value::Integer(3));
        assert!(Index::get("four", &value).is_none());

        assert_eq!(
            Index::get_mut("one", &mut value).unwrap(),
            &Value::Integer(1)
        );
        assert_eq!(
            Index::get_mut("two", &mut value).unwrap(),
            &Value::Integer(2)
        );
        assert_eq!(
            Index::get_mut("three", &mut value).unwrap(),
            &Value::Integer(3)
        );
        assert!(Index::get_mut("four", &mut value).is_none());

        assert_eq!(Index::index("one", &value), &Value::Integer(1));
        assert_eq!(Index::index("two", &value), &Value::Integer(2));
        assert_eq!(Index::index("three", &value), &Value::Integer(3));

        assert_eq!(Index::index_mut("one", &mut value), &Value::Integer(1));
        assert_eq!(Index::index_mut("two", &mut value), &Value::Integer(2));
        assert_eq!(Index::index_mut("three", &mut value), &Value::Integer(3));

        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert!(Index::get("one", &value).is_none());
        assert!(Index::get("two", &value).is_none());
        assert!(Index::get("three", &value).is_none());

        assert!(Index::get_mut("one", &mut value).is_none());
        assert!(Index::get_mut("two", &mut value).is_none());
        assert!(Index::get_mut("three", &mut value).is_none());
    }

    #[test]
    #[should_panic = r#"key "four" is not present in TOML table"#]
    fn str_index_missing_error() {
        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        Index::index("four", &value);
    }

    #[test]
    #[should_panic = r#"key "four" is not present in TOML table"#]
    fn str_index_mut_missing_error() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        Index::index_mut("four", &mut value);
    }

    #[test]
    #[should_panic = "cannot index TOML array with `str`"]
    fn str_index_type_error() {
        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        Index::index("two", &value);
    }

    #[test]
    #[should_panic = "cannot index TOML array with `str`"]
    fn str_index_mut_type_error() {
        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        Index::index_mut("two", &mut value);
    }

    #[test]
    fn string_index() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(
            Index::get(&"one".to_string(), &value).unwrap(),
            &Value::Integer(1)
        );
        assert_eq!(
            Index::get(&"two".to_string(), &value).unwrap(),
            &Value::Integer(2)
        );
        assert_eq!(
            Index::get(&"three".to_string(), &value).unwrap(),
            &Value::Integer(3)
        );
        assert!(Index::get(&"four".to_string(), &value).is_none());

        assert_eq!(
            Index::get_mut(&"one".to_string(), &mut value).unwrap(),
            &Value::Integer(1)
        );
        assert_eq!(
            Index::get_mut(&"two".to_string(), &mut value).unwrap(),
            &Value::Integer(2)
        );
        assert_eq!(
            Index::get_mut(&"three".to_string(), &mut value).unwrap(),
            &Value::Integer(3)
        );
        assert!(Index::get_mut(&"four".to_string(), &mut value).is_none());

        assert_eq!(Index::index(&"one".to_string(), &value), &Value::Integer(1));
        assert_eq!(Index::index(&"two".to_string(), &value), &Value::Integer(2));
        assert_eq!(
            Index::index(&"three".to_string(), &value),
            &Value::Integer(3)
        );

        assert_eq!(
            Index::index_mut(&"one".to_string(), &mut value),
            &Value::Integer(1)
        );
        assert_eq!(
            Index::index_mut(&"two".to_string(), &mut value),
            &Value::Integer(2)
        );
        assert_eq!(
            Index::index_mut(&"three".to_string(), &mut value),
            &Value::Integer(3)
        );

        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert!(Index::get(&"one".to_string(), &value).is_none());
        assert!(Index::get(&"two".to_string(), &value).is_none());
        assert!(Index::get(&"three".to_string(), &value).is_none());

        assert!(Index::get_mut(&"one".to_string(), &mut value).is_none());
        assert!(Index::get_mut(&"two".to_string(), &mut value).is_none());
        assert!(Index::get_mut(&"three".to_string(), &mut value).is_none());
    }

    #[test]
    #[should_panic = r#"key "four" is not present in TOML table"#]
    fn string_index_missing_error() {
        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        Index::index(&"four".to_string(), &value);
    }

    #[test]
    #[should_panic = r#"key "four" is not present in TOML table"#]
    fn string_index_mut_missing_error() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        Index::index_mut(&"four".to_string(), &mut value);
    }

    #[test]
    #[should_panic = "cannot index TOML array with `str`"]
    fn string_index_type_error() {
        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        Index::index(&"two".to_string(), &value);
    }

    #[test]
    #[should_panic = "cannot index TOML array with `str`"]
    fn string_index_mut_type_error() {
        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        Index::index_mut(&"two".to_string(), &mut value);
    }

    #[test]
    fn str_ref_index() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(Index::get(&"one", &value).unwrap(), &Value::Integer(1));
        assert_eq!(Index::get(&"two", &value).unwrap(), &Value::Integer(2));
        assert_eq!(Index::get(&"three", &value).unwrap(), &Value::Integer(3));
        assert!(Index::get(&"four", &value).is_none());

        assert_eq!(
            Index::get_mut(&"one", &mut value).unwrap(),
            &Value::Integer(1)
        );
        assert_eq!(
            Index::get_mut(&"two", &mut value).unwrap(),
            &Value::Integer(2)
        );
        assert_eq!(
            Index::get_mut(&"three", &mut value).unwrap(),
            &Value::Integer(3)
        );
        assert!(Index::get_mut(&"four", &mut value).is_none());

        assert_eq!(Index::index(&"one", &value), &Value::Integer(1));
        assert_eq!(Index::index(&"two", &value), &Value::Integer(2));
        assert_eq!(Index::index(&"three", &value), &Value::Integer(3));

        assert_eq!(Index::index_mut(&"one", &mut value), &Value::Integer(1));
        assert_eq!(Index::index_mut(&"two", &mut value), &Value::Integer(2));
        assert_eq!(Index::index_mut(&"three", &mut value), &Value::Integer(3));

        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert!(Index::get(&"one", &value).is_none());
        assert!(Index::get(&"two", &value).is_none());
        assert!(Index::get(&"three", &value).is_none());

        assert!(Index::get_mut(&"one", &mut value).is_none());
        assert!(Index::get_mut(&"two", &mut value).is_none());
        assert!(Index::get_mut(&"three", &mut value).is_none());
    }

    #[test]
    #[should_panic = r#"key "four" is not present in TOML table"#]
    fn str_ref_index_missing_error() {
        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        Index::index(&"four", &value);
    }

    #[test]
    #[should_panic = r#"key "four" is not present in TOML table"#]
    fn str_ref_index_mut_missing_error() {
        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        Index::index_mut(&"four", &mut value);
    }

    #[test]
    #[should_panic = "cannot index TOML array with `str`"]
    fn str_ref_index_type_error() {
        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        Index::index(&"two", &value);
    }

    #[test]
    #[should_panic = "cannot index TOML array with `str`"]
    fn str_ref_index_mut_type_error() {
        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        Index::index_mut(&"two", &mut value);
    }

    #[test]
    fn value_index() {
        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(value.index(&0), &Value::Integer(1));
        assert_eq!(value.index(&1), &Value::Integer(2));
        assert_eq!(value.index(&2), &Value::Integer(3));

        let value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(value.index(&"one".to_string()), &Value::Integer(1));
        assert_eq!(value.index(&"two".to_string()), &Value::Integer(2));
        assert_eq!(value.index(&"three".to_string()), &Value::Integer(3));
    }

    #[test]
    fn value_index_mut() {
        let mut value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        assert_eq!(value.index_mut(&0), &Value::Integer(1));
        assert_eq!(value.index_mut(&1), &Value::Integer(2));
        assert_eq!(value.index_mut(&2), &Value::Integer(3));

        let mut value = Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        assert_eq!(value.index_mut(&"one".to_string()), &Value::Integer(1));
        assert_eq!(value.index_mut(&"two".to_string()), &Value::Integer(2));
        assert_eq!(value.index_mut(&"three".to_string()), &Value::Integer(3));
    }

    #[test]
    fn value_from() {
        let datetime = || Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        };

        assert_eq!(
            Value::from("hello".to_string()),
            Value::String("hello".to_string())
        );
        assert_eq!(Value::from("hello"), Value::String("hello".to_string()));
        assert_eq!(
            Value::from(Cow::from("hello")),
            Value::String("hello".to_string())
        );
        assert_eq!(Value::from(42_i8), Value::Integer(42));
        assert_eq!(Value::from(42_i16), Value::Integer(42));
        assert_eq!(Value::from(42_i32), Value::Integer(42));
        assert_eq!(Value::from(42_i64), Value::Integer(42));
        assert_eq!(Value::from(42_u8), Value::Integer(42));
        assert_eq!(Value::from(42_u16), Value::Integer(42));
        assert_eq!(Value::from(42_u32), Value::Integer(42));
        assert_eq!(Value::from(42.0_f32), Value::Float(42.0));
        assert_eq!(Value::from(42.0_f64), Value::Float(42.0));
        assert_eq!(Value::from(true), Value::Boolean(true));
        assert_eq!(Value::from(datetime()), Value::Datetime(datetime()));
        assert_eq!(
            Value::from(vec![1, 2, 3]),
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
        assert_eq!(
            Value::from(&[1, 2, 3][..]),
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
        assert_eq!(
            Value::from([1, 2, 3]),
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
        assert_eq!(
            Value::from(hashmap! {
                "a" => 1,
                "b" => 2,
                "c" => 3,
            }),
            Value::Table(hashmap! {
                "a".to_string() => Value::Integer(1),
                "b".to_string() => Value::Integer(2),
                "c".to_string() => Value::Integer(3),
            })
        );
        assert_eq!(
            Value::from(btreemap! {
                "a" => 1,
                "b" => 2,
                "c" => 3,
            }),
            Value::Table(hashmap! {
                "a".to_string() => Value::Integer(1),
                "b".to_string() => Value::Integer(2),
                "c".to_string() => Value::Integer(3),
            })
        );
    }

    #[test]
    fn value_try_from_trait() {
        assert_eq!(
            <Value as TryFrom<i128>>::try_from(42_i128).unwrap(),
            Value::Integer(42)
        );
        assert!(<Value as TryFrom<i128>>::try_from(i128::MIN).is_err());

        assert_eq!(
            <Value as TryFrom<u64>>::try_from(42_u64).unwrap(),
            Value::Integer(42)
        );
        assert!(<Value as TryFrom<u64>>::try_from(u64::MAX).is_err());

        assert_eq!(
            <Value as TryFrom<u128>>::try_from(42_u128).unwrap(),
            Value::Integer(42)
        );
        assert!(<Value as TryFrom<u128>>::try_from(u128::MAX).is_err());
    }

    #[test]
    fn value_from_str() {
        let result = Value::from_str(indoc! {r#"
            # This is a TOML document.

            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates

            [database]
            server = "192.168.1.1"
            ports = [ 8000, 8001, 8002 ]
            connection_max = 5000
            enabled = true

            [servers]

              # Indentation (tabs and/or spaces) is allowed but not required
              [servers.alpha]
              ip = "10.0.0.1"
              dc = "eqdc10"

              [servers.beta]
              ip = "10.0.0.2"
              dc = "eqdc10"

            [clients]
            data = { "gamma" = 1, "delta" = 2 }

            # Line breaks are OK when inside arrays
            hosts = [
              "alpha",
              "omega"
            ]
        "#})
        .unwrap();

        assert_eq!(
            result,
            Value::Table(hashmap! {
                "title".to_string() => Value::String("TOML Example".to_string()),
                "owner".to_string() => Value::Table(hashmap! {
                    "name".to_string() => Value::String("Tom Preston-Werner".to_string()),
                    "dob".to_string() => Value::Datetime(Datetime {
                        date: Some(LocalDate {
                            year: 1979,
                            month: 5,
                            day: 27,
                        }),
                        time: Some(LocalTime {
                            hour: 7,
                            minute: 32,
                            second: 0,
                            nanosecond: 0,
                        }),
                        offset: Some(Offset::Custom { minutes: -480 }),
                    }),
                }),
                "database".to_string() => Value::Table(hashmap! {
                    "server".to_string() => Value::String("192.168.1.1".to_string()),
                    "ports".to_string() => Value::Array(vec![Value::Integer(8000), Value::Integer(8001), Value::Integer(8002)]),
                    "connection_max".to_string() => Value::Integer(5000),
                    "enabled".to_string() => Value::Boolean(true),
                }),
                "servers".to_string() => Value::Table(hashmap! {
                    "alpha".to_string() => Value::Table(hashmap! {
                        "ip".to_string() => Value::String("10.0.0.1".to_string()),
                        "dc".to_string() => Value::String("eqdc10".to_string()),
                    }),
                    "beta".to_string() => Value::Table(hashmap! {
                        "ip".to_string() => Value::String("10.0.0.2".to_string()),
                        "dc".to_string() => Value::String("eqdc10".to_string()),
                    }),
                }),
                "clients".to_string() => Value::Table(hashmap! {
                    "hosts".to_string() => Value::Array(vec![Value::String("alpha".to_string()), Value::String("omega".to_string())]),
                    "data".to_string() => Value::Table(hashmap! {
                        "gamma".to_string() => Value::Integer(1),
                        "delta".to_string() => Value::Integer(2),
                    }),
                }),
            })
        );
    }

    #[test]
    fn value_from_iterator() {
        let result = Value::from_iter(vec![1, 2, 3]);
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );

        let result = Value::from_iter([("one", 1), ("two", 2), ("three", 3)]);
        assert_eq!(
            result,
            Value::Table(hashmap! {
                "one".to_string() => Value::Integer(1),
                "two".to_string() => Value::Integer(2),
                "three".to_string() => Value::Integer(3),
            })
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // not really float cmp, but clippy doesn't know
    fn value_partial_eq_str() {
        let value = Value::String("Hello!".to_string());

        assert_eq!(value, "Hello!");
        assert_eq!("Hello!", value);
        assert_eq!(value, "Hello!".to_string());
        assert_eq!("Hello!".to_string(), value);
        assert_eq!(value, Cow::from("Hello!"));
        assert_eq!(Cow::from("Hello!"), value);

        assert_ne!(value, "Hello");
        assert_ne!("Hello", value);
        assert_ne!(value, "Hello".to_string());
        assert_ne!("Hello".to_string(), value);
        assert_ne!(value, Cow::from("Hello"));
        assert_ne!(Cow::from("Hello"), value);

        assert_ne!(value, 42);
        assert_ne!(42, value);
        assert_ne!(value, 42.0);
        assert_ne!(42.0, value);
        assert_ne!(value, true);
        assert_ne!(true, value);
        assert_ne!(
            value,
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            }
        );
        assert_ne!(
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            },
            value
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // not really float cmp, but clippy doesn't know
    fn value_partial_eq_i64() {
        let value = Value::Integer(42);

        assert_eq!(value, 42);
        assert_eq!(42, value);

        assert_ne!(value, 24);
        assert_ne!(24, value);

        assert_ne!(value, "42");
        assert_ne!("42", value);
        assert_ne!(value, "42".to_string());
        assert_ne!("42".to_string(), value);
        assert_ne!(value, Cow::from("42"));
        assert_ne!(Cow::from("42"), value);
        assert_ne!(value, 42.0);
        assert_ne!(42.0, value);
        assert_ne!(value, true);
        assert_ne!(true, value);
        assert_ne!(
            value,
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            }
        );
        assert_ne!(
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            },
            value
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // strict cmp is fine for literals
    fn value_partial_eq_f64() {
        let value = Value::Float(42.0);

        assert_eq!(value, 42.0);
        assert_eq!(42.0, value);

        assert_ne!(value, 42.01);
        assert_ne!(42.01, value);

        assert_ne!(value, "42");
        assert_ne!("42", value);
        assert_ne!(value, "42".to_string());
        assert_ne!("42".to_string(), value);
        assert_ne!(value, Cow::from("42"));
        assert_ne!(Cow::from("42"), value);
        assert_ne!(value, 42);
        assert_ne!(42, value);
        assert_ne!(value, true);
        assert_ne!(true, value);
        assert_ne!(
            value,
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            }
        );
        assert_ne!(
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            },
            value
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // not really float cmp, but clippy doesn't know
    fn value_partial_eq_bool() {
        let value = Value::Boolean(true);

        assert_eq!(value, true);
        assert_eq!(true, value);

        assert_ne!(value, false);
        assert_ne!(false, value);

        assert_ne!(value, "42");
        assert_ne!("42", value);
        assert_ne!(value, "42".to_string());
        assert_ne!("42".to_string(), value);
        assert_ne!(value, Cow::from("42"));
        assert_ne!(Cow::from("42"), value);
        assert_ne!(value, 42);
        assert_ne!(42, value);
        assert_ne!(value, 42.0);
        assert_ne!(42.0, value);
        assert_ne!(
            value,
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            }
        );
        assert_ne!(
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            },
            value
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // not really float cmp, but clippy doesn't know
    fn value_partial_eq_datetime() {
        let value = Value::Datetime(Datetime {
            date: Some(LocalDate {
                year: 2023,
                month: 1,
                day: 2,
            }),
            time: Some(LocalTime {
                hour: 3,
                minute: 4,
                second: 5,
                nanosecond: 6_000_000,
            }),
            offset: Some(Offset::Custom { minutes: 428 }),
        });

        assert_eq!(
            value,
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            }
        );
        assert_eq!(
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 428 }),
            },
            value
        );

        assert_ne!(
            value,
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 429 }),
            }
        );
        assert_ne!(
            Datetime {
                date: Some(LocalDate {
                    year: 2023,
                    month: 1,
                    day: 2,
                }),
                time: Some(LocalTime {
                    hour: 3,
                    minute: 4,
                    second: 5,
                    nanosecond: 6_000_000,
                }),
                offset: Some(Offset::Custom { minutes: 429 }),
            },
            value
        );

        assert_ne!(value, "42");
        assert_ne!("42", value);
        assert_ne!(value, "42".to_string());
        assert_ne!("42".to_string(), value);
        assert_ne!(value, Cow::from("42"));
        assert_ne!(Cow::from("42"), value);
        assert_ne!(value, 42);
        assert_ne!(42, value);
        assert_ne!(value, 42.0);
        assert_ne!(42.0, value);
        assert_ne!(value, true);
        assert_ne!(true, value);
    }
}
