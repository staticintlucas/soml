use std::io::Write as _;
use std::{fmt, io};

use super::Result;

mod private {
    pub trait Sealed {}
}

pub trait Writer: private::Sealed {
    fn write_str(&mut self, s: &str) -> Result<()>;

    fn write_char(&mut self, c: char) -> Result<()> {
        self.write_str(c.encode_utf8(&mut [0; 4]))
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()>;
}

impl<T> private::Sealed for T where T: fmt::Write {}

impl<T> Writer for T
where
    T: fmt::Write,
{
    fn write_str(&mut self, s: &str) -> Result<()> {
        <Self as fmt::Write>::write_str(self, s).map_err(Into::into)
    }

    fn write_char(&mut self, c: char) -> Result<()> {
        <Self as fmt::Write>::write_char(self, c).map_err(Into::into)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        <Self as fmt::Write>::write_fmt(self, args).map_err(Into::into)
    }
}

// pub struct FmtWriter<T> {
//     inner: T,
// }

// impl<T> FmtWriter<T> {
//     pub fn new(writer: T) -> Self {
//         Self {
//             inner: writer,
//         }
//     }
// }

// impl<T> private::Sealed for FmtWriter<T> where T: fmt::Write {}

// impl<T> Writer for FmtWriter<T>
// where
//     T: fmt::Write,
// {
//     fn write_str(&mut self, s: &str) -> Result<()> {
//         self.inner.write_str(s).map_err(Into::into)
//     }

//     fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
//         self.inner.write_fmt(args).map_err(Into::into)
//     }
// }

#[derive(Debug)]
pub struct IoWriter<T: io::Write> {
    inner: io::BufWriter<T>,
}

impl<T> IoWriter<T>
where
    T: io::Write,
{
    pub fn new(writer: T) -> Self {
        Self {
            inner: io::BufWriter::new(writer),
        }
    }
}

impl<T> private::Sealed for IoWriter<T> where T: io::Write {}

impl<T> Writer for IoWriter<T>
where
    T: io::Write,
{
    fn write_str(&mut self, s: &str) -> Result<()> {
        self.inner.write_all(s.as_bytes()).map_err(Into::into)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        self.inner.write_fmt(args).map_err(Into::into)
    }
}
