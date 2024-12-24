use crate::operations::StatOperation;
use crate::stat::StatValuePair;
use crate::{
    Buffer, Qualifier, QualifierFlag, QualifierQuery, Querier, Stat, StatExt,
    StatInst, StatStream, StatValue,
};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem;

pub(crate) struct StatMapEntry<Q: QualifierFlag> {
    stat: StatInst,
    qualifier: Qualifier<Q>,
    buffer: Buffer,
}

impl<Q: QualifierFlag> Clone for StatMapEntry<Q> {
    fn clone(&self) -> Self {
        Self {
            stat: self.stat,
            qualifier: self.qualifier.clone(),
            buffer: unsafe { self.stat.clone_buffer(&self.buffer) },
        }
    }
}

impl<Q: QualifierFlag> Drop for StatMapEntry<Q> {
    fn drop(&mut self) {
        unsafe { (self.stat.vtable.drop)(&mut self.buffer) };
    }
}

impl<Q: QualifierFlag> StatMapEntry<Q> {
    /// # Safety
    ///
    /// `T` must be the stored type.
    unsafe fn take<T: Send + Sync>(mut self) -> T {
        let result = self.buffer.read_move();
        mem::forget(self);
        result
    }
}

/// A type erased storage component of qualified stats.
///
/// This type can hold any qualifier stat combination as long as the qualifier type is the same.
///
/// # Performance
///
/// The type is intended to hold relatively constant stats and prioritizes querying,
/// not optimized for rapid insertion or removal.
///
/// # Serialization
///
/// Deserialization must be done inside a [`bevy_serde_lens_core`] deserialize scope.
#[derive(Component, Serialize, Deserialize, Reflect, Clone)]
#[reflect(Component, Serialize, Deserialize)]
#[reflect(where Q: Serialize + DeserializeOwned)]
pub struct StatMap<Q: QualifierFlag> {
    #[reflect(ignore)]
    inner: Vec<StatMapEntry<Q>>,
}

impl<Q: QualifierFlag> Debug for StatMap<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Stat(&'static str);
        let mut map = f.debug_map();
        for StatMapEntry {
            stat,
            qualifier,
            buffer,
        } in &self.inner
        {
            map.entry(&(qualifier, Stat(stat.name())), unsafe {
                (stat.vtable.as_debug)(buffer)
            });
        }
        map.finish()
    }
}

impl<Q: QualifierFlag> Default for StatMap<Q> {
    fn default() -> Self {
        StatMap { inner: Vec::new() }
    }
}

fn sort<Q: QualifierFlag>(a: &StatMapEntry<Q>, b: &StatMapEntry<Q>) -> Ordering {
    a.stat.cmp(&b.stat).then(a.qualifier.cmp(&b.qualifier))
}

impl<Q: QualifierFlag, S: Stat> FromIterator<(Qualifier<Q>, S, S::Value)> for StatMap<Q> {
    fn from_iter<T: IntoIterator<Item = (Qualifier<Q>, S, S::Value)>>(iter: T) -> Self {
        let mut inner: Vec<_> = iter
            .into_iter()
            .map(|(qualifier, stat, value)| {
                let stat = stat.as_entry();
                StatMapEntry {
                    stat,
                    qualifier,
                    buffer: Buffer::from(value),
                }
            })
            .collect();
        inner.sort_by(sort);
        StatMap { inner }
    }
}

impl<Q: QualifierFlag, S: Stat> Extend<(Qualifier<Q>, S, S::Value)> for StatMap<Q> {
    fn extend<T: IntoIterator<Item = (Qualifier<Q>, S, S::Value)>>(&mut self, iter: T) {
        self.inner
            .extend(iter.into_iter().map(|(qualifier, stat, value)| {
                let stat = stat.as_entry();
                StatMapEntry {
                    stat,
                    qualifier,
                    buffer: Buffer::from(value),
                }
            }));
        self.inner.sort_by(sort);
    }
}

impl<Q: QualifierFlag> StatMap<Q> {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Drops all items in the map.
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Performs a binary search for a value.
    fn binary_search(&self, qualifier: &Qualifier<Q>, stat: &StatInst) -> Result<usize, usize> {
        self.inner.binary_search_by(
            |StatMapEntry {
                 stat: s,
                 qualifier: q,
                 buffer: _,
             }| { (s, q).cmp(&(stat, qualifier)) },
        )
    }

    /// Inserts a [`Stat::Value`] in its component form.
    pub fn insert<S: Stat>(&mut self, qualifier: Qualifier<Q>, stat: S, value: S::Value) {
        let stat = stat.as_entry();
        let buffer = Buffer::from(value);
        match self.binary_search(&qualifier, &stat) {
            Ok(at) => self.inner[at].buffer = buffer,
            Err(at) => self.inner.insert(
                at,
                StatMapEntry {
                    stat,
                    qualifier,
                    buffer,
                },
            ),
        };
    }

    /// Inserts a [`Stat::Value`] in its evaluated form.
    pub fn insert_base<S: Stat>(
        &mut self,
        qualifier: Qualifier<Q>,
        stat: S,
        base: <S::Value as StatValue>::Base,
    ) {
        let stat = stat.as_entry();
        let buffer = Buffer::from(S::Value::from_base(base));
        match self.binary_search(&qualifier, &stat) {
            Ok(at) => self.inner[at].buffer = buffer,
            Err(at) => self.inner.insert(
                at,
                StatMapEntry {
                    stat,
                    qualifier,
                    buffer,
                },
            ),
        };
    }

    /// Obtains a [`Stat::Value`].
    pub fn get<S: Stat>(&self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&S::Value> {
        let stat = stat.as_entry();
        match self.binary_search(qualifier, &stat) {
            Ok(at) => Some(unsafe { self.inner[at].buffer.as_ref() }),
            Err(_) => None,
        }
    }

    /// Obtains a mutable [`Stat::Value`].
    pub fn get_mut<S: Stat>(
        &mut self,
        qualifier: &Qualifier<Q>,
        stat: &S,
    ) -> Option<&mut S::Value> {
        let stat = stat.as_entry();
        match self.binary_search(qualifier, &stat) {
            Ok(at) => Some(unsafe { self.inner[at].buffer.as_mut() }),
            Err(_) => None,
        }
    }

    /// Removes and obtains a [`Stat::Value`].
    pub fn remove<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<S::Value> {
        let stat = stat.as_entry();
        match self.binary_search(qualifier, &stat) {
            Ok(at) => Some(unsafe { self.inner.remove(at).take() }),
            Err(_) => None,
        }
    }

    /// Obtains a [`Stat::Value`] in its evaluated form.
    pub fn get_evaled<S: Stat>(
        &self,
        qualifier: &Qualifier<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        let stat = stat.as_entry();
        match self.binary_search(qualifier, &stat) {
            Ok(at) => Some(unsafe { self.inner[at].buffer.as_ref::<S::Value>().eval() }),
            Err(_) => None,
        }
    }

    /// Iterate over a particular stat.
    pub(crate) fn slice(&self, stat: StatInst) -> &[StatMapEntry<Q>] {
        let fst = self.inner.partition_point(|x| x.stat < stat);
        let snd = self.inner.partition_point(|x| x.stat <= stat);
        &self.inner[fst..snd]
    }

    /// Iterate over a particular stat.
    pub(crate) fn slice_mut(&mut self, stat: StatInst) -> &mut [StatMapEntry<Q>] {
        let fst = self.inner.partition_point(|x| x.stat < stat);
        let snd = self.inner.partition_point(|x| x.stat <= stat);
        &mut self.inner[fst..snd]
    }

    /// Iterate over a particular stat.
    pub fn iter<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &S::Value)> {
        let stat = stat.as_entry();
        self.slice(stat)
            .iter()
            .map(|x| (&x.qualifier, unsafe { x.buffer.as_ref() }))
    }

    /// Iterate over a particular stat.
    pub fn iter_mut<S: Stat>(
        &mut self,
        stat: &S,
    ) -> impl Iterator<Item = (&Qualifier<Q>, &mut S::Value)> {
        let stat = stat.as_entry();
        self.slice_mut(stat)
            .iter_mut()
            .map(|x| (&x.qualifier, unsafe { x.buffer.as_mut() }))
    }

    /// Remove all instances of a given stat.
    pub fn remove_all<S: Stat>(&mut self, stat: &S) {
        let stat = stat.as_entry();
        let fst = self.inner.partition_point(|x| x.stat < stat);
        let snd = self.inner.partition_point(|x| x.stat <= stat);
        self.inner.drain(fst..snd);
    }

    /// Create or modify a stat via a [`StatOperation`].
    ///
    /// Create a [`Default`] stat if non-existent.
    pub fn modify<S: Stat>(
        &mut self,
        qualifier: Qualifier<Q>,
        stat: S,
        value: StatOperation<S::Value>,
    ) {
        let stat = stat.as_entry();
        match self.binary_search(&qualifier, &stat) {
            Ok(at) => value.write_to(unsafe { self.inner[at].buffer.as_mut() }),
            Err(at) => {
                let buffer = Buffer::from(value.into_stat());
                self.inner.insert(
                    at,
                    StatMapEntry {
                        stat,
                        qualifier,
                        buffer,
                    },
                );
            }
        }
    }

    /// Create or modify a stat via a closure.
    ///
    /// Create a [`Default`] stat if non-existent.
    pub fn modify_with<S: Stat>(
        &mut self,
        qualifier: Qualifier<Q>,
        stat: &S,
        f: impl FnOnce(&mut S::Value),
    ) {
        let stat = stat.as_entry();
        match self.binary_search(&qualifier, &stat) {
            Ok(at) => f(unsafe { self.inner[at].buffer.as_mut() }),
            Err(at) => {
                let mut value = Default::default();
                f(&mut value);
                let buffer = Buffer::from(value);
                self.inner.insert(
                    at,
                    StatMapEntry {
                        stat,
                        qualifier,
                        buffer,
                    },
                );
            }
        }
    }

    pub fn query_stat<S: Stat>(&self, qualifier: &QualifierQuery<Q>, stat: &S) -> S::Value {
        let mut stat = StatValuePair::new_default(stat);
        self.stream_stat(
            Entity::PLACEHOLDER,
            qualifier,
            &mut stat,
            Querier::noop(),
        );
        unsafe { stat.value.into::<S::Value>() }
    }

    pub fn eval_stat<S: Stat>(
        &self,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
    ) -> <S::Value as StatValue>::Out {
        self.query_stat(qualifier, stat).eval()
    }
}

impl<Q: QualifierFlag> StatStream for StatMap<Q> {
    type Qualifier = Q;

    fn stream_stat(
        &self,
        _: Entity,
        qualifier: &crate::QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        _: Querier<Q>,
    ) {
        let f = stat_value.stat.vtable.join;
        for entry in self.slice(stat_value.stat) {
            if entry.qualifier.qualifies_as(qualifier) {
                unsafe { f(&mut stat_value.value, &entry.buffer) };
            }
        }
    }
}

impl<Q: QualifierFlag + Serialize> Serialize for StatMapEntry<Q> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.qualifier)?;
        seq.serialize_element(&self.stat.name())?;
        seq.serialize_element(unsafe { &(self.stat.vtable.as_serialize)(&self.buffer) })?;
        seq.end()
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> Deserialize<'de> for StatMapEntry<Q> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (qualifier, stat, buffer) =
            deserializer.deserialize_seq(TupleSeed::<Q>(PhantomData))?;
        Ok(StatMapEntry {
            stat,
            qualifier,
            buffer,
        })
    }
}

pub struct TupleSeed<Q: QualifierFlag>(PhantomData<Q>);

pub struct DynSeed<Q: QualifierFlag> {
    f: fn(&mut dyn erased_serde::Deserializer) -> erased_serde::Result<Buffer>,
    q: PhantomData<Q>,
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> DeserializeSeed<'de> for TupleSeed<Q> {
    type Value = (Qualifier<Q>, StatInst, Buffer);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(TupleSeed::<Q>(PhantomData))
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> Visitor<'de> for TupleSeed<Q> {
    type Value = (Qualifier<Q>, StatInst, Buffer);

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("(qualifier, stat, value)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let Some(qualifier) = seq.next_element()? else {
            return Err(serde::de::Error::custom("Expected qualifier."));
        };
        let Some(stat) = seq.next_element::<StatInst>()? else {
            return Err(serde::de::Error::custom("Expected stat name."));
        };
        let Some(buffer) = seq.next_element_seed(DynSeed {
            f: stat.vtable.deserialize,
            q: PhantomData::<Q>,
        })?
        else {
            return Err(serde::de::Error::custom("Expected stat value."));
        };
        Ok((qualifier, stat, buffer))
    }
}

impl<'de, Q: QualifierFlag> DeserializeSeed<'de> for DynSeed<Q> {
    type Value = Buffer;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let deserializer = &mut <dyn erased_serde::Deserializer>::erase(deserializer);
        (self.f)(deserializer).map_err(serde::de::Error::custom)
    }
}
