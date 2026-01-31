mod flags;
mod int_pct;
mod number;
mod prioritized;
mod rounded;
pub use flags::StatFlags;
pub use int_pct::StatIntPercentAdditive;
pub use number::{StatAdditive, StatMultiplicative, StatMultiplied};
pub use prioritized::{GetPrioritized, Prioritized};
pub use rounded::StatRounded;

#[cfg(feature = "serde")]
#[allow(unused)]
mod util {
    use crate::Number;

    pub fn num_zero<N: Number>() -> N {
        N::ZERO
    }

    pub fn num_one<N: Number>() -> N {
        N::ONE
    }

    pub fn num_min<N: Number>() -> N {
        N::MIN_VALUE
    }

    pub fn num_max<N: Number>() -> N {
        N::MAX_VALUE
    }

    pub fn if_num_zero<N: Number>(n: &N) -> bool {
        n == &N::ZERO
    }

    pub fn if_num_one<N: Number>(n: &N) -> bool {
        n == &N::ONE
    }

    pub fn if_num_min<N: Number>(n: &N) -> bool {
        n == &N::MIN_VALUE
    }

    pub fn if_num_max<N: Number>(n: &N) -> bool {
        n == &N::MAX_VALUE
    }
}
