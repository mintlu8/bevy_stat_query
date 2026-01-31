use crate::{
    Float, Int,
    rounding::{Rounding, Truncate},
};
use crate::{Fraction, NumCast};
use crate::{StatValue, operations::Unsupported};
use bevy_reflect::TypePath;
use std::marker::PhantomData;

/// An integer stat that multiplies with floating point numbers and rounds back to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C, align(8))]
pub struct StatRounded<T: Int, F: Float, R: Rounding = Truncate> {
    addend: T,
    min: T,
    max: T,
    mult: F,
    #[cfg_attr(feature = "serde", serde(skip))]
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding> StatRounded<T, Fraction<T>, R> {
    pub fn reduce(&mut self) {
        self.mult = self.mult.reduced();
    }

    pub fn reduced(mut self) -> Self {
        self.mult = self.mult.reduced();
        self
    }

    pub fn get_addend(&self) -> T {
        self.addend
    }

    pub fn get_multiplier(&self) -> Fraction<T> {
        self.mult
    }
}

impl<T: Int, F: Float, R: Rounding> Default for StatRounded<T, F, R> {
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

impl<T: Int, F: Float, R: Rounding> StatValue for StatRounded<T, F, R>
where
    T: NumCast<F>,
    F: NumCast<T>,
{
    type Out = T;
    type Base = T;

    fn join(&mut self, other: &Self) {
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
            mult: F::ONE,
            rounding: Default::default(),
        }
    }
}
