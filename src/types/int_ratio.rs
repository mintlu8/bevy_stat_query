use std::marker::PhantomData;
use num_rational::Ratio;
use serde::{Deserialize, Serialize};
use crate::{StatOperation, Int, Float, rounding::{Rounding, Truncate}};
use super::{StatComponents, Unsupported};


/// An integer stat that multiplies with rational numbers and rounds back to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: serde::Serialize, T::PrimInt: serde::Serialize"))]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>, T::PrimInt: serde::Deserialize<'de>"))]
pub struct StatIntFraction<T: Int, R: Rounding=Truncate> {
    addend: T,
    min: T,
    max: T,
    mult: Ratio<T::PrimInt>,
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding> Default for StatIntFraction<T, R> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: Float::ONE,
            rounding: Default::default()
        }
    }
}

impl<T: Int, R: Rounding> StatComponents for StatIntFraction<T, R> {
    type Out = T;

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

    type Mul = Ratio<T::PrimInt>;

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

    fn from_out(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Add(out)
    }
}

/// An integer stat that multiplies with floating point numbers and rounds back to an integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatIntFloatMul<T: Int, F: Float, R: Rounding=Truncate> {
    addend: T,
    min: T,
    max: T,
    mult: F,
    rounding: PhantomData<R>,
}

impl<T: Int, F: Float, R: Rounding> Default for StatIntFloatMul<T, F, R> where T: Into<F>, F: Into<T> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: F::ONE,
            rounding: Default::default()
        }
    }
}

impl<T: Int, F: Float, R: Rounding> StatComponents for StatIntFloatMul<T, F, R> where T: Into<F>, F: Into<T> {
    type Out = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult *= other.mult;
        self.min = self.min.max(other.min);
        self.max = self.max.min(other.max);
    }

    fn eval(&self) -> Self::Out {
        let val = Into::<F>::into(self.addend) * self.mult;
        let int_val: T = R::round(val).into();
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

    fn from_out(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Add(out)
    }
}
