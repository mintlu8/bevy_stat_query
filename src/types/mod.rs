mod flags;
mod float;
mod int_pct;
mod int_ratio;
mod singleton;

use std::fmt::Debug;

use crate::{calc::StatOperation, Serializable};
use bevy_reflect::TypePath;
pub use flags::{StatFlags, StatSet};
pub use float::{StatFloat, StatFloatAdditive, StatMult};
pub use int_pct::{StatIntPercent, StatIntPercentAdditive};
pub use int_ratio::{StatInt, StatIntFloatMul, StatIntFraction};
use serde::{Deserialize, Serialize};
pub use singleton::StatOnce;

/// A never type indicating an operation is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TypePath, Serialize, Deserialize)]
pub enum Unsupported {}

/// Defines unordered operations on a stat's value.
#[allow(unused_variables)]
pub trait StatValue: Serializable + Default {
    type Out: Serializable + Default;
    fn join(&mut self, other: Self);
    fn eval(&self) -> Self::Out;

    type Add: Serializable;
    type Mul: Serializable;
    type Bit: Serializable;
    type Bounds: Serializable;

    fn add(&mut self, other: Self::Add) {}
    fn mul(&mut self, other: Self::Mul) {}

    fn not(&mut self, other: Self::Bit) {}
    fn or(&mut self, other: Self::Bit) {}

    fn min(&mut self, other: Self::Bounds) {}
    fn max(&mut self, other: Self::Bounds) {}

    fn from_base(out: Self::Out) -> StatOperation<Self>;
}
