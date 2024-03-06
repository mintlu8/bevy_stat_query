use std::fmt::Debug;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use crate::{calc::StatOperation, Serializable};
use super::{StatValue, Unsupported};

/// Find if a stat exists.
#[derive(Debug, Default, Clone, Copy, TypePath, Serialize, Deserialize)]
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
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TypePath, Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub enum StatOnce<T: PartialEq + Serializable> {
    #[default]
    NotFound,
    Found(T),
    FoundMultiple,
}

impl<T: PartialEq + Serializable> StatOnce<T> {
    pub fn unwrap(self) -> T {
        self.into_option().unwrap()
    }

    pub fn unwrap_or(self, or: T) -> T {
        self.into_option().unwrap_or(or)
    }

    pub fn unwrap_or_else(self, or: impl Fn() -> T) -> T {
        self.into_option().unwrap_or_else(or)
    }

    pub fn expect(self, msg: &str) -> T {
        self.into_option().expect(msg)
    }

    pub fn is_some(&self) -> bool {
        matches!(self, StatOnce::Found(_))
    }

    pub fn is_none(&self) -> bool {
        !matches!(self, StatOnce::Found(_))
    }

    pub fn into_option(self) -> Option<T> {
        match self {
            StatOnce::Found(r) => Some(r),
            _ => None
        }
    }

    pub fn as_ref(&self) -> Option<&T> {
        match self {
            StatOnce::Found(r) => Some(r),
            _ => None
        }
    }
}

impl<T: PartialEq + Serializable> StatValue for StatOnce<T> {
    type Out = StatOnce<T>;

    fn join(&mut self, other: Self) {
        match (&self, other) {
            (StatOnce::FoundMultiple, _) => (),
            (_, StatOnce::FoundMultiple) => {
                *self = StatOnce::FoundMultiple
            },
            (StatOnce::Found(a), StatOnce::Found(b)) => {
                if a != &b {
                    *self = StatOnce::FoundMultiple
                }
            },
            (StatOnce::Found(_), StatOnce::NotFound) => (),
            (StatOnce::NotFound, StatOnce::Found(a)) => {
                *self = StatOnce::Found(a);
            }
            (StatOnce::NotFound, StatOnce::NotFound) => (),
        }
    }

    fn eval(&self) -> Self::Out {
        self.clone()
    }

    type Add = Unsupported;
    type Mul = Unsupported;
    type Bounds = Unsupported;

    type Bit = T;

    fn or(&mut self, other: Self::Bit) {
        match self {
            StatOnce::NotFound => *self = Self::Found(other),
            StatOnce::Found(val) => {
                if val != &other {
                    *self = StatOnce::FoundMultiple;
                }
            },
            StatOnce::FoundMultiple => (),
        }
    }

    fn from_base(out: Self::Out) -> StatOperation<Self> {
        match out {
            StatOnce::Found(f) => StatOperation::Or(f),
            _ => panic!("Base stat has to be a concrete value."),
        }
    }
}
