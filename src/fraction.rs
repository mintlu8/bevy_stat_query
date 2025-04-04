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
/// # Note
///
/// Unlike other implementations, operators like `add` or `mul` does not
/// perform reductions, this makes aggregation faster, but be careful
/// not use overuse complicated fractions like `33/100` that can cause
/// integer overflows if aggregated.
///
/// All overflows are unspecified behavior.
#[derive(Debug, Clone, Copy, Default, TypePath, PartialEq, Eq, Serialize, Deserialize)]
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

impl Fraction<u32> {
    pub const fn const_new(numer: u32, denom: u32) -> Self {
        let gcd = gcd!(numer, denom);
        Fraction {
            numer: numer / gcd,
            denom: denom / gcd,
        }
    }

    pub const fn const_pct(percent: u32) -> Self {
        Self::const_new(percent, 100)
    }

    pub const fn const_reduce(self) -> Self {
        Self::const_new(self.denom, self.numer)
    }
}

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

    pub fn reduce(self) -> Self {
        Self::new(self.denom, self.numer)
    }

    pub const fn new_raw(numer: I, denom: I) -> Self {
        Self { numer, denom }
    }

    pub const fn from_int(value: I) -> Self {
        Self::new_raw(value, I::ONE)
    }

    pub fn is_negative(&self) -> bool {
        self.numer < I::ZERO || self.denom < I::ZERO
    }

    pub fn floor(self) -> I {
        if self.is_negative() {
            (self.numer - self.denom + I::ONE) / self.denom
        } else {
            self.numer / self.denom
        }
    }

    pub fn ceil(self) -> I {
        if self.is_negative() {
            (self.numer - self.denom) / self.denom
        } else {
            (self.numer + self.denom - I::ONE) / self.denom
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

impl<T: Int> Add<Self> for Fraction<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Fraction {
            numer: self.numer * rhs.denom + rhs.numer * self.denom,
            denom: self.denom * rhs.denom,
        }
    }
}

impl<T: Int> AddAssign<Self> for Fraction<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.numer = self.numer * rhs.denom + rhs.numer * self.denom;
        self.denom *= rhs.denom;
    }
}

impl<T: Int> Sub<Self> for Fraction<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Fraction {
            numer: self.numer * rhs.denom - rhs.numer * self.denom,
            denom: self.denom * rhs.denom,
        }
    }
}

impl<T: Int> SubAssign<Self> for Fraction<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.numer = self.numer * rhs.denom - rhs.numer * self.denom;
        self.denom *= rhs.denom;
    }
}

impl<T: Int> Mul<T> for Fraction<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Fraction {
            numer: self.numer * rhs,
            denom: self.denom,
        }
    }
}

impl<T: Int> Mul<Self> for Fraction<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Fraction {
            numer: self.numer * rhs.numer,
            denom: self.denom * rhs.denom,
        }
    }
}

impl<T: Int> MulAssign<T> for Fraction<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.numer *= rhs;
    }
}

impl<T: Int> MulAssign<Self> for Fraction<T> {
    fn mul_assign(&mut self, rhs: Self) {
        self.numer *= rhs.numer;
        self.denom *= rhs.denom;
    }
}

impl<T: Int> Div<Self> for Fraction<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Fraction {
            numer: self.numer * rhs.denom,
            denom: self.denom * rhs.numer,
        }
    }
}

#[expect(clippy::suspicious_op_assign_impl)]
impl<T: Int> DivAssign<T> for Fraction<T> {
    fn div_assign(&mut self, rhs: T) {
        self.denom *= rhs;
    }
}

impl<T: Int> DivAssign<Self> for Fraction<T> {
    fn div_assign(&mut self, rhs: Self) {
        self.numer *= rhs.denom;
        self.denom *= rhs.numer;
    }
}

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
