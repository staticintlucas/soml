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

#[derive(Debug)]
pub struct Key<'a>(pub &'a str);

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_bare_key = |b| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-');

        if !self.0.is_empty() && self.0.bytes().all(is_bare_key) {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{}", BasicString(self.0))
        }
    }
}

#[derive(Debug)]
pub struct TomlString<'a>(pub &'a str);

impl fmt::Display for TomlString<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO also test where literal strings might be better?
        if self.0.contains('\n') {
            write!(f, "{}", MultilineBasicString(self.0))
        } else {
            write!(f, "{}", BasicString(self.0))
        }
    }
}

#[derive(Debug)]
pub struct BasicString<'a>(pub &'a str);

impl fmt::Display for BasicString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(clippy::trivially_copy_pass_by_ref)] // makes the function more ergonomic to use
        const fn is_escape(ch: &u8) -> bool {
            matches!(*ch, 0x00..=0x1f | b'\"' | b'\\' | 0x7f)
        }

        f.write_str(r#"""#)?;

        let mut rest = self.0;
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
}

#[derive(Debug)]
pub struct MultilineBasicString<'a>(pub &'a str);

impl fmt::Display for MultilineBasicString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(clippy::trivially_copy_pass_by_ref)] // makes the function more ergonomic to use
        const fn is_escape(ch: &u8) -> bool {
            matches!(*ch, 0x00..=0x08 | 0x0b..=0x1f | b'\"' | b'\\' | 0x7f)
        }

        // writeln since newlines after the """ get trimmed anyway
        writeln!(f, r#"""""#)?;

        let mut rest = self.0;
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
}

#[derive(Debug)]
pub struct InlineValue<'a>(pub &'a tree::Value);

impl fmt::Display for InlineValue<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.0 {
            tree::Value::Table(tree::Table::Table(ref table)) => {
                write!(f, "{}", InlineTable(table))
            }
            tree::Value::Table(tree::Table::Array(ref array)) => {
                write!(f, "{}", InlineArrayOfTables(array))
            }
            tree::Value::Inline(ref value) => f.write_str(value),
        }
    }
}

#[derive(Debug)]
pub struct InlineArray<'a>(pub &'a [tree::Value]);

impl fmt::Display for InlineArray<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        if let Some((first, rest)) = self.0.split_first() {
            write!(f, "{}", InlineValue(first))?;
            for value in rest {
                write!(f, ", {}", InlineValue(value))?;
            }
        }
        f.write_str("]")
    }
}

#[derive(Debug)]
pub struct InlineTable<'a>(pub &'a [(String, tree::Value)]);

impl fmt::Display for InlineTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{ ")?;
        if let Some((first, rest)) = self.0.split_first() {
            let (ref key, ref value) = *first;
            write!(f, "{} = {}", Key(key), InlineValue(value))?;
            #[allow(clippy::pattern_type_mismatch)]
            for (key, value) in rest {
                write!(f, ", {} = {}", Key(key), InlineValue(value))?;
            }
        }
        f.write_str(" }")
    }
}

#[derive(Debug)]
pub struct InlineArrayOfTables<'a>(pub &'a [Vec<(String, tree::Value)>]);

impl fmt::Display for InlineArrayOfTables<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        if let Some((first, rest)) = self.0.split_first() {
            write!(f, "{}", InlineTable(first))?;
            for table in rest {
                write!(f, ", {}", InlineTable(table))?;
            }
        }
        f.write_str("]")
    }
}

#[derive(Debug)]
struct Header<'a> {
    path: &'a [&'a String],
    prefix: &'static str,
    suffix: &'static str,
}

impl fmt::Display for Header<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((first, rest)) = self.path.split_first() {
            write!(f, "{}{}", self.prefix, Key(first))?;
            for key in rest {
                write!(f, ".{}", Key(key))?;
            }
            write!(f, "{}", self.suffix)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct TableHeader<'a> {
    pub path: &'a [&'a String],
}

impl fmt::Display for TableHeader<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            Header {
                path: self.path,
                prefix: "[",
                suffix: "]",
            }
        )
    }
}

#[derive(Debug)]
pub struct ArrayHeader<'a> {
    pub path: &'a [&'a String],
}

impl fmt::Display for ArrayHeader<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            Header {
                path: self.path,
                prefix: "[[",
                suffix: "]]",
            }
        )
    }
}

#[derive(Debug)]
pub struct Table<'a> {
    pub table: &'a [(String, tree::Value)],
    pub path: &'a [&'a String],
}

impl fmt::Display for Table<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (inlines, subtables) = split_inlines_and_subtables(self.table);

        // The table header is only needed if the table has inlines (key/value pairs); but if the
        // table is completely empty (no inlines nor subtables) then a reader would have no idea
        // about the existence of the table, so we also write the header in that case.
        let need_header = !inlines.is_empty() || subtables.is_empty();

        // We need a newline between inlines and subtables only if both exist
        let need_nl = !inlines.is_empty() && !subtables.is_empty();

        if need_header {
            writeln!(f, "{}", TableHeader { path: self.path })?;
        }
        write!(f, "{}", Inlines(&inlines))?;
        if need_nl {
            writeln!(f)?;
        }
        writeln!(
            f,
            "{}",
            Subtables {
                subtables: &subtables,
                path: self.path
            }
        )?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ArrayOfTables<'a> {
    pub array: &'a [Vec<(String, tree::Value)>],
    pub path: &'a [&'a String],
}

impl fmt::Display for ArrayOfTables<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for table in self.array {
            let (inlines, subtables) = split_inlines_and_subtables(table);

            // We need a newline between inlines and subtables only if both exist
            let need_nl = !inlines.is_empty() && !subtables.is_empty();

            // Unlike a table, we always need to write the array header to create a new element
            // We also know the path here is never empty (can't have a root array of tables)
            writeln!(f, "{}", ArrayHeader { path: self.path })?;

            write!(f, "{}", Inlines(&inlines))?;
            if need_nl {
                writeln!(f)?;
            }
            writeln!(
                f,
                "{}",
                Subtables {
                    subtables: &subtables,
                    path: self.path
                }
            )?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Inlines<'a, 'b>(pub &'a [(&'b String, &'b String)]);

impl fmt::Display for Inlines<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &(key, value) in self.0 {
            writeln!(f, "{} = {}", Key(key), value)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Subtables<'a> {
    pub subtables: &'a [(&'a String, &'a tree::Table)],
    pub path: &'a [&'a String],
}

impl fmt::Display for Subtables<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_subtable(
            f: &mut fmt::Formatter<'_>,
            key: &String,
            table: &tree::Table,
            path: &[&String],
        ) -> fmt::Result {
            let path = {
                let mut tmp = path.to_vec();
                tmp.push(key);
                tmp
            };
            let result = match *table {
                tree::Table::Array(ref array) => {
                    write!(
                        f,
                        "{}",
                        ArrayOfTables {
                            array,
                            path: path.as_slice()
                        }
                    )
                }
                tree::Table::Table(ref table) => write!(
                    f,
                    "{}",
                    Table {
                        table,
                        path: path.as_slice()
                    }
                ),
            };
            result
        }

        if let Some((first, rest)) = self.subtables.split_first() {
            let (key, table) = *first;
            write_subtable(f, key, table, self.path)?;

            for &(key, table) in rest {
                writeln!(f)?;
                write_subtable(f, key, table, self.path)?;
            }
        }

        Ok(())
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
