use std::fmt::Debug;

use crate::operations::StatOperation;
use crate::{
    Buffer, QualifierFlag, QualifierQuery, Querier, Stat, StatExt, StatStream, StatValue,
    StatValuePair,
};
use crate::{StatCache, StatInst};
use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;
use bevy_ecs::world::World;
use bevy_reflect::TypePath;
use rustc_hash::FxHashMap;

type Bounds<T> = <<T as Stat>::Value as StatValue>::Bounds;

/// Extension on [`World`] and [`App`]
pub trait StatExtension {
    /// Register associated serialization routine for a stat.
    ///
    /// # Panics
    ///
    /// If trying to replace a previous stat entry with a different value.
    fn register_stat<T: Stat>(&mut self) -> &mut Self;

    /// Register a default stat value.
    ///
    /// This is the standard way
    /// to add default bounds to a stat, e.g, in `1..=15`.
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self;

    /// Register the minimum value of a stat.
    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Register the maximum value of a stat.
    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Clear all cached stats.
    fn clear_stat_cache<Q: QualifierFlag>(&mut self);

    /// Register a global stat relation
    /// that will be run on every stat query.
    fn register_stat_relation<Q: QualifierFlag>(
        &mut self,
        relation: impl Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>)
            + Send
            + Sync
            + 'static,
    ) -> &mut Self;
}

impl StatExtension for World {
    fn register_stat<T: Stat>(&mut self) -> &mut Self {
        self.get_resource_or_insert_with::<StatDeserializers>(Default::default)
            .register::<T>();
        self
    }

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

    fn clear_stat_cache<Q: QualifierFlag>(&mut self) {
        self.resource_mut::<StatCache<Q>>().clear();
    }

    fn register_stat_relation<Q: QualifierFlag>(
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
    fn register_stat<T: Stat>(&mut self) -> &mut Self {
        self.world_mut().register_stat::<T>();
        self
    }

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

    fn clear_stat_cache<Q: QualifierFlag>(&mut self) {
        self.world_mut().clear_stat_cache::<Q>()
    }

    fn register_stat_relation<Q: QualifierFlag>(
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
#[derive(Resource, Default, TypePath)]
pub struct GlobalStatDefaults {
    stats: FxHashMap<StatInst, Buffer>,
}

impl std::fmt::Debug for GlobalStatDefaults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Stat(&'static str);
        let mut map = f.debug_map();
        for (s, b) in &self.stats {
            map.entry(&Stat(s.name()), unsafe { (s.vtable.as_debug)(b) });
        }
        map.finish()
    }
}

impl GlobalStatDefaults {
    pub fn new() -> Self {
        Self {
            stats: FxHashMap::default(),
        }
    }

    /// Insert a [`Stat`] and its associated default value.
    pub fn insert<S: Stat>(&mut self, stat: S, value: S::Value) {
        self.stats.insert(stat.as_entry(), Buffer::from(value));
    }

    /// Modify a [`Stat`]'s default value.
    pub fn patch<S: Stat>(&mut self, stat: &S, value: StatOperation<S::Value>) {
        let stat = stat.as_entry();
        match self.stats.get_mut(&stat) {
            Some(v) => value.write_to(unsafe { v.as_mut() }),
            None => {
                self.stats.insert(stat, {
                    let mut stat = S::Value::default();
                    value.write_to(&mut stat);
                    Buffer::from(value)
                });
            }
        }
    }

    /// Obtain a [`Stat`]'s default value.
    pub fn get<S: Stat>(&self, stat: &S) -> S::Value {
        self.stats
            .get(&stat.as_entry())
            .map(|x| unsafe { x.as_ref() })
            .cloned()
            .unwrap_or(Default::default())
    }

    /// Obtain a [`Stat`]'s default value.
    pub(crate) fn get_dyn(&self, stat: StatInst) -> Buffer {
        self.stats
            .get(&stat)
            .map(|x| unsafe { stat.clone_buffer(x) })
            .unwrap_or((stat.vtable.default)())
    }
}

impl Drop for GlobalStatDefaults {
    fn drop(&mut self) {
        for (k, v) in self.stats.iter_mut() {
            unsafe { k.drop_buffer(v) };
        }
    }
}

/// [`Resource`] that stores global [`StatStream`]s that runs on every query.
#[derive(Resource, TypePath)]
pub struct GlobalStatRelations<Q: QualifierFlag> {
    stats:
        Vec<Box<dyn Fn(Entity, &QualifierQuery<Q>, &mut StatValuePair, Querier<Q>) + Send + Sync>>,
}

impl<Q: QualifierFlag> Debug for GlobalStatRelations<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalStatRelations")
            .finish_non_exhaustive()
    }
}

impl<Q: QualifierFlag> Default for GlobalStatRelations<Q> {
    fn default() -> Self {
        Self { stats: Vec::new() }
    }
}

impl<Q: QualifierFlag> GlobalStatRelations<Q> {
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

impl<Q: QualifierFlag> StatStream for GlobalStatRelations<Q> {
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

/// Resource containing a name to instance map of [`Stat`]s.
#[derive(Resource, Default)]
pub struct StatDeserializers {
    pub(crate) concrete: FxHashMap<&'static str, StatInst>,
}

impl Debug for StatDeserializers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatInstances")
            .field("concrete", &self.concrete)
            .finish()
    }
}

impl StatDeserializers {
    /// Register all members of a [`Stat`].
    ///
    /// # Panics
    ///
    /// If a stat registered conflicts with a previous entry.
    pub fn register<T: Stat>(&mut self) {
        T::values().into_iter().for_each(|x| {
            if let Some(prev) = self.concrete.get(x.name()) {
                assert_eq!(prev, &x.as_entry(), "duplicate key {}", x.name())
            } else {
                self.concrete.insert(x.name(), x.as_entry());
            }
        })
    }

    /// Register all members of a [`Stat`].
    ///
    /// Always replaces a registered [`Stat`] of the same key.
    pub fn register_replace<T: Stat>(&mut self) {
        T::values().into_iter().for_each(|x| {
            self.concrete.insert(x.name(), x.as_entry());
        })
    }

    pub fn get(&self, name: &str) -> Option<StatInst> {
        self.concrete.get(name).copied()
    }
}
