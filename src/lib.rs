#![allow(missing_docs, clippy::missing_errors_doc)]

pub use self::de::{from_reader, from_slice, from_str, Deserializer};
pub use self::ser::{to_string, /* to_string_pretty, */ Serializer};
pub use self::value::Value;

pub mod value;

pub mod de;
pub mod ser;
