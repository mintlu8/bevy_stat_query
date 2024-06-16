use crate::{QualifierFlag, QualifierQuery, Stat, StatCache, StatDefaults, StatEntity};
use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, With, WorldQuery},
    system::{Query, ReadOnlySystemParam, Res, SystemParam},
};
use bevy_hierarchy::Children;

pub trait StatStream<Q: QualifierFlag> {
    fn stream<S: Stat>(
        &self,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    );
}

pub trait QueryStream<Q: QualifierFlag> {
    fn stream<S: Stat>(
        &self,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    );
}

impl<Q: QualifierFlag> QueryStream<Q> for () {
    fn stream<S: Stat>(
        &self,
        _: &[Entity],
        _: &QualifierQuery<Q>,
        _: &S,
        _: &mut S::Data,
        _: &impl EraseQuerier<Q>,
    ) {
    }
}

impl<Q: QualifierFlag, A: QueryStream<Q>, B: QueryStream<Q>> QueryStream<Q> for (A, B) {
    fn stream<S: Stat>(
        &self,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    ) {
        self.0.stream(entities, qualifier, stat, value, querier);
        self.1.stream(entities, qualifier, stat, value, querier);
    }
}

impl<Q: QualifierFlag> QueryRelationStream<Q> for () {
    fn relation<S: Stat>(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &S,
        _: &mut S::Data,
        _: &impl EraseQuerier<Q>,
    ) {
    }
}

impl<Q: QualifierFlag, A: QueryRelationStream<Q>, B: QueryRelationStream<Q>> QueryRelationStream<Q>
    for (A, B)
{
    fn relation<S: Stat>(
        &self,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    ) {
        self.0
            .relation(this, other, qualifier, stat, value, querier);
        self.1
            .relation(this, other, qualifier, stat, value, querier);
    }
}

pub trait QueryRelationStream<Q: QualifierFlag>: QueryStream<Q> {
    fn relation<S: Stat>(
        &self,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    );
}

/// Component and context based stat streams on children of [`StatEntity`](crate::StatEntity).
///
/// The item is generated from the [`ReadOnlyQueryData`] and a [`SystemParam`] context,
/// For example an `Asset` can be generated from a `Handle` and context `Assets`.
pub trait ComponentStream<Q: QualifierFlag>: QueryData {
    type Cx: ReadOnlySystemParam;
    fn stream<S: Stat>(
        ctx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    );
}

/// An item that can be used to generate stats when directly added to [`StatEntity`](crate::StatEntity).
///
/// The item also allows querying for "distance" or other relation between paired components on two entities.
pub trait RelationStream<Q: QualifierFlag>: ComponentStream<Q> {
    #[allow(unused)]
    /// Write to `stat` and return true ***if a value is written***.
    fn relation<S: Stat>(
        ctx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        this: <Self::ReadOnly as WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    );
}

#[derive(Debug, SystemParam)]
pub struct Querier<'w, 's, Q: QualifierFlag> {
    cache: Option<Res<'w, StatCache<Q>>>,
    defaults: Option<Res<'w, StatDefaults>>,
    entities: Query<'w, 's, Option<&'static Children>, With<StatEntity>>,
}

impl<'w, 's, Q: QualifierFlag> Querier<'w, 's, Q> {
    pub fn clear_cache(&mut self) {
        if let Some(cache) = &mut self.cache {
            cache.clear()
        }
    }

    pub fn with_component<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<D, F>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, impl QueryStream<Q> + 't, (), ()>
    where
        D: ComponentStream<Q, Cx = ()>,
    {
        JoinedQuerier {
            querier: self,
            component_streams: CxComponentStream { cx: &(), query },
            children_stream: (),
            relationship_streams: (),
        }
    }

    pub fn with_children<'t, D, F: QueryFilter>(
        &'t self,
        query: &'t Query<D, F>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, (), impl QueryStream<Q> + 't, ()>
    where
        D: ComponentStream<Q, Cx = ()>,
    {
        JoinedQuerier {
            querier: self,
            component_streams: (),
            children_stream: CxComponentStream { cx: &(), query },
            relationship_streams: (),
        }
    }

    pub fn with_relation<'t, D, F: QueryFilter>(
        &'t mut self,
        query: &'t Query<D, F>,
    ) -> JoinedQuerier<'_, 'w, 's, Q, (), (), impl QueryRelationStream<Q> + 't>
    where
        D: RelationStream<Q, Cx = ()>,
    {
        JoinedQuerier {
            querier: self,
            component_streams: (),
            children_stream: (),
            relationship_streams: CxComponentStream { cx: &(), query },
        }
    }
}

pub struct JoinedQuerier<
    't,
    'w,
    's,
    Q: QualifierFlag,
    A: QueryStream<Q>,
    B: QueryStream<Q>,
    C: QueryRelationStream<Q>,
> {
    querier: &'t Querier<'w, 's, Q>,
    component_streams: A,
    children_stream: B,
    relationship_streams: C,
}

pub trait EraseQuerier<Q: QualifierFlag> {
    fn query_stat<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Data>;

    fn query_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Data>;
}

pub struct NoopQuerier;

impl<Q: QualifierFlag> EraseQuerier<Q> for NoopQuerier {
    fn query_stat<S: Stat>(&self, _: Entity, _: &QualifierQuery<Q>, _: &S) -> Option<S::Data> {
        None
    }

    fn query_relation<S: Stat>(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &S,
    ) -> Option<S::Data> {
        None
    }
}

impl<Q: QualifierFlag, A: QueryStream<Q>, B: QueryStream<Q>, C: QueryRelationStream<Q>>
    EraseQuerier<Q> for JoinedQuerier<'_, '_, '_, Q, A, B, C>
{
    fn query_stat<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Data> {
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
            .stream(&[entity], query, stat, &mut result, self);
        self.relationship_streams
            .stream(&[entity], query, stat, &mut result, self);
        if let Ok(Some(children)) = self.querier.entities.get(entity) {
            self.component_streams
                .stream(children.as_ref(), query, stat, &mut result, self);
        }
        Some(result)
    }

    fn query_relation<S: Stat>(
        &self,
        from: Entity,
        to: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Data> {
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

pub struct CxComponentStream<'t, 'w, 's, Q: QualifierFlag, C: ComponentStream<Q>, F: QueryFilter> {
    cx: &'t <C::Cx as SystemParam>::Item<'w, 's>,
    query: &'t Query<'w, 's, C, F>,
}

impl<Q: QualifierFlag, C: ComponentStream<Q>, F: QueryFilter> QueryStream<Q>
    for CxComponentStream<'_, '_, '_, Q, C, F>
{
    fn stream<S: Stat>(
        &self,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    ) {
        for item in self.query.iter_many(entities) {
            C::stream(self.cx, item, qualifier, stat, value, querier)
        }
    }
}

impl<Q: QualifierFlag, C: RelationStream<Q>, F: QueryFilter> QueryRelationStream<Q>
    for CxComponentStream<'_, '_, '_, Q, C, F>
{
    fn relation<S: Stat>(
        &self,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Data,
        querier: &impl EraseQuerier<Q>,
    ) {
        let Ok(this) = self.query.get(this) else {
            return;
        };
        let Ok(other) = self.query.get(other) else {
            return;
        };
        C::relation(self.cx, this, other, qualifier, stat, value, querier)
    }
}
