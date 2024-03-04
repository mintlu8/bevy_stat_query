use std::{any::type_name, fmt::Debug};
use crate::{calc::StatOperation, Shareable};

use super::{StatValue, Unsupported};

/// Find if a stat exists.
#[derive(Debug, Default, Clone, Copy)]
pub struct StatExists(bool);

impl StatValue for StatExists {
    type Out = bool;

    type Bit = bool;

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    fn join(&mut self, other: Self) {
        self.0 = other.0;
    }
    
    fn eval(&self) -> Self::Out {
        self.0
    }
    
    fn from_base(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Or(out)
    }

    fn or(&mut self, other: Self::Bit) {
        self.0 |= other
    }
}

/// Finds a single entry of a given stat.
///
/// # Panics
///
/// If no value or more than one value found if `eval` is called.
/// This behavior depends on the const generic values supplied.
/// if not, returns the default value.
#[derive(Debug, Default, Clone, Copy)]
pub enum StatSingleton<T: PartialEq + Shareable + Default, const PANIC_NOT_FOUND: bool=true, const PANIC_MULTIPLE_FOUND: bool=true> {
    #[default]
    NotFound,
    Found(T),
    FoundMultiple,
}

impl<T: PartialEq + Shareable + Default, const PNF: bool, const PMF: bool> StatValue for StatSingleton<T, PNF, PMF> {
    type Out = T;

    fn join(&mut self, other: Self) {
        match (&self, other) {
            (StatSingleton::FoundMultiple, _) => (),
            (_, StatSingleton::FoundMultiple) => {
                *self = StatSingleton::FoundMultiple
            },
            (StatSingleton::Found(a), StatSingleton::Found(b)) => {
                if a != &b {
                    *self = StatSingleton::FoundMultiple
                }
            },
            (StatSingleton::Found(_), StatSingleton::NotFound) => (),
            (StatSingleton::NotFound, StatSingleton::Found(a)) => {
                *self = StatSingleton::Found(a);
            }
            (StatSingleton::NotFound, StatSingleton::NotFound) => (),
        }
    }

    fn eval(&self) -> Self::Out {
        match self {
            StatSingleton::NotFound => if PNF {panic!(
                "StatSingleton<{}> found no matching value.",
                type_name::<T>()
            )} else {
                Default::default()
            },
            StatSingleton::Found(some) => some.clone(),
            StatSingleton::FoundMultiple => if PMF {panic!(
                "StatSingleton<{}> found multiple matching values.",
                type_name::<T>()
            )} else {
                Default::default()
            }
        }
    }

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    type Bit = T;

    fn or(&mut self, other: Self::Bit) {
        match self {
            StatSingleton::NotFound => *self = Self::Found(other),
            StatSingleton::Found(val) => {
                if val != &other {
                    *self = StatSingleton::FoundMultiple;
                }
            },
            StatSingleton::FoundMultiple => (),
        }
    }

    fn from_base(out: Self::Out) -> StatOperation<Self> {
        StatOperation::Or(out)
    }
}
