use std::{fmt::Debug, hash::Hash, ops::{BitAnd, BitOr}};
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::Shareable;

/// A flags like [`Qualifier`] for stats, normally bitflags or a set.
///
/// An application should ideally implement one [`QualifierFlag`] and multiple [`Stat`]s,
/// since different types of stats can still interop if they use the same [`QualifierFlag`].
pub trait QualifierFlags: BitOr<Self, Output=Self> + Ord + Hash + Shareable {
    fn contains(&self, other: &Self) -> bool;
    fn intersects(&self, other: &Self) -> bool;
    fn is_none_or_intersects(&self, other: &Self) -> bool {
        self.is_none() || self.intersects(other)
    }
    fn set_equals(&self, other: &Self) -> bool;
    fn none() -> Self;
    fn is_none(&self) -> bool;
}

impl<T> QualifierFlags for T where T: BitOr<Self, Output=Self> + Ord + Hash + BitAnd<Self, Output = Self> + Default + Shareable + Copy{
    fn contains(&self, other: &Self) -> bool {
        (*self & *other) == *other
    }

    fn set_equals(&self, other: &Self) -> bool {
        self == other
    }

    fn intersects(&self, other: &Self) -> bool {
        !(*self & *other).is_none()
    }

    fn none() -> Self {
        Self::default()
    }

    fn is_none(&self) -> bool {
        self == &Self::default()
    }
}

/// Data side qualifier for a stat.
///
/// # When stored
///
/// * `any_of` requires one or more conditions present.
/// * `all_of` requires all conditions present.
///
/// # Example
///
/// ```
/// // Requires 'fire' to receive buff from 'fire damage'.
/// let fire = QualifierFlags::all_of(Fire);
/// // Requires both 'ice' and 'piercing' to receive buff from 'ice piercing damage'
/// let ice_piercing = QualifierFlags::all_of(Ice | Piercing);
/// // Requires at least one of the elements to receive buff from 'elemental damage'.
/// let elemental = QualifierFlags::any_of(Fire | Water | Earth | Air);
/// // Requires one of the elements and 'piercing'.
/// let elemental_piercing = QualifierFlags::any_of(Fire | Water | Earth | Air)
///     .and_all_of(Piercing);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize, Deserialize)]
pub struct Qualifier<Q: QualifierFlags> {
    pub any_of: Q,
    pub all_of: Q,
}

impl<Q: QualifierFlags> Default for Qualifier<Q> {
    fn default() -> Self {
        Self {
            any_of: Q::none(),
            all_of: Q::none(),
        }
    }
}

impl<Q: QualifierFlags> Qualifier<Q> {

    pub fn none() -> Self {
        Self { 
            any_of: Q::none(), 
            all_of: Q::none() 
        }
    }

    pub fn is_none(&self) -> bool {
        self.any_of.is_none() && self.all_of.is_none()
    }

    pub fn any_of(qualifier: Q) -> Self {
        Self {
            any_of: qualifier,
            all_of: Q::none()
        }
    }

    pub fn all_of(qualifier: Q) -> Self {
        Self {
            any_of: Q::none(),
            all_of: qualifier
        }
    }

    pub fn and_any_of(self, qualifier: Q) -> Self {
        Self {
            any_of: self.any_of | qualifier,
            all_of: self.all_of,
        }
    }

    pub fn and_all_of(self, qualifier: Q) -> Self {
        Self {
            any_of: self.any_of,
            all_of: self.all_of | qualifier,
        }
    }

    /// # Examples
    /// * `elemental_damage` qualifies as `fire_damage`.
    /// * `fire_sword_damage` does not qualify as `fire_damage`.
    /// * `fire_damage` does not qualify as `elemental_damage`.
    /// * `fire_water_earth_air_damage` does not qualify as `elemental_damage`,
    pub fn qualifies_as(&self, queried: &QualifierQuery<Q>) -> bool {
        match queried {
            QualifierQuery::Aggregate(some_of) => {
                some_of.contains(&self.all_of) &&
                self.any_of.is_none_or_intersects(some_of)
            },
            QualifierQuery::Exact { any_of, all_of } => {
                self.any_of.contains(any_of) &&
                &self.all_of == all_of
            },
        }
    }
}


/// Query version of [`Qualifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum QualifierQuery<Q: QualifierFlags> {
    /// Look for qualifier that qualifies as this.
    ///
    /// Queried `any_of` intersects this (or is none) and this contains Queried `all_of`.
    Aggregate(Q),
    /// Look for qualifiers that are this and deny more generalized qualifiers.
    Exact {
        /// Queried `any_of` contains this.
        any_of: Q,
        /// Queried `all_of` equals this.
        all_of: Q,
    }
}

impl<Q: QualifierFlags> Default for QualifierQuery<Q> {
    fn default() -> Self {
        Self::Aggregate(Q::none())
    }
}

impl<Q: QualifierFlags> QualifierQuery<Q> {
    pub fn none() -> Self {
        Self::Aggregate(Q::none())
    }
}
