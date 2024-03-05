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
//! What this means if an effect boosts `Fire Damage`, `Magic Damage`,
//! or simply just `Damage`, the effect will be applied to the stat,
//! but an effect on `Sword Damage` or `Fire Range` won't be applied to the stat.
//!
//! ## Qualifier
//! 
//! [`Qualifier`] is tied to effects, and provides the aforementioned `all_of`,
//! and in addition `any_of`, useful for modelling conditional effects like
//! `Elemental|Damage`, which means `Fire or Water Damage` instead of `Fire and Water Damage`.
//! 
//! Each [`Qualifier`] can only have one group of `any_of` which is a limitation currently.
//! 
//! # Examples
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
//! Add marker component [`StatEntity`] to an `Entity`.
//! If you need caching, add a [`StatCache`] as well.
//! You need to manually clear the cache when state is changed.
//!
//! Let's refer to an `Entity` that can be queried as a `Unit`.
//!
//! [`StatMap`] can be used as base stats for the `Unit`.
//! To add behaviors beyond base stats,
//! we need to implement [`ComponentStream`], which is a [`QueryData`] with some external context.
//! [`StatStream`] implementors can be attached to **children** of the
//! [`StatEntity`] (not on `StatEntity` itself) to take effect.
//!
//! # Unordered Stat Stream
//!
//! `bevy_stat_engine` uses unordered operations to build up stats. This includes
//! `add`, `multiply`, `min`, `max` and `or`. This ensures no explicit ordering is
//! ever needed when querying for stats.
//!
//! Each stat has its components form [`StatValue`], e.g. `(12 * 4).min(99).max(0)`,
//! and its evaluated form, e.g. `48`. You can implement your own `StatValue`
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
//! By implementing [`IntrinsicStream`] for a [`Stat`] we can look for stats
//! directly on components on the `Unit` entity. Additionally intrinsics
//! can be used to obtain `distance` or other bi-unit relationship 
//! for the stat system. This can be used to model aura effects.
//!
//! # Querier
//!
//! The [`StatQuerier`] is the [`SystemParam`] to query stats, it is quite difficult to
//! define one manually so the recommended way is to define a `type` with the
//! [`querier!`] macro. Additionally you can also use the [`StatExtension`] with `World` access.
//!
//! [`StatQuerier`] requires read access to all components in stat system so you cannot mutate
//! anything while having it as a parameter.
//! Using some kind of deferred command queue for mutations might be advisable in this case.
#[allow(unused)]
use bevy_ecs::{query::QueryData, component::Component, system::SystemParam};

pub(crate) static TYPE_ERROR: &str = "Error: a stat does not have the approprate type. \
This is almost certainly a bug since we do not provide a type erased api.";

#[doc(hidden)]
pub use bevy_app::{Plugin, App};

use bevy_reflect::TypePath;
use bevy_serde_project::typetagged::{BevyTypeTagged, FromTypeTagged};
use downcast_rs::Downcast;
mod stream;
use dyn_clone::{clone_trait_object, DynClone};
use serde::{de::DeserializeOwned, Serialize};
pub use stream::StatValuePair;
mod num_traits;
pub use num_traits::{Int, Float, Flags, Fraction};
pub use stream::{ComponentStream, IntrinsicStream, StatStream, StatStreamObject, StatelessStream};
pub mod types;
pub use types::StatValue;
mod qualifier;
pub use qualifier::{Qualifier, QualifierFlag, QualifierQuery};
mod stat;
pub use stat::Stat;
pub(crate) use stat::{StatInstances, DynStat};
mod calc;
pub use calc::{StatOperation, StatDefaults};
mod entity;
pub use entity::{StatCache, StatEntity};
pub mod rounding;
mod plugin;
pub use plugin::StatExtension;
mod querier;
pub use querier::{StatQuerier, hints, QuerierRef};
mod param;
#[doc(hidden)]
pub use param::{ChildStatParam, StatParam};
mod stat_map;
pub use stat_map::{BaseStatMap, FullStatMap, Unqualified, StatOperationsMap};

use std::fmt::Debug;

mod sealed {
    pub struct SealToken;

    pub trait Sealed {}

    pub trait SealedAll {}

    impl<T: ?Sized> SealedAll for T {}
}

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Shareable: Clone + Debug + Send + Sync + 'static {}
impl<T> Shareable for T where T: Clone + Debug + Send + Sync + 'static {}

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Serializable: Clone + Debug + Send + Sync + Serialize + DeserializeOwned + TypePath + 'static {}
impl<T> Serializable for T where T: Clone + Debug + Send + Sync + Sync + Serialize + DeserializeOwned + TypePath + 'static {}

/// [`Any`](std::any::Any) that implements [`Send`], [`Sync`], [`Debug`] and [`Clone`].
pub(crate) trait Data: Send + Sync + Downcast + Debug + DynClone {
    fn name(&self) -> &'static str;
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<T> Data for T where T: Shareable + TypePath + serde::Serialize {
    fn name(&self) -> &'static str {
        T::short_type_path()
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

clone_trait_object!(Data);

impl BevyTypeTagged for Box<dyn Data> {
    fn name(&self) -> impl AsRef<str> {
        self.as_ref().name()
    }

    fn as_serialize(&self) -> &dyn bevy_reflect::erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}


impl<T: Serializable> FromTypeTagged<T> for Box<dyn Data> {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Box::new(item)
    }
}



downcast_rs::impl_downcast!(Data);
