use std::{borrow::Borrow, marker::PhantomData};
use bevy_ecs::{entity::Entity, query::Without, system::{Query, StaticSystemParam, SystemParam}};
use crate::{querier::QuerierRef, stream::ComponentStream, QualifierFlag, QualifierQuery, StatCache, StatValuePair};

/// [`SystemParam`] that can be aggregated as stat components.
pub trait StatParam<Q: QualifierFlag>: SystemParam {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    );
}

pub trait IntrinsicParam<Q: QualifierFlag>: StatParam<Q> {
    /// Returns false if either entity is missing.
    fn distance_stream (
        item: &Self::Item<'_, '_>,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    ) -> bool;
}


/// [`SystemParam`] that queries for a specific [`StatStream`] in an entity.
#[derive(SystemParam)]
pub struct ChildStatParam<'w, 's, T: ComponentStream<Q>, Q: QualifierFlag> {
    pub ctx: StaticSystemParam<'w, 's, <T as ComponentStream<Q>>::Ctx>,
    pub query: Query<'w, 's, <T as ComponentStream<Q>>::QueryData, Without<StatCache<Q>>>,
    p: PhantomData<Q>,
}

impl<T: ComponentStream<Q>, Q: QualifierFlag> StatParam<Q> for ChildStatParam<'_, '_, T, Q> {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    ) {
        for handle in this.query.iter_many(entities) {
            T::stream(&*this.ctx, handle, qualifier, stat, querier);
        }
    }
}

impl<Q: QualifierFlag> StatParam<Q> for () {
    fn stream<E: Borrow<Entity>>(
        _: &Self::Item<'_, '_>,
        _: impl IntoIterator<Item = E> + Clone,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) {}
}

impl<Q: QualifierFlag> IntrinsicParam<Q> for () {
    fn distance_stream (
        _: &Self::Item<'_, '_>,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) -> bool {
        true
    }
}

impl<A, B, Q: QualifierFlag> StatParam<Q> for (A, B) where A: StatParam<Q>, B: StatParam<Q> {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    ) {
        A::stream(&this.0, entities.clone(), qualifier, stat, querier);
        B::stream(&this.1, entities, qualifier, stat, querier);
    }
}
