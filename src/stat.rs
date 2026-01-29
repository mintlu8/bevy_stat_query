use std::{
    any::{Any, TypeId},
    cmp::{Eq, Ord},
    fmt::Debug,
    hash::Hash,
};

use crate::{GlobalStatDefaults, Shareable, ShareableAny, StatValue};

/// Instance of a stat.
///
/// # Safety Invariant
///
/// If two [`StatInst`]s are equal, their corresponding [`Stat`] and [`StatValue`]
/// they are constructed from must be equal.
/// This is achieved through the constraint placed on construction of [`StatVTable`], which makes
/// having the same [`ErasedStatVTable`] on two different [`Stat`]s impossible in safe rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StatUid {
    pub(crate) ty: TypeId,
    pub(crate) index: u64,
}

/// Implement this on your types to qualify them as a [`Stat`].
///
/// Each implementor can have its own `Value` type so you may want multiple of them.
pub trait Stat: Shareable {
    type Value: StatValue;

    /// Returns a globally unique name of the stat.
    fn name(&self) -> &'static str;

    /// Returns a locally unique index of the stat, used in equality comparisons.
    fn as_index(&self) -> u64;

    /// Returns a globally unique uid of the stat, used in equality comparisons.
    fn as_uid(&self) -> StatUid {
        StatUid {
            ty: TypeId::of::<Self>(),
            index: self.as_index(),
        }
    }

    /// Convert from a unique index of the stat.
    ///
    /// This function can panic in case of a mismatch.
    fn from_index(index: u64) -> Self;

    fn index_to_name(index: u64) -> &'static str {
        Self::from_index(index).name()
    }

    /// Register all fields for serialization.
    fn values() -> impl IntoIterator<Item = Self>;

    /// Check for equality on generic stats.
    fn is<T: Stat>(&self, other: &T) -> bool {
        self.as_uid() == other.as_uid()
    }

    /// Cast a generic [`Stat::Value`] to a concrete one.
    fn cast<'t, T: Stat>(&self, value: &'t mut Self::Value) -> Option<(&T, &'t mut T::Value)> {
        if TypeId::of::<Self>() == TypeId::of::<T>() {
            Some((
                (self as &dyn Any).downcast_ref()?,
                (value as &mut dyn Any).downcast_mut()?,
            ))
        } else {
            None
        }
    }

    /// Cast a generic [`Stat::Value`] to a concrete one if stat is equal.
    fn is_then_cast<'t, T: Stat>(
        &self,
        other: &T,
        value: &'t mut Self::Value,
    ) -> Option<&'t mut T::Value> {
        if !self.is(other) {
            return None;
        }
        (value as &mut dyn Any).downcast_mut()
    }
}

pub trait ErasedStat {
    fn name(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_uid(&self) -> StatUid;
    fn create_default_value(&self, defaults: Option<&GlobalStatDefaults>) -> Box<dyn ShareableAny>;
}

impl<T: Stat> ErasedStat for T {
    fn name(&self) -> &str {
        T::name(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_uid(&self) -> StatUid {
        T::as_uid(self)
    }

    fn create_default_value(&self, defaults: Option<&GlobalStatDefaults>) -> Box<dyn ShareableAny> {
        if let Some(defaults) = defaults {
            Box::new(defaults.get(self))
        } else {
            Box::new(T::Value::default())
        }
    }
}

impl Debug for &dyn ErasedStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A pair of stat and value in a query.
///
/// # Safety Invariant
/// `value` must be the correct [`Stat::Value`].
#[derive(Debug)]
pub struct StatValuePair<'t> {
    pub(crate) uid: StatUid,
    pub(crate) stat: &'t dyn ErasedStat,
    pub(crate) value: &'t mut dyn ShareableAny,
}

impl<'t> StatValuePair<'t> {
    pub fn new<S: Stat>(stat: &'t S, value: &'t mut S::Value) -> Self {
        StatValuePair {
            stat,
            value,
            uid: stat.as_uid(),
        }
    }

    pub fn new_dyn(stat: &'t dyn ErasedStat, value: &'t mut dyn ShareableAny) -> Self {
        StatValuePair {
            stat,
            value,
            uid: stat.as_uid(),
        }
    }

    /// Check for equality on generic stats.
    pub fn is<T: Stat>(&self, other: &T) -> bool {
        self.uid == other.as_uid()
    }

    /// Check for equality on generic stats.
    pub fn uid(&self) -> StatUid {
        self.uid
    }

    /// Cast to a concrete [`Stat::Value`].
    pub fn cast<T: Stat>(&mut self) -> Option<(&T, &mut T::Value)> {
        Some((
            self.stat.as_any().downcast_ref()?,
            self.value.downcast_mut()?,
        ))
    }

    /// Cast to a concrete [`Stat::Value`].
    pub fn is_then_cast<T: Stat>(&mut self, other: &T) -> Option<&mut T::Value> {
        if self.uid == other.as_uid() {
            self.value.downcast_mut()
        } else {
            None
        }
    }

    pub(crate) fn clone_value(&self) -> Box<dyn ShareableAny> {
        self.value.clone_boxed()
    }
}
