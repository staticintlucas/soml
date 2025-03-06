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
            temp.sort_by(|a, b| a.0.cmp(&b.0));
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
        mut subtables: Vec<(String, TableKind)>,
        path: &str,
        need_nl: bool,
    ) -> Result<()> {
        // Sort alphabetically for deterministic test output
        if cfg!(test) {
            #[allow(clippy::pattern_type_mismatch)]
            subtables.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
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
        const FLOAT_OPTIONS: WriteFloatOptions = WriteFloatOptions::builder()
            .nan_string(Some(b"nan"))
            .inf_string(Some(b"inf"))
            .build_unchecked();

        Ok(lexical::to_string_with_options::<T, FORMAT>(
            v,
            &FLOAT_OPTIONS,
        ))
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
