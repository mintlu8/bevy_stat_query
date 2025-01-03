use std::fmt::Debug;

use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{operations::Unsupported, Shareable, StatValue};

/// A prioritized attribute that evaluates to the first or
/// last occurrence with the highest priority.
///
/// The [`Default`] priority is `i32::MIN`, if created via `From` or `from_base`,
/// priority is 0.
#[derive(Debug, Clone, Copy, TypePath, Serialize, Deserialize)]
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
    fn join(&mut self, other: Self) {
        if R {
            if self.priority <= other.priority {
                self.value = other.value
            }
        } else {
            if self.priority < other.priority {
                self.value = other.value
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
