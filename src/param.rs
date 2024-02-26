use std::{borrow::Borrow, marker::PhantomData};
use bevy_ecs::{entity::Entity, query::{QueryData, Without}, system::{Query, StaticSystemParam, SystemParam}};
use crate::{stream::{StatQuerier, StatStream}, traits::QualifierFlag, Stat, QualifierQuery, StatCache};


/// [`SystemParam`] that can be aggregated as stat components.
pub trait StatParam<Q: QualifierFlag, D: QueryData>: SystemParam {
    fn stream<S: Stat, E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        write: &mut S::Data,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        querier: &mut impl StatQuerier<Q, D>,
    );
}

/// [`SystemParam`] that queries for a specific [`StatStream`] in an entity.
#[derive(SystemParam)]
pub struct ChildStatParam<'w, 's, T: StatStream<Q, D>, Q: QualifierFlag, D: QueryData + 'static> {
    pub ctx: StaticSystemParam<'w, 's, <T as StatStream<Q, D>>::Ctx>,
    pub query: Query<'w, 's, <T as StatStream<Q, D>>::QueryData, Without<StatCache<Q>>>,
    p: PhantomData<Q>,
}

impl<T: StatStream<Q, D>, Q: QualifierFlag, D: QueryData + 'static> StatParam<Q, D> for ChildStatParam<'_, '_, T, Q, D> {
    fn stream<S: Stat, E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E>,
        write: &mut S::Data,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        querier: &mut impl StatQuerier<Q, D>,
    ) {
        for handle in this.query.iter_many(entities) {
            T::stream(&*this.ctx, handle, write, qualifier, stat, querier);
        }
    }
}

impl<Q: QualifierFlag, D: QueryData> StatParam<Q, D> for () {
    fn stream<S: Stat, E: Borrow<Entity>>(
        _: &Self::Item<'_, '_>,
        _: impl IntoIterator<Item = E>,
        _: &mut S::Data,
        _: &QualifierQuery<Q>,
        _: &S,
        _: &mut impl StatQuerier<Q, D>,
    ) {}
}

impl<A, B, Q: QualifierFlag, D: QueryData> StatParam<Q, D> for (A, B) where A: StatParam<Q, D>, B: StatParam<Q, D> {
    fn stream<S: Stat, E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        write: &mut S::Data,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        querier: &mut impl StatQuerier<Q, D>,
    ) {
        A::stream(&this.0, entities.clone(), write, qualifier, stat, querier);
        B::stream(&this.1, entities, write, qualifier, stat, querier);
    }
}
