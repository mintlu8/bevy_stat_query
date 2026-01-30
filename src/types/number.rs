use crate::num_traits::Number;
use crate::Float;
use crate::{operations::Unsupported, StatValue};
use bevy_reflect::TypePath;

/// A stat represented by a floating point number or a fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C, align(8))]
pub struct StatMultiplicative<T: Number> {
    addend: T,
    min: T,
    max: T,
    mult: T,
}

impl<T: Number> Default for StatMultiplicative<T> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

impl<T: Number> StatValue for StatMultiplicative<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult *= other.mult;
        self.min = self.min._max(other.min);
        self.max = self.max._min(other.max);
    }

    fn eval(&self) -> Self::Out {
        (self.addend * self.mult)._min(self.max)._max(self.min)
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
        self.min = self.min._max(other)
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max._min(other)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C, align(8))]
pub struct StatAdditive<T: Float> {
    addend: T,
    min: T,
    max: T,
    mult: T,
}

impl<T: Float> Default for StatAdditive<T> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ZERO,
        }
    }
}

impl<T: Float> StatValue for StatAdditive<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult += other.mult;
        self.min = self.min._max(other.min);
        self.max = self.max._min(other.max);
    }

    fn eval(&self) -> Self::Out {
        (self.addend * (self.mult + T::ONE))
            ._min(self.max)
            ._max(self.min)
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
        self.min = self.min._max(other)
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max._min(other)
    }

    fn from_base(base: Self::Base) -> Self {
        Self {
            addend: base,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ZERO,
        }
    }
}

/// An floating point or fraction based multiplier aggregation with default value 1. Does not support addition.
#[derive(Debug, Clone, Copy, PartialEq, TypePath)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C, align(8))]
pub struct StatMultiplied<T: Number> {
    min: T,
    max: T,
    mult: T,
}

impl<T: Number> Default for StatMultiplied<T> {
    fn default() -> Self {
        Self {
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

impl<T: Number> StatValue for StatMultiplied<T> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.mult *= other.mult;
        self.min = self.min._max(other.min);
        self.max = self.max._min(other.max);
    }

    fn eval(&self) -> Self::Out {
        self.mult._min(self.max)._max(self.min)
    }

    type Add = Unsupported;

    type Bit = Unsupported;

    type Mul = T;

    type Bounds = T;

    fn mul(&mut self, other: Self::Mul) {
        self.mult *= other;
    }

    fn min(&mut self, other: Self::Bounds) {
        self.min = self.min._max(other);
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max._min(other);
    }

    fn from_base(base: Self::Base) -> Self {
        Self {
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: base,
        }
    }
}
