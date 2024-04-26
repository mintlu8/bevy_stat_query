use std::any::Any;
use std::{borrow::Borrow, collections::BTreeMap, hash::Hash};
use std::ops::{Bound, Deref, DerefMut, RangeBounds};
use bevy_ecs::component::Component;
use bevy_reflect::TypePath;
use ref_cast::RefCast;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use crate::types::DynStatValue;
use crate::{Data, Stat, Qualifier, QualifierFlag, DynStat, StatValue, StatOperation, TYPE_ERROR};
use bevy_serde_lens::typetagged::TypeTagged;
/// A map-like, type erased storage for stats.
/// When present on an entity with [`StatEntity`](crate::StatEntity)
/// will be used as the base stats of the unit.
///
/// This stores the output value of a stat and uses
/// [`StatValue::from_out`] to covert back into
/// [`StatValue`] or [`StatOperation`](crate::StatOperation)
///
/// The map is optimized for looking up all qualifiers with a specific [`Stat`].
///
/// Although the implementation is type erased,
/// the public interface is completely type safe.
#[derive(Debug, Clone, Component, TypePath)]
struct StatMapInner<Q: QualifierFlag, D>{
    inner: BTreeMap<(Box<dyn DynStat>, Qualifier<Q>), D>,
}

impl<Q: QualifierFlag + Serialize, T: Serialize> Serialize for StatMapInner<Q, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.collect_seq(self.inner.iter().map(|((s, q), d)| (q, s, d)))
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>, T: Deserialize<'de>> Deserialize<'de> for StatMapInner<Q, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        let mut result = Self::new();
        deserializer.deserialize_seq(&mut result)?;
        Ok(result)
    }
}

impl<'de, Q: QualifierFlag + Deserialize<'de>, T: Deserialize<'de>> Visitor<'de> for &mut StatMapInner<Q, T> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("stat map")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: serde::de::SeqAccess<'de>, {
        while let Some((q, s, v)) = seq.next_element()? {
            self.inner.insert((s, q), v);
        }
        Ok(())
    }
}


impl<Q: QualifierFlag, D> Default for StatMapInner<Q, D> {
    fn default() -> Self {
        StatMapInner { inner: BTreeMap::new() }
    }
}

type SData<T> = <T as Stat>::Data;
type SOut<T> = <<T as Stat>::Data as StatValue>::Out;
type SOp<T> = StatOperation<<T as Stat>::Data>;

impl<Q: QualifierFlag, D> StatMapInner<Q, D> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear()
    }

    pub fn insert<S: Stat>(&mut self, qualifier: Qualifier<Q>, stat: S, value: D) {
        self.inner.insert((Box::new(stat), qualifier), value);
    }

    pub fn get<S: Stat>(&self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&D> {
        self.inner.get(&(stat as &dyn DynStat, qualifier) as &dyn QueryStatEntry<Q>)
    }

    pub fn get_mut<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&mut D> {
        self.inner.get_mut(&(stat as &dyn DynStat, qualifier) as &dyn QueryStatEntry<Q>)
    }

    pub fn remove<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<D> {
        self.inner.remove(&(stat as &dyn DynStat, qualifier) as &dyn QueryStatEntry<Q>)
    }

    pub fn retain(&mut self, mut f: impl FnMut(&Qualifier<Q>, &dyn Any) -> bool) {
        self.inner.retain(|(s, q), _| f(q, s.as_any()));
    }

    /// Iterate over a particular stat.
    pub fn iter(&self, stat: &dyn DynStat) -> impl Iterator<Item = (&Qualifier<Q>, &D)> {
        self.inner
            .range(stat)
            .map(|((_, q), v)| (q, v))
    }

    /// Iterate over a particular stat.
    pub fn iter_mut(&mut self, stat: &dyn DynStat) -> impl Iterator<Item = (&Qualifier<Q>, &mut D)> {
        self.inner
            .range_mut(stat)
            .map(|((_, q), v)| (q, v))
    }
}

/// An unqualified view of a stat map.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Unqualified<T>(T);

impl<T> Deref for Unqualified<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Unqualified<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

macro_rules! impl_stat_map {
    (
        $(#[$($attrs: tt)*])*
        $name: ident, $stat: ident, $value: ty, $trait_obj: ident
    ) => {
        $(#[$($attrs)*])*
        #[derive(Debug, Clone, Default, Component, TypePath, Serialize, Deserialize)]
        pub struct $name<Q: QualifierFlag>(StatMapInner<Q, TypeTagged<Box<dyn $trait_obj>>>);

        impl<Q: QualifierFlag> $name<Q> {
            pub fn new() -> Self {
                Self(StatMapInner::new())
            }

            /// Obtain an unqualified view.
            pub fn unqualified(&self) -> &Unqualified<Self> {
                Unqualified::ref_cast(self)
            }

            /// Obtain an mutable unqualified view.
            pub fn unqualified_mut(&mut self) -> &mut Unqualified<Self> {
                Unqualified::ref_cast_mut(self)
            }

            pub fn clear(&mut self) {
                self.0.clear()
            }

            pub fn insert<$stat: Stat>(&mut self, qualifier: Qualifier<Q>, stat: $stat, value: $value) {
                self.0.insert(qualifier, stat, TypeTagged(Box::new(value)));
            }

            pub fn get<$stat: Stat>(&self, qualifier: &Qualifier<Q>, stat: &$stat) -> Option<&$value> {
                self.0.get(qualifier, stat).map(|v| v.downcast_ref().expect(TYPE_ERROR))
            }

            pub fn get_mut<$stat: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &$stat) -> Option<&mut $value> {
                self.0.get_mut(qualifier, stat).map(|v| v.downcast_mut().expect(TYPE_ERROR))
            }

            pub fn remove<$stat: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &$stat) -> Option<$value> {
                self.0.remove(qualifier, stat).map(|v| *v.0.downcast().expect(TYPE_ERROR))
            }

            pub fn retain(&mut self, f: impl FnMut(&Qualifier<Q>, &dyn Any) -> bool) {
                self.0.retain(f);
            }

            /// Iterate over a particular stat.
            pub fn iter<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &$value)> {
                self.0.iter(stat)
                    .map(|(q, v)| (q, v.downcast_ref().expect(TYPE_ERROR)))
            }

            /// Iterate over a particular stat.
            pub fn iter_mut<S: Stat>(&mut self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &mut $value)> {
                self.0.iter_mut(stat)
                    .map(|(q, v)| (q, v.downcast_mut().expect(TYPE_ERROR)))
            }

            /// Iterate over a particular stat.
            pub(crate) fn iter_dyn(&self, stat: &dyn DynStat) -> impl Iterator<Item = (&Qualifier<Q>, &dyn $trait_obj)> {
                self.0.iter(stat)
                    .map(|(q, v)| (q, v.as_ref()))
            }

            #[allow(unused)]
            /// Iterate over a particular stat.
            pub(crate) fn iter_dyn_mut(&mut self, stat: &dyn DynStat) -> impl Iterator<Item = (&Qualifier<Q>, &mut dyn $trait_obj)> {
                self.0.iter_mut(stat)
                    .map(|(q, v)| (q, v.as_mut()))
            }
        }

        impl<Q: QualifierFlag> Unqualified<$name<Q>> {
            pub fn insert<S: Stat>(&mut self, stat: S, value: $value) {
                self.0.0.inner.insert((Box::new(stat), Qualifier::default()), TypeTagged(Box::new(value)));
            }

            pub fn get<S: Stat>(&self, stat: &S) -> Option<&$value> {
                self.0.0.inner
                    .get(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
                    .map(|x| x.downcast_ref::<$value>().expect(TYPE_ERROR))
            }

            pub fn get_mut<S: Stat>(&mut self, stat: &S) -> Option<&mut $value> {
                self.0.0.inner
                    .get_mut(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
                    .map(|x| x.downcast_mut::<$value>().expect(TYPE_ERROR))
            }

            pub fn remove<S: Stat>(&mut self, stat: &S) -> Option<$value> {
                self.0.0.inner
                    .remove(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
                    .map(|x| *x.0.downcast::<$value>().expect(TYPE_ERROR))
            }
        }
    };
}

impl_stat_map!(
    /// A map containing associated output value of stats.
    BaseStatMap, S, SOut<S>, Data
);
impl_stat_map!(
    /// A map containing associated [`StatValue`] of stats.
    FullStatMap, S, SData<S>, DynStatValue
);
impl_stat_map!(
    /// A map containing associated [`StatOperation`]s of stats.
    StatOperationsMap, S, SOp<S>, Data
);

impl<Q: QualifierFlag> BaseStatMap<Q> {
    pub fn modify<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S, f: impl FnOnce(&mut SOut<S>)){
        if let Some(val) = self.get_mut(qualifier, stat){
            f(val)
        } else {
            let mut val = SOut::<S>::default();
            f(&mut val);
            self.insert(qualifier.clone(), stat.clone(), val);
        }
    }
}

impl<Q: QualifierFlag> FullStatMap<Q> {
    pub fn modify<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S, f: impl FnOnce(&mut SData<S>)){
        if let Some(val) = self.get_mut(qualifier, stat){
            f(val)
        } else {
            let mut val = SData::<S>::default();
            f(&mut val);
            self.insert(qualifier.clone(), stat.clone(), val);
        }
    }
}

trait QueryStatEntry<Q: QualifierFlag> {
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>>;
    fn stat(&self) -> &dyn DynStat;
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
        self.stat().cmp(other.stat())
            .then(self.qualifier().cmp(&other.qualifier()))
    }
}

impl<Q: QualifierFlag> Hash for dyn QueryStatEntry<Q> + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.qualifier().hash(state);
        self.stat().hash(state);
    }
}


impl<Q: QualifierFlag> QueryStatEntry<Q> for (Box<dyn DynStat>, Qualifier<Q>){
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::Value(&self.1)
    }

    fn stat(&self) -> &dyn DynStat {
        self.0.as_ref()
    }
}

impl<'t, Q: QualifierFlag> QueryStatEntry<Q> for (&'t dyn DynStat, &'t Qualifier<Q>){
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::Value(self.1)
    }

    fn stat(&self) -> &dyn DynStat {
        self.0
    }
}

impl<'t, Q: QualifierFlag> QueryStatEntry<Q> for (&'t dyn DynStat, QuerySort<&'t Qualifier<Q>>){
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        match &self.1 {
            QuerySort::Begin => QuerySort::Begin,
            QuerySort::Value(v) => QuerySort::Value(v),
            QuerySort::End => QuerySort::End,
        }
    }

    fn stat(&self) -> &dyn DynStat {
        self.0
    }
}

impl<'a, Q: QualifierFlag> Borrow<dyn QueryStatEntry<Q> + 'a> for (Box<dyn DynStat>, Qualifier<Q>){
    fn borrow(&self) -> &(dyn QueryStatEntry<Q> + 'a) {
        self
    }
}

impl<'a, Q: QualifierFlag> Borrow<dyn QueryStatEntry<Q> + 'a> for (&'a dyn DynStat, &'a Qualifier<Q>){
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

impl<Q: QualifierFlag> QueryStatEntry<Q> for Begin<&dyn DynStat>{
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::Begin
    }

    fn stat(&self) -> &dyn DynStat {
        self.0
    }
}

impl<Q: QualifierFlag> QueryStatEntry<Q> for End<&dyn DynStat>{
    fn qualifier(&self) -> QuerySort<&Qualifier<Q>> {
        QuerySort::End
    }

    fn stat(&self) -> &dyn DynStat {
        self.0
    }
}

impl<'a, Q: QualifierFlag> RangeBounds<dyn QueryStatEntry<Q> + 'a> for &'a dyn DynStat {
    fn start_bound(&self) -> Bound<&(dyn QueryStatEntry<Q> + 'a)> {
        Bound::Excluded(Begin::ref_cast(self))
    }

    fn end_bound(&self) -> Bound<&(dyn QueryStatEntry<Q> + 'a)> {
        Bound::Excluded(End::ref_cast(self))
    }
}
