use std::{num::Wrapping, ops::*, fmt::Debug};
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use crate::Serializable;

pub trait NumInteger: num_integer::Integer + num_traits::NumAssign {}
impl<T> NumInteger for T where T: num_integer::Integer + num_traits::NumAssign {}

pub trait NumOps: Sized +
    Add<Self, Output = Self> +
    Sub<Self, Output = Self> +
    Mul<Self, Output = Self> +
    AddAssign<Self> +
    MulAssign<Self> {
}

impl<T> NumOps for T where T: Sized +
    Add<Self, Output = Self> +
    Sub<Self, Output = Self> +
    Mul<Self, Output = Self> +
    AddAssign<Self> +
    MulAssign<Self>  {
}


pub trait BitOps: Sized +
    BitAnd<Self, Output = Self> +
    BitOr<Self, Output = Self> +
    BitXor<Self, Output = Self> +
    BitAndAssign<Self> +
    BitOrAssign<Self> +
    BitXorAssign<Self> {
}

impl<T> BitOps for T where T: Sized +
    BitAnd<Self, Output = Self> +
    BitOr<Self, Output = Self> +
    BitXor<Self, Output = Self> +
    BitAndAssign<Self> +
    BitOrAssign<Self> +
    BitXorAssign<Self>  {
}

/// A type that can be treated as flags.
///
/// Automatically implemented on types implementing all three bitwise operations `&|^`.
pub trait Flags: BitOr<Self, Output = Self> + BitOrAssign<Self> + Debug + Default + Serializable {
    /// Exclude a portion of the flags.
    fn exclude(self, other: Self) -> Self;
}

impl<T> Flags for T where T: BitOps + Debug + Default + Serializable{
    fn exclude(self, other: Self) -> Self {
        self.clone() ^ (self & other)
    }
}

/// Trait for an integer.
pub trait Int: NumOps + PartialOrd + Default + Copy + Serializable {
    const ZERO: Self;
    const ONE: Self;

    const MIN_VALUE: Self;
    const MAX_VALUE: Self;

    fn from_i64(value: i64) -> Self;

    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;

    type PrimInt: Int + NumInteger + Clone + Serializable;

    fn into_fraction(self) -> Ratio<Self::PrimInt>;
    fn build_fraction(self, denom: Self) -> Ratio<Self::PrimInt>;
    fn from_fraction(frac: Ratio<Self::PrimInt>) -> Self;
}

macro_rules! impl_int {
    ($($ty: ty),* $(,)?) => {
        $(impl Int for $ty {
            const ZERO: Self = 0;
            const ONE: Self = 1;

            const MIN_VALUE: Self = <$ty>::MIN;
            const MAX_VALUE: Self = <$ty>::MAX;

            fn from_i64(value: i64) -> Self{
                value.clamp(Self::MIN as i64, Self::MAX as i64) as Self
            }

            fn min(self, other: Self) -> Self {
                Ord::min(self, other)
            }

            fn max(self, other: Self) -> Self {
                Ord::max(self, other)
            }

            type PrimInt = $ty;

            fn into_fraction(self) -> Ratio<Self::PrimInt> {
                Ratio::new_raw(self, 1)
            }

            fn build_fraction(self, denom: Self) -> Ratio<Self::PrimInt> {
                Ratio::new(self, denom)
            }

            fn from_fraction(frac: Ratio<Self::PrimInt>) -> Self{
                frac.to_integer()
            }
        })*
    };
}

impl_int!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
);

macro_rules! impl_int_newtype {
    ($($base: ident {$($ty: ty),* $(,)?}),* $(,)?) => {
        $($(impl Int for $base<$ty> {
            const ZERO: Self = Self(0);
            const ONE: Self = Self(1);

            const MIN_VALUE: Self = Self(<$ty>::MIN);
            const MAX_VALUE: Self = Self(<$ty>::MAX);

            fn from_i64(value: i64) -> Self{
                Self(value.clamp(<$ty>::MIN as i64, <$ty>::MAX as i64) as $ty)
            }

            fn min(self, other: Self) -> Self {
                Ord::min(self, other)
            }

            fn max(self, other: Self) -> Self {
                Ord::max(self, other)
            }

            type PrimInt = $ty;

            fn into_fraction(self) -> Ratio<Self::PrimInt> {
                Ratio::new_raw(self.0, 1)
            }

            fn build_fraction(self, denom: Self) -> Ratio<Self::PrimInt> {
                Ratio::new(self.0, denom.0)
            }

            fn from_fraction(frac: Ratio<Self::PrimInt>) -> Self{
                Self(frac.to_integer())
            }
        })*)*
    };
}

impl_int_newtype!(
    Wrapping {
        u8, u16, u32, u64, u128, usize,
        i8, i16, i32, i64, i128, isize,
    },
    // Blocked on serde.
    // Saturating {
    //     u8, u16, u32, u64, u128, usize,
    //     i8, i16, i32, i64, i128, isize,
    // },
);

/// Trait for a floating point number or a [`Ratio`].
pub trait Float: NumOps + PartialOrd + Default + Copy + Serializable {
    const ZERO: Self;
    const ONE: Self;

    const MIN_VALUE: Self;
    const MAX_VALUE: Self;

    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;

    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn trunc(self) -> Self;
    fn round(self) -> Self;
}

impl Float for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const MIN_VALUE: Self = f32::NEG_INFINITY;
    const MAX_VALUE: Self = f32::INFINITY;

    fn min(self, other: Self) -> Self {
        self.min(other)
    }

    fn max(self, other: Self) -> Self {
        self.max(other)
    }

    fn floor(self) -> Self {
        self.floor()
    }

    fn ceil(self) -> Self {
        self.ceil()
    }

    fn trunc(self) -> Self {
        self.trunc()
    }

    fn round(self) -> Self {
        self.round()
    }
}

impl Float for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const MIN_VALUE: Self = f64::NEG_INFINITY;
    const MAX_VALUE: Self = f64::INFINITY;

    fn min(self, other: Self) -> Self {
        self.min(other)
    }

    fn max(self, other: Self) -> Self {
        self.max(other)
    }

    fn floor(self) -> Self {
        self.floor()
    }

    fn ceil(self) -> Self {
        self.ceil()
    }

    fn trunc(self) -> Self {
        self.trunc()
    }

    fn round(self) -> Self {
        self.round()
    }
}


#[derive(Debug, Clone, Copy, Default, TypePath, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent, bound(serialize = "", deserialize = ""))]
pub struct Ratio<I: Int + NumInteger>(num_rational::Ratio<I>);

impl<I: Int + NumInteger> Deref for Ratio<I> {
    type Target = num_rational::Ratio<I>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I: Int + NumInteger> DerefMut for Ratio<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<I: Int + NumInteger> Ratio<I> {
    pub fn new(numer: I, denom: I) -> Self{
        Self(num_rational::Ratio::new(numer, denom))
    }

    pub(crate) const fn new_raw(numer: I, denom: I) -> Self{
        Self(num_rational::Ratio::new_raw(numer, denom))
    }
}

impl<I: Int + NumInteger> Add for Ratio<I> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<I: Int + NumInteger> AddAssign for Ratio<I> {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl<I: Int + NumInteger> Sub for Ratio<I> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl<I: Int + NumInteger> Mul for Ratio<I> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl<I: Int + NumInteger> MulAssign for Ratio<I> {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 *= rhs.0
    }
}

impl<I: Int + NumInteger + Clone> Float for Ratio<I> {
    const ZERO: Self = Ratio::new_raw(I::ZERO, I::ONE);
    const ONE: Self = Ratio::new_raw(I::ONE, I::ONE);
    const MIN_VALUE: Self = Ratio::new_raw(I::MIN_VALUE, I::ONE);
    const MAX_VALUE: Self = Ratio::new_raw(I::MAX_VALUE, I::ONE);

    fn min(self, other: Self) -> Self {
        Ord::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        Ord::max(self, other)
    }

    fn floor(self) -> Self {
        Self(num_rational::Ratio::floor(&self.0))
    }

    fn ceil(self) -> Self {
        Self(num_rational::Ratio::ceil(&self.0))
    }

    fn trunc(self) -> Self {
        Self(num_rational::Ratio::trunc(&self.0))
    }

    fn round(self) -> Self {
        Self(num_rational::Ratio::round(&self.0))
    }
}
