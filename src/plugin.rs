use crate::operations::StatOperation;
use crate::{
    Qualifier, QualifierQuery, Querier, ShareableAny, Stat, StatStream, StatUid, StatValue,
    StatValuePair,
};
use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use bevy_ecs::world::World;
use bevy_reflect::TypePath;
use rustc_hash::FxHashMap;
use std::fmt::Debug;

type Bounds<T> = <<T as Stat>::Value as StatValue>::Bounds;

/// Extension on [`World`] and [`App`]
pub trait StatExtension {
    /// Register a default stat value.
    ///
    /// This is the standard way
    /// to add default bounds to a stat, e.g, in `1..=15`.
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self;

    /// Register the minimum value of a stat.
    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Register the maximum value of a stat.
    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Register a global stat relation
    /// that will be run on every stat query.
    fn register_stat_relation<Q: Qualifier>(
        &mut self,
        relation: impl Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>)
        + Send
        + Sync
        + 'static,
    ) -> &mut Self;
}

impl StatExtension for World {
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self {
        self.get_resource_or_insert_with::<GlobalStatDefaults>(Default::default)
            .insert(stat, value);
        self
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.get_resource_or_insert_with::<GlobalStatDefaults>(Default::default)
            .patch(stat, StatOperation::Min(value));
        self
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.get_resource_or_insert_with::<GlobalStatDefaults>(Default::default)
            .patch(stat, StatOperation::Max(value));
        self
    }

    fn register_stat_relation<Q: Qualifier>(
        &mut self,
        relation: impl Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>)
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.get_resource_or_insert_with(GlobalStatRelations::<Q>::default)
            .push(relation);
        self
    }
}

impl StatExtension for App {
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self {
        self.world_mut().register_stat_default::<S>(stat, value);
        self
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.world_mut().register_stat_min(stat, value);
        self
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.world_mut().register_stat_max(stat, value);
        self
    }

    fn register_stat_relation<Q: Qualifier>(
        &mut self,
        relation: impl Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>)
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.world_mut().register_stat_relation(relation);
        self
    }
}

/// [`Resource`] that stores default [`StatValue`]s per [`Stat`].
///
/// Stats that are not registered are still returned with [`Default::default()`] instead.
#[derive(Debug, Resource, Default, TypePath)]
pub struct GlobalStatDefaults {
    stats: FxHashMap<StatUid, Box<dyn ShareableAny>>,
}

impl GlobalStatDefaults {
    pub fn new() -> Self {
        Self {
            stats: FxHashMap::default(),
        }
    }

    /// Insert a [`Stat`] and its associated default value.
    pub fn insert<S: Stat>(&mut self, stat: S, value: S::Value) {
        self.stats.insert(stat.as_uid(), Box::new(value));
    }

    /// Modify a [`Stat`]'s default value.
    pub fn patch<S: Stat>(&mut self, stat: &S, value: StatOperation<S::Value>) {
        let uid = stat.as_uid();
        if let Some(v) = self.stats.get_mut(&uid) {
            if let Some(v) = v.as_any_mut().downcast_mut() {
                value.write_to(v);
                return;
            }
        }
        self.stats.insert(uid, {
            let mut stat = S::Value::default();
            value.write_to(&mut stat);
            Box::new(stat)
        });
    }

    /// Obtain a [`Stat`]'s default value.
    pub fn get<S: Stat>(&self, stat: &S) -> S::Value {
        self.stats
            .get(&stat.as_uid())
            .and_then(|x| x.as_any().downcast_ref())
            .cloned()
            .unwrap_or(Default::default())
    }
}

/// [`Resource`] that stores global [`StatStream`]s that runs on every query.
#[derive(Resource, TypePath)]
pub struct GlobalStatRelations<Q: Qualifier> {
    stats:
        Vec<Box<dyn Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>) + Send + Sync>>,
}

impl<Q: Qualifier> Debug for GlobalStatRelations<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalStatRelations")
            .finish_non_exhaustive()
    }
}

impl<Q: Qualifier> Default for GlobalStatRelations<Q> {
    fn default() -> Self {
        Self { stats: Vec::new() }
    }
}

impl<Q: Qualifier> GlobalStatRelations<Q> {
    pub fn push(
        &mut self,
        stream: impl Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>)
        + Send
        + Sync
        + 'static,
    ) -> &mut Self {
        self.stats.push(Box::new(stream));
        self
    }

    pub fn with(
        mut self,
        stream: impl Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>)
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.stats.push(Box::new(stream));
        self
    }
}

impl<Q: Qualifier> StatStream for GlobalStatRelations<Q> {
    type Qualifier = Q;

    fn stream_stat(
        &self,
        entity: Entity,
        qualifier: &crate::QualifierQuery<Q>,
        stat_value: &mut crate::StatValuePair,
        querier: crate::Querier<Q>,
    ) {
        for f in self.stats.iter() {
            f(entity, qualifier, stat_value, querier)
        }
    }
}
