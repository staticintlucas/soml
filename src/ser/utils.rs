use std::fmt;

use super::{Error, Result};
use crate::ser::{writer, ErrorKind};

// Serializes something to a TOML key
pub struct KeySerializer<'a, W> {
    pub writer: &'a mut W,
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
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes none
        some unit unit_struct unit_variant newtype_struct newtype_variant seq
        tuple tuple_struct tuple_variant map struct struct_variant
    );

    #[inline]
    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        writer::Formatter::write_key(value, self.writer)?;
        Ok(())
    }
}

// Serializes a string to itself
pub struct RawStringSerializer<'a, W> {
    writer: &'a mut W,
}

impl<'a, W> RawStringSerializer<'a, W>
where
    W: fmt::Write,
{
    /// Creates a new `RawStringSerializer` with the given writer.
    #[inline]
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

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
        type SerializeSeq = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_seq(Option<usize>) -> SerializeSeq, "slice");
    };
    (tuple) => {
        type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_tuple(usize) -> SerializeTuple, "tuple");
    };
    (tuple_struct) => {
        type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_tuple_struct(name: &'static str, usize) -> SerializeTupleStruct);
    };
    (tuple_variant) => {
        type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_tuple_variant(name: &'static str, u32, &str, usize) -> SerializeTupleVariant);
    };
    (map) => {
        type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_map(Option<usize>) -> SerializeMap, "map");
    };
    (struct) => {
        type SerializeStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_struct(name: &'static str, usize) -> SerializeStruct);
    };
    (struct_variant) => {
        type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
        $crate::__serialize_unsupported_method!(serialize_struct_variant(name: &'static str, u32, &str, usize) -> SerializeStructVariant);
    };
}
