use std::ops::*;

use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

use crate::{Float, Int, NumCast};

// Copied from the `gcd` crate by frewsxcv, MIT/Apache-2.0
macro_rules! gcd {
    ($x: expr, $y: expr) => {
        'gcd: {
            let mut u = $x;
            let mut v = $y;
            if u == 0 {
                break 'gcd v;
            }
            if v == 0 {
                break 'gcd u;
            }

            // abs
            #[allow(unused_comparisons)]
            if u < 0 {
                u = 0 - u
            };
            #[allow(unused_comparisons)]
            if v < 0 {
                v = 0 - v
            };

            let shift = (u | v).trailing_zeros();
            u >>= shift;
            v >>= shift;
            u >>= u.trailing_zeros();

            loop {
                v >>= v.trailing_zeros();
                #[allow(clippy::manual_swap)]
                if u > v {
                    // mem::swap(&mut u, &mut v);
                    let temp = u;
                    u = v;
                    v = temp;
                }
                v -= u; // here v >= u
                if v == 0 {
                    break;
                }
            }
            u << shift
        }
    };
}

pub(crate) use gcd;

/// Represents a fractional number.
///
/// # Type Contract
///
/// All combinations of numbers and signs are allowed, as long as denominator is not 0.
/// Some operations like `new` will perform reduction on the value while others won't for performance.
///
/// # Reductions
///
/// Only `new` does full reduction, operators only do partial reduction for powers of 2.
/// In the context of `bevy_stat_query`, use simple numbers like `1/3` over complicated
/// ones like `33/100` to avoid integer overflows.
#[derive(Debug, Clone, Copy, Default, TypePath, Serialize, Deserialize)]
#[repr(C)]
pub struct Fraction<I: Int> {
    numer: I,
    denom: I,
}

impl<T: Int> From<T> for Fraction<T> {
    fn from(value: T) -> Self {
        Self::from_int(value)
    }
}

impl<I: Int> PartialEq for Fraction<I> {
    fn eq(&self, other: &Self) -> bool {
        self.numer * other.denom == self.denom * other.numer
    }
}

impl<I: Int> Eq for Fraction<I> {}

impl<I: Int> PartialOrd for Fraction<I> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: Int> Ord for Fraction<I> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.numer * other.denom).cmp(&(self.denom * other.numer))
    }
}

macro_rules! impl_const_v {
    ($($ty: ident),* $(,)*) => {
        $(impl Fraction<$ty> {
            pub const fn const_new(numer: $ty, denom: $ty) -> Self {
                let gcd = gcd!(numer, denom);
                Fraction {
                    numer: numer / gcd,
                    denom: denom / gcd,
                }
            }

            pub const fn const_pct(percent: $ty) -> Self {
                Self::const_new(percent, 100)
            }

            pub const fn const_reduce(self) -> Self {
                Self::const_new(self.denom, self.numer)
            }
        })*
    };
}

impl_const_v!(u8, u16, u32, u64, u128, usize);
impl_const_v!(i8, i16, i32, i64, i128, isize);

impl<I: Int> Fraction<I> {
    pub fn new(numer: I, denom: I) -> Self {
        let gcd = numer.gcd(denom);
        Fraction {
            numer: numer / gcd,
            denom: denom / gcd,
        }
    }

    pub fn numer(&self) -> I {
        self.numer
    }

    pub fn denom(&self) -> I {
        self.denom
    }

    pub fn pct(percent: I) -> Self {
        Self::new(percent, I::from_i64(100))
    }

    pub fn reduced_pow2(mut self) -> Self {
        self.numer.fast_reduction(&mut self.denom);
        self
    }

    pub fn reduce(&mut self) {
        *self = Self::new(self.numer, self.denom)
    }

    pub fn reduced(self) -> Self {
        Self::new(self.numer, self.denom)
    }

    /// Create a unreduced fraction, does not breach the type contract as long as `denom` is not 0.
    pub const fn new_raw(numer: I, denom: I) -> Self {
        Self { numer, denom }
    }

    /// Create the fraction `value / 1`.
    pub const fn from_int(value: I) -> Self {
        Self::new_raw(value, I::ONE)
    }

    /// Returns true if number is less than zero.
    pub fn is_positive(&self) -> bool {
        !((self.numer < I::ZERO) ^ (self.denom < I::ZERO)) && self.numer != I::ZERO
    }

    /// Returns true if number is zero.
    pub fn is_zero(&self) -> bool {
        self.numer == I::ZERO
    }

    /// Returns true if number is less than zero.
    pub fn is_negative(&self) -> bool {
        (self.numer < I::ZERO) ^ (self.denom < I::ZERO) && self.numer != I::ZERO
    }

    pub fn floor(self) -> I {
        if self.is_negative() {
            (self.numer - self.denom + self.denom.signum()) / self.denom
        } else {
            self.numer / self.denom
        }
    }

    pub fn ceil(self) -> I {
        if self.is_negative() {
            self.numer / self.denom
        } else {
            (self.numer + self.denom - self.denom.signum()) / self.denom
        }
    }

    pub fn trunc(self) -> I {
        self.numer / self.denom
    }

    pub fn round(self) -> I {
        if self.is_negative() {
            (self.numer - self.denom / (I::ONE + I::ONE)) / self.denom
        } else {
            (self.numer + self.denom / (I::ONE + I::ONE)) / self.denom
        }
    }
}

impl<I: Int> NumCast<I> for Fraction<I> {
    fn cast(self) -> I {
        self.trunc()
    }
}

impl<I: Int> NumCast<Fraction<I>> for I {
    fn cast(self) -> Fraction<I> {
        Fraction::from_int(self)
    }
}

macro_rules! impl_ops {
    ($t1: ident, $f1: ident, $t2: ident, $f2: ident, $a: ident, $b: ident, $e1: expr, $e2: expr) => {
        impl<T: Int> $t1<Self> for Fraction<T> {
            type Output = Self;

            fn $f1(self, rhs: Self) -> Self::Output {
                let $a = self;
                let $b = rhs;
                Fraction {
                    numer: $e1,
                    denom: $e2,
                }
                .reduced_pow2()
            }
        }

        impl<T: Int> $t1<T> for Fraction<T> {
            type Output = Self;

            fn $f1(self, rhs: T) -> Self::Output {
                let $a = self;
                let $b = Fraction::from_int(rhs);
                Fraction {
                    numer: $e1,
                    denom: $e2,
                }
                .reduced_pow2()
            }
        }

        impl<T: Int> $t2<Self> for Fraction<T> {
            fn $f2(&mut self, rhs: Self) {
                let $a = *self;
                let $b = rhs;
                *self = Fraction {
                    numer: $e1,
                    denom: $e2,
                }
                .reduced_pow2()
            }
        }

        impl<T: Int> $t2<T> for Fraction<T> {
            fn $f2(&mut self, rhs: T) {
                let $a = *self;
                let $b = Fraction::from_int(rhs);
                *self = Fraction {
                    numer: $e1,
                    denom: $e2,
                }
                .reduced_pow2()
            }
        }
    };
}

impl_ops!(
    Add,
    add,
    AddAssign,
    add_assign,
    a,
    b,
    a.numer * b.denom + a.denom * b.numer,
    a.denom * b.denom
);
impl_ops!(
    Sub,
    sub,
    SubAssign,
    sub_assign,
    a,
    b,
    a.numer * b.denom - a.denom * b.numer,
    a.denom * b.denom
);
impl_ops!(
    Mul,
    mul,
    MulAssign,
    mul_assign,
    a,
    b,
    a.numer * b.numer,
    a.denom * b.denom
);
impl_ops!(
    Div,
    div,
    DivAssign,
    div_assign,
    a,
    b,
    a.numer * b.denom,
    a.denom * b.numer
);

impl<I: Int + Clone> Float for Fraction<I> {
    const ZERO: Self = Fraction::new_raw(I::ZERO, I::ONE);
    const ONE: Self = Fraction::new_raw(I::ONE, I::ONE);
    const MIN_VALUE: Self = Fraction::new_raw(I::MIN_VALUE, I::ONE);
    const MAX_VALUE: Self = Fraction::new_raw(I::MAX_VALUE, I::ONE);

    fn min(self, other: Self) -> Self {
        Ord::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        Ord::max(self, other)
    }

    fn floor(self) -> Self {
        Fraction::from_int(Fraction::floor(self))
    }

    fn ceil(self) -> Self {
        Fraction::from_int(Fraction::ceil(self))
    }

    fn trunc(self) -> Self {
        Fraction::from_int(Fraction::trunc(self))
    }

    fn round(self) -> Self {
        Fraction::from_int(Fraction::round(self))
    }
}
