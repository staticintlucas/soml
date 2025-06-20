use serde::ser;

use crate::ser::utils::{Float, Integer};
use crate::ser::{utils, writer, Error, ErrorKind, Result};
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

#[derive(Debug)]
pub enum Value {
    // Simple value (int, float, string, etc) or inline array/table/etc
    Inline(String),
    // Table or array of tables
    Table(Table),
}

#[derive(Debug)]
pub enum Table {
    // A regular table
    Table(Vec<(String, Value)>),
    // An array of tables
    Array(Vec<Vec<(String, Value)>>),
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
    type SerializeStruct = TableOrDatetimeSerializer;
    type SerializeStructVariant = WrappedTableSerializer;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok> {
        Ok(Value::Inline(
            (if value { "true" } else { "false" }).to_string(),
        ))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Self::Ok> {
        self.serialize_str(value.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        Ok(Value::Inline(writer::TomlString(value).to_string()))
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
        Ok(Self::SerializeStruct::start(name, len))
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
    fn serialize_integer<T: Integer>(self, value: T) -> Result<Value> {
        Ok(Value::Inline(value.to_string()))
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    #[inline]
    fn serialize_float<T: Float>(self, value: T) -> Result<Value> {
        Ok(Value::Inline(value.to_string()))
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

    pub fn end_inner(self) -> Array {
        // If all elements are tables, we can return an array of tables
        if !self.arr.is_empty()
            && self
                .arr
                .iter()
                .all(|v| matches!(*v, Value::Table(Table::Table(_))))
        {
            Array::Table(
                self.arr
                    .into_iter()
                    .map(|table| match table {
                        Value::Table(Table::Table(table)) => table,
                        _ => unreachable!("we just checked they're all tables"),
                    })
                    .collect(),
            )
        }
        // Otherwise format it as an inline array
        else {
            Array::Inline(writer::InlineArray(&self.arr).to_string())
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
        Ok(match self.end_inner() {
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
    table: Vec<(String, Value)>,
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
        self.key = Some(key.serialize(utils::RawStringSerializer)?);
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
        self.table.push((
            key.serialize(utils::RawStringSerializer)?,
            value.serialize(Serializer)?,
        ));
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

#[derive(Debug)]
enum TableOrDatetimeSerializer {
    // Used if type name is AnyDatetime::WRAPPER_TYPE. To detect the datetime type we use the field
    AnyDatetime,
    OffsetDatetime(Option<Vec<u8>>),
    LocalDatetime(Option<Vec<u8>>),
    LocalDate(Option<Vec<u8>>),
    LocalTime(Option<Vec<u8>>),
    Table(TableSerializer),
}

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
                *self = Self::OffsetDatetime(Some(value.serialize(utils::RawBytesSerializer)?));
                Ok(())
            }
            Self::LocalDatetime(None) | Self::AnyDatetime
                if key == LocalDatetime::WRAPPER_FIELD =>
            {
                *self = Self::LocalDatetime(Some(value.serialize(utils::RawBytesSerializer)?));
                Ok(())
            }
            Self::LocalDate(None) | Self::AnyDatetime if key == LocalDate::WRAPPER_FIELD => {
                *self = Self::LocalDate(Some(value.serialize(utils::RawBytesSerializer)?));
                Ok(())
            }
            Self::LocalTime(None) | Self::AnyDatetime if key == LocalTime::WRAPPER_FIELD => {
                *self = Self::LocalTime(Some(value.serialize(utils::RawBytesSerializer)?));
                Ok(())
            }
            Self::OffsetDatetime(Some(_))
            | Self::LocalDatetime(Some(_))
            | Self::LocalDate(Some(_))
            | Self::LocalTime(Some(_)) => Err(ErrorKind::UnsupportedValue(
                "datetime wrapper with more than one member",
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
            Self::OffsetDatetime(Some(bytes))
            | Self::LocalDatetime(Some(bytes))
            | Self::LocalDate(Some(bytes))
            | Self::LocalTime(Some(bytes)) => String::from_utf8(bytes)
                .map(Value::Inline)
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
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
