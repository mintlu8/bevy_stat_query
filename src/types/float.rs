use super::{StatValue, Unsupported};
use crate::Float;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

/// A stat represented by a floating point number or a fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TypePath)]
#[repr(C, align(8))]
pub struct StatFloat<T: Float> {
    pub addend: T,
    pub min: T,
    pub max: T,
    pub mult: T,
}

impl<T: Float> Default for StatFloat<T> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

impl<T: Float> StatValue for StatFloat<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult *= other.mult;
        self.min = self.min.max(other.min);
        self.max = self.max.min(other.max);
    }

    fn eval(&self) -> Self::Out {
        (self.addend * self.mult).min(self.max).max(self.min)
    }

    type Add = T;
    type Mul = T;
    type Bounds = T;

    type Bit = Unsupported;

    fn add(&mut self, other: Self::Add) {
        self.addend += other;
    }

    fn mul(&mut self, other: Self::Mul) {
        self.mult *= other;
    }

    fn min(&mut self, other: Self::Bounds) {
        self.min = self.min.max(other)
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max.min(other)
    }

    fn from_base(base: Self::Base) -> Self {
        Self {
            addend: base,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

/// A stat represented by a floating point number or a fraction, multiplier is additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TypePath)]
#[repr(C, align(8))]
pub struct StatFloatAdditive<T: Float> {
    pub addend: T,
    pub min: T,
    pub max: T,
    pub mult: T,
}

impl<T: Float> Default for StatFloatAdditive<T> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

impl<T: Float> StatValue for StatFloatAdditive<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult += other.mult;
        self.min = self.min.max(other.min);
        self.max = self.max.min(other.max);
    }

    fn eval(&self) -> Self::Out {
        (self.addend * self.max).min(self.max).max(self.min)
    }

    type Add = T;
    type Mul = T;
    type Bounds = T;

    type Bit = Unsupported;

    fn add(&mut self, other: Self::Add) {
        self.addend += other;
    }

    fn mul(&mut self, other: Self::Mul) {
        self.mult += other;
    }

    fn min(&mut self, other: Self::Bounds) {
        self.min = self.min.max(other)
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max.min(other)
    }

    fn from_base(base: Self::Base) -> Self {
        Self {
            addend: base,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

/// An floating point or fraction based multiplier aggregation. Does not support addition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TypePath)]
#[repr(C, align(8))]
pub struct StatMult<T: Float> {
    min: T,
    max: T,
    mult: T,
}

impl<T: Float> Default for StatMult<T> {
    fn default() -> Self {
        Self {
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

impl<T: Float> StatValue for StatMult<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.mult *= other.mult;
        self.min = self.min.max(other.min);
        self.max = self.max.min(other.max);
    }

    fn eval(&self) -> Self::Out {
        self.mult
    }

    type Add = Unsupported;

    type Bit = Unsupported;

    type Mul = T;

    type Bounds = T;

    fn mul(&mut self, other: Self::Mul) {
        self.mult *= other;
    }

    fn min(&mut self, other: Self::Bounds) {
        self.min = self.min.max(other);
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max.min(other);
    }

    fn from_base(base: Self::Base) -> Self {
        Self {
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: base,
        }
    }
}
