use std::marker::PhantomData;
use bevy_reflect::TypePath;
use crate::Fraction;
use serde::{Deserialize, Serialize};
use crate::{rounding::{Rounding, Truncate}, Float, Int, StatOperation};
use super::{StatValue, Unsupported};

/// An integer stat that sums up multipliers additively,
/// then divided by `SCALE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TypePath)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct StatIntPercentAdditive<T: Int, R: Rounding=Truncate, const SCALE: i64=100> {
    addend: T,
    mult: T,
    min: T,
    max: T,
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding, const S: i64> Default for StatIntPercentAdditive<T, R, S> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::from_i64(S),
            rounding: PhantomData
        }
    }
}

impl<T: Int, R: Rounding, const S: i64> StatValue for StatIntPercentAdditive<T, R, S> {
    type Out = T;


    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult += other.mult;
        self.max = self.max.min(other.max);
        self.min = self.min.max(other.min);
    }

    fn eval(&self) -> Self::Out {
        let numer = self.addend * self.mult;
        let base = T::from_fraction(R::round(numer.build_fraction(T::from_i64(S))));
        base.min(self.max).max(self.min)
    }

    type Add = T;
    type Mul = T;
    type Bounds = T;

    type Bit = Unsupported;

    fn add(&mut self, other: Self::Add) {
       self.addend += other;
    }

    fn mul(&mut self, other: Self::Mul) {
        // Since this is "sum the multipliers"
        self.mult += other;
    }

    fn min(&mut self, other: Self::Bounds) {
        self.min = self.min.max(other)
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max.min(other)
    }

    fn from_base(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Add(out)
    }
}


/// An integer stat with integer multipliers divided by `SCALE`.
///
/// Calculated as a fraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TypePath)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct StatIntPercent<T: Int, R: Rounding=Truncate, const SCALE: i64=100> {
    addend: T,
    mult: Fraction<T::PrimInt>,
    min: T,
    max: T,
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding, const S: i64> Default for StatIntPercent<T, R, S> {
    fn default() -> Self {
        Self {
            addend: T::ONE,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: Float::ONE,
            rounding: PhantomData
        }
    }
}

impl<T: Int, R: Rounding, const S: i64> StatValue for StatIntPercent<T, R, S> {
    type Out = T;

    fn join(&mut self, other: Self) {
        self.addend += other.addend;
        self.mult += other.mult;
        self.max = self.max.min(other.max);
        self.min = self.min.max(other.min);
    }

    fn eval(&self) -> Self::Out {
        let fraction = self.addend.into_fraction() * self.mult;
        let int = T::from_fraction(R::round(fraction));
        int.min(self.max).max(self.min)
    }

    type Add = T;
    type Mul = T;
    type Bounds = T;

    type Bit = Unsupported;

    fn add(&mut self, other: Self::Add) {
       self.addend += other;
    }

    fn mul(&mut self, other: Self::Mul) {
        self.mult *= T::build_fraction(other, T::from_i64(S));
    }

    fn min(&mut self, other: Self::Bounds) {
        self.min = self.min.max(other)
    }

    fn max(&mut self, other: Self::Bounds) {
        self.max = self.max.min(other)
    }

    fn from_base(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Add(out)
    }
}
