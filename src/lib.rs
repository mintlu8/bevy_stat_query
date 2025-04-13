#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![doc = include_str!("../README.md")]
#[allow(unused)]
use bevy_ecs::{component::Component, query::QueryData, system::SystemParam};

pub(crate) static TYPE_ERROR: &str = "Error: a stat does not have the appropriate type. \
This is almost certainly a bug since we do not provide a type erased api.";

#[doc(hidden)]
pub use bevy_app::{App, Plugin};

mod fraction;
mod num_traits;
pub use fraction::Fraction;
pub use num_traits::{Flags, Float, Int, NumCast};
mod stream;
pub use stream::*;
mod querier;
pub use querier::*;
mod qualifier;
pub mod types;
pub use qualifier::{Qualifier, QualifierFlag, QualifierQuery};
mod stat;
#[cfg(feature = "derive")]
pub use bevy_stat_query_derive::{Attribute, Stat};
pub(crate) use stat::StatExt;
pub(crate) use stat::StatInst;
pub use stat::{Stat, StatVTable, StatValuePair};
pub mod operations;
pub use operations::StatValue;
mod plugin;
pub use plugin::{GlobalStatDefaults, GlobalStatRelations, StatDeserializers, StatExtension, STAT_DESERIALIZERS};
mod stat_map;
pub use stat_map::StatMap;
mod buffer;
pub mod rounding;
use std::fmt::Debug;
mod attribute;
pub use attribute::Attribute;
mod cowstr;

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
        #[used]
        static _VTABLE: $crate::StatVTable<$ty> = $crate::StatVTable::of::<$ty>();
        &_VTABLE
    }};
}

/// Downcast [`StatValuePair`] to a concrete pair of stat and value.
///
/// # Syntax
///
/// ```
/// # /*
/// match_stat!(stat_value_pair => {
///     // if stat is `MyStat::A`, downcast the value to `MyStat::Value` as `value`.
///     (MyStat::A, value) => {
///         value.add(1);
///     },
///     // if stat is `MyStat`, downcast the stat as `stat` and the value as `value`.
///     (stat @ MyStat, value) => {
///         value.add(1);
///     },
/// }
/// # */
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
        Querier, Stat, StatStream, StatValue,
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
                        value.add(v as i32);
                    },
                    (FlagsStat::E, value) => {
                        value.or(1);
                    },
                    (v @ FlagsStat, value) => {
                        value.or(v as i32);
                    },
                }
            }
        }
    }
}
