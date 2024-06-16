#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//! A pedantic RPG stat system for the bevy engine.
//!
//! # Qualified Stats
//!
//! We describe each stat as a [`Qualifier`] and a [`Stat`].
//! `Stat` is a concrete stat noun like `Strength`, `Magic`, etc.
//! `Qualifier` is a flags based adjective that describes
//! what this `Stat` can be applied to.
//!
//! For example in `FireMagicDamage`, `Fire|Magic` is the qualifier,
//! `Damage` is the `Stat`.
//!
//! What this means if an effect boosts `Fire|Damage`, `Magic|Damage`,
//! or simply just `Damage`, the effect will be applied to the stat,
//! but an effect on `Sword|Damage` or `Fire|Range` won't be applied to the stat.
//!
//! ## Qualifier
//!
//! [`Qualifier`] is tied to effects, and provides the aforementioned `all_of`.
//! In addition `any_of` is provided for modelling conditional effects like
//! `Elemental|Damage`, which means `Fire or Water Damage` instead of `Fire and Water Damage`.
//!
//! Each [`Qualifier`] can only have one group of `any_of` which is a limitation currently.
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
//! ## QualifierQuery
//!
//! [`QualifierQuery`] matches all `Qualifiers` on our character that
//! qualifies as the query we are looking for.
//!
//! [`QualifierQuery::Aggregate`] collects all qualifiers that matches the query.
//!
//! For example, suppose we are looking for `(Fire|Burn|Magic, Damage)`:
//! * `((), Damage)` qualifies.
//! * `(Fire, Damage)` qualifies.
//! * `(Fire|Magic, Damage)` qualifies.
//! * `(Fire|Burn|Magic, Damage)` qualifies.
//! * `(Elemental, Damage)` qualifies.
//! * `(Fire|Sword, Damage)` does not qualify.
//! * `(Fire|Burn|Magic, Defense)` does not qualify.
//!
//! [`QualifierQuery::Exact`] allows us to deny
//! more generalized qualifiers.
//!
//! For example, in order to model a statement like so:
//!
//! ```js
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
//! * What do you mean? My `DarkFire` and `Fire` and totally different things and should be independent.
//!
//! Create a new qualifier `DarkFire` instead of `Dark`|`Fire`.
//!
//! # Getting Started
//!
//! Add marker component [`StatEntity`] to an `Entity`.
//! If you need caching, add a [`StatCache`] as well.
//! You need to manually clear the cache when the state is changed, however.
//!
//! * Implement [`IntrinsicStream`] to make components on the entity queryable.
//! * Implement [`ExternalStream`] to make components on child entities queryable.
//!
//! For example we can add [`BaseStatMap`] to the `Entity` as base stats, if we include
//! it in the `intrinsic` section of the [`querier!`] macro.
//!
//! # Querier
//!
//! [`StatQuerier`] is the [`SystemParam`] to query stats, it is quite difficult to
//! define one manually so the recommended way is to define a `type` with the
//! [`querier!`] macro. Additionally we can also use the [`StatExtension`] with `World` access
//! for similar functionalities.
//!
//! ## Example
//!
//! ```
//! querier!(pub UnitStatQuerier {
//!     qualifier: MyQualifier,
//!     intrinsic: {
//!         Allegiance,
//!         Position
//!     },
//!     external: {
//!         Weapon,
//!         Ability,
//!         Effect,
//!         Potion,
//!     }
//! });
//! ```
//!
//! # Unordered StatStream
//!
//! `bevy_stat_query` uses unordered operations to build up stats. This includes
//! `add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
//! ever needed when querying for stats.
//!
//! Each stat has its components form [`StatValue`], e.g. `(12 * 4).min(99).max(0)`,
//! and its evaluated form, e.g. `48`. You can implement your own `StatValue`
//! to achieve custom behaviors. [`StatOperation`] stores a single operation
//! that can be written to a [`StatValue`].
//!
//! ## Stat Relation
//!
//! We can create relations between different
//! stats using either their components form or their evaluated form.
//! [`StatStream`]s are allowed to query other stats or other entities.
//! Since stat operations are unordered, dependency cycles cannot be resolved.
//! If a cycle is detected, an error will be thrown.
//!
//! ## Entity Relation
//!
//! [`IntrinsicStream`] can be used to provide bi-entity relationship
//! like `distance` or `allegiance`. This can be used to model range based effects.
//!
//! You may find [`StatOnce`](types::StatOnce) useful in implementing these.
//!
//! # Note
//!
//! * [`StatQuerier`] requires read access to all components in stat system so we cannot mutate
//! anything while having it as a parameter.
//! Using system piping or some kind of deferred command queue for mutations
//! might be advisable in this case.
//!
//! * The crate heavily utilizes dynamic dispatch under the hood, and is therefore
//! not fully reflect compatible. The supported serialization method is
//! through the [`bevy_serde_project`] crate, Check out that crate for more information.
//!
//! * if [`StatValue::Bounds`] is a float, their default values are likely `-inf` and `inf`,
//! which are not valid values in `json`. This means `serde_json` will serialize them as
//! `null` and fail when deserialized.
//! If [`FullStatMap`] is used (optional btw), choose a different format.
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
pub(crate) use stat::StatInst;
pub use stat::{Stat, StatExt, StatVTable};
pub mod operations;
pub use operations::StatValue;
mod cache;
pub use cache::StatCache;
mod plugin;
pub use plugin::StatExtension;
mod stat_map;
pub use stat_map::StatMap;
pub mod rounding;

use std::{
    any::type_name, fmt::Debug, mem::{align_of, size_of, MaybeUninit}
};

mod sealed {
    pub trait Sealed {}

    impl<T: ?Sized> Sealed for T {}
}

type Buffer = [MaybeUninit<u64>; 3];

#[inline(always)]
fn validate<T>() {
    if !matches!(align_of::<T>(), 1 | 2 | 4 | 8) {
        panic!("{} has alignment {}. StatMap can only store values with alignment 1, 2, 4 or 8.", type_name::<T>(), align_of::<T>())
    }
    if size_of::<T>() > 24 {
        panic!("{} has size {}. StatMap can only store values up to 24 bytes.", type_name::<T>(), size_of::<T>())
    }
}

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Shareable: Clone + Debug + Send + Sync + 'static {}
impl<T> Shareable for T where T: Clone + Debug + Send + Sync + 'static {}
