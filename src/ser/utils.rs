use std::fmt;
use std::marker::PhantomData;
use std::result::Result as StdResult;

use super::{Error, Result};
use crate::ser::{writer, ErrorKind};

// Serializes something to a TOML key
pub struct KeySerializer<'a, W> {
    writer: &'a mut W,
}

impl<'a, W> KeySerializer<'a, W>
where
    W: fmt::Write,
{
    /// Creates a new `KeySerializer` with the given writer.
    #[inline]
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<W> ser::Serializer for KeySerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    __serialize_unsupported!(
        bool f32 f64 char bytes none unit unit_struct unit_variant newtype_variant
        seq tuple tuple_struct tuple_variant map struct struct_variant
    );

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        write!(self.writer, "{value}")?; // '0'-'9' & '-' are all valid identifiers
        Ok(())
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        writer::Formatter::write_key(value, self.writer)?;
        Ok(())
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }
}

// Serializes a string to itself
pub struct RawStringSerializer<'a, W> {
    pub writer: &'a mut W,
}

// impl<'a, W> RawStringSerializer<'a, W>
// where
//     W: fmt::Write,
// {
//     /// Creates a new `RawStringSerializer` with the given writer.
//     #[inline]
//     pub fn new(writer: &'a mut W) -> Self {
//         Self { writer }
//     }
// }

impl<W> ser::Serializer for RawStringSerializer<'_, W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    __serialize_unsupported!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        self.writer.write_str(value)?;
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        self.writer.write_str(
            std::str::from_utf8(value)
                .map_err(|_| ErrorKind::UnsupportedValue("invalid encoded bytes"))?,
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Impossible<O, E> {
    never: Never,
    _phantom: PhantomData<(O, E)>,
}

#[derive(Debug)]
enum Never {}

impl<O, E> ser::SerializeSeq for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_element<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeTuple for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_element<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeTupleStruct for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeTupleVariant for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeMap for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_key<T>(&mut self, _key: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn serialize_value<T>(&mut self, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeStruct for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

impl<O, E> ser::SerializeStructVariant for Impossible<O, E>
where
    E: ser::Error,
{
    type Ok = O;
    type Error = E;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> StdResult<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.never {}
    }

    fn end(self) -> StdResult<Self::Ok, Self::Error> {
        match self.never {}
    }
}

// Helper for unimplemented Serializer methods
// Adapted from: https://github.com/serde-rs/serde/blob/04ff3e8/serde/src/private/doc.rs#L47
#[doc(hidden)]
#[macro_export]
macro_rules! __serialize_unsupported {
    ($($func:ident)*) => {
        $(
            $crate::__serialize_unsupported_helper!($func);
        )*
    };
}
pub(crate) use __serialize_unsupported;
use serde::ser;

#[doc(hidden)]
#[macro_export]
#[allow(edition_2024_expr_fragment_specifier)]
macro_rules! __serialize_unsupported_method {
    ($func:ident $(<$t:ident>)* ($($arg:ty),*) -> $ret:ident, $msg:expr) => {
        #[inline]
        fn $func $(<$t>)* (self $(, _: $arg)*) -> $crate::ser::Result<Self::$ret>
        where
            $($t: ?Sized + ::serde::Serialize,)*
        {
            Err($crate::ser::ErrorKind::UnsupportedType($msg).into())
        }
    };

    ($func:ident $(<$t:ident>)* (name: $name:ty $(, $arg:ty)*) -> $ret:ident) => {
        #[inline]
        fn $func $(<$t>)* (self, name: $name $(, _: $arg)*) -> $crate::ser::Result<Self::$ret>
        where
            $($t: ?Sized + ::serde::Serialize,)*
        {
            Err($crate::ser::ErrorKind::UnsupportedType(name).into())
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __serialize_unsupported_helper {
    (bool) => {
        $crate::__serialize_unsupported_method!(serialize_bool(bool) -> Ok, "bool");
    };
    (i8) => {
        $crate::__serialize_unsupported_method!(serialize_i8(i8) -> Ok, "i8");
    };
    (i16) => {
        $crate::__serialize_unsupported_method!(serialize_i16(i16) -> Ok, "i16");
    };
    (i32) => {
        $crate::__serialize_unsupported_method!(serialize_i32(i32) -> Ok, "i32");
    };
    (i64) => {
        $crate::__serialize_unsupported_method!(serialize_i64(i64) -> Ok, "i64");
    };
    (i128) => {
        $crate::__serialize_unsupported_method!(serialize_i128(i128) -> Ok, "i128");
    };
    (u8) => {
        $crate::__serialize_unsupported_method!(serialize_u8(u8) -> Ok, "u8");
    };
    (u16) => {
        $crate::__serialize_unsupported_method!(serialize_u16(u16) -> Ok, "u16");
    };
    (u32) => {
        $crate::__serialize_unsupported_method!(serialize_u32(u32) -> Ok, "u32");
    };
    (u64) => {
        $crate::__serialize_unsupported_method!(serialize_u64(u64) -> Ok, "u64");
    };
    (u128) => {
        $crate::__serialize_unsupported_method!(serialize_u128(u128) -> Ok, "u128");
    };
    (f32) => {
        $crate::__serialize_unsupported_method!(serialize_f32(f32) -> Ok, "f32");
    };
    (f64) => {
        $crate::__serialize_unsupported_method!(serialize_f64(f64) -> Ok, "f64");
    };
    (char) => {
        $crate::__serialize_unsupported_method!(serialize_char(char) -> Ok, "char");
    };
    (str) => {
        $crate::__serialize_unsupported_method!(serialize_str(&str) -> Ok, "str");
    };
    (bytes) => {
        $crate::__serialize_unsupported_method!(serialize_bytes(&[u8]) -> Ok, "[u8]");
    };
    (none) => {
        $crate::__serialize_unsupported_method!(serialize_none() -> Ok, "Option");
    };
    (some) => {
        $crate::__serialize_unsupported_method!(serialize_some<T>(&T) -> Ok, "Option");
    };
    (unit) => {
        $crate::__serialize_unsupported_method!(serialize_unit() -> Ok, "()");
    };
    (unit_struct) => {
        $crate::__serialize_unsupported_method!(serialize_unit_struct(name: &'static str) -> Ok);
    };
    (unit_variant) => {
        $crate::__serialize_unsupported_method!(serialize_unit_variant(name: &'static str, u32, &str) -> Ok);
    };
    (newtype_struct) => {
        $crate::__serialize_unsupported_method!(serialize_newtype_struct<T>(name: &'static str, &T) -> Ok);
    };
    (newtype_variant) => {
        $crate::__serialize_unsupported_method!(serialize_newtype_variant<T>(name: &'static str, u32, &str, &T) -> Ok);
    };
    (seq) => {
        type SerializeSeq = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_seq(Option<usize>) -> SerializeSeq, "slice");
    };
    (tuple) => {
        type SerializeTuple = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_tuple(usize) -> SerializeTuple, "tuple");
    };
    (tuple_struct) => {
        type SerializeTupleStruct = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_tuple_struct(name: &'static str, usize) -> SerializeTupleStruct);
    };
    (tuple_variant) => {
        type SerializeTupleVariant = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_tuple_variant(name: &'static str, u32, &str, usize) -> SerializeTupleVariant);
    };
    (map) => {
        type SerializeMap = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_map(Option<usize>) -> SerializeMap, "map");
    };
    (struct) => {
        type SerializeStruct = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_struct(name: &'static str, usize) -> SerializeStruct);
    };
    (struct_variant) => {
        type SerializeStructVariant = $crate::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_struct_variant(name: &'static str, u32, &str, usize) -> SerializeStructVariant);
    };
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use serde::Serializer as _;

    use super::*;

    #[test]
    fn key_serializer() {
        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_str("foo").unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_str("abc.123").unwrap();
        assert_eq!(buf, r#""abc.123""#);

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_str("😎").unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_i8(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_i16(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_i32(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_i64(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_i128(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_u8(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_u16(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_u32(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_u64(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_u128(2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_some(&"foo").unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_some(&"abc.123").unwrap();
        assert_eq!(buf, r#""abc.123""#);

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_some("😎").unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_some(&2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_newtype_struct("Wrapper", &"foo").unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_newtype_struct("Wrapper", &"abc.123").unwrap();
        assert_eq!(buf, r#""abc.123""#);

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_newtype_struct("Wrapper", "😎").unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        ser.serialize_newtype_struct("Wrapper", &2).unwrap();
        assert_eq!(buf, "2");

        let mut buf = String::new();
        let ser = KeySerializer::new(&mut buf);
        assert_matches!(ser.serialize_seq(Some(2)), Err(Error(..)));
    }

    #[test]
    fn raw_string_serializer() {
        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        ser.serialize_str("foo").unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        ser.serialize_str("abc.123").unwrap();
        assert_eq!(buf, "abc.123");

        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        ser.serialize_str("😎").unwrap();
        assert_eq!(buf, "😎");

        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        ser.serialize_bytes(b"foo").unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        assert_matches!(ser.serialize_bytes(b"\xff"), Err(Error(..)));

        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        assert_matches!(ser.serialize_i32(2), Err(Error(..)));

        let mut buf = String::new();
        let ser = RawStringSerializer { writer: &mut buf };
        assert_matches!(ser.serialize_seq(Some(2)), Err(Error(..)));
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn unsupported() {
        struct Unsupported;

        impl ser::Serializer for Unsupported {
            type Ok = ();
            type Error = Error;

            __serialize_unsupported!(
                bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str bytes none
                some unit unit_struct unit_variant newtype_struct newtype_variant seq
                tuple tuple_struct tuple_variant map struct struct_variant
            );
        }

        assert_matches!(Unsupported.serialize_bool(true), Err(Error(..)));
        assert_matches!(Unsupported.serialize_i8(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_i16(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_i32(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_i64(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_i128(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_u8(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_u16(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_u32(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_u64(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_u128(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_f32(2.0), Err(Error(..)));
        assert_matches!(Unsupported.serialize_f64(2.0), Err(Error(..)));
        assert_matches!(Unsupported.serialize_char('a'), Err(Error(..)));
        assert_matches!(Unsupported.serialize_str("foo"), Err(Error(..)));
        assert_matches!(Unsupported.serialize_bytes(b"foo"), Err(Error(..)));
        assert_matches!(Unsupported.serialize_none(), Err(Error(..)));
        assert_matches!(Unsupported.serialize_some(&2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_unit(), Err(Error(..)));
        assert_matches!(Unsupported.serialize_unit_struct("foo"), Err(Error(..)));
        assert_matches!(
            Unsupported.serialize_unit_variant("foo", 2, "bar"),
            Err(Error(..))
        );
        assert_matches!(
            Unsupported.serialize_newtype_struct("foo", &2),
            Err(Error(..))
        );
        assert_matches!(
            Unsupported.serialize_newtype_variant("foo", 2, "bar", &2),
            Err(Error(..))
        );
        assert_matches!(Unsupported.serialize_seq(Some(2)), Err(Error(..)));
        assert_matches!(Unsupported.serialize_tuple(2), Err(Error(..)));
        assert_matches!(Unsupported.serialize_tuple_struct("foo", 2), Err(Error(..)));
        assert_matches!(
            Unsupported.serialize_tuple_variant("foo", 2, "bar", 2),
            Err(Error(..))
        );
        assert_matches!(Unsupported.serialize_map(Some(2)), Err(Error(..)));
        assert_matches!(Unsupported.serialize_struct("foo", 2), Err(Error(..)));
        assert_matches!(
            Unsupported.serialize_struct_variant("foo", 2, "bar", 2),
            Err(Error(..))
        );
    }
}
