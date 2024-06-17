use crate::operations::StatOperation;
use crate::stat::StatInstances;
use crate::{Buffer, QualifierFlag, Stat, StatExt, StatValue};
use crate::{StatCache, StatInst};
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
#[derive(Resource, Default, TypePath)]
pub struct StatDefaults {
    stats: FxHashMap<StatInst, Buffer>,
}

impl std::fmt::Debug for StatDefaults {
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

impl StatDefaults {
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

impl Drop for StatDefaults {
    fn drop(&mut self) {
        for (k, v) in self.stats.iter_mut() {
            unsafe { k.drop_buffer(v) };
        }
    }
}
