use std::fmt::Debug;

use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{operations::Unsupported, Shareable, StatValue};

/// A prioritized attribute that evaluates to the first or
/// last occurrence with the highest priority.
#[derive(Debug, Clone, Copy, Default, TypePath, Serialize, Deserialize)]
#[repr(C)]
pub struct Prioritized<T, const LAST: bool = false> {
    pub value: T,
    pub priority: i32,
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

    type Base = Self;

    fn from_base(base: Self::Base) -> Self {
        base
    }
}
