use std::{borrow::Cow, cmp::{Eq, Ord, Ordering}, fmt::Debug, hash::Hash, str::FromStr};

use bevy_ecs::system::Resource;
use bevy_serde_project::{Error, FromWorldAccess, SerdeProject};
use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::DynClone;
use dyn_hash::DynHash;

use crate::{sealed::SealedAll, types::DynStatValue, Data, Shareable, StatValue, TYPE_ERROR};

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
    type Data: StatValue;

    /// Unique name of the stat.
    fn name(&self) -> &str;

    /// Register all fields,
    /// alternatively register a `FromStr` parser.
    fn values() -> impl IntoIterator<Item = Self>;

    /// Equality comparison between all stat implementors.
    fn is<S: Stat + SealedAll>(&self, other: &S) -> bool{
        self as &dyn DynStat == other as &dyn DynStat
    }
}

/// Object safe version of [`Stat`].
pub(crate) trait DynStat: Downcast + DynClone + DynHash + Debug + Send + Sync {
    fn name(&self) -> &str;
    fn dyn_eq(&self, other: &dyn DynStat) -> bool;
    fn dyn_ord(&self, other: &dyn DynStat) -> Ordering;
    fn default_value(&self) -> Box<dyn DynStatValue>;
    #[allow(clippy::wrong_self_convention)]
    fn from_base(&self, out: &dyn Data) -> Box<dyn Data>;
}

impl_downcast!(DynStat);
dyn_clone::clone_trait_object!(DynStat);
dyn_hash::hash_trait_object!(DynStat);

impl PartialEq for dyn DynStat {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn DynStat {}

impl<S: DynStat> PartialEq<S> for dyn DynStat {
    fn eq(&self, other: &S) -> bool {
        self.dyn_eq(other)
    }
}

impl<S: DynStat> PartialEq<S> for Box<dyn DynStat>  {
    fn eq(&self, other: &S) -> bool {
        self.dyn_eq(other)
    }
}

impl PartialEq<str> for dyn DynStat  {
    fn eq(&self, other: &str) -> bool {
        self.name() == other
    }
}

impl PartialEq<str> for Box<dyn DynStat>  {
    fn eq(&self, other: &str) -> bool {
        self.name() == other
    }
}

impl PartialEq<String> for dyn DynStat  {
    fn eq(&self, other: &String) -> bool {
        self.name() == other.as_str()
    }
}

impl PartialEq<String> for Box<dyn DynStat>  {
    fn eq(&self, other: &String) -> bool {
        self.name() == other.as_str()
    }
}


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

    fn default_value(&self) -> Box<dyn DynStatValue> {
        Box::<<T as Stat>::Data>::default()
    }

    fn from_base(&self, out: &dyn Data) -> Box<dyn Data> {
        Box::new(
            <<T as Stat>::Data>::from_base(
                out.downcast_ref::<<<T as Stat>::Data as StatValue>::Out>()
                    .expect(TYPE_ERROR).clone())
        )
    }
}

#[derive(Resource, Default)]
pub struct StatInstances {
    pub(crate) concrete: HashMap<String, Box<dyn DynStat>>,
    pub(crate) any: Vec<fn(&str) -> Option<Box<dyn DynStat>>>,
}

impl Debug for StatInstances {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatInstances")
            .field("concrete", &self.concrete)
            .field("any", &self.any.len())
            .finish()
    }
}

impl StatInstances {
    pub fn register<T: Stat>(&mut self) {
        T::values().into_iter().for_each(|x| {
            self.concrete.insert(x.name().to_owned(), Box::new(x));
        })
    }

    pub fn register_parser<T: Stat + FromStr>(&mut self) {
        T::values().into_iter().for_each(|x| {
            self.concrete.insert(x.name().to_owned(), Box::new(x));
        });
        self.any.push(|s| T::from_str(s).map(|x| Box::new(x) as Box<dyn DynStat>).ok())
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
        if let Some(result) = ctx.concrete.get(s){
            Ok(result.clone())
        } else if let Some(result) = ctx.any.iter().find_map(|f| f(s)){
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
                use $crate::StatExtension as _;
                $($(world.register_stat::<$name>();)*)*
            }
        }
    };
}
