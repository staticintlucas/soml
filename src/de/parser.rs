use std::collections::hash_map::Entry;
use std::{fmt, str};

use serde::de;

use super::error::{ErrorKind, Result};
use super::{reader, Reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpecialFloat {
    Infinity,
    NegInfinity,
    Nan,
    NegNan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Type {
    String,
    Integer,
    Float,
    Boolean,
    Datetime,
    Array,
    Table,
}

impl Type {
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Datetime => "datetime",
            Self::Array => "array",
            Self::Table => "table",
        }
    }
}

impl fmt::Display for Type {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl From<Type> for de::Unexpected<'_> {
    #[inline]
    fn from(typ: Type) -> Self {
        de::Unexpected::Other(typ.to_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    // String; any escape sequences are already parsed
    String(String),
    // Decimal integer
    Integer(Vec<u8>),
    // Binary integer (without the 0b prefix)
    BinaryInt(Vec<u8>),
    // Octal integer (without the 0o prefix)
    OctalInt(Vec<u8>),
    // Hexadecimal integer (without the 0x prefix)
    HexInt(Vec<u8>),
    // Float
    Float(Vec<u8>),
    // Special float (inf, nan, etc)
    SpecialFloat(SpecialFloat),
    // Boolean
    Boolean(bool),
    // Offset Datetime
    OffsetDatetime(Vec<u8>),
    // Local Datetime
    LocalDatetime(Vec<u8>),
    // Local Date
    LocalDate(Vec<u8>),
    // Local Time
    LocalTime(Vec<u8>),
    // Just a regular inline array
    Array(Vec<Self>),
    // Table defined by a table header. This is immutable aside from being able to add subtables
    Table(Table),
    // Super table created when parsing a subtable header. This can still be explicitly defined
    // later turning it into a `Table`
    UndefinedTable(Table),
    // A table defined by dotted keys. This can be freely added to by other dotted keys
    DottedKeyTable(Table),
    // Inline table
    InlineTable(Table),
    // Array of tables
    ArrayOfTables(Vec<Table>),
}

impl Value {
    #[inline]
    pub const fn typ(&self) -> Type {
        match *self {
            Self::String(_) => Type::String,
            Self::Integer(_) | Self::BinaryInt(_) | Self::OctalInt(_) | Self::HexInt(_) => {
                Type::Integer
            }
            Self::Float(_) | Self::SpecialFloat(_) => Type::Float,
            Self::Boolean(_) => Type::Boolean,
            Self::OffsetDatetime(_)
            | Self::LocalDatetime(_)
            | Self::LocalDate(_)
            | Self::LocalTime(_) => Type::Datetime,
            Self::Array(_) | Self::ArrayOfTables(_) => Type::Array,
            Self::Table(_)
            | Self::InlineTable(_)
            | Self::UndefinedTable(_)
            | Self::DottedKeyTable(_) => Type::Table,
        }
    }
}

pub(super) type Table = std::collections::HashMap<String, Value>;

#[derive(Debug)]
struct Key {
    pub path: Vec<String>, // The path up to the last '.' for dotted keys, otherwise empty
    pub name: String,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for p in &self.path {
            f.write_str(p)?;
            f.write_str(".")?;
        }
        f.write_str(&self.name)
    }
}

#[derive(Debug)]
enum HeaderKind {
    Table,
    Array,
}

#[derive(Debug)]
enum Line {
    TableHeader { key: Key, kind: HeaderKind },
    KeyValuePair { key: Key, value: Value },
    Empty, // Blank line or comment
}

#[derive(Debug)]
pub(super) struct Parser<'de> {
    reader: Reader<'de>,
    line: &'de [u8],
}

impl<'de> Parser<'de> {
    #[must_use]
    #[inline]
    pub fn from_str(str: &'de str) -> Self {
        Self {
            reader: Reader::from_str(str),
            line: b"",
        }
    }

    #[must_use]
    #[inline]
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        Self {
            reader: Reader::from_slice(bytes),
            line: b"",
        }
    }
}

impl Parser<'_> {
    pub fn parse(&mut self) -> Result<Value> {
        let mut root = Table::with_capacity(10);

        // The currently opened table
        let mut table = &mut root;
        // The path to the currently opened table (used for error messages)
        let mut table_path = Key {
            path: vec![],
            name: "root table".to_string(),
        };

        while let Some(line) = self.parse_line()? {
            match line {
                Line::TableHeader { key, kind } => {
                    let parent = root
                        .get_subtable(&key.path)
                        .ok_or_else(|| ErrorKind::InvalidTableHeader(key.to_string().into()))?;

                    table = match kind {
                        HeaderKind::Table => parent.insert_table(key.name.clone()),
                        HeaderKind::Array => parent.append_array_of_tables(key.name.clone()),
                    }
                    .ok_or_else(|| ErrorKind::InvalidTableHeader(key.to_string().into()))?;

                    table_path = key;
                }
                Line::KeyValuePair { key, value } => {
                    let subtable = table.get_dotted_subtable(&key.path, true).ok_or_else(|| {
                        ErrorKind::InvalidKeyPath(
                            key.to_string().into(),
                            table_path.to_string().into(),
                        )
                    })?;

                    // Check if the key is already present
                    if subtable.contains_key(&key.name) {
                        return Err(ErrorKind::DuplicateKey(
                            key.to_string().into(),
                            table_path.to_string().into(),
                        )
                        .into());
                    }
                    subtable.insert(key.name.clone(), value);
                }
                Line::Empty => {}
            }
        }

        Ok(Value::Table(root))
    }

    fn parse_line(&mut self) -> Result<Option<Line>> {
        if self.next_line().is_none() {
            return Ok(None);
        }

        self.skip_whitespace();

        let result = match *self.line {
            // Array header
            [b'[', b'[', ref rest @ ..] => {
                self.line = rest;
                let key = self.parse_array_header()?;
                Line::TableHeader {
                    key,
                    kind: HeaderKind::Array,
                }
            }
            // Table header
            [b'[', ref rest @ ..] => {
                self.line = rest;
                let key = self.parse_table_header()?;
                Line::TableHeader {
                    key,
                    kind: HeaderKind::Table,
                }
            }
            // Key/value pair
            [b, ..] if b.is_toml_word() || b == b'"' || b == b'\'' => {
                let (key, value) = self.parse_key_value_pair()?;
                Line::KeyValuePair { key, value }
            }
            // Anything else should be comments or whitespace, or errors (handled below)
            _ => Line::Empty,
        };

        // Expect whitespace/comments after a line
        self.skip_whitespace();
        self.skip_comment()?;

        // Anything left unparsed at this point is unexpected/illegal
        if let Some(&b) = self.line.first() {
            return Err(if b.is_toml_legal() {
                ErrorKind::ExpectedToken("end of line".into()).into()
            } else {
                ErrorKind::IllegalChar(b).into()
            });
        }

        Ok(Some(result))
    }

    fn parse_array_header(&mut self) -> Result<Key> {
        self.skip_whitespace();
        let key = self.parse_dotted_key()?;

        self.skip_whitespace();
        if let Some(rest) = self.line.strip_prefix(b"]]") {
            self.line = rest;
            Ok(key)
        } else {
            Err(ErrorKind::ExpectedToken("]] after dotted key".into()).into())
        }
    }

    fn parse_table_header(&mut self) -> Result<Key> {
        self.skip_whitespace();
        let key = self.parse_dotted_key()?;

        self.skip_whitespace();
        if let Some(rest) = self.line.strip_prefix(b"]") {
            self.line = rest;
            Ok(key)
        } else {
            Err(ErrorKind::ExpectedToken("] after dotted key".into()).into())
        }
    }

    fn parse_key_value_pair(&mut self) -> Result<(Key, Value)> {
        let path = self.parse_dotted_key()?;

        // Whitespace should already have been consumed by parse_dotted_key looking for another '.'
        if let Some(rest) = self.line.strip_prefix(b"=") {
            self.line = rest;
        } else {
            return Err(ErrorKind::ExpectedToken("= after key".into()).into());
        }
        self.skip_whitespace();

        let value = self.parse_value()?;

        Ok((path, value))
    }

    fn parse_dotted_key(&mut self) -> Result<Key> {
        let mut path = vec![self.parse_key()?];

        self.skip_whitespace();

        while let Some(rest) = self.line.strip_prefix(b".") {
            self.line = rest;
            self.skip_whitespace();
            path.push(self.parse_key()?);
            self.skip_whitespace();
        }

        let name = path.pop().unwrap_or_else(|| unreachable!());

        Ok(Key { path, name })
    }

    fn parse_key(&mut self) -> Result<String> {
        match *self.line {
            [b'"', b'"', b'"', ..] | [b'\'', b'\'', b'\'', ..] => {
                // multiline strings are invalid as keys
                Err(ErrorKind::ExpectedToken("key".into()).into())
            }
            [b'"', ref rest @ ..] => {
                self.line = rest;
                self.parse_basic_str()
            }
            [b'\'', ref rest @ ..] => {
                self.line = rest;
                self.parse_literal_str()
            }
            _ => self.parse_bare_key(),
        }
    }

    fn parse_bare_key(&mut self) -> Result<String> {
        let idx = self
            .line
            .iter()
            .position(|b| !b.is_toml_word())
            .unwrap_or(self.line.len());
        let (key, rest) = self.line.split_at(idx);

        if key.is_empty() {
            Err(ErrorKind::ExpectedToken("key".into()).into())
        } else {
            let result = str::from_utf8(key)
                .map_err(|_| ErrorKind::InvalidEncoding)?
                .to_string();
            self.line = rest;
            Ok(result)
        }
    }

    fn parse_value(&mut self) -> Result<Value> {
        match *self.line {
            // String
            [b'"' | b'\'', ..] => self.parse_string().map(Value::String),
            // Boolean
            [b't' | b'f', ..] => self.parse_bool().map(Value::Boolean),
            // Digit could mean either number or datetime
            [b'0'..=b'9', ..] => self.parse_number_or_datetime(),
            // Number
            [b'+' | b'-', ch, ..] if ch.is_ascii_digit() => self.parse_number_decimal(),
            // Special float
            [b'+' | b'-', b'i' | b'n', ..] | [b'i' | b'n', ..] => {
                self.parse_number_special().map(Value::SpecialFloat)
            }
            // Invalid
            [b'+' | b'-', ..] => Err(ErrorKind::InvalidNumber("missing digits".into()).into()),
            // Array
            [b'[', ref rest @ ..] => {
                // We consume the opening delimiter
                self.line = rest;
                self.parse_array().map(Value::Array)
            }
            // Table
            [b'{', ref rest @ ..] => {
                // We consume the opening delimiter
                self.line = rest;
                self.parse_inline_table().map(Value::InlineTable)
            }
            [ch, ..] if !ch.is_toml_legal() => Err(ErrorKind::IllegalChar(ch).into()),
            _ => Err(ErrorKind::ExpectedToken("a value".into()).into()),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        match *self.line {
            [b'"', b'"', b'"', ref rest @ ..] => {
                self.line = rest;
                self.parse_multiline_basic_str()
            }
            [b'"', ref rest @ ..] => {
                self.line = rest;
                self.parse_basic_str()
            }
            [b'\'', b'\'', b'\'', ref rest @ ..] => {
                self.line = rest;
                self.parse_multiline_literal_str()
            }
            [b'\'', ref rest @ ..] => {
                self.line = rest;
                self.parse_literal_str()
            }
            _ => Err(ErrorKind::ExpectedToken("string".into()).into()),
        }
    }

    fn parse_basic_str(&mut self) -> Result<String> {
        let mut str = String::new();

        loop {
            let orig = self.line;
            let idx = orig
                .iter()
                .position(|b| !b.is_toml_basic_str_sans_escapes())
                .ok_or(ErrorKind::UnterminatedString)?;
            self.line = &orig[idx + 1..];

            str.push_str(str::from_utf8(&orig[..idx]).map_err(|_| ErrorKind::InvalidEncoding)?);
            match orig[idx] {
                b'\\' => str.push(self.parse_escape_seq()?),
                b'"' => break Ok(str),
                char => break Err(ErrorKind::IllegalChar(char).into()),
            }
        }
    }

    fn parse_multiline_basic_str(&mut self) -> Result<String> {
        // Newlines after the first """ are ignored. So if line is empty just populate the next one
        if self.line.is_empty() {
            self.next_line().ok_or(ErrorKind::UnterminatedString)?;
        }

        let mut str = String::new();

        loop {
            // Find the first char we can't directly copy
            let idx = self
                .line
                .iter()
                .position(|b| !b.is_toml_multiline_basic_str_sans_escapes())
                .unwrap_or(self.line.len());

            // Copy everything until idx
            str.push_str(
                str::from_utf8(&self.line[..idx]).map_err(|_| ErrorKind::InvalidEncoding)?,
            );
            self.line = &self.line[idx..];

            match *self.line {
                // Trailing '\'
                [b'\\', ref rest @ ..] if rest.iter().all(TomlByte::is_toml_whitespace) => loop {
                    self.next_line().ok_or(ErrorKind::UnterminatedString)?;
                    self.skip_whitespace();
                    if !self.line.is_empty() {
                        break;
                    }
                },
                // Regular escape seq
                [b'\\', ref rest @ ..] => {
                    self.line = rest;
                    str.push(self.parse_escape_seq()?);
                }
                // End of string """
                [b'"', b'"', b'"', ref rest @ ..] => {
                    self.line = rest;

                    // We can have up to 5 '"'s, 2 quotes inside the string right before the 3
                    // which close the string. So we check for 2 additional '"'s and push them
                    if let Some(rest) = self.line.strip_prefix(b"\"") {
                        self.line = rest;
                        str.push('"');
                        if let Some(rest) = self.line.strip_prefix(b"\"") {
                            self.line = rest;
                            str.push('"');
                        }
                    }

                    break Ok(str);
                }
                // Just a regular '"'
                [b'"', ref rest @ ..] => {
                    self.line = rest;
                    str.push('"');
                }
                // Any other char is illegal
                [char, ..] => break Err(ErrorKind::IllegalChar(char).into()),
                // End of line
                [] => {
                    str.push('\n');
                    self.next_line().ok_or(ErrorKind::UnterminatedString)?;
                }
            }
        }
    }

    fn parse_literal_str(&mut self) -> Result<String> {
        let orig = self.line;
        let idx = orig
            .iter()
            .position(|b| !b.is_toml_literal_str())
            .ok_or(ErrorKind::UnterminatedString)?;
        self.line = &orig[idx + 1..];

        let result = str::from_utf8(&orig[..idx]).map_err(|_| ErrorKind::InvalidEncoding)?;
        match orig[idx] {
            b'\'' => Ok(result.to_string()),
            char => Err(ErrorKind::IllegalChar(char).into()),
        }
    }

    fn parse_multiline_literal_str(&mut self) -> Result<String> {
        // Newlines after the first ''' are ignored. So if line is empty just populate the next one
        if self.line.is_empty() {
            self.next_line().ok_or(ErrorKind::UnterminatedString)?;
        }

        let mut str = String::new();

        loop {
            // Find the first char we can't directly copy
            let idx = self
                .line
                .iter()
                .position(|b| !b.is_toml_multiline_literal_str())
                .unwrap_or(self.line.len());

            // Copy everything until idx
            str.push_str(
                str::from_utf8(&self.line[..idx]).map_err(|_| ErrorKind::InvalidEncoding)?,
            );
            self.line = &self.line[idx..];

            match *self.line {
                // End of string '''
                [b'\'', b'\'', b'\'', ref rest @ ..] => {
                    self.line = rest;

                    // We can have up to 5 '\''s, 2 quotes inside the string right before the 3
                    // which close the string. So we check for 2 additional '\''s and push them
                    if let Some(rest) = self.line.strip_prefix(b"'") {
                        self.line = rest;
                        str.push('\'');
                        if let Some(rest) = self.line.strip_prefix(b"'") {
                            self.line = rest;
                            str.push('\'');
                        }
                    }

                    break Ok(str);
                }
                // Just a regular '\''
                [b'\'', ref rest @ ..] => {
                    self.line = rest;
                    str.push('\'');
                }
                // Any other char is illegal
                [char, ..] => break Err(ErrorKind::IllegalChar(char).into()),
                // End of line
                [] => {
                    str.push('\n');
                    self.next_line().ok_or(ErrorKind::UnterminatedString)?;
                }
            }
        }
    }

    fn parse_escape_seq(&mut self) -> Result<char> {
        let Some((&esc, rest)) = self.line.split_first() else {
            return Err(ErrorKind::UnterminatedString.into());
        };
        let orig = self.line;
        self.line = rest;

        match esc {
            b'b' => Ok('\x08'),
            b't' => Ok('\t'),
            b'n' => Ok('\n'),
            b'f' => Ok('\x0c'),
            b'r' => Ok('\r'),
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'u' => {
                let (bytes, rest) = if rest.len() >= 4 {
                    rest.split_at(4)
                } else {
                    return Err(ErrorKind::UnterminatedString.into());
                };
                let str = str::from_utf8(bytes).map_err(|_| ErrorKind::InvalidEncoding)?;
                let result = u32::from_str_radix(str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| ErrorKind::InvalidEscape(format!("\\u{str}").into()))?;
                self.line = rest;
                Ok(result)
            }
            b'U' => {
                let (bytes, rest) = if rest.len() >= 8 {
                    rest.split_at(8)
                } else {
                    return Err(ErrorKind::UnterminatedString.into());
                };
                let str = str::from_utf8(bytes).map_err(|_| ErrorKind::InvalidEncoding)?;
                let result = u32::from_str_radix(str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| ErrorKind::InvalidEscape(format!("\\U{str}").into()))?;
                self.line = rest;
                Ok(result)
            }
            _ => {
                // We want a char here, not just a byte
                let char = reader::utf8_len(esc)
                    .and_then(|len| str::from_utf8(&orig[..len]).ok())
                    .and_then(|l| l.chars().next())
                    .ok_or(ErrorKind::InvalidEncoding)?;

                Err(ErrorKind::InvalidEscape(format!("\\{char}").into()).into())
            }
        }
    }

    fn parse_bool(&mut self) -> Result<bool> {
        if let Some(rest) = self.line.strip_prefix(b"true") {
            self.line = rest;
            Ok(true)
        } else if let Some(rest) = self.line.strip_prefix(b"false") {
            self.line = rest;
            Ok(false)
        } else {
            Err(ErrorKind::ExpectedToken("true/false".into()).into())
        }
    }

    // Parses anything that starts with a digit. Does not parse special floats or +/- values
    fn parse_number_or_datetime(&mut self) -> Result<Value> {
        match *self.line {
            // Hex literal starts with "0x"
            [b'0', b'x', ref rest @ ..] => {
                self.line = rest;
                self.parse_digits(u8::is_ascii_hexdigit).map(Value::HexInt)
            }
            // Octal literal starts with "0o"
            [b'0', b'o', ref rest @ ..] => {
                self.line = rest;
                self.parse_digits(|&b| matches!(b, b'0'..=b'7'))
                    .map(Value::OctalInt)
            }
            // Binary literal starts with "0b"
            [b'0', b'b', ref rest @ ..] => {
                self.line = rest;
                self.parse_digits(|&b| matches!(b, b'0' | b'1'))
                    .map(Value::BinaryInt)
            }
            // LocalTime has a ':' at index 2
            #[cfg(feature = "datetime")]
            [_, _, b':', ..] => {
                let idx = self
                    .line
                    .iter()
                    .position(|b| !b.is_toml_datetime())
                    .unwrap_or(self.line.len());
                let result = self.line[..idx].to_vec();
                self.line = &self.line[idx..];
                Ok(Value::LocalTime(result))
            }
            // OffsetDateTime, LocalDateTime, or LocalDate have '-' at index 4
            // Also need to check for only digits before to rule out float literals (e.g. 120e-2)
            #[cfg(feature = "datetime")]
            [b'0'..=b'9', b'0'..=b'9', b'0'..=b'9', b'0'..=b'9', b'-', ..] => {
                let end = self
                    .line
                    .iter()
                    .position(|b| !b.is_toml_datetime())
                    .unwrap_or(self.line.len());

                // If we have a 'T' we already have date and time
                let (end, time) = if let Some(t) = self.line[..end]
                    .iter()
                    .position(|&b| matches!(b, b'T' | b't'))
                {
                    (end, t + 1)
                }
                // If we don't have a 'T' we might have a space-delimited datetime of which
                // `value` is the first half, so check for space followed by a digit
                else if matches!(self.line[end..], [b' ', b'0'..=b'9', ..]) {
                    // Discard the space
                    let idx = end + 1;
                    let time = idx;

                    // Get the time
                    let end = self.line[idx..]
                        .iter()
                        .position(|b| !b.is_toml_datetime())
                        .map_or(self.line.len(), |i| idx + i);

                    (end, time)
                }
                // Else we definitely just have a LocalDate
                else {
                    // Consume the date and return the string
                    let result = self.line[..end].to_vec();
                    self.line = &self.line[end..];
                    return Ok(Value::LocalDate(result));
                };

                // Consume the date and return the string
                let result = self.line[..end].to_vec();
                self.line = &self.line[end..];

                // Check for an offset to return correct offset/local type
                if result[time..]
                    .iter()
                    .any(|&b| matches!(b, b'z' | b'Z' | b'+' | b'-'))
                {
                    Ok(Value::OffsetDatetime(result))
                } else {
                    Ok(Value::LocalDatetime(result))
                }
            }
            // Just a plain ol' decimal
            [..] => self.parse_number_decimal(),
        }
    }

    fn parse_number_decimal(&mut self) -> Result<Value> {
        let mut float = false;
        let mut buf = Vec::new();

        // Check for the sign
        if let Some((&b @ (b'+' | b'-'), rest)) = self.line.split_first() {
            buf.push(b);
            self.line = rest;
        }

        // Check for invalid leading zero, but allow "0", "0.123", etc
        if matches!(
            *self.line,
            [b'0', b'0'..=b'9', ..] | [b'0', b'_', b'0'..=b'9', ..]
        ) {
            return Err(ErrorKind::InvalidNumber("leading zero".into()).into());
        }
        // Parse the integer portion
        self.parse_digits_into(u8::is_ascii_digit, &mut buf)?;

        if let Some(rest) = self.line.strip_prefix(b".") {
            float = true;

            buf.push(b'.');
            self.line = rest;

            // Parse the fractional portion
            self.parse_digits_into(u8::is_ascii_digit, &mut buf)?;
        }

        if let Some((&b'e' | &b'E', rest)) = self.line.split_first() {
            float = true;

            buf.push(b'e');
            self.line = rest;

            // Check for the sign
            if let Some((&b @ (b'+' | b'-'), rest)) = self.line.split_first() {
                buf.push(b);
                self.line = rest;
            }

            // Parse the exponent portion
            self.parse_digits_into(u8::is_ascii_digit, &mut buf)?;
        }

        Ok(if float {
            Value::Float(buf)
        } else {
            Value::Integer(buf)
        })
    }

    fn parse_digits(&mut self, is_digit: fn(&u8) -> bool) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.parse_digits_into(is_digit, &mut buf)?;
        Ok(buf)
    }

    fn parse_digits_into(&mut self, is_digit: fn(&u8) -> bool, buf: &mut Vec<u8>) -> Result<()> {
        // Find the first non-digit char
        let idx = self
            .line
            .iter()
            .position(|b| !is_digit(b))
            .unwrap_or(self.line.len());

        if idx == 0 {
            return Err(ErrorKind::InvalidNumber(
                match self.line.get(idx).copied() {
                    Some(b'_') => "leading underscore",
                    _ => "no digits",
                }
                .into(),
            )
            .into());
        }

        buf.extend_from_slice(&self.line[..idx]);
        self.line = &self.line[idx..];

        while let Some(rest) = self.line.strip_prefix(b"_") {
            self.line = rest;

            let idx = self
                .line
                .iter()
                .position(|b| !is_digit(b))
                .unwrap_or(self.line.len());

            if idx == 0 {
                return Err(ErrorKind::InvalidNumber(
                    match self.line.get(idx).copied() {
                        Some(b'_') => "double underscore",
                        _ => "trailing underscore",
                    }
                    .into(),
                )
                .into());
            }

            buf.extend_from_slice(&self.line[..idx]);
            self.line = &self.line[idx..];
        }

        Ok(())
    }

    fn parse_number_special(&mut self) -> Result<SpecialFloat> {
        match *self.line {
            [b'-', ref rest @ ..] => match *rest {
                [b'i', b'n', b'f', ref rest @ ..] => Ok((SpecialFloat::NegInfinity, rest)),
                [b'n', b'a', b'n', ref rest @ ..] => Ok((SpecialFloat::NegNan, rest)),
                _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
            },
            [b'+', ref rest @ ..] | ref rest => match *rest {
                [b'i', b'n', b'f', ref rest @ ..] => Ok((SpecialFloat::Infinity, rest)),
                [b'n', b'a', b'n', ref rest @ ..] => Ok((SpecialFloat::Nan, rest)),
                _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
            },
        }
        .map(|(result, rest)| {
            self.line = rest;
            result
        })
    }

    fn parse_array(&mut self) -> Result<Vec<Value>> {
        fn skip_comments_and_whitespace(slf: &mut Parser<'_>) -> Result<()> {
            slf.skip_whitespace();
            slf.skip_comment()?;
            while slf.line.is_empty() {
                slf.next_line().ok_or(ErrorKind::UnterminatedString)?;
                slf.skip_whitespace();
                slf.skip_comment()?;
            }
            Ok(())
        }

        let mut result = vec![];

        loop {
            skip_comments_and_whitespace(self)?;

            if let Some(rest) = self.line.strip_prefix(b"]") {
                self.line = rest;
                break; // End of array
            }

            result.push(self.parse_value()?);

            skip_comments_and_whitespace(self)?;

            if let Some(rest) = self.line.strip_prefix(b"]") {
                self.line = rest;
                break; // End of array
            }

            if let Some(rest) = self.line.strip_prefix(b",") {
                self.line = rest;
            } else {
                return Err(ErrorKind::ExpectedToken(", or ] after value in array".into()).into());
            }
        }

        Ok(result)
    }

    fn parse_inline_table(&mut self) -> Result<Table> {
        let mut result = Table::with_capacity(10);

        self.skip_whitespace();

        if let Some(rest) = self.line.strip_prefix(b"}") {
            self.line = rest;
            return Ok(result); // End of table
        }

        loop {
            let (key, value) = self.parse_key_value_pair()?;

            // Navigate to the subtable
            let subtable = result
                .get_dotted_subtable(&key.path, false)
                .ok_or_else(|| {
                    ErrorKind::InvalidKeyPath(key.to_string().into(), "inline table".into())
                })?;

            // Check if the key is already present
            if subtable.contains_key(&key.name) {
                return Err(
                    ErrorKind::DuplicateKey(key.to_string().into(), "inline table".into()).into(),
                );
            }
            subtable.insert(key.name.clone(), value);

            self.skip_whitespace();

            if let Some(rest) = self.line.strip_prefix(b"}") {
                self.line = rest;
                break; // End of table
            }

            if let Some(rest) = self.line.strip_prefix(b",") {
                self.line = rest;
                self.skip_whitespace();
            } else {
                return Err(ErrorKind::ExpectedToken(
                    ", or } after key/value pair in inline table".into(),
                )
                .into());
            }
        }

        Ok(result)
    }

    fn skip_whitespace(&mut self) {
        let idx = self
            .line
            .iter()
            .position(|b| !b.is_toml_whitespace())
            .unwrap_or(self.line.len());
        self.line = &self.line[idx..];
    }

    fn skip_comment(&mut self) -> Result<()> {
        if let Some(rest) = self.line.strip_prefix(b"#") {
            // Only validate comments without feature = "fast"
            if cfg!(not(feature = "fast")) {
                // validate UTF-8
                _ = str::from_utf8(rest).map_err(|_| ErrorKind::InvalidEncoding)?;
                // Check for any invalid characters in the comment
                if let Some(ch) = rest.iter().copied().find(|ch| !ch.is_toml_comment()) {
                    return Err(ErrorKind::IllegalChar(ch).into());
                }
            }
            self.line = &self.line[self.line.len()..];
        }

        Ok(())
    }

    fn next_line(&mut self) -> Option<()> {
        self.line = self.reader.next_line()?;
        Some(())
    }
}

trait TomlTable {
    fn get_subtable(&mut self, path: &[String]) -> Option<&mut Self>;
    fn get_dotted_subtable(&mut self, path: &[String], allow_undefined: bool) -> Option<&mut Self>;
    fn insert_table(&mut self, name: String) -> Option<&mut Self>;
    fn append_array_of_tables(&mut self, name: String) -> Option<&mut Self>;
}

impl TomlTable for Table {
    fn get_subtable(&mut self, path: &[String]) -> Option<&mut Self> {
        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::UndefinedTable(Self::with_capacity(10)));
            match *entry {
                Value::Table(ref mut subtable)
                | Value::UndefinedTable(ref mut subtable)
                | Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                Value::ArrayOfTables(ref mut array) => {
                    // we never insert an empty array of tables, so this should always be some
                    debug_assert!(!array.is_empty());
                    array.last_mut()
                }
                _ => None,
            }
        })
    }

    fn get_dotted_subtable(&mut self, path: &[String], allow_undefined: bool) -> Option<&mut Self> {
        // Navigate to the table, converting any UndefinedTables to DottedKeyTables
        path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::DottedKeyTable(Self::with_capacity(10)));

            if allow_undefined {
                if let Value::UndefinedTable(ref mut subtable) = *entry {
                    // Pull out the subtable to take ownership of it
                    let subtable = std::mem::take(subtable);
                    // Replace the UndefinedTable with a DottedKeyTable
                    *entry = Value::DottedKeyTable(subtable);
                }
            }

            // Return a mutable reference to the new table
            match *entry {
                Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                _ => None, // Table already exists and is not UndefinedTable
            }
        })
    }

    fn insert_table(&mut self, name: String) -> Option<&mut Self> {
        // Create the table in the parent, or error if a table already exists
        match self.entry(name) {
            Entry::Vacant(entry) => {
                // Create a new Table if it doesn't exist
                let value = entry.insert(Value::Table(Self::with_capacity(10)));
                let Value::Table(ref mut table) = *value else {
                    unreachable!("we just inserted a new table")
                };
                Some(table)
            }
            Entry::Occupied(mut entry) => {
                if let Value::UndefinedTable(ref mut table) = *entry.get_mut() {
                    // Pull out the subtable to take ownership of it
                    let subtable = std::mem::take(table);
                    // Replace the UndefinedTable with a Table
                    entry.insert(Value::Table(subtable));

                    let Value::Table(ref mut table) = *entry.into_mut() else {
                        unreachable!("we just inserted a new table")
                    };
                    Some(table)
                } else {
                    None // Table already exists and is not UndefinedTable
                }
            }
        }
    }

    fn append_array_of_tables(&mut self, name: String) -> Option<&mut Self> {
        // Create the array in the parent, or append if the array already exists
        let value = self
            .entry(name)
            .or_insert_with(|| Value::ArrayOfTables(Vec::with_capacity(16)));

        if let Value::ArrayOfTables(ref mut subarray) = *value {
            // Push a new table to the array, set the current table and key
            subarray.push(Self::with_capacity(10));
            // we just pushed insert a table, so this should always be some
            subarray.last_mut()
        } else {
            // Table already exists and was not an ArrayOfTables
            None
        }
    }
}

trait TomlByte {
    /// If the byte is TOML whitespace (space or tab)
    fn is_toml_whitespace(&self) -> bool;
    /// If the byte is a TOML word (ASCII alphanumeric or hyphen or underscore)
    fn is_toml_word(&self) -> bool;
    /// If the byte is present in a datetime (ASCII numeric, plus, minus, period, colon, T, t, Z, z)
    fn is_toml_datetime(&self) -> bool;
    /// If the byte is valid in a TOML comment (disallows all ASCII control chars except for tab)
    fn is_toml_comment(&self) -> bool;
    /// If the byte is valid in a basic string, excluding the escape character '\'
    fn is_toml_basic_str_sans_escapes(&self) -> bool;
    /// If the byte is valid in a multiline basic string, excluding the escape character '\'
    fn is_toml_multiline_basic_str_sans_escapes(&self) -> bool;
    /// If the byte is valid in a literal string
    fn is_toml_literal_str(&self) -> bool;
    /// If the byte is valid in a multiline literal string
    fn is_toml_multiline_literal_str(&self) -> bool;
    /// If the byte is legal in a TOML document
    fn is_toml_legal(&self) -> bool;
}

impl TomlByte for u8 {
    #[inline]
    fn is_toml_whitespace(&self) -> bool {
        matches!(*self, b'\t' | b' ')
    }

    #[inline]
    fn is_toml_word(&self) -> bool {
        matches!(*self, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
    }

    #[inline]
    fn is_toml_datetime(&self) -> bool {
        matches!(
            *self,
            b'0'..=b'9' | b'+' | b'-' | b'.' | b':' | b'T' | b't' | b'Z' | b'z'
        )
    }

    #[inline]
    fn is_toml_comment(&self) -> bool {
        matches!(*self, 0x09 | 0x20..=0x7e | 0x80..)
    }

    #[inline]
    fn is_toml_basic_str_sans_escapes(&self) -> bool {
        matches!(*self, 0x09 | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
    }

    #[inline]
    fn is_toml_multiline_basic_str_sans_escapes(&self) -> bool {
        matches!(*self, 0x09 | 0x0a | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
    }

    #[inline]
    fn is_toml_literal_str(&self) -> bool {
        matches!(*self, 0x09 | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
    }

    #[inline]
    fn is_toml_multiline_literal_str(&self) -> bool {
        matches!(*self, 0x09 | 0x0a | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
    }

    #[inline]
    fn is_toml_legal(&self) -> bool {
        matches!(*self, 0x09 | 0x0a | 0x0d | 0x20..=0x7e | 0x80..)
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use maplit::hashmap;

    use super::*;
    use crate::de::Error;

    fn start_parser(bytes: &[u8]) -> Parser<'_> {
        let mut reader = Reader::from_slice(bytes);
        let line = reader.next_line().unwrap_or(b"");

        Parser { reader, line }
    }

    #[test]
    fn type_to_str() {
        assert_eq!(Type::String.to_str(), "string");
        assert_eq!(Type::Integer.to_str(), "integer");
        assert_eq!(Type::Float.to_str(), "float");
        assert_eq!(Type::Boolean.to_str(), "boolean");
        assert_eq!(Type::Datetime.to_str(), "datetime");
        assert_eq!(Type::Array.to_str(), "array");
        assert_eq!(Type::Table.to_str(), "table");
    }

    #[test]
    fn type_display() {
        assert_eq!(Type::String.to_string(), "string");
        assert_eq!(Type::Integer.to_string(), "integer");
        assert_eq!(Type::Float.to_string(), "float");
        assert_eq!(Type::Boolean.to_string(), "boolean");
        assert_eq!(Type::Datetime.to_string(), "datetime");
        assert_eq!(Type::Array.to_string(), "array");
        assert_eq!(Type::Table.to_string(), "table");
    }

    #[test]
    fn unexpected_from_type() {
        assert_eq!(
            de::Unexpected::from(Type::String),
            de::Unexpected::Other("string")
        );
        assert_eq!(
            de::Unexpected::from(Type::Integer),
            de::Unexpected::Other("integer")
        );
        assert_eq!(
            de::Unexpected::from(Type::Float),
            de::Unexpected::Other("float")
        );
        assert_eq!(
            de::Unexpected::from(Type::Boolean),
            de::Unexpected::Other("boolean")
        );
        assert_eq!(
            de::Unexpected::from(Type::Datetime),
            de::Unexpected::Other("datetime")
        );
        assert_eq!(
            de::Unexpected::from(Type::Array),
            de::Unexpected::Other("array")
        );
        assert_eq!(
            de::Unexpected::from(Type::Table),
            de::Unexpected::Other("table")
        );
    }

    #[test]
    fn value_type() {
        assert_eq!(Value::String("foo".into()).typ(), Type::String);
        assert_eq!(Value::Integer(b"123".to_vec()).typ(), Type::Integer);
        assert_eq!(Value::BinaryInt(b"123".to_vec()).typ(), Type::Integer);
        assert_eq!(Value::OctalInt(b"123".to_vec()).typ(), Type::Integer);
        assert_eq!(Value::HexInt(b"123".to_vec()).typ(), Type::Integer);
        assert_eq!(Value::Float(b"123".to_vec()).typ(), Type::Float);
        assert_eq!(
            Value::SpecialFloat(SpecialFloat::Infinity).typ(),
            Type::Float
        );
        assert_eq!(Value::Boolean(true).typ(), Type::Boolean);
        assert_eq!(Value::OffsetDatetime(Vec::new()).typ(), Type::Datetime);
        assert_eq!(Value::LocalDatetime(Vec::new()).typ(), Type::Datetime);
        assert_eq!(Value::LocalDate(Vec::new()).typ(), Type::Datetime);
        assert_eq!(Value::LocalTime(Vec::new()).typ(), Type::Datetime);
        assert_eq!(Value::Array(vec![]).typ(), Type::Array);
        assert_eq!(Value::ArrayOfTables(vec![]).typ(), Type::Array);
        assert_eq!(Value::Table(Table::new()).typ(), Type::Table);
        assert_eq!(Value::InlineTable(Table::new()).typ(), Type::Table);
        assert_eq!(Value::UndefinedTable(Table::new()).typ(), Type::Table);
        assert_eq!(Value::DottedKeyTable(Table::new()).typ(), Type::Table);
    }

    #[test]
    fn key_display() {
        let key = Key {
            path: vec!["a".to_string(), "b".to_string()],
            name: "c".to_string(),
        };
        assert_eq!(key.to_string(), "a.b.c");

        let key = Key {
            path: vec!["a".to_string()],
            name: "b".to_string(),
        };
        assert_eq!(key.to_string(), "a.b");

        let key = Key {
            path: vec![],
            name: "a".to_string(),
        };
        assert_eq!(key.to_string(), "a");
    }

    #[test]
    fn parser_from_str() {
        let mut parser = Parser::from_str("foo = 123");
        assert_matches!(parser.line, b"");
        // Can't check the reader directly, so check it line by line
        assert_matches!(parser.reader.next_line(), Some(b"foo = 123"));
        assert_matches!(parser.reader.next_line(), None);
    }

    #[test]
    fn parser_from_slice() {
        let mut parser = Parser::from_slice(b"foo = 123");
        assert_matches!(parser.line, b"");
        // Can't check the reader directly, so check it line by line
        assert_matches!(parser.reader.next_line(), Some(b"foo = 123"));
        assert_matches!(parser.reader.next_line(), None);
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn parser_parse() {
        let mut parser = Parser::from_slice(b"a = 1\nb = 2");
        assert_matches!(
            parser.parse(),
            Ok(Value::Table(t)) if t == hashmap! {
                "a".into() => Value::Integer(b"1".to_vec()),
                "b".into() => Value::Integer(b"2".to_vec()),
            }
        );

        let mut parser = Parser::from_slice(b"a = 1\r\nb = 2");
        assert_matches!(
            parser.parse(),
            Ok(Value::Table(t)) if t == hashmap! {
                "a".into() => Value::Integer(b"1".to_vec()),
                "b".into() => Value::Integer(b"2".to_vec()),
            }
        );

        let mut parser = Parser::from_slice(indoc! {br#"
            # This is a TOML document.

            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates

            [database]
            server = "192.168.1.1"
            ports = [ 8000, 8001, 8002 ]
            connection_max = 5000
            enabled = true

            [servers]

                # Indentation (tabs and/or spaces) is allowed but not required
                [servers.alpha]
                ip = "10.0.0.1"
                dc = "eqdc10"

                [servers.beta]
                ip = "10.0.0.2"
                dc = "eqdc10"

            [clients]

            # Line breaks are OK when inside arrays
            hosts = [
                "alpha",
                "omega"
            ]

            [[clients.data]]
                value = ["gamma", "delta"]

            [[clients.data]]
                value = [1, 2]
        "#});

        assert_matches!(
            parser.parse(),
            Ok(Value::Table(t)) if t == hashmap! {
                "title".into() => Value::String("TOML Example".into()),
                "owner".into() => Value::Table(hashmap! {
                    "name".into() => Value::String("Tom Preston-Werner".into()),
                    "dob".into() => Value::OffsetDatetime(b"1979-05-27T07:32:00-08:00".to_vec()),
                }),
                "database".into() => Value::Table(hashmap! {
                    "server".into() => Value::String("192.168.1.1".into()),
                    "ports".into() => Value::Array(vec![
                        Value::Integer(b"8000".to_vec()),
                        Value::Integer(b"8001".to_vec()),
                        Value::Integer(b"8002".to_vec()),
                    ]),
                    "connection_max".into() => Value::Integer(b"5000".to_vec()),
                    "enabled".into() => Value::Boolean(true),
                }),
                "servers".into() => Value::Table(hashmap! {
                    "alpha".into() => Value::Table(hashmap! {
                        "ip".into() => Value::String("10.0.0.1".into()),
                        "dc".into() => Value::String("eqdc10".into()),
                    }),
                    "beta".into() => Value::Table(hashmap! {
                        "ip".into() => Value::String("10.0.0.2".into()),
                        "dc".into() => Value::String("eqdc10".into()),
                    }),
                }),
                "clients".into() => Value::Table(hashmap! {
                    "hosts".into() => Value::Array(vec![
                        Value::String("alpha".into()),
                        Value::String("omega".into()),
                    ]),
                    "data".into() => Value::ArrayOfTables(vec![
                        hashmap! {
                            "value".into() => Value::Array(vec![
                                Value::String("gamma".into()),
                                Value::String("delta".into()),
                            ]),
                        },
                        hashmap! {
                            "value".into() => Value::Array(vec![
                                Value::Integer(b"1".to_vec()),
                                Value::Integer(b"2".to_vec()),
                            ]),
                        }
                    ]),
                }),
            }
        );
    }

    #[test]
    fn parser_parse_invalid() {
        let mut parser = Parser::from_slice(indoc! {br"
            a = 123
            a = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::DuplicateKey(..))));

        let mut parser = Parser::from_slice(indoc! {br"
            a = 123

            [a]
            b = 456
        "});
        assert_matches!(
            parser.parse(),
            Err(Error(ErrorKind::InvalidTableHeader(..)))
        );

        let mut parser = Parser::from_slice(indoc! {br"
            a = 123

            [[a]]
            b = 456
        "});
        assert_matches!(
            parser.parse(),
            Err(Error(ErrorKind::InvalidTableHeader(..)))
        );

        let mut parser = Parser::from_slice(indoc! {br"
            a = 123
            a.b = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::InvalidKeyPath(..))));

        let mut parser = Parser::from_slice(indoc! {br"
            [a.b]
            c = 123

            [a]
            b.d = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::InvalidKeyPath(..))));

        let mut parser = Parser::from_slice(indoc! {br"
            [table]
            a.b = 123
            a.b = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::DuplicateKey(..))));

        let mut parser = Parser::from_slice(b"a = 123 $");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::ExpectedToken(..))));

        let mut parser = Parser::from_slice(b"a = 123 \0");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::IllegalChar(..))));

        let mut parser = Parser::from_slice(b"$");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::ExpectedToken(..))));

        let mut parser = Parser::from_slice(b"\0");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::IllegalChar(..))));

        let mut parser = Parser::from_slice(b"a = 1\rb = 2");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::ExpectedToken(..))));
    }

    #[test]
    fn parser_parse_line() {
        let mut parser = Parser::from_slice(b"[[a]]");
        assert_matches!(
            parser.parse_line(),
            Ok(Some(Line::TableHeader { key, kind: HeaderKind::Array })) if key.to_string() == "a"
        );

        let mut parser = Parser::from_slice(b"[a]");
        assert_matches!(
            parser.parse_line(),
            Ok(Some(Line::TableHeader { key, kind: HeaderKind::Table })) if key.to_string() == "a"
        );

        let mut parser = Parser::from_slice(b"a = 1");
        assert_matches!(
            parser.parse_line(),
            Ok(Some(Line::KeyValuePair { key, value })) if key.to_string() == "a" && value == Value::Integer(b"1".to_vec())
        );

        let mut parser = Parser::from_slice(b"'a' = 1");
        assert_matches!(
            parser.parse_line(),
            Ok(Some(Line::KeyValuePair { key, value })) if key.to_string() == "a" && value == Value::Integer(b"1".to_vec())
        );

        let mut parser = Parser::from_slice(br#""a" = 1"#);
        assert_matches!(
            parser.parse_line(),
            Ok(Some(Line::KeyValuePair { key, value })) if key.to_string() == "a" && value == Value::Integer(b"1".to_vec())
        );

        let mut parser = Parser::from_slice(b"\n");
        assert_matches!(parser.parse_line(), Ok(Some(Line::Empty)));

        let mut parser = Parser::from_slice(b"a = 1 # comment");
        assert_matches!(parser.parse_line(), Ok(Some(Line::KeyValuePair { .. })));

        let mut parser = Parser::from_slice(b"");
        assert_matches!(parser.parse_line(), Ok(None));

        let mut parser = Parser::from_slice(b"a = 1 blah");
        assert_matches!(
            parser.parse_line(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_slice(b"a = 1 \0");
        assert_matches!(parser.parse_line(), Err(Error(ErrorKind::IllegalChar(..))));
    }

    #[test]
    fn parser_parse_array_header() {
        let mut parser = start_parser(br#" a .b. "..c"]]"#);
        assert_matches!(parser.parse_array_header(), Ok(s) if s.path == ["a", "b"] && s.name == "..c");

        let mut parser = start_parser(br#""]]""#);
        assert_matches!(
            parser.parse_array_header(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_table_header() {
        let mut parser = start_parser(br#" a .b. "..c"]"#);
        assert_matches!(parser.parse_table_header(), Ok(s) if s.path == ["a", "b"] && s.name == "..c");

        let mut parser = start_parser(br#""]""#);
        assert_matches!(
            parser.parse_table_header(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_key_value_pair() {
        let mut parser = start_parser(br"a = 123");
        assert_matches!(
            parser.parse_key_value_pair(),
            Ok((k, Value::Integer(v))) if k.path.is_empty() && k.name == "a" && &*v == b"123"
        );

        let mut parser = start_parser(br#""a = 123""#);
        assert_matches!(
            parser.parse_key_value_pair(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_dotted_key() {
        let mut parser = start_parser(br#"a .b. "..c""#);
        assert_matches!(parser.parse_dotted_key(), Ok(s) if s.path == ["a", "b"] && s.name == "..c");

        let mut parser = start_parser(b".");
        assert_matches!(
            parser.parse_dotted_key(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"a..b");
        assert_matches!(
            parser.parse_dotted_key(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_key() {
        let mut parser = start_parser(b"abc");
        assert_matches!(parser.parse_key(), Ok(k) if k == "abc");

        let mut parser = start_parser(br#""abc""#);
        assert_matches!(parser.parse_key(), Ok(k) if k == "abc");

        let mut parser = start_parser(b"'abc'");
        assert_matches!(parser.parse_key(), Ok(k) if k == "abc");

        let mut parser = start_parser(br#""""abc""""#);
        assert_matches!(parser.parse_key(), Err(Error(ErrorKind::ExpectedToken(..))));

        let mut parser = start_parser(b"'''abc'''");
        assert_matches!(parser.parse_key(), Err(Error(ErrorKind::ExpectedToken(..))));
    }

    #[test]
    fn parser_parse_bare_key() {
        let mut parser = start_parser(b"abc");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "abc");

        let mut parser = start_parser(b"123");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "123");

        let mut parser = start_parser(b"-");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "-");

        let mut parser = start_parser(b"_");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "_");

        let mut parser = start_parser(b"[key]");
        assert_matches!(
            parser.parse_bare_key(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn parser_parse_value() {
        let mut parser = start_parser(br#""hello""#);
        assert_matches!(parser.parse_value(), Ok(Value::String(s)) if &*s == "hello");

        let mut parser = start_parser(b"true");
        assert_matches!(parser.parse_value(), Ok(Value::Boolean(true)));

        let mut parser = start_parser(b"0.2");
        assert_matches!(parser.parse_value(), Ok(Value::Float(b)) if &*b == b"0.2");

        let mut parser = start_parser(b"0x123abc");
        assert_matches!(parser.parse_value(), Ok(Value::HexInt(v)) if &*v == b"123abc");

        let mut parser = start_parser(b"0001-01-01");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalDate(date)) if date == b"0001-01-01"
        );

        let mut parser = start_parser(b"00:00:00");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalTime(time)) if time == b"00:00:00"
        );

        let mut parser = start_parser(b"0");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(b)) if &*b == b"0");

        let mut parser = start_parser(b"12");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(b)) if &*b == b"12");

        let mut parser = start_parser(b"1234");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(b)) if &*b == b"1234");

        let mut parser = start_parser(b"1234-05-06");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalDate(date)) if date == b"1234-05-06"
        );

        let mut parser = start_parser(b"12:34:56");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalTime(time)) if time == b"12:34:56"
        );

        let mut parser = start_parser(b"-123");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(v)) if &*v == b"-123");

        let mut parser = start_parser(b"+123");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(v)) if &*v == b"+123");

        let mut parser = start_parser(b"+inf");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::Infinity))
        );

        let mut parser = start_parser(b"-nan");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::NegNan))
        );

        let mut parser = start_parser(b"inf");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::Infinity))
        );

        let mut parser = start_parser(b"nan");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::Nan))
        );

        let mut parser = start_parser(b"[123, 456, 789]");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::Array(a)) if a == [
                Value::Integer(b"123".to_vec()),
                Value::Integer(b"456".to_vec()),
                Value::Integer(b"789".to_vec()),
            ]
        );

        let mut parser = start_parser(b"{ a = 123, b = 456, c = 789 }");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::InlineTable(t)) if t == hashmap! {
                "a".into() => Value::Integer(b"123".to_vec()),
                "b".into() => Value::Integer(b"456".to_vec()),
                "c".into() => Value::Integer(b"789".to_vec()),
            }
        );
    }

    #[test]
    fn parser_parse_value_invalid() {
        let mut parser = start_parser(b"01");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"0123");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"+");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"blah");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"\0");
        assert_matches!(parser.parse_value(), Err(Error(ErrorKind::IllegalChar(..))));

        let mut parser = start_parser(b"");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_string() {
        let mut parser = start_parser(indoc! {br#"
            "hello"
        "#});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello");

        let mut parser = start_parser(indoc! {br#"
            """
            hello
            """
        "#});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello\n");

        let mut parser = start_parser(indoc! {br"
            'hello'
        "});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello");

        let mut parser = start_parser(indoc! {br"
            '''
            hello
            '''
        "});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello\n");

        let mut parser = start_parser(indoc! {br#"
            "hello'
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            """
            hello
            "
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            """
            hello
            '''
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            'hello"
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            '''
            hello
            "
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            '''
            hello
            """
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"hello");
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_basic_str() {
        let mut parser = start_parser(indoc! {br#"
            hello\n"
        "#});
        assert_matches!(parser.parse_basic_str(), Ok(s) if s =="hello\n");

        let mut parser = start_parser(indoc! {br#"
            hello\"
        "#});
        assert_matches!(
            parser.parse_basic_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            hello\0"
        "#});
        assert_matches!(
            parser.parse_basic_str(),
            Err(Error(ErrorKind::InvalidEscape(..)))
        );

        let mut parser = start_parser(b"hello\0\"");
        assert_matches!(
            parser.parse_basic_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_parse_multiline_basic_str() {
        let mut parser = start_parser(indoc! {br#"
            hello
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n");

        let mut parser = start_parser(indoc! {br#"
            hello
            """"
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"");

        let mut parser = start_parser(indoc! {br#"
            hello
            """""
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"\"");

        let mut parser = start_parser(indoc! {br#"
            hello
            """"""
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"\""); // Still only 2 "s

        let mut parser = start_parser(indoc! {br#"
            hello
            ""
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"\"\n");

        let mut parser = start_parser(indoc! {br#"
            hello\t
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\t\n");

        let mut parser = start_parser(indoc! {br#"
            hello \
            world
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello world\n");

        let mut parser = start_parser(b"\nhello \\             \n   \nworld\n\"\"\"");
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello world\n");

        let mut parser = start_parser(b"hello\r\n\"\"\"");
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n");

        // \n after opening """ is trimmed
        let mut parser = start_parser(b"\nhello\n\"\"\"");
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n");

        let mut parser = start_parser(indoc! {br#"
            hello
            ""
        "#});
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"\n");
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(indoc! {br#"
            hello\    \
            """
        "#});
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::InvalidEscape(..)))
        );

        let mut parser = start_parser(b"hello\0\"");
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_parse_literal_str() {
        let mut parser = start_parser(b"hello\\n'");
        assert_matches!(parser.parse_literal_str(), Ok(s) if s == "hello\\n");

        let mut parser = start_parser(b"hello\n'");
        assert_matches!(
            parser.parse_literal_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"hello\0'");
        assert_matches!(
            parser.parse_literal_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_parse_multiline_literal_str() {
        let mut parser = start_parser(indoc! {br"
            hello
            '''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n");

        let mut parser = start_parser(indoc! {br"
            hello
            ''''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n'");

        let mut parser = start_parser(indoc! {br"
            hello
            '''''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n''");

        let mut parser = start_parser(indoc! {br"
            hello
            ''''''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n''"); // Still only 2 's

        let mut parser = start_parser(indoc! {br"
            hello
            ''
            '''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n''\n");

        let mut parser = start_parser(b"hello\r\n'''");
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n");

        // \n after opening ''' is trimmed
        let mut parser = start_parser(b"\nhello\n'''");
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n");

        let mut parser = start_parser(indoc! {br"
            hello
            ''
        "});
        assert_matches!(
            parser.parse_multiline_literal_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"\n");
        assert_matches!(
            parser.parse_multiline_literal_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"hello\0'");
        assert_matches!(
            parser.parse_multiline_literal_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn parser_parse_escape_seq() {
        let mut parser = start_parser(b"b");
        assert_matches!(parser.parse_escape_seq(), Ok('\x08'));

        let mut parser = start_parser(b"t");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\t');

        let mut parser = start_parser(b"n");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\n');

        let mut parser = start_parser(b"f");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\x0c');

        let mut parser = start_parser(b"r");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\r');

        let mut parser = start_parser(b"\"");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '"');

        let mut parser = start_parser(b"\\");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\\');

        let mut parser = start_parser(b"u20ac");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '€');

        let mut parser = start_parser(b"u2");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"ulmao");
        assert_matches!(parser.parse_escape_seq(), Err(Error(ErrorKind::InvalidEscape(esc))) if &*esc == "\\ulmao");

        let mut parser = start_parser(b"u\xff\xff\xff\xff");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::InvalidEncoding))
        );

        let mut parser = start_parser(b"U0001f60e");
        assert_matches!(parser.parse_escape_seq(), Ok('😎'));

        let mut parser = start_parser(b"U2");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"UROFLCOPTER");
        assert_matches!(parser.parse_escape_seq(), Err(Error(ErrorKind::InvalidEscape(esc))) if &*esc == "\\UROFLCOPT");

        let mut parser = start_parser(b"U\xff\xff\xff\xff\xff\xff\xff\xff");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::InvalidEncoding))
        );

        let mut parser = start_parser(b"");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = start_parser(b"p");
        assert_matches!(parser.parse_escape_seq(), Err(Error(ErrorKind::InvalidEscape(esc))) if &*esc == "\\p");
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn parser_parse_bool() {
        let mut parser = start_parser(b"true");
        assert_matches!(parser.parse_bool(), Ok(true));

        let mut parser = start_parser(b"false");
        assert_matches!(parser.parse_bool(), Ok(false));

        let mut parser = start_parser(b"TRUE");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"f");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"1");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_number_or_datetime() {
        let mut parser = start_parser(b"0x123");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::HexInt(_)));

        let mut parser = start_parser(b"0o123");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::OctalInt(_)));

        let mut parser = start_parser(b"0b101");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::BinaryInt(_)));

        let mut parser = start_parser(b"1980-01-01T12:00:00.000+02:30");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::OffsetDatetime { .. })
        );

        let mut parser = start_parser(b"1980-01-01 12:00:00Z");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::OffsetDatetime { .. })
        );

        let mut parser = start_parser(b"1980-01-01T12:00:00.000");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::LocalDatetime { .. })
        );

        let mut parser = start_parser(b"1980-01-01 12:00:00");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::LocalDatetime { .. })
        );

        let mut parser = start_parser(b"1980-01-01");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::LocalDate(_)));

        let mut parser = start_parser(b"12:00:00.000000000");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::LocalTime(_)));

        let mut parser = start_parser(b"123");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::Integer(_)));

        let mut parser = start_parser(b"4.5");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::Float(_)));
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn parser_parse_number_decimal() {
        let mut parser = start_parser(b"123_456");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Integer(v)) if &*v == b"123456"
        );

        let mut parser = start_parser(b"+123_456");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Integer(v)) if &*v == b"+123456"
        );

        let mut parser = start_parser(b"-123_456");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Integer(v)) if &*v == b"-123456"
        );

        let mut parser = start_parser(b"0");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Integer(v)) if &*v == b"0"
        );

        let mut parser = start_parser(b"0123");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"0_1");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"abc");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"-abc");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123_456.789_012");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Float(v)) if &*v == b"123456.789012"
        );

        let mut parser = start_parser(b"123_456.789_012e345_678");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Float(v)) if &*v == b"123456.789012e345678"
        );

        let mut parser = start_parser(b"123_456.789_012e+345_678");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Float(v)) if &*v == b"123456.789012e+345678"
        );

        let mut parser = start_parser(b"123_456.789_012e-345_678");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Float(v)) if &*v == b"123456.789012e-345678"
        );

        let mut parser = start_parser(b"123_456e345_678");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Float(v)) if &*v == b"123456e345678"
        );

        let mut parser = start_parser(b".123");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123.");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123e");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"e123");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123.e456");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123.abc");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123.456eabc");
        assert_matches!(
            parser.parse_number_decimal(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );
    }

    #[test]
    fn parser_parse_digits() {
        let mut parser = start_parser(b"123_456");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Ok(v) if &*v == b"123456"
        );

        let mut parser = start_parser(b"abc_def");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_hexdigit),
            Ok(v) if &*v == b"abcdef"
        );

        let mut parser = start_parser(b"abc_def");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"_123_");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"_123");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123_");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"123__456");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = start_parser(b"");
        assert_matches!(
            parser.parse_digits(u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );
    }

    #[test]
    fn parser_parse_number_special() {
        let mut parser = start_parser(b"inf");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Infinity));

        let mut parser = start_parser(b"+inf");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Infinity));

        let mut parser = start_parser(b"-inf");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::NegInfinity));

        let mut parser = start_parser(b"nan");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Nan));

        let mut parser = start_parser(b"+nan");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Nan));

        let mut parser = start_parser(b"-nan");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::NegNan));

        let mut parser = start_parser(b"+1.0e+3");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"NaN");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"INF");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"abc");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"+abc");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"-abc");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_array() {
        let mut parser = start_parser(b"]");
        assert_matches!(parser.parse_array(), Ok(s) if s.is_empty());

        let mut parser = start_parser(b"  ]");
        assert_matches!(parser.parse_array(), Ok(s) if s.is_empty());

        let mut parser = start_parser(indoc! {br"
                # comment
            ]
        "});
        assert_matches!(parser.parse_array(), Ok(s) if s.is_empty());

        let mut parser = start_parser(b"123]");
        assert_matches!(
            parser.parse_array(),
            Ok(s) if s == [Value::Integer(b"123".to_vec())]
        );

        let mut parser = start_parser(b"123,]");
        assert_matches!(
            parser.parse_array(),
            Ok(s) if s == [Value::Integer(b"123".to_vec())]
        );

        let mut parser = start_parser(indoc! {br"
                123,
            ]
        "});
        assert_matches!(
            parser.parse_array(),
            Ok(s) if s == [Value::Integer(b"123".to_vec())]
        );

        let mut parser = start_parser(br"123, 456, 789]");
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".to_vec()),
                Value::Integer(b"456".to_vec()),
                Value::Integer(b"789".to_vec())
            ]
        );

        let mut parser = start_parser(br"123, 456, 789,]");
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".to_vec()),
                Value::Integer(b"456".to_vec()),
                Value::Integer(b"789".to_vec())
            ]
        );

        let mut parser = start_parser(indoc! {br"
                123,
                456,
                789,
            ]
        "});
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".to_vec()),
                Value::Integer(b"456".to_vec()),
                Value::Integer(b"789".to_vec())
            ]
        );

        let mut parser = start_parser(indoc! {br"
                123,
                456, # comment
                789 # comment
            ]
        "});
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".to_vec()),
                Value::Integer(b"456".to_vec()),
                Value::Integer(b"789".to_vec())
            ]
        );

        let mut parser = start_parser(b"123 abc]");
        assert_matches!(
            parser.parse_array(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_inline_table() {
        let mut parser = start_parser(b"}");
        assert_matches!(parser.parse_inline_table(), Ok(t) if t == Table::new());

        let mut parser = start_parser(b"  }");
        assert_matches!(parser.parse_inline_table(), Ok(t) if t == Table::new());

        let mut parser = start_parser(b"abc = 123 }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! { "abc".into() => Value::Integer(b"123".to_vec()) }
        );

        let mut parser = start_parser(br"abc = 123, def = 456, ghi = 789 }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! {
                "abc".into() => Value::Integer(b"123".to_vec()),
                "def".into() => Value::Integer(b"456".to_vec()),
                "ghi".into() => Value::Integer(b"789".to_vec()),
            }
        );

        let mut parser = start_parser(br"abc = { def = 123, ghi = 456 } }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! {
                "abc".into() => Value::InlineTable(hashmap! {
                    "def".into() => Value::Integer(b"123".to_vec()),
                    "ghi".into() => Value::Integer(b"456".to_vec()),
                }),
            }
        );

        let mut parser = start_parser(br"abc.def = 123, abc.ghi = 456 }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! {
                "abc".into() => Value::DottedKeyTable(hashmap! {
                    "def".into() => Value::Integer(b"123".to_vec()),
                    "ghi".into() => Value::Integer(b"456".to_vec()),
                }),
            }
        );

        let mut parser = start_parser(b"abc 123 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"abc = 123, }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"123 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(indoc! {br"
                abc = 123
            }
        "});
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = start_parser(b"abc = 123, abc = 456 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::DuplicateKey(..)))
        );

        let mut parser = start_parser(b"abc = { def = 123 }, abc.ghi = 456 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::InvalidKeyPath(..)))
        );

        let mut parser = start_parser(b"abc = 123, def = 456 ");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_skip_whitespace() {
        let mut parser = start_parser(b"   ");
        parser.skip_whitespace();
        assert!(parser.line.is_empty());

        let mut parser = start_parser(b"   \t");
        parser.skip_whitespace();
        assert!(parser.line.is_empty());

        let mut parser = start_parser(b"   abc");
        parser.skip_whitespace();
        assert_eq!(parser.line, b"abc");

        let mut parser = start_parser(b"   \t   abc");
        parser.skip_whitespace();
        assert_eq!(parser.line, b"abc");

        let mut parser = start_parser(b"   \t   # comment");
        parser.skip_whitespace();
        assert_eq!(parser.line, b"# comment");

        let mut parser = start_parser(b"abc");
        parser.skip_whitespace();
        assert_eq!(parser.line, b"abc");

        let mut parser = start_parser(b"");
        parser.skip_whitespace();
        assert!(parser.line.is_empty());
    }

    #[test]
    fn parser_skip_comment() {
        let mut parser = start_parser(b"# comment");
        parser.skip_comment().unwrap();
        assert!(parser.line.is_empty());

        let mut parser = start_parser(b"# comment\n");
        parser.skip_comment().unwrap();
        assert!(parser.line.is_empty());

        let mut parser = start_parser(b"# comment\r\n");
        parser.skip_comment().unwrap();
        assert!(parser.line.is_empty());

        let mut parser = start_parser(b"abc");
        parser.skip_comment().unwrap();
        assert_eq!(parser.line, b"abc");

        let mut parser = start_parser(b"# comment\xff");
        assert_matches!(
            parser.skip_comment(),
            Err(Error(ErrorKind::InvalidEncoding))
        );

        let mut parser = start_parser(b"# comment\0");
        assert_matches!(
            parser.skip_comment(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_next_line() {
        let mut parser = Parser {
            reader: Reader::from_str(indoc! {r"
                [a]
                b = c
            "}),
            line: b"",
        };
        assert!(parser.next_line().is_some());
        assert_eq!(parser.line, b"[a]");
        assert!(parser.next_line().is_some());
        assert_eq!(parser.line, b"b = c");
        assert!(parser.next_line().is_none());
    }

    #[test]
    fn table_get_subtable() {
        let mut table = hashmap! {
            "a".into() => Value::ArrayOfTables(vec![hashmap! {}]),
            "b".into() => Value::Table(hashmap! {
                "c".into() => Value::UndefinedTable(hashmap! {}),
                "d".into() => Value::DottedKeyTable(hashmap! {}),
                "e".into() => Value::Table(hashmap! {}),
                "f".into() => Value::ArrayOfTables(vec![hashmap! {}]),
                "g".into() => Value::InlineTable(hashmap! {}),
                "h".into() => Value::Integer(b"123".to_vec()),
            }),
        };
        assert_eq!(table.clone().get_subtable(&[]), Some(&mut table));
        assert_eq!(
            table.get_subtable(&["a".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["a".to_string(), "b".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["a".to_string(), "b".to_string(), "c".to_string()]),
            Some(&mut hashmap! {})
        );

        assert_eq!(
            table.get_subtable(&["b".to_string(), "a".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["b".to_string(), "c".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["b".to_string(), "d".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["b".to_string(), "e".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["b".to_string(), "f".to_string()]),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_subtable(&["b".to_string(), "g".to_string()]),
            None
        );
        assert_eq!(
            table.get_subtable(&["b".to_string(), "h".to_string()]),
            None
        );
    }

    #[test]
    fn table_get_dotted_subtable() {
        let mut table = hashmap! {
            "a".into() => Value::DottedKeyTable(hashmap! {}),
            "b".into() => Value::UndefinedTable(hashmap! {}),
            "c".into() => Value::Table(hashmap! {}),
        };
        assert_eq!(
            table.clone().get_dotted_subtable(&[], true),
            Some(&mut table)
        );
        assert_eq!(
            table.get_dotted_subtable(&["a".to_string()], true),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_dotted_subtable(&["a".to_string(), "b".to_string()], true),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_dotted_subtable(&["b".to_string()], true),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_dotted_subtable(&["b".to_string(), "c".to_string()], true),
            Some(&mut hashmap! {})
        );
        assert_eq!(table.get_dotted_subtable(&["c".to_string()], true), None);

        let mut table = hashmap! {
            "a".into() => Value::DottedKeyTable(hashmap! {}),
            "b".into() => Value::UndefinedTable(hashmap! {}),
            "c".into() => Value::Table(hashmap! {}),
        };
        assert_eq!(
            table.clone().get_dotted_subtable(&[], false),
            Some(&mut table)
        );
        assert_eq!(
            table.get_dotted_subtable(&["a".to_string()], false),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.get_dotted_subtable(&["a".to_string(), "b".to_string()], false),
            Some(&mut hashmap! {})
        );
        assert_eq!(table.get_dotted_subtable(&["b".to_string()], false), None);
        assert_eq!(
            table.get_dotted_subtable(&["b".to_string(), "c".to_string()], false),
            None
        );
        assert_eq!(table.get_dotted_subtable(&["c".to_string()], false), None);
    }

    #[test]
    fn table_insert_table() {
        let mut table = hashmap! {
            "a".to_string() => Value::UndefinedTable(hashmap! {}),
        };
        assert_eq!(table.insert_table("a".to_string()), Some(&mut hashmap! {}));
        assert_eq!(table.insert_table("b".to_string()), Some(&mut hashmap! {}));
        assert_eq!(table.len(), 2);
        assert_eq!(table["a"], Value::Table(hashmap! {}));
        assert_eq!(table["b"], Value::Table(hashmap! {}));

        let mut table = hashmap! {
            "a".to_string() => Value::Table(hashmap! {}),
        };
        assert_eq!(table.insert_table("a".to_string()), None);
    }

    #[test]
    #[allow(clippy::pattern_type_mismatch)]
    fn table_append_array_of_tables() {
        let mut table = hashmap! {
            "a".to_string() => Value::ArrayOfTables(vec![]),
        };
        assert_eq!(
            table.append_array_of_tables("a".to_string()),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.append_array_of_tables("a".to_string()),
            Some(&mut hashmap! {})
        );
        assert_eq!(
            table.append_array_of_tables("b".to_string()),
            Some(&mut hashmap! {})
        );
        assert_eq!(table.len(), 2);
        assert_matches!(&table["a"], Value::ArrayOfTables(vec) if vec.len() == 2);
        assert_matches!(&table["b"], Value::ArrayOfTables(vec) if vec.len() == 1);

        let mut table = hashmap! {
            "a".to_string() => Value::Table(hashmap! {}),
        };
        assert_eq!(table.append_array_of_tables("a".to_string()), None);
    }
}
