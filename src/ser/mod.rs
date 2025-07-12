//! TOML serialization functions and trait implementations.

use std::{fmt, io};

use serde::ser;

pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, Result};
pub(crate) use self::utils::Impossible;
pub use self::value::Serializer as ValueSerializer;
use self::writer::{Formatter, IoWriter};
use crate::value::{AnyDatetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

mod error;
mod tree;
mod utils;
mod value;
mod writer;

/// Serializes a value to a TOML string.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a TOML document.
#[inline]
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ser::Serialize,
{
    let mut dst = String::new();
    value.serialize(Serializer::new(&mut dst))?;
    Ok(dst)
}

/// Serializes a value to an [`io::Write`].
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a TOML document.
#[inline]
pub fn to_io_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ser::Serialize,
{
    value.serialize(Serializer::from_io_writer(writer))
}

/// Serializes a value to a [`fmt::Write`].
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a TOML document.
#[inline]
pub fn to_fmt_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: fmt::Write,
    T: ser::Serialize,
{
    value.serialize(Serializer::from_fmt_writer(writer))
}

/// A serializer for a TOML document.
#[derive(Debug)]
pub struct Serializer<W> {
    writer: W,
}

impl<'a> Serializer<&'a mut String> {
    /// Create a new TOML serializer that serializes to the given buffer.
    #[must_use]
    #[inline]
    pub fn new(buf: &'a mut String) -> Self {
        Self::from_fmt_writer(buf)
    }
}

impl<W> Serializer<W>
where
    W: fmt::Write,
{
    /// Create a new TOML serializer that serializes to the given writer.
    #[inline]
    pub fn from_fmt_writer(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> Serializer<IoWriter<W>>
where
    W: io::Write,
{
    /// Create a new TOML serializer that serializes to the given writer.
    #[inline]
    pub fn from_io_writer(writer: W) -> Self {
        Self {
            writer: IoWriter::new(writer),
        }
    }
}

impl<W> ser::Serializer for Serializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeTupleVariant = WrappedArraySerializer<W>;
    type SerializeMap = TableSerializer<W>;
    type SerializeStruct = TableSerializer<W>;
    type SerializeStructVariant = WrappedTableSerializer<W>;

    utils::__serialize_unsupported!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str bytes none
        some unit unit_struct unit_variant newtype_struct seq tuple tuple_struct
    );

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(Self::SerializeTupleVariant::start(
            self.writer,
            variant,
            len,
        ))
    }

    #[inline]
    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        Ok(Self::SerializeMap::start(self.writer, len))
    }

    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        match name {
            AnyDatetime::WRAPPER_TYPE
            | OffsetDatetime::WRAPPER_TYPE
            | LocalDatetime::WRAPPER_TYPE
            | LocalDate::WRAPPER_TYPE
            | LocalTime::WRAPPER_TYPE => Err(ErrorKind::UnsupportedType(name).into()),
            _ => Ok(Self::SerializeStruct::start(self.writer, Some(len))),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        Ok(Self::SerializeStructVariant::start(
            self.writer,
            variant,
            len,
        ))
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct WrappedArraySerializer<W> {
    writer: W,
    key: &'static str,
    arr: tree::ArraySerializer,
}

impl<W> WrappedArraySerializer<W> {
    #[inline]
    fn start(writer: W, key: &'static str, len: usize) -> Self {
        Self {
            writer,
            key,
            arr: tree::ArraySerializer::start(Some(len)),
        }
    }
}

impl<W> ser::SerializeTupleVariant for WrappedArraySerializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
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
    fn end(mut self) -> Result<Self::Ok> {
        match self.arr.end_inner()? {
            tree::Array::Inline(value) => {
                Formatter::write_inline(self.key, &value, &mut self.writer)?;
            }
            tree::Array::Table(array) => {
                Formatter::write_array_of_tables(
                    &array,
                    &[&self.key.to_string()],
                    &mut self.writer,
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct TableSerializer<W> {
    writer: W,
    table: tree::TableSerializer,
}

impl<W> TableSerializer<W> {
    #[inline]
    fn start(writer: W, len: Option<usize>) -> Self {
        Self {
            writer,
            table: tree::TableSerializer::start(len),
        }
    }
}

impl<W> ser::SerializeMap for TableSerializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.table.serialize_key(key)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.table.serialize_value(value)
    }

    #[inline]
    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        self.table.serialize_entry(key, value)
    }

    #[inline]
    fn end(mut self) -> Result<Self::Ok> {
        Formatter::write_table(&self.table.end_inner(), &[], &mut self.writer)?;
        Ok(())
    }
}

impl<W> ser::SerializeStruct for TableSerializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
    type Error = Error;

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
#[doc(hidden)]
pub struct WrappedTableSerializer<W> {
    writer: W,
    key: &'static str,
    table: tree::TableSerializer,
}

impl<W> WrappedTableSerializer<W> {
    #[inline]
    fn start(writer: W, key: &'static str, len: usize) -> Self {
        Self {
            writer,
            key,
            table: tree::TableSerializer::start(Some(len)),
        }
    }
}

impl<W> ser::SerializeStructVariant for WrappedTableSerializer<W>
where
    W: fmt::Write,
{
    type Ok = ();
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
    fn end(mut self) -> Result<Self::Ok> {
        Formatter::write_table(
            &self.table.end_inner(),
            &[&self.key.to_owned()],
            &mut self.writer,
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use maplit::btreemap;
    use serde::Serializer as _;

    use super::*;
    use crate::value::Offset;

    mod example {
        use std::collections::BTreeMap;

        use crate::value::OffsetDatetime;

        #[derive(Debug, PartialEq, Eq, serde::Serialize)]
        pub struct Struct {
            pub title: String,
            pub owner: Owner,
            pub database: Database,
            pub servers: BTreeMap<String, Server>,
            pub clients: Clients,
        }

        #[derive(Debug, PartialEq, Eq, serde::Serialize)]
        pub struct Owner {
            pub name: String,
            pub dob: OffsetDatetime,
        }

        #[derive(Debug, PartialEq, Eq, serde::Serialize)]
        pub struct Database {
            pub server: String,
            pub ports: Vec<u16>,
            pub connection_max: usize,
            pub enabled: bool,
        }

        #[derive(Debug, PartialEq, Eq, serde::Serialize)]
        pub struct Server {
            pub ip: String,
            pub dc: String,
        }

        #[derive(Debug, PartialEq, Eq, serde::Serialize)]
        pub struct Clients {
            pub hosts: Vec<String>,
            pub data: BTreeMap<String, usize>,
        }
    }

    #[test]
    fn ser_to_string() {
        let result = to_string(&example::Struct {
            title: "TOML Example".into(),
            owner: example::Owner {
                name: "Tom Preston-Werner".into(),
                dob: OffsetDatetime {
                    date: LocalDate {
                        year: 1979,
                        month: 5,
                        day: 27,
                    },
                    time: LocalTime {
                        hour: 7,
                        minute: 32,
                        second: 0,
                        nanosecond: 0,
                    },
                    offset: Offset::Custom { minutes: -480 },
                },
            },
            database: example::Database {
                server: "192.168.1.1".into(),
                ports: vec![8000, 8001, 8002],
                connection_max: 5000,
                enabled: true,
            },
            servers: btreemap! {
                "alpha".into() => example::Server {
                    ip: "10.0.0.1".into(),
                    dc: "eqdc10".into(),
                },
                "beta".into() => example::Server {
                    ip: "10.0.0.2".into(),
                    dc: "eqdc10".into(),
                },
            },
            clients: example::Clients {
                hosts: vec!["alpha".into(), "omega".into()],
                data: btreemap! {
                    "gamma".into() => 1,
                    "delta".into() => 2,
                },
            },
        })
        .unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                title = "TOML Example"

                [owner]
                name = "Tom Preston-Werner"
                dob = 1979-05-27T07:32:00-08:00

                [database]
                server = "192.168.1.1"
                ports = [8000, 8001, 8002]
                connection_max = 5000
                enabled = true

                [servers.alpha]
                ip = "10.0.0.1"
                dc = "eqdc10"

                [servers.beta]
                ip = "10.0.0.2"
                dc = "eqdc10"

                [clients]
                hosts = ["alpha", "omega"]

                [clients.data]
                delta = 2
                gamma = 1
            "#}
        );
    }

    #[test]
    fn ser_to_io_writer() {
        let mut result = Vec::new();
        to_io_writer(
            &mut result,
            &example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
                    dob: OffsetDatetime {
                        date: LocalDate {
                            year: 1979,
                            month: 5,
                            day: 27,
                        },
                        time: LocalTime {
                            hour: 7,
                            minute: 32,
                            second: 0,
                            nanosecond: 0,
                        },
                        offset: Offset::Custom { minutes: -480 },
                    },
                },
                database: example::Database {
                    server: "192.168.1.1".into(),
                    ports: vec![8000, 8001, 8002],
                    connection_max: 5000,
                    enabled: true,
                },
                servers: btreemap! {
                    "alpha".into() => example::Server {
                        ip: "10.0.0.1".into(),
                        dc: "eqdc10".into(),
                    },
                    "beta".into() => example::Server {
                        ip: "10.0.0.2".into(),
                        dc: "eqdc10".into(),
                    },
                },
                clients: example::Clients {
                    hosts: vec!["alpha".into(), "omega".into()],
                    data: btreemap! {
                        "gamma".into() => 1,
                        "delta".into() => 2,
                    },
                },
            },
        )
        .unwrap();

        assert_eq!(
            result,
            indoc! {br#"
                title = "TOML Example"

                [owner]
                name = "Tom Preston-Werner"
                dob = 1979-05-27T07:32:00-08:00

                [database]
                server = "192.168.1.1"
                ports = [8000, 8001, 8002]
                connection_max = 5000
                enabled = true

                [servers.alpha]
                ip = "10.0.0.1"
                dc = "eqdc10"

                [servers.beta]
                ip = "10.0.0.2"
                dc = "eqdc10"

                [clients]
                hosts = ["alpha", "omega"]

                [clients.data]
                delta = 2
                gamma = 1
            "#}
        );
    }

    #[test]
    fn ser_to_fmt_writer() {
        let mut result = String::new();
        to_fmt_writer(
            &mut result,
            &example::Struct {
                title: "TOML Example".into(),
                owner: example::Owner {
                    name: "Tom Preston-Werner".into(),
                    dob: OffsetDatetime {
                        date: LocalDate {
                            year: 1979,
                            month: 5,
                            day: 27,
                        },
                        time: LocalTime {
                            hour: 7,
                            minute: 32,
                            second: 0,
                            nanosecond: 0,
                        },
                        offset: Offset::Custom { minutes: -480 },
                    },
                },
                database: example::Database {
                    server: "192.168.1.1".into(),
                    ports: vec![8000, 8001, 8002],
                    connection_max: 5000,
                    enabled: true,
                },
                servers: btreemap! {
                    "alpha".into() => example::Server {
                        ip: "10.0.0.1".into(),
                        dc: "eqdc10".into(),
                    },
                    "beta".into() => example::Server {
                        ip: "10.0.0.2".into(),
                        dc: "eqdc10".into(),
                    },
                },
                clients: example::Clients {
                    hosts: vec!["alpha".into(), "omega".into()],
                    data: btreemap! {
                        "gamma".into() => 1,
                        "delta".into() => 2,
                    },
                },
            },
        )
        .unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                title = "TOML Example"

                [owner]
                name = "Tom Preston-Werner"
                dob = 1979-05-27T07:32:00-08:00

                [database]
                server = "192.168.1.1"
                ports = [8000, 8001, 8002]
                connection_max = 5000
                enabled = true

                [servers.alpha]
                ip = "10.0.0.1"
                dc = "eqdc10"

                [servers.beta]
                ip = "10.0.0.2"
                dc = "eqdc10"

                [clients]
                hosts = ["alpha", "omega"]

                [clients.data]
                delta = 2
                gamma = 1
            "#}
        );
    }

    #[test]
    fn serializer_new() {
        let mut buf = String::new();
        let serializer = Serializer::new(&mut buf);
        assert_eq!(serializer.writer, "");
    }

    #[test]
    fn serializer_from_io_writer() {
        let mut buf = Vec::new();
        let serializer = Serializer::from_io_writer(&mut buf);
        assert_eq!(serializer.writer.writer, b"");
    }

    #[test]
    fn serializer_from_fmt_writer() {
        let mut buf = String::new();
        let serializer = Serializer::from_fmt_writer(&mut buf);
        assert_eq!(serializer.writer, "");
    }

    #[test]
    fn serializer_serialize_newtype_variant() {
        let mut buf = String::new();
        let serializer = Serializer { writer: &mut buf };

        serializer
            .serialize_newtype_variant("name", 0, "foo", &42)
            .unwrap();

        assert_eq!(
            buf,
            indoc! {r"
                foo = 42
            "}
        );
    }

    #[test]
    fn serializer_serialize_tuple_variant() {
        let mut buf = String::new();
        let serializer = Serializer { writer: &mut buf };

        let seq = serializer
            .serialize_tuple_variant("name", 0, "foo", 2)
            .unwrap();

        assert_matches!(seq, WrappedArraySerializer {
            writer: _,
            key: "foo",
            arr: tree::ArraySerializer { arr },
        } if arr.capacity() == 2);
    }

    #[test]
    fn serializer_serialize_map() {
        let mut buf = String::new();
        let serializer = Serializer { writer: &mut buf };

        let seq = serializer.serialize_map(Some(2)).unwrap();

        assert_matches!(seq, TableSerializer {
            writer: _,
            table: tree::TableSerializer { table, .. }
        } if table.capacity() == 2);
    }

    #[test]
    fn serializer_serialize_struct() {
        let mut buf = String::new();
        let serializer = Serializer { writer: &mut buf };

        let seq = serializer.serialize_struct("name", 2).unwrap();

        assert_matches!(seq, TableSerializer {
            writer: _,
            table: tree::TableSerializer { table, .. }
        } if table.capacity() == 2);

        let mut buf = String::new();
        let serializer = Serializer { writer: &mut buf };

        let seq = serializer.serialize_struct(OffsetDatetime::WRAPPER_TYPE, 1);
        assert_matches!(seq, Err(Error(ErrorKind::UnsupportedType(..))));
    }

    #[test]
    fn serializer_serialize_struct_variant() {
        let mut buf = String::new();
        let serializer = Serializer { writer: &mut buf };

        let seq = serializer
            .serialize_struct_variant("name", 0, "foo", 2)
            .unwrap();

        assert_matches!(seq, WrappedTableSerializer {
            writer: _,
            key: "foo",
            table: tree::TableSerializer { table, .. },
        } if table.capacity() == 2);
    }

    #[test]
    fn wrapped_array_serializer() {
        use ser::SerializeTupleVariant as _;

        let mut buf = String::new();
        let mut array = WrappedArraySerializer::start(&mut buf, "foo", 2);
        assert_eq!(array.key, "foo");
        assert!(array.arr.arr.is_empty());
        assert_eq!(array.arr.arr.capacity(), 2);

        array.serialize_field(&42).unwrap();
        assert_eq!(array.arr.arr.len(), 1);

        array.serialize_field("bar").unwrap();
        assert_eq!(array.arr.arr.len(), 2);

        array.end().unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                foo = [42, "bar"]
            "#}
        );

        let mut buf = String::new();
        let mut array = WrappedArraySerializer::start(&mut buf, "foo", 2);
        assert_eq!(array.key, "foo");
        assert!(array.arr.arr.is_empty());
        assert_eq!(array.arr.arr.capacity(), 2);

        array.serialize_field(&btreemap! {"bar" => 42}).unwrap();
        assert_eq!(array.arr.arr.len(), 1);

        array.serialize_field(&btreemap! {"baz" => "qux"}).unwrap();
        assert_eq!(array.arr.arr.len(), 2);

        array.end().unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                [[foo]]
                bar = 42

                [[foo]]
                baz = "qux"
            "#}
        );
    }

    #[test]
    fn table_serializer_map() {
        use ser::SerializeMap as _;

        let mut buf = String::new();
        let mut table = TableSerializer::start(&mut buf, None);
        assert!(table.table.table.is_empty());
        assert_eq!(table.table.table.capacity(), 0);

        table.serialize_key("foo").unwrap();
        assert!(table.table.table.is_empty());

        table.serialize_value(&42).unwrap();
        assert_eq!(table.table.table.len(), 1);

        table.serialize_entry("bar", &"baz").unwrap();
        assert_eq!(table.table.table.len(), 2);

        table.end().unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                foo = 42
                bar = "baz"
            "#}
        );
    }

    #[test]
    fn table_serializer_struct() {
        use ser::SerializeStruct as _;

        let mut buf = String::new();
        let mut table = TableSerializer::start(&mut buf, Some(2));
        assert!(table.table.table.is_empty());
        assert_eq!(table.table.table.capacity(), 2);

        table.serialize_field("foo", &42).unwrap();
        assert_eq!(table.table.table.len(), 1);

        table.serialize_field("bar", &"baz").unwrap();
        assert_eq!(table.table.table.len(), 2);

        table.end().unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                foo = 42
                bar = "baz"
            "#}
        );
    }

    #[test]
    fn wrapped_table_serializer() {
        use ser::SerializeStructVariant as _;

        let mut buf = String::new();
        let mut table = WrappedTableSerializer::start(&mut buf, "foo", 2);
        assert_eq!(table.key, "foo");
        assert!(table.table.table.is_empty());
        assert_eq!(table.table.table.capacity(), 2);

        table.serialize_field("bar", &42).unwrap();
        assert_eq!(table.table.table.len(), 1);

        table.serialize_field("baz", &"qux").unwrap();
        assert_eq!(table.table.table.len(), 2);

        table.end().unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                [foo]
                bar = 42
                baz = "qux"
            "#}
        );
    }
}
