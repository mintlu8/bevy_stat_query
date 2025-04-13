use crate::{operations::Unsupported, StatValue};
use crate::{
    rounding::{Rounding, Truncate},
    Float, Int,
};
use crate::{Fraction, NumCast};
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// A stat represented by an integer, does not support floating point multipliers.
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

/// An integer stat that multiplies with floating point numbers and rounds back to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath, Serialize, Deserialize)]
#[repr(C, align(8))]
pub struct StatIntRounded<T: Int, F: Float, R: Rounding = Truncate> {
    addend: T,
    min: T,
    max: T,
    mult: F,
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding> StatIntRounded<T, Fraction<T>, R> {
    pub fn reduce(&mut self) {
        self.mult = self.mult.reduced();
    }

    pub fn reduced(mut self) -> Self {
        self.mult = self.mult.reduced();
        self
    }
}

impl<T: Int, F: Float, R: Rounding> Default for StatIntRounded<T, F, R> {
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

impl<T: Int, F: Float, R: Rounding> StatValue for StatIntRounded<T, F, R>
where
    T: NumCast<F>,
    F: NumCast<T>,
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
        let val = self.addend.cast() * self.mult;
        let int_val: T = R::round(val).cast();
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
