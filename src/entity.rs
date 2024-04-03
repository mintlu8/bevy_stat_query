use std::borrow::Borrow;
use std::{fmt::Debug, hash::Hash};
use bevy_ecs::component::Component;
use bevy_utils::hashbrown::HashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use crate::types::DynStatValue;
use crate::{Stat, TYPE_ERROR};
use crate::{QualifierFlag, QualifierQuery, DynStat};

pub type StatQuery<Q> = (QualifierQuery<Q>, Box<dyn DynStat>);

/// The core marker component. Stat querying is only allowed on entities marked as [`StatEntity`].
#[derive(Debug, Component, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StatEntity;

/// This component acts as a cache to stats.
///
/// If using this component
/// the user must manually invalidate the cache if something has changed.
#[derive(Debug, Component, Serialize, Deserialize)]
#[serde(bound(serialize="", deserialize=""))]
pub struct StatCache<Q: QualifierFlag>{
    #[serde(skip)]
    pub(crate) cache: Mutex<HashMap<StatQuery<Q>, Box<dyn DynStatValue>>>
}

impl<Q: QualifierFlag> Default for StatCache<Q> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Q: QualifierFlag> StatCache<Q> {
    pub fn new() -> Self {
        Self { cache: Mutex::default() }
    }

    pub fn cache<S: Stat>(&mut self,
        query: QualifierQuery<Q>,
        stat: S,
        value: S::Data
    ) {
        self.cache.lock().insert((query, Box::new(stat)), Box::new(value));
    }

    pub fn try_get_cached<S: Stat>(
        &self,
        query: &QualifierQuery<Q>,
        stat: &S,
    ) -> Option<S::Data> {
        self.cache.lock().get(&(query, stat as &dyn DynStat) as &dyn StatQueryKey<Q>)
            .map(|value| value.downcast_ref::<S::Data>().expect(TYPE_ERROR).clone())
    }

    pub(crate) fn cache_dyn(&self,
        query: QualifierQuery<Q>,
        stat: Box<dyn DynStat>,
        value: Box<dyn DynStatValue>
    ) {
        self.cache.lock().insert((query, stat), value);
    }

    pub(crate) fn try_get_cached_dyn(
        &self,
        query: &QualifierQuery<Q>,
        stat: &dyn DynStat,
    ) -> Option<Box<dyn DynStatValue>> {
        self.cache.lock().get(&(query, stat) as &dyn StatQueryKey<Q>)
            .cloned()
    }

    pub fn invalidate_all(&self) {
        self.cache.lock().clear();
    }

    pub fn invalidate_stat<S: Stat>(&self, stat: &S) {
        self.cache.lock().retain(|(_, s), _| s == stat);
    }
}

trait StatQueryKey<Q: QualifierFlag> {
    fn qualifier(&self) -> &QualifierQuery<Q>;
    fn stat(&self) -> &dyn DynStat;
}


impl<Q: QualifierFlag> PartialEq for dyn StatQueryKey<Q> + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.qualifier() == other.qualifier() && self.stat() == other.stat()
    }
}

impl<Q: QualifierFlag> Eq for dyn StatQueryKey<Q> + '_ {}

impl<Q: QualifierFlag> Hash for dyn StatQueryKey<Q> + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.qualifier().hash(state);
        self.stat().hash(state);
    }
}

impl<Q: QualifierFlag> StatQueryKey<Q> for StatQuery<Q> {
    fn qualifier(&self) -> &QualifierQuery<Q> {
        &self.0
    }

    fn stat(&self) -> &dyn DynStat {
        self.1.as_ref()
    }
}

impl<'a, Q: QualifierFlag> Borrow<dyn StatQueryKey<Q> + 'a> for StatQuery<Q> {
    fn borrow(&self) -> &(dyn StatQueryKey<Q> + 'a) {
        self
    }
}


impl<Q: QualifierFlag> StatQueryKey<Q> for (&QualifierQuery<Q>, &dyn DynStat) {
    fn qualifier(&self) -> &QualifierQuery<Q> {
        self.0
    }

    fn stat(&self) -> &dyn DynStat {
        self.1
    }
}

impl<'a, Q: QualifierFlag> Borrow<dyn StatQueryKey<Q> + 'a> for (&'a QualifierQuery<Q>, &'a dyn DynStat) {
    fn borrow(&self) -> &(dyn StatQueryKey<Q> + 'a) {
        self
    }
}
