use bevy_ecs::system::Resource;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{Stat, DynStat, types::StatComponents, Data, TYPE_ERROR};

/// An single step calculation on a [`StatComponents`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum StatOperation<S: StatComponents> {
    Add(S::Add),
    Mul(S::Mul),
    Or(S::Bit),
    Not(S::Bit),
    Min(S::Bounds),
    Max(S::Bounds),
}

impl<S: StatComponents> StatOperation<S> {
    pub fn write_to(&self, to: &mut S) {
        match self.clone() {
            StatOperation::Add(item) => to.add(item),
            StatOperation::Mul(item) => to.mul(item),
            StatOperation::Or(item) => to.or(item),
            StatOperation::Not(item) => to.not(item),
            StatOperation::Min(item) => to.min(item),
            StatOperation::Max(item) => to.max(item),
        }
    }
}

/// [`Resource`] that stores default [`StatComponents`] value per [`Stat`].
///
/// Uses [`Default::default()`] instead if not registered.
#[derive(Debug, Resource, Default)]
pub struct StatDefaults {
    stats: FxHashMap<Box<dyn DynStat>, Box<dyn Data>>,
}

impl StatDefaults {
    pub fn new() -> Self {
        Self {
            stats: FxHashMap::default(),
        }
    }

    /// Insert a [`Stat`] and its associated default value.
    pub fn insert<S: Stat>(&mut self, stat: S, value: S::Data) {
        self.stats.insert(Box::new(stat), Box::new(value));
    }

    /// Modify a [`Stat`]'s default value.
    pub fn patch<S: Stat>(&mut self, stat: &S, value: StatOperation<S::Data>) {
        match self.stats.get_mut(stat as &dyn DynStat) {
            Some(stat) => value.write_to(stat.downcast_mut::<S::Data>().expect(TYPE_ERROR)),
            None => {
                self.stats.insert(Box::new(stat.clone()), {
                    let mut stat = S::Data::default();
                    value.write_to(&mut stat);
                    Box::new(stat)
                });
            }
        }
    }

    /// Obtain a [`Stat`]'s default value.
    pub fn get<S: Stat>(&self, stat: &S) -> S::Data {
        self.stats.get(stat as &dyn DynStat)
            .and_then(|x| x.downcast_ref::<S::Data>())
            .cloned()
            .unwrap_or(Default::default())
    }

    /// Obtain a [`Stat`]'s default value.
    pub(crate) fn get_dyn(&self, stat: &dyn DynStat) -> Box<dyn Data> {
        self.stats.get(stat as &dyn DynStat)
            .cloned()
            .unwrap_or(stat.default_value())
    }
}
