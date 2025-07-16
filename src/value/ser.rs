use serde::ser;

use super::Value;
#[cfg(feature = "datetime")]
use super::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};
use crate::ser::{Error, ErrorKind};
use crate::{Table, __serialize_unsupported};

impl ser::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Self::String(ref str) => str.serialize(serializer),
            Self::Integer(int) => int.serialize(serializer),
            Self::Float(float) => float.serialize(serializer),
            Self::Boolean(bool) => bool.serialize(serializer),
            #[cfg(feature = "datetime")]
            Self::Datetime(ref datetime) => datetime.serialize(serializer),
            Self::Array(ref array) => array.serialize(serializer),
            Self::Table(ref table) => table.serialize(serializer),
        }
    }
}

pub struct ToValueSerializer;

impl ser::Serializer for ToValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = ToValueArraySerializer;
    type SerializeTuple = ToValueArraySerializer;
    type SerializeTupleStruct = ToValueArraySerializer;
    type SerializeTupleVariant = ToValueWrappedArraySerializer;
    type SerializeMap = ToValueTableSerializer;
    #[cfg(feature = "datetime")]
    type SerializeStruct = ToValueTableOrDatetimeSerializer;
    #[cfg(not(feature = "datetime"))]
    type SerializeStruct = ToValueTableSerializer;
    type SerializeStructVariant = ToValueWrappedTableSerializer;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Boolean(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Integer(value))
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(
            value
                .try_into()
                .map_err(|_| ErrorKind::UnsupportedValue("integer out of range of i64"))?,
        )
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(
            value
                .try_into()
                .map_err(|_| ErrorKind::UnsupportedValue("integer out of range of i64"))?,
        )
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(
            value
                .try_into()
                .map_err(|_| ErrorKind::UnsupportedValue("integer out of range of i64"))?,
        )
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(value.into())
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Float(value))
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(value.encode_utf8(&mut [0; 4]))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::String(value.into()))
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeSeq as _;

        let mut seq = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ErrorKind::UnsupportedValue("None").into())
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ErrorKind::UnsupportedType("()").into())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Self::SerializeSeq::start(len)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Self::SerializeTuple::start(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Self::SerializeTupleStruct::start(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Self::SerializeTupleVariant::start(len, variant)
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Self::SerializeMap::start(len)
    }

    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        match name {
            #[cfg(feature = "datetime")]
            name => Self::SerializeStruct::start(Some(len), name),
            #[cfg(not(feature = "datetime"))]
            _ => Self::SerializeStruct::start(Some(len)),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Self::SerializeStructVariant::start(len, variant)
    }
}

#[derive(Debug)]
pub struct ToValueArraySerializer {
    array: Vec<Value>,
}

impl ToValueArraySerializer {
    #[allow(clippy::unnecessary_wraps)]
    #[inline]
    fn start(len: Option<usize>) -> Result<Self, Error> {
        let array = Vec::with_capacity(len.unwrap_or(0));
        Ok(Self { array })
    }
}

impl ser::SerializeSeq for ToValueArraySerializer {
    type Ok = Value;
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.array.push(value.serialize(ToValueSerializer)?);
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Array(self.array))
    }
}

impl ser::SerializeTuple for ToValueArraySerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for ToValueArraySerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

#[derive(Debug)]
pub struct ToValueWrappedArraySerializer {
    key: String,
    array: ToValueArraySerializer,
}

impl ToValueWrappedArraySerializer {
    #[inline]
    fn start(len: usize, key: &'static str) -> Result<Self, Error> {
        Ok(Self {
            key: key.to_owned(),
            array: ToValueArraySerializer::start(Some(len))?,
        })
    }
}

impl ser::SerializeTupleVariant for ToValueWrappedArraySerializer {
    type Ok = Value;
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeTuple as _;

        self.array.serialize_element(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeTuple as _;

        Ok(Value::Table([(self.key, self.array.end()?)].into()))
    }
}

#[derive(Debug)]
pub struct ToValueTableSerializer {
    key: Option<String>,
    table: Table,
}

impl ToValueTableSerializer {
    #[allow(clippy::unnecessary_wraps)]
    #[inline]
    fn start(_len: Option<usize>) -> Result<Self, Error> {
        let table = Table::new(); // BTreeMap has no with_capacity
        Ok(Self { key: None, table })
    }
}

impl ser::SerializeMap for ToValueTableSerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.key = Some(key.serialize(RawStringSerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        #[allow(clippy::panic)]
        let Some(key) = self.key.take() else {
            panic!("ToValueTableSerializer::serialize_value called without calling ToValueTableSerializer::serialize_key first")
        };

        self.table.insert(key, value.serialize(ToValueSerializer)?);
        Ok(())
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.table.insert(
            key.serialize(RawStringSerializer)?,
            value.serialize(ToValueSerializer)?,
        );
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Table(self.table))
    }
}

impl ser::SerializeStruct for ToValueTableSerializer {
    type Ok = <Self as ser::SerializeMap>::Ok;
    type Error = <Self as ser::SerializeMap>::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

#[cfg(feature = "datetime")]
#[derive(Debug)]
pub enum ToValueTableOrDatetimeSerializer {
    AnyDatetime, // Used if we see AnyDatetime::WRAPPER_TYPE, use the *::WRAPPER_FIELD to determine which type to use
    OffsetDatetime(Option<Vec<u8>>),
    LocalDatetime(Option<Vec<u8>>),
    LocalDate(Option<Vec<u8>>),
    LocalTime(Option<Vec<u8>>),
    Table(ToValueTableSerializer),
}

#[cfg(feature = "datetime")]
impl ToValueTableOrDatetimeSerializer {
    fn start(len: Option<usize>, name: &'static str) -> Result<Self, Error> {
        Ok(match name {
            AnyDatetime::WRAPPER_TYPE => Self::AnyDatetime,
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(None),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(None),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(None),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(None),
            _ => Self::Table(ToValueTableSerializer::start(len)?),
        })
    }
}

#[cfg(feature = "datetime")]
impl ser::SerializeStruct for ToValueTableOrDatetimeSerializer {
    type Ok = <ToValueTableSerializer as ser::SerializeStruct>::Ok;
    type Error = <ToValueTableSerializer as ser::SerializeStruct>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match *self {
            // For AnyDatetime use the key to determine the type
            Self::AnyDatetime => match key {
                OffsetDatetime::WRAPPER_FIELD => {
                    *self = Self::OffsetDatetime(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalDatetime::WRAPPER_FIELD => {
                    *self = Self::LocalDatetime(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalDate::WRAPPER_FIELD => {
                    *self = Self::LocalDate(Some(value.serialize(RawBytesSerializer)?));
                }
                LocalTime::WRAPPER_FIELD => {
                    *self = Self::LocalTime(Some(value.serialize(RawBytesSerializer)?));
                }
                _ => return Err(ErrorKind::UnsupportedValue(key).into()),
            },
            Self::OffsetDatetime(ref mut inner @ None) if key == OffsetDatetime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalDatetime(ref mut inner @ None) if key == LocalDatetime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalDate(ref mut inner @ None) if key == LocalDate::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::LocalTime(ref mut inner @ None) if key == LocalTime::WRAPPER_FIELD => {
                *inner = Some(value.serialize(RawBytesSerializer)?);
            }
            Self::OffsetDatetime(Some(_))
            | Self::LocalDatetime(Some(_))
            | Self::LocalDate(Some(_))
            | Self::LocalTime(Some(_)) => {
                return Err(ErrorKind::UnsupportedValue(
                    "datetime wrapper with more than one member",
                )
                .into())
            }
            Self::OffsetDatetime(_)
            | Self::LocalDatetime(_)
            | Self::LocalDate(_)
            | Self::LocalTime(_) => return Err(ErrorKind::UnsupportedValue(key).into()),
            Self::Table(ref mut ser) => ser.serialize_field(key, value)?,
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Self::OffsetDatetime(Some(bytes)) => OffsetDatetime::from_slice(&bytes)
                .map(Into::into)
                .map(Value::Datetime)
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalDatetime(Some(bytes)) => LocalDatetime::from_slice(&bytes)
                .map(Into::into)
                .map(Value::Datetime)
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalDate(Some(bytes)) => LocalDate::from_slice(&bytes)
                .map(Into::into)
                .map(Value::Datetime)
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded datetime").into()),
            Self::LocalTime(Some(bytes)) => LocalTime::from_slice(&bytes)
                .map(Into::into)
                .map(Value::Datetime)
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
pub struct ToValueWrappedTableSerializer {
    key: String,
    table: ToValueTableSerializer,
}

impl ToValueWrappedTableSerializer {
    #[inline]
    fn start(len: usize, key: &'static str) -> Result<Self, Error> {
        Ok(Self {
            key: key.to_owned(),
            table: ToValueTableSerializer::start(Some(len))?,
        })
    }
}

impl ser::SerializeStructVariant for ToValueWrappedTableSerializer {
    type Ok = Value;
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        self.table.serialize_entry(key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeMap as _;

        Ok(Value::Table([(self.key, self.table.end()?)].into()))
    }
}

#[derive(Debug)]
struct RawStringSerializer;

impl ser::Serializer for RawStringSerializer {
    type Ok = String;
    type Error = Error;

    __serialize_unsupported!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_string())
    }
}

#[derive(Debug)]
struct RawBytesSerializer;

impl ser::Serializer for RawBytesSerializer {
    type Ok = Vec<u8>;
    type Error = Error;

    __serialize_unsupported!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_vec())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use maplit::btreemap;
    use serde::Serializer as _;
    #[cfg(feature = "datetime")]
    use serde_bytes::Bytes;
    use serde_test::{assert_ser_tokens, Token};

    use super::*;
    #[cfg(feature = "datetime")]
    use crate::value::Datetime;

    #[test]
    fn serialize_value() {
        let value = Value::String("Hello!".to_string());
        let tokens = [Token::Str("Hello!")];
        assert_ser_tokens(&value, &tokens);

        let value = Value::Integer(42);
        let tokens = [Token::I64(42)];
        assert_ser_tokens(&value, &tokens);

        let value = Value::Float(42.0);
        let tokens = [Token::F64(42.0)];
        assert_ser_tokens(&value, &tokens);

        let value = Value::Boolean(true);
        let tokens = [Token::Bool(true)];
        assert_ser_tokens(&value, &tokens);

        #[cfg(feature = "datetime")]
        {
            let value = Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME);
            let tokens = [
                Token::Struct {
                    name: AnyDatetime::WRAPPER_TYPE,
                    len: 1,
                },
                Token::Str(OffsetDatetime::WRAPPER_FIELD),
                Token::Bytes(OffsetDatetime::EXAMPLE_BYTES),
                Token::StructEnd,
            ];
            assert_ser_tokens(&value, &tokens);
        };

        let value = Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]);
        let tokens = [
            Token::Seq { len: Some(3) },
            Token::I64(1),
            Token::I64(2),
            Token::I64(3),
            Token::SeqEnd,
        ];
        assert_ser_tokens(&value, &tokens);

        let value = Value::Table(btreemap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        });
        let tokens = [
            Token::Map { len: Some(3) },
            Token::Str("one"),
            Token::I64(1),
            Token::Str("three"), // BTreeMap will alphabetise the keys
            Token::I64(3),
            Token::Str("two"),
            Token::I64(2),
            Token::MapEnd,
        ];
        assert_ser_tokens(&value, &tokens);
    }

    #[test]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn to_value_serializer() {
        let result = ToValueSerializer.serialize_bool(true).unwrap();
        assert_eq!(result, Value::Boolean(true));

        let result = ToValueSerializer.serialize_i8(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_i16(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_i32(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_i64(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_i128(42).unwrap();
        assert_eq!(result, Value::Integer(42));
        let result = ToValueSerializer.serialize_i128(i128::MIN);
        assert_matches!(result, Err(Error(ErrorKind::UnsupportedValue(..))));

        let result = ToValueSerializer.serialize_u8(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_u16(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_u32(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_u64(42).unwrap();
        assert_eq!(result, Value::Integer(42));
        let result = ToValueSerializer.serialize_u64(u64::MAX);
        assert_matches!(result, Err(Error(ErrorKind::UnsupportedValue(..))));

        let result = ToValueSerializer.serialize_u128(42).unwrap();
        assert_eq!(result, Value::Integer(42));
        let result = ToValueSerializer.serialize_u128(u128::MAX);
        assert_matches!(result, Err(Error(ErrorKind::UnsupportedValue(..))));

        let result = ToValueSerializer.serialize_f32(42.0).unwrap();
        assert_eq!(result, Value::Float(42.0));

        let result = ToValueSerializer.serialize_f64(42.0).unwrap();
        assert_eq!(result, Value::Float(42.0));

        let result = ToValueSerializer.serialize_char('a').unwrap();
        assert_eq!(result, Value::String("a".to_string()));

        let result = ToValueSerializer.serialize_str("Hello!").unwrap();
        assert_eq!(result, Value::String("Hello!".to_string()));

        let result = ToValueSerializer.serialize_bytes(&[1, 2, 3]).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );

        let result = ToValueSerializer.serialize_none();
        assert_matches!(result, Err(Error(ErrorKind::UnsupportedValue(..))));

        let result = ToValueSerializer.serialize_some(&42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_unit();
        assert_matches!(result, Err(Error(ErrorKind::UnsupportedType(..))));

        let result = ToValueSerializer.serialize_unit_struct("UnitStruct");
        assert_matches!(result, Err(Error(ErrorKind::UnsupportedType(..))));

        let result = ToValueSerializer
            .serialize_unit_variant("Enum", 0, "UnitVariant")
            .unwrap();
        assert_eq!(result, Value::String("UnitVariant".to_string()));

        let result = ToValueSerializer
            .serialize_newtype_struct("NewtypeStruct", &42)
            .unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer
            .serialize_newtype_variant("Enum", 0, "NewtypeVariant", &42)
            .unwrap();
        assert_eq!(
            result,
            Value::Table(btreemap! { "NewtypeVariant".to_string() => Value::Integer(42) })
        );

        // These create a type-specific serializer which is tested below, so just unwrap to test for panics
        assert_matches!(
            ToValueSerializer.serialize_seq(Some(3)),
            Ok(ToValueArraySerializer { .. })
        );
        assert_matches!(
            ToValueSerializer.serialize_tuple(3),
            Ok(ToValueArraySerializer { .. })
        );
        assert_matches!(
            ToValueSerializer.serialize_tuple_struct("TupleStruct", 3),
            Ok(ToValueArraySerializer { .. })
        );
        assert_matches!(
            ToValueSerializer.serialize_tuple_variant("Enum", 0, "TupleVariant", 3),
            Ok(ToValueWrappedArraySerializer { .. })
        );
        assert_matches!(
            ToValueSerializer.serialize_map(Some(3)),
            Ok(ToValueTableSerializer { .. })
        );
        #[cfg(feature = "datetime")]
        assert_matches!(
            ToValueSerializer.serialize_struct("Struct", 3),
            Ok(ToValueTableOrDatetimeSerializer::Table { .. })
        );
        #[cfg(not(feature = "datetime"))]
        assert_matches!(
            ToValueSerializer.serialize_struct("Struct", 3),
            Ok(ToValueTableSerializer { .. })
        );
        assert_matches!(
            ToValueSerializer.serialize_struct_variant("Enum", 0, "StructVariant", 3),
            Ok(ToValueWrappedTableSerializer { .. })
        );
    }

    #[test]
    fn to_value_array_serializer_seq() {
        use ser::SerializeSeq as _;

        let mut serializer = ToValueArraySerializer::start(Some(3)).unwrap();
        serializer.serialize_element(&1).unwrap();
        serializer.serialize_element(&2).unwrap();
        serializer.serialize_element(&3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
    }

    #[test]
    fn to_value_array_serializer_tuple() {
        use ser::SerializeTuple as _;

        let mut serializer = ToValueArraySerializer::start(Some(3)).unwrap();
        serializer.serialize_element(&1).unwrap();
        serializer.serialize_element(&2).unwrap();
        serializer.serialize_element(&3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
    }

    #[test]
    fn to_value_array_serializer_tuple_struct() {
        use ser::SerializeTupleStruct as _;

        let mut serializer = ToValueArraySerializer::start(Some(3)).unwrap();
        serializer.serialize_field(&1).unwrap();
        serializer.serialize_field(&2).unwrap();
        serializer.serialize_field(&3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3)
            ])
        );
    }

    #[test]
    fn to_value_wrapped_array_serializer() {
        use ser::SerializeTupleVariant as _;

        let mut serializer = ToValueWrappedArraySerializer::start(3, "TupleVariant").unwrap();
        serializer.serialize_field(&1).unwrap();
        serializer.serialize_field(&2).unwrap();
        serializer.serialize_field(&3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Table(
                btreemap! { "TupleVariant".to_string() => Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]) }
            )
        );
    }

    #[test]
    fn to_value_table_serializer_map() {
        use ser::SerializeMap as _;

        let mut serializer = ToValueTableSerializer::start(Some(3)).unwrap();
        serializer.serialize_key("one").unwrap();
        serializer.serialize_value(&1).unwrap();
        serializer.serialize_key("two").unwrap();
        serializer.serialize_value(&2).unwrap();
        serializer.serialize_key("three").unwrap();
        serializer.serialize_value(&3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Table(
                btreemap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }
            )
        );
    }

    #[test]
    #[should_panic = "ToValueTableSerializer::serialize_value called without calling ToValueTableSerializer::serialize_key first"]
    fn to_value_table_serializer_map_error() {
        use ser::SerializeMap as _;

        let mut serializer = ToValueTableSerializer::start(Some(3)).unwrap();
        serializer.serialize_value(&1).unwrap();
    }

    #[test]
    fn to_value_table_serializer_struct() {
        use ser::SerializeStruct as _;

        let mut serializer = ToValueTableSerializer::start(Some(3)).unwrap();
        serializer.serialize_field("one", &1).unwrap();
        serializer.serialize_field("two", &2).unwrap();
        serializer.serialize_field("three", &3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Table(
                btreemap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }
            )
        );
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn to_value_table_or_datetime_serializer() {
        use ser::SerializeStruct as _;

        let mut serializer = ToValueTableOrDatetimeSerializer::start(Some(3), "Struct").unwrap();
        serializer.serialize_field("one", &1).unwrap();
        serializer.serialize_field("two", &2).unwrap();
        serializer.serialize_field("three", &3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Table(
                btreemap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }
            )
        );

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), OffsetDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), LocalDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                Bytes::new(LocalDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), LocalDate::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                LocalDate::WRAPPER_FIELD,
                Bytes::new(LocalDate::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), LocalTime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                LocalTime::WRAPPER_FIELD,
                Bytes::new(LocalTime::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), AnyDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                Bytes::new(OffsetDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_OFFSET_DATETIME));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), AnyDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                LocalDatetime::WRAPPER_FIELD,
                Bytes::new(LocalDatetime::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_LOCAL_DATETIME));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), AnyDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                LocalDate::WRAPPER_FIELD,
                Bytes::new(LocalDate::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_LOCAL_DATE));

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), AnyDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                LocalTime::WRAPPER_FIELD,
                Bytes::new(LocalTime::EXAMPLE_BYTES),
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(result, Value::Datetime(Datetime::EXAMPLE_LOCAL_TIME));
    }

    #[cfg(feature = "datetime")]
    #[test]
    fn to_value_table_or_datetime_serializer_error() {
        use ser::SerializeStruct as _;

        let tests = [
            (
                OffsetDatetime::WRAPPER_TYPE,
                OffsetDatetime::WRAPPER_FIELD,
                OffsetDatetime::EXAMPLE_BYTES,
            ),
            (
                LocalDatetime::WRAPPER_TYPE,
                LocalDatetime::WRAPPER_FIELD,
                LocalDatetime::EXAMPLE_BYTES,
            ),
            (
                LocalDate::WRAPPER_TYPE,
                LocalDate::WRAPPER_FIELD,
                LocalDate::EXAMPLE_BYTES,
            ),
            (
                LocalTime::WRAPPER_TYPE,
                LocalTime::WRAPPER_FIELD,
                LocalTime::EXAMPLE_BYTES,
            ),
            (
                AnyDatetime::WRAPPER_TYPE,
                OffsetDatetime::WRAPPER_FIELD,
                OffsetDatetime::EXAMPLE_BYTES,
            ),
            (
                AnyDatetime::WRAPPER_TYPE,
                LocalDatetime::WRAPPER_FIELD,
                LocalDatetime::EXAMPLE_BYTES,
            ),
            (
                AnyDatetime::WRAPPER_TYPE,
                LocalDate::WRAPPER_FIELD,
                LocalDate::EXAMPLE_BYTES,
            ),
            (
                AnyDatetime::WRAPPER_TYPE,
                LocalTime::WRAPPER_FIELD,
                LocalTime::EXAMPLE_BYTES,
            ),
        ];

        for (name, field, bytes) in tests {
            let serializer = ToValueTableOrDatetimeSerializer::start(Some(0), name).unwrap();
            assert_matches!(
                serializer.end(),
                Err(Error(ErrorKind::UnsupportedValue(..)))
            );

            let mut serializer = ToValueTableOrDatetimeSerializer::start(Some(1), name).unwrap();
            assert_matches!(
                serializer.serialize_field("one", &1),
                Err(Error(ErrorKind::UnsupportedValue(..)))
            );

            let mut serializer = ToValueTableOrDatetimeSerializer::start(Some(2), name).unwrap();
            serializer
                .serialize_field(field, Bytes::new(bytes))
                .unwrap();
            assert_matches!(
                serializer.serialize_field(field, Bytes::new(bytes)),
                Err(Error(ErrorKind::UnsupportedValue(..)))
            );

            let mut serializer = ToValueTableOrDatetimeSerializer::start(Some(1), name).unwrap();
            serializer
                .serialize_field(field, Bytes::new(b"blah"))
                .unwrap();
            assert_matches!(
                serializer.end(),
                Err(Error(ErrorKind::UnsupportedValue(..)))
            );
        }
    }

    #[test]
    fn to_value_wrapped_table_serializer() {
        use ser::SerializeStructVariant as _;

        let mut serializer = ToValueWrappedTableSerializer::start(3, "StructVariant").unwrap();
        serializer.serialize_field("one", &1).unwrap();
        serializer.serialize_field("two", &2).unwrap();
        serializer.serialize_field("three", &3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Table(
                btreemap! { "StructVariant".to_string() => Value::Table(btreemap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }) }
            )
        );
    }

    #[test]
    fn raw_string_serializer() {
        let result = RawStringSerializer.serialize_str("Hello!").unwrap();
        assert_eq!(result, "Hello!".to_string());
    }
}
