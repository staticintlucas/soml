use serde::ser;

use super::Value;

impl ser::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Self::String(ref str) => str.serialize(serializer),
            Self::Integer(int) => int.serialize(serializer),
            Self::Float(float) => float.serialize(serializer),
            Self::Boolean(bool) => bool.serialize(serializer),
            Self::Datetime(ref datetime) => datetime.serialize(serializer),
            Self::Array(ref array) => array.serialize(serializer),
            Self::Table(ref table) => table.serialize(serializer),
        }
    }
}
