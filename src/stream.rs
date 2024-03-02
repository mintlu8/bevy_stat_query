use bevy_ecs::{entity::Entity, query::{QueryData, ReadOnlyQueryData, WorldQuery}, system::SystemParam};
use crate::{sealed::Sealed, QualifierFlag, QualifierQuery, Stat, StatOperationsMap};

/// A stat that is obtainable from intrinsic properties of a unit.
/// Implementing this enables the `query_intrinsic` and `query_distance`
/// functions on [`StatQuerier`]
///
/// # Note
///
/// This is separate from the regulat stat evaluation routine,
/// so the stat is either not directly evaluatable, aka a dummy stat,
/// or you have to manually implement its fetching routine based on this.
pub trait FromIntrinsics: Stat {
    type IntrisicQuery: ReadOnlyQueryData;

    /// Obtain an intrinsic value from the current unit.
    #[allow(clippy::wrong_self_convention)]
    fn from_intrinsic(&self,
        unit: &<<Self::IntrisicQuery as QueryData>::ReadOnly as WorldQuery>::Item<'_>,
    ) -> <Self as Stat>::Data;

    /// Obtain a "distance" value from 2 units. Can be used for other relations like allegiance.
    #[allow(clippy::wrong_self_convention)]
    fn from_distance(&self,
        this: &<<Self::IntrisicQuery as QueryData>::ReadOnly as WorldQuery>::Item<'_>,
        other: &<<Self::IntrisicQuery as QueryData>::ReadOnly as WorldQuery>::Item<'_>,
    ) -> <Self as Stat>::Data;
}

/// A trait that can be used to obtain stat relations from other stats or other entities.
pub trait StatQuerier<Q: QualifierFlag, D: QueryData>: Sealed {
    fn query<S: Stat>(&mut self, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
    fn query_other<S: Stat>(&mut self, entity: Entity, qualifier: &QualifierQuery<Q>, stat: &S) -> Option<S::Data>;
    fn query_intrinsic<S: Stat + FromIntrinsics<IntrisicQuery = D>>(&mut self, stat: &S) -> Option<S::Data>;
    fn query_distance<S: Stat + FromIntrinsics<IntrisicQuery = D>>(&mut self, entity: Entity, stat: &S) -> Option<S::Data>;
}

/// An item that can be used to generate stats when its associated component
/// is added as child to a queriable unit.
///
/// The item is generated from the [`QueryData`] and a [`SystemParam`] context,
/// For example an `Asset` can be generated from a `Handle` and context `Assets`.
pub trait StatStream<Qualifier: QualifierFlag, Intrinsic: QueryData>: 'static {
    type Ctx: SystemParam;
    type QueryData: ReadOnlyQueryData;
    fn stream<S: Stat>(
        ctx: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        component: <Self::QueryData as WorldQuery>::Item<'_>,
        write: &mut S::Data,
        qualifier: &QualifierQuery<Qualifier>,
        stat: &S,
        querier: &mut impl StatQuerier<Qualifier, Intrinsic>
    );
}

impl<Q: QualifierFlag, D: QueryData> StatStream<Q, D> for StatOperationsMap<Q> {
    type Ctx = ();
    type QueryData = &'static Self;

    fn stream<S: Stat>(
        _: &<Self::Ctx as SystemParam>::Item<'_, '_>,
        this: <Self::QueryData as WorldQuery>::Item<'_>,
        write: &mut S::Data, qualifier: &QualifierQuery<Q>,
        stat: &S,
        _: &mut impl StatQuerier<Q, D>
    ){
        this.iter(stat)
            .filter(|(q, _)| q.qualifies_as(qualifier))
            .for_each(|(_, v)| v.write_to(write))
    }
}
