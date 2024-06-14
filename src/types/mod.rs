mod flags;
mod float;
mod int_pct;
mod int_ratio;
mod singleton;

use std::fmt::Debug;

use crate::{calc::StatOperation, Data, Serializable, TYPE_ERROR};

use bevy_reflect::TypePath;
use bevy_serde_lens::typetagged::{FromTypeTagged, TraitObject};
use downcast_rs::impl_downcast;
use dyn_clone::clone_trait_object;
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

pub(crate) trait DynStatValue: Data {
    fn apply_op(&mut self, other: &dyn Data);
    fn join_value(&mut self, other: &dyn DynStatValue);
}

impl<T: StatValue + TypePath + Serialize> FromTypeTagged<T> for Box<dyn DynStatValue> {
    fn name() -> impl AsRef<str> {
        T::short_type_path()
    }

    fn from_type_tagged(item: T) -> Self {
        Box::new(item)
    }
}

impl_downcast!(DynStatValue);
clone_trait_object!(DynStatValue);

impl<T> DynStatValue for T
where
    T: StatValue + TypePath + Serialize,
    StatOperation<T>: TypePath + Serialize,
{
    fn apply_op(&mut self, other: &dyn Data) {
        other
            .downcast_ref::<StatOperation<T>>()
            .expect(TYPE_ERROR)
            .write_to(self)
    }

    fn join_value(&mut self, other: &dyn DynStatValue) {
        self.join(other.downcast_ref::<Self>().expect(TYPE_ERROR).clone())
    }
}

impl TraitObject for Box<dyn DynStatValue> {
    fn name(&self) -> impl AsRef<str> {
        <dyn DynStatValue>::name(self.as_ref())
    }

    fn as_serialize(&self) -> &dyn bevy_reflect::erased_serde::Serialize {
        self.as_ref().as_serialize()
    }
}
