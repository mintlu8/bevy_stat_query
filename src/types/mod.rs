mod int_ratio;
mod int_pct;
mod float;
mod flags;
mod singleton;

use std::fmt::Debug;

use crate::{calc::StatOperation, Data, Shareable, TYPE_ERROR};

use downcast_rs::impl_downcast;
use dyn_clone::clone_trait_object;
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
pub trait StatValue: Shareable + Default + Clone{
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

    fn from_base(out: Self::Out) -> StatOperation<Self>;
}

pub(crate) trait DynStatValue: Data {
    fn apply_op(&mut self, other: &dyn Data);
    fn join_value(&mut self, other: &dyn Data);
}

impl_downcast!(DynStatValue);

clone_trait_object!(DynStatValue);

impl<T> DynStatValue for T where T: StatValue {
    fn apply_op(&mut self, other: &dyn Data) {
        other.downcast_ref::<StatOperation<T>>().expect(TYPE_ERROR).write_to(self)
    }

    fn join_value(&mut self, other: &dyn Data) {
        self.join(other.downcast_ref::<Self>().expect(TYPE_ERROR).clone())
    }
}