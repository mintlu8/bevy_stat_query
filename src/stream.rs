use bevy_ecs::{entity::Entity, query::{QueryData, ReadOnlyQueryData, WorldQuery}, system::SystemParam};
use crate::{sealed::{SealToken, Sealed}, Data, DynStat, QualifierFlag, QualifierQuery, Stat, StatMap, StatOperationsMap, TYPE_ERROR};

pub trait StatValuePair {
    fn as_dyn(&mut self, sealed: SealToken) -> (&dyn DynStat, &mut dyn Data);
}

impl dyn StatValuePair {
    fn is_then<S: Stat>(&mut self, is: &S, then: impl FnOnce(&mut S::Data)) {
        let (stat, data) = self.as_dyn(SealToken);
        if stat == is as &dyn DynStat {
            then(data.downcast_mut::<S::Data>().expect(TYPE_ERROR))
        }
    }
}

pub struct StatValueMut<'t, S: Stat> {
    pub stat: &'t S,
    pub write: &'t mut S::Data,
}

impl StatValuePair for (&dyn DynStat, &mut dyn Data) {
    fn as_dyn(&mut self, sealed: SealToken) -> (&dyn DynStat, &mut dyn Data) {
        *self
    }
}

impl<T: Stat> StatValuePair for StatValueMut<'_, T> {
    fn as_dyn(&mut self, sealed: SealToken) -> (&dyn DynStat, &mut dyn Data) {
        (self.stat, self.write)
    }
}

/// A trait that can be used to obtain stat relations from other stats or other entities.
pub trait StatQuerier<Q: QualifierFlag>: Sealed {
    fn query<S: Stat>(&mut self, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
    fn query_other<S: Stat>(&mut self, entity: Entity, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
    fn query_distance<S: Stat>(&mut self, entity: Entity, stat: &S) -> Option<S::Data>;
}

/// An item that can be used to generate stats when its associated component
/// is added as child to a queriable unit.
///
/// The item is generated from the [`QueryData`] and a [`SystemParam`] context,
/// For example an `Asset` can be generated from a `Handle` and context `Assets`.
pub trait StatStream<Q: QualifierFlag>: 'static {
    type Ctx: SystemParam;
    type QueryData: ReadOnlyQueryData;
    fn stream (
        ctx: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        component: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        stat: &mut dyn StatValuePair,
        querier: &mut impl StatQuerier<Q>
    );
}

impl<Q: QualifierFlag> StatMap<Q> {
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut dyn StatValuePair) {
        let (stat, value) = pair.as_dyn(SealToken);
        self.iter_dyn(stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|_|())
            //.for_each(|(_, v)| v.write_to(write))
    }
}

impl<Q: QualifierFlag> StatOperationsMap<Q> {
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut dyn StatValuePair) {
        let (stat, value) = pair.as_dyn(SealToken);
        self.iter_dyn(stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|_|())
            //.for_each(|(_, v)| v.write_to(write))
    }
}

impl<Q: QualifierFlag> StatStream<Q> for StatOperationsMap<Q> {
    type Ctx = ();
    type QueryData = &'static Self;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut dyn StatValuePair,
        _: &mut impl StatQuerier<Q>
    ){
        this.iter_write(qualifier, pair);
    }
}


/// An item that can be used to generate stats when directly added to `Entity`.
///
/// The item also allows querying for "distance" or other relation between two entities.
pub trait ContextStream<Qualifier: QualifierFlag>: StatStream<Qualifier> {
    fn distance<S: Stat>(
        ctx: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        other: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Qualifier>,
        stat: &mut dyn StatValuePair,
        querier: &mut impl StatQuerier<Qualifier>
    );
}
