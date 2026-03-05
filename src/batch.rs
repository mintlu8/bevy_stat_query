use std::fmt::Debug;

use bevy_ecs::entity::Entity;
use ref_cast::RefCast;

use crate::{ErasedQuerier, Qualifier, QualifierQuery, Stat, StatValue};

/// Query for many stats at once, implemented on tuples.
///
/// [`Eval(MyStat)`](Eval) can be used to obtain evaluated results immediately.
pub trait StatQueryBatch: Debug + 'static {
    type Output: Default;

    #[doc(hidden)]
    fn query<Q: Qualifier>(
        &self,
        querier: &impl ErasedQuerier<Q>,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
    ) -> Option<Self::Output>;
}

/// Evaluate stat in [`StatQueryBatch`].
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Eval<T>(pub T);

impl<T> Eval<T> {
    pub fn from_ref(r: &T) -> &Self {
        Self::ref_cast(r)
    }
}

impl<T: Stat> StatQueryBatch for T {
    type Output = T::Value;

    fn query<Q: Qualifier>(
        &self,
        querier: &impl ErasedQuerier<Q>,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
    ) -> Option<Self::Output> {
        querier
            .query_stat_erased(entity, qualifier, self)
            .and_then(|x| x.unbox())
    }
}

impl<T: Stat> StatQueryBatch for Eval<T> {
    type Output = <T::Value as StatValue>::Out;

    fn query<Q: Qualifier>(
        &self,
        querier: &impl ErasedQuerier<Q>,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
    ) -> Option<Self::Output> {
        querier
            .query_stat_erased(entity, qualifier, &self.0)
            .and_then(|x| x.unbox::<T::Value>())
            .map(|x| x.eval())
    }
}

macro_rules! impl_batch {
    ($($T: ident)*) => {
        impl<$($T: StatQueryBatch,)*> StatQueryBatch for ($($T,)*) {
            type Output = ($($T::Output,)*);

            #[allow(non_snake_case)]
            fn query<Q: Qualifier>(&self, querier: &impl ErasedQuerier<Q>, entity: Entity, qualifier: &QualifierQuery<Q>) -> Option<Self::Output> {
                let ($($T,)*) = self;
                Some(($($T::query($T, querier, entity, qualifier)?,)*))
            }
        }
    };
}

impl_batch!(T0);
impl_batch!(T0 T1);
impl_batch!(T0 T1 T2);
impl_batch!(T0 T1 T2 T3);
impl_batch!(T0 T1 T2 T3 T4);
impl_batch!(T0 T1 T2 T3 T4 T5);
impl_batch!(T0 T1 T2 T3 T4 T5 T6);
impl_batch!(T0 T1 T2 T3 T4 T5 T6 T7);
impl_batch!(T0 T1 T2 T3 T4 T5 T6 T7 T8);
impl_batch!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9);
impl_batch!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10);
impl_batch!(T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11);
