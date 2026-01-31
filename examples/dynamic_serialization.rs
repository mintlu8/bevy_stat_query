#![allow(clippy::collapsible_if)]
//! This replicates the old `StatMap` functionality that could erase any type and perform serialization.
//!
//! In 0.5 serialization is no longer a semi-mandatory supertrait of `Stat` and `StatValue`,
//! so this is provided as an alternative custom implementation.
use bevy_reflect::TypePath;
use bevy_stat_query::{
    DeserializeEntry, Fraction, SerializeEntry, ShareableAny, Stat, StatDispatch, StatDispatchTo,
    StatMap, StatUid, StatValue,
    types::{
        Prioritized, StatAdditive, StatFlags, StatIntPercentAdditive, StatMultiplicative,
        StatMultiplied, StatRounded,
    },
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::{any::Any, fmt::Debug};

pub trait ErasedStat: Debug + Send + Sync + erased_serde::Serialize {
    fn get_uid(&self) -> StatUid;
    fn dyn_clone(&self) -> Box<dyn ErasedStat>;
    fn join_to(&self, value: &dyn ErasedStatValue, into: &mut dyn bevy_stat_query::ShareableAny);
    fn deserialize_stat(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
    ) -> erased_serde::Result<Box<dyn ErasedStatValue>>;
}

impl<T> ErasedStat for T
where
    T: Stat + Serialize,
    T::Value: Serialize + DeserializeOwned,
{
    fn get_uid(&self) -> StatUid {
        T::as_uid(self)
    }

    fn dyn_clone(&self) -> Box<dyn ErasedStat> {
        Box::new(self.clone())
    }

    fn join_to(&self, value: &dyn ErasedStatValue, into: &mut dyn bevy_stat_query::ShareableAny) {
        if let Some(value) = value.as_any().downcast_ref::<T::Value>() {
            if let Some(other) = into.downcast_mut::<T::Value>() {
                other.join(value)
            }
        }
    }

    fn deserialize_stat(
        &self,
        deserializer: &mut dyn erased_serde::Deserializer,
    ) -> erased_serde::Result<Box<dyn ErasedStatValue>> {
        erased_serde::deserialize::<T::Value>(deserializer)
            .map(|x| Box::new(x) as Box<dyn ErasedStatValue>)
    }
}

impl Clone for Box<dyn ErasedStat> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

pub trait ErasedStatValue: ShareableAny + erased_serde::Serialize {
    fn join_dyn(&mut self, other: &dyn Any);
    fn dyn_clone(&self) -> Box<dyn ErasedStatValue>;
}

impl<T> ErasedStatValue for T
where
    T: StatValue + Serialize,
{
    fn join_dyn(&mut self, other: &dyn Any) {
        if let Some(item) = other.downcast_ref::<Self>() {
            self.join(item);
        }
    }
    fn dyn_clone(&self) -> Box<dyn ErasedStatValue> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ErasedStatValue> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

impl StatDispatch for Box<dyn ErasedStat> {
    type Value = Box<dyn ErasedStatValue>;

    fn get_uid(&self) -> StatUid {
        (**self).get_uid()
    }

    fn join_to(&self, value: &Self::Value, into: &mut dyn bevy_stat_query::ShareableAny) {
        (**self).join_to(&**value, into);
    }
}

impl<T: Stat<Value: Serialize + DeserializeOwned> + Serialize> StatDispatchTo<T>
    for Box<dyn ErasedStat>
{
    fn from_stat(stat: T) -> Self {
        Box::new(stat)
    }

    fn from_value(value: <T as Stat>::Value) -> Self::Value {
        Box::new(value)
    }

    fn get_value(value: &Self::Value) -> Option<&<T as Stat>::Value> {
        value.as_any().downcast_ref()
    }

    fn get_value_mut(value: &mut Self::Value) -> Option<&mut <T as Stat>::Value> {
        value.as_any_mut().downcast_mut()
    }

    fn get_value_owned(value: Self::Value) -> Option<<T as Stat>::Value> {
        value.as_any_boxed().downcast().ok().map(|x| *x)
    }

    fn try_join_to(&self, value: &Self::Value, into: &mut <T as Stat>::Value) {
        if let Some(stat) = value.as_any().downcast_ref() {
            into.join(stat);
        }
    }
}

impl SerializeEntry for Box<dyn ErasedStat> {
    fn serialize_item<S: Serializer>(
        &self,
        value: &Self::Value,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        erased_serde::serialize(&**value, serializer)
    }
}

impl DeserializeEntry for Box<dyn ErasedStat> {
    fn deserialize_item<'de, D: serde::Deserializer<'de>>(
        &self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let mut de = <dyn erased_serde::Deserializer>::erase(deserializer);
        self.deserialize_stat(&mut de)
            .map_err(serde::de::Error::custom)
    }
}

erased_serde::serialize_trait_object!(ErasedStat);

impl<'de> Deserialize<'de> for Box<dyn ErasedStat> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        DESERIALIZE_LIST
            .iter()
            .find(|x| x.0 == name)
            .map(|x| x.1.dyn_clone())
            .ok_or(serde::de::Error::custom("Invalid stat."))
    }
}

macro_rules! impl_stat {
    ($($name: ident: $ty: ty),* $(,)?) => {
        // Simulates a linkme setup
        pub static DESERIALIZE_LIST: &[(&str, &dyn ErasedStat)] = &[
            $((stringify!($name), &$name)),*
        ];

        $(#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name;

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                stringify!($name).serialize(serializer)
            }
        }

        impl Stat for $name {
            type Value = $ty;

            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn values() -> impl IntoIterator<Item = Self> {
                [Self]
            }

            fn as_index(&self) -> u64 {
                0
            }

            fn from_index(_: u64) -> Self {
                Self
            }
        })*
    };
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize, TypePath)]
    pub struct MyFlags: u32 {
        const A = 1;
        const B = 2;
        const C = 4;
        const D = 8;
        const E = 16;
        const F = 32;
        const G = 64;
    }
}

impl_stat!(
    SInt: StatAdditive<i32>,
    SUInt: StatAdditive<u32>,
    SFloat32: StatMultiplicative<f32>,
    SFlags: StatFlags<MyFlags>,
    SString: Prioritized<Box<str>>,
    SIntPct: StatIntPercentAdditive<i32>,
    SIntFrac: StatRounded<i8, Fraction<i8>>,
    SMul: StatMultiplied<f32>,
    SFracMul: StatMultiplied<Fraction<i32>>
);

fn main() {
    let mut map = StatMap::<u32, Box<dyn ErasedStat>>::default();
    map.insert_base(0u32.into(), SInt, -4);
    map.insert_base(0u32.into(), SUInt, 7);
    map.insert_base(0u32.into(), SFloat32, 3.5);
    map.insert_base(0u32.into(), SFlags, MyFlags::F);
    map.insert_base(0u32.into(), SString, "Ferris the Rustacean".into());
    map.insert_base(0u32.into(), SIntFrac, 69);
    map.insert_base(0u32.into(), SIntPct, 420);
    map.insert_base(0u32.into(), SMul, 1.5);
    map.insert_base(0u32.into(), SFracMul, Fraction::new(44, 57));
    let json = serde_json::to_string_pretty(&map).unwrap();
    println!("{}", json);

    let deserialized = serde_json::from_str::<StatMap<u32, Box<dyn ErasedStat>>>(&json).unwrap();
    dbg!(&deserialized);
}
