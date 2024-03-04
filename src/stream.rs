use bevy_ecs::{entity::Entity, query::{ReadOnlyQueryData, WorldQuery}, system::SystemParam};
use crate::{sealed::Sealed, stat_map::FullStatMap, types::DynStatValue, BaseStatMap, DynStat, QualifierFlag, QualifierQuery, Stat, StatOperationsMap, TYPE_ERROR};

/// Opaque type that contains a stat and a value.
#[derive(Debug)]
pub struct StatValuePair<'t>(pub(crate) &'t dyn DynStat, pub(crate) &'t mut dyn DynStatValue);

impl StatValuePair<'_> {
    pub fn is_then<'a, S: Stat>(&'a mut self, is: &S, then: impl FnOnce(&'a mut S::Data)) -> bool {
        let StatValuePair(stat, data) = self;
        if *stat == is as &dyn DynStat {
            then(data.downcast_mut::<S::Data>().expect(TYPE_ERROR));
            true
        } else {
            false
        }
    }

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

/// A trait that can be used to obtain stat relations from other stats or other entities.
pub trait StatQuerier<Q: QualifierFlag>: Sealed {
    fn query<S: Stat>(&mut self, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
    fn query_other<S: Stat>(&mut self, entity: Entity, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
    fn query_distance<S: Stat>(&mut self, entity: Entity, qualifier: &crate::QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
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
        stat: &mut StatValuePair,
        querier: &mut impl StatQuerier<Q>
    );
}

impl<Q: QualifierFlag> BaseStatMap<Q> {
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut StatValuePair) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(&stat.from_base(op)))
    }
}

impl<Q: QualifierFlag> FullStatMap<Q> {
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut StatValuePair) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.join_value(op))
    }
}

impl<Q: QualifierFlag> StatOperationsMap<Q> {
    pub fn iter_write(&self, qualifier: &QualifierQuery<Q>, pair: &mut StatValuePair) {
        let StatValuePair(stat, data) = pair;
        self.iter_dyn(*stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, op)| data.apply_op(op))
    }
}

impl<Q: QualifierFlag> StatStream<Q> for StatOperationsMap<Q> {
    type Ctx = ();
    type QueryData = &'static Self;

    fn stream (
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        pair: &mut StatValuePair,
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
        stat: &mut StatValuePair,
        querier: &mut impl StatQuerier<Qualifier>
    );
}
