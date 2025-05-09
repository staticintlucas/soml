use core::{fmt, str};
use std::collections::hash_map::Entry;
use std::io;
use std::marker::PhantomData;

use serde::de;

use super::error::{ErrorKind, Result};
use super::reader::IoReader;
use super::{Reader, SliceReader};
use crate::value::{LocalDate, LocalDatetime, LocalTime, Offset, OffsetDatetime};

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
    #[inline]
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
    OffsetDatetime(OffsetDatetime),
    // Local Datetime
    LocalDatetime(LocalDatetime),
    // Local Date
    LocalDate(LocalDate),
    // Local Time
    LocalTime(LocalTime),
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
pub(super) struct Parser<'de, R: Reader<'de>> {
    reader: R,
    _phantom: PhantomData<&'de ()>,
}

impl<'de> Parser<'de, SliceReader<'de>> {
    #[must_use]
    #[inline]
    pub fn from_str(str: &'de str) -> Self {
        Self {
            reader: SliceReader::from_str(str),
            _phantom: PhantomData,
        }
    }

    #[must_use]
    #[inline]
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        Self {
            reader: SliceReader::from_slice(bytes),
            _phantom: PhantomData,
        }
    }
}

impl<R> Parser<'_, IoReader<R>>
where
    R: io::Read,
{
    #[must_use]
    #[inline]
    pub fn from_reader(read: R) -> Self {
        Self {
            reader: IoReader::from_reader(read),
            _phantom: PhantomData,
        }
    }
}

impl<'de, R> Parser<'de, R>
where
    R: Reader<'de>,
{
    pub fn parse(&mut self) -> Result<Value> {
        let mut root = Table::with_capacity(10);

        // The currently opened table
        let mut table = &mut root;
        // The path to the currently opened table (used for error messages)
        let mut table_path = Vec::with_capacity(16);

        loop {
            self.skip_comments_and_whitespace()?;

            // Parse array header
            if self.reader.eat_str(b"[[")? {
                let key = self.parse_array_header()?;
                table = Self::get_array_header(&mut root, &key)
                    .ok_or_else(|| ErrorKind::InvalidTableHeader(key.join(".").into()))?;
                table_path = key;
            }
            // Parse table header
            else if self.reader.eat_char(b'[')? {
                let key = self.parse_table_header()?;
                table = Self::get_table_header(&mut root, &key)
                    .ok_or_else(|| ErrorKind::InvalidTableHeader(key.join(".").into()))?;
                table_path = key;
            }
            // Parse key/value pair
            else if self
                .reader
                .peek()?
                .is_some_and(|ch| is_toml_word(&ch) || ch == b'"' || ch == b'\'')
            {
                let (full_key, value) = self.parse_key_value_pair()?;
                let (key, path) = full_key
                    .split_last()
                    .unwrap_or_else(|| unreachable!("path cannot be empty"));

                // Navigate to the subtable
                let subtable = Self::get_dotted_key(table, path).ok_or_else(|| {
                    ErrorKind::InvalidKeyPath(
                        full_key.join(".").into(),
                        if table_path.is_empty() {
                            "root table".into()
                        } else {
                            table_path.join(".").into()
                        },
                    )
                })?;

                // Check if the key is already present
                if subtable.contains_key(key) {
                    return Err(ErrorKind::DuplicateKey(
                        full_key.join(".").into(),
                        if table_path.is_empty() {
                            "root table".into()
                        } else {
                            table_path.join(".").into()
                        },
                    )
                    .into());
                }
                subtable.insert(key.clone(), value);
            }
            // Anything else is unexpected
            else if let Some(ch) = self.reader.next()? {
                return Err(if is_toml_legal(&ch) {
                    ErrorKind::ExpectedToken("table header or key/value pair".into()).into()
                } else {
                    ErrorKind::IllegalChar(ch).into()
                });
            }
            // Or if there was no more input we break for EOF
            else {
                break;
            }

            // Expect newline/comment after a key/value pair or table/array header
            self.skip_whitespace()?;
            match self.reader.peek()? {
                Some(b'\n') => self.reader.discard()?,
                Some(b'\r') if self.reader.peek_at(1)?.is_some_and(|ch| ch == b'\n') => {
                    self.reader.discard_n(2)?; // b"\r\n"
                }
                Some(b'#') => {
                    self.skip_comment()?;
                }
                Some(ch) => {
                    return Err(if is_toml_legal(&ch) {
                        ErrorKind::ExpectedToken("end of line".into()).into()
                    } else {
                        ErrorKind::IllegalChar(ch).into()
                    });
                }
                None => break,
            }
        }

        Ok(Value::Table(root))
    }

    fn parse_array_header(&mut self) -> Result<Vec<String>> {
        self.skip_whitespace()?;
        let key = self.parse_dotted_key()?;

        self.skip_whitespace()?;
        self.reader
            .eat_str(b"]]")?
            .then_some(key)
            .ok_or_else(|| ErrorKind::ExpectedToken("]] after dotted key".into()).into())
    }

    fn parse_table_header(&mut self) -> Result<Vec<String>> {
        self.skip_whitespace()?;
        let key = self.parse_dotted_key()?;

        self.skip_whitespace()?;
        self.reader
            .eat_char(b']')?
            .then_some(key)
            .ok_or_else(|| ErrorKind::ExpectedToken("] after dotted key".into()).into())
    }

    fn parse_key_value_pair(&mut self) -> Result<(Vec<String>, Value)> {
        let path = self.parse_dotted_key()?;

        // Whitespace should already have been consumed by parse_dotted_key looking for another '.'
        if !self.reader.eat_char(b'=')? {
            return Err(ErrorKind::ExpectedToken("= after key".into()).into());
        }
        self.skip_whitespace()?;

        let value = self.parse_value()?;

        Ok((path, value))
    }

    fn parse_dotted_key(&mut self) -> Result<Vec<String>> {
        let mut result = vec![self.parse_key()?];

        self.skip_whitespace()?;

        while self.reader.eat_char(b'.')? {
            self.skip_whitespace()?;
            result.push(self.parse_key()?);
            self.skip_whitespace()?;
        }

        Ok(result)
    }

    fn parse_key(&mut self) -> Result<String> {
        if self.reader.eat_char(b'"')? {
            if self.reader.eat_str(br#""""#)? {
                // multiline strings are invalid as keys
                Err(ErrorKind::ExpectedToken("key".into()).into())
            } else {
                self.parse_basic_str()
            }
        } else if self.reader.eat_char(b'\'')? {
            if self.reader.eat_str(b"''")? {
                // multiline strings are invalid as keys
                Err(ErrorKind::ExpectedToken("key".into()).into())
            } else {
                self.parse_literal_str()
            }
        } else {
            self.parse_bare_key()
        }
    }

    fn parse_bare_key(&mut self) -> Result<String> {
        let key = self.reader.next_str_while(is_toml_word)?;

        (!key.is_empty())
            .then_some(key)
            .ok_or_else(|| ErrorKind::ExpectedToken("key".into()).into())
    }

    fn parse_value(&mut self) -> Result<Value> {
        match self.reader.peek()? {
            // String
            Some(b'"' | b'\'') => self.parse_string().map(Value::String),
            // Boolean
            Some(b't' | b'f') => self.parse_bool().map(Value::Boolean),
            // Digit could mean either number or datetime
            Some(b'0'..=b'9') => self.parse_number_or_datetime(),
            // Number (either int/float or special float)
            Some(b'+' | b'-') => {
                match self.reader.peek_at(1)? {
                    // Number
                    Some(ch) if ch.is_ascii_digit() => self.parse_number_decimal(),
                    // Special float
                    Some(b'i' | b'n') => self.parse_number_special().map(Value::SpecialFloat),
                    // Invalid
                    _ => Err(ErrorKind::InvalidNumber("missing digits".into()).into()),
                }
            }
            // Special float (inf or nan)
            Some(b'i' | b'n') => self.parse_number_special().map(Value::SpecialFloat),
            // Array
            Some(b'[') => {
                self.reader.discard()?; // We consume the opening delimiter
                self.parse_array().map(Value::Array)
            }
            // Table
            Some(b'{') => {
                self.reader.discard()?; // We consume the opening delimiter
                self.parse_inline_table().map(Value::InlineTable)
            }
            Some(ch) if is_toml_legal(&ch) => {
                Err(ErrorKind::ExpectedToken("a value".into()).into())
            }
            Some(ch) => Err(ErrorKind::IllegalChar(ch).into()),
            None => Err(ErrorKind::UnexpectedEof.into()),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        if self.reader.eat_char(b'"')? {
            if self.reader.eat_str(br#""""#)? {
                self.parse_multiline_basic_str()
            } else {
                self.parse_basic_str()
            }
        } else if self.reader.eat_char(b'\'')? {
            if self.reader.eat_str(b"''")? {
                self.parse_multiline_literal_str()
            } else {
                self.parse_literal_str()
            }
        } else {
            Err(ErrorKind::ExpectedToken("string".into()).into())
        }
    }

    fn parse_basic_str(&mut self) -> Result<String> {
        let mut str = self.reader.next_str_while(is_toml_basic_str_sans_escapes)?;

        loop {
            match self.reader.next()? {
                Some(b'\\') => {
                    // Parse escape sequence
                    str.push(self.parse_escape_seq()?);
                }
                Some(b'"') => {
                    break Ok(str);
                }
                None | Some(b'\r' | b'\n') => {
                    break Err(ErrorKind::UnterminatedString.into());
                }
                Some(char) => {
                    break Err(ErrorKind::IllegalChar(char).into());
                }
            }

            str.push_str(&self.reader.next_str_while(is_toml_basic_str_sans_escapes)?);
        }
    }

    fn parse_multiline_basic_str(&mut self) -> Result<String> {
        // Newlines after the first """ are ignored
        let _ = self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?;

        let mut str = self
            .reader
            .next_str_while(is_toml_multiline_basic_str_sans_escapes)?;

        loop {
            match self.reader.next()? {
                Some(b'\\') => {
                    // Trailing '\' means eat all whitespace and newlines
                    if self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")? {
                        let _ws = self.reader.next_while(is_toml_whitespace_or_newline)?;
                    }
                    // If there's space after the \ we assume a trailing \ with trailing whitespace,
                    // but we need to verify there's only whitespace chars before the next newline
                    else if let Some(char) = self.reader.next_if(is_toml_whitespace)? {
                        let _ws = self.reader.next_while(is_toml_whitespace)?;
                        if !(self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?) {
                            return Err(ErrorKind::InvalidEscape(
                                format!("{:?}", char::from(char)).into(),
                            )
                            .into());
                        }
                        let _ws = self.reader.next_while(is_toml_whitespace_or_newline)?;
                    } else {
                        // Parse a regular escape sequence and continue
                        str.push(self.parse_escape_seq()?);
                    }
                }
                Some(b'"') => {
                    // Check for 2 more '"'s
                    if self.reader.eat_str(b"\"\"")? {
                        // We can have up to 5 '"'s, 2 quotes inside the string right before the 3
                        // which close the string. So we check for 2 additional '"'s and push them
                        if self.reader.eat_char(b'"')? {
                            str.push('"');
                            if self.reader.eat_char(b'"')? {
                                str.push('"');
                            }
                        }

                        break Ok(str);
                    }
                    str.push('"');
                }
                None => {
                    break Err(ErrorKind::UnterminatedString.into());
                }
                Some(b'\r') if matches!(self.reader.peek()?, Some(b'\n')) => {
                    // Ignore '\r' followed by '\n', else it's handled by the illegal char branch
                }
                Some(char) => break Err(ErrorKind::IllegalChar(char).into()),
            }

            str.push_str(
                &self
                    .reader
                    .next_str_while(is_toml_multiline_basic_str_sans_escapes)?,
            );
        }
    }

    fn parse_literal_str(&mut self) -> Result<String> {
        let str = self.reader.next_str_while(is_toml_literal_str)?;

        match self.reader.next()? {
            Some(b'\'') => Ok(str),
            None | Some(b'\r' | b'\n') => Err(ErrorKind::UnterminatedString.into()),
            Some(char) => Err(ErrorKind::IllegalChar(char).into()),
        }
    }

    fn parse_multiline_literal_str(&mut self) -> Result<String> {
        // Newlines after the first ''' are ignored
        let _ = self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?;

        let mut str = self.reader.next_str_while(is_toml_multiline_literal_str)?;

        loop {
            match self.reader.next()? {
                Some(b'\'') => {
                    // Check for 2 more '\''s
                    if self.reader.eat_str(b"''")? {
                        // We can have up to 5 '\''s, 2 quotes inside the string right before the 3
                        // which close the string. So we check for 2 additional '\''s and push them
                        if self.reader.eat_char(b'\'')? {
                            str.push('\'');
                            if self.reader.eat_char(b'\'')? {
                                str.push('\'');
                            }
                        }

                        break Ok(str);
                    }
                    str.push('\'');
                }
                None => {
                    break Err(ErrorKind::UnterminatedString.into());
                }
                Some(b'\r') if matches!(self.reader.peek()?, Some(b'\n')) => {
                    // Ignore '\r' followed by '\n', else it's handled by the illegal char branch
                }
                Some(char) => break Err(ErrorKind::IllegalChar(char).into()),
            }

            str.push_str(&self.reader.next_str_while(is_toml_multiline_literal_str)?);
        }
    }

    fn parse_escape_seq(&mut self) -> Result<char> {
        let Some(char) = self.reader.peek()? else {
            return Err(ErrorKind::UnterminatedString.into());
        };

        match char {
            b'b' => self.reader.discard().map(|()| '\x08'),
            b't' => self.reader.discard().map(|()| '\t'),
            b'n' => self.reader.discard().map(|()| '\n'),
            b'f' => self.reader.discard().map(|()| '\x0c'),
            b'r' => self.reader.discard().map(|()| '\r'),
            b'"' => self.reader.discard().map(|()| '"'),
            b'\\' => self.reader.discard().map(|()| '\\'),
            b'u' => {
                self.reader.discard()?;
                let bytes = self
                    .reader
                    .next_n(4)?
                    .ok_or(ErrorKind::UnterminatedString)?;
                let str = str::from_utf8(&bytes).map_err(|_| ErrorKind::InvalidEncoding)?;
                u32::from_str_radix(str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| ErrorKind::InvalidEscape(format!("\\u{str}").into()).into())
            }
            b'U' => {
                self.reader.discard()?;
                let bytes = self
                    .reader
                    .next_n(8)?
                    .ok_or(ErrorKind::UnterminatedString)?;
                let str = str::from_utf8(&bytes).map_err(|_| ErrorKind::InvalidEncoding)?;
                u32::from_str_radix(str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| ErrorKind::InvalidEscape(format!("\\U{str}").into()).into())
            }
            _ => Err(ErrorKind::InvalidEscape(
                self.reader
                    .next_char()? // We want a char here, not just a byte
                    .ok_or(ErrorKind::UnterminatedString)
                    .map(|ch| format!("\\{ch}").into())?,
            )
            .into()),
        }
    }

    fn parse_bool(&mut self) -> Result<bool> {
        // Match against the whole word, don't just parse the first n characters so we don't
        // successfully parse e.g. true92864yhowkalgp98y
        let word = self.reader.next_while(is_toml_word)?;

        match &word[..] {
            b"true" => Ok(true),
            b"false" => Ok(false),
            _ => Err(ErrorKind::ExpectedToken("true/false".into()).into()),
        }
    }

    // Parses anything that starts with a digit. Does not parse special floats or +/- values
    fn parse_number_or_datetime(&mut self) -> Result<Value> {
        fn remove_start(mut bytes: Vec<u8>, n: usize) -> Vec<u8> {
            bytes.drain(..n);
            bytes
        }

        let value = self.reader.next_while(is_toml_number_or_datetime)?;
        match *value {
            // Hex literal starts with "0x"
            [b'0', b'x', ..] => {
                let digits = remove_start(value, 2);
                Self::normalize_number(digits, u8::is_ascii_hexdigit).map(Value::HexInt)
            }
            // Octal literal starts with "0o"
            [b'0', b'o', ..] => {
                let digits = remove_start(value, 2);
                Self::normalize_number(digits, |&b| matches!(b, b'0'..=b'7')).map(Value::OctalInt)
            }
            // Binary literal starts with "0b"
            [b'0', b'b', ..] => {
                let digits = remove_start(value, 2);
                Self::normalize_number(digits, |&b| matches!(b, b'0' | b'1')).map(Value::BinaryInt)
            }
            // LocalTime has a ':' at index 2
            #[cfg(feature = "datetime")]
            [_, _, b':', ..] => LocalTime::from_slice(&value).map(Value::LocalTime),
            // OffsetDateTime, LocalDateTime, or LocalDate have '-' at index 4
            // Also need to check for only digits before to rule out float literals (e.g. 120e-2)
            #[cfg(feature = "datetime")]
            [b'0'..=b'9', b'0'..=b'9', b'0'..=b'9', b'0'..=b'9', b'-', ..] => {
                // If we have a 'T' split the date and time
                let (date, time) =
                    if let Some(idx) = value.iter().position(|&b| matches!(b, b'T' | b't')) {
                        (&value[..idx], &value[idx + 1..])
                    }
                    // If we don't have a 'T' we might have a space-delimited datetime which we have only
                    // read part of, so check for space followed by a digit
                    else if self
                        .reader
                        .peek_n(2)?
                        .is_some_and(|b| b[0] == b' ' && b[1].is_ascii_digit())
                    {
                        // Discard the space
                        self.reader.discard()?;
                        // Read in the time
                        (
                            &value[..],
                            &*self.reader.next_while(is_toml_number_or_datetime)?,
                        )
                    }
                    // Else we definitely just have a LocalDate
                    else {
                        return LocalDate::from_slice(&value).map(Value::LocalDate);
                    };

                if let Some(idx) = time
                    .iter()
                    .position(|&b| matches!(b, b'z' | b'Z' | b'+' | b'-'))
                {
                    let (time, offset) = time.split_at(idx);
                    Ok(Value::OffsetDatetime(OffsetDatetime {
                        date: LocalDate::from_slice(date)?,
                        time: LocalTime::from_slice(time)?,
                        offset: Offset::from_slice(offset)?,
                    }))
                // Otherwise it's just a LocalDateTime
                } else {
                    Ok(Value::LocalDatetime(LocalDatetime {
                        date: LocalDate::from_slice(date)?,
                        time: LocalTime::from_slice(time)?,
                    }))
                }
            }
            // Just a plain ol' decimal
            [..] => Self::normalize_number_decimal(value),
        }
    }

    fn parse_number_decimal(&mut self) -> Result<Value> {
        let value = self.reader.next_while(is_toml_number_or_datetime)?;
        Self::normalize_number_decimal(value)
    }

    fn normalize_number_decimal(digits: Vec<u8>) -> Result<Value> {
        fn split_at_byte(bytes: &[u8], pred: impl FnMut(&u8) -> bool) -> Option<(&[u8], &[u8])> {
            bytes
                .iter()
                .position(pred)
                .map(|i| (&bytes[..i], &bytes[i + 1..]))
        }

        let mut float = false;

        // Split the number into integer, fraction, and exponent parts (if present)
        let (integer, fraction, exponent) =
            if let Some((i, rest)) = split_at_byte(&digits, |&b| b == b'.') {
                let (f, e) = split_at_byte(rest, |&b| matches!(b, b'e' | b'E'))
                    .map_or((Some(rest), None), |(f, e)| (Some(f), Some(e)));
                (i, f, e)
            } else {
                let (i, e) = split_at_byte(&digits, |&b| matches!(b, b'e' | b'E'))
                    .map_or((&*digits, None), |(i, e)| (i, Some(e)));
                (i, None, e)
            };

        // Validate each part
        let integer = if matches!(integer.first().copied(), Some(b'+' | b'-')) {
            &integer[1..]
        } else {
            integer
        };
        Self::check_underscores(integer)?;
        if integer.len() > 1 && integer[0] == b'0' {
            // Only fail for len > 1; we allow "0", "0.123", etc
            return Err(ErrorKind::InvalidNumber("leading zero".into()).into());
        }
        if !integer.iter().all(|&b| matches!(b, b'0'..=b'9' | b'_')) {
            return Err(ErrorKind::InvalidNumber("invalid digit".into()).into());
        }

        if let Some(fraction) = fraction {
            float = true;
            Self::check_underscores(fraction)?;
            if !fraction.iter().all(|&b| matches!(b, b'0'..=b'9' | b'_')) {
                return Err(ErrorKind::InvalidNumber("invalid digit".into()).into());
            }
        }
        if let Some(exponent) = exponent {
            float = true;
            let exponent = if matches!(exponent.first().copied(), Some(b'+' | b'-')) {
                &exponent[1..]
            } else {
                exponent
            };
            Self::check_underscores(exponent)?;
            if !exponent.iter().all(|&b| matches!(b, b'0'..=b'9' | b'_')) {
                return Err(ErrorKind::InvalidNumber("invalid digit".into()).into());
            }
        }

        // Now we can just strip the underscores
        let number = Self::strip_underscores(digits);

        Ok(if float {
            Value::Float(number)
        } else {
            Value::Integer(number)
        })
    }

    fn normalize_number(digits: Vec<u8>, is_digit: fn(&u8) -> bool) -> Result<Vec<u8>> {
        Self::check_underscores(&digits)?;
        let digits = Self::strip_underscores(digits);

        if digits.iter().all(is_digit) {
            Ok(digits)
        } else {
            Err(ErrorKind::InvalidNumber("invalid digit".into()).into())
        }
    }

    fn check_underscores(digits: &[u8]) -> Result<()> {
        if digits.is_empty() {
            return Err(ErrorKind::InvalidNumber("no digits".into()).into());
        }
        if digits.starts_with(b"_") {
            return Err(ErrorKind::InvalidNumber("leading underscore".into()).into());
        }
        if digits.ends_with(b"_") {
            return Err(ErrorKind::InvalidNumber("trailing underscore".into()).into());
        }
        if digits.windows(2).any(|w| w == b"__") {
            return Err(ErrorKind::InvalidNumber("double underscore".into()).into());
        }
        Ok(())
    }

    fn strip_underscores(digits: Vec<u8>) -> Vec<u8> {
        if let Some(idx) = digits.iter().position(|&b| b == b'_') {
            let mut result = Vec::with_capacity(digits.len() - 1); // Upper bound
            result.extend_from_slice(&digits[..idx]);
            let mut digits = &digits[idx + 1..];
            while let Some(idx) = digits.iter().position(|&b| b == b'_') {
                result.extend_from_slice(&digits[..idx]);
                digits = &digits[idx + 1..];
            }
            result.extend_from_slice(digits);
            result
        } else {
            digits
        }
    }

    fn parse_number_special(&mut self) -> Result<SpecialFloat> {
        // In each case we match against the whole word, don't just parse the first n characters so
        // we don't successfully parse e.g. inf92864yhowkalgp98y
        match self.reader.peek()? {
            Some(b'+') => {
                self.reader.discard()?;
                match &self.reader.next_while(is_toml_word)?[..] {
                    b"inf" => Ok(SpecialFloat::Infinity),
                    b"nan" => Ok(SpecialFloat::Nan),
                    _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
                }
            }
            Some(b'-') => {
                self.reader.discard()?;
                match &self.reader.next_while(is_toml_word)?[..] {
                    b"inf" => Ok(SpecialFloat::NegInfinity),
                    b"nan" => Ok(SpecialFloat::NegNan),
                    _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
                }
            }
            _ => match &self.reader.next_while(is_toml_word)?[..] {
                b"inf" => Ok(SpecialFloat::Infinity),
                b"nan" => Ok(SpecialFloat::Nan),
                _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
            },
        }
    }

    fn parse_array(&mut self) -> Result<Vec<Value>> {
        let mut result = vec![];

        loop {
            self.skip_comments_and_whitespace()?;

            if self.reader.eat_char(b']')? {
                break; // End of array
            }

            result.push(self.parse_value()?);

            self.skip_comments_and_whitespace()?;

            if self.reader.eat_char(b']')? {
                break; // End of array
            }
            if !self.reader.eat_char(b',')? {
                return Err(ErrorKind::ExpectedToken(", or ] after value in array".into()).into());
            }
        }

        Ok(result)
    }

    fn parse_inline_table(&mut self) -> Result<Table> {
        let mut result = Table::with_capacity(10);

        self.skip_whitespace()?;

        if self.reader.eat_char(b'}')? {
            return Ok(result); // End of table
        }

        loop {
            let (full_key, value) = self.parse_key_value_pair()?;

            let (key, path) = full_key
                .split_last()
                .unwrap_or_else(|| unreachable!("path cannot be empty"));

            // Navigate to the subtable
            let subtable = Self::get_inline_subtable(&mut result, path).ok_or_else(|| {
                ErrorKind::InvalidKeyPath(full_key.join(".").into(), "inline table".into())
            })?;

            // Check if the key is already present
            if subtable.contains_key(key) {
                return Err(ErrorKind::DuplicateKey(
                    full_key.join(".").into(),
                    "inline table".into(),
                )
                .into());
            }
            subtable.insert(key.clone(), value);

            self.skip_whitespace()?;

            if self.reader.eat_char(b'}')? {
                break; // End of array
            } else if self.reader.eat_char(b',')? {
                self.skip_whitespace()?;
            } else {
                return Err(if self.reader.next()?.is_some() {
                    ErrorKind::ExpectedToken(", or } after key/value pair in inline table".into())
                        .into()
                } else {
                    ErrorKind::UnexpectedEof.into()
                });
            }
        }

        Ok(result)
    }

    fn skip_whitespace(&mut self) -> Result<()> {
        while self.reader.next_if(is_toml_whitespace)?.is_some() {}
        Ok(())
    }

    fn skip_comment(&mut self) -> Result<()> {
        if self.reader.eat_char(b'#')? {
            // Skip validating comments with feature = "fast"
            if cfg!(feature = "fast") {
                let _comment = self.reader.next_while(|&ch| ch != b'\n')?;
            } else {
                // next_str_while will validate UTF-8
                let comment = self.reader.next_str_while(|&ch| ch != b'\n')?;
                // Trim trailing \r (since \r\n is valid)
                let comment = comment.strip_suffix('\r').unwrap_or(&comment);
                // Check for any invalid characters in the comment
                if let Some(ch) = comment.bytes().find(|c| !is_toml_comment(c)) {
                    return Err(ErrorKind::IllegalChar(ch).into());
                }
            }
        }
        Ok(())
    }

    fn skip_comments_and_whitespace(&mut self) -> Result<()> {
        self.skip_whitespace()?;
        self.skip_comment()?;
        while self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")? {
            self.skip_whitespace()?;
            self.skip_comment()?;
        }
        Ok(())
    }

    fn get_table_header<'a>(parent: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
        let Some((key, path)) = path.split_last() else {
            return Some(parent);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(parent, |table, key| {
            match table.entry(key.clone()) {
                Entry::Vacant(entry) => {
                    // Create a new UndefinedTable if it doesn't exist
                    let value = entry.insert(Value::UndefinedTable(Table::with_capacity(10)));

                    // Return a mutable reference to the new table
                    debug_assert!(matches!(*value, Value::UndefinedTable(_)));
                    match *value {
                        Value::UndefinedTable(ref mut subtable) => Some(subtable),
                        _ => None, // unreachable, we just inserted an UndefinedTable
                    }
                }
                Entry::Occupied(entry) => {
                    match *entry.into_mut() {
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
                }
            }
        })?;

        // Create the table in the parent, or error if a table already exists
        let value = match parent.entry(key.clone()) {
            Entry::Vacant(entry) => {
                // Create a new Table if it doesn't exist
                entry.insert(Value::Table(Table::with_capacity(10)))
            }
            Entry::Occupied(mut entry) => {
                let Value::UndefinedTable(ref mut subtable) = *entry.get_mut() else {
                    return None; // Table already exists and is not UndefinedTable
                };

                // Pull out the subtable to take ownership of it
                let subtable = std::mem::take(subtable);
                // Replace the UndefinedTable with a Table
                entry.insert(Value::Table(subtable));

                entry.into_mut()
            }
        };

        // Return a mutable reference to the new table
        debug_assert!(matches!(*value, Value::Table(_)));
        match *value {
            Value::Table(ref mut subtable) => Some(subtable),
            _ => None, // unreachable, we just inserted an Table
        }
    }

    fn get_array_header<'a>(parent: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
        let Some((key, path)) = path.split_last() else {
            return Some(parent);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(parent, |table, key| {
            match table.entry(key.clone()) {
                Entry::Vacant(entry) => {
                    // Create a new UndefinedTable if it doesn't exist
                    let value = entry.insert(Value::UndefinedTable(Table::with_capacity(10)));

                    // Return a mutable reference to the new table
                    debug_assert!(matches!(*value, Value::UndefinedTable(_)));
                    match *value {
                        Value::UndefinedTable(ref mut subtable) => Some(subtable),
                        _ => None, // unreachable, we just inserted an UndefinedTable
                    }
                }
                Entry::Occupied(entry) => {
                    match *entry.into_mut() {
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
                }
            }
        })?;

        let value = parent
            .entry(key.clone())
            .or_insert_with(|| Value::ArrayOfTables(Vec::with_capacity(16)));
        if let Value::ArrayOfTables(ref mut subarray) = *value {
            subarray.push(Table::with_capacity(10));
            subarray.last_mut()
        } else {
            None
        }
    }

    fn get_dotted_key<'a>(parent: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
        // Navigate to the table, converting any UndefinedTables to DottedKeyTables
        path.iter().try_fold(parent, |table, key| {
            let value = table
                .entry(key.clone())
                .or_insert_with(|| Value::DottedKeyTable(Table::with_capacity(10)));

            if let Value::UndefinedTable(ref mut subtable) = *value {
                // Pull out the subtable to take ownership of it
                let subtable = std::mem::take(subtable);
                // Replace the UndefinedTable with a DottedKeyTable
                *value = Value::DottedKeyTable(subtable);
            }

            // Return a mutable reference to the new table
            match *value {
                Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                _ => None, // Table already exists and is not UndefinedTable
            }
        })
    }

    fn get_inline_subtable<'a>(parent: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
        // Navigate to the subtable with the given name for each element in the path
        path.iter().try_fold(parent, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::DottedKeyTable(Table::with_capacity(10)));
            match *entry {
                Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                _ => None,
            }
        })
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_whitespace(char: &u8) -> bool {
    matches!(*char, b'\t' | b' ')
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_whitespace_or_newline(char: &u8) -> bool {
    matches!(*char, b'\t' | b' ' | b'\r' | b'\n')
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_word(char: &u8) -> bool {
    matches!(*char, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_number_or_datetime(char: &u8) -> bool {
    matches!(
        *char,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'+' | b'-' | b'.' | b':'
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_comment(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09)
    matches!(*char, 0x09 | 0x20..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_basic_str_sans_escapes(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), the delimiter '"' (0x22), and escape
    // char '\' (0x5c)
    matches!(*char, 0x09 | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_multiline_basic_str_sans_escapes(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), '\n' (0x0a), the delimiter '"' (0x22),
    // and escape char '\' (0x5c)
    matches!(*char, 0x09 | 0x0a | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_literal_str(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), and the delimiter '\'' (0x27)
    matches!(*char, 0x09 | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_multiline_literal_str(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), '\n' (0x0a), and the delimiter '\''
    // (0x27)
    matches!(*char, 0x09 | 0x0a | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
#[inline]
const fn is_toml_legal(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), carriage return (0x0d) and newline (0x0a)
    matches!(*char, 0x09 | 0x0a | 0x0d | 0x20..=0x7e | 0x80..)
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use assert_matches::assert_matches;
    use indoc::indoc;
    use maplit::hashmap;

    use super::*;
    use crate::de::Error;

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
    fn type_to_string() {
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
        assert_eq!(Value::Integer(b"123".into()).typ(), Type::Integer);
        assert_eq!(Value::BinaryInt(b"123".into()).typ(), Type::Integer);
        assert_eq!(Value::OctalInt(b"123".into()).typ(), Type::Integer);
        assert_eq!(Value::HexInt(b"123".into()).typ(), Type::Integer);
        assert_eq!(Value::Float(b"123".into()).typ(), Type::Float);
        assert_eq!(
            Value::SpecialFloat(SpecialFloat::Infinity).typ(),
            Type::Float
        );
        assert_eq!(Value::Boolean(true).typ(), Type::Boolean);
        assert_eq!(
            Value::OffsetDatetime(OffsetDatetime::EXAMPLE).typ(),
            Type::Datetime
        );
        assert_eq!(
            Value::LocalDatetime(LocalDatetime::EXAMPLE).typ(),
            Type::Datetime
        );
        assert_eq!(Value::LocalDate(LocalDate::EXAMPLE).typ(), Type::Datetime);
        assert_eq!(Value::LocalTime(LocalTime::EXAMPLE).typ(), Type::Datetime);
        assert_eq!(Value::Array(vec![]).typ(), Type::Array);
        assert_eq!(Value::ArrayOfTables(vec![]).typ(), Type::Array);
        assert_eq!(Value::Table(Table::new()).typ(), Type::Table);
        assert_eq!(Value::InlineTable(Table::new()).typ(), Type::Table);
        assert_eq!(Value::UndefinedTable(Table::new()).typ(), Type::Table);
        assert_eq!(Value::DottedKeyTable(Table::new()).typ(), Type::Table);
    }

    #[test]
    fn parser_from_str() {
        let mut parser = Parser::from_str("foo = 123");
        assert_matches!(parser.reader.next_while(|_| true), Ok(b) if &*b == b"foo = 123"
        );
    }

    #[test]
    fn parser_from_slice() {
        let mut parser = Parser::from_slice(b"foo = 123");
        assert_matches!(parser.reader.next_while(|_| true), Ok(b) if &*b == b"foo = 123"
        );
    }

    #[test]
    fn parser_from_reader() {
        let mut parser = Parser::from_reader(b"foo = 123".as_slice());
        assert_matches!(parser.reader.next_while(|_| true), Ok(b) if &*b == b"foo = 123"
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn parser_parse() {
        let mut parser = Parser::from_str("a = 1\nb = 2");
        assert_matches!(
            parser.parse(),
            Ok(Value::Table(t)) if t == hashmap! {
                "a".into() => Value::Integer(b"1".into()),
                "b".into() => Value::Integer(b"2".into()),
            }
        );

        let mut parser = Parser::from_str("a = 1\r\nb = 2");
        assert_matches!(
            parser.parse(),
            Ok(Value::Table(t)) if t == hashmap! {
                "a".into() => Value::Integer(b"1".into()),
                "b".into() => Value::Integer(b"2".into()),
            }
        );

        let mut parser = Parser::from_str(indoc! {r#"
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
                    "dob".into() => Value::OffsetDatetime(OffsetDatetime {
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
                    }),
                }),
                "database".into() => Value::Table(hashmap! {
                    "server".into() => Value::String("192.168.1.1".into()),
                    "ports".into() => Value::Array(vec![
                        Value::Integer(b"8000".into()),
                        Value::Integer(b"8001".into()),
                        Value::Integer(b"8002".into()),
                    ]),
                    "connection_max".into() => Value::Integer(b"5000".into()),
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
                                Value::Integer(b"1".into()),
                                Value::Integer(b"2".into()),
                            ]),
                        }
                    ]),
                }),
            }
        );
    }

    #[test]
    fn parser_parse_invalid() {
        let mut parser = Parser::from_str(indoc! {r"
            a = 123
            a = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::DuplicateKey(..))));

        let mut parser = Parser::from_str(indoc! {r"
            a = 123

            [a]
            b = 456
        "});
        assert_matches!(
            parser.parse(),
            Err(Error(ErrorKind::InvalidTableHeader(..)))
        );

        let mut parser = Parser::from_str(indoc! {r"
            a = 123

            [[a]]
            b = 456
        "});
        assert_matches!(
            parser.parse(),
            Err(Error(ErrorKind::InvalidTableHeader(..)))
        );

        let mut parser = Parser::from_str(indoc! {r"
            a = 123
            a.b = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::InvalidKeyPath(..))));

        let mut parser = Parser::from_str(indoc! {r"
            [a.b]
            c = 123

            [a]
            b.d = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::InvalidKeyPath(..))));

        let mut parser = Parser::from_str(indoc! {r"
            [table]
            a.b = 123
            a.b = 456
        "});
        assert_matches!(parser.parse(), Err(Error(ErrorKind::DuplicateKey(..))));

        let mut parser = Parser::from_str("a = 123 $");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::ExpectedToken(..))));

        let mut parser = Parser::from_str("a = 123 \0");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::IllegalChar(..))));

        let mut parser = Parser::from_str("$");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::ExpectedToken(..))));

        let mut parser = Parser::from_str("\0");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::IllegalChar(..))));

        let mut parser = Parser::from_str("a = 1\rb = 2");
        assert_matches!(parser.parse(), Err(Error(ErrorKind::ExpectedToken(..))));
    }

    #[test]
    fn parser_parse_array_header() {
        let mut parser = Parser::from_str(r#" a .b. "..c"]]"#);
        assert_matches!(parser.parse_array_header(), Ok(s) if s == ["a", "b", "..c"]);

        let mut parser = Parser::from_str(r#""]]""#);
        assert_matches!(
            parser.parse_array_header(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_table_header() {
        let mut parser = Parser::from_str(r#" a .b. "..c"]"#);
        assert_matches!(parser.parse_table_header(), Ok(s) if s == ["a", "b", "..c"]);

        let mut parser = Parser::from_str(r#""]""#);
        assert_matches!(
            parser.parse_table_header(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_key_value_pair() {
        let mut parser = Parser::from_str(r"a = 123");
        assert_matches!(
            parser.parse_key_value_pair(),
            Ok((k, Value::Integer(v))) if k == ["a"] && &*v == b"123"
        );

        let mut parser = Parser::from_str(r#""a = 123""#);
        assert_matches!(
            parser.parse_key_value_pair(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_dotted_key() {
        let mut parser = Parser::from_str(r#"a .b. "..c""#);
        assert_matches!(parser.parse_dotted_key(), Ok(s) if s == ["a", "b", "..c"]);

        let mut parser = Parser::from_str(".");
        assert_matches!(
            parser.parse_dotted_key(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("a..b");
        assert_matches!(
            parser.parse_dotted_key(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_key() {
        let mut parser = Parser::from_str("abc");
        assert_matches!(parser.parse_key(), Ok(k) if k == "abc");

        let mut parser = Parser::from_str(r#""abc""#);
        assert_matches!(parser.parse_key(), Ok(k) if k == "abc");

        let mut parser = Parser::from_str("'abc'");
        assert_matches!(parser.parse_key(), Ok(k) if k == "abc");

        let mut parser = Parser::from_str(r#""""abc""""#);
        assert_matches!(parser.parse_key(), Err(Error(ErrorKind::ExpectedToken(..))));

        let mut parser = Parser::from_str("'''abc'''");
        assert_matches!(parser.parse_key(), Err(Error(ErrorKind::ExpectedToken(..))));
    }

    #[test]
    fn parser_parse_bare_key() {
        let mut parser = Parser::from_str("abc");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "abc");

        let mut parser = Parser::from_str("123");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "123");

        let mut parser = Parser::from_str("-");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "-");

        let mut parser = Parser::from_str("_");
        assert_matches!(parser.parse_bare_key(), Ok(k) if k == "_");
    }

    #[test]
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn parser_parse_value() {
        let mut parser = Parser::from_str(r#""hello""#);
        assert_matches!(parser.parse_value(), Ok(Value::String(s)) if &*s == "hello");

        let mut parser = Parser::from_str("true");
        assert_matches!(parser.parse_value(), Ok(Value::Boolean(true)));

        let mut parser = Parser::from_str("0.2");
        assert_matches!(parser.parse_value(), Ok(Value::Float(b)) if &*b == b"0.2");

        let mut parser = Parser::from_str("0x123abc");
        assert_matches!(parser.parse_value(), Ok(Value::HexInt(v)) if &*v == b"123abc");

        let mut parser = Parser::from_str("0001-01-01");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalDate(LocalDate {
                year: 1,
                month: 1,
                day: 1
            }))
        );

        let mut parser = Parser::from_str("00:00:00");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalTime(LocalTime {
                hour: 0,
                minute: 0,
                second: 0,
                nanosecond: 0
            }))
        );

        let mut parser = Parser::from_str("0");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(b)) if &*b == b"0");

        let mut parser = Parser::from_str("12");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(b)) if &*b == b"12");

        let mut parser = Parser::from_str("1234");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(b)) if &*b == b"1234");

        let mut parser = Parser::from_str("1234-05-06");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalDate(LocalDate {
                year: 1234,
                month: 5,
                day: 6
            }))
        );

        let mut parser = Parser::from_str("12:34:56");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::LocalTime(LocalTime {
                hour: 12,
                minute: 34,
                second: 56,
                nanosecond: 0
            }))
        );

        let mut parser = Parser::from_str("-123");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(v)) if &*v == b"-123");

        let mut parser = Parser::from_str("+123");
        assert_matches!(parser.parse_value(), Ok(Value::Integer(v)) if &*v == b"+123");

        let mut parser = Parser::from_str("+inf");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::Infinity))
        );

        let mut parser = Parser::from_str("-nan");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::NegNan))
        );

        let mut parser = Parser::from_str("inf");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::Infinity))
        );

        let mut parser = Parser::from_str("nan");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::SpecialFloat(SpecialFloat::Nan))
        );

        let mut parser = Parser::from_str("[123, 456, 789]");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::Array(a)) if a == [
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into()),
            ]
        );

        let mut parser = Parser::from_str("{ a = 123, b = 456, c = 789 }");
        assert_matches!(
            parser.parse_value(),
            Ok(Value::InlineTable(t)) if t == hashmap! {
                "a".into() => Value::Integer(b"123".into()),
                "b".into() => Value::Integer(b"456".into()),
                "c".into() => Value::Integer(b"789".into()),
            }
        );
    }

    #[test]
    fn parser_parse_value_invalid() {
        let mut parser = Parser::from_str("01");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = Parser::from_str("0123");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = Parser::from_str("+");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        let mut parser = Parser::from_str("blah");
        assert_matches!(
            parser.parse_value(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("\0");
        assert_matches!(parser.parse_value(), Err(Error(ErrorKind::IllegalChar(..))));

        let mut parser = Parser::from_str("");
        assert_matches!(parser.parse_value(), Err(Error(ErrorKind::UnexpectedEof)));
    }

    #[test]
    fn parser_parse_string() {
        let mut parser = Parser::from_str(indoc! {r#"
            "hello"
        "#});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello");

        let mut parser = Parser::from_str(indoc! {r#"
            """
            hello
            """
        "#});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello\n");

        let mut parser = Parser::from_str(indoc! {r"
            'hello'
        "});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello");

        let mut parser = Parser::from_str(indoc! {r"
            '''
            hello
            '''
        "});
        assert_matches!(parser.parse_string(), Ok(s) if s == "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            "hello'
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            """
            hello
            "
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            """
            hello
            '''
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            'hello"
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            '''
            hello
            "
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            '''
            hello
            """
        "#});
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str("hello");
        assert_matches!(
            parser.parse_string(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_basic_str() {
        let mut parser = Parser::from_str(indoc! {r#"
            hello\n"
        "#});
        assert_matches!(parser.parse_basic_str(), Ok(s) if s =="hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello\"
        "#});
        assert_matches!(
            parser.parse_basic_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            hello\0"
        "#});
        assert_matches!(
            parser.parse_basic_str(),
            Err(Error(ErrorKind::InvalidEscape(..)))
        );

        let mut parser = Parser::from_str("hello\0\"");
        assert_matches!(
            parser.parse_basic_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_parse_multiline_basic_str() {
        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """"
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """""
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"\"");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """"""
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"\""); // Still only 2 "s

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            ""
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n\"\"\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello\t
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\t\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello \
            world
            """
        "#});
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello world\n");

        let mut parser = Parser::from_str(concat!(
            indoc! {r#"
            hello \             "#}, // Use concat to avoid trimming trailing space after the \
            indoc! {r#"

                world
                """
            "#}
        ));
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello world\n");

        let mut parser = Parser::from_str("hello\r\n\"\"\"");
        assert_matches!(parser.parse_multiline_basic_str(), Ok(s) if s == "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            ""
        "#});
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str(indoc! {r#"
            hello\    \
            """
        "#});
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::InvalidEscape(..)))
        );

        let mut parser = Parser::from_str("hello\0\"");
        assert_matches!(
            parser.parse_multiline_basic_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_parse_literal_str() {
        let mut parser = Parser::from_str("hello\\n'");
        assert_matches!(parser.parse_literal_str(), Ok(s) if s == "hello\\n");

        let mut parser = Parser::from_str("hello\n'");
        assert_matches!(
            parser.parse_literal_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str("hello\0'");
        assert_matches!(
            parser.parse_literal_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    fn parser_parse_multiline_literal_str() {
        let mut parser = Parser::from_str(indoc! {r"
            hello
            '''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n'");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            '''''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n''");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''''''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n''"); // Still only 2 's

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''
            '''
        "});
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n''\n");

        let mut parser = Parser::from_str("hello\r\n'''");
        assert_matches!(parser.parse_multiline_literal_str(), Ok(s) if s == "hello\n");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''
        "});
        assert_matches!(
            parser.parse_multiline_literal_str(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str("hello\0'");
        assert_matches!(
            parser.parse_multiline_literal_str(),
            Err(Error(ErrorKind::IllegalChar(..)))
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn parser_parse_escape_seq() {
        let mut parser = Parser::from_str("b");
        assert_matches!(parser.parse_escape_seq(), Ok('\x08'));

        let mut parser = Parser::from_str("t");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\t');

        let mut parser = Parser::from_str("n");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\n');

        let mut parser = Parser::from_str("f");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\x0c');

        let mut parser = Parser::from_str("r");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\r');

        let mut parser = Parser::from_str("\"");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '"');

        let mut parser = Parser::from_str("\\");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '\\');

        let mut parser = Parser::from_str("u20ac");
        assert_matches!(parser.parse_escape_seq(), Ok(s) if s == '€');

        let mut parser = Parser::from_str("u2");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str("ulmao");
        assert_matches!(parser.parse_escape_seq(), Err(Error(ErrorKind::InvalidEscape(esc))) if &*esc == "\\ulmao");

        let mut parser = Parser::from_slice(b"u\xff\xff\xff\xff");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::InvalidEncoding))
        );

        let mut parser = Parser::from_str("U0001f60e");
        assert_matches!(parser.parse_escape_seq(), Ok('😎'));

        let mut parser = Parser::from_str("U2");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str("UROFLCOPTER");
        assert_matches!(parser.parse_escape_seq(), Err(Error(ErrorKind::InvalidEscape(esc))) if &*esc == "\\UROFLCOPT");

        let mut parser = Parser::from_slice(b"U\xff\xff\xff\xff\xff\xff\xff\xff");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::InvalidEncoding))
        );

        let mut parser = Parser::from_slice(b"");
        assert_matches!(
            parser.parse_escape_seq(),
            Err(Error(ErrorKind::UnterminatedString))
        );

        let mut parser = Parser::from_str("p");
        assert_matches!(parser.parse_escape_seq(), Err(Error(ErrorKind::InvalidEscape(esc))) if &*esc == "\\p");
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn parser_parse_bool() {
        let mut parser = Parser::from_str("true");
        assert_matches!(parser.parse_bool(), Ok(true));

        let mut parser = Parser::from_str("false");
        assert_matches!(parser.parse_bool(), Ok(false));

        let mut parser = Parser::from_str("TRUE");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("f");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("1");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("trueueue");
        assert_matches!(
            parser.parse_bool(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_number_or_datetime() {
        let mut parser = Parser::from_str("0x123");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::HexInt(_)));

        let mut parser = Parser::from_str("0o123");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::OctalInt(_)));

        let mut parser = Parser::from_str("0b101");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::BinaryInt(_)));

        let mut parser = Parser::from_str("1980-01-01T12:00:00.000+02:30");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::OffsetDatetime(_))
        );

        let mut parser = Parser::from_str("1980-01-01 12:00:00Z");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::OffsetDatetime(_))
        );

        let mut parser = Parser::from_str("1980-01-01T12:00:00.000");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::LocalDatetime(_))
        );

        let mut parser = Parser::from_str("1980-01-01 12:00:00");
        assert_matches!(
            parser.parse_number_or_datetime(),
            Ok(Value::LocalDatetime(_))
        );

        let mut parser = Parser::from_str("1980-01-01");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::LocalDate(_)));

        let mut parser = Parser::from_str("12:00:00.000000000");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::LocalTime(_)));

        let mut parser = Parser::from_str("123");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::Integer(_)));

        let mut parser = Parser::from_str("4.5");
        assert_matches!(parser.parse_number_or_datetime(), Ok(Value::Float(_)));
    }

    #[test]
    fn parse_number_decimal() {
        let mut parser = Parser::from_str("123");
        assert_matches!(
            parser.parse_number_decimal(),
            Ok(Value::Integer(v)) if &*v == b"123"
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn parser_normalize_number_decimal() {
        type Parser<'a> = super::Parser<'a, SliceReader<'a>>;

        assert_matches!(
            Parser::normalize_number_decimal(b"123_456".into()),
            Ok(Value::Integer(v)) if &*v == b"123456"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"+123_456".into()),
            Ok(Value::Integer(v)) if &*v == b"+123456"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"-123_456".into()),
            Ok(Value::Integer(v)) if &*v == b"-123456"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"0".into()),
            Ok(Value::Integer(v)) if &*v == b"0"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"0123".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"abc".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"-abc".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123_456.789_012".into()),
            Ok(Value::Float(v)) if &*v == b"123456.789012"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123_456.789_012e345_678".into()),
            Ok(Value::Float(v)) if &*v == b"123456.789012e345678"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123_456.789_012e+345_678".into()),
            Ok(Value::Float(v)) if &*v == b"123456.789012e+345678"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123_456.789_012e-345_678".into()),
            Ok(Value::Float(v)) if &*v == b"123456.789012e-345678"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123_456e345_678".into()),
            Ok(Value::Float(v)) if &*v == b"123456e345678"
        );

        assert_matches!(
            Parser::normalize_number_decimal(b".123".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123.".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123e".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"e123".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123.e456".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123e456.789".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123.abc".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number_decimal(b"123.456eabc".into()),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );
    }

    #[test]
    fn parser_normalize_number() {
        type Parser<'a> = super::Parser<'a, SliceReader<'a>>;

        assert_matches!(
            Parser::normalize_number(b"123_456".into(), u8::is_ascii_digit),
            Ok(v) if &*v == b"123456"
        );

        assert_matches!(
            Parser::normalize_number(b"abc_def".into(), u8::is_ascii_hexdigit),
            Ok(v) if &*v == b"abcdef"
        );

        assert_matches!(
            Parser::normalize_number(b"abc_def".into(), u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::normalize_number(b"_123_".into(), u8::is_ascii_digit),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );
    }

    #[test]
    fn parser_check_underscores() {
        type Parser<'a> = super::Parser<'a, SliceReader<'a>>;

        assert!(Parser::check_underscores(b"123_456").is_ok());

        assert_matches!(
            Parser::check_underscores(b"_123"),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::check_underscores(b"123_"),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::check_underscores(b"123__456"),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );

        assert_matches!(
            Parser::check_underscores(b""),
            Err(Error(ErrorKind::InvalidNumber(..)))
        );
    }

    #[test]
    fn parser_strip_underscores() {
        type Parser<'a> = super::Parser<'a, SliceReader<'a>>;

        assert_eq!(Parser::strip_underscores(b"123456".into()), b"123456");

        assert_eq!(Parser::strip_underscores(b"123_456".into()), b"123456");

        assert_eq!(
            Parser::strip_underscores(b"123_456_789".into()),
            b"123456789"
        );

        assert_eq!(
            Parser::strip_underscores(b"123_456_789_abc".into()),
            b"123456789abc"
        );
    }

    #[test]
    fn parser_parse_number_special() {
        let mut parser = Parser::from_str("inf");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Infinity));

        let mut parser = Parser::from_str("+inf");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Infinity));

        let mut parser = Parser::from_str("-inf");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::NegInfinity));

        let mut parser = Parser::from_str("nan");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Nan));

        let mut parser = Parser::from_str("+nan");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::Nan));

        let mut parser = Parser::from_str("-nan");
        assert_matches!(parser.parse_number_special(), Ok(SpecialFloat::NegNan));

        let mut parser = Parser::from_str("+1.0e+3");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("NaN");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("INF");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("abc");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("+abc");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("-abc");
        assert_matches!(
            parser.parse_number_special(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_array() {
        let mut parser = Parser::from_str("]");
        assert_matches!(parser.parse_array(), Ok(s) if s.is_empty());

        let mut parser = Parser::from_str("  ]");
        assert_matches!(parser.parse_array(), Ok(s) if s.is_empty());

        let mut parser = Parser::from_str(indoc! {r"
                # comment
            ]
        "});
        assert_matches!(parser.parse_array(), Ok(s) if s.is_empty());

        let mut parser = Parser::from_str("123]");
        assert_matches!(
            parser.parse_array(),
            Ok(s) if s == [Value::Integer(b"123".into())]
        );

        let mut parser = Parser::from_str("123,]");
        assert_matches!(
            parser.parse_array(),
            Ok(s) if s == [Value::Integer(b"123".into())]
        );

        let mut parser = Parser::from_str(indoc! {r"
                123,
            ]
        "});
        assert_matches!(
            parser.parse_array(),
            Ok(s) if s == [Value::Integer(b"123".into())]
        );

        let mut parser = Parser::from_str(r"123, 456, 789]");
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into())
            ]
        );

        let mut parser = Parser::from_str(r"123, 456, 789,]");
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into())
            ]
        );

        let mut parser = Parser::from_str(indoc! {r"
                123,
                456,
                789,
            ]
        "});
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into())
            ]
        );

        let mut parser = Parser::from_str(indoc! {r"
                123,
                456, # comment
                789 # comment
            ]
        "});
        assert_matches!(
            parser.parse_array(),
            Ok(a) if a == [
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into())
            ]
        );

        let mut parser = Parser::from_str("123 abc]");
        assert_matches!(
            parser.parse_array(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );
    }

    #[test]
    fn parser_parse_inline_table() {
        let mut parser = Parser::from_str("}");
        assert_matches!(parser.parse_inline_table(), Ok(t) if t == Table::new());

        let mut parser = Parser::from_str("  }");
        assert_matches!(parser.parse_inline_table(), Ok(t) if t == Table::new());

        let mut parser = Parser::from_str("abc = 123 }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! { "abc".into() => Value::Integer(b"123".into()) }
        );

        let mut parser = Parser::from_str(r"abc = 123, def = 456, ghi = 789 }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! {
                "abc".into() => Value::Integer(b"123".into()),
                "def".into() => Value::Integer(b"456".into()),
                "ghi".into() => Value::Integer(b"789".into()),
            }
        );

        let mut parser = Parser::from_str(r"abc = { def = 123, ghi = 456 } }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! {
                "abc".into() => Value::InlineTable(hashmap! {
                    "def".into() => Value::Integer(b"123".into()),
                    "ghi".into() => Value::Integer(b"456".into()),
                }),
            }
        );

        let mut parser = Parser::from_str(r"abc.def = 123, abc.ghi = 456 }");
        assert_matches!(
            parser.parse_inline_table(),
            Ok(t) if t == hashmap! {
                "abc".into() => Value::DottedKeyTable(hashmap! {
                    "def".into() => Value::Integer(b"123".into()),
                    "ghi".into() => Value::Integer(b"456".into()),
                }),
            }
        );

        let mut parser = Parser::from_str("abc 123 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("abc = 123, }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("123 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str(indoc! {r"
                abc = 123
            }
        "});
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::ExpectedToken(..)))
        );

        let mut parser = Parser::from_str("abc = 123, abc = 456 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::DuplicateKey(..)))
        );

        let mut parser = Parser::from_str("abc = { def = 123 }, abc.ghi = 456 }");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::InvalidKeyPath(..)))
        );

        let mut parser = Parser::from_str("abc = 123, def = 456 ");
        assert_matches!(
            parser.parse_inline_table(),
            Err(Error(ErrorKind::UnexpectedEof))
        );
    }

    #[test]
    fn parser_skip_whitespace() {
        let mut parser = Parser::from_str("   ");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(None));

        let mut parser = Parser::from_str("   \t");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(None));

        let mut parser = Parser::from_str("   abc");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(Some(b'a')));

        let mut parser = Parser::from_str("   \t   abc");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(Some(b'a')));

        let mut parser = Parser::from_str("   \t   # comment");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(Some(b'#')));

        let mut parser = Parser::from_str("abc");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(Some(b'a')));

        let mut parser = Parser::from_str("");
        assert!(parser.skip_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(None));
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn parser_skip_comment() {
        let mut parser = Parser::from_str("# comment");
        parser.skip_comment().unwrap();
        assert_matches!(parser.reader.peek(), Ok(None));

        let mut parser = Parser::from_str("# comment\n");
        parser.skip_comment().unwrap();
        assert_matches!(parser.reader.peek(), Ok(Some(b'\n')));

        let mut parser = Parser::from_str("# comment\r\n");
        parser.skip_comment().unwrap();
        assert_matches!(parser.reader.peek(), Ok(Some(b'\n')));

        let mut parser = Parser::from_str("abc");
        parser.skip_comment().unwrap();
        assert_matches!(parser.reader.peek(), Ok(Some(b'a')));

        if cfg!(not(feature = "fast")) {
            let mut parser = Parser::from_slice(b"# comment\xff");
            assert_matches!(
                parser.skip_comment(),
                Err(Error(ErrorKind::InvalidEncoding))
            );

            let mut parser = Parser::from_str("# comment\0");
            assert_matches!(
                parser.skip_comment(),
                Err(Error(ErrorKind::IllegalChar(..)))
            );
        }
    }

    #[test]
    fn parser_skip_comments_and_whitespace() {
        let mut parser = Parser::from_str(indoc! {r"

            # comment

            abc
        "});
        assert!(parser.skip_comments_and_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(Some(b'a')));

        let mut parser = Parser::from_str("# comment\r\n\tabc");
        assert!(parser.skip_comments_and_whitespace().is_ok());
        assert_matches!(parser.reader.peek(), Ok(Some(b'a')));
    }

    #[test]
    fn parser_get_table_header() {
        type Parser<'de> = super::Parser<'de, SliceReader<'de>>;

        let map = || {
            hashmap! {
                "a".into() => Value::UndefinedTable(hashmap! {}),
                "b".into() => Value::Table(hashmap! {
                    "c".into() => Value::UndefinedTable(hashmap! {}),
                    "d".into() => Value::DottedKeyTable(hashmap! {}),
                    "e".into() => Value::Table(hashmap! {}),
                    "f".into() => Value::ArrayOfTables(vec![hashmap! {}]),
                    "g".into() => Value::InlineTable(hashmap! {}),
                    "h".into() => Value::Integer(b"123".into()),
                }),
            }
        };
        assert!(Parser::get_table_header(&mut map(), &[]).is_some());
        assert!(Parser::get_table_header(&mut map(), &["a"].map(Into::into)).is_some());
        assert!(Parser::get_table_header(&mut map(), &["a", "b"].map(Into::into)).is_some());
        assert!(Parser::get_table_header(&mut map(), &["a", "b", "c"].map(Into::into)).is_some());

        assert!(Parser::get_table_header(&mut map(), &["b", "c", "d"].map(Into::into)).is_some());
        assert!(Parser::get_table_header(&mut map(), &["b", "d", "e"].map(Into::into)).is_some());
        assert!(Parser::get_table_header(&mut map(), &["b", "e", "f"].map(Into::into)).is_some());
        assert!(Parser::get_table_header(&mut map(), &["b", "f", "g"].map(Into::into)).is_some());
        assert!(Parser::get_table_header(&mut map(), &["b", "g", "h"].map(Into::into)).is_none());
        assert!(Parser::get_table_header(&mut map(), &["b"].map(Into::into)).is_none());
    }

    #[test]
    fn table_get_array_header() {
        type Parser<'de> = super::Parser<'de, SliceReader<'de>>;

        let map = || {
            hashmap! {
                "a".into() => Value::ArrayOfTables(vec![hashmap! {}]),
                "b".into() => Value::Table(hashmap! {
                    "c".into() => Value::UndefinedTable(hashmap! {}),
                    "d".into() => Value::DottedKeyTable(hashmap! {}),
                    "e".into() => Value::Table(hashmap! {}),
                    "f".into() => Value::ArrayOfTables(vec![hashmap! {}]),
                    "g".into() => Value::InlineTable(hashmap! {}),
                    "h".into() => Value::Integer(b"123".into()),
                }),
            }
        };
        assert!(Parser::get_array_header(&mut map(), &[]).is_some());
        assert!(Parser::get_array_header(&mut map(), &["a"].map(Into::into)).is_some());
        assert!(Parser::get_array_header(&mut map(), &["a", "b"].map(Into::into)).is_some());
        assert!(Parser::get_array_header(&mut map(), &["a", "b", "c"].map(Into::into)).is_some());

        assert!(Parser::get_array_header(&mut map(), &["b", "c", "d"].map(Into::into)).is_some());
        assert!(Parser::get_array_header(&mut map(), &["b", "d", "e"].map(Into::into)).is_some());
        assert!(Parser::get_array_header(&mut map(), &["b", "e", "f"].map(Into::into)).is_some());
        assert!(Parser::get_array_header(&mut map(), &["b", "f", "g"].map(Into::into)).is_some());
        assert!(Parser::get_array_header(&mut map(), &["b", "g", "h"].map(Into::into)).is_none());
        assert!(Parser::get_array_header(&mut map(), &["b"].map(Into::into)).is_none());
    }

    #[test]
    fn table_get_dotted_key() {
        type Parser<'de> = super::Parser<'de, SliceReader<'de>>;

        let map = || {
            hashmap! {
                "a".into() => Value::DottedKeyTable(hashmap! {}),
                "b".into() => Value::UndefinedTable(hashmap! {}),
                "c".into() => Value::Table(hashmap! {}),
            }
        };
        assert!(Parser::get_dotted_key(&mut map(), &[]).is_some());
        assert!(Parser::get_dotted_key(&mut map(), &["a"].map(Into::into)).is_some());
        assert!(Parser::get_dotted_key(&mut map(), &["a", "b"].map(Into::into)).is_some());
        assert!(Parser::get_dotted_key(&mut map(), &["b"].map(Into::into)).is_some());
        assert!(Parser::get_dotted_key(&mut map(), &["b", "c"].map(Into::into)).is_some());
        assert!(Parser::get_dotted_key(&mut map(), &["c"].map(Into::into)).is_none());
    }

    #[test]
    fn table_get_inline_subtable() {
        type Parser<'de> = super::Parser<'de, SliceReader<'de>>;

        let map = || {
            hashmap! {
                "a".into() => Value::DottedKeyTable(hashmap! {}),
                "b".into() => Value::InlineTable(hashmap! {}),
                // Shouldn't actually exist in an inline table
                "c".into() => Value::UndefinedTable(hashmap! {}),
            }
        };
        assert!(Parser::get_inline_subtable(&mut map(), &[]).is_some());
        assert!(Parser::get_inline_subtable(&mut map(), &["a"].map(Into::into)).is_some());
        assert!(Parser::get_inline_subtable(&mut map(), &["a", "b"].map(Into::into)).is_some());
        assert!(
            Parser::get_inline_subtable(&mut map(), &["a", "b", "c"].map(Into::into)).is_some()
        );
        assert!(Parser::get_inline_subtable(&mut map(), &["b"].map(Into::into)).is_none());
        assert!(Parser::get_inline_subtable(&mut map(), &["b", "c"].map(Into::into)).is_none());
    }
}
