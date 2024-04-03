use std::{borrow::Borrow, marker::PhantomData};
use bevy_ecs::{entity::Entity, system::{Query, ReadOnlySystemParam, StaticSystemParam, SystemParam}};
use crate::{querier::QuerierRef, stream::ExternalStream, IntrinsicStream, QualifierFlag, QualifierQuery, StatValuePair};

/// [`SystemParam`] that can be aggregated as stat components.
pub trait StatParam<Q: QualifierFlag>: ReadOnlySystemParam {
    fn stream<E: Borrow<Entity>>(
        this: &Self::Item<'_, '_>,
        entities: impl IntoIterator<Item = E> + Clone,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    );
}

/// [`SystemParam`] that can be used to query relation.
pub trait IntrinsicParam<Q: QualifierFlags>: StatParam<Q> {
    /// Returns false if either entity is missing.
    fn distance_stream (
        item: &Self::Item<'_, '_>,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    );
}

/// [`SystemParam`] that queries for a specific [`ExternalStream`] in an entity.
#[derive(SystemParam)]
pub struct ChildStatParam<'w, 's, T: ExternalStream<Q>, Q: QualifierFlags> {
    pub ctx: StaticSystemParam<'w, 's, <T as ExternalStream<Q>>::Ctx>,
    pub query: Query<'w, 's, <T as ExternalStream<Q>>::QueryData>,
    p: PhantomData<Q>,
}

impl<T: ExternalStream<Q>, Q: QualifierFlags> StatParam<Q> for ChildStatParam<'_, '_, T, Q> {
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

impl<T: IntrinsicStream<Q>, Q: QualifierFlags> IntrinsicParam<Q> for ChildStatParam<'_, '_, T, Q> {
    fn distance_stream (
        item: &Self::Item<'_, '_>,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    ) {
        if let Ok((a, b)) = item.query.get(this).and_then(|x| Ok((x, item.query.get(other)?))){
            T::distance(&*item.ctx, a, b, qualifier, stat, querier);
        }
    }
}

impl<Q: QualifierFlags> StatParam<Q> for () {
    fn stream<E: Borrow<Entity>>(
        _: &Self::Item<'_, '_>,
        _: impl IntoIterator<Item = E> + Clone,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) {}
}

impl<Q: QualifierFlags> IntrinsicParam<Q> for () {
    fn distance_stream (
        _: &Self::Item<'_, '_>,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: &mut QuerierRef<'_, Q>,
    ) {}
}

impl<A, B, Q: QualifierFlags> StatParam<Q> for (A, B) where A: StatParam<Q>, B: StatParam<Q> {
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


impl<A, B, Q: QualifierFlags> IntrinsicParam<Q> for (A, B) where A: IntrinsicParam<Q>, B: IntrinsicParam<Q> {
    fn distance_stream (
        item: &Self::Item<'_, '_>,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &mut StatValuePair,
        querier: &mut QuerierRef<'_, Q>,
    ) {
        A::distance_stream(&item.0, this, other, qualifier, stat, querier);
        B::distance_stream(&item.1, this, other, qualifier, stat, querier);
    }
}
