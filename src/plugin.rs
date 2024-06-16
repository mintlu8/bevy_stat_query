use std::any::Any;

use crate::operations::StatOperation;
use crate::stat::StatInstances;
use crate::{QualifierFlag, Stat, StatExt, StatValue};
use crate::{StatCache, StatInst, TYPE_ERROR};
use bevy_app::App;
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
    /// to add default bounds to a stat, e.g, in `1..15`.
    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self;

    /// Register the minimum value of a stat.
    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Register the maximum value of a stat.
    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self;

    /// Clear all cached stats.
    fn clear_stat_cache<Q: QualifierFlag>(&mut self);
}

impl StatExtension for World {
    fn register_stat<T: Stat>(&mut self) -> &mut Self {
        self.get_resource_or_insert_with::<StatInstances>(Default::default)
            .register::<T>();
        self
    }

    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self {
        self.get_resource_or_insert_with::<StatDefaults>(Default::default)
            .insert(stat, value);
        self
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.get_resource_or_insert_with::<StatDefaults>(Default::default)
            .patch(stat, StatOperation::Min(value));
        self
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.get_resource_or_insert_with::<StatDefaults>(Default::default)
            .patch(stat, StatOperation::Max(value));
        self
    }

    fn clear_stat_cache<Q: QualifierFlag>(&mut self) {
        self.resource_mut::<StatCache<Q>>().clear();
    }
}

impl StatExtension for App {
    fn register_stat<T: Stat>(&mut self) -> &mut Self {
        self.world.register_stat::<T>();
        self
    }

    fn register_stat_default<S: Stat>(&mut self, stat: S, value: S::Value) -> &mut Self {
        self.world.register_stat_default::<S>(stat, value);
        self
    }

    fn register_stat_min<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.world.register_stat_min(stat, value);
        self
    }

    fn register_stat_max<S: Stat>(&mut self, stat: &S, value: Bounds<S>) -> &mut Self {
        self.world.register_stat_max(stat, value);
        self
    }

    fn clear_stat_cache<Q: QualifierFlag>(&mut self) {
        self.world.clear_stat_cache::<Q>()
    }
}

/// [`Resource`] that stores default [`StatValue`]s per [`Stat`].
///
/// Stats that are not registered are still returned with [`Default::default()`] instead.
#[derive(Debug, Resource, Default, TypePath)]
pub struct StatDefaults {
    stats: FxHashMap<StatInst, Box<dyn Any + Send + Sync>>,
}

impl StatDefaults {
    pub fn new() -> Self {
        Self {
            stats: FxHashMap::default(),
        }
    }

    /// Insert a [`Stat`] and its associated default value.
    pub fn insert<S: Stat>(&mut self, stat: S, value: S::Value) {
        self.stats.insert(stat.as_entry(), Box::new(value));
    }

    /// Modify a [`Stat`]'s default value.
    pub fn patch<S: Stat>(&mut self, stat: &S, value: StatOperation<S::Value>) {
        match self.stats.get_mut(&stat.as_entry()) {
            Some(stat) => value.write_to(stat.downcast_mut::<S::Value>().expect(TYPE_ERROR)),
            None => {
                self.stats.insert(stat.as_entry(), {
                    let mut stat = S::Value::default();
                    value.write_to(&mut stat);
                    Box::new(stat)
                });
            }
        }
    }

    /// Obtain a [`Stat`]'s default value.
    pub fn get<S: Stat>(&self, stat: &S) -> S::Value {
        self.stats
            .get(&stat.as_entry())
            .and_then(|x| x.downcast_ref::<S::Value>())
            .cloned()
            .unwrap_or(Default::default())
    }
}
