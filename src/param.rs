use std::{borrow::Borrow, marker::PhantomData};
use bevy_ecs::{entity::Entity, query::Without, system::{Query, StaticSystemParam, SystemParam}};
use crate::{stream::{StatQuerier, StatStream}, QualifierFlag, QualifierQuery, StatCache, StatValuePair};

/// [`SystemParam`] that can be aggregated as stat components.
pub trait StatParam<Q: QualifierFlag>: SystemParam {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut dyn StatValuePair,
        querier: &mut impl StatQuerier<Q>,
    );
}

/// [`SystemParam`] that queries for a specific [`StatStream`] in an entity.
#[derive(SystemParam)]
pub struct ChildStatParam<'w, 's, T: StatStream<Q>, Q: QualifierFlag> {
    pub ctx: StaticSystemParam<'w, 's, <T as StatStream<Q>>::Ctx>,
    pub query: Query<'w, 's, <T as StatStream<Q>>::QueryData, Without<StatCache<Q>>>,
    p: PhantomData<Q>,
}

impl<T: StatStream<Q>, Q: QualifierFlag> StatParam<Q> for ChildStatParam<'_, '_, T, Q> {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut dyn StatValuePair,
        querier: &mut impl StatQuerier<Q>,
    ) {
        for handle in this.query.iter_many(entities) {
            T::stream(&*this.ctx, handle, qualifier, stat, querier);
        }
    }
}

impl<Q: QualifierFlag> StatParam<Q> for () {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut dyn StatValuePair,
        querier: &mut impl StatQuerier<Q>,
    ) {}
}

impl<A, B, Q: QualifierFlag> StatParam<Q> for (A, B) where A: StatParam<Q>, B: StatParam<Q> {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut dyn StatValuePair,
        querier: &mut impl StatQuerier<Q>,
    ) {
        A::stream(&this.0, entities.clone(), qualifier, stat, querier);
        B::stream(&this.1, entities, qualifier, stat, querier);
    }
}
