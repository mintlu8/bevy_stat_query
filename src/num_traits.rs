use crate::{
    fraction::{gcd, Fraction},
    Shareable,
};
use std::{
    fmt::Debug,
    num::{Saturating, Wrapping},
    ops::*,
};

pub trait NumOps:
    Sized
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + AddAssign<Self>
    + MulAssign<Self>
{
}

impl<T> NumOps for T where
    T: Sized
        + Add<Self, Output = Self>
        + Sub<Self, Output = Self>
        + Mul<Self, Output = Self>
        + AddAssign<Self>
        + MulAssign<Self>
{
}

pub trait BitOps:
    Sized
    + BitAnd<Self, Output = Self>
    + BitOr<Self, Output = Self>
    + BitXor<Self, Output = Self>
    + BitAndAssign<Self>
    + BitOrAssign<Self>
    + BitXorAssign<Self>
{
}

impl<T> BitOps for T where
    T: Sized
        + BitAnd<Self, Output = Self>
        + BitOr<Self, Output = Self>
        + BitXor<Self, Output = Self>
        + BitAndAssign<Self>
        + BitOrAssign<Self>
        + BitXorAssign<Self>
{
}

/// A type that can be treated as flags.
///
/// Automatically implemented on types implementing all three bitwise operations `&|^`.
pub trait Flags:
    BitOr<Self, Output = Self> + BitOrAssign<Self> + Debug + Default + Shareable
{
    /// Exclude a portion of the flags.
    fn exclude(self, other: Self) -> Self;
}

impl<T> Flags for T
where
    T: BitOps + Debug + Default + Shareable,
{
    fn exclude(self, other: Self) -> Self {
        self.clone() ^ (self & other)
    }
}

/// Trait for an integer.
pub trait Int:
    NumOps + Div<Self, Output = Self> + BitOps + Ord + Default + Copy + Shareable
{
    const ZERO: Self;
    const ONE: Self;

    const MIN_VALUE: Self;
    const MAX_VALUE: Self;

    fn from_i64(value: i64) -> Self;

    fn as_f32(self) -> f32;
    fn as_f64(self) -> f64;

    fn from_f32(value: f32) -> Self;
    fn from_f64(value: f64) -> Self;

    fn abs(self) -> Self;
    fn signum(self) -> Self;

    fn gcd(self, other: Self) -> Self;
    #[doc(hidden)]
    fn fast_reduction(&mut self, other: &mut Self);

    type PrimInt: Int + Clone + Shareable;

    fn into_fraction(self) -> Fraction<Self::PrimInt>;
    fn build_fraction(self, denom: Self) -> Fraction<Self::PrimInt>;
    fn from_fraction(frac: Fraction<Self::PrimInt>) -> Self;
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

            fn as_f32(self) -> f32 {
                self as f32
            }

            fn as_f64(self) -> f64 {
                self as f64
            }

            fn from_f32(value: f32) -> Self{
                value as Self
            }

            fn from_f64(value: f64) -> Self{
                value as Self
            }

            fn abs(self) -> Self {
                #[allow(unused_comparisons)]
                if self < 0 {
                    0 - self
                } else {
                    self
                }
            }

            fn signum(self) -> Self {
                #[allow(unused_comparisons, arithmetic_overflow)]
                if self < 0 {
                    0 - 1
                } else if self == 0 {
                    0
                } else {
                    1
                }
            }

            fn fast_reduction(&mut self, other: &mut Self) {
                let u = self.abs();
                let v = other.abs();
                let shift = (u | v).trailing_zeros();
                *self >>= shift;
                *other >>= shift;
            }

            fn gcd(self, other: Self) -> Self {
                gcd!(self, other)
            }

            type PrimInt = $ty;

            fn into_fraction(self) -> Fraction<Self::PrimInt> {
                Fraction::new_raw(self, 1)
            }

            fn build_fraction(self, denom: Self) -> Fraction<Self::PrimInt> {
                Fraction::new(self, denom)
            }

            fn from_fraction(frac: Fraction<Self::PrimInt>) -> Self{
                frac.trunc()
            }
        })*
    };
}

impl_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize,);

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

            fn as_f32(self) -> f32 {
                self.0 as f32
            }

            fn as_f64(self) -> f64 {
                self.0 as f64
            }

            fn from_f32(value: f32) -> Self{
                Self(value as $ty)
            }

            fn from_f64(value: f64) -> Self{
                Self(value as $ty)
            }

            fn abs(self) -> Self {
                #[allow(unused_comparisons)]
                if self < Self(0) {
                    Self(0 - self.0)
                } else {
                    self
                }
            }

            fn signum(self) -> Self {
                #[allow(unused_comparisons, arithmetic_overflow)]
                Self(if self.0 < 0 {
                    0 - 1
                } else if self.0 == 0 {
                    0
                } else {
                    1
                })
            }


            fn fast_reduction(&mut self, other: &mut Self) {
                let u = self.0.abs();
                let v = other.0.abs();
                let shift = (u | v).trailing_zeros();
                self.0 >>= shift;
                other.0 >>= shift;
            }

            fn gcd(self, other: Self) -> Self {
                Self(gcd!(self.0, other.0))
            }

            type PrimInt = $ty;

            fn into_fraction(self) -> Fraction<Self::PrimInt> {
                Fraction::new_raw(self.0, 1)
            }

            fn build_fraction(self, denom: Self) -> Fraction<Self::PrimInt> {
                Fraction::new(self.0, denom.0)
            }

            fn from_fraction(frac: Fraction<Self::PrimInt>) -> Self{
                Self(frac.trunc())
            }
        })*)*
    };
}

impl_int_newtype!(
    Wrapping {
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
    },
    Saturating {
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
    },
);

/// Trait for a floating point number or a [`Fraction`].
pub trait Float: NumOps + PartialOrd + Default + Copy + Shareable {
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
    const MIN_VALUE: Self = f32::MIN;
    const MAX_VALUE: Self = f32::MAX;

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
    const MIN_VALUE: Self = f64::MIN;
    const MAX_VALUE: Self = f64::MAX;

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

pub trait NumCast<T> {
    fn cast(self) -> T;
}

impl<I: Int> NumCast<f32> for I {
    fn cast(self) -> f32 {
        self.as_f32()
    }
}

impl<I: Int> NumCast<f64> for I {
    fn cast(self) -> f64 {
        self.as_f64()
    }
}

impl<I: Int> NumCast<I> for f32 {
    fn cast(self) -> I {
        I::from_f32(self)
    }
}

impl<I: Int> NumCast<I> for f64 {
    fn cast(self) -> I {
        I::from_f64(self)
    }
}
