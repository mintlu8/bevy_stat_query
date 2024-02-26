mod int_ratio;
mod int_pct;
mod float;
mod flags;
mod singleton;

use std::fmt::Debug;

use crate::{calc::StatOperation, Shareable};

pub use int_pct::{StatIntPercentAdditive, StatIntPercent};
pub use int_ratio::StatIntFraction;
pub use float::{StatFloat, StatFloatAdditive, StatMult};
pub use flags::{StatFlags, StatSet};
pub use singleton::StatSingleton;

/// A never type indicating an operation is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unsupported {}

/// Defines unordered operations on a stat's value.
#[allow(unused_variables)]
pub trait StatComponents: Shareable + Default + Clone{
    type Out: Shareable + Default;
    fn join(&mut self, other: Self);
    fn eval(&self) -> Self::Out;

    type Add: Shareable;
    type Mul: Shareable;
    type Bit: Shareable;
    type Bounds: Shareable;

    fn add(&mut self, other: Self::Add) {}
    fn mul(&mut self, other: Self::Mul) {}

    fn not(&mut self, other: Self::Bit) {}
    fn or(&mut self, other: Self::Bit) {}

    fn min(&mut self, other: Self::Bounds) {}
    fn max(&mut self, other: Self::Bounds) {}

    fn from_out(out: Self::Out) -> StatOperation<Self>;
}
