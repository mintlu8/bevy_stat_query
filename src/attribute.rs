/// Represents either a string or a typed enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute<'t> {
    String(&'t str),
    Enum { tag: usize, index: u64 },
}

impl Attribute<'_> {
    pub fn is<'t, T: ?Sized>(&self, item: &'t T) -> bool
    where
        &'t T: Into<Attribute<'t>>,
    {
        *self == item.into()
    }
}

impl PartialEq<String> for Attribute<'_> {
    fn eq(&self, other: &String) -> bool {
        self.is(other)
    }
}

impl PartialEq<&str> for Attribute<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.is(other)
    }
}

impl<T: ?Sized> PartialEq<T> for Attribute<'_>
where
    for<'t> &'t T: Into<Attribute<'t>>,
{
    fn eq(&self, other: &T) -> bool {
        self.is(other)
    }
}

impl<'t> From<&'t str> for Attribute<'t> {
    fn from(val: &'t str) -> Self {
        Attribute::String(val)
    }
}

#[cfg(test)]
mod test {
    use crate::Attribute;

    #[test]
    fn test_attribute_eq() {
        assert!(Attribute::String("Hello").is("Hello"));
        assert_eq!(Attribute::String("Hello"), "Hello");
        assert_eq!(Attribute::String("Hello"), "Hello".to_owned());
    }
}
