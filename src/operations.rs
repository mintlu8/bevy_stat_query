use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

/// An single step unordered operation on a [`StatValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum StatOperation<S: StatValue> {
    Add(S::Add),
    Mul(S::Mul),
    Or(S::Bit),
    Not(S::Bit),
    Min(S::Bounds),
    Max(S::Bounds),
    Base(S::Base),
}

pub use StatOperation::*;

use crate::Shareable;

impl<S: StatValue> StatOperation<S> {
    pub fn write_to(&self, to: &mut S) {
        match self.clone() {
            StatOperation::Add(item) => to.add(item),
            StatOperation::Mul(item) => to.mul(item),
            StatOperation::Or(item) => to.or(item),
            StatOperation::Not(item) => to.not(item),
            StatOperation::Min(item) => to.min(item),
            StatOperation::Max(item) => to.max(item),
            StatOperation::Base(item) => *to = S::from_base(item),
        }
    }

    pub fn into_stat(self) -> S {
        let mut v = S::default();
        self.write_to(&mut v);
        v
    }
}

/// A never type indicating an operation is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TypePath, Serialize, Deserialize)]
pub enum Unsupported {}

/// Defines unordered operations on a stat's value.
#[allow(unused_variables)]
pub trait StatValue: Shareable + Default {
    type Out: Shareable + Default;

    fn join(&mut self, other: Self);

    fn join_by_ref(&mut self, other: &Self) {
        self.join(other.clone())
    }

    fn eval(&self) -> Self::Out;

    type Add: Shareable;
    type Mul: Shareable;
    type Bit: Shareable;
    type Bounds: Shareable;
    type Base: Shareable;

    fn add(&mut self, other: Self::Add) {}
    fn mul(&mut self, other: Self::Mul) {}

    fn not(&mut self, other: Self::Bit) {}
    fn or(&mut self, other: Self::Bit) {}

    fn min(&mut self, other: Self::Bounds) {}
    fn max(&mut self, other: Self::Bounds) {}

    fn from_base(base: Self::Base) -> Self;
}

impl StatValue for bool {
    type Out = bool;

    fn join(&mut self, other: Self) {
        *self |= other
    }

    fn eval(&self) -> Self::Out {
        *self
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
