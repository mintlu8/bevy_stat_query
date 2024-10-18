use crate::{stat::StatValuePair, QualifierFlag, QualifierQuery, Querier};
#[allow(unused)]
use bevy_ecs::component::Component;
use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, WorldQuery},
    system::{Query, ReadOnlySystemParam, SystemParam},
};

/// A stream that writes to a given stat query.
pub trait StatStream<Q: QualifierFlag> {
    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
    );
}

mod sealed {
    use bevy_ecs::entity::Entity;

    use crate::{stat::StatValuePair, QualifierFlag, QualifierQuery, Querier};

    pub trait QueryStream<Q: QualifierFlag> {
        fn stream(
            &self,
            entity: Entity,
            entities: &[Entity],
            qualifier: &QualifierQuery<Q>,
            stat_value: &mut StatValuePair,
            querier: Querier<Q>,
        );

        fn has_attribute(
            &self,
            entity: Entity,
            entities: &[Entity],
            attribute: &str,
            querier: Querier<Q>,
        ) -> bool;
    }

    pub trait QueryRelationStream<Q: QualifierFlag>: QueryStream<Q> {
        fn relation(
            &self,
            this: Entity,
            other: Entity,
            qualifier: &QualifierQuery<Q>,
            stat_value: &mut StatValuePair,
            querier: Querier<Q>,
        );
    }
}

pub(crate) use sealed::*;

impl<Q: QualifierFlag> QueryStream<Q> for () {
    fn stream(
        &self,
        _: Entity,
        _: &[Entity],
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: Querier<Q>,
    ) {
    }

    fn has_attribute(
        &self,
        _: Entity,
        _: &[Entity],
        _: &str,
        _: Querier<Q>,
    ) -> bool {
        false
    }
}

impl<Q: QualifierFlag, A: QueryStream<Q>, B: QueryStream<Q>> QueryStream<Q> for (A, B) {
    fn stream(
        &self,
        entity: Entity,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
    ) {
        self.0
            .stream(entity, entities, qualifier, stat_value, querier);
        self.1
            .stream(entity, entities, qualifier, stat_value, querier);
    }

    fn has_attribute(
        &self,
        entity: Entity,
        entities: &[Entity],
        attribute: &str,
        querier: Querier<Q>,
    ) -> bool {
        self.0.has_attribute(entity, entities, attribute, querier) || self.1.has_attribute(entity, entities, attribute, querier)
    }
}

impl<Q: QualifierFlag> QueryRelationStream<Q> for () {
    fn relation(
        &self,
        _: Entity,
        _: Entity,
        _: &QualifierQuery<Q>,
        _: &mut StatValuePair,
        _: Querier<Q>,
    ) {
    }
}

impl<Q: QualifierFlag, A: QueryRelationStream<Q>, B: QueryRelationStream<Q>> QueryRelationStream<Q>
    for (A, B)
{
    fn relation(
        &self,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
    ) {
        self.0.relation(this, other, qualifier, stat_value, querier);
        self.1.relation(this, other, qualifier, stat_value, querier);
    }
}

/// A [`Component`] or [`QueryData`] that can be used to query stats
/// when added to a [`Entity`] or a child of the entity.
#[allow(unused_variables)]
pub trait ComponentStream<Q: QualifierFlag>: QueryData {
    type Cx: ReadOnlySystemParam;
    /// Writes to queried stats.
    fn stream(
        this: Entity,
        cx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as WorldQuery>::Item<'_>,
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
    ){}
    
    fn has_attribute(
        this: Entity,
        cx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        component: <Self::ReadOnly as WorldQuery>::Item<'_>,
        attribute: &str,
        querier: Querier<Q>,
    ) -> bool { false }
}

/// A [`Component`] or [`QueryData`] that can be used to query relation between entities.
pub trait RelationStream<Q: QualifierFlag>: ComponentStream<Q> {
    #[allow(unused)]
    /// Writes to queried stats representing the relationship between two entities.
    fn relation(
        this: <Self::ReadOnly as WorldQuery>::Item<'_>,
        other: <Self::ReadOnly as WorldQuery>::Item<'_>,
        cx: &<Self::Cx as SystemParam>::Item<'_, '_>,
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
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
    fn stream(
        &self,
        entity: Entity,
        entities: &[Entity],
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
    ) {
        for item in self.query.iter_many(entities) {
            C::stream(entity, self.cx, item, qualifier, stat_value, querier)
        }
    }

    fn has_attribute(
        &self,
        entity: Entity,
        entities: &[Entity],
        attribute: &str,
        querier: Querier<Q>,
    ) -> bool {
        for item in self.query.iter_many(entities) {
            if C::has_attribute(entity, self.cx, item, attribute, querier) {
                return true;
            }
        }
        false
    }
}

impl<Q: QualifierFlag, C: RelationStream<Q>, F: QueryFilter> QueryRelationStream<Q>
    for CxComponentStream<'_, '_, '_, Q, C, F>
{
    fn relation(
        &self,
        this: Entity,
        other: Entity,
        qualifier: &QualifierQuery<Q>,
        stat_value: &mut StatValuePair,
        querier: Querier<Q>,
    ) {
        let Ok(this) = self.query.get(this) else {
            return;
        };
        let Ok(other) = self.query.get(other) else {
            return;
        };
        C::relation(this, other, self.cx, qualifier, stat_value, querier)
    }
}
