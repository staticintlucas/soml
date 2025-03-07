use std::{fmt, io};

use lexical::{NumberFormatBuilder, WriteFloatOptions, WriteIntegerOptions};
use serde::ser;

use self::_impl::{
    Float, InlineArraySerializer, InlineTableOrDatetimeSerializer, InlineTableSerializer,
    InlineWrappedArraySerializer, InlineWrappedTableSerializer, Integer, TableKind,
    TableSerializer, ValueKind, WrappedArraySerializer, WrappedTableSerializer,
    __serialize_unimplemented,
};
pub(crate) use self::error::ErrorKind;
pub use self::error::{Error, Result};
use self::writer::{IoWriter, Writer};

mod _impl;
mod error;
mod writer;

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ser::Serialize,
{
    let mut dst = String::new();
    value.serialize(Serializer::new(&mut dst))?;
    Ok(dst)
}

#[derive(Debug)]
pub struct Serializer<W> {
    writer: W,
}

impl<'a> Serializer<&'a mut String> {
    #[must_use]
    pub fn new(buf: &'a mut String) -> Self {
        Self::from_fmt_writer(buf)
    }
}

impl<T> Serializer<T>
where
    T: fmt::Write,
{
    pub fn from_fmt_writer(writer: T) -> Self {
        Self { writer }
    }
}

impl<T> Serializer<IoWriter<T>>
where
    T: io::Write,
{
    pub fn from_io_writer(writer: T) -> Self {
        Self {
            writer: IoWriter::new(writer),
        }
    }
}

impl<W> Serializer<W>
where
    W: Writer,
{
    #[expect(clippy::type_complexity)] // It's not that complex
    fn split_inlines_and_subtables(
        table: Vec<(String, ValueKind)>,
    ) -> (Vec<(String, String)>, Vec<(String, TableKind)>) {
        table.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut inlines, mut subtables), (k, v)| {
                match v {
                    ValueKind::InlineValue(value) => inlines.push((k, value)),
                    ValueKind::Table(table) => subtables.push((k, table)),
                }
                (inlines, subtables)
            },
        )
    }

    fn write(mut self, table: Vec<(String, ValueKind)>) -> Result<()> {
        let (inlines, subtables) = Self::split_inlines_and_subtables(table);
        let need_nl = !inlines.is_empty();

        let path = "";
        self.write_table_inlines(inlines)?;
        self.write_table_subtables(subtables, path, need_nl)?;

        Ok(())
    }

    fn write_table(&mut self, table: Vec<(String, ValueKind)>, path: &str) -> Result<()> {
        let (inlines, subtables) = Self::split_inlines_and_subtables(table);
        let need_nl = !inlines.is_empty() && !subtables.is_empty();

        // The table header is only needed if the table has inlines (key/value pairs); but if there
        // are no inlines and subtables then a reader would have no idea about the existence of the
        // table, so we also write the header in that case.
        if !inlines.is_empty() || subtables.is_empty() {
            writeln!(self.writer, "[{path}]")?;
        }

        self.write_table_inlines(inlines)?;
        self.write_table_subtables(subtables, path, need_nl)?;

        Ok(())
    }

    fn write_array_of_tables(
        &mut self,
        array: Vec<Vec<(String, ValueKind)>>,
        path: &str,
    ) -> Result<()> {
        for table in array {
            let (inlines, subtables) = Self::split_inlines_and_subtables(table);
            let need_nl = !inlines.is_empty() && !subtables.is_empty();

            // Always write the array header
            writeln!(self.writer, "[[{path}]]")?;

            self.write_table_inlines(inlines)?;
            self.write_table_subtables(subtables, path, need_nl)?;
        }

        Ok(())
    }

    fn write_table_inlines(&mut self, inlines: Vec<(String, String)>) -> Result<()> {
        // Sort alphabetically for deterministic test output
        #[cfg(test)]
        let inlines = {
            let mut temp = inlines;
            #[allow(clippy::pattern_type_mismatch)]
            temp.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            temp
        };

        for (key, value) in inlines {
            writeln!(self.writer, "{key} = {value}")?;
        }

        Ok(())
    }

    fn write_subtable(&mut self, key: String, table: TableKind, path: &str) -> Result<()> {
        let path = if path.is_empty() {
            key
        } else {
            format!("{path}.{key}")
        };

        match table {
            TableKind::Array(array) => self.write_array_of_tables(array, &path),
            TableKind::Table(table) => self.write_table(table, &path),
        }
    }

    fn write_table_subtables(
        &mut self,
        subtables: Vec<(String, TableKind)>,
        path: &str,
        need_nl: bool,
    ) -> Result<()> {
        // Sort alphabetically for deterministic test output
        #[cfg(test)]
        let subtables = {
            let mut temp = subtables;
            #[allow(clippy::pattern_type_mismatch)]
            temp.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            temp
        };
        let mut subtables = subtables.into_iter();

        let Some((key, table)) = subtables.next() else {
            return Ok(());
        };
        if need_nl {
            writeln!(self.writer)?;
        }
        self.write_subtable(key, table, path)?;

        for (key, table) in subtables {
            writeln!(self.writer)?;
            self.write_subtable(key, table, path)?;
        }

        Ok(())
    }
}

impl<W> ser::Serializer for Serializer<W>
where
    W: Writer,
{
    type Ok = ();
    type Error = Error;

    type SerializeTupleVariant = WrappedArraySerializer<W>;
    type SerializeMap = TableSerializer<W>;
    type SerializeStruct = TableSerializer<W>;
    type SerializeStructVariant = WrappedTableSerializer<W>;

    __serialize_unimplemented!(
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str bytes none
        some unit unit_struct unit_variant newtype_struct seq tuple tuple_struct
    );

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        use ser::SerializeMap as _;

        let mut map = self.serialize_map(Some(1))?;
        map.serialize_entry(variant, value)?;
        map.end()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Self::SerializeTupleVariant::start(self, len, variant)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Self::SerializeMap::start(self, len)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Self::SerializeStruct::start(self, Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Self::SerializeStructVariant::start(self, len, variant)
    }
}

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct InlineSerializer;

impl ser::Serializer for InlineSerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = InlineArraySerializer<Self>;
    type SerializeTuple = InlineArraySerializer<Self>;
    type SerializeTupleStruct = InlineArraySerializer<Self>;
    type SerializeTupleVariant = InlineWrappedArraySerializer<Self>;
    type SerializeMap = InlineTableSerializer<Self>;
    type SerializeStruct = InlineTableOrDatetimeSerializer<Self>;
    type SerializeStructVariant = InlineWrappedTableSerializer<Self>;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok> {
        Ok(if value { "true" } else { "false" }.to_owned())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok> {
        self.serialize_integer(value)
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok> {
        self.serialize_float(value)
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok> {
        self.serialize_str(value.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok> {
        // TODO also test where literal strings might be better?
        if value.contains('\n') {
            self.serialize_multiline_basic_str(value)
        } else {
            self.serialize_basic_str(value)
        }
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok> {
        use ser::SerializeSeq as _;

        let mut seq = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Err(ErrorKind::UnsupportedValue("None").into())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(ErrorKind::UnsupportedType("()").into())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

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

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Self::SerializeSeq::start(len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        Self::SerializeTuple::start(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Self::SerializeTupleStruct::start(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Self::SerializeTupleVariant::start(len, variant)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Self::SerializeMap::start(len)
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Self::SerializeStruct::start(Some(len), name)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Self::SerializeStructVariant::start(len, variant)
    }
}

impl InlineSerializer {
    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    fn serialize_integer<T: Integer>(self, v: T) -> Result<String> {
        const FORMAT: u128 = NumberFormatBuilder::new().build();
        const INT_OPTIONS: WriteIntegerOptions = WriteIntegerOptions::new();

        Ok(lexical::to_string_with_options::<T, FORMAT>(
            v,
            &INT_OPTIONS,
        ))
    }
}

impl InlineSerializer {
    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    fn serialize_float<T: Float>(self, v: T) -> Result<String> {
        const FORMAT: u128 = NumberFormatBuilder::new().build();
        const FLOAT_OPTIONS: WriteFloatOptions = WriteFloatOptions::new();

        Ok(if v.is_nan() {
            if v.is_sign_positive() {
                "nan".into()
            } else {
                "-nan".into() // We preserve the sign for NaN
            }
        } else if !v.is_finite() {
            if v.is_sign_positive() {
                "inf".into()
            } else {
                "-inf".into()
            }
        } else {
            lexical::to_string_with_options::<T, FORMAT>(v, &FLOAT_OPTIONS)
        })
    }
}

impl InlineSerializer {
    #[allow(clippy::unused_self)]
    fn serialize_basic_str(self, value: &str) -> Result<String> {
        #[allow(clippy::trivially_copy_pass_by_ref)] // makes the function more ergonomic to use
        const fn is_escape(ch: &u8) -> bool {
            matches!(*ch, 0x00..=0x1f | b'\"' | b'\\' | 0x7f)
        }

        let mut buf = String::with_capacity(value.len() * 2);
        buf.write_char('"')?;

        let mut rest = value;
        loop {
            let esc_pos = rest
                .as_bytes()
                .iter()
                .position(is_escape)
                .unwrap_or(rest.len());
            buf.write_str(&rest[..esc_pos])?;
            rest = &rest[esc_pos..];

            let Some(ch) = rest.chars().next() else { break };
            match ch {
                // Backspace
                '\x08' => buf.write_str("\\b")?,
                // Tab - doesn't need escaping per se, but it's pretty ugly in a single line string
                '\x09' => buf.write_str("\\t")?,
                // Newline
                '\n' => buf.write_str("\\n")?,
                // Form feed
                '\x0c' => buf.write_str("\\f")?,
                // Carriage return
                '\r' => buf.write_str("\\r")?,
                // Quote
                '"' => buf.write_str("\\\"")?,
                // Backslash
                '\\' => buf.write_str("\\\\")?,
                // Other control characters
                '\x00'..='\x1f' | '\x7f' => write!(buf, "\\u{:04x}", u32::from(ch))?,
                // Other characters (unreachable)
                ch => {
                    unreachable!("unexpected character: {ch}")
                }
            }
            rest = &rest[ch.len_utf8()..];
        }

        buf.write_char('"')?;

        Ok(buf)
    }

    #[allow(clippy::unused_self)]
    fn serialize_multiline_basic_str(self, value: &str) -> Result<String> {
        #[allow(clippy::trivially_copy_pass_by_ref)] // makes the function more ergonomic to use
        const fn is_escape(ch: &u8) -> bool {
            matches!(*ch, 0x00..=0x08 | 0x0b..=0x1f | b'\"' | b'\\' | 0x7f)
        }

        let mut buf = String::with_capacity(value.len() * 2);
        buf.write_str("\"\"\"\n")?;

        let mut rest = value;
        loop {
            let esc_pos = rest
                .as_bytes()
                .iter()
                .position(is_escape)
                .unwrap_or(rest.len());
            buf.write_str(&rest[..esc_pos])?;
            rest = &rest[esc_pos..];

            let Some(ch) = rest.chars().next() else { break };
            match ch {
                // Backspace
                '\x08' => buf.write_str("\\b")?,
                // Form feed
                '\x0c' => buf.write_str("\\f")?,
                // Carriage return - we always use unix line endings, so always escape \r
                '\r' => buf.write_str("\\r")?,
                // We don't need to escape double quotes as long as we don't have a sequence of 3
                // But it's easier to escape all double quotes by default
                '"' => buf.write_str("\\\"")?,
                // Backslash
                '\\' => buf.write_str("\\\\")?,
                // Other control characters
                '\x00'..='\x1f' | '\x7f' => write!(buf, "\\u{:04x}", u32::from(ch))?,
                // Other characters (unreachable)
                ch => {
                    unreachable!("unexpected character: {ch}")
                }
            }
            rest = &rest[ch.len_utf8()..];
        }

        buf.write_str("\"\"\"")?;

        Ok(buf)
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use indoc::indoc;
    use maplit::hashmap;
    use serde::Serializer as _;

    use super::*;
    use crate::value::{LocalDate, LocalTime, Offset, OffsetDatetime};

    mod example {
        use std::collections::HashMap;

        use crate::value::OffsetDatetime;

        #[derive(Debug, PartialEq, Eq, serde::Serialize)]
        pub struct Struct {
            pub title: String,
            pub owner: Owner,
            pub database: Database,
            pub servers: HashMap<String, Server>,
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
            pub data: HashMap<String, usize>,
        }
    }

    #[test]
    fn test_to_string() {
        let value = example::Struct {
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
            servers: hashmap! {
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
                data: hashmap! {
                    "gamma".into() => 1,
                    "delta".into() => 2,
                },
            },
        };

        assert_eq!(
            to_string(&value).unwrap(),
            indoc! {r#"
                title = "TOML Example"

                [clients]
                hosts = ["alpha", "omega"]

                [clients.data]
                delta = 2
                gamma = 1

                [database]
                connection_max = 5000
                enabled = true
                ports = [8000, 8001, 8002]
                server = "192.168.1.1"

                [owner]
                dob = 1979-05-27T07:32:00-08:00
                name = "Tom Preston-Werner"

                [servers.alpha]
                dc = "eqdc10"
                ip = "10.0.0.1"

                [servers.beta]
                dc = "eqdc10"
                ip = "10.0.0.2"
            "#}
        );
    }

    #[test]
    fn serializer_new() {
        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        assert_eq!(serializer.writer.as_ptr(), result.as_ptr());
    }

    #[test]
    fn serializer_from_fmt_writer() {
        let mut result = String::new();
        let serializer = Serializer::from_fmt_writer(&mut result);

        assert_eq!(serializer.writer.as_ptr(), result.as_ptr());
    }

    #[test]
    fn serializer_from_io_writer() {
        let mut result = Vec::new();
        let _serializer = Serializer::from_io_writer(&mut result);

        // TODO we don't have access to the inner buffer here
        // assert_eq!(serializer.writer.as_ptr(), result.as_ptr());
    }

    #[test]
    fn serializer_split_inlines_and_subtables() {
        let table = vec![
            ("foo".into(), ValueKind::InlineValue("bar".into())),
            (
                "baz".into(),
                ValueKind::Table(TableKind::Table(vec![(
                    "qux".into(),
                    ValueKind::InlineValue("quux".into()),
                )])),
            ),
        ];

        let (inlines, subtables) = Serializer::<String>::split_inlines_and_subtables(table);

        assert_eq!(inlines, vec![("foo".into(), "bar".into())]);
        assert_eq!(
            subtables,
            vec![(
                "baz".into(),
                TableKind::Table(vec![("qux".into(), ValueKind::InlineValue("quux".into())),])
            )]
        );
    }

    #[test]
    fn serializer_write() {
        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        serializer
            .write(vec![
                ("foo".into(), ValueKind::InlineValue("bar".into())),
                (
                    "baz".into(),
                    ValueKind::Table(TableKind::Table(vec![(
                        "qux".into(),
                        ValueKind::InlineValue("quux".into()),
                    )])),
                ),
            ])
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                foo = bar

                [baz]
                qux = quux
            "}
        );
    }

    #[test]
    fn serializer_write_table() {
        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table(
                vec![
                    ("bar".into(), ValueKind::InlineValue("baz".into())),
                    (
                        "qux".into(),
                        ValueKind::Table(TableKind::Table(vec![(
                            "quux".into(),
                            ValueKind::InlineValue("corge".into()),
                        )])),
                    ),
                ],
                "foo",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [foo]
                bar = baz

                [foo.qux]
                quux = corge
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table(
                vec![(
                    "bar".into(),
                    ValueKind::Table(TableKind::Table(vec![(
                        "baz".into(),
                        ValueKind::InlineValue("qux".into()),
                    )])),
                )],
                "foo",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [foo.bar]
                baz = qux
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer.write_table(vec![], "foo").unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [foo]
            "}
        );
    }

    #[test]
    fn serializer_write_array_of_tables() {
        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_array_of_tables(
                vec![vec![
                    ("bar".into(), ValueKind::InlineValue("baz".into())),
                    (
                        "qux".into(),
                        ValueKind::Table(TableKind::Table(vec![(
                            "quux".into(),
                            ValueKind::InlineValue("corge".into()),
                        )])),
                    ),
                ]],
                "foo",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [[foo]]
                bar = baz

                [foo.qux]
                quux = corge
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_array_of_tables(
                vec![vec![(
                    "bar".into(),
                    ValueKind::Table(TableKind::Table(vec![(
                        "baz".into(),
                        ValueKind::InlineValue("qux".into()),
                    )])),
                )]],
                "foo",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [[foo]]
                [foo.bar]
                baz = qux
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_array_of_tables(vec![vec![]], "foo")
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [[foo]]
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer.write_array_of_tables(vec![], "foo").unwrap();

        assert_eq!(result, indoc! {r""});
    }

    #[test]
    fn serializer_write_table_inlines() {
        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table_inlines(vec![
                ("foo".into(), "bar".into()),
                ("baz".into(), "qux".into()),
            ])
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                baz = qux
                foo = bar
            "}
        );
    }

    #[test]
    fn serializer_write_subtable() {
        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_subtable(
                "bar".into(),
                TableKind::Table(vec![("baz".into(), ValueKind::InlineValue("qux".into()))]),
                "foo",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [foo.bar]
                baz = qux
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_subtable(
                "bar".into(),
                TableKind::Array(vec![vec![(
                    "baz".into(),
                    ValueKind::InlineValue("qux".into()),
                )]]),
                "foo",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [[foo.bar]]
                baz = qux
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_subtable(
                "foo".into(),
                TableKind::Table(vec![("bar".into(), ValueKind::InlineValue("baz".into()))]),
                "",
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [foo]
                bar = baz
            "}
        );
    }

    #[test]
    fn serializer_write_table_subtables() {
        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table_subtables(
                vec![
                    (
                        "bar".into(),
                        TableKind::Table(vec![(
                            "baz".into(),
                            ValueKind::InlineValue("qux".into()),
                        )]),
                    ),
                    (
                        "quux".into(),
                        TableKind::Array(vec![vec![(
                            "corge".into(),
                            ValueKind::InlineValue("grault".into()),
                        )]]),
                    ),
                ],
                "foo",
                false,
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"
                [foo.bar]
                baz = qux

                [[foo.quux]]
                corge = grault
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table_subtables(
                vec![
                    (
                        "bar".into(),
                        TableKind::Table(vec![(
                            "baz".into(),
                            ValueKind::InlineValue("qux".into()),
                        )]),
                    ),
                    (
                        "quux".into(),
                        TableKind::Array(vec![vec![(
                            "corge".into(),
                            ValueKind::InlineValue("grault".into()),
                        )]]),
                    ),
                ],
                "foo",
                true,
            )
            .unwrap();

        assert_eq!(
            result,
            indoc! {r"

                [foo.bar]
                baz = qux

                [[foo.quux]]
                corge = grault
            "}
        );

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table_subtables(vec![], "foo", false)
            .unwrap();

        assert_eq!(result, "");

        let mut result = String::new();
        let mut serializer = Serializer::new(&mut result);

        serializer
            .write_table_subtables(vec![], "foo", true)
            .unwrap();

        assert_eq!(result, "");
    }

    #[test]
    fn serializer_serialize_newtype_variant() {
        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        serializer
            .serialize_newtype_variant("name", 0, "foo", "bar")
            .unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                foo = "bar"
            "#}
        );
    }

    #[test]
    fn serializer_serialize_tuple_variant() {
        use ser::SerializeTupleVariant as _;

        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        let mut wrapped_arr_ser = serializer
            .serialize_tuple_variant("name", 0, "foo", 2)
            .unwrap();
        wrapped_arr_ser.serialize_field(&42).unwrap();
        wrapped_arr_ser.serialize_field("bar").unwrap();
        wrapped_arr_ser.end().unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                foo = [42, "bar"]
            "#}
        );
    }

    #[test]
    fn serializer_serialize_map() {
        use ser::SerializeMap as _;

        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        let mut wrapped_map_ser = serializer.serialize_map(Some(1)).unwrap();
        wrapped_map_ser.serialize_entry("foo", "bar").unwrap();
        wrapped_map_ser.end().unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                foo = "bar"
            "#}
        );
    }

    #[test]
    fn serializer_serialize_struct() {
        use ser::SerializeStruct as _;

        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        let mut wrapped_struct_ser = serializer.serialize_struct("name", 1).unwrap();
        wrapped_struct_ser.serialize_field("foo", "bar").unwrap();
        wrapped_struct_ser.end().unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                foo = "bar"
            "#}
        );
    }

    #[test]
    fn serializer_serialize_struct_variant() {
        use ser::SerializeStructVariant as _;

        let mut result = String::new();
        let serializer = Serializer::new(&mut result);

        let mut wrapped_struct_ser = serializer
            .serialize_struct_variant("name", 0, "foo", 1)
            .unwrap();
        wrapped_struct_ser.serialize_field("bar", "baz").unwrap();
        wrapped_struct_ser.serialize_field("qux", &42).unwrap();
        wrapped_struct_ser.end().unwrap();

        assert_eq!(
            result,
            indoc! {r#"
                [foo]
                bar = "baz"
                qux = 42
            "#}
        );
    }

    #[test]
    fn inline_serializer_serialize_bool() {
        assert_eq!(InlineSerializer.serialize_bool(true).unwrap(), "true");
        assert_eq!(InlineSerializer.serialize_bool(false).unwrap(), "false");
    }

    #[test]
    fn inline_serializer_serialize_i8() {
        assert_eq!(InlineSerializer.serialize_i8(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_i8(-12).unwrap(), "-12");
    }

    #[test]
    fn inline_serializer_serialize_i16() {
        assert_eq!(InlineSerializer.serialize_i16(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_i16(-12).unwrap(), "-12");
    }

    #[test]
    fn inline_serializer_serialize_i32() {
        assert_eq!(InlineSerializer.serialize_i32(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_i32(-12).unwrap(), "-12");
    }

    #[test]
    fn inline_serializer_serialize_i64() {
        assert_eq!(InlineSerializer.serialize_i64(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_i64(-12).unwrap(), "-12");
    }

    #[test]
    fn inline_serializer_serialize_i128() {
        assert_eq!(InlineSerializer.serialize_i128(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_i128(-12).unwrap(), "-12");
    }

    #[test]
    fn inline_serializer_serialize_u8() {
        assert_eq!(InlineSerializer.serialize_u8(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_u8(12).unwrap(), "12");
    }

    #[test]
    fn inline_serializer_serialize_u16() {
        assert_eq!(InlineSerializer.serialize_u16(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_u16(12).unwrap(), "12");
    }

    #[test]
    fn inline_serializer_serialize_u32() {
        assert_eq!(InlineSerializer.serialize_u32(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_u32(12).unwrap(), "12");
    }

    #[test]
    fn inline_serializer_serialize_u64() {
        assert_eq!(InlineSerializer.serialize_u64(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_u64(12).unwrap(), "12");
    }

    #[test]
    fn inline_serializer_serialize_u128() {
        assert_eq!(InlineSerializer.serialize_u128(42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_u128(12).unwrap(), "12");
    }

    #[test]
    fn inline_serializer_serialize_f32() {
        assert_eq!(InlineSerializer.serialize_f32(42.0).unwrap(), "42.0");
        assert_eq!(InlineSerializer.serialize_f32(-12.0).unwrap(), "-12.0");
        assert_eq!(InlineSerializer.serialize_f32(1e12).unwrap(), "1.0e12");
        assert_eq!(InlineSerializer.serialize_f32(0.5e-9).unwrap(), "5.0e-10");
        assert_eq!(
            InlineSerializer.serialize_f32(f32::INFINITY).unwrap(),
            "inf"
        );
        assert_eq!(
            InlineSerializer.serialize_f32(f32::NEG_INFINITY).unwrap(),
            "-inf"
        );
        assert_eq!(InlineSerializer.serialize_f32(f32::NAN).unwrap(), "nan");
        assert_eq!(InlineSerializer.serialize_f32(-f32::NAN).unwrap(), "-nan");
    }

    #[test]
    fn inline_serializer_serialize_f64() {
        assert_eq!(InlineSerializer.serialize_f64(42.0).unwrap(), "42.0");
        assert_eq!(InlineSerializer.serialize_f64(-12.0).unwrap(), "-12.0");
        assert_eq!(InlineSerializer.serialize_f64(1e12).unwrap(), "1.0e12");
        assert_eq!(InlineSerializer.serialize_f64(0.5e-9).unwrap(), "5.0e-10");
        assert_eq!(
            InlineSerializer.serialize_f64(f64::INFINITY).unwrap(),
            "inf"
        );
        assert_eq!(
            InlineSerializer.serialize_f64(f64::NEG_INFINITY).unwrap(),
            "-inf"
        );
        assert_eq!(InlineSerializer.serialize_f64(f64::NAN).unwrap(), "nan");
        assert_eq!(InlineSerializer.serialize_f64(-f64::NAN).unwrap(), "-nan");
    }

    #[test]
    fn inline_serializer_serialize_char() {
        assert_eq!(InlineSerializer.serialize_char('a').unwrap(), r#""a""#);
        assert_eq!(InlineSerializer.serialize_char('😎').unwrap(), r#""😎""#);
        assert_eq!(
            InlineSerializer.serialize_char('\n').unwrap(),
            indoc! {r#"
                """

                """"#}
        );
    }

    #[test]
    fn inline_serializer_serialize_str() {
        assert_eq!(InlineSerializer.serialize_str("foo").unwrap(), r#""foo""#);
        assert_eq!(InlineSerializer.serialize_str("😎").unwrap(), r#""😎""#);
        assert_eq!(
            InlineSerializer.serialize_str("abc\ndef\n").unwrap(),
            indoc! {r#"
                """
                abc
                def
                """"#}
        );
    }

    #[test]
    fn inline_serializer_serialize_bytes() {
        assert_eq!(
            InlineSerializer.serialize_bytes(b"foo").unwrap(),
            "[102, 111, 111]"
        );
        assert_eq!(
            InlineSerializer
                .serialize_bytes(b"\xF0\x9F\x98\x8E")
                .unwrap(),
            "[240, 159, 152, 142]"
        );
        assert_eq!(
            InlineSerializer.serialize_bytes(b"abc\ndef\n").unwrap(),
            "[97, 98, 99, 10, 100, 101, 102, 10]"
        );
    }

    #[test]
    fn inline_serializer_serialize_none() {
        InlineSerializer.serialize_none().unwrap_err();
    }

    #[test]
    fn inline_serializer_serialize_some() {
        assert_eq!(InlineSerializer.serialize_some(&42).unwrap(), "42");
        assert_eq!(InlineSerializer.serialize_some("foo").unwrap(), r#""foo""#);
    }

    #[test]
    fn inline_serializer_serialize_unit() {
        InlineSerializer.serialize_unit().unwrap_err();
    }

    #[test]
    fn inline_serializer_serialize_unit_struct() {
        InlineSerializer.serialize_unit_struct("name").unwrap_err();
    }

    #[test]
    fn inline_serializer_serialize_unit_variant() {
        assert_eq!(
            InlineSerializer
                .serialize_unit_variant("name", 0, "foo")
                .unwrap(),
            r#""foo""#
        );
    }

    #[test]
    fn inline_serializer_serialize_newtype_struct() {
        assert_eq!(
            InlineSerializer
                .serialize_newtype_struct("name", &42)
                .unwrap(),
            "42"
        );
    }

    #[test]
    fn inline_serializer_serialize_newtype_variant() {
        assert_eq!(
            InlineSerializer
                .serialize_newtype_variant("name", 0, "foo", &42)
                .unwrap(),
            "{ foo = 42 }"
        );
    }

    #[test]
    fn inline_serializer_serialize_seq() {
        use ser::SerializeSeq as _;

        let mut seq = InlineSerializer.serialize_seq(Some(2)).unwrap();
        seq.serialize_element(&42).unwrap();
        seq.serialize_element(&"foo").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"[42, "foo"]"#);
    }

    #[test]
    fn inline_serializer_serialize_tuple() {
        use ser::SerializeTuple as _;

        let mut seq = InlineSerializer.serialize_tuple(2).unwrap();
        seq.serialize_element(&42).unwrap();
        seq.serialize_element(&"foo").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"[42, "foo"]"#);
    }

    #[test]
    fn inline_serializer_serialize_tuple_struct() {
        use ser::SerializeTupleStruct as _;

        let mut seq = InlineSerializer.serialize_tuple_struct("name", 2).unwrap();
        seq.serialize_field(&42).unwrap();
        seq.serialize_field(&"foo").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"[42, "foo"]"#);
    }

    #[test]
    fn inline_serializer_serialize_tuple_variant() {
        use ser::SerializeTupleVariant as _;

        let mut seq = InlineSerializer
            .serialize_tuple_variant("name", 0, "foo", 2)
            .unwrap();
        seq.serialize_field(&42).unwrap();
        seq.serialize_field(&"bar").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"{ foo = [42, "bar"] }"#);
    }

    #[test]
    fn inline_serializer_serialize_map() {
        use ser::SerializeMap as _;

        let mut seq = InlineSerializer.serialize_map(Some(2)).unwrap();
        seq.serialize_entry("foo", &42).unwrap();
        seq.serialize_entry("bar", &"baz").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"{ foo = 42, bar = "baz" }"#);
    }

    #[test]
    fn inline_serializer_serialize_struct() {
        use ser::SerializeStruct as _;

        let mut seq = InlineSerializer.serialize_struct("name", 2).unwrap();
        seq.serialize_field("foo", &42).unwrap();
        seq.serialize_field("bar", &"baz").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"{ foo = 42, bar = "baz" }"#);
    }

    #[test]
    fn inline_serializer_serialize_struct_variant() {
        use ser::SerializeStructVariant as _;

        let mut seq = InlineSerializer
            .serialize_struct_variant("name", 0, "foo", 2)
            .unwrap();
        seq.serialize_field("bar", &42).unwrap();
        seq.serialize_field("baz", &"qux").unwrap();
        let result = seq.end().unwrap();

        assert_eq!(result, r#"{ foo = { bar = 42, baz = "qux" } }"#);
    }

    #[test]
    fn inline_serializer_serialize_basic_str() {
        assert_eq!(
            InlineSerializer.serialize_basic_str("foo").unwrap(),
            r#""foo""#
        );
        assert_eq!(
            InlineSerializer.serialize_basic_str("😎").unwrap(),
            r#""😎""#
        );
        assert_eq!(
            InlineSerializer.serialize_basic_str("abc\ndef\n").unwrap(),
            r#""abc\ndef\n""#
        );
        assert_eq!(
            InlineSerializer
                .serialize_basic_str("\x08\x09\x0A\x0C\x0D\"\\")
                .unwrap(),
            r#""\b\t\n\f\r\"\\""#
        );
        assert_eq!(
            InlineSerializer
                .serialize_basic_str(
                    "\x00\x01\x02\x03\x04\x05\x06\x07\x0B\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F"
                )
                .unwrap(),
                r#""\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\u000b\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f""#
        );
    }

    #[test]
    fn inline_serializer_serialize_multiline_basic_str() {
        assert_eq!(
            InlineSerializer
                .serialize_multiline_basic_str("foo")
                .unwrap(),
            indoc! {r#"
                """
                foo""""#}
        );
        assert_eq!(
            InlineSerializer
                .serialize_multiline_basic_str("😎")
                .unwrap(),
            indoc! {r#"
                """
                😎""""#}
        );
        assert_eq!(
            InlineSerializer
                .serialize_multiline_basic_str("abc\ndef\n")
                .unwrap(),
            indoc! {r#"
                """
                abc
                def
                """"#}
        );
        assert_eq!(
            InlineSerializer
                .serialize_multiline_basic_str("\x08\x09\x0A\x0C\x0D\"\\")
                .unwrap(),
            indoc! {"
                \"\"\"
                \\b\t
                \\f\\r\\\"\\\\\"\"\""}
        );
        assert_eq!(
            InlineSerializer
                .serialize_multiline_basic_str(
                    "\x00\x01\x02\x03\x04\x05\x06\x07\x0B\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F"
                )
                .unwrap(),
            indoc! {r#"
                """
                \u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\u000b\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f""""#}
        );
    }
}
