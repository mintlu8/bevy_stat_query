use std::any::Any;

use bevy_ecs::system::Resource;
use bevy_reflect::TypePath;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{types::StatValue, Stat, StatInst, TYPE_ERROR};

/// An single step unordered operation on a [`StatValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize, TypePath)]
#[type_path = "bevy_stat_query"]
#[type_name = "Op"]
pub enum StatOperation<S: StatValue> {
    Add(S::Add),
    Mul(S::Mul),
    Or(S::Bit),
    Not(S::Bit),
    Min(S::Bounds),
    Max(S::Bounds),
    Base(S::Base),
}

impl<S: StatValue> StatOperation<S> {
    pub fn write_to(&self, to: &mut S) {
        match self.clone() {
            StatOperation::Add(item) => to.add(item),
            StatOperation::Mul(item) => to.mul(item),
            StatOperation::Or(item) => to.or(item),
            StatOperation::Not(item) => to.not(item),
            StatOperation::Min(item) => to.min(item),
            StatOperation::Max(item) => to.max(item),
            StatOperation::Base(item) => *to = S::from_base(item),
        }
    }

    pub fn into_stat(self) -> S {
        let mut v = S::default();
        self.write_to(&mut v);
        v
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
    pub fn insert<S: Stat>(&mut self, stat: S, value: S::Data) {
        self.stats.insert(stat.as_entry(), Box::new(value));
    }

    /// Modify a [`Stat`]'s default value.
    pub fn patch<S: Stat>(&mut self, stat: &S, value: StatOperation<S::Data>) {
        match self.stats.get_mut(&stat.as_entry()) {
            Some(stat) => value.write_to(stat.downcast_mut::<S::Data>().expect(TYPE_ERROR)),
            None => {
                self.stats.insert(stat.as_entry(), {
                    let mut stat = S::Data::default();
                    value.write_to(&mut stat);
                    Box::new(stat)
                });
            }
        }
    }

    /// Obtain a [`Stat`]'s default value.
    pub fn get<S: Stat>(&self, stat: &S) -> S::Data {
        self.stats
            .get(&stat.as_entry())
            .and_then(|x| x.downcast_ref::<S::Data>())
            .cloned()
            .unwrap_or(Default::default())
    }
}
