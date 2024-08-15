use std::{
    any::type_name,
    cell::UnsafeCell,
    mem::{align_of, size_of, MaybeUninit},
};

#[inline(always)]
pub(crate) fn validate<T>() {
    if !matches!(align_of::<T>(), 1 | 2 | 4 | 8) {
        panic!(
            "{} has alignment {}. Stat::Value can only be values with alignments 1, 2, 4 or 8.",
            type_name::<T>(),
            align_of::<T>()
        )
    }
    if size_of::<T>() > 24 {
        panic!(
            "{} has size {}. Stat::Value can only be values up to 24 bytes.",
            type_name::<T>(),
            size_of::<T>()
        )
    }
}

/// A type that should be able to hold everything in rust within constraints.
///
/// # Compatibility
///
/// This version requires [`UnsafeCell`] for soundness, if `Freeze` is stabilized,
/// we might drop [`UnsafeCell`] for performance, thus preventing internally mutable
/// types like `Mutex` from being used as `StatValue`.
#[repr(C, align(8))]
pub struct Buffer(UnsafeCell<[MaybeUninit<u8>; 24]>);

unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    /// Convert to a concrete item.
    pub(crate) unsafe fn as_ref<T: Send + Sync>(&self) -> &T {
        validate::<T>();
        unsafe { (self.0.get() as *const T).as_ref() }.unwrap()
    }

    /// Convert to a concrete item.
    pub(crate) unsafe fn as_mut<T: Send + Sync>(&mut self) -> &mut T {
        validate::<T>();
        unsafe { (self.0.get_mut().as_ptr() as *mut T).as_mut() }.unwrap()
    }

    /// Convert to a concrete item.
    pub(crate) unsafe fn into<T: Send + Sync>(mut self) -> T {
        validate::<T>();
        unsafe { (self.0.get_mut().as_ptr() as *mut T).read() }
    }

    /// Convert from a concrete item.
    pub(crate) fn from<T: Send + Sync>(item: T) -> Self {
        validate::<T>();
        let mut buffer = [MaybeUninit::uninit(); 24];
        unsafe { (buffer.as_mut_ptr() as *mut T).write(item) };
        Buffer(UnsafeCell::new(buffer))
    }

    /// Read from a mutable reference to buffer.
    ///
    /// # Safety
    ///
    /// Buffer must not be read after and should be dropped immediately.
    pub(crate) unsafe fn read_move<T: Send + Sync>(&mut self) -> T {
        validate::<T>();
        unsafe { (self.0.get_mut().as_ptr() as *mut T).read() }
    }
}
