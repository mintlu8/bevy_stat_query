use std::any::Any;
use std::{borrow::Borrow, collections::BTreeMap, hash::Hash};
use std::ops::{Bound, Deref, DerefMut, RangeBounds};
use bevy_ecs::component::Component;
use bevy_reflect::{Reflect, TypePath};
use ref_cast::RefCast;
use crate::{Data, Stat, Qualifier, QualifierFlag, DynStat, StatComponents, StatOperation, TYPE_ERROR};

/// A map-like, type erased storage for stats.
/// When present on an entity with [`StatEntity`](crate::StatEntity)
/// will be used as the base stats of the unit.
///
/// This stores the output value of a stat and uses
/// [`StatComponents::from_out`] to covert back into
/// [`StatComponents`] or [`StatOperation`](crate::StatOperation)
///
/// The map is optimized for looking up all qualifiers with a specific [`Stat`].
///
/// Although the implementation is type erased,
/// the public interface is completely type safe.
#[derive(Debug, Clone, Component, TypePath)]
#[type_path = "bse"]
struct StatMapInner<Q: QualifierFlag, D>{
    inner: BTreeMap<(Box<dyn DynStat>, Qualifier<Q>), D>,
}

impl<Q: QualifierFlag, D> Default for StatMapInner<Q, D> {
    fn default() -> Self {
        StatMapInner { inner: BTreeMap::new() }
    }
}

type SOut<T> = <<T as Stat>::Data as StatComponents>::Out;

impl<Q: QualifierFlag, D> StatMapInner<Q, D> {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::default(),
        }
    }

    /// Obtain an unqualified view of a [`StatMap`].
    pub fn unqualified(&self) -> &Unqualified<Self> {
        Unqualified::ref_cast(self)
    }

    /// Obtain an mutable unqualified view of a [`StatMap`].
    pub fn unqualified_mut(&mut self) -> &mut Unqualified<Self> {
        Unqualified::ref_cast_mut(self)
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

    /// Iterate over a particulat stat.
    pub fn iter<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &D)> {
        self.inner
            .range(stat as &dyn DynStat)
            .map(|((_, q), v)| (q, v))
    }

    /// Iterate over a particulat stat.
    pub fn iter_mut<S: Stat>(&mut self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &mut D)> {
        self.inner
            .range_mut(stat as &dyn DynStat)
            .map(|((_, q), v)| (q, v))
    }
}

/// An unqualified view of a [`StatMap`].
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
    ($name: ident, $concrete: ident, $stat: ident, $value: ty) => {
        #[derive(Debug, Clone, Default, Component, TypePath)]
        #[type_path = "bse"]
        pub struct $name<Q: QualifierFlag>(StatMapInner<Q, Box<dyn Data>>);

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
                self.0.insert(qualifier, stat, Box::new(value));
            }

            pub fn get<$stat: Stat>(&self, qualifier: &Qualifier<Q>, stat: &$stat) -> Option<&$value> {
                self.0.get(qualifier, stat).map(|v| v.downcast_ref().expect(TYPE_ERROR))
            }

            pub fn get_mut<$stat: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &$stat) -> Option<&mut $value> {
                self.0.get_mut(qualifier, stat).map(|v| v.downcast_mut().expect(TYPE_ERROR))
            }

            pub fn remove<$stat: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &$stat) -> Option<$value> {
                self.0.remove(qualifier, stat).map(|v| *v.downcast().expect(TYPE_ERROR))
            }

            pub fn retain(&mut self, f: impl FnMut(&Qualifier<Q>, &dyn Any) -> bool) {
                self.0.retain(f);
            }

            /// Iterate over a particulat stat.
            pub fn iter<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &$value)> {
                self.0.iter(stat)
                    .map(|(q, v)| (q, v.downcast_ref().expect(TYPE_ERROR)))
            }

            /// Iterate over a particulat stat.
            pub fn iter_mut<S: Stat>(&mut self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &mut $value)> {
                self.0.iter_mut(stat)
                    .map(|(q, v)| (q, v.downcast_mut().expect(TYPE_ERROR)))
            }
        }
    };
}

impl_stat_map!(StatMap, TypedStatMap, S, SOut<S>);

// impl<Q: QualifierFlag> Unqualified<StatMapInner<Q>> {
//     pub fn insert<S: Stat>(&mut self, stat: S, value: SOut<S>) {
//         self.inner.insert((Box::new(stat), Qualifier::default()), Box::new(value));
//     }

//     pub fn get<S: Stat>(&self, stat: &S) -> Option<&SOut<S>> {
//         self.inner
//             .get(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
//             .map(|x| x.downcast_ref::<SOut<S>>().expect(TYPE_ERROR))
//     }

//     pub fn get_mut<S: Stat>(&mut self, stat: &S) -> Option<&mut SOut<S>> {
//         self.inner
//             .get_mut(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
//             .map(|x| x.downcast_mut::<SOut<S>>().expect(TYPE_ERROR))
//     }

//     pub fn remove<S: Stat>(&mut self, stat: &S) -> Option<SOut<S>> {
//         self.inner
//             .remove(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
//             .map(|x| *x.downcast::<SOut<S>>().expect(TYPE_ERROR))
//     }
// }

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

// /// A map-like, type erased storage for [`StatOperation`]s.
// ///
// /// The map is optimized for looking up all qualifiers with a specific [`Stat`].
// ///
// /// Although the implementation is type erased,
// /// the public interface is completely type safe.
// #[derive(Debug, Clone, Default, Component)]
// pub struct StatOperationsMap<Q: QualifierFlag>(StatMapInner<Q>);

// type SOp<T> = StatOperation<<T as Stat>::Data>;

// impl<Q: QualifierFlag> StatOperationsMap<Q> {
//     pub fn new() -> Self {
//         Self(StatMapInner::new())
//     }

//     /// Obtain an unqualified view of a [`StatMap`].
//     pub fn unqualified(&self) -> &Unqualified<Self> {
//         Unqualified::ref_cast(self)
//     }

//     /// Obtain an mutable unqualified view of a [`StatMap`].
//     pub fn unqualified_mut(&mut self) -> &mut Unqualified<Self> {
//         Unqualified::ref_cast_mut(self)
//     }

//     pub fn clear(&mut self) {
//         self.0.clear()
//     }

//     pub fn insert<S: Stat>(&mut self, qualifier: Qualifier<Q>, stat: S, value: SOp<S>) {
//         self.0.inner.insert((Box::new(stat), qualifier), Box::new(value));
//     }

//     pub fn get<S: Stat>(&self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&SOp<S>> {
//         self.0.inner
//             .get(&(stat as &dyn DynStat, qualifier) as &dyn QueryStatEntry<Q>)
//             .map(|x| x.downcast_ref::<SOp<S>>().expect(TYPE_ERROR))
//     }

//     pub fn get_mut<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<&mut SOp<S>> {
//         self.0.inner
//             .get_mut(&(stat as &dyn DynStat, qualifier) as &dyn QueryStatEntry<Q>)
//             .map(|x| x.downcast_mut::<SOp<S>>().expect(TYPE_ERROR))
//     }

//     pub fn remove<S: Stat>(&mut self, qualifier: &Qualifier<Q>, stat: &S) -> Option<SOp<S>> {
//         self.0.inner
//             .remove(&(stat as &dyn DynStat, qualifier) as &dyn QueryStatEntry<Q>)
//             .map(|x| *x.downcast::<SOp<S>>().expect(TYPE_ERROR))
//     }

//     pub fn retain(&mut self, mut f: impl FnMut(&Qualifier<Q>, &dyn Any) -> bool) {
//         self.0.inner.retain(|(s, q), _| f(q, s.as_any()));
//     }

//     /// Iterate over a particulat stat.
//     pub fn iter_stat<S: Stat>(&self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &SOp<S>)> {
//         self.0.inner
//             .range(stat as &dyn DynStat)
//             .map(|((_, q), v)| (q, v.downcast_ref().expect(TYPE_ERROR)))
//     }

//     /// Iterate over a particulat stat.
//     pub fn iter_stat_mut<S: Stat>(&mut self, stat: &S) -> impl Iterator<Item = (&Qualifier<Q>, &mut SOp<S>)> {
//         self.0.inner
//             .range_mut(stat as &dyn DynStat)
//             .map(|((_, q), v)| (q, v.downcast_mut().expect(TYPE_ERROR)))
//     }
// }

// impl<Q: QualifierFlag> Unqualified<StatOperationsMap<Q>> {
//     pub fn insert<S: Stat>(&mut self, stat: S, value: SOp<S>) {
//         self.0.0.inner.insert((Box::new(stat), Qualifier::default()), Box::new(value));
//     }

//     pub fn get<S: Stat>(&self, stat: &S) -> Option<&SOp<S>> {
//         self.0.0.inner
//             .get(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
//             .map(|x| x.downcast_ref::<SOp<S>>().expect(TYPE_ERROR))
//     }

//     pub fn get_mut<S: Stat>(&mut self, stat: &S) -> Option<&mut SOp<S>> {
//         self.0.0.inner
//             .get_mut(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
//             .map(|x| x.downcast_mut::<SOp<S>>().expect(TYPE_ERROR))
//     }

//     pub fn remove<S: Stat>(&mut self, stat: &S) -> Option<SOp<S>> {
//         self.0.0.inner
//             .remove(&(stat as &dyn DynStat, &Qualifier::default()) as &dyn QueryStatEntry<Q>)
//             .map(|x| *x.downcast::<SOp<S>>().expect(TYPE_ERROR))
//     }
// }

// impl<Q: QualifierFlag + TypePath + Reflect> bevy_reflect::Map for StatMapInner<Q> {
//     fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
//         todo!()
//     }

//     fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
//         todo!()
//     }

//     fn get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
//         todo!()
//     }

//     fn get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)> {
//         todo!()
//     }

//     fn len(&self) -> usize {
//         todo!()
//     }

//     fn iter(&self) -> bevy_reflect::MapIter {
//         todo!()
//     }

//     fn drain(self: Box<Self>) -> Vec<(Box<dyn Reflect>, Box<dyn Reflect>)> {
//         todo!()
//     }

//     fn clone_dynamic(&self) -> bevy_reflect::DynamicMap {
//         todo!()
//     }

//     fn insert_boxed(
//         &mut self,
//         key: Box<dyn Reflect>,
//         value: Box<dyn Reflect>,
//     ) -> Option<Box<dyn Reflect>> {
//         todo!()
//     }

//     fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
//         todo!()
//     }
// }
