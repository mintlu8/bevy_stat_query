use std::fmt::Debug;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use crate::{calc::StatOperation, num_traits::Flags, Shareable};

use super::{StatComponents, Unsupported};

/// A flags based on a type that supports bitwise operations,
/// like integer, `bitflgs` or `enumset`.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct StatFlags<T: Flags> {
    pub not: T,
    pub or: T,
}

impl<T: Flags> StatComponents for StatFlags<T> {
    type Out = T;

    fn join(&mut self, other: Self) {
        self.or |= other.or;
        self.not |=  other.not;
    }

    fn eval(&self) -> Self::Out {
        self.or.clone().exclude(self.not.clone())
    }

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    type Bit = T;

    fn or(&mut self, other: Self::Bit) {
        self.or |= other
    }

    fn not(&mut self, other: Self::Bit) {
        self.not |= other
    }

    fn from_out(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Or(out)
    }
}

/// A stat flags backed by a `HashSet`.
/// Use [`StatFlags`] if possible for better performance.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StatSet<T: Shareable + Hash + Eq + Default> {
    pub not: FxHashSet<T>,
    pub or: FxHashSet<T>,
}

impl<T: Shareable + Hash + Eq + Default> StatComponents for StatSet<T> {
    type Out = FxHashSet<T>;

    fn join(&mut self, other: Self) {
        self.not.extend(other.not);
        self.or.extend(other.or);
    }

    fn eval(&self) -> Self::Out {
        let mut result = self.or.clone();
        for item in &self.not {
            result.remove(item);
        }
        result
    }

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    type Bit = FxHashSet<T>;

    fn or(&mut self, other: Self::Bit) {
        self.or.extend(other);
    }

    fn not(&mut self, other: Self::Bit) {
        self.not.extend(other);
    }

    fn from_out(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Or(out)
    }
}
