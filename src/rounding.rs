use crate::Float;
use bevy_reflect::TypePath;
use std::fmt::Debug;

/// Rounding method for a floating point number.
pub trait Rounding: TypePath + Default + Debug + Copy + Send + Sync + 'static {
    /// Rounds to the an integer.
    fn round<F: Float>(input: F) -> F;
}

/// Rounds to 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TypePath)]
pub struct Truncate;

impl Rounding for Truncate {
    fn round<F: Float>(input: F) -> F {
        input.trunc()
    }
}

/// Rounds to the largest integer smaller than the float.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TypePath)]
pub struct Floor;

impl Rounding for Floor {
    fn round<F: Float>(input: F) -> F {
        input.floor()
    }
}

/// Rounds to the smallest integer larger than the float.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TypePath)]
pub struct Ceil;

impl Rounding for Ceil {
    fn round<F: Float>(input: F) -> F {
        input.ceil()
    }
}

/// Rounds to the nearest integer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TypePath)]
pub struct Round;

impl Rounding for Round {
    fn round<F: Float>(input: F) -> F {
        input.round()
    }
}

/// Rounds `x > 0` to at least `1`,
/// rounds `x < 0` to at most `-1`.
/// rounds `x == 0` to `0`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TypePath)]
pub struct TruncateSigned;

impl Rounding for TruncateSigned {
    fn round<F: Float>(input: F) -> F {
        if input > F::ZERO {
            input.trunc()._max(F::ONE)
        } else if input < F::ZERO {
            input.trunc()._min(F::ZERO - F::ONE)
        } else {
            input
        }
    }
}
