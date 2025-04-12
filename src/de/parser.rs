use core::{fmt, str};
use std::borrow::Cow;
use std::io;
use std::marker::PhantomData;

use lexical::{FromLexicalWithOptions as _, NumberFormatBuilder, ParseIntegerOptions};
use serde::de;

use super::error::{ErrorKind, Result};
use super::reader::IoReader;
use super::{Reader, SliceReader};

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
pub(super) enum Value<'de> {
    // String; any escape sequences are already parsed
    String(Cow<'de, str>),
    // Decimal integer
    Integer(Cow<'de, [u8]>),
    // Binary integer (without the 0b prefix)
    BinaryInt(Cow<'de, [u8]>),
    // Octal integer (without the 0o prefix)
    OctalInt(Cow<'de, [u8]>),
    // Hexadecimal integer (without the 0x prefix)
    HexInt(Cow<'de, [u8]>),
    // Float
    Float(Cow<'de, [u8]>),
    // Special float (inf, nan, etc)
    SpecialFloat(SpecialFloat),
    // Boolean
    Boolean(bool),
    // Offset Datetime
    OffsetDatetime(Cow<'de, [u8]>),
    // Local Datetime
    LocalDatetime(Cow<'de, [u8]>),
    // Local Date
    LocalDate(Cow<'de, [u8]>),
    // Local Time
    LocalTime(Cow<'de, [u8]>),
    // Just a regular inline array
    Array(Vec<Self>),
    // Table defined by a table header. This is immutable aside from being able to add subtables
    Table(Table<'de>),
    // Super table created when parsing a subtable header. This can still be explicitly defined
    // later turning it into a `Table`
    UndefinedTable(Table<'de>),
    // A table defined by dotted keys. This can be freely added to by other dotted keys
    DottedKeyTable(Table<'de>),
    // Inline table
    InlineTable(Table<'de>),
    // Array of tables
    ArrayOfTables(Vec<Table<'de>>),
}

impl Value<'_> {
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

pub(super) type Table<'de> = std::collections::HashMap<Cow<'de, str>, Value<'de>>;

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
    pub fn parse(&mut self) -> Result<Value<'de>> {
        let mut root = Table::new();

        // The currently opened table
        let mut table = &mut root;
        // The path to the currently opened table (used for error messages)
        let mut table_path = Vec::new();

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
                    ErrorKind::ExpectedToken("table header or key/value pair".into())
                } else {
                    ErrorKind::IllegalChar(ch)
                }
                .into());
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
                        ErrorKind::ExpectedToken("end of line".into())
                    } else {
                        ErrorKind::IllegalChar(ch)
                    }
                    .into());
                }
                None => break,
            }
        }

        Ok(Value::Table(root))
    }

    fn parse_array_header(&mut self) -> Result<Vec<Cow<'de, str>>> {
        self.skip_whitespace()?;
        let key = self.parse_dotted_key()?;

        self.skip_whitespace()?;
        self.reader
            .eat_str(b"]]")?
            .then_some(key)
            .ok_or_else(|| ErrorKind::ExpectedToken("]] after dotted key".into()).into())
    }

    fn parse_table_header(&mut self) -> Result<Vec<Cow<'de, str>>> {
        self.skip_whitespace()?;
        let key = self.parse_dotted_key()?;

        self.skip_whitespace()?;
        self.reader
            .eat_char(b']')?
            .then_some(key)
            .ok_or_else(|| ErrorKind::ExpectedToken("] after dotted key".into()).into())
    }

    fn parse_key_value_pair(&mut self) -> Result<(Vec<Cow<'de, str>>, Value<'de>)> {
        let path = self.parse_dotted_key()?;

        // Whitespace should already have been consumed by parse_dotted_key looking for another '.'
        if !self.reader.eat_char(b'=')? {
            return Err(ErrorKind::ExpectedToken("= after key".into()).into());
        }
        self.skip_whitespace()?;

        let value = self.parse_value()?;

        Ok((path, value))
    }

    fn parse_dotted_key(&mut self) -> Result<Vec<Cow<'de, str>>> {
        let mut result = vec![self.parse_key()?];

        self.skip_whitespace()?;

        while self.reader.eat_char(b'.')? {
            self.skip_whitespace()?;
            result.push(self.parse_key()?);
            self.skip_whitespace()?;
        }

        Ok(result)
    }

    fn parse_key(&mut self) -> Result<Cow<'de, str>> {
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

    fn parse_bare_key(&mut self) -> Result<Cow<'de, str>> {
        let key = self.reader.next_str_while(is_toml_word)?;

        (!key.is_empty())
            .then_some(key)
            .ok_or_else(|| ErrorKind::ExpectedToken("key".into()).into())
    }

    fn parse_value(&mut self) -> Result<Value<'de>> {
        match self.reader.peek()? {
            // String
            Some(b'"' | b'\'') => self.parse_string().map(Value::String),
            // Boolean
            Some(b't' | b'f') => self.parse_bool().map(Value::Boolean),
            // Leading 0 => either prefixed int, date/time, just 0, or invalid
            Some(b'0') => {
                match self.reader.peek_at(1)? {
                    // Floats with leading 0 before decimal/exponent
                    Some(b'.' | b'e' | b'E') => self.parse_number_decimal(),
                    // 0x, 0o, 0b, etc
                    Some(ch) if ch.is_ascii_alphabetic() => self.parse_number_radix(),
                    // Date/time or invalid number (leading 0 is not allowed)
                    Some(ch) if ch.is_ascii_digit() => {
                        // We only know whether we're parsing a datetime or a number when we see a
                        // '-' after 4 digits or a ':' after 2, so we need to look ahead here
                        let n_digits = self.reader.peek_while(u8::is_ascii_digit)?.len();
                        if (n_digits == 4 && matches!(self.reader.peek_at(4)?, Some(b'-')))
                            || (n_digits == 2 && matches!(self.reader.peek_at(2)?, Some(b':')))
                        {
                            self.parse_datetime()
                        } else {
                            Err(ErrorKind::InvalidNumber("leading zero".into()).into())
                        }
                    }
                    // Parse just the 0
                    _ => self.parse_number_decimal(),
                }
            }
            // Either date/time or number
            Some(b'1'..=b'9') => {
                // We only know whether we're parsing a datetime or a number when we see a
                // '-' after 4 digits or a ':' after 2, so we need to look ahead here
                let n_digits = self.reader.peek_while(u8::is_ascii_digit)?.len();
                if (n_digits == 4 && self.reader.peek_at(4)?.is_some_and(|ch| ch == b'-'))
                    || (n_digits == 2 && self.reader.peek_at(2)?.is_some_and(|ch| ch == b':'))
                {
                    self.parse_datetime()
                } else {
                    self.parse_number_decimal()
                }
            }
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
            Some(ch) => Err(if is_toml_legal(&ch) {
                ErrorKind::ExpectedToken("a value".into())
            } else {
                ErrorKind::IllegalChar(ch)
            }
            .into()),
            None => Err(ErrorKind::UnexpectedEof.into()),
        }
    }

    fn parse_string(&mut self) -> Result<Cow<'de, str>> {
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

    fn parse_basic_str(&mut self) -> Result<Cow<'de, str>> {
        let mut str = self.reader.next_str_while(is_toml_basic_str_sans_escapes)?;

        loop {
            match self.reader.next()? {
                Some(b'\\') => {
                    // Parse escape sequence
                    str.to_mut().push(self.parse_escape_seq()?);
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

            str.to_mut()
                .push_str(&self.reader.next_str_while(is_toml_basic_str_sans_escapes)?);
        }
    }

    fn parse_multiline_basic_str(&mut self) -> Result<Cow<'de, str>> {
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
                        self.reader.next_while(is_toml_whitespace_or_newline)?;
                    }
                    // If there's space after the \ we assume a trailing \ with trailing whitespace,
                    // but we need to verify there's only whitespace chars before the next newline
                    else if let Some(char) = self.reader.next_if(is_toml_whitespace)? {
                        self.reader.next_while(is_toml_whitespace)?;
                        if !(self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?) {
                            return Err(ErrorKind::InvalidEscape(
                                format!("{:?}", char::from(char)).into(),
                            )
                            .into());
                        }
                        self.reader.next_while(is_toml_whitespace_or_newline)?;
                    } else {
                        // Parse a regular escape sequence and continue
                        str.to_mut().push(self.parse_escape_seq()?);
                    }
                }
                Some(b'"') => {
                    // Check for 2 more '"'s
                    if self.reader.eat_str(b"\"\"")? {
                        // We can have up to 5 '"'s, 2 quotes inside the string right before the 3
                        // which close the string. So we check for 2 additional '"'s and push them
                        if self.reader.eat_char(b'"')? {
                            str.to_mut().push('"');
                            if self.reader.eat_char(b'"')? {
                                str.to_mut().push('"');
                            }
                        }

                        break Ok(str);
                    }
                    str.to_mut().push('"');
                }
                None => {
                    break Err(ErrorKind::UnterminatedString.into());
                }
                Some(b'\r') if matches!(self.reader.peek()?, Some(b'\n')) => {
                    // Ignore '\r' followed by '\n', else it's handled by the illegal char branch
                }
                Some(char) => break Err(ErrorKind::IllegalChar(char).into()),
            }

            str.to_mut().push_str(
                &self
                    .reader
                    .next_str_while(is_toml_multiline_basic_str_sans_escapes)?,
            );
        }
    }

    fn parse_literal_str(&mut self) -> Result<Cow<'de, str>> {
        let str = self.reader.next_str_while(is_toml_literal_str)?;

        match self.reader.next()? {
            Some(b'\'') => Ok(str),
            None | Some(b'\r' | b'\n') => Err(ErrorKind::UnterminatedString.into()),
            Some(char) => Err(ErrorKind::IllegalChar(char).into()),
        }
    }

    fn parse_multiline_literal_str(&mut self) -> Result<Cow<'de, str>> {
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
                            str.to_mut().push('\'');
                            if self.reader.eat_char(b'\'')? {
                                str.to_mut().push('\'');
                            }
                        }

                        break Ok(str);
                    }
                    str.to_mut().push('\'');
                }
                None => {
                    break Err(ErrorKind::UnterminatedString.into());
                }
                Some(b'\r') if matches!(self.reader.peek()?, Some(b'\n')) => {
                    // Ignore '\r' followed by '\n', else it's handled by the illegal char branch
                }
                Some(char) => break Err(ErrorKind::IllegalChar(char).into()),
            }

            str.to_mut()
                .push_str(&self.reader.next_str_while(is_toml_multiline_literal_str)?);
        }
    }

    fn parse_escape_seq(&mut self) -> Result<char> {
        const HEX_ESCAPE_FORMAT: u128 = NumberFormatBuilder::new()
            .mantissa_radix(16)
            .no_positive_mantissa_sign(true)
            .build();

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
                let options = ParseIntegerOptions::default();
                u32::from_lexical_with_options::<HEX_ESCAPE_FORMAT>(bytes.as_ref(), &options)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| {
                        str::from_utf8(bytes.as_ref()).map_or_else(
                            |_| ErrorKind::InvalidEncoding.into(),
                            |s| ErrorKind::InvalidEscape(format!("\\u{s}").into()).into(),
                        )
                    })
            }
            b'U' => {
                self.reader.discard()?;
                let bytes = self
                    .reader
                    .next_n(8)?
                    .ok_or(ErrorKind::UnterminatedString)?;
                let options = ParseIntegerOptions::default();
                u32::from_lexical_with_options::<HEX_ESCAPE_FORMAT>(bytes.as_ref(), &options)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| {
                        str::from_utf8(bytes.as_ref()).map_or_else(
                            |_| ErrorKind::InvalidEncoding.into(),
                            |s| ErrorKind::InvalidEscape(format!("\\u{s}").into()).into(),
                        )
                    })
            }
            _ => Err(ErrorKind::InvalidEscape(
                self.reader
                    .next_char()? // We want a char here, not just a byte
                    .ok_or(ErrorKind::UnterminatedString)?
                    .to_string()
                    .into(),
            )
            .into()),
        }
    }

    fn parse_bool(&mut self) -> Result<bool> {
        // Match against the whole word, don't just parse the first n characters so we don't
        // successfully parse e.g. true92864yhowkalgp98y
        let word = self.reader.next_while(is_toml_word)?;
        let result = match word.as_ref() {
            b"true" => Ok(true),
            b"false" => Ok(false),
            _ => Err(ErrorKind::ExpectedToken("true/false".into()).into()),
        };
        result
    }

    fn parse_datetime(&mut self) -> Result<Value<'de>> {
        self.reader.start_seq(); // Start sequence for datetime

        // Use the number of digits to determine whether we have a date or time
        let first_num = self.reader.peek_while(u8::is_ascii_digit)?;

        // 4 digits = year for date/datetime
        if first_num.len() == 4 {
            self.check_date()?;

            // Check if we have a time or just a date
            let have_time = match self.reader.peek()? {
                Some(b'T' | b't') => true,
                Some(b' ') if matches!(self.reader.peek_at(1)?, Some(b'0'..=b'9')) => true,
                _ => false,
            };
            if !have_time {
                // Check we're at the end of the value (i.e. no trailing invalid chars)
                if self.reader.peek()?.as_ref().is_some_and(is_toml_word) {
                    return Err(ErrorKind::InvalidDatetime.into());
                }

                return self.reader.end_seq().map(Value::LocalDate);
            }
            self.reader.discard()?; // Skip the 'T'/space

            self.check_time()?;

            // Check for the offset
            let have_offset = matches!(self.reader.peek()?, Some(b'Z' | b'z' | b'+' | b'-'));
            if !have_offset {
                // Check we're at the end of the value (i.e. no trailing invalid chars)
                if self.reader.peek()?.as_ref().is_some_and(is_toml_word) {
                    return Err(ErrorKind::InvalidDatetime.into());
                }

                return self.reader.end_seq().map(Value::LocalDatetime);
            }

            self.check_offset()?;

            // Check we're at the end of the value (i.e. no trailing invalid chars)
            if self.reader.peek()?.as_ref().is_some_and(is_toml_word) {
                return Err(ErrorKind::InvalidDatetime.into());
            }

            self.reader.end_seq().map(Value::OffsetDatetime)
        }
        // 2 digits = hour for time
        else if first_num.len() == 2 {
            self.check_time()?;

            // Check for bogus offset (offset time is not a thing in TOML)
            // Check we're at the end of the value (i.e. no trailing invalid chars)
            if matches!(self.reader.peek()?, Some(b'Z' | b'z' | b'+' | b'-'))
                || self.reader.peek()?.as_ref().is_some_and(is_toml_word)
            {
                return Err(ErrorKind::InvalidDatetime.into());
            }

            self.reader.end_seq().map(Value::LocalTime)
        }
        // Any other number of digits is invalid
        else {
            Err(ErrorKind::ExpectedToken("datetime".into()).into())
        }
    }

    fn check_date(&mut self) -> Result<()> {
        if self
            .reader
            .next_n(4)?
            .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
            && self.reader.eat_char(b'-')?
            && self
                .reader
                .next_n(2)?
                .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
            && self.reader.eat_char(b'-')?
            && self
                .reader
                .next_n(2)?
                .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
        {
            Ok(())
        } else {
            Err(ErrorKind::InvalidDatetime.into())
        }
    }

    fn check_time(&mut self) -> Result<()> {
        if !(self
            .reader
            .next_n(2)?
            .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
            && self.reader.eat_char(b':')?
            && self
                .reader
                .next_n(2)?
                .is_some_and(|a| a.iter().all(u8::is_ascii_digit)))
        {
            return Err(ErrorKind::InvalidDatetime.into());
        }

        if !self.reader.eat_char(b':')? {
            return Ok(()); // Seconds are optional, so just return hh:mm
        }
        if !self
            .reader
            .next_n(2)?
            .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
        {
            return Err(ErrorKind::InvalidDatetime.into());
        }

        if !self.reader.eat_char(b'.')? {
            return Ok(()); // Fractional seconds are also optional, so just return hh:mm:ss
        }
        if self.reader.next_while(u8::is_ascii_digit)?.is_empty() {
            return Err(ErrorKind::InvalidDatetime.into());
        }

        Ok(())
    }

    fn check_offset(&mut self) -> Result<()> {
        if self.reader.eat_char(b'Z')?
            || self.reader.eat_char(b'z')?
            || ((self.reader.eat_char(b'+')? || self.reader.eat_char(b'-')?)
                && self
                    .reader
                    .next_n(2)?
                    .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
                && self.reader.eat_char(b':')?
                && self
                    .reader
                    .next_n(2)?
                    .is_some_and(|a| a.iter().all(u8::is_ascii_digit)))
        {
            Ok(())
        } else {
            Err(ErrorKind::InvalidDatetime.into())
        }
    }

    fn parse_number_radix(&mut self) -> Result<Value<'de>> {
        if self.reader.eat_str(b"0x")? {
            Ok(Value::HexInt(
                self.reader.next_while(is_toml_word).and_then(|bytes| {
                    bytes
                        .iter()
                        .all(|byte| byte.is_ascii_hexdigit() || *byte == b'_')
                        .then_some(bytes)
                        .ok_or_else(|| {
                            ErrorKind::InvalidNumber("invalid hexadecimal digit".into()).into()
                        })
                })?,
            ))
        } else if self.reader.eat_str(b"0o")? {
            Ok(Value::OctalInt(
                self.reader.next_while(is_toml_word).and_then(|bytes| {
                    bytes
                        .iter()
                        .all(|&b| matches!(b, b'0'..=b'7' | b'_'))
                        .then_some(bytes)
                        .ok_or_else(|| {
                            ErrorKind::InvalidNumber("invalid octal digit".into()).into()
                        })
                })?,
            ))
        } else if self.reader.eat_str(b"0b")? {
            Ok(Value::BinaryInt(
                self.reader.next_while(is_toml_word).and_then(|bytes| {
                    bytes
                        .iter()
                        .all(|&b| matches!(b, b'0' | b'1' | b'_'))
                        .then_some(bytes)
                        .ok_or_else(|| {
                            ErrorKind::InvalidNumber("invalid binary digit".into()).into()
                        })
                })?,
            ))
        } else {
            Err(ErrorKind::ExpectedToken("number with radix".into()).into())
        }
    }

    fn parse_number_special(&mut self) -> Result<SpecialFloat> {
        // In each case we match against the whole word, don't just parse the first n characters so
        // we don't successfully parse e.g. inf92864yhowkalgp98y
        match self.reader.peek()? {
            Some(b'+') => {
                self.reader.discard()?;
                match self.reader.next_while(is_toml_word)?.as_ref() {
                    b"inf" => Ok(SpecialFloat::Infinity),
                    b"nan" => Ok(SpecialFloat::Nan),
                    _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
                }
            }
            Some(b'-') => {
                self.reader.discard()?;
                match self.reader.next_while(is_toml_word)?.as_ref() {
                    b"inf" => Ok(SpecialFloat::NegInfinity),
                    b"nan" => Ok(SpecialFloat::NegNan),
                    _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
                }
            }
            _ => match self.reader.next_while(is_toml_word)?.as_ref() {
                b"inf" => Ok(SpecialFloat::Infinity),
                b"nan" => Ok(SpecialFloat::Nan),
                _ => Err(ErrorKind::ExpectedToken("inf/nan".into()).into()),
            },
        }
    }

    fn parse_number_decimal(&mut self) -> Result<Value<'de>> {
        let mut float = false;

        self.reader.start_seq(); // Start sequence for number parsing

        let _ = self.reader.eat_char(b'+')? || self.reader.eat_char(b'-')?; // Optional sign

        // We need at least one digit
        // Note: leading 0s are not allowed, but that is checked in parse_value
        if self.reader.next_if(u8::is_ascii_digit)?.is_none() {
            return Err(match self.reader.peek()? {
                Some(b'_') => ErrorKind::InvalidNumber("leading underscore".into()).into(),
                _ => ErrorKind::InvalidNumber("missing digits".into()).into(),
            });
        }

        // Remainder of integer digits
        loop {
            self.reader.next_while(u8::is_ascii_digit)?;

            if self.reader.eat_char(b'_')? {
                // Need at least one digit after an '_'
                match self.reader.peek()? {
                    Some(b'0'..=b'9') => Ok(()),
                    Some(b'_') => Err(ErrorKind::InvalidNumber("double underscore".into())),
                    _ => Err(ErrorKind::InvalidNumber("trailing underscore".into())),
                }?;
            } else {
                break;
            }
        }

        // Check for decimal point
        if self.reader.eat_char(b'.')? {
            float = true;

            // We need at least one digit
            match self.reader.peek()? {
                Some(b'0'..=b'9') => Ok(()),
                Some(b'_') => Err(ErrorKind::InvalidNumber("leading underscore".into())),
                _ => Err(ErrorKind::InvalidNumber("missing digits".into())),
            }?;
            self.reader.next_while(u8::is_ascii_digit)?;

            // Remainder of integer digits
            loop {
                self.reader.next_while(u8::is_ascii_digit)?;

                if self.reader.eat_char(b'_')? {
                    // Need at least one digit after an '_'
                    match self.reader.peek()? {
                        Some(b'0'..=b'9') => Ok(()),
                        Some(b'_') => Err(ErrorKind::InvalidNumber("double underscore".into())),
                        _ => Err(ErrorKind::InvalidNumber("trailing underscore".into())),
                    }?;
                } else {
                    break;
                }
            }
        }

        // Check for exponent
        if self.reader.eat_char(b'e')? || self.reader.eat_char(b'E')? {
            float = true;

            // Optional sign
            let _ = self.reader.eat_char(b'+')? || self.reader.eat_char(b'-')?;

            // We need at least one digit
            match self.reader.peek()? {
                Some(b'0'..=b'9') => Ok(()),
                Some(b'_') => Err(ErrorKind::InvalidNumber("leading underscore".into())),
                _ => Err(ErrorKind::InvalidNumber("missing digits".into())),
            }?;
            self.reader.next_while(u8::is_ascii_digit)?;

            // Remainder of integer digits
            loop {
                self.reader.next_while(u8::is_ascii_digit)?;

                if self.reader.eat_char(b'_')? {
                    // Need at least one digit after an '_'
                    match self.reader.peek()? {
                        Some(b'0'..=b'9') => Ok(()),
                        Some(b'_') => Err(ErrorKind::InvalidNumber("double underscore".into())),
                        _ => Err(ErrorKind::InvalidNumber("trailing underscore".into())),
                    }?;
                } else {
                    break;
                }
            }
        }

        let number = self.reader.end_seq()?; // End sequence for number parsing

        Ok(if float {
            Value::Float(number)
        } else {
            Value::Integer(number)
        })
    }

    fn parse_array(&mut self) -> Result<Vec<Value<'de>>> {
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

    fn parse_inline_table(&mut self) -> Result<Table<'de>> {
        let mut result = Table::new();

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
                } else {
                    ErrorKind::UnexpectedEof
                }
                .into());
            }
        }

        Ok(result)
    }

    fn skip_whitespace(&mut self) -> Result<()> {
        while self.reader.next_if(is_toml_whitespace)?.is_some() {}
        Ok(())
    }

    fn skip_comment(&mut self) -> Result<bool> {
        if self.reader.eat_char(b'#')? {
            // Skip validating comments with feature = "fast"
            if cfg!(feature = "fast") {
                self.reader.next_while(|&ch| ch != b'\n')?;
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

            Ok(true)
        } else {
            Ok(false)
        }
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

    fn get_table_header<'a>(
        parent: &'a mut Table<'de>,
        path: &[Cow<'de, str>],
    ) -> Option<&'a mut Table<'de>> {
        let Some((key, path)) = path.split_last() else {
            return Some(parent);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(parent, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::UndefinedTable(Table::new()));
            match *entry {
                Value::Table(ref mut subtable)
                | Value::UndefinedTable(ref mut subtable)
                | Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                Value::ArrayOfTables(ref mut array) => {
                    Some(array.last_mut().unwrap_or_else(|| {
                        unreachable!("we never insert an empty array of tables")
                    }))
                }
                _ => None,
            }
        })?;

        // Create the table in the parent, or error if a table already exists
        match parent.get(key) {
            None => {
                parent.insert(key.clone(), Value::Table(Table::new()));
            }
            Some(&Value::UndefinedTable(_)) => {
                // Need to remove the entry to take ownership of the subtable
                let Some(Value::UndefinedTable(subtable)) = parent.remove(key) else {
                    unreachable!("we just checked this key")
                };
                parent.insert(key.clone(), Value::Table(subtable));
            }
            Some(_) => return None,
        }
        let Some(&mut Value::Table(ref mut subtable)) = parent.get_mut(key) else {
            unreachable!("we just inserted a Table")
        };
        Some(subtable)
    }

    fn get_array_header<'a>(
        parent: &'a mut Table<'de>,
        path: &[Cow<'de, str>],
    ) -> Option<&'a mut Table<'de>> {
        let Some((key, path)) = path.split_last() else {
            return Some(parent);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(parent, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::UndefinedTable(Table::new()));
            match *entry {
                Value::Table(ref mut subtable)
                | Value::UndefinedTable(ref mut subtable)
                | Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                Value::ArrayOfTables(ref mut array) => {
                    Some(array.last_mut().unwrap_or_else(|| {
                        unreachable!("we never insert an empty array of tables")
                    }))
                }
                _ => None,
            }
        })?;

        // Then find the array of tables in the parent, or create a new one if it doesn't exist
        if let Value::ArrayOfTables(ref mut subarray) = *parent
            .entry(key.clone())
            .or_insert_with(|| Value::ArrayOfTables(Vec::new()))
        {
            subarray.push(Table::new());
            subarray.last_mut()
        } else {
            None
        }
    }

    fn get_dotted_key<'a>(
        parent: &'a mut Table<'de>,
        path: &[Cow<'de, str>],
    ) -> Option<&'a mut Table<'de>> {
        // Navigate to the table, converting any UndefinedTables to DottedKeyTables
        path.iter().try_fold(parent, |table, key| {
            match table.get(key) {
                None => {
                    table.insert(key.clone(), Value::DottedKeyTable(Table::new()));
                }
                Some(&Value::UndefinedTable(_)) => {
                    // Need to remove the entry to take ownership of the subtable
                    let Some(Value::UndefinedTable(subtable)) = table.remove(key) else {
                        unreachable!("we just checked this key")
                    };
                    table.insert(key.clone(), Value::DottedKeyTable(subtable));
                }
                Some(&Value::DottedKeyTable(_)) => {} // Already exists
                Some(_) => return None,
            }
            let Some(&mut Value::DottedKeyTable(ref mut subtable)) = table.get_mut(key) else {
                unreachable!("we just inserted a DottedKeyTable")
            };
            Some(subtable)
        })
    }

    fn get_inline_subtable<'a>(
        parent: &'a mut Table<'de>,
        path: &[Cow<'de, str>],
    ) -> Option<&'a mut Table<'de>> {
        // Navigate to the subtable with the given name for each element in the path
        path.iter().try_fold(parent, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::DottedKeyTable(Table::new()));
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
        assert_eq!(Value::OffsetDatetime(b"foo".into()).typ(), Type::Datetime);
        assert_eq!(Value::LocalDatetime(b"foo".into()).typ(), Type::Datetime);
        assert_eq!(Value::LocalDate(b"foo".into()).typ(), Type::Datetime);
        assert_eq!(Value::LocalTime(b"foo".into()).typ(), Type::Datetime);
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
        assert_eq!(
            parser.reader.next_while(|_| true).unwrap(),
            b"foo = 123".as_slice()
        );
    }

    #[test]
    fn parser_from_slice() {
        let mut parser = Parser::from_slice(b"foo = 123");
        assert_eq!(
            parser.reader.next_while(|_| true).unwrap(),
            b"foo = 123".as_slice()
        );
    }

    #[test]
    fn parser_from_reader() {
        let mut parser = Parser::from_reader(b"foo = 123".as_slice());
        assert_eq!(
            parser.reader.next_while(|_| true).unwrap(),
            b"foo = 123".as_slice()
        );
    }

    #[test]
    fn parser_parse() {
        let mut parser = Parser::from_str("a = 1\nb = 2");
        assert_eq!(
            parser.parse().unwrap(),
            Value::Table(hashmap! {
                "a".into() => Value::Integer(b"1".into()),
                "b".into() => Value::Integer(b"2".into()),
            })
        );

        let mut parser = Parser::from_str("a = 1\r\nb = 2");
        assert_eq!(
            parser.parse().unwrap(),
            Value::Table(hashmap! {
                "a".into() => Value::Integer(b"1".into()),
                "b".into() => Value::Integer(b"2".into()),
            })
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

        assert_eq!(
            parser.parse().unwrap(),
            Value::Table(hashmap! {
                "title".into() => Value::String("TOML Example".into()),
                "owner".into() => Value::Table(hashmap! {
                    "name".into() => Value::String("Tom Preston-Werner".into()),
                    "dob".into() => Value::OffsetDatetime(b"1979-05-27T07:32:00-08:00".into()),
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
            })
        );
    }

    #[test]
    fn parser_parse_invalid() {
        let mut parser = Parser::from_str(indoc! {r"
            a = 123
            a = 456
        "});
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str(indoc! {r"
            a = 123

            [a]
            b = 456
        "});
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str(indoc! {r"
            a = 123

            [[a]]
            b = 456
        "});
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str(indoc! {r"
            a = 123
            a.b = 456
        "});
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str(indoc! {r"
            [a.b]
            c = 123

            [a]
            b.d = 456
        "});
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str(indoc! {r"
            [table]
            a.b = 123
            a.b = 456
        "});
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str("a = 123 $");
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str("a = 123 \0");
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str("$");
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str("\0");
        assert!(parser.parse().is_err());

        let mut parser = Parser::from_str("a = 1\rb = 2");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn parser_parse_array_header() {
        let mut parser = Parser::from_str(r#" a .b. "..c"]]"#);
        assert_eq!(parser.parse_array_header().unwrap(), vec!["a", "b", "..c"]);

        let mut parser = Parser::from_str(r#""]]""#);
        assert!(parser.parse_array_header().is_err());
    }

    #[test]
    fn parser_parse_table_header() {
        let mut parser = Parser::from_str(r#" a .b. "..c"]"#);
        assert_eq!(parser.parse_table_header().unwrap(), vec!["a", "b", "..c"]);

        let mut parser = Parser::from_str(r#""]""#);
        assert!(parser.parse_table_header().is_err());
    }

    #[test]
    fn parser_parse_key_value_pair() {
        let mut parser = Parser::from_str(r"a = 123");
        assert_eq!(
            parser.parse_key_value_pair().unwrap(),
            (vec!["a".into()], Value::Integer(b"123".into()))
        );

        let mut parser = Parser::from_str(r#""a = 123""#);
        assert!(parser.parse_key_value_pair().is_err());
    }

    #[test]
    fn parser_parse_dotted_key() {
        let mut parser = Parser::from_str(r#"a .b. "..c""#);
        assert_eq!(parser.parse_dotted_key().unwrap(), vec!["a", "b", "..c"]);

        let mut parser = Parser::from_str(".");
        assert!(parser.parse_dotted_key().is_err());

        let mut parser = Parser::from_str("a..b");
        assert!(parser.parse_dotted_key().is_err());
    }

    #[test]
    fn parser_parse_key() {
        let mut parser = Parser::from_str("abc");
        assert_eq!(parser.parse_key().unwrap(), "abc");

        let mut parser = Parser::from_str(r#""abc""#);
        assert_eq!(parser.parse_key().unwrap(), "abc");

        let mut parser = Parser::from_str("'abc'");
        assert_eq!(parser.parse_key().unwrap(), "abc");

        let mut parser = Parser::from_str(r#""""abc""""#);
        assert!(parser.parse_key().is_err());

        let mut parser = Parser::from_str("'''abc'''");
        assert!(parser.parse_key().is_err());
    }

    #[test]
    fn parser_parse_bare_key() {
        let mut parser = Parser::from_str("abc");
        assert_eq!(parser.parse_bare_key().unwrap(), "abc");

        let mut parser = Parser::from_str("123");
        assert_eq!(parser.parse_bare_key().unwrap(), "123");

        let mut parser = Parser::from_str("-");
        assert_eq!(parser.parse_bare_key().unwrap(), "-");

        let mut parser = Parser::from_str("_");
        assert_eq!(parser.parse_bare_key().unwrap(), "_");
    }

    #[test]
    fn parser_parse_value() {
        let mut parser = Parser::from_str(r#""hello""#);
        assert_eq!(parser.parse_value().unwrap(), Value::String("hello".into()));

        let mut parser = Parser::from_str("true");
        assert_eq!(parser.parse_value().unwrap(), Value::Boolean(true));

        let mut parser = Parser::from_str("0.2");
        assert_eq!(parser.parse_value().unwrap(), Value::Float(b"0.2".into()));

        let mut parser = Parser::from_str("0x123abc");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::HexInt(b"123abc".into())
        );

        let mut parser = Parser::from_str("0001-01-01");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::LocalDate(b"0001-01-01".into())
        );

        let mut parser = Parser::from_str("00:00:00");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::LocalTime(b"00:00:00".into())
        );

        let mut parser = Parser::from_str("0");
        assert_eq!(parser.parse_value().unwrap(), Value::Integer(b"0".into()));

        let mut parser = Parser::from_str("12");
        assert_eq!(parser.parse_value().unwrap(), Value::Integer(b"12".into()));

        let mut parser = Parser::from_str("1234");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::Integer(b"1234".into())
        );

        let mut parser = Parser::from_str("1234-56-78");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::LocalDate(b"1234-56-78".into())
        );

        let mut parser = Parser::from_str("12:34:56");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::LocalTime(b"12:34:56".into())
        );

        let mut parser = Parser::from_str("-123");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::Integer(b"-123".into())
        );

        let mut parser = Parser::from_str("+123");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::Integer(b"+123".into())
        );

        let mut parser = Parser::from_str("+inf");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::SpecialFloat(SpecialFloat::Infinity)
        );

        let mut parser = Parser::from_str("-nan");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::SpecialFloat(SpecialFloat::NegNan)
        );

        let mut parser = Parser::from_str("inf");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::SpecialFloat(SpecialFloat::Infinity)
        );

        let mut parser = Parser::from_str("nan");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::SpecialFloat(SpecialFloat::Nan)
        );

        let mut parser = Parser::from_str("[123, 456, 789]");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::Array(vec![
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into()),
            ])
        );

        let mut parser = Parser::from_str("{ a = 123, b = 456, c = 789 }");
        assert_eq!(
            parser.parse_value().unwrap(),
            Value::InlineTable(hashmap! {
                "a".into() => Value::Integer(b"123".into()),
                "b".into() => Value::Integer(b"456".into()),
                "c".into() => Value::Integer(b"789".into()),
            })
        );
    }

    #[test]
    fn parser_parse_value_invalid() {
        let mut parser = Parser::from_str("01");
        assert!(parser.parse_value().is_err());

        let mut parser = Parser::from_str("0123");
        assert!(parser.parse_value().is_err());

        let mut parser = Parser::from_str("+");
        assert!(parser.parse_value().is_err());

        let mut parser = Parser::from_str("blah");
        assert!(parser.parse_value().is_err());

        let mut parser = Parser::from_str("\0");
        assert!(parser.parse_value().is_err());

        let mut parser = Parser::from_str("");
        assert!(parser.parse_value().is_err());
    }

    #[test]
    fn parser_parse_string() {
        let mut parser = Parser::from_str(indoc! {r#"
            "hello"
        "#});
        assert_eq!(parser.parse_string().unwrap(), "hello");

        let mut parser = Parser::from_str(indoc! {r#"
            """
            hello
            """
        "#});
        assert_eq!(parser.parse_string().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r"
            'hello'
        "});
        assert_eq!(parser.parse_string().unwrap(), "hello");

        let mut parser = Parser::from_str(indoc! {r"
            '''
            hello
            '''
        "});
        assert_eq!(parser.parse_string().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            "hello'
        "#});
        assert!(parser.parse_string().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            """
            hello
            "
        "#});
        assert!(parser.parse_string().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            """
            hello
            '''
        "#});
        assert!(parser.parse_string().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            'hello"
        "#});
        assert!(parser.parse_string().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            '''
            hello
            "
        "#});
        assert!(parser.parse_string().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            '''
            hello
            """
        "#});
        assert!(parser.parse_string().is_err());

        let mut parser = Parser::from_str("hello");
        assert!(parser.parse_string().is_err());
    }

    #[test]
    fn parser_parse_basic_str() {
        let mut parser = Parser::from_str(indoc! {r#"
            hello\n"
        "#});
        assert_eq!(parser.parse_basic_str().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello\"
        "#});
        assert!(parser.parse_basic_str().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            hello\0"
        "#});
        assert!(parser.parse_basic_str().is_err());

        let mut parser = Parser::from_str("hello\0\"");
        assert!(parser.parse_basic_str().is_err());
    }

    #[test]
    fn parser_parse_multiline_basic_str() {
        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """"
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\n\"");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """""
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\n\"\"");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            """"""
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\n\"\""); // Still only 2 "s

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            ""
            """
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\n\"\"\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello\t
            """
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\t\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello \
            world
            """
        "#});
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello world\n");

        let mut parser = Parser::from_str(concat!(
            indoc! {r#"
            hello \             "#}, // Use concat to avoid trimming trailing space after the \
            indoc! {r#"

                world
                """
            "#}
        ));
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello world\n");

        let mut parser = Parser::from_str("hello\r\n\"\"\"");
        assert_eq!(parser.parse_multiline_basic_str().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r#"
            hello
            ""
        "#});
        assert!(parser.parse_multiline_basic_str().is_err());

        let mut parser = Parser::from_str(indoc! {r#"
            hello\    \
            """
        "#});
        assert!(parser.parse_multiline_basic_str().is_err());

        let mut parser = Parser::from_str("hello\0\"");
        assert!(parser.parse_multiline_basic_str().is_err());
    }

    #[test]
    fn parser_parse_literal_str() {
        let mut parser = Parser::from_str("hello\\n'");
        assert_eq!(parser.parse_literal_str().unwrap(), "hello\\n");

        let mut parser = Parser::from_str("hello\n'");
        assert!(parser.parse_literal_str().is_err());

        let mut parser = Parser::from_str("hello\0'");
        assert!(parser.parse_literal_str().is_err());
    }

    #[test]
    fn parser_parse_multiline_literal_str() {
        let mut parser = Parser::from_str(indoc! {r"
            hello
            '''
        "});
        assert_eq!(parser.parse_multiline_literal_str().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''''
        "});
        assert_eq!(parser.parse_multiline_literal_str().unwrap(), "hello\n'");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            '''''
        "});
        assert_eq!(parser.parse_multiline_literal_str().unwrap(), "hello\n''");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''''''
        "});
        assert_eq!(parser.parse_multiline_literal_str().unwrap(), "hello\n''"); // Still only 2 's

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''
            '''
        "});
        assert_eq!(parser.parse_multiline_literal_str().unwrap(), "hello\n''\n");

        let mut parser = Parser::from_str("hello\r\n'''");
        assert_eq!(parser.parse_multiline_literal_str().unwrap(), "hello\n");

        let mut parser = Parser::from_str(indoc! {r"
            hello
            ''
        "});
        assert!(parser.parse_multiline_literal_str().is_err());

        let mut parser = Parser::from_str("hello\0'");
        assert!(parser.parse_multiline_literal_str().is_err());
    }

    #[test]
    fn parser_parse_escape_seq() {
        let mut parser = Parser::from_str("b");
        assert_eq!(parser.parse_escape_seq().unwrap(), '\x08');

        let mut parser = Parser::from_str("t");
        assert_eq!(parser.parse_escape_seq().unwrap(), '\t');

        let mut parser = Parser::from_str("n");
        assert_eq!(parser.parse_escape_seq().unwrap(), '\n');

        let mut parser = Parser::from_str("f");
        assert_eq!(parser.parse_escape_seq().unwrap(), '\x0c');

        let mut parser = Parser::from_str("r");
        assert_eq!(parser.parse_escape_seq().unwrap(), '\r');

        let mut parser = Parser::from_str("\"");
        assert_eq!(parser.parse_escape_seq().unwrap(), '"');

        let mut parser = Parser::from_str("\\");
        assert_eq!(parser.parse_escape_seq().unwrap(), '\\');

        let mut parser = Parser::from_str("u20ac");
        assert_eq!(parser.parse_escape_seq().unwrap(), '€');

        let mut parser = Parser::from_str("u2");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_str("ulmao");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_slice(b"u\xff\xff\xff\xff");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_str("U0001f60e");
        assert_eq!(parser.parse_escape_seq().unwrap(), '😎');

        let mut parser = Parser::from_str("U2");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_str("UROFLCOPTER");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_slice(b"U\xff\xff\xff\xff\xff\xff\xff\xff");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_slice(b"");
        assert!(parser.parse_escape_seq().is_err());

        let mut parser = Parser::from_str("p");
        assert!(parser.parse_escape_seq().is_err());
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn parser_parse_bool() {
        let mut parser = Parser::from_str("true");
        assert_eq!(parser.parse_bool().unwrap(), true);

        let mut parser = Parser::from_str("false");
        assert_eq!(parser.parse_bool().unwrap(), false);

        let mut parser = Parser::from_str("TRUE");
        assert!(parser.parse_bool().is_err());

        let mut parser = Parser::from_str("f");
        assert!(parser.parse_bool().is_err());

        let mut parser = Parser::from_str("1");
        assert!(parser.parse_bool().is_err());

        let mut parser = Parser::from_str("trueueue");
        assert!(parser.parse_bool().is_err());
    }

    #[test]
    fn parser_parse_datetime() {
        let mut parser = Parser::from_str("1980-01-01T12:00:00.000000000+02:30");
        assert_matches!(parser.parse_datetime().unwrap(), Value::OffsetDatetime(_));

        let mut parser = Parser::from_str("1980-01-01 12:00:00.000000000+02:30");
        assert_matches!(parser.parse_datetime().unwrap(), Value::OffsetDatetime(_));

        let mut parser = Parser::from_str("1980-01-01T12:00:00.000000000Z");
        assert_matches!(parser.parse_datetime().unwrap(), Value::OffsetDatetime(_));

        let mut parser = Parser::from_str("1980-01-01 12:00:00.000000000Z");
        assert_matches!(parser.parse_datetime().unwrap(), Value::OffsetDatetime(_));

        let mut parser = Parser::from_str("1980-01-01T12:00:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDatetime(_));

        let mut parser = Parser::from_str("1980-01-01 12:00:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDatetime(_));

        let mut parser = Parser::from_str("1980-01-01T12:00:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDatetime(_));

        let mut parser = Parser::from_str("1980-01-01 12:00:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDatetime(_));

        let mut parser = Parser::from_str("1980-01-01T12:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDatetime(_));

        let mut parser = Parser::from_str("1980-01-01 12:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDatetime(_));

        let mut parser = Parser::from_str("1980-01-01");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalDate(_));

        let mut parser = Parser::from_str("12:00:00.000000000");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalTime(_));

        let mut parser = Parser::from_str("12:00:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalTime(_));

        let mut parser = Parser::from_str("12:00");
        assert_matches!(parser.parse_datetime().unwrap(), Value::LocalTime(_));

        let mut parser = Parser::from_str("1980-01-01T12:00:00.000000000+02:30abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("1980-01-01T12:00:00.000000000Zabc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("1980-01-01T12:00:00.000000000abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("1980-01-01T12:00:00abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("1980-01-01T12:00abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("1980-01-01abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("12:00:00.000000000abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("12:00:00abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("12:00abc");
        assert!(parser.parse_datetime().is_err());

        let mut parser = Parser::from_str("abc");
        assert!(parser.parse_datetime().is_err());
    }

    #[test]
    fn parser_check_date() {
        let mut parser = Parser::from_str("1980-01-01");
        assert!(parser.check_date().is_ok());

        let mut parser = Parser::from_str("1980-01-01abc"); // Shouldn't care about what comes after
        assert!(parser.check_date().is_ok());

        let mut parser = Parser::from_str("1980");
        assert!(parser.check_date().is_err());

        let mut parser = Parser::from_str("1980-01");
        assert!(parser.check_date().is_err());

        let mut parser = Parser::from_str("198-01-01");
        assert!(parser.check_date().is_err());
    }

    #[test]
    fn parser_check_time() {
        let mut parser = Parser::from_str("12:00:00.000000000");
        assert!(parser.check_time().is_ok());

        let mut parser = Parser::from_str("12:00:00");
        assert!(parser.check_time().is_ok());

        let mut parser = Parser::from_str("12:00");
        assert!(parser.check_time().is_ok());

        let mut parser = Parser::from_str("12:00:00abc"); // Shouldn't care about what comes after
        assert!(parser.check_time().is_ok());

        let mut parser = Parser::from_str("12:00:00.abc");
        assert!(parser.check_time().is_err());

        let mut parser = Parser::from_str("12:00:abc");
        assert!(parser.check_time().is_err());

        let mut parser = Parser::from_str("198:01:01");
        assert!(parser.check_time().is_err());
    }

    #[test]
    fn parser_check_offset() {
        let mut parser = Parser::from_str("Z");
        assert!(parser.check_offset().is_ok());

        let mut parser = Parser::from_str("z");
        assert!(parser.check_offset().is_ok());

        let mut parser = Parser::from_str("+02:30");
        assert!(parser.check_offset().is_ok());

        let mut parser = Parser::from_str("-02:30");
        assert!(parser.check_offset().is_ok());

        let mut parser = Parser::from_str("02:30");
        assert!(parser.check_offset().is_err());

        let mut parser = Parser::from_str("+002:30");
        assert!(parser.check_offset().is_err());
    }

    #[test]
    fn parser_parse_number_radix() {
        let mut parser = Parser::from_str("0x123");
        assert_eq!(
            parser.parse_number_radix().unwrap(),
            Value::HexInt(b"123".into())
        );

        let mut parser = Parser::from_str("0o123");
        assert_eq!(
            parser.parse_number_radix().unwrap(),
            Value::OctalInt(b"123".into())
        );

        let mut parser = Parser::from_str("0b101");
        assert_eq!(
            parser.parse_number_radix().unwrap(),
            Value::BinaryInt(b"101".into())
        );

        let mut parser = Parser::from_str("0X123");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("0O123");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("0B101");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("0xabcdefg");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("0o12345678");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("0b012");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("0q123abc");
        assert!(parser.parse_number_radix().is_err());

        let mut parser = Parser::from_str("123");
        assert!(parser.parse_number_radix().is_err());
    }

    #[test]
    fn parser_parse_number_special() {
        let mut parser = Parser::from_str("inf");
        assert_eq!(
            parser.parse_number_special().unwrap(),
            SpecialFloat::Infinity
        );

        let mut parser = Parser::from_str("+inf");
        assert_eq!(
            parser.parse_number_special().unwrap(),
            SpecialFloat::Infinity
        );

        let mut parser = Parser::from_str("-inf");
        assert_eq!(
            parser.parse_number_special().unwrap(),
            SpecialFloat::NegInfinity
        );

        let mut parser = Parser::from_str("nan");
        assert_eq!(parser.parse_number_special().unwrap(), SpecialFloat::Nan);

        let mut parser = Parser::from_str("+nan");
        assert_eq!(parser.parse_number_special().unwrap(), SpecialFloat::Nan);

        let mut parser = Parser::from_str("-nan");
        assert_eq!(parser.parse_number_special().unwrap(), SpecialFloat::NegNan);

        let mut parser = Parser::from_str("+1.0e+3");
        assert!(parser.parse_number_special().is_err());

        let mut parser = Parser::from_str("NaN");
        assert!(parser.parse_number_special().is_err());

        let mut parser = Parser::from_str("INF");
        assert!(parser.parse_number_special().is_err());

        let mut parser = Parser::from_str("abc");
        assert!(parser.parse_number_special().is_err());

        let mut parser = Parser::from_str("+abc");
        assert!(parser.parse_number_special().is_err());

        let mut parser = Parser::from_str("-abc");
        assert!(parser.parse_number_special().is_err());
    }

    #[test]
    fn parser_parse_number_decimal_int() {
        let mut parser = Parser::from_str("123");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Integer(b"123".into())
        );

        let mut parser = Parser::from_str("+123");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Integer(b"+123".into())
        );

        let mut parser = Parser::from_str("-123");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Integer(b"-123".into())
        );

        let mut parser = Parser::from_str("123_456_789");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Integer(b"123_456_789".into())
        );

        let mut parser = Parser::from_str("_123_456");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123_456_");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123__456");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("e123");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("abc");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("+abc");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("-abc");
        assert!(parser.parse_number_decimal().is_err());
    }

    #[test]
    fn parser_parse_number_decimal_float() {
        let mut parser = Parser::from_str("123.456");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"123.456".into())
        );

        let mut parser = Parser::from_str("+123.456");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"+123.456".into())
        );

        let mut parser = Parser::from_str("-123.456");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"-123.456".into())
        );

        let mut parser = Parser::from_str("123.456e+3");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"123.456e+3".into())
        );

        let mut parser = Parser::from_str("123.456e-3");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"123.456e-3".into())
        );

        let mut parser = Parser::from_str("123.456e3");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"123.456e3".into())
        );

        let mut parser = Parser::from_str("123_456.123_456");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"123_456.123_456".into())
        );

        let mut parser = Parser::from_str("1.23e456_789");
        assert_eq!(
            parser.parse_number_decimal().unwrap(),
            Value::Float(b"1.23e456_789".into())
        );

        let mut parser = Parser::from_str("_123.456");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123_.456");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123._456");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123.456_");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123__456.789");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123.456__789");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("1.23e_456_789");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("1.23e456_789_");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("1.23e456__789");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str(".123");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("123.");
        assert!(parser.parse_number_decimal().is_err());

        let mut parser = Parser::from_str("1.23e");
        assert!(parser.parse_number_decimal().is_err());
    }

    #[test]
    fn parser_parse_array() {
        let mut parser = Parser::from_str("]");
        assert_eq!(parser.parse_array().unwrap(), vec![]);

        let mut parser = Parser::from_str("  ]");
        assert_eq!(parser.parse_array().unwrap(), vec![]);

        let mut parser = Parser::from_str(indoc! {r"
                # comment
            ]
        "});
        assert_eq!(parser.parse_array().unwrap(), vec![]);

        let mut parser = Parser::from_str("123]");
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![Value::Integer(b"123".into())]
        );

        let mut parser = Parser::from_str("123,]");
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![Value::Integer(b"123".into())]
        );

        let mut parser = Parser::from_str(indoc! {r"
                123,
            ]
        "});
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![Value::Integer(b"123".into())]
        );

        let mut parser = Parser::from_str(r"123, 456, 789]");
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into())
            ]
        );

        let mut parser = Parser::from_str(r"123, 456, 789,]");
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![
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
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![
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
        assert_eq!(
            parser.parse_array().unwrap(),
            vec![
                Value::Integer(b"123".into()),
                Value::Integer(b"456".into()),
                Value::Integer(b"789".into())
            ]
        );

        let mut parser = Parser::from_str("123 abc]");
        assert!(parser.parse_array().is_err());
    }

    #[test]
    fn parser_parse_inline_table() {
        let mut parser = Parser::from_str("}");
        assert_eq!(parser.parse_inline_table().unwrap(), Table::new());

        let mut parser = Parser::from_str("  }");
        assert_eq!(parser.parse_inline_table().unwrap(), Table::new());

        let mut parser = Parser::from_str("abc = 123 }");
        assert_eq!(
            parser.parse_inline_table().unwrap(),
            hashmap! { "abc".into() => Value::Integer(b"123".into()) }
        );

        let mut parser = Parser::from_str(r"abc = 123, def = 456, ghi = 789 }");
        assert_eq!(
            parser.parse_inline_table().unwrap(),
            hashmap! {
                "abc".into() => Value::Integer(b"123".into()),
                "def".into() => Value::Integer(b"456".into()),
                "ghi".into() => Value::Integer(b"789".into()),
            }
        );

        let mut parser = Parser::from_str(r"abc = { def = 123, ghi = 456 } }");
        assert_eq!(
            parser.parse_inline_table().unwrap(),
            hashmap! {
                "abc".into() => Value::InlineTable(hashmap! {
                    "def".into() => Value::Integer(b"123".into()),
                    "ghi".into() => Value::Integer(b"456".into()),
                }),
            }
        );

        let mut parser = Parser::from_str(r"abc.def = 123, abc.ghi = 456 }");
        assert_eq!(
            parser.parse_inline_table().unwrap(),
            hashmap! {
                "abc".into() => Value::DottedKeyTable(hashmap! {
                    "def".into() => Value::Integer(b"123".into()),
                    "ghi".into() => Value::Integer(b"456".into()),
                }),
            }
        );

        let mut parser = Parser::from_str("abc 123 }");
        assert!(parser.parse_inline_table().is_err());

        let mut parser = Parser::from_str("abc = 123, }");
        assert!(parser.parse_inline_table().is_err());

        let mut parser = Parser::from_str("123 }");
        assert!(parser.parse_inline_table().is_err());

        let mut parser = Parser::from_str(indoc! {r"
                abc = 123
            }
        "});
        assert!(parser.parse_inline_table().is_err());

        let mut parser = Parser::from_str("abc = 123, abc = 456 }");
        assert!(parser.parse_inline_table().is_err());

        let mut parser = Parser::from_str("abc = { def = 123 }, abc.ghi = 456 }");
        assert!(parser.parse_inline_table().is_err());

        let mut parser = Parser::from_str("abc = 123, def = 456 ");
        assert!(parser.parse_inline_table().is_err());
    }

    #[test]
    fn parser_skip_whitespace() {
        let mut parser = Parser::from_str("   ");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), None);

        let mut parser = Parser::from_str("   \t");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), None);

        let mut parser = Parser::from_str("   abc");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), Some(b'a'));

        let mut parser = Parser::from_str("   \t   abc");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), Some(b'a'));

        let mut parser = Parser::from_str("   \t   # comment");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), Some(b'#'));

        let mut parser = Parser::from_str("abc");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), Some(b'a'));

        let mut parser = Parser::from_str("");
        assert!(parser.skip_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), None);
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)]
    fn parser_skip_comment() {
        let mut parser = Parser::from_str("# comment");
        assert_eq!(parser.skip_comment().unwrap(), true);
        assert_eq!(parser.reader.peek().unwrap(), None);

        let mut parser = Parser::from_str("# comment\n");
        assert_eq!(parser.skip_comment().unwrap(), true);
        assert_eq!(parser.reader.peek().unwrap(), Some(b'\n'));

        let mut parser = Parser::from_str("# comment\r\n");
        assert_eq!(parser.skip_comment().unwrap(), true);
        assert_eq!(parser.reader.peek().unwrap(), Some(b'\n'));

        let mut parser = Parser::from_str("abc");
        assert_eq!(parser.skip_comment().unwrap(), false);
        assert_eq!(parser.reader.peek().unwrap(), Some(b'a'));

        if cfg!(not(feature = "fast")) {
            let mut parser = Parser::from_slice(b"# comment\xff");
            assert!(parser.skip_comment().is_err());

            let mut parser = Parser::from_str("# comment\0");
            assert!(parser.skip_comment().is_err());
        }
    }

    #[test]
    fn parser_skip_comments_and_whitespace() {
        let mut parser = Parser::from_str(indoc! {r"

            # comment

            abc
        "});
        assert!(parser.skip_comments_and_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), Some(b'a'));

        let mut parser = Parser::from_str("# comment\r\n\tabc");
        assert!(parser.skip_comments_and_whitespace().is_ok());
        assert_eq!(parser.reader.peek().unwrap(), Some(b'a'));
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
