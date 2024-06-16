use std::{
    any::{Any, TypeId},
    borrow::Cow,
    cmp::{Eq, Ord, Ordering},
    fmt::Debug,
    hash::Hash,
    mem::MaybeUninit,
    ptr,
};

use bevy_ecs::system::Resource;
use bevy_serde_lens::with_world_mut;
use rustc_hash::FxHashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{validate, Buffer, Shareable, StatValue};

pub struct StatVTable {
    pub name: fn(u64) -> &'static str,
    pub as_serialize: unsafe fn(&Buffer) -> &dyn erased_serde::Serialize,
    pub deserialize: unsafe fn(&mut dyn erased_serde::Deserializer) -> erased_serde::Result<Buffer>,
    pub drop: unsafe fn(Buffer),
}

impl StatVTable {
    pub const fn of<T: Stat<Data: Serialize + DeserializeOwned>>() -> Self {
        StatVTable {
            name: |id| T::index_to_name(id),
            as_serialize: |buffer| {
                validate::<T::Data>();
                let ptr = ptr::from_ref(buffer).cast::<T::Data>();
                unsafe { ptr.as_ref() }.unwrap()
            },
            deserialize: |deserializer| {
                validate::<T::Data>();
                let value: T::Data = erased_serde::deserialize(deserializer)?;
                let mut buffer = [MaybeUninit::uninit(); 3];
                let ptr = buffer.as_mut_ptr() as *mut T::Data;
                unsafe { ptr.write(value) };
                Ok(buffer)
            },
            drop: |buffer| {
                validate::<T::Data>();
                let ptr = ptr::from_ref(&buffer).cast::<T::Data>();
                let value = unsafe { ptr.read() };
                drop(value)
            },
        }
    }

    pub const fn no_serialize<T: Stat>() -> Self {
        StatVTable {
            name: |id| T::index_to_name(id),
            as_serialize: |_| panic!("Serialization is not supported."),
            deserialize: |_| panic!("Deserialization is not supported."),
            drop: |buffer| {
                validate::<T::Data>();
                let ptr = ptr::from_ref(&buffer).cast::<T::Data>();
                let value = unsafe { ptr.read() };
                drop(value)
            },
        }
    }
}

impl Debug for StatVTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatVTable").finish_non_exhaustive()
    }
}

fn ref_cmp<T>(a: &T, b: &T) -> Ordering {
    (a as *const T as usize).cmp(&(b as *const T as usize))
}

#[derive(Debug, Clone, Copy)]
pub struct StatInst {
    pub(crate) index: u64,
    pub(crate) vtable: &'static StatVTable,
}

impl PartialEq for StatInst {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && ptr::eq(self.vtable, other.vtable)
    }
}

impl Eq for StatInst {}

impl PartialOrd for StatInst {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StatInst {
    fn cmp(&self, other: &Self) -> Ordering {
        ref_cmp(self, other)
    }
}

impl Hash for StatInst {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        (ptr::from_ref(self.vtable) as usize).hash(state);
    }
}

/// Implement this on your types to qualify them as a [`Stat`].
///
/// Similar to bevy's labels, you can either use one instance per stat,
/// or use one type per [`StatValue`].
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
    fn name(&self) -> &'static str;

    fn vtable() -> &'static StatVTable;

    fn as_index(&self) -> u64;

    fn from_index(index: u64) -> Self;

    /// Register all fields,
    /// alternatively register a `FromStr` parser.
    fn values() -> impl IntoIterator<Item = Self>;

    fn index_to_name(index: u64) -> &'static str {
        Self::from_index(index).name()
    }

    fn as_entry(&self) -> StatInst {
        StatInst {
            index: self.as_index(),
            vtable: Self::vtable(),
        }
    }

    fn is<T: Stat>(&self, other: &T) -> bool {
        self.as_entry() == other.as_entry()
    }

    fn is_then_cast<'t, T: Stat>(
        &self,
        other: &T,
        value: &'t mut Self::Data,
    ) -> Option<&'t mut T::Data> {
        if !self.is(other) {
            return None;
        }
        self.cast::<T>(value)
    }

    fn cast<'t, T: Stat>(&self, value: &'t mut Self::Data) -> Option<&'t mut T::Data> {
        if TypeId::of::<Self>() == TypeId::of::<T>() {
            (value as &mut dyn Any).downcast_mut()
        } else {
            None
        }
    }
}

#[derive(Resource, Default)]
pub struct StatInstances {
    pub(crate) concrete: FxHashMap<String, StatInst>,
}

impl Debug for StatInstances {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatInstances")
            .field("concrete", &self.concrete)
            .finish()
    }
}

impl StatInstances {
    /// Register all members of a [`Stat`].
    ///
    /// # Panics
    ///
    /// If a stat registered conflicts with a previous entry.
    pub fn register<T: Stat>(&mut self) {
        T::values().into_iter().for_each(|x| {
            if let Some(prev) = self.concrete.get(x.name()) {
                assert_eq!(prev, &x.as_entry(), "duplicate key {}", x.name())
            } else {
                self.concrete.insert(x.name().to_owned(), x.as_entry());
            }
        })
    }

    /// Register all members of a [`Stat`].
    ///
    /// Always replaces a registered [`Stat`] of the same key.
    pub fn register_replace<T: Stat>(&mut self) {
        T::values().into_iter().for_each(|x| {
            self.concrete.insert(x.name().to_owned(), x.as_entry());
        })
    }

    pub fn get(&self, name: &str) -> Option<StatInst> {
        self.concrete.get(name).copied()
    }
}

impl Serialize for StatInst {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.vtable.name)(self.index).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StatInst {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <Cow<str>>::deserialize(deserializer)?;
        with_world_mut::<_, D>(|world| {
            let ctx = world.resource::<StatInstances>();
            if let Some(result) = ctx.concrete.get(s.as_ref()) {
                Ok(*result)
            } else {
                Err(serde::de::Error::custom(format!(
                    "Unable to parse Stat \"{s}\"."
                )))
            }
        })?
    }
}
