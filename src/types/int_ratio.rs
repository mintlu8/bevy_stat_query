use crate::Fraction;
use crate::{operations::Unsupported, StatValue};
use crate::{
    rounding::{Rounding, Truncate},
    Float, Int,
};
use bevy_reflect::TypePath;
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A stat represented by a floating point number or a fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TypePath)]
#[repr(C, align(8))]
pub struct StatInt<T: Int> {
    addend: T,
    min: T,
    max: T,
    mult: T,
}

impl<T: Int> Default for StatInt<T> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ONE,
        }
    }
}

impl<T: Int> StatValue for StatInt<T> {
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

/// An integer stat that multiplies with rational numbers and rounds back to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Int<PrimInt: Serialize> + Serialize, R: Rounding"))]
#[serde(bound(deserialize = "T: Int<PrimInt: Deserialize<'de>> + Deserialize<'de>, R: Rounding"))]
#[repr(C, align(8))]
pub struct StatIntFraction<T: Int, R: Rounding = Truncate> {
    addend: T,
    min: T,
    max: T,
    mult: Fraction<T::PrimInt>,
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding> Default for StatIntFraction<T, R> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: Float::ONE,
            rounding: Default::default(),
        }
    }
}

impl<T: Int, R: Rounding> StatValue for StatIntFraction<T, R> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult *= other.mult;
        self.min = self.min.max(other.min);
        self.max = self.max.min(other.max);
    }

    fn eval(&self) -> Self::Out {
        let val = self.mult * self.addend.into_fraction();
        let int_val = T::from_fraction(R::round(val));
        int_val.min(self.max).max(self.min)
    }

    type Add = T;

    type Mul = Fraction<T::PrimInt>;

    type Bit = Unsupported;

    type Bounds = T;

    fn add(&mut self, other: Self::Add) {
        self.addend += other;
    }

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
            addend: base,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: Float::ONE,
            rounding: Default::default(),
        }
    }
}

/// An integer stat that multiplies with floating point numbers and rounds back to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath, Serialize, Deserialize)]
#[repr(C, align(8))]
pub struct StatIntFloatMul<T: Int, F: Float, R: Rounding = Truncate> {
    addend: T,
    min: T,
    max: T,
    mult: F,
    rounding: PhantomData<R>,
}

impl<T: Int, F: Float, R: Rounding> Default for StatIntFloatMul<T, F, R> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: F::ONE,
            rounding: Default::default(),
        }
    }
}

impl<T: Int, F: Float, R: Rounding> StatValue for StatIntFloatMul<T, F, R>
where
    T: AsPrimitive<F>,
    F: AsPrimitive<T>,
{
    type Out = T;
    type Base = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult *= other.mult;
        self.min = self.min.max(other.min);
        self.max = self.max.min(other.max);
    }

    fn eval(&self) -> Self::Out {
        let val = self.addend.as_() * self.mult;
        let int_val: T = R::round(val).as_();
        int_val.min(self.max).max(self.min)
    }

    type Add = T;
    type Mul = F;
    type Bounds = T;

    type Bit = Unsupported;

    fn add(&mut self, other: Self::Add) {
        self.addend += other;
    }

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
            addend: base,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: Float::ONE,
            rounding: Default::default(),
        }
    }
}
