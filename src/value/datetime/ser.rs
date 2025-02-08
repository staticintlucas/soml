use serde::ser;

use super::{Datetime, LocalDate, LocalDatetime, LocalTime, OffsetDatetime};

impl ser::Serialize for Datetime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
        s.end()
    }
}

impl ser::Serialize for OffsetDatetime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
        s.end()
    }
}

impl ser::Serialize for LocalDatetime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
        s.end()
    }
}

impl ser::Serialize for LocalDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
        s.end()
    }
}

impl ser::Serialize for LocalTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct as _;

        let mut s = serializer.serialize_struct(Self::WRAPPER_TYPE, 1)?;
        s.serialize_field(Self::WRAPPER_FIELD, self.to_string().as_str())?;
        s.end()
    }
}
