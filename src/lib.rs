#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(rustdoc::invalid_rust_codeblocks)]
#![doc = include_str!("../README.md")]
#[allow(unused)]
use bevy_ecs::{component::Component, query::QueryData, system::SystemParam};

#[doc(hidden)]
pub use bevy_app::{App, Plugin};

mod fraction;
mod num_traits;
pub use fraction::Fraction;
pub use num_traits::{Flags, Float, Int, NumCast, Number};
mod stream;
pub use stream::*;
mod querier;
pub use querier::*;
mod qualifier;
#[cfg(feature = "serde")]
mod serde_map;
#[doc(hidden)]
#[cfg(feature = "serde")]
pub use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DError};
#[cfg(feature = "serde")]
pub use serde_map::{DeserializeEntry, SerializeEntry};
pub mod types;
pub use qualifier::{Qualifier, QualifierConstraint, QualifierItem, QualifierKey, QualifierQuery};
mod stat;
#[cfg(feature = "derive")]
pub use bevy_stat_query_derive::{Attribute, Stat, StatDispatch};
pub use stat::{Stat, StatUid, StatValuePair};
pub mod operations;
pub use operations::StatValue;
mod plugin;
pub use plugin::{GlobalStatDefaults, GlobalStatRelations, StatExtension};
mod stat_map;
pub use stat_map::{StatDispatch, StatDispatchTo, StatMapBase};
/// Standard [`StatMapBase`] with [`QualifierItem`] as its [`QualifierKey`]
pub type StatMap<Q, S> = StatMapBase<QualifierItem<Q>, S>;
/// Standard [`StatMapBase`] with [`QualifierItem`] as its [`QualifierKey`]
pub type SimpleStatMap<Q, S> = StatMapBase<PhantomData<Q>, S>;
pub mod rounding;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
};
mod attribute;
pub use attribute::{AsAttribute, Attribute};

/// Alias for `Clone + Debug + Send + Sync + 'static`.
pub trait Shareable: Clone + Debug + Send + Sync + 'static {}

impl<T> Shareable for T where T: Clone + Debug + Send + Sync + 'static {}

/// Dyn compatible version for `Clone + Debug + Send + Sync + Any`, with support for downcast and dynamic clone.
pub trait ShareableAny: Debug + Send + Sync + 'static {
    fn type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any_boxed(self: Box<Self>) -> Box<dyn Any>;
    #[doc(hidden)]
    fn clone_boxed(&self) -> Box<dyn ShareableAny>;
}

impl<T> ShareableAny for T
where
    T: Clone + Debug + Send + Sync + 'static,
{
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_boxed(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn clone_boxed(&self) -> Box<dyn ShareableAny> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ShareableAny> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

impl dyn ShareableAny {
    pub fn downcast_ref<T: Shareable>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }
    pub fn downcast_mut<T: Shareable>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
    pub fn unbox<T: Shareable>(self: Box<Self>) -> Option<T> {
        self.as_any_boxed().downcast().map(|x| *x).ok()
    }
}

/// Downcast [`StatValuePair`] to a concrete pair of stat and value.
///
/// # Syntax
///
/// ```
/// # use bevy_stat_query::{StatValuePair, Stat, types::StatRounded, match_stat, StatValue};
/// # #[derive(Debug, Clone, Stat)]
/// # #[stat(value = "StatRounded<i32, f32>")]
/// # pub enum MyStat {A, B};
/// # fn f(stat_value_pair: &mut StatValuePair) {
/// match_stat!(stat_value_pair => {
///     // if stat is `MyStat::A`, downcast the value to `MyStat::Value` as `value`.
///     (MyStat::A, value) => {
///         value.add(1);
///     },
///     // if stat is `MyStat`, downcast the stat as `stat` and the value as `value`.
///     (stat @ MyStat, value) => {
///         value.add(1);
///     },
/// })
/// # }
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

#[cfg(test)]
mod test {
    use bevy_ecs::component::Component;
    use num_enum::{FromPrimitive, IntoPrimitive};
    use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

    use crate::{
        Querier, Stat, StatStream, StatValue,
        stat::StatValuePair,
        types::{StatFlags, StatIntPercentAdditive},
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

    impl StatStream for X {
        type Qualifier = u32;

        fn stream_stat(
            &self,
            _: bevy::prelude::Entity,
            _: &crate::QualifierQuery<Self::Qualifier>,
            stat_value: &mut StatValuePair,
            _: Querier<Self::Qualifier>,
        ) {
            match_stat! {
                stat_value => {
                    (IntStat::A, value) => {
                        value.add(1);
                    },
                    (IntStat::B, value) => {
                        value.add(2);
                    },
                    (v @ IntStat, value) => {
                        value.add((*v) as i32);
                    },
                    (FlagsStat::E, value) => {
                        value.or(1);
                    },
                    (v @ FlagsStat, value) => {
                        value.or((*v) as i32);
                    },
                }
            }
        }
    }
}
