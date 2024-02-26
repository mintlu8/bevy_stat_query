#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//! An over-engineered RPG stat query system for the bevy engine.
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
//! # [`Qualifier`] and [`QualifierQuery`]
//!
//! [`Qualifier`] attached to stat modifiers on characters,
//! while [`QualifierQuery`] is used to search all `Qualifier`s
//! that matches its description.
//!
//! ## Qualifier
//!
//! Qualifier has two fields: `any_of` and `all_of`.
//!
//! `all_of` is good for most cases.
//!
//! ```
//! let fire = Qualifier::all_of(Flag::Fire);
//! let fire_magic = Qualifier::all_of(Flag::Fire|Flag::Magic);
//! ```
//!
//! Sometimes we want to match if one of a group of flags exists, `any_of` can help.
//!
//! ```
//! let elemental = Qualifier::any_of(Fire|Water|Air|Earth);
//! let elemental_magic = Qualifier::any_of(Fire|Water|Air|Earth)
//!     .and_all_of(Magic);
//! ```
//!
//! Note: matching multiple groups of `all_of` is not supported.
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
//! * `(Elemental, Damage)` qualifies.
//! * `(Fire|Sword, Damage)` does not qualify.
//! * `(Fire|Burn|Magic, Defense)` does not qualify.
//!
//! [`QualifierQuery::Specific`] allows you to deny
//! more generalized qualifiers that qualifies as this.
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
//! QualifierQuery::Specific {
//!     any_of: None,
//!     all_of: Magic,
//!     some_of: Magic,
//! }
//! ```
//!
//! # Getting Started
//!
//! Add [`StatEntity`] to an `Entity` that has stats.
//! If you need caching, add a [`StatCache`] as well.
//! You need to manually clear the cache when state is changed.
//!
//! Let's refer to an `Entity` that can be queired as a `Unit`.

//! [`StatMap`] can be used as base stats for the `Unit`.
//! To add behaviors beyond base stats,
//! we need to implement [`StatStream`], which is a [`QueryData`] with some external context.
//! If you don't know what [`QueryData`] is,
//! think either a [`Component`] or a group of components on a single Entity.
//! [`StatStream`] implementors can be attached to **children** of the
//! [`StatEntity`] (not on `StatEntity` itself) to take effect.
//!
//! # Unordered Stat Stream
//!
//! `bevy_stat_engine` uses unordered operations to build up stats. This includes
//! `add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
//! ever needed when querying for stats.
//!
//! Each stat has its components form [`StatComponents`], e.g. `(12 * 4).min(99).max(0)`,
//! and its evaluated form, e.g. `48`. You can implement your own `StatComponents`
//! to achieve custom behaviors.
//!
//! Additionally you can create relations between different
//! stats using either their components form or their evaluated form.
//! [`StatStream`]s are allowed to query other stats or stats on other entities.
//! Since stat operations are unordered, dependency cycles cannot be resolved.
//! If a cycle is detected, an error will be thrown.
//!
//! # Intrinsics
//!
//! The stat system can obtain intrinsic information on the `Unit`.
//! By implementing [`FromIntrinsics`] for a [`Stat`] we can obatin data
//! not found in that stat system directly from the `Unit` entity.
//! This is crucial to obtain distance and other relationships between units.
//! The type of intrinsic can be passed to the [`Querier`] in the [`querier!`] macro.
//!
//! # Querier
//!
//! The [`Querier`] is the [`SystemParam`] to query stats, it is quite difficult to
//! define one manually so the recommended way is to define a `type` with the
//! [`querier!`] macro. Additionally you can also use the [`Querier`] with [`World`] access.
//!
//! [`Querier`] requires read access to all components in stat system so you cannot mutate
//! anything while having it as a parameter.
//! Using some kind of deferred command queue for mutations might be advisable in this case.

#[allow(unused)]
use bevy_ecs::{query::QueryData, component::Component, system::SystemParam, world::World};

pub(crate) static TYPE_ERROR: &str = "Error: a stat does not have the approprate type. \
This is almost certainly a bug since we do not provide a type erased api.";

use downcast_rs::Downcast;
mod stream;
mod num_traits;
pub use num_traits::{Int, Float, Flags};
pub use num_rational::Ratio;
pub use stream::{StatStream, FromIntrinsics, StatQuerier};
pub mod types;
pub use types::StatComponents;
mod traits;
pub use traits::{Qualifier, QualifierFlag, QualifierQuery, Stat};
use traits::DynStat;
mod calc;
pub use calc::{StatOperation, StatDefaults, DefaultStatLogic};
mod entity;
pub use entity::{StatCache, StatEntity};
pub mod rounding;
mod plugin;
pub use plugin::{StatEnginePlugin, StatExtension};
mod querier;
pub use querier::{Querier, hints};
mod param;
#[doc(hidden)]
pub use param::{ChildStatParam, StatParam};
mod stat_map;
pub use stat_map::{StatMapInner, Unqualified, StatOperationsMap};
mod reflect;

use std::fmt::Debug;

mod sealed {
    pub trait Sealed {}

    pub trait SealedAll {}

    impl<T: ?Sized> SealedAll for T {}
}

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Shareable: Clone + Debug + Send + Sync + 'static {}
impl<T> Shareable for T where T: Clone + Debug + Send + Sync + 'static {}

/// [`Any`](std::any::Any) that implements [`Send`], [`Sync`], [`Debug`] and [`Clone`].
pub(crate) trait Data: Send + Sync + Downcast + Debug {
    fn dyn_clone(&self) -> Box<dyn Data>;
}

impl<T> Data for T where T: Send + Sync + Downcast + Debug + Clone{
    fn dyn_clone(&self) -> Box<dyn Data> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Data> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

downcast_rs::impl_downcast!(Data);
