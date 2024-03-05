use bevy_ecs::{query::{ReadOnlyQueryData, WorldQuery}, system::{ReadOnlySystemParam, SystemParam}};
use bevy_reflect::TypePath;
use bevy_serde_project::typetagged::{BevyTypeTagged, FromTypeTagged};
use dyn_clone::{clone_trait_object, DynClone};
use serde::{de::DeserializeOwned, Serialize};
use crate::{QuerierRef, FullStatMap, types::DynStatValue, BaseStatMap, DynStat, QualifierFlag, QualifierQuery, Stat, StatOperationsMap, TYPE_ERROR};

/// Opaque type that contains a stat and a mutable value.
#[derive(Debug)]
pub struct StatValuePair<'t>(pub(crate) &'t dyn DynStat, pub(crate) &'t mut dyn DynStatValue);

impl StatValuePair<'_> {
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

    /// If stat is of a type, downcast the stat and value.
    pub fn as_then<'a, S: Stat>(&'a mut self, then: impl FnOnce(&S, &'a mut S::Data)) -> bool {
        let StatValuePair(stat, data) = self;
        if let Some(stat) = stat.downcast_ref::<S>() {
            then(stat, data.downcast_mut::<S::Data>().expect(TYPE_ERROR));
            true
        } else {
            false
        }
    }
}

/// A generalized object safe stat relation.
pub trait StatStream<Q: QualifierFlag>: Send + Sync + 'static {
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
pub trait StatStreamObject<Q: QualifierFlag>: StatStream<Q> + DynClone {
    fn name(&self) -> &'static str;
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<Q: QualifierFlag, T: StatStream<Q>> StatStreamObject<Q> for T where T: TypePath + Clone + Serialize {
    fn name(&self) -> &'static str {
        T::short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

clone_trait_object!(<Q: QualifierFlag> StatStreamObject<Q>);

impl<Q: QualifierFlag> BevyTypeTagged for Box<dyn StatStreamObject<Q>>{
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}


impl<Q, T> FromTypeTagged<T> for Box<dyn StatStreamObject<Q>> where Q: QualifierFlag, T: StatStreamObject<Q> + TypePath + DeserializeOwned {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Box::new(item)
    }
}

/// Component and context based stat stream.
///
/// The item is generated from the [`ReadOnlyQueryData`] and a [`SystemParam`] context,
/// For example an `Asset` can be generated from a `Handle` and context `Assets`.
pub trait ComponentStream<Q: QualifierFlag>: 'static {
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

impl<Q: QualifierFlag> BaseStatMap<Q> {
    /// Write qualified entries to a [`StatValuePair`].
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut StatValuePair) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(stat.from_base(op).as_ref()))
    }
}

impl<Q: QualifierFlag> FullStatMap<Q> {
    /// Write qualified entries to a [`StatValuePair`].
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut StatValuePair) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.join_value(op))
    }
}

impl<Q: QualifierFlag> StatOperationsMap<Q> {
    /// Write qualified entries to a [`StatValuePair`].
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut StatValuePair) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(op))
    }
}

impl<Q: QualifierFlag> ComponentStream<Q> for BaseStatMap<Q> {
    type Ctx = ();
    type QueryData = &'static Self;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ){
        this.iter_write(qualifier, pair);
    }
}

impl<Q: QualifierFlag> ComponentStream<Q> for FullStatMap<Q> {
    type Ctx = ();
    type QueryData = &'static Self;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ){
        this.iter_write(qualifier, pair);
    }
}

impl<Q: QualifierFlag> ComponentStream<Q> for StatOperationsMap<Q> {
    type Ctx = ();
    type QueryData = &'static Self;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ){
        this.iter_write(qualifier, pair);
    }
}

/// An item that can be used to generate stats when directly added to `Entity`.
///
/// The item also allows querying for "distance" or other relation between paired components on two entities.
pub trait IntrinsicStream<Qualifier: QualifierFlag>: ComponentStream<Qualifier> {
    /// Write to `stat` and return true ***if a value is written***.
    fn distance (
        ctx: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        other: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Qualifier>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<Qualifier>
    ) -> bool;
}
