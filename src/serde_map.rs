use std::marker::PhantomData;

use crate::{QualifierKey, StatDispatch, stat_map::StatMapEntry};
use serde::de::{DeserializeOwned, DeserializeSeed, Error as DError, MapAccess, SeqAccess};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Visitor, ser::SerializeStruct};

/// Enables [`StatMap`](crate::StatMapBase) serialization.
///
/// Default mode uses the [`Serialize`] implementations on [`StatDispatch`] and [`StatDispatch::Value`].
///
/// Custom implementations can utilize the [`StatDispatch`] variable in serializing [`StatDispatch::Value`],
/// as long as [`Serialize`] is not implemented on the `Value`.
pub trait SerializeEntry: StatDispatch + Serialize {
    /// Serialize the value.
    fn serialize_item<S: Serializer>(
        &self,
        value: &Self::Value,
        serializer: S,
    ) -> Result<S::Ok, S::Error>;
}

/// Enables [`StatMap`](crate::StatMapBase) deserialization.
///
/// Default mode uses the [`Deserialize`] implementations on [`StatDispatch`] and [`StatDispatch::Value`].
///
/// Custom implementations can utilize the [`StatDispatch`] variable in deserializing [`StatDispatch::Value`],
/// as long as [`Deserialize`] is not implemented on the `Value`.
pub trait DeserializeEntry: StatDispatch + DeserializeOwned {
    /// Should return `false` for custom implementations, `true` for default serde implementations.
    const USE_SERDE_IMPLS: bool = false;

    /// Use serde implementation to deserialize the value, do not touch for custom implementations.
    #[allow(unused_variables)]
    fn _use_deserialize_impl<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom(
            "Does not have a deserialize implementation.",
        ))
    }

    /// Deserialize the value.
    fn deserialize_item<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error>;
}

impl<T: StatDispatch> SerializeEntry for T
where
    T: Serialize,
    T::Value: Serialize,
{
    fn serialize_item<S: Serializer>(
        &self,
        value: &Self::Value,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.serialize(serializer)
    }
}

impl<T: StatDispatch> DeserializeEntry for T
where
    T: DeserializeOwned,
    T::Value: DeserializeOwned,
{
    const USE_SERDE_IMPLS: bool = true;

    fn _use_deserialize_impl<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        T::Value::deserialize(deserializer)
    }

    fn deserialize_item<'de, D: Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        T::Value::deserialize(deserializer)
    }
}

impl<Q: QualifierKey + Serialize, S: SerializeEntry> Serialize for StatMapEntry<Q, S> {
    fn serialize<T: Serializer>(&self, serializer: T) -> Result<T::Ok, T::Error> {
        struct SerEntry<'t, S: SerializeEntry>(&'t S, &'t S::Value);
        impl<E: SerializeEntry> Serialize for SerEntry<'_, E> {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.serialize_item(self.1, serializer)
            }
        }
        let mut state = serializer.serialize_struct("Entry", 3)?;
        state.serialize_field("qualifier", &self.qualifier)?;
        state.serialize_field("stat", &self.stat)?;
        state.serialize_field("value", &SerEntry(&self.stat, &self.value))?;
        state.end()
    }
}

impl<'de, Q: QualifierKey + Deserialize<'de>, S: DeserializeEntry> Deserialize<'de>
    for StatMapEntry<Q, S>
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_struct(
            "Entry",
            &["qualifier", "stat", "value"],
            EntryVisitor::<Q, S>(PhantomData),
        )
    }
}

enum Fields {
    Qualifier,
    Stat,
    Value,
    Unknown,
}

struct FieldVisitor;

impl<'de> Deserialize<'de> for Fields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_identifier(FieldVisitor)
    }
}

impl<'de> Visitor<'de> for FieldVisitor {
    type Value = Fields;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct StatMapEntry")
    }

    fn visit_str<E: DError>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "qualifier" => Ok(Fields::Qualifier),
            "stat" => Ok(Fields::Stat),
            "value" => Ok(Fields::Value),
            _ => Ok(Fields::Unknown),
        }
    }

    fn visit_u64<E: DError>(self, v: u64) -> Result<Self::Value, E> {
        match v {
            0 => Ok(Fields::Qualifier),
            1 => Ok(Fields::Stat),
            2 => Ok(Fields::Value),
            _ => Ok(Fields::Unknown),
        }
    }

    fn visit_bytes<E: DError>(self, v: &[u8]) -> Result<Self::Value, E> {
        match v {
            b"qualifier" => Ok(Fields::Qualifier),
            b"stat" => Ok(Fields::Stat),
            b"value" => Ok(Fields::Value),
            _ => Ok(Fields::Unknown),
        }
    }
}

struct DeWithoutCx<S: DeserializeEntry>(S::Value);

impl<'de, S: DeserializeEntry> Deserialize<'de> for DeWithoutCx<S> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        S::_use_deserialize_impl(deserializer).map(DeWithoutCx)
    }
}

struct DeCx<'t, S: DeserializeEntry>(&'t S);

impl<'de, S: DeserializeEntry> DeserializeSeed<'de> for DeCx<'_, S> {
    type Value = S::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.0.deserialize_item(deserializer)
    }
}

struct EntryVisitor<Q: QualifierKey, S: DeserializeEntry>(PhantomData<(Q, S)>);

impl<'de, Q: QualifierKey + Deserialize<'de>, S: DeserializeEntry> Visitor<'de>
    for EntryVisitor<Q, S>
{
    type Value = StatMapEntry<Q, S>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct StatMapEntry")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let Some(qualifier) = seq.next_element::<Q>()? else {
            return Err(DError::invalid_length(
                0usize,
                &"struct StatMapEntry with 3 elements",
            ));
        };
        let Some(stat) = seq.next_element::<S>()? else {
            return Err(DError::invalid_length(
                0usize,
                &"struct StatMapEntry with 3 elements",
            ));
        };
        let value = if S::USE_SERDE_IMPLS {
            let Some(value) = seq.next_element::<DeWithoutCx<S>>()? else {
                return Err(DError::invalid_length(
                    0usize,
                    &"struct StatMapEntry with 3 elements",
                ));
            };
            value.0
        } else {
            let Some(value) = seq.next_element_seed::<DeCx<S>>(DeCx(&stat))? else {
                return Err(DError::invalid_length(
                    0usize,
                    &"struct StatMapEntry with 3 elements",
                ));
            };
            value
        };
        Ok(StatMapEntry {
            stat,
            qualifier,
            value,
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut qualifier = None;
        let mut stat = None;
        let mut value = None;

        while let Some(field) = map.next_key::<Fields>()? {
            match field {
                Fields::Qualifier => {
                    if qualifier.is_some() {
                        return Err(A::Error::duplicate_field("qualifier"));
                    }
                    qualifier = Some(map.next_value()?)
                }
                Fields::Stat => {
                    if stat.is_some() {
                        return Err(A::Error::duplicate_field("stat"));
                    }
                    stat = Some(map.next_value()?)
                }
                Fields::Value => {
                    if value.is_some() {
                        return Err(A::Error::duplicate_field("value"));
                    }
                    if S::USE_SERDE_IMPLS {
                        value = Some(map.next_value::<DeWithoutCx<S>>()?.0)
                    } else if let Some(stat) = &stat {
                        value = Some(map.next_value_seed(DeCx(stat))?)
                    } else {
                        return Err(A::Error::custom(
                            "Requires a serialization format that preserves order.",
                        ));
                    }
                }
                Fields::Unknown => {
                    let () = map.next_value()?;
                }
            }
        }

        let qualifier = match qualifier {
            Some(v) => v,
            None => return Err(A::Error::missing_field("qualifier")),
        };
        let stat = match stat {
            Some(v) => v,
            None => return Err(A::Error::missing_field("stat")),
        };
        let value = match value {
            Some(v) => v,
            None => return Err(A::Error::missing_field("value")),
        };
        Ok(StatMapEntry {
            qualifier,
            stat,
            value,
        })
    }
}
