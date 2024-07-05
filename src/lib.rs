#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//! Blazing fast and versatile RPG stat system for the bevy engine.
//!
//! # Qualified Stats
//!
//! We describe each stat as a [`Qualifier`] and a [`Stat`].
//! `Stat` is a concrete stat noun like "Strength", "Magic", etc.
//! `Qualifier` is a flags based adjective that describes
//! what this `Stat` can be applied to.
//!
//! For example in "FireMagicDamage", "Fire|Magic" is the qualifier,
//! "Damage" is the `Stat`.
//!
//! What this means if an effect boosts "Fire|Damage", "Magic|Damage",
//! or simply just "Damage", the effect will be applied to the stat,
//! but an effect on "Sword|Damage" or "Fire|Range" won't be affecting the stat.
//!
//! ## [Qualifier]
//!
//! `Qualifier` additionally provides `any_of` for modelling conditional effects like
//! "Elemental|Damage", which matches "Fire or Water or Wind Damage"
//! instead of "Fire and Water and Wind Damage".
//! Each [`Qualifier`] can only have one group of `any_of`.
//!
//! ### Examples
//!
//! ```
//! let fire = Qualifier::all_of(Flag::Fire);
//! let fire_magic = Qualifier::all_of(Flag::Fire|Flag::Magic);
//! let elemental = Qualifier::any_of(Fire|Water|Air|Earth);
//! let elemental_magic = Qualifier::any_of(Fire|Water|Air|Earth)
//!     .and_all_of(Magic);
//! ```
//!
//! ## [QualifierQuery]
//!
//! `QualifierQuery` matches all `Qualifiers` on our entity that
//! qualifies as the query we are looking for.
//!
//! [`QualifierQuery::Aggregate`] collects all qualifiers that matches the query.
//!
//! For example, suppose we are looking for `(Frost|Piercing|Magic, Damage)`:
//! * `((), Damage)` qualifies.
//! * `(Frost, Damage)` qualifies.
//! * `(Frost|Magic, Damage)` qualifies.
//! * `(Frost|Piercing|Magic, Damage)` qualifies.
//! * `(Elemental, Damage)` qualifies.
//! * `(Frost|Sword, Damage)` does not qualify.
//! * `(Fire|Piercing|Magic, Defense)` does not qualify.
//!
//! [`QualifierQuery::Exact`] allows us to deny
//! more generalized qualifiers.
//!
//! For example, in order to model a statement like so:
//!
//! ```text
//! Add 50% of the character's magic damage to physical damage.
//! ```
//!
//! Querying `(Magic, Damage)`, which contains `((), Damage)`,
//! and adding to `(Physical, Damage)` would cause a duplication.
//!
//! Therefore the query should be:
//!
//! ```
//! QualifierQuery::Exact {
//!     any_of: None,
//!     all_of: Magic,
//! }
//! ```
//!
//! ## [Stat]
//!
//! An app usually has a single [`QualifierFlag`] but multiple [`Stat`] implementors. This is because
//! each [`Stat`] can associate to a different type. For example `strength` and `magic` can be a `i32`,
//! `hp` can be a `f32`, `is_dragon` can be a `bool` etc. `Stat`s are usually enums and you might find
//! the `strum` crate useful in implementing them.
//!
//! # Unordered StatStream
//!
//! `bevy_stat_query` uses unordered operations to build up stats. This includes
//! `add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
//! ever needed when querying for stats.
//!
//! Each stat has its components form [`StatValue`], e.g. `(12 * 4).min(99).max(0)`,
//! and its evaluated form, e.g. `48`. You can implement your own `StatValue`
//! to achieve custom behaviors.
//!
//! # Queries
//!
//! [`StatQuery`] is the [`SystemParam`] to query stats. `StatQuery` only collects [`StatEntity`]s, which are
//! marker components for queryable entities. To actually query for stats, you need to join it with
//! [`ComponentStream`]s and [`RelationStream`]s. They can query stats from components and children of
//! the `Entity`.
//!
//! ## Relations
//!
//! All [`StatStream`]s have access to a [`Querier`], which can query for other stats
//! from any entity in the world. In addition, [`RelationStream`] allows the stat system to
//! query for relationship between entities, for example to model an aura effect base on distance.
//!
//! # [StatMap]
//!
//! `StatMap` is a optimized map like storage for all stats that implements [`StatStream`].
//!
//! ## Serialization
//!
//! Due to the type of dynamic dispatch used by [`StatMap`], we only have native serialization support
//! via `bevy_serde_lens`.
//! Call [`StatExtension::register_stat`] on the world for each [`Stat`] used in deserialization.
//! 
//! To use `Reflect` deserialization you must wrap your deserialization inside
//! a [`bevy_serde_lens_core::private::de_scope`] scope.
//!
//! # [StatCache]
//!
//! A resource that must be manually added.
//! If added, will cache all query results.
//! If state has changed, must be manually cleared
//! either via [`StatQuery`] or directly on the resource.
//!
//! # [GlobalStatDefaults]
//!
//! A resource that contains default values of stats. If
//! you want to constrain `HP` to `0..=99` it should be done here.
//!
//! Extension methods exists on the `App` like [`StatExtension::register_stat_max`] to
//! set default values of stats.
//!
//! # [GlobalStatRelations]
//!
//! A resource that contains [`StatStream`]s that runs on all queries.
//!
//! Extension method [`StatExtension::register_stat_relation`] on `App` can be used to
//! register these.
#[allow(unused)]
use bevy_ecs::{component::Component, query::QueryData, system::SystemParam};

pub(crate) static TYPE_ERROR: &str = "Error: a stat does not have the appropriate type. \
This is almost certainly a bug since we do not provide a type erased api.";

#[doc(hidden)]
pub use bevy_app::{App, Plugin};

mod num_traits;
pub use num_traits::{Flags, Float, Fraction, Int};
mod stream;
pub use stream::*;
mod querier;
pub use querier::*;
mod qualifier;
pub mod types;
pub use qualifier::{Qualifier, QualifierFlag, QualifierQuery};
mod stat;
pub(crate) use stat::StatExt;
pub(crate) use stat::StatInst;
pub use stat::{Stat, StatVTable, StatValuePair};
pub mod operations;
pub use operations::StatValue;
mod cache;
pub use cache::StatCache;
mod plugin;
pub use plugin::{GlobalStatDefaults, GlobalStatRelations, StatDeserializers, StatExtension};
mod stat_map;
pub use stat_map::StatMap;
mod buffer;
pub mod rounding;
use std::fmt::Debug;

mod sealed {
    pub trait Sealed {}

    impl<T: ?Sized> Sealed for T {}
}

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Shareable: Clone + Debug + Send + Sync + 'static {}
impl<T> Shareable for T where T: Clone + Debug + Send + Sync + 'static {}

/// Construct a reference to a static [`StatVTable`] with serialization support.
/// ```
/// vtable!(Type);
/// ```
/// Equivalent to
/// ```
/// {
///     static VTABLE: StatVTable<Type> = StatVTable::of::<Type>();
///     &VTABLE
/// }
/// ```
#[macro_export]
macro_rules! vtable {
    ($ty: ty) => {{
        static _VTABLE: $crate::StatVTable<$ty> = $crate::StatVTable::of::<$ty>();
        &_VTABLE
    }};
}

/// Downcast [`StatValuePair`] to a concrete pair of stat and value.
///
/// # Syntax
///
/// ```
/// match_stat!(stat_value_pair => {
///     // if stat is `MyStat::A`, downcast the value to `MyStat::Value` as `value`.
///     (MyStat::A, v) => {
///         value.add(1);
///     },
///     // if stat is `MyStat`, downcast the stat as `stat` and the value as `value`.
///     (stat @ MyStat, value) => {
///         value.add(*v as i32);
///     },
/// }
/// ```
#[macro_export]
macro_rules! match_stat {
    ($stat_value: expr => {($ident: ident @ $ty: ty, $value: pat) => $expr: expr $(, $($tt: tt)*)?}) => {
        if let Some(($ident, $value)) = $stat_value.cast::<$ty>() {
            $expr
        } $(
            else {
                $crate::match_stat!($stat_value => {$($tt)*})
            }
        )?
    };
    ($stat_value: expr => {($is: expr, $value: pat) => $expr: expr $(, $($tt: tt)*)?}) => {
        if let Some($value) = $stat_value.is_then_cast(&$is) {
            $expr
        } $(
            else {
                $crate::match_stat!($stat_value => {$($tt)*})
            }
        )?
    };
    ($stat_value: expr => {_ => $expr: expr $(,)?}) => {
        $expr
    };
    // Matches the last comma case.
    ($stat_value: expr => {}) => {()};
}

use buffer::{validate, Buffer};

#[cfg(test)]
mod test {
    use bevy_ecs::component::Component;
    use num_enum::{FromPrimitive, IntoPrimitive};
    use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

    use crate::{
        stat::StatValuePair,
        types::{StatFlags, StatIntPercentAdditive},
        ComponentStream, Querier, Stat, StatValue,
    };

    #[derive(Component)]
    pub struct X;

    #[derive(Debug, Clone, Copy, IntoStaticStr, EnumIter, FromPrimitive, IntoPrimitive)]
    #[repr(u64)]
    pub enum IntStat {
        #[default]
        A,
        B,
        C,
        D,
    }

    impl Stat for IntStat {
        type Value = StatIntPercentAdditive<i32>;

        fn name(&self) -> &'static str {
            self.into()
        }

        fn vtable() -> &'static crate::StatVTable<Self> {
            vtable!(IntStat)
        }

        fn as_index(&self) -> u64 {
            (*self).into()
        }

        fn from_index(index: u64) -> Self {
            index.into()
        }

        fn values() -> impl IntoIterator<Item = Self> {
            IntStat::iter()
        }
    }

    #[derive(Debug, Clone, Copy, IntoStaticStr, EnumIter, FromPrimitive, IntoPrimitive)]
    #[repr(u64)]
    pub enum FlagsStat {
        #[default]
        E,
        F,
        G,
        H,
    }

    impl Stat for FlagsStat {
        type Value = StatFlags<i32>;

        fn name(&self) -> &'static str {
            self.into()
        }

        fn vtable() -> &'static crate::StatVTable<Self> {
            vtable!(FlagsStat)
        }

        fn as_index(&self) -> u64 {
            (*self).into()
        }

        fn from_index(index: u64) -> Self {
            index.into()
        }

        fn values() -> impl IntoIterator<Item = Self> {
            FlagsStat::iter()
        }
    }

    impl ComponentStream<u32> for &X {
        type Cx = ();

        fn stream(
            _: bevy::prelude::Entity,
            _: &<Self::Cx as bevy_ecs::system::SystemParam>::Item<'_, '_>,
            _: <Self::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
            _: &crate::QualifierQuery<u32>,
            stat_value: &mut StatValuePair,
            _: Querier<u32>,
        ) {
            match_stat!(
                stat_value => {
                    (IntStat::A, value) => {
                        value.add(1);
                    },
                    (IntStat::B, value) => {
                        value.add(2);
                    },
                    (v @ IntStat, value) => {
                        value.add(v as i32);
                    },
                    (FlagsStat::E, value) => {
                        value.or(1);
                    },
                    (FlagsStat::F, value) => {
                        value.not(2);
                    },
                    (v @ FlagsStat, value) => {
                        value.or(v as i32);
                    },
                }
            )
        }
    }
}
