use serde::{Deserialize, Serialize};

use crate::types::StatValue;

/// An single step unordered operation on a [`StatValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum StatOperation<S: StatValue> {
    Add(S::Add),
    Mul(S::Mul),
    Or(S::Bit),
    Not(S::Bit),
    Min(S::Bounds),
    Max(S::Bounds),
    Base(S::Base),
}

pub use StatOperation::*;

impl<S: StatValue> StatOperation<S> {
    pub fn write_to(&self, to: &mut S) {
        match self.clone() {
            StatOperation::Add(item) => to.add(item),
            StatOperation::Mul(item) => to.mul(item),
            StatOperation::Or(item) => to.or(item),
            StatOperation::Not(item) => to.not(item),
            StatOperation::Min(item) => to.min(item),
            StatOperation::Max(item) => to.max(item),
            StatOperation::Base(item) => *to = S::from_base(item),
        }
    }

    pub fn into_stat(self) -> S {
        let mut v = S::default();
        self.write_to(&mut v);
        v
    }
}
