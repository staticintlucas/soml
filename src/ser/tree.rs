use std::result::Result as StdResult;

use serde::ser;

use crate::ser::{utils, writer, Error, ErrorKind, Result};
#[cfg(feature = "datetime")]
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    // Simple value (int, float, string, etc) or inline array/table/etc
    Inline(String),
    // Table or array of tables
    Table(Table),
}

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Self::Inline(ref s) => serializer.serialize_str(s),
            Self::Table(ref table) => table.serialize(serializer),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Table {
    // A regular table
    Table(Vec<(String, Value)>),
    // An array of tables
    Array(Vec<Vec<(String, Value)>>),
}

impl ser::Serialize for Table {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::{SerializeMap as _, SerializeSeq as _};

        struct SerTable<'a>(&'a [(String, Value)]);

        impl ser::Serialize for SerTable<'_> {
            fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
            where
                S: ser::Serializer,
            {
                let mut map = serializer.serialize_map(Some(self.0.len()))?;
                #[allow(clippy::pattern_type_mismatch)]
                for (key, value) in self.0 {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }

        match *self {
            Self::Table(ref table) => SerTable(table).serialize(serializer),
            Self::Array(ref arr) => {
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for table in arr {
                    seq.serialize_element(&SerTable(table))?;
                }
                seq.end()
            }
        }
    }
}

#[derive(Debug)]
pub enum Array {
    // An inline array
    Inline(String),
    // An array of tables
    Table(Vec<Vec<(String, Value)>>),
}

#[derive(Debug)]
struct Serializer;

impl ser::Serializer for Serializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = ArraySerializer;
    type SerializeTuple = ArraySerializer;
    type SerializeTupleStruct = ArraySerializer;
    type SerializeTupleVariant = WrappedArraySerializer;
    type SerializeMap = TableSerializer;
    #[cfg(feature = "datetime")]
    type SerializeStruct = TableOrDatetimeSerializer;
    #[cfg(not(feature = "datetime"))]
    type SerializeStruct = TableSerializer;
    type SerializeStructVariant = WrappedTableSerializer;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok> {
        Ok(Value::Inline(
            (if value { "true" } else { "false" }).to_string(),
        ))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        self.serialize_integer(&value)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok> {
        self.serialize_float(&value)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok> {
        self.serialize_float(&value)
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Self::Ok> {
        self.serialize_str(value.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        let mut buf = String::new();
        writer::Formatter::write_string(value, &mut buf)?;
        Ok(Value::Inline(buf))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        use ser::SerializeSeq as _;

        let mut seq = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        Err(ErrorKind::UnsupportedValue("None").into())
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(ErrorKind::UnsupportedType("()").into())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(Self::SerializeSeq::start(len))
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Ok(Self::SerializeTuple::start(Some(len)))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(Self::SerializeTupleStruct::start(Some(len)))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(Self::SerializeTupleVariant::start(variant, len))
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(Self::SerializeMap::start(len))
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        match name {
            #[cfg(feature = "datetime")]
            name => Ok(Self::SerializeStruct::start(name, len)),
            #[cfg(not(feature = "datetime"))]
            _ => Ok(Self::SerializeStruct::start(Some(len))),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(Self::SerializeStructVariant::start(variant, len))
    }
}

impl Serializer {
    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    #[inline]
    fn serialize_integer<T: writer::Integer>(self, value: &T) -> Result<Value> {
        let mut buf = String::new();
        writer::Formatter::write_integer(value, &mut buf)?;
        Ok(Value::Inline(buf))
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    #[inline]
    fn serialize_float<T: writer::Float>(self, value: &T) -> Result<Value> {
        let mut buf = String::new();
        writer::Formatter::write_float(value, &mut buf)?;
        Ok(Value::Inline(buf))
    }
}

#[derive(Debug)]
pub struct ArraySerializer {
    pub arr: Vec<Value>,
}

impl ArraySerializer {
    pub fn start(len: Option<usize>) -> Self {
        Self {
            arr: len.map_or_else(Vec::new, Vec::with_capacity),
        }
    }

    pub fn end_inner(self) -> Result<Array> {
        // If all elements are tables, we can return an array of tables
        if !self.arr.is_empty()
            && self
                .arr
                .iter()
                .all(|v| matches!(*v, Value::Table(Table::Table(_))))
        {
            Ok(Array::Table(
                self.arr
                    .into_iter()
                    .map(|table| match table {
                        Value::Table(Table::Table(table)) => table,
                        _ => unreachable!("we just checked they're all tables"),
                    })
                    .collect(),
            ))
        }
        // Otherwise format it as an inline array
        else {
            use ser::{SerializeSeq as _, Serializer as _};

            let mut buf = String::new();
            let mut ser = InlineSerializer::new(&mut buf).serialize_seq(Some(self.arr.len()))?;
            for el in &self.arr {
                ser.serialize_element(el)?;
            }
            ser.end()?;

            Ok(Array::Inline(buf))
        }
    }
}

impl ser::SerializeSeq for ArraySerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.arr.push(value.serialize(Serializer)?);
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        Ok(match self.end_inner()? {
            Array::Table(arr) => Value::Table(Table::Array(arr)),
            Array::Inline(arr) => Value::Inline(arr),
        })
    }
}

impl ser::SerializeTuple for ArraySerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for ArraySerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

#[derive(Debug)]
struct WrappedArraySerializer {
    key: &'static str,
    arr: ArraySerializer,
}

impl WrappedArraySerializer {
    #[inline]
    pub fn start(key: &'static str, len: usize) -> Self {
        Self {
            key,
            arr: ArraySerializer::start(Some(len)),
        }
    }

    pub fn end_inner(self) -> Result<(String, Value)> {
        use ser::SerializeTuple as _;

        Ok((self.key.to_string(), self.arr.end()?))
    }
}

impl ser::SerializeTupleVariant for WrappedArraySerializer {
    type Ok = Value;
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeTuple as _;

        self.arr.serialize_element(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Table(Table::Table(vec![self.end_inner()?])))
    }
}

#[derive(Debug)]
pub struct TableSerializer {
    pub table: Vec<(String, Value)>,
    key: Option<String>,
}

impl TableSerializer {
    #[inline]
    pub fn start(len: Option<usize>) -> Self {
        Self {
            table: len.map_or_else(Vec::new, Vec::with_capacity),
            key: None,
        }
    }

    #[inline]
    pub fn end_inner(self) -> Vec<(String, Value)> {
        self.table
    }
}

impl ser::SerializeMap for TableSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        let mut buf = String::new();
        key.serialize(utils::KeySerializer::new(&mut buf))?;
        self.key = Some(buf);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        #[allow(clippy::panic)]
        let Some(key) = self.key.take() else {
            panic!("TableSerializer::serialize_value called without calling TableSerializer::serialize_key first")
        };

        self.table.push((key, value.serialize(Serializer)?));
        Ok(())
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        let mut buf = String::new();
        key.serialize(utils::KeySerializer::new(&mut buf))?;
        self.table.push((buf, value.serialize(Serializer)?));
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        Ok(Value::Table(Table::Table(self.end_inner())))
    }
}

impl ser::SerializeStruct for TableSerializer {
    type Ok = <Self as ser::SerializeMap>::Ok;
    type Error = <Self as ser::SerializeMap>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

#[cfg(feature = "datetime")]
#[derive(Debug)]
enum TableOrDatetimeSerializer {
    // Used if type name is AnyDatetime::WRAPPER_TYPE. To detect the date-time type we use the field
    AnyDatetime,
    OffsetDatetime(Option<String>),
    LocalDatetime(Option<String>),
    LocalDate(Option<String>),
    LocalTime(Option<String>),
    Table(TableSerializer),
}

#[cfg(feature = "datetime")]
impl TableOrDatetimeSerializer {
    #[inline]
    pub fn start(name: &'static str, len: usize) -> Self {
        match name {
            AnyDatetime::WRAPPER_TYPE => Self::AnyDatetime,
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(None),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(None),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(None),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(None),
            _ => Self::Table(TableSerializer::start(Some(len))),
        }
    }
}

#[cfg(feature = "datetime")]
impl ser::SerializeStruct for TableOrDatetimeSerializer {
    type Ok = <TableSerializer as ser::SerializeStruct>::Ok;
    type Error = <TableSerializer as ser::SerializeStruct>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match *self {
            // For AnyDatetime use the key to determine the type
            Self::OffsetDatetime(None) | Self::AnyDatetime
                if key == OffsetDatetime::WRAPPER_FIELD =>
            {
                let mut buf = String::new();
                value.serialize(utils::RawStringSerializer { writer: &mut buf })?;
                *self = Self::OffsetDatetime(Some(buf));
                Ok(())
            }
            Self::LocalDatetime(None) | Self::AnyDatetime
                if key == LocalDatetime::WRAPPER_FIELD =>
            {
                let mut buf = String::new();
                value.serialize(utils::RawStringSerializer { writer: &mut buf })?;
                *self = Self::LocalDatetime(Some(buf));
                Ok(())
            }
            Self::LocalDate(None) | Self::AnyDatetime if key == LocalDate::WRAPPER_FIELD => {
                let mut buf = String::new();
                value.serialize(utils::RawStringSerializer { writer: &mut buf })?;
                *self = Self::LocalDate(Some(buf));
                Ok(())
            }
            Self::LocalTime(None) | Self::AnyDatetime if key == LocalTime::WRAPPER_FIELD => {
                let mut buf = String::new();
                value.serialize(utils::RawStringSerializer { writer: &mut buf })?;
                *self = Self::LocalTime(Some(buf));
                Ok(())
            }
            Self::OffsetDatetime(Some(_))
            | Self::LocalDatetime(Some(_))
            | Self::LocalDate(Some(_))
            | Self::LocalTime(Some(_)) => Err(ErrorKind::UnsupportedValue(
                "date-time wrapper with more than one member",
            )
            .into()),
            Self::AnyDatetime
            | Self::OffsetDatetime(_)
            | Self::LocalDatetime(_)
            | Self::LocalDate(_)
            | Self::LocalTime(_) => Err(ErrorKind::UnsupportedValue(key).into()),
            Self::Table(ref mut ser) => ser.serialize_field(key, value),
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        match self {
            Self::OffsetDatetime(Some(str))
            | Self::LocalDatetime(Some(str))
            | Self::LocalDate(Some(str))
            | Self::LocalTime(Some(str)) => Ok(Value::Inline(str)),
            Self::AnyDatetime
            | Self::OffsetDatetime(None)
            | Self::LocalDatetime(None)
            | Self::LocalDate(None)
            | Self::LocalTime(None) => {
                Err(ErrorKind::UnsupportedValue("empty date-time wrapper").into())
            }
            Self::Table(ser) => ser.end(),
        }
    }
}

#[derive(Debug)]
struct WrappedTableSerializer {
    key: &'static str,
    table: TableSerializer,
}

impl WrappedTableSerializer {
    #[inline]
    pub fn start(key: &'static str, len: usize) -> Self {
        Self {
            key,
            table: TableSerializer::start(Some(len)),
        }
    }
}

impl ser::SerializeStructVariant for WrappedTableSerializer {
    type Ok = Value;
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeStruct as _;

        self.table.serialize_field(key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        use ser::SerializeStruct as _;

        Ok(Value::Table(Table::Table(vec![(
            self.key.to_string(),
            self.table.end()?,
        )])))
    }
}

// Serializes to an inline string, assuming inline values have already been serialized as inline
// strings by `Serializer`.
#[derive(Debug)]
struct InlineSerializer<'a> {
    buf: &'a mut String,
}

impl<'a> InlineSerializer<'a> {
    #[inline]
    pub fn new(buf: &'a mut String) -> Self {
        Self { buf }
    }
}

impl<'a> ser::Serializer for InlineSerializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = InlineArraySerializer<'a>;
    type SerializeMap = InlineTableSerializer<'a>;

    utils::__serialize_unsupported!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes none
        some unit unit_struct unit_variant newtype_struct newtype_variant
        tuple tuple_struct tuple_variant struct struct_variant
    );

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        self.buf.push_str(value);
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(Self::SerializeSeq::start(self.buf))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(Self::SerializeMap::start(self.buf))
    }
}

#[derive(Debug)]
pub struct InlineArraySerializer<'a> {
    buf: &'a mut String,
    first: bool,
}

impl<'a> InlineArraySerializer<'a> {
    #[inline]
    pub fn start(buf: &'a mut String) -> Self {
        buf.push('[');
        Self { buf, first: true }
    }
}

impl ser::SerializeSeq for InlineArraySerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.buf.push_str(", ");
        }
        self.first = false;

        value.serialize(InlineSerializer::new(self.buf))
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.buf.push(']');
        Ok(())
    }
}

#[derive(Debug)]
pub struct InlineTableSerializer<'a> {
    buf: &'a mut String,
    first: bool,
}

impl<'a> InlineTableSerializer<'a> {
    #[inline]
    pub fn start(buf: &'a mut String) -> Self {
        buf.push_str("{ ");
        Self { buf, first: true }
    }
}

impl ser::SerializeMap for InlineTableSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if !self.first {
            self.buf.push_str(", ");
        }
        self.first = false;

        key.serialize(utils::KeySerializer::new(self.buf))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.buf.push_str(" = ");
        value.serialize(InlineSerializer::new(self.buf))
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.buf.push_str(" }");
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use maplit::hashmap;
    use serde::Serializer as _;
    #[cfg(feature = "datetime")]
    use serde_bytes::Bytes;
    use serde_test::{assert_ser_tokens, Token};

    use super::*;

    #[test]
    fn serialize_value() {
        let value = Value::Inline("42".to_string());
        let tokens = [Token::Str("42")];
        assert_ser_tokens(&value, &tokens);

        let value = Value::Table(Table::Table(vec![
            ("foo".to_string(), Value::Inline("42".to_string())),
            ("bar".to_string(), Value::Inline(r#""baz""#.to_string())),
        ]));
        let tokens = [
            Token::Map { len: Some(2) },
            Token::Str("foo"),
            Token::Str("42"),
            Token::Str("bar"),
            Token::Str(r#""baz""#),
            Token::MapEnd,
        ];
        assert_ser_tokens(&value, &tokens);
    }

    #[test]
    fn serialize_table() {
        let value = Table::Table(vec![
            ("foo".to_string(), Value::Inline("42".to_string())),
            ("bar".to_string(), Value::Inline(r#""baz""#.to_string())),
        ]);
        let tokens = [
            Token::Map { len: Some(2) },
            Token::Str("foo"),
            Token::Str("42"),
            Token::Str("bar"),
            Token::Str(r#""baz""#),
            Token::MapEnd,
        ];
        assert_ser_tokens(&value, &tokens);

        let value = Table::Array(vec![
            vec![("foo".to_string(), Value::Inline("42".to_string()))],
            vec![("bar".to_string(), Value::Inline(r#""baz""#.to_string()))],
        ]);
        let tokens = [
            Token::Seq { len: Some(2) },
            Token::Map { len: Some(1) },
            Token::Str("foo"),
            Token::Str("42"),
            Token::MapEnd,
            Token::Map { len: Some(1) },
            Token::Str("bar"),
            Token::Str(r#""baz""#),
            Token::MapEnd,
            Token::SeqEnd,
        ];
        assert_ser_tokens(&value, &tokens);
    }

    #[test]
    fn serializer_serialize_bool() {
        assert_matches!(Serializer.serialize_bool(true), Ok(Value::Inline(v)) if v == "true");
        assert_matches!(Serializer.serialize_bool(false), Ok(Value::Inline(v)) if v == "false");
    }

    #[test]
    fn serializer_serialize_i8() {
        assert_matches!(Serializer.serialize_i8(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_i8(-12), Ok(Value::Inline(v)) if v == "-12");
    }

    #[test]
    fn serializer_serialize_i16() {
        assert_matches!(Serializer.serialize_i16(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_i16(-12), Ok(Value::Inline(v)) if v == "-12");
    }

    #[test]
    fn serializer_serialize_i32() {
        assert_matches!(Serializer.serialize_i32(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_i32(-12), Ok(Value::Inline(v)) if v == "-12");
    }

    #[test]
    fn serializer_serialize_i64() {
        assert_matches!(Serializer.serialize_i64(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_i64(-12), Ok(Value::Inline(v)) if v == "-12");
    }

    #[test]
    fn serializer_serialize_i128() {
        assert_matches!(Serializer.serialize_i128(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_i128(-12), Ok(Value::Inline(v)) if v == "-12");
    }

    #[test]
    fn serializer_serialize_u8() {
        assert_matches!(Serializer.serialize_u8(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_u8(12), Ok(Value::Inline(v)) if v == "12");
    }

    #[test]
    fn serializer_serialize_u16() {
        assert_matches!(Serializer.serialize_u16(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_u16(12), Ok(Value::Inline(v)) if v == "12");
    }

    #[test]
    fn serializer_serialize_u32() {
        assert_matches!(Serializer.serialize_u32(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_u32(12), Ok(Value::Inline(v)) if v == "12");
    }

    #[test]
    fn serializer_serialize_u64() {
        assert_matches!(Serializer.serialize_u64(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_u64(12), Ok(Value::Inline(v)) if v == "12");
    }

    #[test]
    fn serializer_serialize_u128() {
        assert_matches!(Serializer.serialize_u128(42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_u128(12), Ok(Value::Inline(v)) if v == "12");
    }

    #[test]
    fn serializer_serialize_f32() {
        assert_matches!(Serializer.serialize_f32(42.0), Ok(Value::Inline(v)) if v == "42.0");
        assert_matches!(Serializer.serialize_f32(-12.0), Ok(Value::Inline(v)) if v == "-12.0");
        assert_matches!(Serializer.serialize_f32(1e28), Ok(Value::Inline(v)) if v == "1e28");
        assert_matches!(Serializer.serialize_f32(0.5e-9), Ok(Value::Inline(v)) if v == "5e-10");
        assert_matches!(
            Serializer.serialize_f32(f32::INFINITY),
            Ok(Value::Inline(v)) if v == "inf"
        );
        assert_matches!(
            Serializer.serialize_f32(f32::NEG_INFINITY),
            Ok(Value::Inline(v)) if v == "-inf"
        );
        assert_matches!(Serializer.serialize_f32(f32::NAN), Ok(Value::Inline(v)) if v == "nan");
        assert_matches!(Serializer.serialize_f32(-f32::NAN), Ok(Value::Inline(v)) if v == "-nan");
    }

    #[test]
    fn serializer_serialize_f64() {
        assert_matches!(Serializer.serialize_f64(42.0), Ok(Value::Inline(v)) if v == "42.0");
        assert_matches!(Serializer.serialize_f64(-12.0), Ok(Value::Inline(v)) if v == "-12.0");
        assert_matches!(Serializer.serialize_f64(1e28), Ok(Value::Inline(v)) if v == "1e28");
        assert_matches!(Serializer.serialize_f64(0.5e-9), Ok(Value::Inline(v)) if v == "5e-10");
        assert_matches!(
            Serializer.serialize_f64(f64::INFINITY),
            Ok(Value::Inline(v)) if v == "inf"
        );
        assert_matches!(
            Serializer.serialize_f64(f64::NEG_INFINITY),
            Ok(Value::Inline(v)) if v == "-inf"
        );
        assert_matches!(Serializer.serialize_f64(f64::NAN), Ok(Value::Inline(v)) if v == "nan");
        assert_matches!(Serializer.serialize_f64(-f64::NAN), Ok(Value::Inline(v)) if v == "-nan");
    }

    #[test]
    fn serializer_serialize_char() {
        assert_matches!(Serializer.serialize_char('a'), Ok(Value::Inline(v)) if v == r#""a""#);
        assert_matches!(Serializer.serialize_char('😎'), Ok(Value::Inline(v)) if v == r#""😎""#);
        assert_matches!(
            Serializer.serialize_char('\n'),
            Ok(Value::Inline(v))
                if v == indoc! {r#"
                    """

                    """"#}
        );
    }

    #[test]
    fn serializer_serialize_str() {
        assert_matches!(Serializer.serialize_str("foo"), Ok(Value::Inline(v)) if v == r#""foo""#);
        assert_matches!(Serializer.serialize_str("😎"), Ok(Value::Inline(v)) if v == r#""😎""#);
        assert_matches!(
            Serializer.serialize_str("abc\ndef\n"),
            Ok(Value::Inline(v))
                if v == indoc! {r#"
                    """
                    abc
                    def
                    """"#}
        );
    }

    #[test]
    fn serializer_serialize_bytes() {
        assert_matches!(
            Serializer.serialize_bytes(b"foo"),
            Ok(Value::Inline(v)) if v == "[102, 111, 111]"
        );
        assert_matches!(
            Serializer.serialize_bytes(b"\xF0\x9F\x98\x8E"),
            Ok(Value::Inline(v)) if v == "[240, 159, 152, 142]"
        );
        assert_matches!(
            Serializer.serialize_bytes(b"abc\ndef\n"),
            Ok(Value::Inline(v)) if v == "[97, 98, 99, 10, 100, 101, 102, 10]"
        );
    }

    #[test]
    fn serializer_serialize_none() {
        assert_matches!(
            Serializer.serialize_none(),
            Err(Error(ErrorKind::UnsupportedValue(..)))
        );
    }

    #[test]
    fn serializer_serialize_some() {
        assert_matches!(Serializer.serialize_some(&42), Ok(Value::Inline(v)) if v == "42");
        assert_matches!(Serializer.serialize_some("foo"), Ok(Value::Inline(v)) if v == r#""foo""#);
    }

    #[test]
    fn serializer_serialize_unit() {
        assert_matches!(
            Serializer.serialize_unit(),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
    }

    #[test]
    fn serializer_serialize_unit_struct() {
        assert_matches!(
            Serializer.serialize_unit_struct("name"),
            Err(Error(ErrorKind::UnsupportedType(..)))
        );
    }

    #[test]
    fn serializer_serialize_unit_variant() {
        assert_matches!(
            Serializer.serialize_unit_variant("name", 0, "foo"),
            Ok(Value::Inline(v)) if v == r#""foo""#
        );
    }

    #[test]
    fn serializer_serialize_newtype_struct() {
        assert_matches!(
            Serializer.serialize_newtype_struct("name", &42),
            Ok(Value::Inline(v)) if v == "42"
        );
    }

    #[test]
    fn serializer_serialize_newtype_variant() {
        assert_matches!(
            Serializer.serialize_newtype_variant("name", 0, "foo", &42),
            Ok(Value::Table(Table::Table(t)))
                if matches!(t[..], [(ref k, Value::Inline(ref v))] if k == "foo" && v == "42")
        );
    }

    #[test]
    fn serializer_serialize_seq() {
        assert_matches!(
            Serializer.serialize_seq(Some(2)),
            Ok(ArraySerializer { arr }) if arr.capacity() == 2
        );
    }

    #[test]
    fn serializer_serialize_tuple() {
        assert_matches!(
            Serializer.serialize_tuple(2),
            Ok(ArraySerializer { arr }) if arr.capacity() == 2
        );
    }

    #[test]
    fn serializer_serialize_tuple_struct() {
        assert_matches!(
            Serializer.serialize_tuple_struct("name", 2),
            Ok(ArraySerializer { arr }) if arr.capacity() == 2
        );
    }

    #[test]
    fn serializer_serialize_tuple_variant() {
        assert_matches!(
            Serializer.serialize_tuple_variant("name", 0, "foo", 2),
            Ok(WrappedArraySerializer {
                key,
                arr: ArraySerializer { arr },
            }) if key == "foo" && arr.capacity() == 2
        );
    }

    #[test]
    fn serializer_serialize_map() {
        assert_matches!(
            Serializer.serialize_map(Some(2)),
            Ok(TableSerializer { table, key: None }) if table.capacity() == 2
        );
    }

    #[test]
    fn serializer_serialize_struct() {
        #[cfg(feature = "datetime")]
        assert_matches!(
            Serializer.serialize_struct("name", 2),
            Ok(TableOrDatetimeSerializer::Table(TableSerializer { table, key: None }))
                if table.capacity() == 2
        );

        #[cfg(not(feature = "datetime"))]
        assert_matches!(
            Serializer.serialize_struct("name", 2),
            Ok(TableSerializer { table, key: None })
                if table.capacity() == 2
        );
    }

    #[test]
    fn serializer_serialize_struct_variant() {
        assert_matches!(
            Serializer.serialize_struct_variant("name", 0, "foo", 2),
            Ok(WrappedTableSerializer {
                key,
                table: TableSerializer { table, key: None }
            }) if key == "foo" && table.capacity() == 2
        );
    }

    #[test]
    fn array_serializer_seq() {
        use ser::SerializeSeq as _;

        let mut array = ArraySerializer::start(None);
        assert!(array.arr.is_empty());
        assert_eq!(array.arr.capacity(), 0);

        array.serialize_element(&42).unwrap();
        assert_eq!(array.arr.len(), 1);

        array.serialize_element(&"foo").unwrap();
        assert_eq!(array.arr.len(), 2);

        let result = array.end().unwrap();
        assert_matches!(result, Value::Inline(v) if v == r#"[42, "foo"]"#);
    }

    #[test]
    fn array_serializer_tuple() {
        use ser::SerializeTuple as _;

        let mut array = ArraySerializer::start(Some(2));
        assert!(array.arr.is_empty());
        assert_eq!(array.arr.capacity(), 2);

        array.serialize_element(&42).unwrap();
        assert_eq!(array.arr.len(), 1);

        array.serialize_element(&"foo").unwrap();
        assert_eq!(array.arr.len(), 2);

        assert_matches!(array.end().unwrap(), Value::Inline(v) if v == r#"[42, "foo"]"#);
    }

    #[test]
    fn array_serializer_tuple_struct() {
        use ser::SerializeTupleStruct as _;

        let mut array = ArraySerializer::start(Some(2));
        assert!(array.arr.is_empty());
        assert_eq!(array.arr.capacity(), 2);

        array.serialize_field(&42).unwrap();
        assert_eq!(array.arr.len(), 1);

        array.serialize_field(&"foo").unwrap();
        assert_eq!(array.arr.len(), 2);

        assert_matches!(array.end().unwrap(), Value::Inline(v) if v == r#"[42, "foo"]"#);
    }

    #[test]
    fn array_serializer_array_of_tables() {
        use ser::SerializeSeq as _;

        let mut array = ArraySerializer::start(None);
        assert!(array.arr.is_empty());
        assert_eq!(array.arr.capacity(), 0);

        array
            .serialize_element(&hashmap! { "foo" => "bar" })
            .unwrap();
        assert_eq!(array.arr.len(), 1);

        array
            .serialize_element(&hashmap! { "baz" => "qux" })
            .unwrap();
        assert_eq!(array.arr.len(), 2);

        assert_matches!(
            array.end().unwrap(),
            Value::Table(Table::Array(a))
                if matches!(
                    a[..],
                    [ref t1, ref t2]
                        if matches!(
                            t1[..],
                            [(ref k, Value::Inline(ref v))] if k == "foo" && v == r#""bar""#
                        ) && matches!(
                            t2[..],
                            [(ref k, Value::Inline(ref v))] if k == "baz" && v == r#""qux""#
                        )
                )
        );
    }

    #[test]
    fn wrapped_array_serializer() {
        use ser::SerializeTupleVariant as _;

        let mut array = WrappedArraySerializer::start("foo", 2);
        assert_eq!(array.key, "foo");
        assert!(array.arr.arr.is_empty());
        assert_eq!(array.arr.arr.capacity(), 2);

        array.serialize_field(&42).unwrap();
        assert_eq!(array.arr.arr.len(), 1);

        array.serialize_field(&"bar").unwrap();
        assert_eq!(array.arr.arr.len(), 2);

        assert_matches!(array.end().unwrap(), Value::Table(Table::Table(t))
            if matches!(
                t[..],
                [(ref k, Value::Inline(ref v))] if k == "foo" && v == r#"[42, "bar"]"#
            )
        );
    }

    #[test]
    fn table_serializer_map() {
        use ser::SerializeMap as _;

        let mut table = TableSerializer::start(None);
        assert!(table.key.is_none());
        assert!(table.table.is_empty());
        assert_eq!(table.table.capacity(), 0);

        table.serialize_key("foo").unwrap();
        assert!(table.key.is_some());
        assert!(table.table.is_empty());

        table.serialize_value(&42).unwrap();
        assert!(table.key.is_none());
        assert_eq!(table.table.len(), 1);

        table.serialize_entry("bar", &"baz").unwrap();
        assert!(table.key.is_none());
        assert_eq!(table.table.len(), 2);

        assert_matches!(
            table.end().unwrap(),
            Value::Table(Table::Table(t))
                if matches!(
                    t[..],
                    [(ref k1, Value::Inline(ref v1)), (ref k2, Value::Inline(ref v2))]
                        if k1 == "foo" && v1 == "42" && k2 == "bar" && v2 == r#""baz""#
                )
        );
    }

    #[test]
    fn table_serializer_struct() {
        use ser::SerializeStruct as _;

        let mut table = TableSerializer::start(Some(2));
        assert!(table.key.is_none());
        assert!(table.table.is_empty());
        assert_eq!(table.table.capacity(), 2);

        table.serialize_field("foo", &42).unwrap();
        assert!(table.key.is_none());
        assert_eq!(table.table.len(), 1);
        assert_eq!(table.table.capacity(), 2);

        table.serialize_field("bar", &"baz").unwrap();
        assert!(table.key.is_none());
        assert_eq!(table.table.len(), 2);
        assert_eq!(table.table.capacity(), 2);

        assert_matches!(
            table.end().unwrap(),
            Value::Table(Table::Table(t))
                if matches!(
                    t[..],
                    [(ref k1, Value::Inline(ref v1)), (ref k2, Value::Inline(ref v2))]
                        if k1 == "foo" && v1 == "42" && k2 == "bar" && v2 == r#""baz""#
                )
        );
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_struct() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start("foo", 2);
        assert_matches!(table, TODS::Table(ref table) if table.key.is_none());
        assert_matches!(table, TODS::Table(ref table) if table.table.is_empty());
        assert_matches!(table, TODS::Table(ref table) if table.table.capacity() == 2);

        table.serialize_field("foo", &42).unwrap();
        assert_matches!(table, TODS::Table(ref table) if table.key.is_none());
        assert_matches!(table, TODS::Table(ref table) if table.table.len() == 1);
        assert_matches!(table, TODS::Table(ref table) if table.table.capacity() == 2);

        table.serialize_field("bar", &"baz").unwrap();
        assert_matches!(table, TODS::Table(ref table) if table.key.is_none());
        assert_matches!(table, TODS::Table(ref table) if table.table.len() == 2);
        assert_matches!(table, TODS::Table(ref table) if table.table.capacity() == 2);

        assert_matches!(
            table.end().unwrap(),
            Value::Table(Table::Table(t))
                if matches!(
                    t[..],
                    [(ref k1, Value::Inline(ref v1)), (ref k2, Value::Inline(ref v2))]
                        if k1 == "foo" && v1 == "42" && k2 == "bar" && v2 == r#""baz""#
                )
        );
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_offset_datetime() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(OffsetDatetime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::OffsetDatetime(None));

        table
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(
            table,
            TODS::OffsetDatetime(Some(ref d)) if d == OffsetDatetime::EXAMPLE_STR
        );

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == OffsetDatetime::EXAMPLE_STR);
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_local_datetime() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(LocalDatetime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::LocalDatetime(None));

        table
            .serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                Bytes::new(LocalDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(table, TODS::LocalDatetime(Some(ref d)) if d == LocalDatetime::EXAMPLE_STR);

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == LocalDatetime::EXAMPLE_STR);
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_local_date() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(LocalDate::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::LocalDate(None));

        table
            .serialize_field(
                LocalDate::WRAPPER_FIELD,
                Bytes::new(LocalDate::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(table, TODS::LocalDate(Some(ref d)) if d == LocalDate::EXAMPLE_STR);

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == LocalDate::EXAMPLE_STR);
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_local_time() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(LocalTime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::LocalTime(None));

        table
            .serialize_field(
                LocalTime::WRAPPER_FIELD,
                Bytes::new(LocalTime::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(table, TODS::LocalTime(Some(ref d)) if d == LocalTime::EXAMPLE_STR);

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == LocalTime::EXAMPLE_STR);
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_any_datetime() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(AnyDatetime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::AnyDatetime);

        table
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(
            table,
            TODS::OffsetDatetime(Some(ref d)) if d == OffsetDatetime::EXAMPLE_STR
        );

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == OffsetDatetime::EXAMPLE_STR);

        let mut table = TODS::start(AnyDatetime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::AnyDatetime);

        table
            .serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                Bytes::new(LocalDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(table, TODS::LocalDatetime(Some(ref d)) if d == LocalDatetime::EXAMPLE_STR);

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == LocalDatetime::EXAMPLE_STR);

        let mut table = TODS::start(AnyDatetime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::AnyDatetime);

        table
            .serialize_field(
                LocalDate::WRAPPER_FIELD,
                Bytes::new(LocalDate::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(table, TODS::LocalDate(Some(ref d)) if d == LocalDate::EXAMPLE_STR);

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == LocalDate::EXAMPLE_STR);

        let mut table = TODS::start(AnyDatetime::WRAPPER_TYPE, 1);
        assert_matches!(table, TODS::AnyDatetime);

        table
            .serialize_field(
                LocalTime::WRAPPER_FIELD,
                Bytes::new(LocalTime::EXAMPLE_BYTES),
            )
            .unwrap();
        assert_matches!(table, TODS::LocalTime(Some(ref d)) if d == LocalTime::EXAMPLE_STR);

        assert_matches!(table.end().unwrap(), Value::Inline(v) if v == LocalTime::EXAMPLE_STR);
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_multiple_fields() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(OffsetDatetime::WRAPPER_TYPE, 2);

        table
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            )
            .unwrap();

        let result = table.serialize_field(
            OffsetDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_incorrect_key() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(OffsetDatetime::WRAPPER_TYPE, 2);

        let result = table.serialize_field(
            LocalDatetime::WRAPPER_FIELD,
            Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_incorrect_type() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let mut table = TODS::start(OffsetDatetime::WRAPPER_TYPE, 2);

        let result = table.serialize_field(OffsetDatetime::WRAPPER_FIELD, &42);
        assert!(result.is_err());
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn table_or_datetime_serializer_no_fields() {
        use ser::SerializeStruct as _;
        use TableOrDatetimeSerializer as TODS;

        let table = TODS::start(OffsetDatetime::WRAPPER_TYPE, 2);
        assert!(table.end().is_err());
    }

    #[test]
    fn wrapped_table_serializer() {
        use ser::SerializeStructVariant as _;

        let mut table = WrappedTableSerializer::start("foo", 2);
        assert_eq!(table.key, "foo");
        assert!(table.table.key.is_none());
        assert!(table.table.table.is_empty());
        assert_eq!(table.table.table.capacity(), 2);

        table.serialize_field("bar", &42).unwrap();
        assert!(table.table.key.is_none());
        assert_eq!(table.table.table.len(), 1);
        assert_eq!(table.table.table.capacity(), 2);

        table.serialize_field("baz", &"qux").unwrap();
        assert!(table.table.key.is_none());
        assert_eq!(table.table.table.len(), 2);
        assert_eq!(table.table.table.capacity(), 2);

        assert_matches!(
            table.end().unwrap(),
            Value::Table(Table::Table(t))
                if matches!(
                    t[..],
                    [(ref k, Value::Table(Table::Table(ref v)))]
                        if k == "foo" && matches!(
                            v[..],
                            [(ref k1, Value::Inline(ref v1)), (ref k2, Value::Inline(ref v2))]
                                if k1 == "bar" && v1 == "42" && k2 == "baz" && v2 == r#""qux""#
                        )
                )
        );
    }

    #[test]
    fn inline_serializer_serialize_str() {
        let mut buf = String::new();
        let ser = InlineSerializer::new(&mut buf);
        ser.serialize_str("foo").unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        let ser = InlineSerializer::new(&mut buf);
        ser.serialize_str("😎").unwrap();
        assert_eq!(buf, "😎");
    }

    #[test]
    fn inline_serializer_serialize_seq() {
        let mut buf = String::new();
        let ser = InlineSerializer::new(&mut buf);
        assert_matches!(ser.serialize_seq(Some(2)), Ok(InlineArraySerializer { .. }));
    }

    #[test]
    fn inline_serializer_serialize_map() {
        let mut buf = String::new();
        let ser = InlineSerializer::new(&mut buf);
        assert_matches!(ser.serialize_map(Some(2)), Ok(InlineTableSerializer { .. }));
    }

    #[test]
    fn inline_array_serializer() {
        use ser::SerializeSeq as _;

        let mut buf = String::new();
        let mut array = InlineArraySerializer::start(&mut buf);

        array.serialize_element("42").unwrap();
        array.serialize_element(r#""foo""#).unwrap();

        array.end().unwrap();
        assert_eq!(buf, r#"[42, "foo"]"#);
    }

    #[test]
    fn inline_table_serializer() {
        use ser::SerializeMap as _;

        let mut buf = String::new();
        let mut table = InlineTableSerializer::start(&mut buf);

        table.serialize_key("foo").unwrap();
        table.serialize_value("42").unwrap();
        table.serialize_entry("bar", &r#""baz""#).unwrap();

        table.end().unwrap();
        assert_eq!(buf, r#"{ foo = 42, bar = "baz" }"#);
    }
}
