use crate::num_traits::Flags;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::{operations::Unsupported, StatValue};

/// A flags based on a type that supports bitwise operations,
/// like integer, `bitflgs` or `enumset`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, TypePath)]
#[repr(C, align(8))]
pub struct StatFlags<T: Flags> {
    or: T,
}

impl<T: Flags> StatValue for StatFlags<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.or |= other.or;
    }

    fn eval(&self) -> Self::Out {
        self.or.clone()
    }

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    type Bit = T;

    fn or(&mut self, other: Self::Bit) {
        self.or |= other
    }

    fn from_base(base: Self::Base) -> Self {
        Self { or: base }
    }
}
