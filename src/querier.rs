use std::fmt::Debug;

use crate::plugin::GlobalStatRelations;
use crate::stat::StatExt;
use crate::{
    plugin::GlobalStatDefaults, Buffer, QualifierFlag, QualifierQuery, Stat, StatCache, StatInst,
    StatStream,
};
use crate::{validate, StatValue, StatValuePair};
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Query, Res, SystemParam},
};
use bevy_hierarchy::Children;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

/// The core marker component. Stat querying is only allowed on entities marked as [`StatEntity`].
#[derive(Debug, Component, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct StatEntity;

/// A root [`SystemParam`] that curates all entities marked as [`StatEntity`].
///
/// Add queries via methods like [`StatQuery::with_component`] to start querying for stats.
#[derive(Debug, SystemParam)]
pub struct StatEntities<'w, 's, Q: QualifierFlag> {
    cache: Option<Res<'w, StatCache<Q>>>,
    defaults: Option<Res<'w, GlobalStatDefaults>>,
    relations: Option<Res<'w, GlobalStatRelations<Q>>>,
    entities: Query<'w, 's, Option<&'static Children>, With<StatEntity>>,
}

impl<'w, 's, Q: QualifierFlag> StatEntities<'w, 's, Q> {
    pub fn join<'t, S: StatStream<Qualifier = Q>>(
        &'t self,
        stream: S,
    ) -> JoinedQuerier<'w, 's, 't, Q, S> {
        JoinedQuerier {
            entities: self,
            stream,
        }
    }

    pub fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.clear();
        }
    }
}

pub struct JoinedQuerier<'w, 's, 't, Q: QualifierFlag, S: StatStream<Qualifier = Q>> {
    entities: &'t StatEntities<'w, 's, Q>,
    stream: S,
}

impl<'w, 's, 't, Q: QualifierFlag, S: StatStream<Qualifier = Q>> JoinedQuerier<'w, 's, 't, Q, S> {
    pub fn join<T: StatStream<Qualifier = Q>>(
        &self,
        stream: T,
    ) -> JoinedQuerier<'w, 's, 't, Q, (&S, T)> {
        JoinedQuerier {
            entities: self.entities,
            stream: (&self.stream, stream),
        }
    }

    pub fn query_stat<T: Stat>(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &T,
    ) -> Option<T::Value> {
        self.query_stat_erased(entity, qualifier, stat.as_entry())
            .map(|x| unsafe { x.into() })
    }

    pub fn query_relation<T: Stat>(
        &self,
        from: Entity,
        to: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &T,
    ) -> Option<T::Value> {
        self.query_relation_erased(from, to, qualifier, stat.as_entry())
            .map(|x| unsafe { x.into() })
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

    pub fn has_attribute(&self, entity: Entity, attribute: &str) -> bool {
        self.has_attribute_erased(entity, attribute)
    }

    pub fn clear_cache(&self) {
        self.entities.clear_cache()
    }
}

impl<Q: QualifierFlag, S: StatStream<Qualifier = Q>> ErasedQuerier<Q>
    for JoinedQuerier<'_, '_, '_, Q, S>
{
    fn query_stat_erased(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: StatInst,
    ) -> Option<Buffer> {
        let value = if let Some(defaults) = &self.entities.defaults {
            defaults.get_dyn(stat)
        } else {
            (stat.vtable.default)()
        };
        let mut pair = StatValuePair { stat, value };
        self.stream
            .stream_stat(entity, query, &mut pair, Querier(self));
        Some(pair.value)
    }

    fn query_relation_erased(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: StatInst,
    ) -> Option<Buffer> {
        let value = if let Some(defaults) = &self.entities.defaults {
            defaults.get_dyn(stat)
        } else {
            (stat.vtable.default)()
        };
        let mut pair = StatValuePair { stat, value };
        self.stream
            .stream_relation(&self.stream, from, to, query, &mut pair, Querier(self));
        Some(pair.value)
    }

    fn has_attribute_erased(&self, entity: Entity, attribute: &str) -> bool {
        self.stream.has_attribute(entity, attribute)
    }
}

/// An erased type that can query for stats on entities in the world.
///
/// Notable implementors are [`NoopQuerier`] and [`JoinedQuerier`].
trait ErasedQuerier<Q: QualifierFlag> {
    /// Query for a stat in its component form.
    fn query_stat_erased(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: StatInst,
    ) -> Option<Buffer>;

    /// Query for a relation stat in its component form.
    fn query_relation_erased(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: StatInst,
    ) -> Option<Buffer>;

    /// Query for the existence of a string attribute.
    fn has_attribute_erased(&self, entity: Entity, attribute: &str) -> bool;
}

/// An erased type that can query for stats on entities in the world.
pub struct Querier<'t, Q: QualifierFlag>(&'t dyn ErasedQuerier<Q>);

impl<Q: QualifierFlag> Clone for Querier<'_, Q> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Q: QualifierFlag> Copy for Querier<'_, Q> {}

impl<Q: QualifierFlag> Debug for Querier<'_, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Querier").finish_non_exhaustive()
    }
}

impl<Q: QualifierFlag> Querier<'_, Q> {
    /// Create a noop querier.
    pub fn noop(querier: &NoopQuerier) -> Querier<Q> {
        Querier(querier)
    }

    /// Query for a stat in its component form.
    pub fn query_stat<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value> {
        validate::<S::Value>();
        self.0
            .query_stat_erased(entity, query, stat.as_entry())
            .map(|x| unsafe { x.into() })
    }

    /// Query for a relation stat in its component form.
    pub fn query_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value> {
        validate::<S::Value>();
        self.0
            .query_relation_erased(from, to, query, stat.as_entry())
            .map(|x| unsafe { x.into() })
    }

    /// Query for a stat in its evaluated form.
    pub fn query_eval<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        validate::<S::Value>();
        self.query_stat(entity, query, stat)
            .map(|x| StatValue::eval(&x))
    }

    /// Query for a relation stat in its evaluated form.
    pub fn query_relation_eval<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        validate::<S::Value>();
        self.query_relation(from, to, query, stat)
            .map(|x| StatValue::eval(&x))
    }

    /// Query for the existence of a string attribute.
    pub fn has_attribute(&self, entity: Entity, attribute: &str) -> bool {
        self.0.has_attribute_erased(entity, attribute)
    }
}

/// A [`Querier`] that does not provide the ability to query other entities.
pub struct NoopQuerier;

impl<Q: QualifierFlag> ErasedQuerier<Q> for NoopQuerier {
    fn query_relation_erased(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: StatInst,
    ) -> Option<Buffer> {
        None
    }

    fn query_stat_erased(&self, _: Entity, _: &QualifierQuery<Q>, _: StatInst) -> Option<Buffer> {
        None
    }

    fn has_attribute_erased(&self, _: Entity, _: &str) -> bool {
        false
    }
}
