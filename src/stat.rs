use std::{
    any::{Any, TypeId},
    borrow::Cow,
    cmp::{Eq, Ord, Ordering},
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    ptr,
};

use bevy_ecs::system::Resource;
use bevy_serde_lens::with_world_mut;
use rustc_hash::FxHashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{validate, Buffer, Shareable, StatValue};

/// A `vtable` of dynamic functions on [`Stat::Value`].
#[repr(transparent)]
pub struct StatVTable<T = ()> {
    vtable: ErasedStatVTable,
    p: PhantomData<T>,
}

pub(crate) struct ErasedStatVTable {
    pub name: fn(u64) -> &'static str,
    pub join: unsafe fn(&mut Buffer, &Buffer),
    pub default: fn() -> Buffer,
    pub as_debug: unsafe fn(&Buffer) -> &dyn Debug,
    pub as_serialize: unsafe fn(&Buffer) -> &dyn erased_serde::Serialize,
    pub deserialize: fn(&mut dyn erased_serde::Deserializer) -> erased_serde::Result<Buffer>,
    pub clone: unsafe fn(&Buffer) -> Buffer,
    pub drop: unsafe fn(&mut Buffer),
}

impl StatVTable {
    /// Create a [`StatVTable`] of a given [`Stat`] type, complete with serialization support.
    pub const fn of<T: Stat<Value: Serialize + DeserializeOwned>>() -> StatVTable<T> {
        StatVTable {
            vtable: ErasedStatVTable {
                name: |id| T::index_to_name(id),
                join: |to, from| {
                    validate::<T::Value>();
                    let to = ptr::from_mut(to).cast::<T::Value>();
                    let from = ptr::from_ref(from).cast::<T::Value>();
                    unsafe { to.as_mut() }
                        .unwrap()
                        .join_by_ref(unsafe { from.as_ref().unwrap() })
                },
                default: || Buffer::from(T::Value::default()),
                as_debug: |buffer| unsafe { buffer.as_ref::<T::Value>() },
                as_serialize: |buffer| unsafe { buffer.as_ref::<T::Value>() },
                deserialize: |deserializer| {
                    validate::<T::Value>();
                    let value: T::Value = erased_serde::deserialize(deserializer)?;
                    Ok(Buffer::from(value))
                },
                clone: |buffer| Buffer::from(unsafe { buffer.as_ref::<T::Value>() }.clone()),
                drop: |buffer| {
                    let value = unsafe { buffer.read_move::<T::Value>() };
                    drop(value)
                },
            },
            p: PhantomData,
        }
    }

    /// Create a [`StatVTable`] of a given [`Stat`] type, panics on serialization.
    pub const fn no_serialize<T: Stat>() -> StatVTable<T> {
        StatVTable {
            vtable: ErasedStatVTable {
                name: |id| T::index_to_name(id),
                join: |to, from| {
                    validate::<T::Value>();
                    let to = ptr::from_mut(to).cast::<T::Value>();
                    let from = ptr::from_ref(from).cast::<T::Value>();
                    unsafe { to.as_mut() }
                        .unwrap()
                        .join_by_ref(unsafe { from.as_ref().unwrap() })
                },
                default: || Buffer::from(T::Value::default()),
                as_debug: |buffer| {
                    validate::<T::Value>();
                    let ptr = ptr::from_ref(buffer).cast::<T::Value>();
                    unsafe { ptr.as_ref() }.unwrap()
                },
                as_serialize: |_| panic!("Serialization is not supported."),
                deserialize: |_| panic!("Deserialization is not supported."),
                clone: |buffer| Buffer::from(unsafe { buffer.as_ref::<T::Value>() }.clone()),
                drop: |buffer| {
                    let value = unsafe { buffer.read_move::<T::Value>() };
                    drop(value)
                },
            },
            p: PhantomData,
        }
    }
}

impl Debug for ErasedStatVTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatVTable").finish_non_exhaustive()
    }
}

fn ref_cmp<T>(a: &T, b: &T) -> Ordering {
    (a as *const T as usize).cmp(&(b as *const T as usize))
}

/// Instance of a stat.
///
/// # Safety Invariant
///
/// If two [`StatInst`]s are equal, their corresponding [`Stat`] and [`StatValue`]
/// they are constructed from must be equal.
/// This is achieved through the constraint placed on construction of [`StatVTable`], which makes
/// having the same [`ErasedStatVTable`] on two different [`Stat`]s impossible in safe rust.
#[derive(Debug, Clone, Copy)]
pub struct StatInst {
    pub(crate) index: u64,
    pub(crate) vtable: &'static ErasedStatVTable,
}

impl StatInst {
    pub fn name(&self) -> &'static str {
        (self.vtable.name)(self.index)
    }

    pub unsafe fn clone_buffer(&self, buffer: &Buffer) -> Buffer {
        (self.vtable.clone)(buffer)
    }

    pub unsafe fn drop_buffer(&self, buffer: &mut Buffer) {
        (self.vtable.drop)(buffer)
    }
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
        ref_cmp(self.vtable, other.vtable).then(self.index.cmp(&other.index))
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
/// Each implementor can have its own `Value` type so you may want multiple of them.
pub trait Stat: Shareable {
    type Value: StatValue;

    /// Returns a globally unique name of the stat.
    fn name(&self) -> &'static str;

    /// Return a reference to a static [`StatVTable`] that supports `Debug`, `Drop` and serialization.
    ///
    /// # Example
    ///
    /// ```
    /// fn vtable() -> &'static StatVTable{
    ///     static vtable: StatVTable = StatVTable::of::<Self>();
    ///     &vtable
    /// }
    /// ```
    ///
    /// # Safety
    ///
    /// The resulting pointer must be unique across all implementors, this is achieved
    /// by having a generic constraint of `Self`.
    fn vtable() -> &'static StatVTable<Self>;

    /// Returns a locally unique index of the stat, used in equality comparisons.
    fn as_index(&self) -> u64;

    /// Convert from a unique index of the stat.
    ///
    /// This function can panic in case of a mismatch.
    fn from_index(index: u64) -> Self;

    /// Register all fields for serialization.
    fn values() -> impl IntoIterator<Item = Self>;

    /// Check for equality on generic stats.
    fn is<T: Stat>(&self, other: &T) -> bool {
        self.as_entry() == other.as_entry()
    }
}

/// Extension methods to [`Stat`].
pub(crate) trait StatExt: Stat {
    fn index_to_name(index: u64) -> &'static str {
        Self::from_index(index).name()
    }

    fn as_entry(&self) -> StatInst {
        StatInst {
            index: self.as_index(),
            vtable: &Self::vtable().vtable,
        }
    }

    /// Cast a generic [`Stat::Value`] to a concrete one. This is usually free in a generic context due to monomorphization.
    fn cast<'t, T: Stat>(&self, value: &'t mut Self::Value) -> Option<(&T, &'t mut T::Value)> {
        if TypeId::of::<Self>() == TypeId::of::<T>() {
            Some((
                (self as &dyn Any).downcast_ref()?,
                (value as &mut dyn Any).downcast_mut()?,
            ))
        } else {
            None
        }
    }

    /// Cast a generic [`Stat::Value`] to a concrete one if stat is equal.
    fn is_then_cast<'t, T: Stat>(
        &self,
        other: &T,
        value: &'t mut Self::Value,
    ) -> Option<&'t mut T::Value> {
        if !self.is(other) {
            return None;
        }
        (value as &mut dyn Any).downcast_mut()
    }
}

impl<T> StatExt for T where T: Stat {}

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

/// A pair of stat and value in a query.
///
/// # Safety Invariant
/// `value` must be the correct [`Stat::Value`].
pub struct StatValuePair {
    pub(crate) stat: StatInst,
    pub(crate) value: Buffer,
}

impl Debug for StatValuePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatValuePair")
            .field("stat", &self.stat.name())
            .field("value", unsafe {
                &(self.stat.vtable.as_debug)(&self.value)
            })
            .finish()
    }
}

impl StatValuePair {
    pub fn new<S: Stat>(stat: &S, value: S::Value) -> Self {
        StatValuePair {
            stat: stat.as_entry(),
            value: Buffer::from(value),
        }
    }

    pub fn new_default<S: Stat>(stat: &S) -> Self {
        StatValuePair {
            stat: stat.as_entry(),
            value: Buffer::from(S::Value::default()),
        }
    }

    /// Check for equality on generic stats.
    pub fn is<T: Stat>(&self, other: &T) -> bool {
        self.stat == other.as_entry()
    }

    /// Cast to a concrete [`Stat::Value`].
    pub fn cast<'t, T: Stat>(&mut self) -> Option<(T, &'t mut T::Value)> {
        validate::<T>();
        if ptr::eq(self.stat.vtable, &T::vtable().vtable) {
            let ptr = ptr::from_mut(&mut self.value) as *mut T::Value;
            Some((
                T::from_index(self.stat.index),
                unsafe { ptr.as_mut() }.unwrap(),
            ))
        } else {
            None
        }
    }

    /// Cast to a concrete [`Stat::Value`].
    pub fn is_then_cast<'t, T: Stat>(&mut self, other: &T) -> Option<&'t mut T::Value> {
        validate::<T>();
        if self.stat == other.as_entry() {
            let ptr = ptr::from_mut(&mut self.value) as *mut T::Value;
            unsafe { ptr.as_mut() }
        } else {
            None
        }
    }

    /// Cast to a concrete [`Stat::Value`].
    pub fn into_result<T: Stat>(self) -> Option<T::Value> {
        validate::<T>();
        if ptr::eq(self.stat.vtable, &T::vtable().vtable) {
            Some(unsafe { self.value.into() })
        } else {
            None
        }
    }

    pub(crate) fn clone_buffer(&self) -> Buffer {
        // Safety: Safe because invariant.
        unsafe { self.stat.clone_buffer(&self.value) }
    }
}
