use bevy_reflect::Reflect;
use std::{
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ops::{BitAnd, BitOr},
};

#[allow(unused)]
use crate::{Shareable, Stat};

/// A flags like [`Qualifier`] for stats, normally bitflags or a set.
///
/// An application should ideally implement one [`Qualifier`] and multiple [`Stat`]s,
/// since different types of stats can still interop if they use the same [`Qualifier`].
pub trait Qualifier: BitOr<Self, Output = Self> + Ord + Hash + Shareable {
    fn contains(&self, other: &Self) -> bool;
    fn intersects(&self, other: &Self) -> bool;
    fn is_none_or_intersects(&self, other: &Self) -> bool {
        self.is_none() || self.intersects(other)
    }
    fn set_equals(&self, other: &Self) -> bool;
    fn none() -> Self;
    fn is_none(&self) -> bool;
}

impl<T> Qualifier for T
where
    T: BitOr<Self, Output = Self>
        + Ord
        + Hash
        + BitAnd<Self, Output = Self>
        + Default
        + Shareable
        + Copy,
{
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

/// The standard [`QualifierKey`] on a stat modifier, typically used in [`StatMap`](crate::StatMap).
///
/// This provides a `all_of` field and a singular `any_of` field.
///
/// * `all_of` requires all conditions present.
/// * `any_of` requires one or more conditions present (if not none).
///
/// `all_of` represents most common descriptors like `sword, slashing, physical`,
/// while `any_of` represents descriptors like `elemental`, which can represent `fire | water | earth | air`.
///
/// # Example
///
/// ```
/// // Requires 'fire' to receive buff from 'fire damage'.
/// let fire = QualifierItem::all_of(Fire);
/// // Requires both 'ice' and 'piercing' to receive buff from 'ice piercing damage'
/// let ice_piercing = QualifierItem::all_of(Ice | Piercing);
/// // Requires at least one of the elements to receive buff from 'elemental damage'.
/// let elemental = QualifierItem::any_of(Fire | Water | Earth | Air);
/// // Requires one of the elements, Sword, Slashing and Physical.
/// let elemental_slash = QualifierItem {
///     any_of: Fire | Water | Earth | Air,
///     all_of: Sword | Slashing | Physical
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QualifierItem<Q: Qualifier> {
    pub all_of: Q,
    pub any_of: Q,
}

impl<Q: Qualifier> Default for QualifierItem<Q> {
    fn default() -> Self {
        Self {
            any_of: Q::none(),
            all_of: Q::none(),
        }
    }
}

impl<Q: Qualifier> From<Q> for QualifierItem<Q> {
    fn from(value: Q) -> Self {
        QualifierItem {
            all_of: value,
            any_of: Q::none(),
        }
    }
}

impl<Q: Qualifier> QualifierItem<Q> {
    pub fn none() -> Self {
        Self {
            any_of: Q::none(),
            all_of: Q::none(),
        }
    }

    pub fn is_none(&self) -> bool {
        self.any_of.is_none() && self.all_of.is_none()
    }

    pub fn any_of(qualifier: Q) -> Self {
        Self {
            any_of: qualifier,
            all_of: Q::none(),
        }
    }

    pub fn all_of(qualifier: Q) -> Self {
        Self {
            any_of: Q::none(),
            all_of: qualifier,
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
        self.qualify_query(queried)
    }
}

/// Query version of [`Qualifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum QualifierQuery<'t, Q: Qualifier> {
    /// Look for qualifier that qualifies as this.
    ///
    /// Queried `any_of` intersects this (or is none) and this contains Queried `all_of`.
    Aggregate(Q),
    /// Look for qualifiers that satisfies all conditions.
    Custom(&'t [QualifierConstraint<Q>]),
}

impl<Q: Qualifier> QualifierQuery<'_, Q> {
    pub fn qualifies_none(&self) -> bool {
        QualifierItem::none().qualifies_as(self)
    }

    pub fn qualify(&self, qualifier: &impl QualifierKey<Qualifier = Q>) -> bool {
        qualifier.qualify_query(self)
    }
}

impl<Q: Qualifier> Default for QualifierQuery<'_, Q> {
    fn default() -> Self {
        Self::Aggregate(Q::none())
    }
}

impl<Q: Qualifier> QualifierQuery<'_, Q> {
    pub fn none() -> Self {
        Self::Aggregate(Q::none())
    }
}

impl<Q: Qualifier> From<Q> for QualifierQuery<'_, Q> {
    fn from(value: Q) -> Self {
        QualifierQuery::Aggregate(value)
    }
}

/// Constraint for [`QualifierQuery::Custom`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum QualifierConstraint<Q: Qualifier> {
    /// The default query, contains `all_of` and intersects `any_of`.
    Aggregate(Q),
    /// Requires an exact match of the item's `all_of`.
    Exact(Q),
    /// Requires the item's `all_of` to contain this qualifier.
    Contains(Q),
    /// Requires an exact match of the item's `any_of`.
    ExactAnyOf(Q),
    /// Requires the item's `any_of` to contain this qualifier.
    ContainsAnyOf(Q),
}

/// Qualifier on a modifier of a stat.
///
/// # Implementations
///
/// Usually [`QualifierItem`],
/// [`PhantomData<Qualifier>`](std::marker::PhantomData) can
/// also be used for non-qualified stats.
pub trait QualifierKey: Ord {
    type Qualifier: Qualifier;
    fn qualify(&self, query: &Self::Qualifier) -> bool;
    fn qualify_item(&self, query: &QualifierConstraint<Self::Qualifier>) -> bool;
    fn qualify_query(&self, query: &QualifierQuery<Self::Qualifier>) -> bool {
        match query {
            QualifierQuery::Aggregate(q) => self.qualify(q),
            QualifierQuery::Custom(items) => {
                for item in items.iter() {
                    if !self.qualify_item(item) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl<Q: Qualifier> QualifierKey for QualifierItem<Q> {
    type Qualifier = Q;
    fn qualify(&self, query: &Self::Qualifier) -> bool {
        query.contains(&self.all_of) && self.any_of.is_none_or_intersects(query)
    }

    fn qualify_item(&self, query: &QualifierConstraint<Self::Qualifier>) -> bool {
        match query {
            QualifierConstraint::Aggregate(v) => {
                v.contains(&self.all_of) && self.any_of.is_none_or_intersects(v)
            }
            QualifierConstraint::Exact(v) => &self.all_of == v,
            QualifierConstraint::Contains(v) => self.all_of.contains(v),
            QualifierConstraint::ExactAnyOf(v) => &self.any_of == v,
            QualifierConstraint::ContainsAnyOf(v) => {
                !self.any_of.is_none() && v.contains(&self.any_of)
            }
        }
    }
}

impl<Q: Qualifier> QualifierKey for PhantomData<Q> {
    type Qualifier = Q;
    fn qualify(&self, _: &Self::Qualifier) -> bool {
        true
    }

    fn qualify_item(&self, query: &QualifierConstraint<Self::Qualifier>) -> bool {
        match query {
            QualifierConstraint::Aggregate(_) => true,
            QualifierConstraint::Exact(v) => v.is_none(),
            QualifierConstraint::Contains(v) => v.is_none(),
            QualifierConstraint::ExactAnyOf(v) => v.is_none(),
            QualifierConstraint::ContainsAnyOf(v) => v.is_none(),
        }
    }
}
