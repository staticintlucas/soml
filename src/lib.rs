#![allow(missing_docs, clippy::missing_errors_doc)]

pub mod value;

pub mod de;
// pub mod ser;

pub use de::{from_reader, from_slice, from_str, Deserializer};
pub use value::Value;
