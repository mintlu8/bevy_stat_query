use std::fmt::Debug;

use crate::attribute::Attribute;
use crate::plugin::GlobalStatRelations;
use crate::stat::ErasedStat;
use crate::{Qualifier, QualifierQuery, Stat, StatStream, plugin::GlobalStatDefaults};
use crate::{ShareableAny, StatValue, StatValuePair};
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Query, Res, SystemParam},
};
use bevy_reflect::Reflect;

/// The core marker component. Stat querying is only allowed on entities marked as [`StatEntity`].
#[derive(Debug, Component, Clone, PartialEq, Eq, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
pub struct StatEntity;

/// A root [`SystemParam`] that curates all entities marked as [`StatEntity`].
///
/// Join with [`StatStream`]s via [`StatEntities::join`] to start querying.
#[derive(Debug, SystemParam)]
pub struct StatEntities<'w, 's, Q: Qualifier> {
    defaults: Option<Res<'w, GlobalStatDefaults>>,
    relations: Option<Res<'w, GlobalStatRelations<Q>>>,
    entities: Query<'w, 's, Entity, With<StatEntity>>,
}

impl<'w, 's, Q: Qualifier> StatEntities<'w, 's, Q> {
    pub fn join<'t, S: StatStream<Qualifier = Q>>(
        &'t self,
        stream: S,
    ) -> JoinedQuerier<'w, 's, 't, Q, S> {
        JoinedQuerier { base: self, stream }
    }
}

/// [`StatEntities`] joined with multiple [`StatStream`]s.
pub struct JoinedQuerier<'w, 's, 't, Q: Qualifier, S: StatStream<Qualifier = Q>> {
    base: &'t StatEntities<'w, 's, Q>,
    stream: S,
}

impl<'w, 's, 't, Q: Qualifier, S: StatStream<Qualifier = Q>> JoinedQuerier<'w, 's, 't, Q, S> {
    pub fn join<T: StatStream<Qualifier = Q>>(
        self,
        stream: T,
    ) -> JoinedQuerier<'w, 's, 't, Q, (S, T)> {
        JoinedQuerier {
            base: self.base,
            stream: (self.stream, stream),
        }
    }

    pub fn query_stat<T: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &T,
    ) -> Option<T::Value> {
        self.query_stat_erased(entity, qualifier, stat)
            .and_then(|x| x.unbox())
    }

    pub fn query_relation<T: Stat>(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &T,
    ) -> Option<T::Value> {
        self.query_relation_erased(from, to, qualifier, stat)
            .and_then(|x| x.unbox())
    }

    pub fn eval_stat<T: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &T,
    ) -> Option<<T::Value as StatValue>::Out> {
        self.query_stat(entity, qualifier, stat).map(|x| x.eval())
    }

    pub fn eval_relation<T: Stat>(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &T,
    ) -> Option<<T::Value as StatValue>::Out> {
        self.query_relation(from, to, qualifier, stat)
            .map(|x| x.eval())
    }

    pub fn has_attribute<'a>(&self, entity: Entity, attribute: impl Into<Attribute<'a>>) -> bool {
        self.has_attribute_erased(entity, attribute.into())
    }
}

impl<Q: Qualifier, S: StatStream<Qualifier = Q>> ErasedQuerier<Q>
    for JoinedQuerier<'_, '_, '_, Q, S>
{
    fn query_stat_erased(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &dyn ErasedStat,
    ) -> Option<Box<dyn ShareableAny>> {
        let mut value = stat.create_default_value(self.base.defaults.as_deref());
        let mut pair = StatValuePair::new_dyn(stat, &mut *value);
        if let Some(relations) = &self.base.relations {
            relations.stream_stat(entity, query, &mut pair, Querier(self));
        }
        self.stream
            .stream_stat(entity, query, &mut pair, Querier(self));
        Some(value)
    }

    fn query_relation_erased(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &dyn ErasedStat,
    ) -> Option<Box<dyn ShareableAny>> {
        let mut value = stat.create_default_value(self.base.defaults.as_deref());
        let mut pair = StatValuePair::new_dyn(stat, &mut *value);
        self.stream
            .stream_relation(&self.stream, from, to, query, &mut pair, Querier(self));
        Some(value)
    }

    fn has_attribute_erased(&self, entity: Entity, attribute: Attribute) -> bool {
        self.stream.has_attribute(entity, attribute)
    }
}

/// An erased type that can query for stats on entities in the world.
///
/// Notable implementors are [`NoopQuerier`] and [`JoinedQuerier`].
trait ErasedQuerier<Q: Qualifier> {
    /// Query for a stat in its component form.
    fn query_stat_erased(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &dyn ErasedStat,
    ) -> Option<Box<dyn ShareableAny>>;

    /// Query for a relation stat in its component form.
    fn query_relation_erased(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &dyn ErasedStat,
    ) -> Option<Box<dyn ShareableAny>>;

    /// Query for the existence of a string attribute.
    fn has_attribute_erased(&self, entity: Entity, attribute: Attribute) -> bool;
}

/// An erased type that can query for stats on entities in the world.
pub struct Querier<'t, Q: Qualifier>(&'t dyn ErasedQuerier<Q>);

impl<Q: Qualifier> Clone for Querier<'_, Q> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Q: Qualifier> Copy for Querier<'_, Q> {}

impl<Q: Qualifier> Debug for Querier<'_, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier").finish_non_exhaustive()
    }
}

impl<Q: Qualifier> Querier<'_, Q> {
    /// Create a noop querier.
    pub fn noop() -> Querier<'static, Q> {
        static _Q: NoopQuerier = NoopQuerier;
        Querier(&_Q)
    }

    /// Query for a stat in its component form.
    pub fn query_stat<S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value> {
        self.0
            .query_stat_erased(entity, qualifier, stat)
            .and_then(|x| x.unbox())
    }

    /// Query for a relation stat in its component form.
    pub fn query_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value> {
        self.0
            .query_relation_erased(from, to, qualifier, stat)
            .and_then(|x| x.unbox())
    }

    /// Query for a stat in its evaluated form.
    pub fn eval_stat<S: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        self.query_stat(entity, qualifier, stat)
            .map(|x| StatValue::eval(&x))
    }

    /// Query for a relation stat in its evaluated form.
    pub fn eval_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        self.query_relation(from, to, qualifier, stat)
            .map(|x| StatValue::eval(&x))
    }

    /// Query for the existence of an attribute.
    pub fn has_attribute<'a>(&self, entity: Entity, attribute: impl Into<Attribute<'a>>) -> bool {
        self.0.has_attribute_erased(entity, attribute.into())
    }
}

/// A [`Querier`] that does not provide the ability to query other entities.
pub struct NoopQuerier;

impl<Q: Qualifier> ErasedQuerier<Q> for NoopQuerier {
    fn query_relation_erased(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &dyn ErasedStat,
    ) -> Option<Box<dyn ShareableAny>> {
        None
    }

    fn query_stat_erased(
        &self,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &dyn ErasedStat,
    ) -> Option<Box<dyn ShareableAny>> {
        None
    }

    fn has_attribute_erased(&self, _: Entity, _: Attribute) -> bool {
        false
    }
}
