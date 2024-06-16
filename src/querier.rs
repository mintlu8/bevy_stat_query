use crate::{
    plugin::StatDefaults, ComponentStream, CxComponentStream, QualifierFlag, QualifierQuery,
    QueryRelationStream, QueryStream, RelationStream, Stat, StatCache, StatValue,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryFilter, With},
    system::{Query, Res, SystemParam},
};
use bevy_hierarchy::Children;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

/// The core marker component. Stat querying is only allowed on entities marked as [`StatEntity`].
#[derive(Debug, Component, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct StatEntity;

/// A root [`SystemParam`] that curates all entities marked as [`StatEntity`].
///
/// Add queries via methods like [`StatQuery::with_component`] to start querying for stats.
#[derive(Debug, SystemParam)]
pub struct StatQuery<'w, 's, Q: QualifierFlag> {
    cache: Option<Res<'w, StatCache<Q>>>,
    defaults: Option<Res<'w, StatDefaults>>,
    entities: Query<'w, 's, Option<&'static Children>, With<StatEntity>>,
}

/// [`StatQuery`] with appended queries.
pub struct JoinedQuerier<
    't,
    'w,
    's,
    Q: QualifierFlag,
    A: QueryStream<Q>,
    B: QueryStream<Q>,
    C: QueryRelationStream<Q>,
> {
    querier: &'t StatQuery<'w, 's, Q>,
    component_streams: A,
    children_streams: B,
    relationship_streams: C,
}

/// An erased type that can query for stats on entities in the world.
///
/// Notable implementors are [`NoopQuerier`] and [`JoinedQuerier`].
pub trait Querier<Q: QualifierFlag> {
    fn query_stat<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value>;

    fn query_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value>;

    fn query_eval<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        self.query_stat(entity, query, stat)
            .map(|x| StatValue::eval(&x))
    }

    fn query_relation_eval<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<<S::Value as StatValue>::Out> {
        self.query_relation(from, to, query, stat)
            .map(|x| StatValue::eval(&x))
    }
}

/// A [`Querier`] that does not provide the ability to query other entities.
pub struct NoopQuerier;

impl<Q: QualifierFlag> Querier<Q> for NoopQuerier {
    fn query_stat<S: Stat>(&self, _: Entity, _: &QualifierQuery<Q>, _: &S) -> Option<S::Value> {
        None
    }

    fn query_relation<S: Stat>(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &S,
    ) -> Option<S::Value> {
        None
    }
}

impl<Q: QualifierFlag, A: QueryStream<Q>, B: QueryStream<Q>, C: QueryRelationStream<Q>> Querier<Q>
    for JoinedQuerier<'_, '_, '_, Q, A, B, C>
{
    fn query_stat<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value> {
        if !self.querier.entities.contains(entity) {
            return None;
        }
        if let Some(cached) = self
            .querier
            .cache
            .as_ref()
            .and_then(|c| c.try_get_cached(entity, query, stat))
        {
            return Some(cached);
        }
        let mut result = self
            .querier
            .defaults
            .as_ref()
            .map(|d| d.get(stat))
            .unwrap_or_default();
        self.component_streams
            .stream(entity, &[entity], query, stat, &mut result, self);
        self.relationship_streams
            .stream(entity, &[entity], query, stat, &mut result, self);
        if let Ok(Some(children)) = self.querier.entities.get(entity) {
            self.children_streams
                .stream(entity, children.as_ref(), query, stat, &mut result, self);
        }
        if let Some(cache) = self.querier.cache.as_ref() {
            cache.cache(entity, query.clone(), stat, result.clone())
        }
        Some(result)
    }

    fn query_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Value> {
        if !self.querier.entities.contains(from) || !self.querier.entities.contains(to) {
            return None;
        }
        let mut result = self
            .querier
            .defaults
            .as_ref()
            .map(|d| d.get(stat))
            .unwrap_or_default();
        self.relationship_streams
            .relation(from, to, query, stat, &mut result, self);
        Some(result)
    }
}

impl<'w, 's, Q: QualifierFlag> StatQuery<'w, 's, Q> {
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.clear()
        }
    }

    pub fn with_component<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<'w, 's, D, F>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, impl QueryStream<Q> + 't, (), ()>
    where
        D: ComponentStream<Q, Cx = ()>,
    {
        JoinedQuerier {
            querier: self,
            component_streams: CxComponentStream { cx: &(), query },
            children_streams: (),
            relationship_streams: (),
        }
    }

    pub fn with_children<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<'w, 's, D, F>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, (), impl QueryStream<Q> + 't, ()>
    where
        D: ComponentStream<Q, Cx = ()>,
    {
        JoinedQuerier {
            querier: self,
            component_streams: (),
            children_streams: CxComponentStream { cx: &(), query },
            relationship_streams: (),
        }
    }

    pub fn with_relation<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<'w, 's, D, F>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, (), (), impl QueryRelationStream<Q> + 't>
    where
        D: RelationStream<Q, Cx = ()>,
    {
        JoinedQuerier {
            querier: self,
            component_streams: (),
            children_streams: (),
            relationship_streams: CxComponentStream { cx: &(), query },
        }
    }

    pub fn with_component_cx<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<'w, 's, D, F>,
        cx: &'t <D::Cx as SystemParam>::Item<'w, 's>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, impl QueryStream<Q> + 't, (), ()>
    where
        D: ComponentStream<Q>,
        't: 'w,
        't: 's,
    {
        JoinedQuerier {
            querier: self,
            component_streams: CxComponentStream { cx, query },
            children_streams: (),
            relationship_streams: (),
        }
    }

    pub fn with_children_cx<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<'w, 's, D, F>,
        cx: &'t <D::Cx as SystemParam>::Item<'w, 's>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, (), impl QueryStream<Q> + 't, ()>
    where
        D: ComponentStream<Q>,
        't: 'w,
        't: 's,
    {
        JoinedQuerier {
            querier: self,
            component_streams: (),
            children_streams: CxComponentStream { cx, query },
            relationship_streams: (),
        }
    }

    pub fn with_relation_cx<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<'w, 's, D, F>,
        cx: &'t <D::Cx as SystemParam>::Item<'w, 's>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, (), (), impl QueryRelationStream<Q> + 't>
    where
        D: RelationStream<Q>,
        't: 'w,
        't: 's,
    {
        JoinedQuerier {
            querier: self,
            component_streams: (),
            children_streams: (),
            relationship_streams: CxComponentStream { cx, query },
        }
    }
}

impl<
        'u,
        'w,
        's,
        Q: QualifierFlag,
        A: QueryStream<Q>,
        B: QueryStream<Q>,
        C: QueryRelationStream<Q>,
    > JoinedQuerier<'u, 'w, 's, Q, A, B, C>
{
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.querier.cache {
            cache.clear()
        }
    }

    pub fn with_component<'t, D, F: QueryFilter>(
        self,
        query: &'t Query<D, F>,
    ) -> JoinedQuerier<'u, 'w, 's, Q, impl QueryStream<Q> + 't, B, C>
    where
        D: ComponentStream<Q, Cx = ()>,
        A: 't,
    {
        JoinedQuerier {
            querier: self.querier,
            component_streams: (self.component_streams, CxComponentStream { cx: &(), query }),
            children_streams: self.children_streams,
            relationship_streams: self.relationship_streams,
        }
    }

    pub fn with_children<'t, D, F: QueryFilter>(
        self,
        query: &'t Query<D, F>,
    ) -> JoinedQuerier<'u, 'w, 's, Q, A, impl QueryStream<Q> + 't, C>
    where
        D: ComponentStream<Q, Cx = ()>,
        B: 't,
    {
        JoinedQuerier {
            querier: self.querier,
            component_streams: self.component_streams,
            children_streams: (self.children_streams, CxComponentStream { cx: &(), query }),
            relationship_streams: self.relationship_streams,
        }
    }

    pub fn with_relation<'t, D, F: QueryFilter>(
        self,
        query: &'t Query<D, F>,
    ) -> JoinedQuerier<'u, 'w, 's, Q, A, B, impl QueryRelationStream<Q> + 't>
    where
        D: RelationStream<Q, Cx = ()>,
        C: 't,
    {
        JoinedQuerier {
            querier: self.querier,
            component_streams: self.component_streams,
            children_streams: self.children_streams,
            relationship_streams: (
                self.relationship_streams,
                CxComponentStream { cx: &(), query },
            ),
        }
    }

    pub fn with_component_cx<'t, D, F: QueryFilter>(
        self,
        query: &'t Query<'w, 's, D, F>,
        cx: &'t <D::Cx as SystemParam>::Item<'w, 's>,
    ) -> JoinedQuerier<'u, 'w, 's, Q, impl QueryStream<Q> + 't, B, C>
    where
        D: ComponentStream<Q>,
        A: 't,
        't: 'w,
        't: 's,
    {
        JoinedQuerier {
            querier: self.querier,
            component_streams: (self.component_streams, CxComponentStream { cx, query }),
            children_streams: self.children_streams,
            relationship_streams: self.relationship_streams,
        }
    }

    pub fn with_children_cx<'t, D, F: QueryFilter>(
        self,
        query: &'t Query<'w, 's, D, F>,
        cx: &'t <D::Cx as SystemParam>::Item<'w, 's>,
    ) -> JoinedQuerier<'u, 'w, 's, Q, A, impl QueryStream<Q> + 't, C>
    where
        D: ComponentStream<Q>,
        B: 't,
        't: 'w,
        't: 's,
    {
        JoinedQuerier {
            querier: self.querier,
            component_streams: self.component_streams,
            children_streams: (self.children_streams, CxComponentStream { cx, query }),
            relationship_streams: self.relationship_streams,
        }
    }

    pub fn with_relation_cx<'t, D, F: QueryFilter>(
        self,
        query: &'t Query<'w, 's, D, F>,
        cx: &'t <D::Cx as SystemParam>::Item<'w, 's>,
    ) -> JoinedQuerier<'u, 'w, 's, Q, A, B, impl QueryRelationStream<Q> + 't>
    where
        D: RelationStream<Q>,
        C: 't,
        't: 'w,
        't: 's,
    {
        JoinedQuerier {
            querier: self.querier,
            component_streams: self.component_streams,
            children_streams: self.children_streams,
            relationship_streams: (self.relationship_streams, CxComponentStream { cx, query }),
        }
    }
}
