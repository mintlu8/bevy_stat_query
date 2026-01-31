use std::{any::type_name, fmt::Debug, marker::PhantomData};

use crate::{Shareable, Stat, StatValue, operations::Unsupported};

/// A prioritized attribute that evaluates to the first or
/// last occurrence with the highest priority.
///
/// The [`Default`] priority is `i32::MIN`, if created via `From` or `from_base`,
/// priority is 0.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct Prioritized<T, const LAST: bool = true> {
    value: T,
    priority: i32,
}

impl<T: Default, const L: bool> Default for Prioritized<T, L> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            priority: i32::MIN,
        }
    }
}

impl<T, const L: bool> Prioritized<T, L> {
    pub const fn new(value: T, priority: i32) -> Self {
        Prioritized { value, priority }
    }

    pub const fn new_min(value: T) -> Self {
        Prioritized {
            value,
            priority: i32::MIN,
        }
    }

    pub const fn get(&self) -> &T {
        &self.value
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T, const L: bool> From<T> for Prioritized<T, L> {
    fn from(value: T) -> Self {
        Prioritized { value, priority: 0 }
    }
}

impl<T: Shareable + Default, const R: bool> StatValue for Prioritized<T, R> {
    type Out = T;

    #[allow(clippy::collapsible_else_if)]
    fn join(&mut self, other: &Self) {
        if R {
            if self.priority <= other.priority {
                self.value = other.value.clone()
            }
        } else {
            if self.priority < other.priority {
                self.value = other.value.clone()
            }
        }
    }

    fn eval(&self) -> Self::Out {
        self.value.clone()
    }

    type Add = Unsupported;

    type Mul = Unsupported;

    type Bit = Self;

    type Bounds = Unsupported;

    type Base = T;

    fn from_base(base: Self::Base) -> Self {
        Self {
            value: base,
            priority: 0,
        }
    }
}

/// A standard [`Stat`] for [`Prioritized`].
#[derive(Debug, Clone)]
pub struct GetPrioritized<T: Shareable + Default, const LAST: bool = true>(PhantomData<T>);

impl<T: Shareable + Default, const LAST: bool> Default for GetPrioritized<T, LAST> {
    fn default() -> Self {
        GetPrioritized(PhantomData)
    }
}

impl<T: Shareable + Default> GetPrioritized<T, true> {
    const fn new() -> Self {
        GetPrioritized(PhantomData)
    }
}

impl<T: Shareable + Default> GetPrioritized<T, false> {
    /// Use non-standard `Prioritized` that chooses the first occurrence instead of the last.
    const fn new_choose_first() -> Self {
        GetPrioritized(PhantomData)
    }
}

impl<T: Shareable + Default, const LAST: bool> Copy for GetPrioritized<T, LAST> {}

impl<T: Shareable + Default, const LAST: bool> PartialEq for GetPrioritized<T, LAST> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<T: Shareable + Default, const LAST: bool> Eq for GetPrioritized<T, LAST> {}

impl<T: Shareable + Default, const LAST: bool> Stat for GetPrioritized<T, LAST> {
    type Value = Prioritized<T, LAST>;

    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn as_index(&self) -> u64 {
        0
    }

    fn from_index(_: u64) -> Self {
        Self(PhantomData)
    }

    fn values() -> impl IntoIterator<Item = Self> {
        [Self(PhantomData)]
    }
}
