use crate::{attribute::Attribute, stat::StatValuePair, QualifierFlag, QualifierQuery, Querier};
#[allow(unused)]
use bevy_ecs::component::Component;
use bevy_ecs::{
    component::Mutable, entity::Entity, hierarchy::Children, query::QueryData, relationship::RelationshipTarget, system::{Query, StaticSystemParam, SystemParam}
};

/// An isolated item that provides stat modifiers to a stat query.
#[allow(unused_variables)]
pub trait StatStream {
    type Qualifier: QualifierFlag;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
    }

    fn stream_relation(
        &self,
        other: &Self,
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        false
    }
}

impl<T> StatStream for &T
where
    T: StatStream,
{
    type Qualifier = T::Qualifier;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        T::stream_stat(self, entity, qualifier, stat_value, querier);
    }

    fn stream_relation(
        &self,
        other: &Self,
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        T::stream_relation(self, other, entity, target, qualifier, stat_value, querier);
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        T::has_attribute(self, entity, attribute)
    }
}

impl<A, B> StatStream for (A, B)
where
    A: StatStream,
    B: StatStream<Qualifier = A::Qualifier>,
{
    type Qualifier = A::Qualifier;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        self.0.stream_stat(entity, qualifier, stat_value, querier);
        self.1.stream_stat(entity, qualifier, stat_value, querier);
    }

    fn stream_relation(
        &self,
        other: &Self,
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        self.0
            .stream_relation(&other.0, entity, target, qualifier, stat_value, querier);
        self.1
            .stream_relation(&other.1, entity, target, qualifier, stat_value, querier);
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        self.0.has_attribute(entity, attribute) || self.1.has_attribute(entity, attribute)
    }
}

/// A set of [`Component`]s and external [`SystemParam`]s that provide
/// stat modifiers for an [`Entity`].
#[allow(unused_variables)]
pub trait QueryStream: 'static {
    type Qualifier: QualifierFlag;
    type Query: QueryData + 'static;
    type Context: SystemParam + 'static;

    fn stream_stat(
        query: <<Self::Query as QueryData>::ReadOnly as QueryData>::Item<'_>,
        context: &<Self::Context as SystemParam>::Item<'_, '_>,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
    }

    fn stream_relation(
        this: <<Self::Query as QueryData>::ReadOnly as QueryData>::Item<'_>,
        other: <<Self::Query as QueryData>::ReadOnly as QueryData>::Item<'_>,
        context: &<Self::Context as SystemParam>::Item<'_, '_>,
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
    }

    fn has_attribute(
        query: <<Self::Query as QueryData>::ReadOnly as QueryData>::Item<'_>,
        context: &<Self::Context as SystemParam>::Item<'_, '_>,
        entity: Entity,
        attribute: Attribute,
    ) -> bool {
        false
    }
}

impl<T> QueryStream for T
where
    T: Component<Mutability = Mutable> + StatStream,
{
    type Qualifier = T::Qualifier;
    type Query = &'static mut T;
    type Context = ();

    fn stream_stat(
        query: &T,
        _: &(),
        entity: Entity,
        qualifier: &QualifierQuery<T::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<T::Qualifier>,
    ) {
        query.stream_stat(entity, qualifier, stat_value, querier);
    }

    fn stream_relation(
        this: &T,
        other: &T,
        _: &(),
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<T::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<T::Qualifier>,
    ) {
        this.stream_relation(other, entity, target, qualifier, stat_value, querier);
    }

    fn has_attribute(query: &T, _: &(), entity: Entity, attribute: Attribute) -> bool {
        query.has_attribute(entity, attribute)
    }
}

/// [`SystemParam`] for querying a [`QueryStream`].
#[derive(SystemParam)]
pub struct StatQuery<'w, 's, T: QueryStream> {
    pub query: Query<'w, 's, <<T as QueryStream>::Query as QueryData>::ReadOnly>,
    pub context: StaticSystemParam<'w, 's, <T as QueryStream>::Context>,
}

/// [`SystemParam`] for querying a [`QueryStream`].
///
/// Unlike [`StatQuery`], [`StatQueryMut`]'s query portion uses a mutable query,
/// this is useful if you want to query and modify at the same time.
#[derive(SystemParam)]
pub struct StatQueryMut<'w, 's, T: QueryStream> {
    pub query: Query<'w, 's, <T as QueryStream>::Query>,
    pub context: StaticSystemParam<'w, 's, <T as QueryStream>::Context>,
}

impl<T: QueryStream> StatStream for StatQuery<'_, '_, T> {
    type Qualifier = T::Qualifier;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Ok(item) = self.query.get(entity) {
            T::stream_stat(item, &self.context, entity, qualifier, stat_value, querier);
        }
    }

    fn stream_relation(
        &self,
        _: &Self,
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Ok([this, other]) = self.query.get_many([entity, target]) {
            T::stream_relation(
                this,
                other,
                &self.context,
                entity,
                target,
                qualifier,
                stat_value,
                querier,
            );
        }
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        if let Ok(item) = self.query.get(entity) {
            T::has_attribute(item, &self.context, entity, attribute)
        } else {
            false
        }
    }
}

impl<T: QueryStream> StatStream for StatQueryMut<'_, '_, T> {
    type Qualifier = T::Qualifier;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Ok(item) = self.query.get(entity) {
            T::stream_stat(item, &self.context, entity, qualifier, stat_value, querier);
        }
    }

    fn stream_relation(
        &self,
        _: &Self,
        entity: Entity,
        target: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Ok([this, other]) = self.query.get_many([entity, target]) {
            T::stream_relation(
                this,
                other,
                &self.context,
                entity,
                target,
                qualifier,
                stat_value,
                querier,
            );
        }
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        if let Ok(item) = self.query.get(entity) {
            T::has_attribute(item, &self.context, entity, attribute)
        } else {
            false
        }
    }
}

/// A component that references other entities, like [`Children`].
pub trait EntityReference: Component + 'static {
    fn iter_entities(&self) -> impl Iterator<Item = Entity>;
}

impl<T> EntityReference for T
where
    T: RelationshipTarget,
{
    fn iter_entities(&self) -> impl Iterator<Item = Entity> {
        self.iter()
    }
}

/// [`SystemParam`] for querying [`QueryStream`]s on entities referenced by a component like [`Children`].
///
/// `query_relation` implementation is disabled since the behavior is undefined.
#[derive(SystemParam)]
pub struct ChildQuery<'w, 's, T: QueryStream, C: EntityReference = Children> {
    pub query: Query<'w, 's, <<T as QueryStream>::Query as QueryData>::ReadOnly>,
    pub context: StaticSystemParam<'w, 's, <T as QueryStream>::Context>,
    pub children: Query<'w, 's, &'static C>,
}

/// [`SystemParam`] for querying [`QueryStream`]s on entities referenced by a component like [`Children`].
///
/// `query_relation` implementation is disabled since the behavior is undefined.
#[derive(SystemParam)]
pub struct ChildQueryMut<'w, 's, T: QueryStream, C: EntityReference = Children> {
    pub query: Query<'w, 's, <T as QueryStream>::Query>,
    pub context: StaticSystemParam<'w, 's, <T as QueryStream>::Context>,
    pub children: Query<'w, 's, &'static C>,
}

impl<T: QueryStream, C: EntityReference> StatStream for ChildQuery<'_, '_, T, C> {
    type Qualifier = T::Qualifier;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Ok(children) = self.children.get(entity) {
            for item in self.query.iter_many(children.iter_entities()) {
                T::stream_stat(item, &self.context, entity, qualifier, stat_value, querier);
            }
        }
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        if let Ok(children) = self.children.get(entity) {
            for item in self.query.iter_many(children.iter_entities()) {
                if T::has_attribute(item, &self.context, entity, attribute) {
                    return true;
                }
            }
        }
        false
    }
}

impl<T: QueryStream, C: EntityReference> StatStream for ChildQueryMut<'_, '_, T, C> {
    type Qualifier = T::Qualifier;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &QualifierQuery<Self::Qualifier>,
        stat_value: &mut StatValuePair,
        querier: Querier<Self::Qualifier>,
    ) {
        if let Ok(children) = self.children.get(entity) {
            for item in self.query.iter_many(children.iter_entities()) {
                T::stream_stat(item, &self.context, entity, qualifier, stat_value, querier);
            }
        }
    }

    fn has_attribute(&self, entity: Entity, attribute: Attribute) -> bool {
        if let Ok(children) = self.children.get(entity) {
            for item in self.query.iter_many(children.iter_entities()) {
                if T::has_attribute(item, &self.context, entity, attribute) {
                    return true;
                }
            }
        }
        false
    }
}
