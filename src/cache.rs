use crate::stat::StatValuePair;
use crate::{Buffer, QualifierFlag, QualifierQuery, StatInst};
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::{fmt::Debug, hash::Hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CachedEntry<Q: QualifierFlag> {
    pub entity: Entity,
    pub query: QualifierQuery<Q>,
    pub stat: StatInst,
}

/// This component acts as a cache to stats.
///
/// If using this component
/// the user must manually invalidate the cache if something has changed.
#[derive(Resource, Serialize, Deserialize, TypePath)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct StatCache<Q: QualifierFlag> {
    #[serde(skip)]
    pub(crate) cache: RwLock<FxHashMap<CachedEntry<Q>, Buffer>>,
}

impl<Q: QualifierFlag> Debug for StatCache<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Stat(&'static str);
        let mut map = f.debug_map();
        for (c, b) in self.cache.read().unwrap().iter() {
            map.entry(&(c.entity, &c.query, Stat(c.stat.name())), unsafe {
                (c.stat.vtable.as_debug)(b)
            });
        }
        map.finish()
    }
}

impl<Q: QualifierFlag> Default for StatCache<Q> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Q: QualifierFlag> StatCache<Q> {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
        }
    }

    pub fn cache_pair(&self, entity: Entity, query: QualifierQuery<Q>, pair: &StatValuePair) {
        self.cache.write().unwrap().insert(
            CachedEntry {
                entity,
                query,
                stat: pair.stat,
            },
            pair.clone_buffer(),
        );
    }

    pub(crate) fn try_get_cached_dyn(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: StatInst,
    ) -> Option<Buffer> {
        self.cache
            .read()
            .unwrap()
            .get(&CachedEntry {
                entity,
                query: query.clone(),
                stat,
            })
            .map(|x| unsafe { stat.clone_buffer(x) })
    }

    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        for (k, v) in cache.iter_mut() {
            unsafe { k.stat.drop_buffer(v) };
        }
        cache.clear()
    }
}

impl<Q: QualifierFlag> Drop for StatCache<Q> {
    fn drop(&mut self) {
        for (k, v) in self.cache.write().unwrap().iter_mut() {
            unsafe { k.stat.drop_buffer(v) };
        }
    }
}
