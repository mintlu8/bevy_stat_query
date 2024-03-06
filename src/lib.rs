#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
//! An over-engineered RPG stat system for the bevy engine.
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
//! * What do you mean? My `DarkFire` and `Fire` are totally different things and should be independent.
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
//! and its evaluated form, e.g. `48`. [`StatOperation`] stores a single operation
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
//! * [`StatQuerier`] requires read access to all components in the stat system so we cannot mutate
//! anything while having it as a parameter.
//! Using system piping or some kind of deferred command queue for mutations 
//! might be advisable in this case.
//! 
//! * The crate heavily utilizes dynamic dispatch under the hood, and is therefore
//! not fully reflect compatible. The supported serialization method is
//! through the [`bevy_serde_project`] crate, Check out that crate for more information.
#[allow(unused)]
use bevy_ecs::{query::QueryData, component::Component, system::SystemParam};

pub(crate) static TYPE_ERROR: &str = "Error: a stat does not have the appropriate type. \
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
pub use stream::{ExternalStream, IntrinsicStream, StatStream, StatStreamObject, StatExtend};
pub mod types;
pub use types::StatValue;
mod qualifier;
pub use qualifier::{Qualifier, QualifierFlags, QualifierQuery};
mod stat;
pub use stat::Stat;
pub(crate) use stat::{StatInstances, DynStat};
mod calc;
pub use calc::{StatOperation, StatDefaults, StatDefaultRelations};
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
    pub trait Sealed {}

    impl<T: ?Sized> Sealed for T {}
}

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Shareable: Clone + Debug + Send + Sync + 'static {}
impl<T> Shareable for T where T: Clone + Debug + Send + Sync + 'static {}

/// Alias for `Shareable + Serialize + DeserializeOwned + TypePath`.
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
