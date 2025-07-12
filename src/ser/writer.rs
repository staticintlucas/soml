use std::{fmt, io};

use crate::ser::tree;

#[derive(Debug)]
pub struct IoWriter<T: io::Write> {
    pub(super) writer: T,
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
        self.writer.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }

    #[inline]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.writer.write_fmt(args).map_err(|_| fmt::Error)
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
                ch => unreachable!("unexpected character: {ch}"),
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
                ch => unreachable!("unexpected character: {ch}"),
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
        if let Some((first, rest)) = array.split_first() {
            let (inlines, subtables) = split_inlines_and_subtables(first);

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

            for table in rest {
                writeln!(f)?; // Newline between subtables

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
    use indoc::indoc;

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

    #[test]
    fn formatter_write_key() {
        let mut buf = String::new();
        Formatter::write_key("foo", &mut buf).unwrap();
        assert_eq!(buf, "foo");

        let mut buf = String::new();
        Formatter::write_key("abc.123", &mut buf).unwrap();
        assert_eq!(buf, r#""abc.123""#);

        let mut buf = String::new();
        Formatter::write_key("😎", &mut buf).unwrap();
        assert_eq!(buf, r#""😎""#);
    }

    #[test]
    fn formatter_write_string() {
        let mut buf = String::new();
        Formatter::write_string("foo", &mut buf).unwrap();
        assert_eq!(buf, r#""foo""#);

        let mut buf = String::new();
        Formatter::write_string("😎", &mut buf).unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        Formatter::write_string("abc\ndef\n", &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """
                abc
                def
                """"#}
        );
    }

    #[test]
    fn formatter_write_basic_str() {
        let mut buf = String::new();
        Formatter::write_basic_string("foo", &mut buf).unwrap();
        assert_eq!(buf, r#""foo""#);

        let mut buf = String::new();
        Formatter::write_basic_string("😎", &mut buf).unwrap();
        assert_eq!(buf, r#""😎""#);

        let mut buf = String::new();
        Formatter::write_basic_string("abc\ndef\n", &mut buf).unwrap();
        assert_eq!(buf, r#""abc\ndef\n""#);

        let mut buf = String::new();
        Formatter::write_basic_string("\x08\x09\x0A\x0C\x0D\"\\", &mut buf).unwrap();
        assert_eq!(buf, r#""\b\t\n\f\r\"\\""#);

        let mut buf = String::new();
        Formatter::write_basic_string("\x00\x01\x02\x03\x04\x05\x06\x07\x0B\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F", &mut buf).unwrap();
        assert_eq!(
            buf,
            r#""\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\u000b\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f""#
        );
    }

    #[test]
    fn formatter_write_multiline_basic_str() {
        let mut buf = String::new();
        Formatter::write_multiline_basic_string("foo", &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """
                foo""""#}
        );
        let mut buf = String::new();
        Formatter::write_multiline_basic_string("😎", &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """
                😎""""#}
        );
        let mut buf = String::new();
        Formatter::write_multiline_basic_string("abc\ndef\n", &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """
                abc
                def
                """"#}
        );
        let mut buf = String::new();
        Formatter::write_multiline_basic_string("\x08\x09\x0A\x0C\x0D\"\\", &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {"
                \"\"\"
                \\b\t
                \\f\\r\\\"\\\\\"\"\""}
        );
        let mut buf = String::new();
        Formatter::write_multiline_basic_string("\x00\x01\x02\x03\x04\x05\x06\x07\x0B\x0E\x0F\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F", &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r#"
                """
                \u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\u000b\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f""""#}
        );
    }

    #[test]
    fn formatter_write_integer() {
        let mut buf = String::new();
        Formatter::write_integer(&42, &mut buf).unwrap();
        assert_eq!(buf, "42");

        let mut buf = String::new();
        Formatter::write_integer(&-12, &mut buf).unwrap();
        assert_eq!(buf, "-12");
    }

    #[test]
    fn formatter_write_float() {
        let mut buf = String::new();
        Formatter::write_float(&42.0, &mut buf).unwrap();
        assert_eq!(buf, "42.0");

        let mut buf = String::new();
        Formatter::write_float(&-12.0, &mut buf).unwrap();
        assert_eq!(buf, "-12.0");

        let mut buf = String::new();
        Formatter::write_float(&1e28, &mut buf).unwrap();
        assert_eq!(buf, "1e28");

        let mut buf = String::new();
        Formatter::write_float(&0.5e-9, &mut buf).unwrap();
        assert_eq!(buf, "5e-10");

        let mut buf = String::new();
        Formatter::write_float(&f64::INFINITY, &mut buf).unwrap();
        assert_eq!(buf, "inf");

        let mut buf = String::new();
        Formatter::write_float(&f64::NEG_INFINITY, &mut buf).unwrap();
        assert_eq!(buf, "-inf");

        let mut buf = String::new();
        Formatter::write_float(&f64::NAN, &mut buf).unwrap();
        assert_eq!(buf, "nan");

        let mut buf = String::new();
        Formatter::write_float(&-f64::NAN, &mut buf).unwrap();
        assert_eq!(buf, "-nan");
    }

    #[test]
    fn formatter_write_table_header() {
        let mut buf = String::new();
        Formatter::write_table_header(&[], &mut buf).unwrap();
        assert_eq!(buf, "");

        let path = ["a", "b", "c"].map(ToString::to_string);
        let mut buf = String::new();
        Formatter::write_table_header(&path.each_ref(), &mut buf).unwrap();
        assert_eq!(buf, "[a.b.c]\n");

        let path = ["a", "b.c", "d"].map(ToString::to_string);
        let mut buf = String::new();
        Formatter::write_table_header(&path.each_ref(), &mut buf).unwrap();
        assert_eq!(buf, "[a.\"b.c\".d]\n");

        let path = ["a", "😎", "b"].map(ToString::to_string);
        let mut buf = String::new();
        Formatter::write_table_header(&path.each_ref(), &mut buf).unwrap();
        assert_eq!(buf, "[a.\"😎\".b]\n");
    }

    #[test]
    fn formatter_write_array_header() {
        let mut buf = String::new();
        Formatter::write_array_header(&[], &mut buf).unwrap();
        assert_eq!(buf, "");

        let path = ["a", "b", "c"].map(ToString::to_string);
        let mut buf = String::new();
        Formatter::write_array_header(&path.each_ref(), &mut buf).unwrap();
        assert_eq!(buf, "[[a.b.c]]\n");

        let path = ["a", "b.c", "d"].map(ToString::to_string);
        let mut buf = String::new();
        Formatter::write_array_header(&path.each_ref(), &mut buf).unwrap();
        assert_eq!(buf, "[[a.\"b.c\".d]]\n");

        let path = ["a", "😎", "b"].map(ToString::to_string);
        let mut buf = String::new();
        Formatter::write_array_header(&path.each_ref(), &mut buf).unwrap();
        assert_eq!(buf, "[[a.\"😎\".b]]\n");
    }

    #[test]
    fn formatter_write_table() {
        use tree::{Table, Value};

        let mut buf = String::new();
        Formatter::write_table(
            &[
                ("bar".to_string(), Value::Inline("baz".to_string())),
                (
                    "qux".to_string(),
                    Value::Table(Table::Table(vec![(
                        "quux".to_string(),
                        Value::Inline("corge".to_string()),
                    )])),
                ),
            ],
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [foo]
                bar = baz

                [foo.qux]
                quux = corge
            "}
        );

        let mut buf = String::new();
        Formatter::write_table(
            &[(
                "bar".to_string(),
                Value::Table(Table::Table(vec![(
                    "baz".to_string(),
                    Value::Inline("qux".to_string()),
                )])),
            )],
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [foo.bar]
                baz = qux
            "}
        );

        let mut buf = String::new();
        Formatter::write_table(&[], &[&"foo".to_string()], &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [foo]
            "}
        );
    }

    #[test]
    fn formatter_write_array_of_tables() {
        use tree::{Table, Value};

        let mut buf = String::new();
        Formatter::write_array_of_tables(
            &[
                vec![
                    ("bar".to_string(), Value::Inline("baz".to_string())),
                    (
                        "qux".to_string(),
                        Value::Table(Table::Table(vec![(
                            "quux".to_string(),
                            Value::Inline("corge".to_string()),
                        )])),
                    ),
                ],
                vec![("grault".to_string(), Value::Inline("garply".to_string()))],
            ],
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [[foo]]
                bar = baz

                [foo.qux]
                quux = corge

                [[foo]]
                grault = garply
            "}
        );

        let mut buf = String::new();
        Formatter::write_array_of_tables(
            &[vec![(
                "bar".to_string(),
                Value::Table(Table::Table(vec![(
                    "baz".to_string(),
                    Value::Inline("qux".to_string()),
                )])),
            )]],
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [[foo]]
                [foo.bar]
                baz = qux
            "}
        );

        let mut buf = String::new();
        Formatter::write_array_of_tables(&[vec![]], &[&"foo".to_string()], &mut buf).unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [[foo]]
            "}
        );

        let mut buf = String::new();
        Formatter::write_array_of_tables(&[], &[&"foo".to_string()], &mut buf).unwrap();
        assert_eq!(buf, indoc! {r""});
    }

    #[test]
    fn formatter_write_inlines() {
        let mut buf = String::new();
        Formatter::write_inlines(
            &[
                (&"foo".to_string(), &"bar".to_string()),
                (&"baz".to_string(), &"qux".to_string()),
            ],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                foo = bar
                baz = qux
            "}
        );
    }

    #[test]
    fn formatter_write_inline() {
        let mut buf = String::new();
        Formatter::write_inline("foo", "bar", &mut buf).unwrap();
        assert_eq!(buf, "foo = bar\n");

        let mut buf = String::new();
        Formatter::write_inline("a.b", "blah", &mut buf).unwrap();
        assert_eq!(buf, "\"a.b\" = blah\n");

        let mut buf = String::new();
        Formatter::write_inline("😎", "😎", &mut buf).unwrap();
        assert_eq!(buf, "\"😎\" = 😎\n");
    }

    #[test]
    fn formatter_write_subtables() {
        use tree::{Table, Value};

        let mut buf = String::new();
        Formatter::write_subtables(
            &[
                (
                    &"bar".to_string(),
                    &Table::Table(vec![("baz".to_string(), Value::Inline("qux".to_string()))]),
                ),
                (
                    &"quux".to_string(),
                    &Table::Array(vec![vec![(
                        "corge".to_string(),
                        Value::Inline("grault".to_string()),
                    )]]),
                ),
            ],
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [foo.bar]
                baz = qux

                [[foo.quux]]
                corge = grault
            "}
        );

        let mut buf = String::new();
        Formatter::write_subtables(&[], &[&"foo".to_string()], &mut buf).unwrap();
        assert_eq!(buf, "");
    }

    #[test]
    fn formatter_write_subtable() {
        use tree::{Table, Value};

        let mut buf = String::new();
        Formatter::write_subtable(
            &"bar".to_string(),
            &Table::Table(vec![("baz".to_string(), Value::Inline("qux".to_string()))]),
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [foo.bar]
                baz = qux
            "}
        );

        let mut buf = String::new();
        Formatter::write_subtable(
            &"bar".to_string(),
            &Table::Array(vec![vec![(
                "baz".to_string(),
                Value::Inline("qux".to_string()),
            )]]),
            &[&"foo".to_string()],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [[foo.bar]]
                baz = qux
            "}
        );

        let mut buf = String::new();
        Formatter::write_subtable(
            &"foo".to_string(),
            &Table::Table(vec![("bar".to_string(), Value::Inline("baz".to_string()))]),
            &[],
            &mut buf,
        )
        .unwrap();
        assert_eq!(
            buf,
            indoc! {r"
                [foo]
                bar = baz
            "}
        );
    }

    #[test]
    fn test_split_inlines_and_subtables() {
        use tree::{Table, Value};

        let table = vec![
            ("foo".to_string(), Value::Inline("bar".to_string())),
            (
                "baz".to_string(),
                Value::Table(Table::Table(vec![(
                    "qux".to_string(),
                    Value::Inline("quux".to_string()),
                )])),
            ),
        ];
        let (inlines, subtables) = split_inlines_and_subtables(&table);

        assert_eq!(inlines, vec![(&"foo".to_string(), &"bar".to_string())]);
        assert_eq!(
            subtables,
            vec![(
                &"baz".to_string(),
                &Table::Table(vec![("qux".to_string(), Value::Inline("quux".to_string())),])
            )]
        );
    }
}
