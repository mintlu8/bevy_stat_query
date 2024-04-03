use bevy_ecs::{query::{ReadOnlyQueryData, WorldQuery}, system::{ReadOnlySystemParam, SystemParam}};
use bevy_reflect::TypePath;
use bevy_serde_project::typetagged::{BevyTypeTagged, FromTypeTagged};
use dyn_clone::{clone_trait_object, DynClone};
use serde::{de::DeserializeOwned, Serialize};
use crate::{QuerierRef, FullStatMap, types::DynStatValue, BaseStatMap, DynStat, QualifierFlags, QualifierQuery, Stat, StatOperationsMap, TYPE_ERROR};

/// Opaque type that contains a stat and a mutable value.
#[derive(Debug)]
pub struct StatValuePair<'t>(pub(crate) &'t dyn DynStat, pub(crate) &'t mut dyn DynStatValue);

impl<'t> StatValuePair<'t> {

    pub fn new<S: Stat>(stat: &'t S, value: &'t mut S::Data) -> Self{
        StatValuePair(stat, value)
    }

    pub fn is<'a, S: Stat>(&'a mut self, is: &S) -> Option<&'a mut S::Data> {
        let StatValuePair(stat, data) = self;
        if *stat == is as &dyn DynStat {
            Some(data.downcast_mut::<S::Data>().expect(TYPE_ERROR))
        } else {
            None
        }
    }

    /// If stat is a concrete stat, downcast value.
    pub fn is_then<'a, S: Stat>(&'a mut self, is: &S, then: impl FnOnce(&'a mut S::Data)) -> bool {
        let StatValuePair(stat, data) = self;
        if *stat == is as &dyn DynStat {
            then(data.downcast_mut::<S::Data>().expect(TYPE_ERROR));
            true
        } else {
            false
        }
    }


    pub fn cast<S: Stat>(&mut self) -> Option<(&S, &mut S::Data)> {
        let StatValuePair(stat, data) = self;
        if let Some(stat) = stat.downcast_ref::<S>() {
            Some((stat, data.downcast_mut::<S::Data>().expect(TYPE_ERROR)))
        } else {
            None
        }
    }


    /// If stat is of a type, downcast the stat and value.
    pub fn cast_then<'a, S: Stat>(&'a mut self, then: impl FnOnce(&S, &'a mut S::Data)) -> bool {
        let StatValuePair(stat, data) = self;
        if let Some(stat) = stat.downcast_ref::<S>() {
            then(stat, data.downcast_mut::<S::Data>().expect(TYPE_ERROR));
            true
        } else {
            false
        }
    }

    /// Extend the stat value with a stateless stream.
    pub fn extend<Q: QualifierFlags>(&mut self, qualifier: &QualifierQuery<Q>, extend: impl StatExtend<Q>) {
        extend.stat_extend(qualifier, self)
    }

    /// Extend the stat value with a stateful stream.
    pub fn stateful_extend<Q: QualifierFlags>(&mut self, qualifier: &QualifierQuery<Q>, querier: &mut QuerierRef<'_, Q>, extend: impl StatStream<Q>) {
        extend.stream(qualifier, self, querier)
    }
}

/// A generalized object safe stat relation.
pub trait StatExtend<Q: QualifierFlags>: Send + Sync + 'static {
    fn stat_extend (
        &self,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
    );
}

/// A generalized object safe stat relation.
pub trait StatStream<Q: QualifierFlags>: Send + Sync + 'static {
    fn stream (
        &self,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    );
}

/// A generalized object safe stat relation that can be serialized.
/// 
/// Automatically implemented on implementors of [`StatStream`], [`TypePath`] and [`Serialize`].
pub trait StatStreamObject<Q: QualifierFlags>: StatStream<Q> + DynClone {
    fn name(&self) -> &'static str;
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<Q: QualifierFlags, T: StatStream<Q>> StatStreamObject<Q> for T where T: TypePath + Clone + Serialize {
    fn name(&self) -> &'static str {
        T::short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

clone_trait_object!(<Q: QualifierFlags> StatStreamObject<Q>);

impl<Q: QualifierFlags> BevyTypeTagged for Box<dyn StatStreamObject<Q>>{
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}


impl<Q, T> FromTypeTagged<T> for Box<dyn StatStreamObject<Q>> where Q: QualifierFlags, T: StatStreamObject<Q> + TypePath + DeserializeOwned {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Box::new(item)
    }
}

/// An item that can be used to generate stats when directly added to [`StatEntity`](crate::StatEntity).
///
/// The item also allows querying for "distance" or other relation between paired components on two entities.
pub trait IntrinsicStream<Qualifier: QualifierFlags>: ExternalStream<Qualifier> {
    /// Write to `stat` and return true ***if a value is written***.
    fn distance (
        ctx: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        other: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Qualifier>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<Qualifier>
    );
}


/// Component and context based stat streams on children of [`StatEntity`](crate::StatEntity).
///
/// The item is generated from the [`ReadOnlyQueryData`] and a [`SystemParam`] context,
/// For example an `Asset` can be generated from a `Handle` and context `Assets`.
pub trait ExternalStream<Q: QualifierFlags>: 'static {
    type Ctx: ReadOnlySystemParam;
    type QueryData: ReadOnlyQueryData;
    fn stream (
        ctx: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        component: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    );
}

impl<Q: QualifierFlags> StatExtend<Q> for BaseStatMap<Q> {
    fn stat_extend (
        &self,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
    ) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(stat.from_base(op).as_ref()))
    }
}


impl<Q: QualifierFlags> StatExtend<Q> for FullStatMap<Q> {
    fn stat_extend (
        &self,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
    ) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.join_value(op))
    }
}

impl<Q: QualifierFlags> StatStream<Q> for StatOperationsMap<Q> {
    fn stream (
        &self,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(op))
    }
}


impl<Q: QualifierFlags> StatStream<Q> for BaseStatMap<Q> {
    fn stream (
        &self,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(stat.from_base(op).as_ref()))
    }
}


impl<Q: QualifierFlags> StatStream<Q> for FullStatMap<Q> {
    fn stream (
        &self,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.join_value(op))
    }
}

impl<Q: QualifierFlags> StatExtend<Q> for StatOperationsMap<Q> {
    fn stat_extend (
        &self,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
    ) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(op))
    }
}

impl<Q: QualifierFlags> ExternalStream<Q> for BaseStatMap<Q> {
    type Ctx = ();
    type QueryData = Option<&'static Self>;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ){
        if let Some(this) = this {
            this.stat_extend(qualifier, pair);
        }
    }
}

impl<Q: QualifierFlags> ExternalStream<Q> for FullStatMap<Q> {
    type Ctx = ();
    type QueryData = Option<&'static Self>;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ){
        if let Some(this) = this {
            this.stat_extend(qualifier, pair);
        }    
    }
}

impl<Q: QualifierFlags> ExternalStream<Q> for StatOperationsMap<Q> {
    type Ctx = ();
    type QueryData = Option<&'static Self>;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ){
        if let Some(this) = this {
            this.stat_extend(qualifier, pair);
        }    
    }
}


impl<Q: QualifierFlags> IntrinsicStream<Q> for BaseStatMap<Q> {
    fn distance (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        _: <Self::QueryData as WorldQuery>::Item<'_>,
        _: <Self::QueryData as WorldQuery>::Item<'_>,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<Q>
    ) {}
}

impl<Q: QualifierFlags> IntrinsicStream<Q> for FullStatMap<Q> {
    fn distance (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        _: <Self::QueryData as WorldQuery>::Item<'_>,
        _: <Self::QueryData as WorldQuery>::Item<'_>,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<Q>
    ) {}
}

impl<Q: QualifierFlags> IntrinsicStream<Q> for StatOperationsMap<Q> {
    fn distance (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        _: <Self::QueryData as WorldQuery>::Item<'_>,
        _: <Self::QueryData as WorldQuery>::Item<'_>,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<Q>
    ) {}
}

