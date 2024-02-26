use std::{any::{Any, TypeId}, borrow::Cow, cmp::Ordering, fmt::Debug, hash::Hash, ops::{BitAnd, BitOr}};
use bevy_reflect::Reflect;
use dyn_hash::DynHash;
use downcast_rs::{impl_downcast, Downcast};

use crate::{sealed::SealedAll, types::StatComponents, Data, Shareable, TYPE_ERROR};

/// A flags like [`Qualifier`] for stats, normally bitflags or a set.
///
/// An application should idealy implement one [`QualifierFlag`] and multiple [`Stat`]s,
/// since different types of stats can still interop if they use the same [`QualifierFlag`].
pub trait QualifierFlag: BitOr<Self, Output=Self> + Ord + Hash + Shareable {
    fn contains(&self, other: &Self) -> bool;
    fn intersects(&self, other: &Self) -> bool;
    fn is_none_or_intersects(&self, other: &Self) -> bool {
        self.is_none() || self.intersects(other)
    }
    fn set_equals(&self, other: &Self) -> bool;
    fn none() -> Self;
    fn is_none(&self) -> bool;
}

impl<T> QualifierFlag for T where T: BitOr<Self, Output=Self> + Ord + Hash + BitAnd<Self, Output = Self> + Default + Shareable + Copy{
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct Qualifier<Q: QualifierFlag> {
    pub any_of: Q,
    pub all_of: Q,
}

impl<Q: QualifierFlag> Default for Qualifier<Q> {
    fn default() -> Self {
        Self {
            any_of: Q::none(),
            all_of: Q::none(),
        }
    }
}

pub trait IntoQualifierQuery<Q: QualifierFlag> {
    fn into_flags(self) -> QualifierQuery<Q>;
    fn to_flags_ref(&self) -> Cow<'_, QualifierQuery<Q>>;
}

impl<Q: QualifierFlag> IntoQualifierQuery<Q> for Qualifier<Q> {
    fn into_flags(self) -> QualifierQuery<Q> {
        QualifierQuery::Specific {
            any_of: self.any_of,
            all_of: self.all_of,
            some_of: Q::none()
        }
    }
    fn to_flags_ref(&self) -> Cow<'_, QualifierQuery<Q>>{
        Cow::Owned(self.clone().into_flags())
    }
}

impl<Q: QualifierFlag> IntoQualifierQuery<Q> for Q {
    fn into_flags(self) -> QualifierQuery<Q> {
        QualifierQuery::Aggregate(self)
    }
    fn to_flags_ref(&self) -> Cow<'_, QualifierQuery<Q>>{
        Cow::Owned(self.clone().into_flags())
    }
}

impl<Q: QualifierFlag> IntoQualifierQuery<Q> for QualifierQuery<Q> {
    fn into_flags(self) -> QualifierQuery<Q> {
        self
    }
    fn to_flags_ref(&self) -> Cow<'_, QualifierQuery<Q>>{
        Cow::Borrowed(self)
    }
}

impl<Q: QualifierFlag> Qualifier<Q> {

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
    ///     since left hand side is `all_of`, right hand side is `any_of`.
    pub fn qualifies_as(&self, queried: &impl IntoQualifierQuery<Q>) -> bool {
        let queried = queried.to_flags_ref();
        match queried.as_ref() {
            QualifierQuery::Aggregate(some_of) => {
                some_of.contains(&self.all_of) &&
                self.any_of.is_none_or_intersects(some_of)
            },
            QualifierQuery::Specific { any_of, all_of, some_of } => {
                self.any_of.contains(any_of) &&
                self.all_of.contains(all_of) &&
                some_of.contains(&self.all_of) &&
                self.any_of.is_none_or_intersects(some_of)
            },
        }
    }
}


/// Query version of [`Qualifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum QualifierQuery<Q: QualifierFlag> {
    /// Look for qualifier that qualifies as this.
    ///
    /// Queried `any_of` intersects this (or is none) and this contains Queried `all_of`.
    Aggregate(Q),
    /// Look for qualifiers that are this and deny more generalized qualifiers.
    Specific {
        /// Queried `any_of` contains this.
        any_of: Q,
        /// Queried `all_of` contains this.
        all_of: Q,
        /// Same constraint as `Aggregate`.
        some_of: Q,
    }
}

impl<Q: QualifierFlag> Default for QualifierQuery<Q> {
    fn default() -> Self {
        Self::Aggregate(Q::none())
    }
}

/// Implement this on your types to qualify them as a [`Stat`].
///
/// Similar to bevy's labels, you can either use one instance per stat,
/// or use one type per [`StatComponents`].
///
/// # Example
/// ```
/// struct Attack;
/// struct Defense;
/// impl Stat for Attack { .. }
/// impl Stat for Defense { .. }
/// ```
/// or
/// ```
/// enum MyStat{
///     Attack,
///     Defense
/// }
/// impl Stat for MyStat { .. }
/// ```
pub trait Stat: Any + Clone + Hash + Debug + Eq + Ord + Send + Sync + 'static {
    type Data: StatComponents;

    /// Equality comparison between all stat implementors.
    fn is<S: Stat + SealedAll>(&self, other: &S) -> bool{
        self as &dyn DynStat == other as &dyn DynStat
    }

    /// If a generic stat is a concrete stat, cast associated `Data`
    /// as the concrete stat's associated `Data`.
    fn is_then<S: Stat + SealedAll>(&self,
        other: &S,
        data: &Self::Data,
        f: impl FnOnce(&S::Data)
    ) -> bool {
        if self as &dyn DynStat == other as &dyn DynStat {
            f(data.as_any().downcast_ref().expect(TYPE_ERROR));
            true
        } else {
            false
        }
    }

    /// If a generic stat is a concrete stat, cast associated `Data`
    /// as the concrete stat's associated `Data`.
    fn is_then_mut<S: Stat + SealedAll>(&self,
        other: &S,
        mut_ref: &mut Self::Data,
        f: impl FnOnce(&mut S::Data)
    ) -> bool {
        if self as &dyn DynStat == other as &dyn DynStat {
            f(mut_ref.as_any_mut().downcast_mut().expect(TYPE_ERROR));
            true
        } else {
            false
        }
    }
}

#[macro_export]
macro_rules! match_stat {
    ($stat: expr, ref $mut_ref: expr => {
        $(,)?
    }) => {
        ()
    };
    ($stat: expr, mut $mut_ref: expr => {
        $(,)?
    }) => {
        ()
    };
    ($stat: expr, ref $mut_ref: expr => {
        $first_arm: expr => $first_out: expr
        $(,$arm: expr => $out: expr)* $(,)?
    }) => {
        if !$stat.is_then(&$first_arm, $mut_ref, $first_out) {
            $crate::match_stat!($stat, ref $mut_ref => {$($arm => $out),*})
        }
    };
    ($stat: expr, mut $mut_ref: expr => {
        $first_arm: expr => $first_out: expr
        $(,$arm: expr => $out: expr)* $(,)?
    }) => {
        if !$stat.is_then_mut(&$first_arm, $mut_ref, $first_out) {
            $crate::match_stat!($stat, mut $mut_ref => {$($arm => $out),*})
        }
    };

}

/// Object safe version of [`Stat`].
pub(crate) trait DynStat: Downcast + DynHash + Debug + Send + Sync {
    fn type_id(&self) -> TypeId;
    fn dyn_eq(&self, other: &dyn DynStat) -> bool;
    fn dyn_ord(&self, other: &dyn DynStat) -> Ordering;
    fn boxed_clone(&self) -> Box<dyn DynStat>;
    fn default_value(&self) -> Box<dyn Data>;
    fn compose_stat(&self, from: &mut dyn Data, with: &dyn Data);
}

impl_downcast!(DynStat);
dyn_hash::hash_trait_object!(DynStat);

impl PartialEq for dyn DynStat {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl<S: DynStat> PartialEq<S> for Box<dyn DynStat>  {
    fn eq(&self, other: &S) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn DynStat {}

impl PartialOrd for dyn DynStat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for dyn DynStat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dyn_ord(other)
    }
}

impl Clone for Box<dyn DynStat> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl<T> From<T> for Box<dyn DynStat> where T: Stat {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl<T> DynStat for T where T:Stat {
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn dyn_eq(&self, other: &dyn DynStat) -> bool {
        other.downcast_ref::<Self>()
            .map(|x| x == self)
            .unwrap_or(false)
    }

    fn dyn_ord(&self, other: &dyn DynStat) -> Ordering {
        other.downcast_ref::<Self>()
            .map(|x| x.cmp(self))
            .unwrap_or(self.type_id().cmp(&DynStat::type_id(other)))
    }

    fn boxed_clone(&self) -> Box<dyn DynStat> {
        Box::new(self.clone())
    }

    fn default_value(&self) -> Box<dyn Data> {
        Box::<<T as Stat>::Data>::default()
    }

    fn compose_stat(&self, from: &mut dyn Data, with: &dyn Data) {
        let from = from.downcast_mut::<T::Data>().expect("Wrong data type in compose.");
        let with = with.downcast_ref::<T::Data>().expect("Wrong data type in compose.");
        from.join(with.clone());
    }
}
