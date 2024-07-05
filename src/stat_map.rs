use crate::operations::StatOperation;
use crate::stat::StatValuePair;
use crate::{
    Buffer, Qualifier, QualifierFlag, Querier, Stat, StatExt, StatInst, StatStream, StatValue,
};
use bevy_ecs::component::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use ref_cast::RefCast;
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::{borrow::Borrow, collections::BTreeMap, hash::Hash};

/// A map-like, type erased storage of qualified stats.
///
/// # Serialization
///
/// Deserialization must be done inside a [`bevy_serde_lens_core`] deserialize scope.
///
/// # Safety Invariant
///
/// `StatInst` and `Buffer` must match on all entries.
#[derive(Component, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
#[reflect(where Q: Serialize + DeserializeOwned)]
pub struct StatMap<Q: QualifierFlag> {
    #[reflect(ignore)]
    inner: BTreeMap<(StatInst, Qualifier<Q>), Buffer>,
}

impl<Q: QualifierFlag> Debug for StatMap<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Stat(&'static str);
        let mut map = f.debug_map();
        for ((s, q), b) in &self.inner {
            map.entry(&(q, Stat(s.name())), unsafe { (s.vtable.as_debug)(b) });
        }
        map.finish()
    }
}

impl<Q: QualifierFlag> Clone for StatMap<Q> {
    fn clone(&self) -> Self {
        StatMap {
            inner: self
                .inner
                .iter()
                .map(|((s, q), b)| ((*s, q.clone()), unsafe { s.clone_buffer(b) }))
                .collect(),
        }
    }
}

impl<Q: QualifierFlag> Drop for StatMap<Q> {
    fn drop(&mut self) {
        for ((s, _), b) in &mut self.inner {
            unsafe { (s.vtable.drop)(b) }
        }
    }
}

impl<Q: QualifierFlag> Default for StatMap<Q> {
    fn default() -> Self {
        StatMap {
            inner: BTreeMap::new(),
        }
    }
}

impl<Q: QualifierFlag> StatMap<Q> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::default(),
        }
    }

    /// Drops all items in the map.
    pub fn clear(&mut self) {
        for ((s, _), b) in &mut self.inner {
            unsafe { (s.vtable.drop)(b) }
        }
        self.inner.clear()
    }

    /// Inserts a [`Stat::Value`] in its component form.
    pub fn insert<S: Stat>(&mut self, qualifier: Qualifier<Q>, stat: S, value: S::Value) {
        self.inner
            .insert((stat.as_entry(), qualifier), Buffer::from(value));
    }

    /// Inserts a [`Stat::Value`] in its evaluated form.
    pub fn insert_base<S: Stat>(
        &mut self,
        qualifier: Qualifier<Q>,
        stat: S,
        base: <S::Value as StatValue>::Base,
    ) {
        self.inner.insert(
            (stat.as_entry(), qualifier),
            Buffer::from(S::Value::from_base(base)),
        );
    }

    /// Obtains a [`Stat::Value`].
    pub fn get<S: Stat>(&self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&S::Value> {
        self.inner
            .get(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { buffer.as_ref() })
    }

    /// Obtains a mutable [`Stat::Value`].
    pub fn get_mut<S: Stat>(
        &mut self,
        qualifier: &Qualifier<Q>,
        stat: &S,
    ) -> Option<&mut S::Value> {
        self.inner
            .get_mut(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { buffer.as_mut() })
    }

    /// Removes and obtains a [`Stat::Value`].
    pub fn remove<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<S::Value> {
        self.inner
            .remove(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { buffer.into() })
    }

    /// Obtains a [`Stat::Value`] in its evaluated form.
    pub fn get_evaled<S: Stat>(
        &self,
        qualifier: &Qualifier<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        self.inner
            .get(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { buffer.as_ref::<S::Value>().eval() })
    }

    /// Iterate over a particular stat.
    pub fn iter<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &S::Value)> {
        self.inner
            .range(stat.as_entry())
            .map(|((_, q), v)| (q, unsafe { v.as_ref() }))
    }

    /// Iterate over a particular stat.
    pub fn iter_mut<S: Stat>(
        &mut self,
        stat: &S,
    ) -> impl Iterator<Item = (&Qualifier<Q>, &mut S::Value)> {
        self.inner
            .range_mut(stat.as_entry())
            .map(|((_, q), v)| (q, unsafe { v.as_mut() }))
    }

    /// Remove all instances of a given stat.
    pub fn remove_all<S: Stat>(&mut self, stat: &S) {
        self.inner.retain(|(s, _), v| {
            if s == &stat.as_entry() {
                unsafe { s.drop_buffer(v) }
                false
            } else {
                true
            }
        })
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
        self.inner
            .entry((stat.as_entry(), qualifier))
            .and_modify(|buffer| value.write_to(unsafe { buffer.as_mut() }))
            .or_insert(Buffer::from(value.into_stat()));
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
        if let Some(val) = self.get_mut(&qualifier, stat) {
            f(val)
        } else {
            let mut val = Default::default();
            f(&mut val);
            self.insert(qualifier, stat.clone(), val);
        }
    }
}

impl<Q: QualifierFlag> StatStream<Q> for StatMap<Q> {
    fn stream_stat(
        &self,
        qualifier: &crate::QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        _: Querier<Q>,
    ) {
        self.inner.range(stat_value.stat).for_each(|((s, q), v)| {
            if q.qualifies_as(qualifier) {
                unsafe { (s.vtable.join)(&mut stat_value.value, v) };
            }
        })
    }
}

trait QueryStatEntry<Q: QualifierFlag> {
    fn qualifier(&self) -> QuerySort<Q>;
    fn stat(&self) -> StatInst;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum QuerySort<'t, Q: QualifierFlag> {
    Begin,
    Value(&'t Qualifier<Q>),
    End,
}

impl<Q: QualifierFlag> PartialEq for dyn QueryStatEntry<Q> + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.qualifier() == other.qualifier() && self.stat() == other.stat()
    }
}

impl<Q: QualifierFlag> Eq for dyn QueryStatEntry<Q> + '_ {}

impl<Q: QualifierFlag> PartialOrd for dyn QueryStatEntry<Q> + '_ {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Q: QualifierFlag> Ord for dyn QueryStatEntry<Q> + '_ {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.stat()
            .cmp(&other.stat())
            .then(self.qualifier().cmp(&other.qualifier()))
    }
}

impl<Q: QualifierFlag> QueryStatEntry<Q> for (StatInst, Qualifier<Q>) {
    fn qualifier(&self) -> QuerySort<Q> {
        QuerySort::Value(&self.1)
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<'t, Q: QualifierFlag> QueryStatEntry<Q> for (StatInst, &'t Qualifier<Q>) {
    fn qualifier(&self) -> QuerySort<Q> {
        QuerySort::Value(self.1)
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<Q: QualifierFlag> QueryStatEntry<Q> for (StatInst, QuerySort<'_, Q>) {
    fn qualifier(&self) -> QuerySort<Q> {
        match &self.1 {
            QuerySort::Begin => QuerySort::Begin,
            QuerySort::Value(v) => QuerySort::Value(v),
            QuerySort::End => QuerySort::End,
        }
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<'a, Q: QualifierFlag> Borrow<dyn QueryStatEntry<Q> + 'a> for (StatInst, Qualifier<Q>) {
    fn borrow(&self) -> &(dyn QueryStatEntry<Q> + 'a) {
        self
    }
}

impl<'a, Q: QualifierFlag> Borrow<dyn QueryStatEntry<Q> + 'a> for (StatInst, &'a Qualifier<Q>) {
    fn borrow(&self) -> &(dyn QueryStatEntry<Q> + 'a) {
        self
    }
}

#[derive(Debug, RefCast)]
#[repr(transparent)]
struct Begin<T>(T);

#[derive(Debug, RefCast)]
#[repr(transparent)]
struct End<T>(T);

impl<Q: QualifierFlag> QueryStatEntry<Q> for Begin<StatInst> {
    fn qualifier(&self) -> QuerySort<Q> {
        QuerySort::Begin
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<Q: QualifierFlag> QueryStatEntry<Q> for End<StatInst> {
    fn qualifier(&self) -> QuerySort<Q> {
        QuerySort::End
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<'a, Q: QualifierFlag> RangeBounds<dyn QueryStatEntry<Q> + 'a> for StatInst {
    fn start_bound(&self) -> Bound<&(dyn QueryStatEntry<Q> + 'a)> {
        Bound::Included(Begin::ref_cast(self))
    }

    fn end_bound(&self) -> Bound<&(dyn QueryStatEntry<Q> + 'a)> {
        Bound::Included(End::ref_cast(self))
    }
}

impl<Q: QualifierFlag + Serialize> Serialize for StatMap<Q> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.inner.len()))?;
        for ((s, q), d) in &self.inner {
            seq.serialize_element(&SeqTuple3((q, (s.vtable.name)(s.index), unsafe {
                (s.vtable.as_serialize)(d)
            })))?;
        }
        seq.end()
    }
}

pub struct SeqTuple3<A, B, C>((A, B, C));

impl<A: Serialize, B: Serialize, C: Serialize> Serialize for SeqTuple3<A, B, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(3))?;
        seq.serialize_element(&self.0 .0)?;
        seq.serialize_element(&self.0 .1)?;
        seq.serialize_element(&self.0 .2)?;
        seq.end()
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> Deserialize<'de> for StatMap<Q> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut result = Self::new();
        deserializer.deserialize_seq(&mut result)?;
        Ok(result)
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> Visitor<'de> for &mut StatMap<Q> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("stat map sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while let Some((q, s, v)) = seq.next_element_seed(TupleSeed(PhantomData::<Q>))? {
            self.inner.insert((s, q), v);
        }
        Ok(())
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
