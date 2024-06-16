use crate::operations::StatOperation;
use crate::{validate, Buffer, Qualifier, QualifierFlag, Stat, StatInst, StatStream, StatValue};
use bevy_ecs::component::Component;
use bevy_reflect::TypePath;
use ref_cast::RefCast;
use serde::de::{DeserializeSeed, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Bound, RangeBounds};
use std::{borrow::Borrow, collections::BTreeMap, hash::Hash};

/// A map-like, type erased storage for stats.
#[derive(Debug, Clone, Component, TypePath)]
pub struct StatMap<Q: QualifierFlag> {
    inner: BTreeMap<(StatInst, Qualifier<Q>), Buffer>,
}

impl<Q: QualifierFlag> Default for StatMap<Q> {
    fn default() -> Self {
        StatMap {
            inner: BTreeMap::new(),
        }
    }
}

unsafe fn from_buffer_ref<T>(buffer: &Buffer) -> &T {
    validate::<T>();
    unsafe { (buffer.as_ptr() as *const T).as_ref() }.unwrap()
}

unsafe fn from_buffer_mut<T>(buffer: &mut Buffer) -> &mut T {
    validate::<T>();
    unsafe { (buffer.as_mut_ptr() as *mut T).as_mut() }.unwrap()
}

unsafe fn from_buffer<T>(mut buffer: Buffer) -> T {
    validate::<T>();
    unsafe { (buffer.as_mut_ptr() as *mut T).read() }
}

unsafe fn into_buffer<T>(item: T) -> Buffer {
    validate::<T>();
    let mut buffer = [MaybeUninit::uninit(); 3];
    unsafe { (buffer.as_mut_ptr() as *mut T).write(item) };
    buffer
}

impl<Q: QualifierFlag> StatMap<Q> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn insert<S: Stat>(&mut self, qualifier: Qualifier<Q>, stat: S, value: S::Data) {
        self.inner
            .insert((stat.as_entry(), qualifier), unsafe { into_buffer(value) });
    }

    pub fn insert_base<S: Stat>(
        &mut self,
        qualifier: Qualifier<Q>,
        stat: S,
        base: <S::Data as StatValue>::Base,
    ) {
        self.inner.insert((stat.as_entry(), qualifier), unsafe {
            into_buffer(S::Data::from_base(base))
        });
    }

    pub fn insert_op<S: Stat>(
        &mut self,
        qualifier: Qualifier<Q>,
        stat: S,
        value: StatOperation<S::Data>,
    ) {
        self.inner.insert((stat.as_entry(), qualifier), unsafe {
            into_buffer(value.into_stat())
        });
    }

    pub fn get<S: Stat>(&self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&S::Data> {
        self.inner
            .get(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { from_buffer_ref(buffer) })
    }

    pub fn get_mut<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&mut S::Data> {
        self.inner
            .get_mut(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { from_buffer_mut(buffer) })
    }

    pub fn remove<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<S::Data> {
        self.inner
            .remove(&(stat.as_entry(), qualifier) as &dyn QueryStatEntry<Q>)
            .map(|buffer| unsafe { from_buffer(buffer) })
    }

    pub fn retain(&mut self, mut f: impl FnMut(&Qualifier<Q>, &str) -> bool) {
        self.inner
            .retain(|(s, q), _| f(q, (s.vtable.name)(s.index)));
    }

    /// Iterate over a particular stat.
    pub fn iter<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &S::Data)> {
        self.inner
            .range(stat.as_entry())
            .map(|((_, q), v)| (q, unsafe { from_buffer_ref(v) }))
    }

    /// Iterate over a particular stat.
    pub fn iter_mut<S: Stat>(
        &mut self,
        stat: &S,
    ) -> impl Iterator<Item = (&Qualifier<Q>, &mut S::Data)> {
        self.inner
            .range_mut(stat.as_entry())
            .map(|((_, q), v)| (q, unsafe { from_buffer_mut(v) }))
    }

    pub fn modify<S: Stat>(
        &mut self,
        qualifier: &Qualifier<Q>,
        stat: &S,
        f: impl FnOnce(&mut S::Data),
    ) {
        if let Some(val) = self.get_mut(qualifier, stat) {
            f(val)
        } else {
            let mut val = Default::default();
            f(&mut val);
            self.insert(qualifier.clone(), stat.clone(), val);
        }
    }
}

impl<Q: QualifierFlag> StatStream<Q> for StatMap<Q> {
    fn stream_stat<S: Stat>(
        &self,
        qualifier: &crate::QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        _: &impl crate::Querier<Q>,
    ) {
        self.iter(stat).for_each(|(q, v)| {
            if q.qualifies_as(qualifier) {
                value.join(v.clone())
            }
        })
    }
}

trait QueryStatEntry<Q: QualifierFlag> {
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>>;
    fn stat(&self) -> StatInst;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum QuerySort<T> {
    Begin,
    Value(T),
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

impl<Q: QualifierFlag> Hash for dyn QueryStatEntry<Q> + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.qualifier().hash(state);
        self.stat().hash(state);
    }
}

impl<Q: QualifierFlag> QueryStatEntry<Q> for (StatInst, Qualifier<Q>) {
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::Value(&self.1)
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<'t, Q: QualifierFlag> QueryStatEntry<Q> for (StatInst, &'t Qualifier<Q>) {
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::Value(self.1)
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<'t, Q: QualifierFlag> QueryStatEntry<Q> for (StatInst, QuerySort<&'t Qualifier<Q>>) {
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
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
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::Begin
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<Q: QualifierFlag> QueryStatEntry<Q> for End<StatInst> {
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::End
    }

    fn stat(&self) -> StatInst {
        self.0
    }
}

impl<'a, Q: QualifierFlag> RangeBounds<dyn QueryStatEntry<Q> + 'a> for StatInst {
    fn start_bound(&self) -> Bound<&(dyn QueryStatEntry<Q> + 'a)> {
        Bound::Excluded(Begin::ref_cast(self))
    }

    fn end_bound(&self) -> Bound<&(dyn QueryStatEntry<Q> + 'a)> {
        Bound::Excluded(End::ref_cast(self))
    }
}

impl<Q: QualifierFlag + Serialize> Serialize for StatMap<Q> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.inner.iter().map(|((s, q), d)| {
            (q, (s.vtable.name)(s.index), unsafe {
                (s.vtable.as_serialize)(d)
            })
        }))
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
        formatter.write_str("stat map")
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
    f: unsafe fn(&mut dyn erased_serde::Deserializer) -> erased_serde::Result<Buffer>,
    q: PhantomData<Q>,
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> DeserializeSeed<'de> for TupleSeed<Q> {
    type Value = (Qualifier<Q>, StatInst, Buffer);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(TupleSeed::<Q>(PhantomData))
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>> Visitor<'de> for TupleSeed<Q> {
    type Value = (Qualifier<Q>, StatInst, Buffer);

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("stat map")
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
        unsafe { (self.f)(deserializer) }.map_err(serde::de::Error::custom)
    }
}
