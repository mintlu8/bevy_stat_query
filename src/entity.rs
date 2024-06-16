use crate::{QualifierFlag, QualifierQuery, StatInst};
use crate::{Stat, TYPE_ERROR};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::RwLock;
use std::{fmt::Debug, hash::Hash};

/// The core marker component. Stat querying is only allowed on entities marked as [`StatEntity`].
#[derive(Debug, Component, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TypePath)]
pub struct StatEntity;

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
#[derive(Debug, Resource, Serialize, Deserialize, TypePath)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct StatCache<Q: QualifierFlag> {
    #[serde(skip)]
    pub(crate) cache: RwLock<FxHashMap<CachedEntry<Q>, Box<dyn Any + Send + Sync>>>,
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

    pub fn cache<S: Stat>(
        &self,
        entity: Entity,
        query: QualifierQuery<Q>,
        stat: S,
        value: S::Data,
    ) {
        self.cache.write().unwrap().insert(
            CachedEntry {
                entity,
                query,
                stat: stat.as_entry(),
            },
            Box::new(value),
        );
    }

    pub fn try_get_cached<S: Stat>(
        &self,
        entity: Entity,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Data> {
        self.cache
            .read()
            .unwrap()
            .get(&CachedEntry {
                entity,
                query: query.clone(),
                stat: stat.as_entry(),
            })
            .map(|value| value.downcast_ref::<S::Data>().expect(TYPE_ERROR).clone())
    }

    pub fn clear(&self) {
        self.cache.write().unwrap().clear()
    }
}
