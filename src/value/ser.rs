#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::str::FromStr;

use serde::ser;

use super::{Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime, Value};
use crate::__serialize_unimplemented;
use crate::ser::{Error, ErrorKind};

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Self::String(ref str) => str.serialize(serializer),
            Self::Integer(int) => int.serialize(serializer),
            Self::Float(float) => float.serialize(serializer),
            Self::Boolean(bool) => bool.serialize(serializer),
            Self::Datetime(ref datetime) => datetime.serialize(serializer),
            Self::Array(ref array) => array.serialize(serializer),
            #[cfg(test)]
            Self::Table(ref table) => table
                .iter()
                .collect::<BTreeMap<_, _>>()
                .serialize(serializer),
            #[cfg(not(test))]
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
    type SerializeStruct = ToValueTableOrDatetimeSerializer;
    type SerializeStructVariant = ToValueWrappedTableSerializer;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Boolean(value))
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Integer(value))
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(
            value
                .try_into()
                .map_err(|_| ErrorKind::UnsupportedValue("integer out of range of i64"))?,
        )
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(value.into())
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(
            value
                .try_into()
                .map_err(|_| ErrorKind::UnsupportedValue("integer out of range of i64"))?,
        )
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(
            value
                .try_into()
                .map_err(|_| ErrorKind::UnsupportedValue("integer out of range of i64"))?,
        )
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(value.into())
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Float(value))
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(value.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::String(value.into()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeSeq as _;

        let mut seq = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ErrorKind::UnsupportedValue("None").into())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ErrorKind::UnsupportedType("()").into())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

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

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Self::SerializeSeq::start(len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Self::SerializeTuple::start(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Self::SerializeTupleStruct::start(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Self::SerializeTupleVariant::start(len, variant)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Self::SerializeMap::start(len)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Self::SerializeStruct::start(Some(len), name)
    }

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

pub struct ToValueArraySerializer {
    array: Vec<Value>,
}

impl ToValueArraySerializer {
    #[allow(clippy::unnecessary_wraps)]
    fn start(len: Option<usize>) -> Result<Self, Error> {
        let array = Vec::with_capacity(len.unwrap_or(0).min(256));
        Ok(Self { array })
    }
}

impl ser::SerializeSeq for ToValueArraySerializer {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.array.push(value.serialize(ToValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Array(self.array))
    }
}

impl ser::SerializeTuple for ToValueArraySerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for ToValueArraySerializer {
    type Ok = <Self as ser::SerializeSeq>::Ok;
    type Error = <Self as ser::SerializeSeq>::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub struct ToValueWrappedArraySerializer {
    key: String,
    array: ToValueArraySerializer,
}

impl ToValueWrappedArraySerializer {
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

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeTuple as _;

        self.array.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeTuple as _;

        Ok(Value::Table([(self.key, self.array.end()?)].into()))
    }
}

pub struct ToValueTableSerializer {
    key: Option<String>,
    table: HashMap<String, Value>,
}

impl ToValueTableSerializer {
    #[allow(clippy::unnecessary_wraps)]
    fn start(len: Option<usize>) -> Result<Self, Error> {
        let table = HashMap::with_capacity(len.unwrap_or(0).min(256));
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

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Table(self.table))
    }
}

impl ser::SerializeStruct for ToValueTableSerializer {
    type Ok = <Self as ser::SerializeMap>::Ok;
    type Error = <Self as ser::SerializeMap>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

pub enum ToValueTableOrDatetimeSerializer {
    Datetime(Option<String>),
    OffsetDatetime(Option<String>),
    LocalDatetime(Option<String>),
    LocalDate(Option<String>),
    LocalTime(Option<String>),
    Table(ToValueTableSerializer),
}

impl ToValueTableOrDatetimeSerializer {
    fn start(len: Option<usize>, name: &'static str) -> Result<Self, Error> {
        Ok(match name {
            Datetime::WRAPPER_TYPE => Self::Datetime(None),
            OffsetDatetime::WRAPPER_TYPE => Self::OffsetDatetime(None),
            LocalDatetime::WRAPPER_TYPE => Self::LocalDatetime(None),
            LocalDate::WRAPPER_TYPE => Self::LocalDate(None),
            LocalTime::WRAPPER_TYPE => Self::LocalTime(None),
            _ => Self::Table(ToValueTableSerializer::start(len)?),
        })
    }
}

impl ser::SerializeStruct for ToValueTableOrDatetimeSerializer {
    type Ok = <ToValueTableSerializer as ser::SerializeStruct>::Ok;
    type Error = <ToValueTableSerializer as ser::SerializeStruct>::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match (self, key) {
            (&mut Self::Datetime(ref mut inner), Datetime::WRAPPER_FIELD)
            | (&mut Self::OffsetDatetime(ref mut inner), OffsetDatetime::WRAPPER_FIELD)
            | (&mut Self::LocalDatetime(ref mut inner), LocalDatetime::WRAPPER_FIELD)
            | (&mut Self::LocalDate(ref mut inner), LocalDate::WRAPPER_FIELD)
            | (&mut Self::LocalTime(ref mut inner), LocalTime::WRAPPER_FIELD) => match *inner {
                None => {
                    *inner = Some(value.serialize(RawStringSerializer)?);
                    Ok(())
                }
                Some(_) => Err(ErrorKind::UnsupportedValue(
                    "datetime wrapper with more than one member",
                )
                .into()),
            },
            (&mut Self::Table(ref mut ser), _) => ser.serialize_field(key, value),

            // If we don't have the right key for one of the date types
            _ => Err(ErrorKind::UnsupportedValue(key).into()),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Self::Datetime(inner)
            | Self::OffsetDatetime(inner)
            | Self::LocalDatetime(inner)
            | Self::LocalDate(inner)
            | Self::LocalTime(inner) => Ok(Value::Datetime(
                Datetime::from_str(
                    &inner.ok_or(ErrorKind::UnsupportedValue("empty date-time wrapper"))?,
                )
                .map_err(|_| ErrorKind::UnsupportedValue("invalid datetime value"))?,
            )),
            Self::Table(ser) => ser.end(),
        }
    }
}

pub struct ToValueWrappedTableSerializer {
    key: String,
    table: ToValueTableSerializer,
}

impl ToValueWrappedTableSerializer {
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

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        self.table.serialize_entry(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        use ser::SerializeMap as _;

        Ok(Value::Table([(self.key, self.table.end()?)].into()))
    }
}

struct RawStringSerializer;

impl ser::Serializer for RawStringSerializer {
    type Ok = String;
    type Error = Error;

    __serialize_unimplemented!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use maplit::hashmap;
    use serde::Serializer as _;

    use super::*;
    use crate::value::Offset;

    #[test]
    fn serialize_value() {
        let result = serde_json::to_string(&Value::String("Hello!".to_string())).unwrap();
        assert_eq!(result, r#""Hello!""#);

        let result = serde_json::to_string(&Value::Integer(42)).unwrap();
        assert_eq!(result, "42");

        let result = serde_json::to_string(&Value::Float(42.0)).unwrap();
        assert_eq!(result, "42.0");

        let result = serde_json::to_string(&Value::Boolean(true)).unwrap();
        assert_eq!(result, "true");

        let result = serde_json::to_string(&Value::Datetime(Datetime {
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
        }))
        .unwrap();
        assert_eq!(
            result,
            r#"{"<soml::_impl::Datetime::Wrapper::Field>":"2023-01-02T03:04:05.006+07:08"}"#
        );

        let result = serde_json::to_string(&Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3),
        ]))
        .unwrap();
        assert_eq!(result, "[1,2,3]");

        let result = serde_json::to_string(&Value::Table(hashmap! {
            "one".to_string() => Value::Integer(1),
            "two".to_string() => Value::Integer(2),
            "three".to_string() => Value::Integer(3),
        }))
        .unwrap();
        assert_eq!(result, r#"{"one":1,"three":3,"two":2}"#);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
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
        assert!(result.is_err());

        let result = ToValueSerializer.serialize_u8(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_u16(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_u32(42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_u64(42).unwrap();
        assert_eq!(result, Value::Integer(42));
        let result = ToValueSerializer.serialize_u64(u64::MAX);
        assert!(result.is_err());

        let result = ToValueSerializer.serialize_u128(42).unwrap();
        assert_eq!(result, Value::Integer(42));
        let result = ToValueSerializer.serialize_u128(u128::MAX);
        assert!(result.is_err());

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
        assert!(result.is_err());

        let result = ToValueSerializer.serialize_some(&42).unwrap();
        assert_eq!(result, Value::Integer(42));

        let result = ToValueSerializer.serialize_unit();
        assert!(result.is_err());

        let result = ToValueSerializer.serialize_unit_struct("UnitStruct");
        assert!(result.is_err());

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
            Value::Table(hashmap! { "NewtypeVariant".to_string() => Value::Integer(42) })
        );

        // These create a type-specific serializer which is tested below, so just unwrap to test for panics
        ToValueSerializer.serialize_seq(Some(3)).unwrap();
        ToValueSerializer.serialize_tuple(3).unwrap();
        ToValueSerializer
            .serialize_tuple_struct("TupleStruct", 3)
            .unwrap();
        ToValueSerializer
            .serialize_tuple_variant("Enum", 0, "TupleVariant", 3)
            .unwrap();
        ToValueSerializer.serialize_map(Some(3)).unwrap();
        ToValueSerializer.serialize_struct("Struct", 3).unwrap();
        ToValueSerializer
            .serialize_struct_variant("Enum", 0, "StructVariant", 3)
            .unwrap();
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
                hashmap! { "TupleVariant".to_string() => Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]) }
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
                hashmap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }
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
                hashmap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }
            )
        );
    }

    #[test]
    fn to_value_table_or_datetime_serializer() {
        use ser::SerializeStruct as _;

        let date = || LocalDate {
            year: 2023,
            month: 1,
            day: 2,
        };
        let time = || LocalTime {
            hour: 3,
            minute: 4,
            second: 5,
            nanosecond: 6_000_000,
        };
        let offset = || Offset::Custom { minutes: 428 };

        let mut serializer = ToValueTableOrDatetimeSerializer::start(Some(3), "Struct").unwrap();
        serializer.serialize_field("one", &1).unwrap();
        serializer.serialize_field("two", &2).unwrap();
        serializer.serialize_field("three", &3).unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Table(
                hashmap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }
            )
        );

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), Datetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(Datetime::WRAPPER_FIELD, &"2023-01-02T03:04:05.006+07:08")
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Datetime(Datetime {
                date: Some(date()),
                time: Some(time()),
                offset: Some(offset()),
            })
        );

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), OffsetDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(
                OffsetDatetime::WRAPPER_FIELD,
                &"2023-01-02T03:04:05.006+07:08",
            )
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Datetime(Datetime {
                date: Some(date()),
                time: Some(time()),
                offset: Some(offset()),
            })
        );

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), LocalDatetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(LocalDatetime::WRAPPER_FIELD, &"2023-01-02T03:04:05.006")
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Datetime(Datetime {
                date: Some(date()),
                time: Some(time()),
                offset: None,
            })
        );

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), LocalDate::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(LocalDate::WRAPPER_FIELD, &"2023-01-02")
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Datetime(Datetime {
                date: Some(date()),
                time: None,
                offset: None,
            })
        );

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), LocalTime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(LocalTime::WRAPPER_FIELD, &"03:04:05.006")
            .unwrap();
        let result = serializer.end().unwrap();
        assert_eq!(
            result,
            Value::Datetime(Datetime {
                date: None,
                time: Some(time()),
                offset: None,
            })
        );
    }

    #[test]
    fn to_value_table_or_datetime_serializer_error() {
        use ser::SerializeStruct as _;

        let serializer =
            ToValueTableOrDatetimeSerializer::start(Some(0), Datetime::WRAPPER_TYPE).unwrap();
        assert!(serializer.end().is_err());

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), Datetime::WRAPPER_TYPE).unwrap();
        assert!(serializer.serialize_field("one", &1).is_err());

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(2), Datetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(Datetime::WRAPPER_FIELD, &"2023-01-02T03:04:05.006+07:08")
            .unwrap();
        assert!(serializer
            .serialize_field(Datetime::WRAPPER_FIELD, &"2023-01-02T03:04:05.006+07:08")
            .is_err());

        let mut serializer =
            ToValueTableOrDatetimeSerializer::start(Some(1), Datetime::WRAPPER_TYPE).unwrap();
        serializer
            .serialize_field(Datetime::WRAPPER_FIELD, &"blah")
            .unwrap();
        assert!(serializer.end().is_err());
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
                hashmap! { "StructVariant".to_string() => Value::Table(hashmap! { "one".to_string() => Value::Integer(1), "two".to_string() => Value::Integer(2), "three".to_string() => Value::Integer(3) }) }
            )
        );
    }

    #[test]
    fn raw_string_serializer() {
        let result = RawStringSerializer.serialize_str("Hello!").unwrap();
        assert_eq!(result, "Hello!".to_string());
    }
}
