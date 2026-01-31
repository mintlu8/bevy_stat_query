use crate::{
    Int,
    rounding::{Rounding, Truncate},
};
use crate::{StatValue, operations::Unsupported};
use bevy_reflect::TypePath;
use std::marker::PhantomData;

/// An integer stat that sums up multipliers additively,
/// then divided by `SCALE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TypePath)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C, align(8))]
pub struct StatIntPercentAdditive<T: Int, R: Rounding = Truncate, const SCALE: i64 = 100> {
    #[cfg_attr(feature = "serde", serde(default = "super::util::num_zero"))]
    addend: T,
    #[cfg_attr(feature = "serde", serde(default = "super::util::num_zero"))]
    mult: T,
    #[cfg_attr(feature = "serde", serde(default = "super::util::num_min"))]
    min: T,
    #[cfg_attr(feature = "serde", serde(default = "super::util::num_max"))]
    max: T,
    #[cfg_attr(feature = "serde", serde(skip))]
    rounding: PhantomData<R>,
}

impl<T: Int, R: Rounding, const S: i64> Default for StatIntPercentAdditive<T, R, S> {
    fn default() -> Self {
        Self {
            addend: T::ZERO,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ZERO,
            rounding: PhantomData,
        }
    }
}

impl<T: Int, R: Rounding, const S: i64> StatValue for StatIntPercentAdditive<T, R, S> {
    type Out = T;
    type Base = T;

    fn join(&mut self, other: &Self) {
        self.addend += other.addend;
        self.mult += other.mult;
        self.max = self.max.min(other.max);
        self.min = self.min.max(other.min);
    }

    fn eval(&self) -> Self::Out {
        let numer = self.addend * (self.mult + T::from_i64(S));
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

    fn from_base(base: Self::Base) -> Self {
        Self {
            addend: base,
            min: T::MIN_VALUE,
            max: T::MAX_VALUE,
            mult: T::ZERO,
            rounding: PhantomData,
        }
    }
}
