use core::{fmt, str};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;

use lexical::{FromLexicalWithOptions, NumberFormatBuilder, ParseIntegerOptions};
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl From<Type> for de::Unexpected<'_> {
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
    Table(HashMap<Cow<'de, str>, Self>),
    // Super table created when parsing a subtable header. This can still be explicitly defined
    // later turning it into a `Table`
    UndefinedTable(HashMap<Cow<'de, str>, Self>),
    // A table defined by dotted keys. This can be freely added to by other dotted keys
    DottedKeyTable(HashMap<Cow<'de, str>, Self>),
    // Inline table
    InlineTable(HashMap<Cow<'de, str>, Self>),
    // Array of tables
    ArrayOfTables(Vec<HashMap<Cow<'de, str>, Self>>),
}

impl Value<'_> {
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

#[derive(Debug)]
pub(super) struct Parser<'de, R: Reader<'de>> {
    reader: R,
    _phantom: PhantomData<&'de ()>,
}

impl<'de> Parser<'de, SliceReader<'de>> {
    #[must_use]
    pub fn from_str(str: &'de str) -> Self {
        Self {
            reader: SliceReader::from_str(str),
            _phantom: PhantomData,
        }
    }

    #[must_use]
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
        let mut root = HashMap::new();

        // The currently opened table
        let mut table = &mut root;
        // The path to the currently opened table (used for error messages)
        let mut table_path = Vec::new();

        loop {
            self.skip_comments_and_whitespace()?;

            // Parse array header
            if self.reader.eat_str(b"[[")? {
                let key = self.parse_array_header()?;
                table = root
                    .get_array_header(&key)
                    .ok_or_else(|| ErrorKind::InvalidTableHeader(key.join(".").into()))?;
                table_path = key;
            }
            // Parse table header
            else if self.reader.eat_char(b'[')? {
                let key = self.parse_table_header()?;
                table = root
                    .get_table_header(&key)
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
                let subtable = table.get_dotted_key(path).ok_or_else(|| {
                    ErrorKind::InvalidKeyPath(
                        full_key.join(".").into(),
                        (!table_path.is_empty())
                            .then(|| table_path.join(".").into())
                            .unwrap_or_else(|| "root table".into()),
                    )
                })?;

                // Check if the key is already present
                if subtable.contains_key(key) {
                    return Err(ErrorKind::DuplicateKey(
                        full_key.join(".").into(),
                        (!table_path.is_empty())
                            .then(|| table_path.join(".").into())
                            .unwrap_or_else(|| "root table".into()),
                    )
                    .into());
                }
                subtable.insert(key.clone(), value);
            }
            // Anything else is unexpected
            else if let Some(ch) = self.reader.peek()? {
                self.reader.discard()?;
                return Err(if is_toml_legal(&ch) {
                    ErrorKind::ExpectedToken("table header or key/value pair".into())
                } else {
                    ErrorKind::IllegalChar(ch)
                }
                .into());
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
            self.parse_basic_str()
        } else if self.reader.eat_char(b'\'')? {
            self.parse_literal_str()
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
                    continue;
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
                    continue;
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
                    .next_char()?
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
            } else if !self.reader.eat_char(b',')? {
                return Err(ErrorKind::ExpectedToken(", or ] after value in array".into()).into());
            }
        }

        Ok(result)
    }

    fn parse_inline_table(&mut self) -> Result<HashMap<Cow<'de, str>, Value<'de>>> {
        let mut result = HashMap::new();

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
            let subtable = result.get_inline_subtable(path).ok_or_else(|| {
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
}

trait TomlTable<'a>: 'a {
    fn get_table_header(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self>;
    fn get_array_header(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self>;
    fn get_dotted_key(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self>;
    fn get_inline_subtable(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self>;
}

impl<'a> TomlTable<'a> for HashMap<Cow<'a, str>, Value<'a>> {
    fn get_table_header(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self> {
        let Some((key, path)) = path.split_last() else {
            return Some(self);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::UndefinedTable(Self::new()));
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
                parent.insert(key.clone(), Value::Table(Self::new()));
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

    fn get_array_header(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self> {
        let Some((key, path)) = path.split_last() else {
            return Some(self);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::UndefinedTable(Self::new()));
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
            subarray.push(Self::new());
            subarray.last_mut()
        } else {
            None
        }
    }

    fn get_dotted_key(&mut self, path: &[Cow<'a, str>]) -> Option<&mut Self> {
        let Some((key, path)) = path.split_last() else {
            return Some(self);
        };

        // Navigate to the parent table, converting any UndefinedTables to DottedKeyTables
        let parent = path.iter().try_fold(self, |table, key| {
            match table.get(key) {
                None => {
                    table.insert(key.clone(), Value::DottedKeyTable(Self::new()));
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
        })?;

        // Find the table in the parent, or create a new one if it doesn't exist. Unlike the parent
        // tables, we make this a Table instead of a UndefinedTable
        match parent.get(key) {
            None => {
                parent.insert(key.clone(), Value::DottedKeyTable(Self::new()));
            }
            Some(&Value::UndefinedTable(_)) => {
                // Need to remove the entry to take ownership of the subtable
                let Some(Value::UndefinedTable(subtable)) = parent.remove(key) else {
                    unreachable!("we just checked this key")
                };
                parent.insert(key.clone(), Value::DottedKeyTable(subtable));
            }
            Some(&Value::DottedKeyTable(_)) => {} // Already exists
            Some(_) => return None,
        }
        let Some(&mut Value::DottedKeyTable(ref mut subtable)) = parent.get_mut(key) else {
            unreachable!("we just inserted a table")
        };
        Some(subtable)
    }

    fn get_inline_subtable<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self> {
        // Navigate to the subtable with the given name for each element in the path
        path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| Value::DottedKeyTable(Self::new()));
            match *entry {
                Value::DottedKeyTable(ref mut subtable) => Some(subtable),
                _ => None,
            }
        })
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_whitespace(char: &u8) -> bool {
    matches!(*char, b'\t' | b' ')
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_whitespace_or_newline(char: &u8) -> bool {
    matches!(*char, b'\t' | b' ' | b'\r' | b'\n')
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_word(char: &u8) -> bool {
    matches!(*char, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_comment(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09)
    matches!(*char, 0x09 | 0x20..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_basic_str_sans_escapes(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), the delimiter '"' (0x22), and escape
    // char '\' (0x5c)
    matches!(*char, 0x09 | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_multiline_basic_str_sans_escapes(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), '\n' (0x0a), the delimiter '"' (0x22),
    // and escape char '\' (0x5c)
    matches!(*char, 0x09 | 0x0a | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_literal_str(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), and the delimiter '\'' (0x27)
    matches!(*char, 0x09 | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_multiline_literal_str(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), '\n' (0x0a), and the delimiter '\''
    // (0x27)
    matches!(*char, 0x09 | 0x0a | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
}

#[allow(clippy::trivially_copy_pass_by_ref)] // this makes it more ergonomic to use these
const fn is_toml_legal(char: &u8) -> bool {
    // Disallow ASCII control chars except tab (0x09), carriage return (0x0d) and newline (0x0a)
    matches!(*char, 0x09 | 0x0a | 0x0d | 0x20..=0x7e | 0x80..)
}
