use core::{fmt, str};
use std::marker::PhantomData;
use std::{borrow::Cow, collections::HashMap};

use lexical::{FromLexicalWithOptions, NumberFormatBuilder, ParseIntegerOptions};
use serde::de;

use super::error::{Error, Result};
use super::{Reader, StrReader};

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
pub(super) enum RawValue<'de> {
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
    Array(Vec<RawValue<'de>>),
    // Table defined by a table header. This is immutable aside from being able to add subtables
    Table(HashMap<Cow<'de, str>, RawValue<'de>>),
    // Super table created when parsing a subtable header. This can still be explicitly defined
    // later turning it into a `Table`
    UndefinedTable(HashMap<Cow<'de, str>, RawValue<'de>>),
    // A table defined by dotted keys. This can be freely added to by other dotted keys
    DottedKeyTable(HashMap<Cow<'de, str>, RawValue<'de>>),
    // Inline table
    InlineTable(HashMap<Cow<'de, str>, RawValue<'de>>),
    // Array of tables
    ArrayOfTables(Vec<HashMap<Cow<'de, str>, RawValue<'de>>>),
}

impl RawValue<'_> {
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

impl<'de> Parser<'de, StrReader<'de>> {
    #[allow(clippy::should_implement_trait)]
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn from_str(str: &'de str) -> Self {
        Self::from_reader(StrReader::from_str(str))
    }

    #[allow(clippy::should_implement_trait)]
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        Self::from_reader(StrReader::from_slice(bytes))
    }
}

impl<'de, R> Parser<'de, R>
where
    R: Reader<'de>,
{
    #[allow(clippy::missing_const_for_fn)] // TODO decide on constness of public API
    #[must_use]
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader,
            _phantom: PhantomData,
        }
    }
}

impl<'de, R> Parser<'de, R>
where
    R: Reader<'de>,
{
    pub fn parse(&mut self) -> Result<RawValue<'de>> {
        let mut root = HashMap::new();

        // The currently opened table
        let mut table = &mut root;
        // The path to the currently opened table (used for error messages)
        let mut table_path = Vec::new();

        loop {
            self.skip_comments_and_whitespace()?;

            // Parse array header
            if self.reader.eat_str(b"[[")? {
                let position = self.reader.position();
                let key = self.parse_array_header()?;
                table = root
                    .get_array_header(&key)
                    .ok_or_else(|| Error::invalid_table_header(&key, position))?;
                table_path = key;
            }
            // Parse table header
            else if self.reader.eat_char(b'[')? {
                let position = self.reader.position();
                let key = self.parse_table_header()?;
                table = root
                    .get_table_header(&key)
                    .ok_or_else(|| Error::invalid_table_header(&key, position))?;
                table_path = key;
            }
            // Parse key/value pair
            else if self
                .reader
                .peek()?
                .is_some_and(|ch| TomlChar::is_word(&ch) || ch == b'"' || ch == b'\'')
            {
                let (full_key, value) = self.parse_key_value_pair()?;
                let (key, path) = full_key
                    .split_last()
                    .unwrap_or_else(|| unreachable!("path cannot be empty"));

                // Navigate to the subtable
                let subtable = table.get_dotted_key(path).ok_or_else(|| {
                    Error::invalid_key_path(&full_key, &table_path, self.reader.position())
                })?;

                // Check if the key is already present
                if subtable.contains_key(key) {
                    return Err(Error::duplicate_key(
                        &full_key,
                        &table_path,
                        self.reader.position(),
                    ));
                }
                subtable.insert(key.clone(), value);
            }
            // Anything else is unexpected
            else if let Some(ch) = self.reader.next()? {
                return Err(Error::unexpected_char(
                    char::from(ch),
                    self.reader.position() - 1,
                ));
            }

            // Expect newline/comment after a key/value pair or table/array header
            self.skip_whitespace()?;
            match self.reader.peek()? {
                Some(b'\n') => self.reader.discard()?,
                Some(b'\r') if self.reader.peek_at(1)?.is_some_and(|ch| ch == b'\n') => {
                    self.reader.discard()?; // '\r'
                    self.reader.discard()?; // '\n'
                }
                Some(b'#') => {
                    self.skip_comment()?;
                }
                Some(ch) => {
                    return Err(Error::unexpected_char(
                        char::from(ch),
                        self.reader.position(),
                    ))
                }
                None => break,
            }
        }

        Ok(RawValue::Table(root))
    }

    fn parse_array_header(&mut self) -> Result<Vec<Cow<'de, str>>> {
        self.skip_whitespace()?;
        let key = self.parse_dotted_key()?;

        self.skip_whitespace()?;
        self.reader
            .eat_str(b"]]")?
            .then_some(key)
            .ok_or_else(|| Error::expected("]] after dotted key", self.reader.position()))
    }

    fn parse_table_header(&mut self) -> Result<Vec<Cow<'de, str>>> {
        self.skip_whitespace()?;
        let key = self.parse_dotted_key()?;

        self.skip_whitespace()?;
        self.reader
            .eat_char(b']')?
            .then_some(key)
            .ok_or_else(|| Error::expected("] after dotted key", self.reader.position()))
    }

    fn parse_key_value_pair(&mut self) -> Result<(Vec<Cow<'de, str>>, RawValue<'de>)> {
        let path = self.parse_dotted_key()?;

        // Whitespace should already have been consumed by parse_dotted_key looking for another '.'
        if !self.reader.eat_char(b'=')? {
            return Err(Error::expected("= after key", self.reader.position()));
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
        let key = self.reader.next_str_while(TomlChar::is_word)?;

        (!key.is_empty())
            .then_some(key)
            .ok_or_else(|| Error::expected("key", self.reader.position()))
    }

    fn parse_value(&mut self) -> Result<RawValue<'de>> {
        match self.reader.peek()? {
            // String
            Some(b'"' | b'\'') => self.parse_string().map(RawValue::String),
            // Boolean
            Some(b't' | b'f') => self.parse_bool().map(RawValue::Boolean),
            // Leading 0 => either prefixed int, date/time, just 0, or invalid
            Some(b'0') => {
                // Floats with leading 0 before decimal/exponent
                if matches!(self.reader.peek_at(1)?, Some(b'.' | b'e' | b'E')) {
                    self.parse_number_decimal()
                }
                // 0x, 0o, 0b, etc
                else if matches!(self.reader.peek_at(1)?, Some(ch) if ch.is_ascii_alphabetic()) {
                    self.parse_number_radix()
                }
                // Date/time or invalid number (leading 0 is not allowed)
                else if matches!(self.reader.peek_at(1)?, Some(ch) if ch.is_ascii_digit()) {
                    // We only know whether we're parsing a datetime or a number when we see a
                    // '-' after 4 digits or a ':' after 2, so we need to look ahead here
                    let n_digits = self.reader.peek_while(u8::is_ascii_digit)?.len();
                    if (n_digits == 4 && self.reader.peek_at(4)?.is_some_and(|ch| ch == b'-'))
                        || (n_digits == 2 && self.reader.peek_at(2)?.is_some_and(|ch| ch == b':'))
                    {
                        self.parse_datetime()
                    } else {
                        Err(Error::invalid_number(
                            "leading zero",
                            self.reader.position(),
                        ))
                    }
                }
                // Parse just the 0
                else {
                    self.parse_number_decimal()
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
                    Some(b'i' | b'n') => self.parse_number_special().map(RawValue::SpecialFloat),
                    // Invalid
                    _ => Err(Error::invalid_number(
                        "missing digits",
                        self.reader.position(),
                    )),
                }
            }
            // Special float (inf or nan)
            Some(b'i' | b'n') => self.parse_number_special().map(RawValue::SpecialFloat),
            // Array
            Some(b'[') => {
                self.reader.discard()?; // We consume the opening delimiter
                self.parse_array().map(RawValue::Array)
            }
            // Table
            Some(b'{') => {
                self.reader.discard()?; // We consume the opening delimiter
                self.parse_inline_table().map(RawValue::InlineTable)
            }
            Some(char) => Err(Error::unexpected_char(
                char::from(char),
                self.reader.position(),
            )),
            None => Err(Error::eof(self.reader.position())),
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
            Err(Error::expected("string", self.reader.position()))
        }
    }

    fn parse_basic_str(&mut self) -> Result<Cow<'de, str>> {
        let mut str = self
            .reader
            .next_str_while(TomlChar::is_basic_str_sans_escapes)?;

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
                    break Err(Error::unterminated_string(self.reader.position()));
                }
                Some(char) => {
                    break Err(Error::illegal_char(
                        char::from(char),
                        self.reader.position(),
                    ));
                }
            }

            str.to_mut().push_str(
                &self
                    .reader
                    .next_str_while(TomlChar::is_basic_str_sans_escapes)?,
            );
        }
    }

    fn parse_multiline_basic_str(&mut self) -> Result<Cow<'de, str>> {
        // Newlines after the first """ are ignored
        let _ = self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?;

        let mut str = self
            .reader
            .next_str_while(TomlChar::is_multiline_basic_str_sans_escapes)?;

        loop {
            match self.reader.next()? {
                Some(b'\\') => {
                    // Trailing '\' means eat all whitespace and newlines
                    if self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")? {
                        self.reader.next_while(TomlChar::is_whitespace_or_newline)?;
                    }
                    // If there's a space after the '\' we assume a trailing '\' with trailing
                    // whitespace, but we need to verify that's the case by checking for a newline
                    else if let Some(char) = self.reader.next_if(TomlChar::is_whitespace)? {
                        let position = self.reader.position() - 1;
                        self.reader.next_while(TomlChar::is_whitespace)?;
                        if !(self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?) {
                            return Err(Error::invalid_escape(
                                format!("{:?}", char::from(char)),
                                position,
                            ));
                        }
                        self.reader.next_while(TomlChar::is_whitespace_or_newline)?;
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
                    break Err(Error::unterminated_string(self.reader.position()));
                }
                Some(b'\r') if matches!(self.reader.peek()?, Some(b'\n')) => {
                    // Ignore '\r' followed by '\n', else it's handled by the illegal char branch
                    continue;
                }
                Some(char) => {
                    break Err(Error::illegal_char(
                        char::from(char),
                        self.reader.position(),
                    ))
                }
            }

            str.to_mut().push_str(
                &self
                    .reader
                    .next_str_while(TomlChar::is_multiline_basic_str_sans_escapes)?,
            );
        }
    }

    fn parse_literal_str(&mut self) -> Result<Cow<'de, str>> {
        let str = self.reader.next_str_while(TomlChar::is_literal_str)?;

        match self.reader.next()? {
            Some(b'\'') => Ok(str),
            None | Some(b'\r' | b'\n') => Err(Error::unterminated_string(self.reader.position())),
            Some(char) => Err(Error::illegal_char(
                char::from(char),
                self.reader.position(),
            )),
        }
    }

    fn parse_multiline_literal_str(&mut self) -> Result<Cow<'de, str>> {
        // Newlines after the first ''' are ignored
        let _ = self.reader.eat_char(b'\n')? || self.reader.eat_str(b"\r\n")?;

        let mut str = self
            .reader
            .next_str_while(TomlChar::is_multiline_literal_str)?;

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
                    break Err(Error::unterminated_string(self.reader.position()));
                }
                Some(b'\r') if matches!(self.reader.peek()?, Some(b'\n')) => {
                    // Ignore '\r' followed by '\n', else it's handled by the illegal char branch
                    continue;
                }
                Some(char) => {
                    break Err(Error::illegal_char(
                        char::from(char),
                        self.reader.position(),
                    ))
                }
            }

            str.to_mut().push_str(
                &self
                    .reader
                    .next_str_while(TomlChar::is_multiline_literal_str)?,
            );
        }
    }

    fn parse_escape_seq(&mut self) -> Result<char> {
        const HEX_ESCAPE_FORMAT: u128 = NumberFormatBuilder::new()
            .mantissa_radix(16)
            .no_positive_mantissa_sign(true)
            .build();

        let position = self.reader.position();

        let Some(char) = self.reader.next()? else {
            return Err(Error::unterminated_string(position));
        };

        match char {
            b'b' => Ok('\x08'),
            b't' => Ok('\t'),
            b'n' => Ok('\n'),
            b'f' => Ok('\x0c'),
            b'r' => Ok('\r'),
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'u' => {
                let bytes = self
                    .reader
                    .next_array::<4>()?
                    .ok_or_else(|| Error::unterminated_string(position))?;
                let options = ParseIntegerOptions::default();
                u32::from_lexical_with_options::<HEX_ESCAPE_FORMAT>(bytes.as_ref(), &options)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| {
                        Error::invalid_escape(
                            format!("u{}", String::from_utf8_lossy(bytes.as_ref())),
                            position,
                        )
                    })
            }
            b'U' => {
                let bytes = self
                    .reader
                    .next_array::<8>()?
                    .ok_or_else(|| Error::unterminated_string(position))?;
                let options = ParseIntegerOptions::default();
                u32::from_lexical_with_options::<HEX_ESCAPE_FORMAT>(bytes.as_ref(), &options)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| {
                        Error::invalid_escape(
                            format!("U{}", String::from_utf8_lossy(bytes.as_ref())),
                            position,
                        )
                    })
            }
            _ => Err(Error::invalid_escape(
                format!("{:?}", char::from(char)),
                position,
            )),
        }
    }

    fn parse_bool(&mut self) -> Result<bool> {
        // Match against the whole word, don't just parse the first n characters so we don't
        // successfully parse e.g. true92864yhowkalgp98y
        let word = self.reader.next_while(TomlChar::is_word)?;
        let result = match word.as_ref() {
            b"true" => Ok(true),
            b"false" => Ok(false),
            _ => Err(Error::expected(
                "true/false",
                self.reader.position() - word.len(),
            )),
        };
        result
    }

    fn parse_datetime(&mut self) -> Result<RawValue<'de>> {
        let position = self.reader.position();
        self.reader.start_seq(); // Start sequence for datetime

        // Use the number of digits to determine whether we have a date or time
        let first_num = self.reader.peek_while(u8::is_ascii_digit)?;

        // 4 digits = year for date/datetime
        if first_num.len() == 4 {
            self.check_date(position)?;

            // Check if we have a time or just a date
            let have_time = match self.reader.peek()? {
                Some(b'T' | b't') => true,
                Some(b' ') if matches!(self.reader.peek_at(1)?, Some(b'0'..=b'9')) => true,
                _ => false,
            };
            if !have_time {
                // Check we're at the end of the value (i.e. no trailing invalid chars)
                if self.reader.peek()?.as_ref().is_some_and(TomlChar::is_word) {
                    return Err(Error::invalid_datetime(position));
                }

                return self.reader.end_seq().map(RawValue::LocalDate);
            };
            self.reader.discard()?; // Skip the 'T'/space

            self.check_time(position)?;

            // Check for the offset
            let have_offset = matches!(self.reader.peek()?, Some(b'Z' | b'z' | b'+' | b'-'));
            if !have_offset {
                // Check we're at the end of the value (i.e. no trailing invalid chars)
                if self.reader.peek()?.as_ref().is_some_and(TomlChar::is_word) {
                    return Err(Error::invalid_datetime(position));
                }

                return self.reader.end_seq().map(RawValue::LocalDatetime);
            };

            self.check_offset(position)?;

            // Check we're at the end of the value (i.e. no trailing invalid chars)
            if self.reader.peek()?.as_ref().is_some_and(TomlChar::is_word) {
                return Err(Error::invalid_datetime(position));
            }

            self.reader.end_seq().map(RawValue::OffsetDatetime)
        }
        // 2 digits = hour for time
        else if first_num.len() == 2 {
            self.check_time(position)?;

            // Check for bogus offset (offset time is not a thing in TOML)
            // Check we're at the end of the value (i.e. no trailing invalid chars)
            if matches!(self.reader.peek()?, Some(b'Z' | b'z' | b'+' | b'-'))
                || self.reader.peek()?.as_ref().is_some_and(TomlChar::is_word)
            {
                return Err(Error::invalid_datetime(position));
            }

            self.reader.end_seq().map(RawValue::LocalTime)
        }
        // Any other number of digits is invalid
        else {
            Err(Error::expected("datetime", self.reader.position()))
        }
    }

    fn check_date(&mut self, position: usize) -> Result<()> {
        if self
            .reader
            .next_array::<4>()?
            .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
            && self.reader.eat_char(b'-')?
            && self
                .reader
                .next_array::<2>()?
                .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
            && self.reader.eat_char(b'-')?
            && self
                .reader
                .next_array::<2>()?
                .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
        {
            Ok(())
        } else {
            Err(Error::invalid_datetime(position))
        }
    }

    fn check_time(&mut self, position: usize) -> Result<()> {
        if !(self
            .reader
            .next_array::<2>()?
            .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
            && self.reader.eat_char(b':')?
            && self
                .reader
                .next_array::<2>()?
                .is_some_and(|a| a.iter().all(u8::is_ascii_digit)))
        {
            return Err(Error::invalid_datetime(position));
        }

        if !self.reader.eat_char(b':')? {
            return Ok(()); // Seconds are optional, so just return hh:mm
        }
        if !self
            .reader
            .next_array::<2>()?
            .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
        {
            return Err(Error::invalid_datetime(position));
        }

        if !self.reader.eat_char(b'.')? {
            return Ok(()); // Fractional seconds are also optional, so just return hh:mm:ss
        }
        if self.reader.next_while(u8::is_ascii_digit)?.is_empty() {
            return Err(Error::invalid_datetime(position));
        }

        Ok(())
    }

    fn check_offset(&mut self, position: usize) -> Result<()> {
        if self.reader.eat_char(b'Z')?
            || self.reader.eat_char(b'z')?
            || ((self.reader.eat_char(b'+')? || self.reader.eat_char(b'-')?)
                && self
                    .reader
                    .next_array::<2>()?
                    .is_some_and(|a| a.iter().all(u8::is_ascii_digit))
                && self.reader.eat_char(b':')?
                && self
                    .reader
                    .next_array::<2>()?
                    .is_some_and(|a| a.iter().all(u8::is_ascii_digit)))
        {
            Ok(())
        } else {
            Err(Error::invalid_datetime(position))
        }
    }

    fn parse_number_radix(&mut self) -> Result<RawValue<'de>> {
        let position = self.reader.position();
        if self.reader.eat_str(b"0x")? {
            Ok(RawValue::HexInt(
                self.reader
                    .next_while(TomlChar::is_word)
                    .and_then(|bytes| {
                        bytes
                            .iter()
                            .all(|byte| byte.is_ascii_hexdigit() || *byte == b'_')
                            .then_some(bytes)
                            .ok_or_else(|| {
                                Error::invalid_number("invalid hexadecimal digit", position)
                            })
                    })?,
            ))
        } else if self.reader.eat_str(b"0o")? {
            Ok(RawValue::OctalInt(
                self.reader
                    .next_while(TomlChar::is_word)
                    .and_then(|bytes| {
                        bytes
                            .iter()
                            .all(|&b| matches!(b, b'0'..=b'7' | b'_'))
                            .then_some(bytes)
                            .ok_or_else(|| Error::invalid_number("invalid octal digit", position))
                    })?,
            ))
        } else if self.reader.eat_str(b"0b")? {
            Ok(RawValue::BinaryInt(
                self.reader
                    .next_while(TomlChar::is_word)
                    .and_then(|bytes| {
                        bytes
                            .iter()
                            .all(|&b| matches!(b, b'0' | b'1' | b'_'))
                            .then_some(bytes)
                            .ok_or_else(|| Error::invalid_number("invalid binary digit", position))
                    })?,
            ))
        } else {
            Err(Error::expected("number with radix", position))
        }
    }

    fn parse_number_special(&mut self) -> Result<SpecialFloat> {
        let position = self.reader.position();
        // In each case we match against the whole word, don't just parse the first n characters so
        // we don't successfully parse e.g. inf92864yhowkalgp98y
        match self.reader.peek()? {
            Some(b'+') => {
                self.reader.discard()?;
                match self.reader.next_while(TomlChar::is_word)?.as_ref() {
                    b"inf" => Ok(SpecialFloat::Infinity),
                    b"nan" => Ok(SpecialFloat::Nan),
                    _ => Err(Error::expected("inf/nan", position)),
                }
            }
            Some(b'-') => {
                self.reader.discard()?;
                match self.reader.next_while(TomlChar::is_word)?.as_ref() {
                    b"inf" => Ok(SpecialFloat::NegInfinity),
                    b"nan" => Ok(SpecialFloat::NegNan),
                    _ => Err(Error::expected("inf/nan", position)),
                }
            }
            _ => match self.reader.next_while(TomlChar::is_word)?.as_ref() {
                b"inf" => Ok(SpecialFloat::Infinity),
                b"nan" => Ok(SpecialFloat::Nan),
                _ => Err(Error::expected("inf/nan", position)),
            },
        }
    }

    fn parse_number_decimal(&mut self) -> Result<RawValue<'de>> {
        let position = self.reader.position();
        let mut float = false;

        self.reader.start_seq(); // Start sequence for number parsing

        let _ = self.reader.eat_char(b'+')? || self.reader.eat_char(b'-')?; // Optional sign

        // We need at least one digit
        // Note: leading 0s are not allowed, but that is checked in parse_value
        if self.reader.next_if(u8::is_ascii_digit)?.is_none() {
            return Err(match self.reader.peek()? {
                Some(b'_') => Error::invalid_number("leading underscore", position),
                _ => Error::invalid_number("missing digits", position),
            });
        }

        // Remainder of integer digits
        loop {
            self.reader.next_while(u8::is_ascii_digit)?;

            if self.reader.eat_char(b'_')? {
                // Need at least one digit after an '_'
                match self.reader.peek()? {
                    Some(b'0'..=b'9') => Ok(()),
                    Some(b'_') => Err(Error::invalid_number("double underscore", position)),
                    _ => Err(Error::invalid_number("trailing underscore", position)),
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
                Some(b'_') => Err(Error::invalid_number("leading underscore", position)),
                _ => Err(Error::invalid_number("missing digits", position)),
            }?;
            self.reader.next_while(u8::is_ascii_digit)?;

            // Remainder of integer digits
            loop {
                self.reader.next_while(u8::is_ascii_digit)?;

                if self.reader.eat_char(b'_')? {
                    // Need at least one digit after an '_'
                    match self.reader.peek()? {
                        Some(b'0'..=b'9') => Ok(()),
                        Some(b'_') => Err(Error::invalid_number("double underscore", position)),
                        _ => Err(Error::invalid_number("trailing underscore", position)),
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
                Some(b'_') => Err(Error::invalid_number("leading underscore", position)),
                _ => Err(Error::invalid_number("missing digits", position)),
            }?;
            self.reader.next_while(u8::is_ascii_digit)?;

            // Remainder of integer digits
            loop {
                self.reader.next_while(u8::is_ascii_digit)?;

                if self.reader.eat_char(b'_')? {
                    // Need at least one digit after an '_'
                    match self.reader.peek()? {
                        Some(b'0'..=b'9') => Ok(()),
                        Some(b'_') => Err(Error::invalid_number("double underscore", position)),
                        _ => Err(Error::invalid_number("trailing underscore", position)),
                    }?;
                } else {
                    break;
                }
            }
        }

        let number = self.reader.end_seq()?; // End sequence for number parsing

        Ok(if float {
            RawValue::Float(number)
        } else {
            RawValue::Integer(number)
        })
    }

    fn parse_array(&mut self) -> Result<Vec<RawValue<'de>>> {
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
                return Err(Error::expected(
                    ", or ] after value in array",
                    self.reader.position(),
                ));
            }
        }

        Ok(result)
    }

    fn parse_inline_table(&mut self) -> Result<HashMap<Cow<'de, str>, RawValue<'de>>> {
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
                Error::invalid_key_path(&full_key, &["inline table"], self.reader.position())
            })?;

            // Check if the key is already present
            if subtable.contains_key(key) {
                return Err(Error::duplicate_key(
                    &full_key,
                    &["inline table"],
                    self.reader.position(),
                ));
            }
            subtable.insert(key.clone(), value);

            self.skip_whitespace()?;

            if self.reader.eat_char(b'}')? {
                break; // End of array
            } else if !self.reader.eat_char(b',')? {
                return Err(Error::expected(
                    ", or } after key/value pair in inline table",
                    self.reader.position(),
                ));
            }

            self.skip_whitespace()?;
        }

        Ok(result)
    }

    fn skip_whitespace(&mut self) -> Result<()> {
        while self.reader.next_if(TomlChar::is_whitespace)?.is_some() {}
        Ok(())
    }

    fn skip_comment(&mut self) -> Result<bool> {
        if self.reader.eat_char(b'#')? {
            // Skip validating comments with feature = "fast"
            if cfg!(feature = "fast") {
                self.reader.next_while(|&ch| ch != b'\n')?;
            } else {
                let position = self.reader.position();
                // next_str_while will validate UTF-8
                let comment = self.reader.next_str_while(|&ch| ch != b'\n')?;
                // Trim trailing \r (since \r\n is valid)
                let comment = comment.strip_suffix('\r').unwrap_or(&comment);
                // Check for any invalid characters in the comment
                if let Some((i, ch)) = comment.bytes().enumerate().find(|&(_, c)| !c.is_comment()) {
                    return Err(Error::illegal_char(char::from(ch), position + i));
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
    fn get_table_header<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self>;
    fn get_array_header<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self>;
    fn get_dotted_key<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self>;
    fn get_inline_subtable<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self>;
}

impl<'a> TomlTable<'a> for HashMap<Cow<'a, str>, RawValue<'a>> {
    fn get_table_header<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self> {
        let Some((key, path)) = path.split_last() else {
            return Some(self);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| RawValue::UndefinedTable(Self::new()));
            match *entry {
                RawValue::Table(ref mut subtable)
                | RawValue::UndefinedTable(ref mut subtable)
                | RawValue::DottedKeyTable(ref mut subtable) => Some(subtable),
                RawValue::ArrayOfTables(ref mut array) => {
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
                parent.insert(key.clone(), RawValue::Table(Self::new()));
            }
            Some(&RawValue::UndefinedTable(_)) => {
                // Need to remove the entry to take ownership of the subtable
                let Some(RawValue::UndefinedTable(subtable)) = parent.remove(key) else {
                    unreachable!("we just checked this key")
                };
                parent.insert(key.clone(), RawValue::Table(subtable));
            }
            Some(_) => return None,
        };
        let Some(&mut RawValue::Table(ref mut subtable)) = parent.get_mut(key) else {
            unreachable!("we just inserted a Table")
        };
        Some(subtable)
    }

    fn get_array_header<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self> {
        let Some((key, path)) = path.split_last() else {
            return Some(self);
        };

        // Navigate to the parent table, either a subtable with the given name or the last element
        // in an array of tables
        let parent = path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| RawValue::UndefinedTable(Self::new()));
            match *entry {
                RawValue::Table(ref mut subtable)
                | RawValue::UndefinedTable(ref mut subtable)
                | RawValue::DottedKeyTable(ref mut subtable) => Some(subtable),
                RawValue::ArrayOfTables(ref mut array) => {
                    Some(array.last_mut().unwrap_or_else(|| {
                        unreachable!("we never insert an empty array of tables")
                    }))
                }
                _ => None,
            }
        })?;

        // Then find the array of tables in the parent, or create a new one if it doesn't exist
        if let RawValue::ArrayOfTables(ref mut subarray) = *parent
            .entry(key.clone())
            .or_insert_with(|| RawValue::ArrayOfTables(Vec::new()))
        {
            subarray.push(Self::new());
            subarray.last_mut()
        } else {
            None
        }
    }

    fn get_dotted_key<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self> {
        let Some((key, path)) = path.split_last() else {
            return Some(self);
        };

        // Navigate to the parent table, converting any UndefinedTables to DottedKeyTables
        let parent = path.iter().try_fold(self, |table, key| {
            match table.get(key) {
                None => {
                    table.insert(key.clone(), RawValue::DottedKeyTable(Self::new()));
                }
                Some(&RawValue::UndefinedTable(_)) => {
                    // Need to remove the entry to take ownership of the subtable
                    let Some(RawValue::UndefinedTable(subtable)) = table.remove(key) else {
                        unreachable!("we just checked this key")
                    };
                    table.insert(key.clone(), RawValue::DottedKeyTable(subtable));
                }
                Some(&RawValue::DottedKeyTable(_)) => {} // Already exists
                Some(_) => return None,
            };
            let Some(&mut RawValue::DottedKeyTable(ref mut subtable)) = table.get_mut(key) else {
                unreachable!("we just inserted a DottedKeyTable")
            };
            Some(subtable)
        })?;

        // Find the table in the parent, or create a new one if it doesn't exist. Unlike the parent
        // tables, we make this a Table instead of a UndefinedTable
        match parent.get(key) {
            None => {
                parent.insert(key.clone(), RawValue::DottedKeyTable(Self::new()));
            }
            Some(&RawValue::UndefinedTable(_)) => {
                // Need to remove the entry to take ownership of the subtable
                let Some(RawValue::UndefinedTable(subtable)) = parent.remove(key) else {
                    unreachable!("we just checked this key")
                };
                parent.insert(key.clone(), RawValue::DottedKeyTable(subtable));
            }
            Some(&RawValue::DottedKeyTable(_)) => {} // Already exists
            Some(_) => return None,
        };
        let Some(&mut RawValue::DottedKeyTable(ref mut subtable)) = parent.get_mut(key) else {
            unreachable!("we just inserted a table")
        };
        Some(subtable)
    }

    fn get_inline_subtable<'b>(&'b mut self, path: &[Cow<'a, str>]) -> Option<&'b mut Self> {
        // Navigate to the subtable with the given name for each element in the path
        path.iter().try_fold(self, |table, key| {
            let entry = table
                .entry(key.clone())
                .or_insert_with(|| RawValue::DottedKeyTable(Self::new()));
            match *entry {
                RawValue::DottedKeyTable(ref mut subtable) => Some(subtable),
                _ => None,
            }
        })
    }
}

trait TomlChar {
    fn is_whitespace(&self) -> bool;
    fn is_whitespace_or_newline(&self) -> bool;
    fn is_word(&self) -> bool;
    fn is_comment(&self) -> bool;
    fn is_basic_str_sans_escapes(&self) -> bool;
    fn is_multiline_basic_str_sans_escapes(&self) -> bool;
    fn is_literal_str(&self) -> bool;
    fn is_multiline_literal_str(&self) -> bool;
}

impl TomlChar for u8 {
    fn is_whitespace(&self) -> bool {
        matches!(*self, b'\t' | b' ')
    }

    fn is_whitespace_or_newline(&self) -> bool {
        matches!(*self, b'\t' | b' ' | b'\r' | b'\n')
    }

    fn is_word(&self) -> bool {
        matches!(*self, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
    }

    fn is_comment(&self) -> bool {
        // Disallow ASCII control chars except tab (0x09)
        matches!(*self, 0x09 | 0x20..=0x7e | 0x80..)
    }

    fn is_basic_str_sans_escapes(&self) -> bool {
        // Disallow ASCII control chars except tab (0x09), the delimiter '"' (0x22), and escape
        // char '\' (0x5c)
        matches!(*self, 0x09 | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
    }

    fn is_multiline_basic_str_sans_escapes(&self) -> bool {
        // Disallow ASCII control chars except tab (0x09), '\n' (0x0a), the delimiter '"' (0x22),
        // and escape char '\' (0x5c)
        matches!(*self, 0x09 | 0x0a | 0x20 | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80..)
    }

    fn is_literal_str(&self) -> bool {
        // Disallow ASCII control chars except tab (0x09), and the delimiter '\'' (0x27)
        matches!(*self, 0x09 | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
    }

    fn is_multiline_literal_str(&self) -> bool {
        // Disallow ASCII control chars except tab (0x09), '\n' (0x0a), and the delimiter '\''
        // (0x27)
        matches!(*self, 0x09 | 0x0a | 0x20..=0x26 | 0x28..=0x7e | 0x80..)
    }
}

// #[cfg(test)]
// mod tests {
//     use maplit::hashmap;

//     use super::*;

//     #[test]
//     #[allow(clippy::too_many_lines)]
//     fn test_datetime() {
//         let value = Parser::from_str(
//             r#"
// space = 1987-07-05 17:45:00Z

// # ABNF is case-insensitive, both "Z" and "z" must be supported.
// lower = 1987-07-05t17:45:00z
// "#,
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "space".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "lower".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// first-offset = 0001-01-01 00:00:00Z
// first-local  = 0001-01-01 00:00:00
// first-date   = 0001-01-01

// last-offset = 9999-12-31 23:59:59Z
// last-local  = 9999-12-31 23:59:59
// last-date   = 9999-12-31
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "first-offset".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1,
//                         month: 1,
//                         day: 1,
//                     }),
//                     time: Some(Time {
//                         hour: 0,
//                         minute: 0,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "first-local".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1,
//                         month: 1,
//                         day: 1,
//                     }),
//                     time: Some(Time {
//                         hour: 0,
//                         minute: 0,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "first-date".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1,
//                         month: 1,
//                         day: 1,
//                     }),
//                     time: None,
//                     offset: None,
//                 }),
//                 "last-offset".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 9999,
//                         month: 12,
//                         day: 31,
//                     }),
//                     time: Some(Time {
//                         hour: 23,
//                         minute: 59,
//                         second: 59,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "last-local".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 9999,
//                         month: 12,
//                         day: 31,
//                     }),
//                     time: Some(Time {
//                         hour: 23,
//                         minute: 59,
//                         second: 59,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "last-date".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 9999,
//                         month: 12,
//                         day: 31,
//                     }),
//                     time: None,
//                     offset: None,
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// 2000-datetime       = 2000-02-29 15:15:15Z
// 2000-datetime-local = 2000-02-29 15:15:15
// 2000-date           = 2000-02-29

// 2024-datetime       = 2024-02-29 15:15:15Z
// 2024-datetime-local = 2024-02-29 15:15:15
// 2024-date           = 2024-02-29
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "2000-datetime".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 2000,
//                         month: 2,
//                         day: 29,
//                     }),
//                     time: Some(Time {
//                         hour: 15,
//                         minute: 15,
//                         second: 15,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "2000-datetime-local".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 2000,
//                         month: 2,
//                         day: 29,
//                     }),
//                     time: Some(Time {
//                         hour: 15,
//                         minute: 15,
//                         second: 15,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "2000-date".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 2000,
//                         month: 2,
//                         day: 29,
//                     }),
//                     time: None,
//                     offset: None,
//                 }),
//                 "2024-datetime".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 2024,
//                         month: 2,
//                         day: 29,
//                     }),
//                     time: Some(Time {
//                         hour: 15,
//                         minute: 15,
//                         second: 15,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "2024-datetime-local".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 2024,
//                         month: 2,
//                         day: 29,
//                     }),
//                     time: Some(Time {
//                         hour: 15,
//                         minute: 15,
//                         second: 15,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "2024-date".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 2024,
//                         month: 2,
//                         day: 29,
//                     }),
//                     time: None,
//                     offset: None,
//                 }),
//             })
//         );

//         let value = Parser::from_str("bestdayever = 1987-07-05")
//             .parse()
//             .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "bestdayever".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: None,
//                     offset: None,
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// besttimeever = 17:45:00
// milliseconds = 10:32:00.555
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "besttimeever".into() => RawValue::Datetime(Datetime {
//                     date: None,
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "milliseconds".into() => RawValue::Datetime(Datetime {
//                     date: None,
//                     time: Some(Time {
//                         hour: 10,
//                         minute: 32,
//                         second: 0,
//                         nanosecond: 555_000_000,
//                     }),
//                     offset: None,
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// local = 1987-07-05T17:45:00
// milli = 1977-12-21T10:32:00.555
// space = 1987-07-05 17:45:00
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "local".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "milli".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1977,
//                         month: 12,
//                         day: 21,
//                     }),
//                     time: Some(Time {
//                         hour: 10,
//                         minute: 32,
//                         second: 0,
//                         nanosecond: 555_000_000,
//                     }),
//                     offset: None,
//                 }),
//                 "space".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// utc1  = 1987-07-05T17:45:56.123Z
// utc2  = 1987-07-05T17:45:56.6Z
// wita1 = 1987-07-05T17:45:56.123+08:00
// wita2 = 1987-07-05T17:45:56.6+08:00
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "utc1".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 123_000_000,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "utc2".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 600_000_000,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "wita1".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 123_000_000,
//                     }),
//                     offset: Some(Offset::Custom {
//                         hours: 8,
//                         minutes: 0,
//                     }),
//                 }),
//                 "wita2".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 600_000_000,
//                     }),
//                     offset: Some(Offset::Custom {
//                         hours: 8,
//                         minutes: 0,
//                     }),
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// # Seconds are optional in date-time and time.
// without-seconds-1 = 13:37
// without-seconds-2 = 1979-05-27 07:32Z
// without-seconds-3 = 1979-05-27 07:32-07:00
// without-seconds-4 = 1979-05-27T07:32
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "without-seconds-1".into() => RawValue::Datetime(Datetime {
//                     date: None,
//                     time: Some(Time {
//                         hour: 13,
//                         minute: 37,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//                 "without-seconds-2".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1979,
//                         month: 5,
//                         day: 27,
//                     }),
//                     time: Some(Time {
//                         hour: 7,
//                         minute: 32,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "without-seconds-3".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1979,
//                         month: 5,
//                         day: 27,
//                     }),
//                     time: Some(Time {
//                         hour: 7,
//                         minute: 32,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Custom {
//                         hours: -7,
//                         minutes: 0,
//                     }),
//                 }),
//                 "without-seconds-4".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1979,
//                         month: 5,
//                         day: 27,
//                     }),
//                     time: Some(Time {
//                         hour: 7,
//                         minute: 32,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: None,
//                 }),
//             })
//         );

//         let value = Parser::from_str(
//             r"
// utc  = 1987-07-05T17:45:56Z
// pdt  = 1987-07-05T17:45:56-05:00
// nzst = 1987-07-05T17:45:56+12:00
// nzdt = 1987-07-05T17:45:56+13:00  # DST
// ",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "utc".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "pdt".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Custom {
//                         hours: -5,
//                         minutes: 0,
//                     }),
//                 }),
//                 "nzst".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Custom {
//                         hours: 12,
//                         minutes: 0,
//                     }),
//                 }),
//                 "nzdt".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 56,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Custom {
//                         hours: 13,
//                         minutes: 0,
//                     }),
//                 }),
//             })
//         );
//     }

//     #[test]
//     fn test_empty() {
//         let value = Parser::from_str("").parse().unwrap();

//         assert_eq!(value, RawValue::Table(hashmap! {}));
//     }

//     #[test]
//     fn test_example() {
//         let value = Parser::from_str(
//             r"
// best-day-ever = 1987-07-05T17:45:00Z

// [numtheory]
// boring = false
// perfection = [6, 28, 496]",
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "best-day-ever".into() => RawValue::Datetime(Datetime {
//                     date: Some(Date {
//                         year: 1987,
//                         month: 7,
//                         day: 5,
//                     }),
//                     time: Some(Time {
//                         hour: 17,
//                         minute: 45,
//                         second: 0,
//                         nanosecond: 0,
//                     }),
//                     offset: Some(Offset::Z),
//                 }),
//                 "numtheory".into() => RawValue::Table(hashmap! {
//                     "boring".into() => RawValue::Boolean(false),
//                     "perfection".into() => RawValue::Array(vec![
//                         RawValue::Integer(b"6".into()),
//                         RawValue::Integer(b"28".into()),
//                         RawValue::Integer(b"496".into()),
//                     ]),
//                 }),
//             })
//         );
//     }

//     #[test]
//     fn test_implicit_and_explicit_after() {
//         let value = Parser::from_str("[a.b.c]\nanswer = 42\n\n[a]\nbetter = 43\n")
//             .parse()
//             .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "a".into() => RawValue::Table(hashmap! {
//                     "b".into() => RawValue::UndefinedTable(hashmap! {
//                         "c".into() => RawValue::Table(hashmap! {
//                             "answer".into() => RawValue::Integer(b"42".into()),
//                         }),
//                     }),
//                     "better".into() => RawValue::Integer(b"43".into()),
//                 }),
//             })
//         );
//     }

//     #[test]
//     fn test_implicit_and_explicit_before() {
//         let value = Parser::from_str("[a]\nbetter = 43\n\n[a.b.c]\nanswer = 42\n")
//             .parse()
//             .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "a".into() => RawValue::Table(hashmap! {
//                     "better".into() => RawValue::Integer(b"43".into()),
//                     "b".into() => RawValue::UndefinedTable(hashmap! {
//                         "c".into() => RawValue::Table(hashmap! {
//                             "answer".into() => RawValue::Integer(b"42".into()),
//                         }),
//                     }),
//                 }),
//             })
//         );
//     }

//     #[test]
//     fn test_implicit_groups() {
//         let value = Parser::from_str("[a.b.c]\nanswer = 42\n").parse().unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "a".into() => RawValue::UndefinedTable(hashmap! {
//                     "b".into() => RawValue::UndefinedTable(hashmap! {
//                         "c".into() => RawValue::Table(hashmap! {
//                             "answer".into() => RawValue::Integer(b"42".into()),
//                         }),
//                     }),
//                 }),
//             })
//         );
//     }

//     #[test]
//     fn test_newline_crlf() {
//         let value = Parser::from_str("os = \"DOS\"\r\nnewline = \"crlf\"\r\n")
//             .parse()
//             .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "os".into() => RawValue::String("DOS".into()),
//                 "newline".into() => RawValue::String("crlf".into()),
//             })
//         );
//     }

//     #[test]
//     fn test_newline_lf() {
//         let value = Parser::from_str("os = \"unix\"\nnewline = \"lf\"\n")
//             .parse()
//             .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "os".into() => RawValue::String("unix".into()),
//                 "newline".into() => RawValue::String("lf".into()),
//             })
//         );
//     }

//     #[test]
//     fn test_example_1_compact() {
//         let value = Parser::from_str(
//             r#"#Useless spaces eliminated.
// title="TOML Example"
// [owner]
// name="Lance Uppercut"
// dob=1979-05-27T07:32:00-08:00#First class dates
// [database]
// server="192.168.1.1"
// ports=[8001,8001,8002]
// connection_max=5000
// enabled=true
// [servers]
// [servers.alpha]
// ip="10.0.0.1"
// dc="eqdc10"
// [servers.beta]
// ip="10.0.0.2"
// dc="eqdc10"
// [clients]
// data=[["gamma","delta"],[1,2]]
// hosts=[
// "alpha",
// "omega"
// ]"#,
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "title".into() => RawValue::String("TOML Example".into()),
//                 "owner".into() => RawValue::Table(hashmap! {
//                     "name".into() => RawValue::String("Lance Uppercut".into()),
//                     "dob".into() => RawValue::Datetime(Datetime {
//                         date: Some(Date {
//                             year: 1979,
//                             month: 5,
//                             day: 27,
//                         }),
//                         time: Some(Time {
//                             hour: 7,
//                             minute: 32,
//                             second: 0,
//                             nanosecond: 0,
//                         }),
//                         offset: Some(Offset::Custom {
//                             hours: -8,
//                             minutes: 0,
//                         }),
//                     }),
//                 }),
//                 "database".into() => RawValue::Table(hashmap! {
//                     "server".into() => RawValue::String("192.168.1.1".into()),
//                     "ports".into() => RawValue::Array(vec![
//                         RawValue::Integer(b"8001".into()),
//                         RawValue::Integer(b"8001".into()),
//                         RawValue::Integer(b"8002".into()),
//                     ]),
//                     "connection_max".into() => RawValue::Integer(b"5000".into()),
//                     "enabled".into() => RawValue::Boolean(true),
//                 }),
//                 "servers".into() => RawValue::Table(hashmap! {
//                     "alpha".into() => RawValue::Table(hashmap! {
//                         "ip".into() => RawValue::String("10.0.0.1".into()),
//                         "dc".into() => RawValue::String("eqdc10".into()),
//                     }),
//                     "beta".into() => RawValue::Table(hashmap! {
//                         "ip".into() => RawValue::String("10.0.0.2".into()),
//                         "dc".into() => RawValue::String("eqdc10".into()),
//                     }),
//                 }),
//                 "clients".into() => RawValue::Table(hashmap! {
//                     "data".into() => RawValue::Array(vec![
//                         RawValue::Array(vec![RawValue::String("gamma".into()), RawValue::String("delta".into())]),
//                         RawValue::Array(vec![RawValue::Integer(b"1".into()), RawValue::Integer(b"2".into())]),
//                     ]),
//                     "hosts".into() => RawValue::Array(vec![RawValue::String("alpha".into()), RawValue::String("omega".into())]),
//                 }),
//             })
//         );
//     }

//     #[test]
//     fn test_example_1() {
//         let value = Parser::from_str(
//             r#"
// # This is a TOML document. Boom.

// title = "TOML Example"

// [owner]
// name = "Lance Uppercut"
// dob = 1979-05-27T07:32:00-08:00 # First class dates? Why not?

// [database]
// server = "192.168.1.1"
// ports = [ 8001, 8001, 8002 ]
// connection_max = 5000
// enabled = true

// [servers]

//   # You can indent as you please. Tabs or spaces. TOML don't care.
//   [servers.alpha]
//   ip = "10.0.0.1"
//   dc = "eqdc10"

//   [servers.beta]
//   ip = "10.0.0.2"
//   dc = "eqdc10"

// [clients]
// data = [ ["gamma", "delta"], [1, 2] ]

// # Line breaks are OK when inside arrays
// hosts = [
//   "alpha",
//   "omega"
// ]
//         "#,
//         )
//         .parse()
//         .unwrap();

//         assert_eq!(
//             value,
//             RawValue::Table(hashmap! {
//                 "title".into() => RawValue::String("TOML Example".into()),
//                 "owner".into() => RawValue::Table(hashmap! {
//                     "name".into() => RawValue::String("Lance Uppercut".into()),
//                     "dob".into() => RawValue::Datetime(Datetime {
//                         date: Some(Date {
//                             year: 1979,
//                             month: 5,
//                             day: 27,
//                         }),
//                         time: Some(Time {
//                             hour: 7,
//                             minute: 32,
//                             second: 0,
//                             nanosecond: 0,
//                         }),
//                         offset: Some(Offset::Custom {
//                             hours: -8,
//                             minutes: 0,
//                         }),
//                     }),
//                 }),
//                 "database".into() => RawValue::Table(hashmap! {
//                     "server".into() => RawValue::String("192.168.1.1".into()),
//                     "ports".into() => RawValue::Array(vec![
//                         RawValue::Integer(b"8001".into()),
//                         RawValue::Integer(b"8001".into()),
//                         RawValue::Integer(b"8002".into()),
//                     ]),
//                     "connection_max".into() => RawValue::Integer(b"5000".into()),
//                     "enabled".into() => RawValue::Boolean(true),
//                 }),
//                 "servers".into() => RawValue::Table(hashmap! {
//                     "alpha".into() => RawValue::Table(hashmap! {
//                         "ip".into() => RawValue::String("10.0.0.1".into()),
//                         "dc".into() => RawValue::String("eqdc10".into()),
//                     }),
//                     "beta".into() => RawValue::Table(hashmap! {
//                         "ip".into() => RawValue::String("10.0.0.2".into()),
//                         "dc".into() => RawValue::String("eqdc10".into()),
//                     }),
//                 }),
//                 "clients".into() => RawValue::Table(hashmap! {
//                     "data".into() => RawValue::Array(vec![
//                         RawValue::Array(vec![RawValue::String("gamma".into()), RawValue::String("delta".into())]),
//                         RawValue::Array(vec![RawValue::Integer(b"1".into()), RawValue::Integer(b"2".into())]),
//                     ]),
//                     "hosts".into() => RawValue::Array(vec![RawValue::String("alpha".into()), RawValue::String("omega".into())]),
//                 }),
//             })
//         );
//     }

//     //     #[test]
//     //     fn test_test() {
//     //         Parser::from_str(
//     //             r#"[[a]]
//     //     [[a.b]]
//     //         [a.b.c]
//     //             d = "val0"
//     //     [[a.b]]
//     //         [a.b.c]
//     //             d = "val1"
//     // "#,
//     //         )
//     //         .parse()
//     //         .unwrap();
//     //     }
// }
