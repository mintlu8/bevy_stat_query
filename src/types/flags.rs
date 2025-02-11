use crate::num_traits::Flags;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, mem, ops::BitAnd};

use crate::{operations::Unsupported, StatValue};

/// A flags based on a type that supports bitwise operations,
/// like integer, `bitflgs` or `enumset`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, TypePath)]
#[repr(transparent)]
pub struct StatFlags<T: Flags>(T);

impl<T: Flags> StatFlags<T> {
    pub const fn new(item: T) -> Self {
        StatFlags(item)
    }

    pub fn exclude(&mut self, item: T) {
        let this = mem::take(self);
        self.0 = this.0.exclude(item);
    }

    pub fn contains(&self, item: T) -> bool
    where
        T: BitAnd<Output = T> + PartialEq,
    {
        self.0.clone() & item == self.0
    }
}

impl<T: Flags> StatValue for StatFlags<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.0 |= other.0;
    }

    fn eval(&self) -> Self::Out {
        self.0.clone()
    }

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    type Bit = T;

    fn or(&mut self, other: Self::Bit) {
        self.0 |= other
    }

    fn from_base(base: Self::Base) -> Self {
        Self(base)
    }
}
