use std::{borrow::Cow, fmt};

use serde::{
    de::{Error, Visitor},
    Deserializer,
};

struct CowStrVisitor;

impl<'de> Visitor<'de> for CowStrVisitor {
    type Value = Cow<'de, str>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<E: Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Cow::Borrowed(value))
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Cow::Owned(value.to_owned()))
    }

    fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Cow::Owned(value))
    }
}

pub fn deserialize_cow_str<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Cow<'de, str>, D::Error> {
    deserializer.deserialize_str(CowStrVisitor)
}
