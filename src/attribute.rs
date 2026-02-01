/// Represents either a string or a typed enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute<'t> {
    String(&'t str),
    Enum { tag: usize, index: u64 },
}

impl Attribute<'_> {
    pub fn is<T: AsAttribute + ?Sized>(&self, item: &T) -> bool {
        *self == item.as_attribute()
    }
}

impl<T: AsAttribute + ?Sized> PartialEq<T> for Attribute<'_> {
    fn eq(&self, other: &T) -> bool {
        self.is(other)
    }
}

impl<'t> From<&'t str> for Attribute<'t> {
    fn from(val: &'t str) -> Self {
        Attribute::String(val)
    }
}

pub trait AsAttribute {
    fn as_attribute(&self) -> Attribute<'_>;
}

impl<T: AsAttribute + ?Sized> AsAttribute for &T {
    fn as_attribute(&self) -> Attribute<'_> {
        (*self).as_attribute()
    }
}

impl AsAttribute for Attribute<'_> {
    fn as_attribute(&self) -> Attribute<'_> {
        match self {
            Attribute::String(s) => Attribute::String(s),
            Attribute::Enum { tag, index } => Attribute::Enum { tag: *tag, index: *index },
        }
    }
}


impl AsAttribute for str {
    fn as_attribute(&self) -> Attribute<'_> {
        Attribute::String(self)
    }
}

impl AsAttribute for String {
    fn as_attribute(&self) -> Attribute<'_> {
        Attribute::String(self)
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
