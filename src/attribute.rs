/// Represents either a string or a typed enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute<'t> {
    String(&'t str),
    Enum { tag: usize, index: u64 },
}

impl<'t> From<&'t str> for Attribute<'t> {
    fn from(val: &'t str) -> Self {
        Attribute::String(val)
    }
}
