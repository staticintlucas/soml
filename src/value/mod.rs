use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::result::Result as StdResult;
use std::str::FromStr;
use std::{fmt, ops};

use serde::Serialize;

pub use self::datetime::{
    Date, Datetime, LocalDate, LocalDatetime, LocalTime, Offset, OffsetDatetime, Time,
};

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
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(Datetime),
    Array(Vec<Self>),
    Table(HashMap<String, Self>),
}

impl Value {
    // TODO
    // pub fn try_from<T>(value: T) -> Result<Self, crate::ser::Error>
    // where
    //     T: ser::Serialize,
    // {
    //     value.serialize(Serializer)
    // }

    pub fn get(&self, index: impl Index) -> Option<&Self> {
        index.get(self)
    }

    pub fn get_mut(&mut self, index: impl Index) -> Option<&mut Self> {
        index.get_mut(self)
    }

    /// Returns `true` if `self` is a string.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(*self, Self::String(_))
    }

    /// Equivalent to [`Self::is_string`], but for compatibility with [`toml`]
    ///
    /// [`toml`]: https://github.com/toml-rs/toml
    #[must_use]
    pub fn is_str(&self) -> bool {
        self.is_string()
    }

    /// Returns `true` if `self` is an integer.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(*self, Self::Integer(_))
    }

    /// Returns `true` if `self` is a float.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_float(&self) -> bool {
        matches!(*self, Self::Float(_))
    }

    /// Returns `true` if `self` is a boolean.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(*self, Self::Boolean(_))
    }

    /// Equivalent to [`Self::is_boolean`], but for compatibility with [`toml`]
    ///
    /// [`toml`]: https://github.com/toml-rs/toml
    #[must_use]
    pub fn is_bool(&self) -> bool {
        self.is_boolean()
    }

    /// Returns `true` if `self` is a datetime.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_datetime(&self) -> bool {
        matches!(*self, Self::Datetime(_))
    }

    /// Returns `true` if `self` is an array.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(*self, Self::Array(_))
    }

    /// Returns `true` if `self` is a table.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn is_table(&self) -> bool {
        matches!(*self, Self::Table(_))
    }

    /// If `self` is a string, returns it as a `&str`.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Self::String(ref str) => Some(str),
            _ => None,
        }
    }

    /// If `self` is an integer, returns it as an `i64`.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Self::Integer(int) => Some(int),
            _ => None,
        }
    }

    /// If `self` is a float, returns it as an `f64`.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Self::Float(float) => Some(float),
            _ => None,
        }
    }

    /// If `self` is a boolean, returns it as a `bool`.
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Boolean(bool) => Some(bool),
            _ => None,
        }
    }

    /// If `self` is a datetime, returns it as a [`Datetime`].
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_datetime(&self) -> Option<&Datetime> {
        match *self {
            Self::Datetime(ref datetime) => Some(datetime),
            _ => None,
        }
    }

    /// If `self` is an array, returns it as a [`Vec<Value>`].
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        match *self {
            Self::Array(ref array) => Some(array),
            _ => None,
        }
    }

    /// If `self` is an array, returns a mutable reference as a [`Vec<Value>`].
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match *self {
            Self::Array(ref mut array) => Some(array),
            _ => None,
        }
    }

    /// If `self` is a table, returns it as a [`HashMap<String, Self>`].
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_table(&self) -> Option<&HashMap<String, Self>> {
        match *self {
            Self::Table(ref table) => Some(table),
            _ => None,
        }
    }

    /// If `self` is a table, returns a mutable reference as a [`HashMap<String, Self>`].
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn as_table_mut(&mut self) -> Option<&mut HashMap<String, Self>> {
        match *self {
            Self::Table(ref mut table) => Some(table),
            _ => None,
        }
    }

    /// Returns `true` if two values have the same type.
    #[must_use]
    pub fn same_type(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    #[must_use]
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
    pub fn type_str(&self) -> &'static str {
        self.typ().to_str()
    }
}

impl fmt::Display for Value {
    #[allow(clippy::panic)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.serialize(crate::ser::InlineSerializer) {
            Ok(s) => s.fmt(f),
            Err(e) => panic!("{e}"),
        }
    }
}

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
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        match *value {
            Value::Array(ref array) => array.get(*self),
            _ => None,
        }
    }

    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        match *value {
            Value::Array(ref mut array) => array.get_mut(*self),
            _ => None,
        }
    }

    #[allow(clippy::panic)]
    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        match *value {
            Value::Array(ref array) => array
                .get(*self)
                .unwrap_or_else(|| panic!("index {self} is out of bounds of TOML array")),
            _ => panic!("cannot index TOML {} with usize", value.type_str()),
        }
    }

    #[allow(clippy::panic)]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        match *value {
            Value::Array(ref mut array) => array
                .get_mut(*self)
                .unwrap_or_else(|| panic!("index {self} is out of bounds of TOML array")),
            _ => panic!("cannot index TOML {} with usize", value.type_str()),
        }
    }
}

impl private::Sealed for str {}

impl Index for str {
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        match *value {
            Value::Table(ref table) => table.get(self),
            _ => None,
        }
    }

    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        match *value {
            Value::Table(ref mut table) => table.get_mut(self),
            _ => None,
        }
    }

    #[allow(clippy::panic)]
    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        match *value {
            Value::Table(ref table) => table
                .get(self)
                .unwrap_or_else(|| panic!("key {self} not present in TOML table")),
            _ => panic!("cannot index TOML {} with string", value.type_str()),
        }
    }

    #[allow(clippy::panic)]
    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        match *value {
            Value::Table(ref mut table) => table
                .get_mut(self)
                .unwrap_or_else(|| panic!("key {self} not present in TOML table")),
            _ => panic!("cannot index TOML {} with string", value.type_str()),
        }
    }
}

impl private::Sealed for String {}

impl Index for String {
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        <str as Index>::get(self, value)
    }

    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        <str as Index>::get_mut(self, value)
    }

    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        <str as Index>::index(self, value)
    }

    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        <str as Index>::index_mut(self, value)
    }
}

impl<T> private::Sealed for &T where T: Index {}

impl<T> Index for &T
where
    T: Index,
{
    fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        T::get(self, value)
    }

    fn get_mut<'a>(&self, value: &'a mut Value) -> Option<&'a mut Value> {
        T::get_mut(self, value)
    }

    fn index<'a>(&self, value: &'a Value) -> &'a Value {
        T::index(self, value)
    }

    fn index_mut<'a>(&self, value: &'a mut Value) -> &'a mut Value {
        T::index_mut(self, value)
    }
}

impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Self;

    fn index(&self, index: I) -> &Self::Output {
        index.index(self)
    }
}

impl<I> ops::IndexMut<I> for Value
where
    I: Index,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.index_mut(self)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<Cow<'_, str>> for Value {
    fn from(value: Cow<'_, str>) -> Self {
        Self::String(value.into_owned())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Self::Integer(value.into())
    }
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Self::Integer(value.into())
    }
}

// TODO
// impl From<u64> for Value {
//     fn from(value: u64) -> Self {
//         Self::Integer(value)
//     }
// }

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Integer(value.into())
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Self::Integer(value.into())
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self::Integer(value.into())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(value.into())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<Datetime> for Value {
    fn from(value: Datetime) -> Self {
        Self::Datetime(value)
    }
}

impl<V> From<Vec<V>> for Value
where
    V: Into<Self>,
{
    fn from(value: Vec<V>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<V> From<&[V]> for Value
where
    V: Into<Self> + Clone,
{
    fn from(value: &[V]) -> Self {
        Self::Array(value.iter().cloned().map(Into::into).collect())
    }
}

impl<V, const N: usize> From<[V; N]> for Value
where
    V: Into<Self>,
{
    fn from(value: [V; N]) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<V> From<HashMap<String, V>> for Value
where
    V: Into<Self>,
{
    fn from(value: HashMap<String, V>) -> Self {
        Self::Table(value.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl<V> From<BTreeMap<String, V>> for Value
where
    V: Into<Self>,
{
    fn from(value: BTreeMap<String, V>) -> Self {
        Self::Table(value.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl FromStr for Value {
    type Err = crate::de::Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        crate::from_str(s)
    }
}

impl<V> FromIterator<V> for Value
where
    V: Into<Self>,
{
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        Self::Array(iter.into_iter().map(Into::into).collect())
    }
}

impl<K, V> FromIterator<(K, V)> for Value
where
    K: Into<String>,
    V: Into<Self>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::Table(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        match *self {
            Self::String(ref str) => str == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for &str {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::String(ref str) => self == str,
            _ => false,
        }
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        match *self {
            Self::String(ref str) => str == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for String {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::String(ref str) => self == str,
            _ => false,
        }
    }
}

impl PartialEq<Cow<'_, str>> for Value {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        match *self {
            Self::String(ref str) => str == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for Cow<'_, str> {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::String(ref str) => self == str,
            _ => false,
        }
    }
}

impl PartialEq<i64> for Value {
    fn eq(&self, other: &i64) -> bool {
        match *self {
            Self::Integer(int) => int == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for i64 {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Integer(int) => *self == int,
            _ => false,
        }
    }
}

impl PartialEq<f64> for Value {
    fn eq(&self, other: &f64) -> bool {
        match *self {
            Self::Float(float) => float == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for f64 {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Float(float) => *self == float,
            _ => false,
        }
    }
}

impl PartialEq<bool> for Value {
    fn eq(&self, other: &bool) -> bool {
        match *self {
            Self::Boolean(bool) => bool == *other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for bool {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Boolean(bool) => *self == bool,
            _ => false,
        }
    }
}

impl PartialEq<Datetime> for Value {
    fn eq(&self, other: &Datetime) -> bool {
        match *self {
            Self::Datetime(ref datetime) => datetime == other,
            _ => false,
        }
    }
}

impl PartialEq<Value> for Datetime {
    fn eq(&self, other: &Value) -> bool {
        match *other {
            Value::Datetime(ref datetime) => self == datetime,
            _ => false,
        }
    }
}
