use std::{fmt, io};

use serde::ser::Error as _;

use crate::ser::tree;

#[derive(Debug)]
pub struct IoWriter<T: io::Write> {
    writer: T,
}

impl<T> IoWriter<T>
where
    T: io::Write,
{
    #[inline]
    pub fn new(writer: T) -> Self {
        Self { writer }
    }
}

impl<T> fmt::Write for IoWriter<T>
where
    T: io::Write,
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer
            .write_all(s.as_bytes())
            .map_err(|e| fmt::Error::custom(e.to_string()))
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.writer
            .write_fmt(args)
            .map_err(|e| fmt::Error::custom(e.to_string()))
    }
}

pub struct Formatter;

impl Formatter {
    pub fn write_key(key: &str, f: &mut dyn fmt::Write) -> fmt::Result {
        let is_bare_key = |b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-');

        if !key.is_empty() && key.bytes().all(is_bare_key) {
            f.write_str(key)
        } else {
            Self::write_basic_string(key, f)
        }
    }

    pub fn write_string(value: &str, f: &mut dyn fmt::Write) -> fmt::Result {
        // TODO also test where literal strings might be better?
        if value.contains('\n') {
            Self::write_multiline_basic_string(value, f)
        } else {
            Self::write_basic_string(value, f)
        }
    }

    pub fn write_basic_string(value: &str, f: &mut dyn fmt::Write) -> fmt::Result {
        #[allow(clippy::trivially_copy_pass_by_ref)] // makes the function more ergonomic to use
        const fn is_escape(ch: &u8) -> bool {
            matches!(*ch, 0x00..=0x1f | b'\"' | b'\\' | 0x7f)
        }

        f.write_str(r#"""#)?;

        let mut rest = value;
        loop {
            let esc_pos = rest
                .as_bytes()
                .iter()
                .position(is_escape)
                .unwrap_or(rest.len());
            f.write_str(&rest[..esc_pos])?;
            rest = &rest[esc_pos..];

            let Some(ch) = rest.chars().next() else { break };
            match ch {
                // Backspace
                '\x08' => f.write_str("\\b")?,
                // Tab - doesn't need escaping per se, but it's pretty ugly in a single line string
                '\x09' => f.write_str("\\t")?,
                // Newline
                '\n' => f.write_str("\\n")?,
                // Form feed
                '\x0c' => f.write_str("\\f")?,
                // Carriage return
                '\r' => f.write_str("\\r")?,
                // Quote
                '"' => f.write_str("\\\"")?,
                // Backslash
                '\\' => f.write_str("\\\\")?,
                // Other control characters
                '\x00'..='\x1f' | '\x7f' => write!(f, "\\u{:04x}", u32::from(ch))?,
                // Other characters (unreachable)
                ch => {
                    unreachable!("unexpected character: {ch}")
                }
            }
            rest = &rest[ch.len_utf8()..];
        }

        f.write_str(r#"""#)
    }

    pub fn write_multiline_basic_string(value: &str, f: &mut dyn fmt::Write) -> fmt::Result {
        #[allow(clippy::trivially_copy_pass_by_ref)] // makes the function more ergonomic to use
        const fn is_escape(ch: &u8) -> bool {
            matches!(*ch, 0x00..=0x08 | 0x0b..=0x1f | b'\"' | b'\\' | 0x7f)
        }

        // writeln since newlines after the """ get trimmed anyway
        writeln!(f, r#"""""#)?;

        let mut rest = value;
        loop {
            let esc_pos = rest
                .as_bytes()
                .iter()
                .position(is_escape)
                .unwrap_or(rest.len());
            f.write_str(&rest[..esc_pos])?;
            rest = &rest[esc_pos..];

            let Some(ch) = rest.chars().next() else { break };
            match ch {
                // Backspace
                '\x08' => f.write_str("\\b")?,
                // Form feed
                '\x0c' => f.write_str("\\f")?,
                // Carriage return - we always use unix line endings, so always escape \r
                '\r' => f.write_str("\\r")?,
                // We don't need to escape double quotes as long as we don't have a sequence of 3
                // But it's easier to escape all double quotes by default
                '"' => f.write_str("\\\"")?,
                // Backslash
                '\\' => f.write_str("\\\\")?,
                // Other control characters
                '\x00'..='\x1f' | '\x7f' => write!(f, "\\u{:04x}", u32::from(ch))?,
                // Other characters (unreachable)
                ch => {
                    unreachable!("unexpected character: {ch}")
                }
            }
            rest = &rest[ch.len_utf8()..];
        }

        f.write_str(r#"""""#)
    }

    #[inline]
    pub fn write_integer<I: Integer>(value: &I, f: &mut dyn fmt::Write) -> fmt::Result {
        value.fmt(f)
    }

    #[inline]
    pub fn write_float<F: Float>(value: &F, f: &mut dyn fmt::Write) -> fmt::Result {
        value.fmt(f)
    }

    #[inline]
    pub fn write_table_header(path: &[&String], f: &mut dyn fmt::Write) -> fmt::Result {
        Self::write_header(path, "[", "]", f)
    }

    #[inline]
    pub fn write_array_header(path: &[&String], f: &mut dyn fmt::Write) -> fmt::Result {
        Self::write_header(path, "[[", "]]", f)
    }

    pub fn write_header(
        path: &[&String],
        prefix: &str,
        suffix: &str,
        f: &mut dyn fmt::Write,
    ) -> fmt::Result {
        if let Some((first, rest)) = path.split_first() {
            f.write_str(prefix)?;
            Self::write_key(first, f)?;
            for key in rest {
                f.write_str(".")?;
                Self::write_key(key, f)?;
            }
            f.write_str(suffix)?;
            f.write_str("\n")?;
        }
        Ok(())
    }

    pub fn write_table(
        table: &[(String, tree::Value)],
        path: &[&String],
        f: &mut dyn fmt::Write,
    ) -> fmt::Result {
        let (inlines, subtables) = split_inlines_and_subtables(table);

        // The table header is only needed if the table has inlines (key/value pairs); but if the
        // table is completely empty (no inlines nor subtables) then a reader would have no idea
        // about the existence of the table, so we also write the header in that case.
        let need_header = !inlines.is_empty() || subtables.is_empty();

        // We need a newline between inlines and subtables only if both exist
        let need_nl = !inlines.is_empty() && !subtables.is_empty();

        if need_header {
            Self::write_table_header(path, f)?;
        }
        Self::write_inlines(&inlines, f)?;
        if need_nl {
            writeln!(f)?;
        }
        Self::write_subtables(&subtables, path, f)
    }

    pub fn write_array_of_tables(
        array: &[Vec<(String, tree::Value)>],
        path: &[&String],
        f: &mut dyn fmt::Write,
    ) -> fmt::Result {
        for table in array {
            let (inlines, subtables) = split_inlines_and_subtables(table);

            // We need a newline between inlines and subtables only if both exist
            let need_nl = !inlines.is_empty() && !subtables.is_empty();

            // Unlike a table, we always need to write the array header to create a new element
            // We also know the path here is never empty (can't have a root array of tables)
            Self::write_array_header(path, f)?;

            Self::write_inlines(&inlines, f)?;
            if need_nl {
                writeln!(f)?;
            }
            Self::write_subtables(&subtables, path, f)?;
        }

        Ok(())
    }

    pub fn write_inlines(inlines: &[(&String, &String)], f: &mut dyn fmt::Write) -> fmt::Result {
        for &(key, value) in inlines {
            Self::write_inline(key, value, f)?;
        }
        Ok(())
    }

    pub fn write_inline(key: &str, value: &str, f: &mut dyn fmt::Write) -> fmt::Result {
        Self::write_key(key, f)?;
        f.write_str(" = ")?;
        f.write_str(value)?;
        f.write_str("\n")
    }

    pub fn write_subtables(
        subtables: &[(&String, &tree::Table)],
        path: &[&String],
        f: &mut dyn fmt::Write,
    ) -> fmt::Result {
        if let Some((first, rest)) = subtables.split_first() {
            let (key, table) = *first;
            Self::write_subtable(key, table, path, f)?;

            for &(key, table) in rest {
                writeln!(f)?;
                Self::write_subtable(key, table, path, f)?;
            }
        }

        Ok(())
    }

    pub fn write_subtable(
        key: &String,
        table: &tree::Table,
        path: &[&String],
        f: &mut dyn fmt::Write,
    ) -> fmt::Result {
        let path = {
            let mut tmp = path.to_vec();
            tmp.push(key);
            tmp
        };
        match *table {
            tree::Table::Array(ref array) => Self::write_array_of_tables(array, &path, f),
            tree::Table::Table(ref table) => Self::write_table(table, &path, f),
        }
    }
}

#[allow(clippy::type_complexity)]
fn split_inlines_and_subtables(
    table: &[(String, tree::Value)],
) -> (Vec<(&String, &String)>, Vec<(&String, &tree::Table)>) {
    #[allow(clippy::pattern_type_mismatch)]
    table.iter().fold(
        (Vec::new(), Vec::new()),
        |(mut inlines, mut subtables), (k, v)| {
            match *v {
                tree::Value::Inline(ref value) => inlines.push((k, value)),
                tree::Value::Table(ref table) => subtables.push((k, table)),
            }
            (inlines, subtables)
        },
    )
}

pub trait Integer {
    fn fmt(&self, f: &mut dyn fmt::Write) -> fmt::Result;
}

macro_rules! impl_integer {
    ($($t:ident)*) => ($(
        impl Integer for $t {
            #[inline]
            fn fmt(&self, f: &mut dyn fmt::Write) -> fmt::Result {
                // Just use the Display impl for integer types
                f.write_fmt(format_args!("{}", self))
            }
        }
    )*);
}

impl_integer!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize);

pub trait Float {
    fn fmt(&self, f: &mut dyn fmt::Write) -> fmt::Result;
}

macro_rules! impl_float {
    ($($t:ident)*) => ($(
        impl Float for $t {
            fn fmt(&self, f: &mut dyn fmt::Write) -> fmt::Result {
                if self.is_nan() {
                    // Ryu stringifies nan as NaN and never prints the sign, TOML wants lowercase
                    // and we want to preserve the sign
                    f.write_str(if self.is_sign_positive() { "nan" } else { "-nan" })
                } else {
                    let mut buf = ryu::Buffer::new();
                    f.write_str(buf.format(*self))
                }
            }
        }
    )*);
}

impl_float!(f32 f64);

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use fmt::Write as _;

    use super::*;

    #[test]
    fn io_writer() {
        let mut writer = IoWriter::new(Vec::new());

        writer.write_str("hello").unwrap();
        writer.write_char(' ').unwrap();
        let name = "world";
        writer.write_fmt(format_args!("{name}")).unwrap();

        assert_eq!(writer.writer, b"hello world");
    }
}
