// use crate::{
//     querier::QuerierRef, stream::StreamQuery, RelationStream, QualifierFlag, QualifierQuery, Stat, StatValuePair
// };
// use bevy_ecs::{
//     entity::Entity,
//     system::{Query, ReadOnlySystemParam, StaticSystemParam, SystemParam},
// };
// use std::{borrow::Borrow, marker::PhantomData};

// /// [`SystemParam`] that can be aggregated as stat components.
// pub trait StatParam<Q: QualifierFlag>: ReadOnlySystemParam {
//     fn stream<E: Borrow<Entity>, S: Stat>(
//         this: &Self::Item<'_, '_>,
//         entities: impl IntoIterator<Item = E> + Clone,
//         qualifier: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         querier: &mut QuerierRef<'_, Q>,
//     );
// }

// /// [`SystemParam`] that can be used to query relation.
// pub trait IntrinsicParam<Q: QualifierFlag>: StatParam<Q> {
//     /// Returns false if either entity is missing.
//     fn distance_stream<S: Stat>(
//         item: &Self::Item<'_, '_>,
//         this: Entity,
//         other: Entity,
//         qualifier: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         querier: &mut QuerierRef<'_, Q>,
//     );
// }

// /// [`SystemParam`] that queries for a specific [`StatStream`] in an entity.
// #[derive(SystemParam)]
// pub struct ChildStatParam<'w, 's, T: StreamQuery<Q>, Q: QualifierFlag> {
//     pub ctx: StaticSystemParam<'w, 's, <T as StreamQuery<Q>>::Ctx>,
//     pub query: Query<'w, 's, <T as StreamQuery<Q>>::QueryData>,
//     p: PhantomData<Q>,
// }

// impl<T: StreamQuery<Q>, Q: QualifierFlag> StatParam<Q> for ChildStatParam<'_, '_, T, Q> {
//     fn stream<E: Borrow<Entity>, S: Stat>(
//         this: &Self::Item<'_, '_>,
//         entities: impl IntoIterator<Item = E> + Clone,
//         qualifier: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         querier: &mut QuerierRef<'_, Q>,
//     ) {
//         for handle in this.query.iter_many(entities) {
//             T::stream(&*this.ctx, handle, qualifier, stat, value, querier);
//         }
//     }
// }

// impl<T: RelationStream<Q>, Q: QualifierFlag> IntrinsicParam<Q> for ChildStatParam<'_, '_, T, Q> {
//     fn distance_stream<S: Stat>(
//         item: &Self::Item<'_, '_>,
//         this: Entity,
//         other: Entity,
//         qualifier: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         querier: &mut QuerierRef<'_, Q>,
//     ) {
//         if let Ok((a, b)) = item
//             .query
//             .get(this)
//             .and_then(|x| Ok((x, item.query.get(other)?)))
//         {
//             T::relation(&*item.ctx, a, b, qualifier, stat, value, querier);
//         }
//     }
// }

// impl<Q: QualifierFlag> StatParam<Q> for () {
//     fn stream<E: Borrow<Entity>, S: Stat>(
//         _: &Self::Item<'_, '_>,
//         _: impl IntoIterator<Item = E> + Clone,
//         _: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         _: &mut QuerierRef<'_, Q>,
//     ) {
//     }
// }

// impl<Q: QualifierFlag> IntrinsicParam<Q> for () {
//     fn distance_stream<S: Stat>(
//         _: &Self::Item<'_, '_>,
//         _: Entity,
//         _: Entity,
//         _: &QualifierQuery<Q>,
//         _: &S,
//         _: &mut S::Data,
//         _: &mut QuerierRef<'_, Q>,
//     ) {
//     }
// }

// impl<A, B, Q: QualifierFlag> StatParam<Q> for (A, B)
// where
//     A: StatParam<Q>,
//     B: StatParam<Q>,
// {
//     fn stream<E: Borrow<Entity>, S: Stat>(
//         this: &Self::Item<'_, '_>,
//         entities: impl IntoIterator<Item = E> + Clone,
//         qualifier: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         querier: &mut QuerierRef<'_, Q>,
//     ) {
//         A::stream(&this.0, entities.clone(), qualifier, stat, value, querier);
//         B::stream(&this.1, entities, qualifier, stat, value, querier);
//     }
// }

// impl<A, B, Q: QualifierFlag> IntrinsicParam<Q> for (A, B)
// where
//     A: IntrinsicParam<Q>,
//     B: IntrinsicParam<Q>,
// {
//     fn distance_stream<S: Stat>(
//         item: &Self::Item<'_, '_>,
//         this: Entity,
//         other: Entity,
//         qualifier: &QualifierQuery<Q>,
//         stat: &S,
//         value: &mut S::Data,
//         querier: &mut QuerierRef<'_, Q>,
//     ) {
//         A::distance_stream(&item.0, this, other, qualifier, stat, value, querier);
//         B::distance_stream(&item.1, this, other, qualifier, stat, value, querier);
//     }
// }
