use crate::{QualifierFlag, QualifierQuery, Querier, Stat};
use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    system::{Query, ReadOnlySystemParam, SystemParam},
};
#[allow(unused)]
use bevy_ecs::component::Component;


/// A stream that writes to a given stat query.
pub trait StatStream<Q: QualifierFlag> {
    fn stream_stat<S: Stat>(
        &self,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    );
}

mod sealed {
    use bevy_ecs::entity::Entity;

    use crate::{QualifierFlag, QualifierQuery, Querier, Stat};

    pub trait QueryStream<Q: QualifierFlag> {
        fn stream<S: Stat>(
            &self,
            entity: Entity,
            entities: &[Entity],
            qualifier: &QualifierQuery<Q>,
            stat: &S,
            value: &mut S::Value,
            querier: &impl Querier<Q>,
        );
    }

    pub trait QueryRelationStream<Q: QualifierFlag>: QueryStream<Q> {
        fn relation<S: Stat>(
            &self,
            this: Entity,
            other: Entity,
            qualifier: &QualifierQuery<Q>,
            stat: &S,
            value: &mut S::Value,
            querier: &impl Querier<Q>,
        );
    }
}

pub(crate) use sealed::*;

impl<Q: QualifierFlag> QueryStream<Q> for () {
    fn stream<S: Stat>(
        &self,
        _: Entity,
        _: &[Entity],
        _: &QualifierQuery<Q>,
        _: &S,
        _: &mut S::Value,
        _: &impl Querier<Q>,
    ) {
    }
}

impl<Q: QualifierFlag, A: QueryStream<Q>, B: QueryStream<Q>> QueryStream<Q> for (A, B) {
    fn stream<S: Stat>(
        &self,
        entity: Entity,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    ) {
        self.0
            .stream(entity, entities, qualifier, stat, value, querier);
        self.1
            .stream(entity, entities, qualifier, stat, value, querier);
    }
}

impl<Q: QualifierFlag> QueryRelationStream<Q> for () {
    fn relation<S: Stat>(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &S,
        _: &mut S::Value,
        _: &impl Querier<Q>,
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
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    ) {
        self.0
            .relation(this, other, qualifier, stat, value, querier);
        self.1
            .relation(this, other, qualifier, stat, value, querier);
    }
}

/// A [`Component`] or [`QueryData`] that can be used to query stats 
/// when added to a [`Entity`] or a child of the entity.
pub trait ComponentStream<Q: QualifierFlag>: QueryData {
    type Cx: ReadOnlySystemParam;
    /// Writes to queried stats.
    fn stream<S: Stat>(
        this: Entity,
        cx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    );
}

/// A [`Component`] or [`QueryData`] that can be used to query relation between entities.
pub trait RelationStream<Q: QualifierFlag>: ComponentStream<Q> {
    #[allow(unused)]
    /// Writes to queried stats representing the relationship between two entities.
    fn relation<S: Stat>(
        this: <Self::ReadOnly as WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as WorldQuery>::Item<'_>,
        cx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    );
}

pub(crate) struct CxComponentStream<
    't,
    'w,
    's,
    Q: QualifierFlag,
    C: ComponentStream<Q>,
    F: QueryFilter,
> {
    pub cx: &'t <C::Cx as SystemParam>::Item<'w, 's>,
    pub query: &'t Query<'w, 's, C, F>,
}

impl<Q: QualifierFlag, C: ComponentStream<Q>, F: QueryFilter> QueryStream<Q>
    for CxComponentStream<'_, '_, '_, Q, C, F>
{
    fn stream<S: Stat>(
        &self,
        entity: Entity,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat: &S,
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    ) {
        for item in self.query.iter_many(entities) {
            C::stream(entity, self.cx, item, qualifier, stat, value, querier)
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
        value: &mut S::Value,
        querier: &impl Querier<Q>,
    ) {
        let Ok(this) = self.query.get(this) else {
            return;
        };
        let Ok(other) = self.query.get(other) else {
            return;
        };
        C::relation(this, other, self.cx, qualifier, stat, value, querier)
    }
}
