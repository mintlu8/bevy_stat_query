use std::{borrow::Cow, cmp::{Eq, Ord, Ordering}, fmt::Debug, hash::Hash};

use bevy_ecs::system::Resource;
use bevy_serde_project::{Error, FromWorldAccess, SerdeProject};
use downcast_rs::{impl_downcast, Downcast};
use dyn_hash::DynHash;
use rustc_hash::FxHashMap;

use crate::{sealed::SealedAll, Data, Shareable, StatComponents, TYPE_ERROR};


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
pub trait Stat: Shareable + Hash + Debug + Eq + Ord {
    type Data: StatComponents;

    fn name(&self) -> &str;

    fn values() -> impl IntoIterator<Item = Self>;

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
    fn name(&self) -> &str;
    fn dyn_eq(&self, other: &dyn DynStat) -> bool;
    fn dyn_ord(&self, other: &dyn DynStat) -> Ordering;
    fn boxed_clone(&self) -> Box<dyn DynStat>;
    fn default_value(&self) -> Box<dyn Data>;
    fn from_out(&self, out: &dyn Data) -> Box<dyn Data>;
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
    fn name(&self) -> &str {
        self.name()
    }

    fn dyn_eq(&self, other: &dyn DynStat) -> bool {
        other.downcast_ref::<Self>()
            .map(|x| x == self)
            .unwrap_or(false)
    }

    fn dyn_ord(&self, other: &dyn DynStat) -> Ordering {
        use std::any::Any;
        other.downcast_ref::<Self>()
            .map(|x| x.cmp(self))
            .unwrap_or(self.type_id().cmp(&other.type_id()))
    }

    fn boxed_clone(&self) -> Box<dyn DynStat> {
        Box::new(self.clone())
    }

    fn default_value(&self) -> Box<dyn Data> {
        Box::<<T as Stat>::Data>::default()
    }

    fn from_out(&self, out: &dyn Data) -> Box<dyn Data> {
        Box::new(
            <<T as Stat>::Data>::from_out(
                out.downcast_ref::<<<T as Stat>::Data as StatComponents>::Out>()
                    .expect(TYPE_ERROR).clone())
        )
    }

    fn compose_stat(&self, from: &mut dyn Data, with: &dyn Data) {
        let from = from.downcast_mut::<T::Data>().expect("Wrong data type in compose.");
        let with = with.downcast_ref::<T::Data>().expect("Wrong data type in compose.");
        from.join(with.clone());
    }
}

#[derive(Debug, Resource, Default)]
pub struct StatInstances (
    pub(crate) FxHashMap<String, Box<dyn DynStat>>
);

impl StatInstances {
    pub fn register<T: Stat>(&mut self) {
        T::values().into_iter().for_each(|x| {
            self.0.insert(x.name().to_owned(), Box::new(x));
        })
    }
}

impl SerdeProject for Box<dyn DynStat> {
    type Ctx = StatInstances;
    type Ser<'t> = &'t str;
    type De<'de> = Cow<'de, str>;

    fn to_ser<'t>(&'t self, _: &&StatInstances) -> Result<Self::Ser<'t>, Box<Error>> {
        Ok(self.name())
    }

    fn from_de(ctx: &mut <Self::Ctx as FromWorldAccess>::Mut<'_>, de: Self::De<'_>) -> Result<Self, Box<Error>> {
        let s = de.as_ref();
        if let Some(result) = ctx.0.get(s){
            Ok(result.clone())
        } else {
            Err(Error::custom(format!("Unable to parse Stat \"{s}\".")))
        }
    }
}

#[macro_export]
macro_rules! stats {
    (@name $ident: ident) => {
        stringify!($name)
    };
    (@name $ident: ident as $name: literal) => {
        $name
    };
    (@single $data: ty, $ident: ident $(as $name: literal)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;

        impl $crate::Stat for $ident {
            type Data = $data;

            fn name(&self) -> &str {
                $crate::stats!(@name $ident $(as $name)?)
            }

            fn values() -> impl IntoIterator<Item = Self> {
                [Self]
            }
        }
    };

    (@single $data: ty, $ty: ident $(as $_: literal)? {
        $($ident:ident $(as $name: literal)?),*
        $(,)?
    }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $ty {
            $($ident),*
        }

        impl $crate::Stat for $ty {
            type Data = $data;

            fn name(&self) -> &str {
                match self {
                    $(Self::$ident => $crate::stats!(@name $ident $(as $name)?),)*
                }
            }

            fn values() -> impl IntoIterator<Item = Self> {
                [$(Self::$ident),*]
            }
        }
    };

    ($plugin: ident {
        $($data: ty {
            $($name: ident $(as $ty_name: literal)? $({
                $($variant: ident $(as $variant_name: literal)?),* $(,)?
            })?),*
            $(,)?
        }),* $(,)?
    }) => {
        $($(
            $crate::stats!(@single $data, $name $(as $ty_name)?
            $({
                $($variant $(as $variant_name)?),*
            })?);
        )*)*

        #[derive(Debug, Default)]
        pub struct $plugin;

        impl $crate::Plugin for $plugin {
            fn build(&self, world: &mut $crate::App) {
                use $crate::WorldExtension as _;
                $($(world.register_stat::<$name>();)*)*
            }
        }
    };
}
