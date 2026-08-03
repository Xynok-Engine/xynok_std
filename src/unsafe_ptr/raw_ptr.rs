use core::fmt;

use crate::unsafe_ptr::{raw_ref::RawRef, raw_ref_mut::RawRefMut};

/// Owning raw pointer. Heap-allocates via `Box` and frees on drop.
/// Not `Copy` or `Clone` — sole owner of the allocation.
pub struct RawPtr<T>
{
    ptr: *const T,
}

impl<T> RawPtr<T>
{
    pub fn new(val: T) -> Self { Self { ptr: Box::into_raw(Box::new(val)) as *const T } }

    pub fn as_ref(&self) -> RawRef<T> { RawRef { ptr: self.ptr } }
    pub fn as_ref_mut(&self) -> RawRefMut<T> { RawRefMut { ptr: self.ptr } }

    pub(crate) fn get(&self) -> &T { unsafe { &*self.ptr } }
    pub(crate) fn get_mut(&mut self) -> &mut T { unsafe { &mut *(self.ptr as *mut T) } }
}

impl<T> Drop for RawPtr<T>
{
    fn drop(&mut self) { unsafe { drop(Box::from_raw(self.ptr as *mut T)) } }
}

impl<T> std::ops::Deref for RawPtr<T>
{
    type Target = T;
    fn deref(&self) -> &T { self.get() }
}
impl<T> std::ops::DerefMut for RawPtr<T>
{
    fn deref_mut(&mut self) -> &mut T { self.get_mut() }
}

unsafe impl<T> Send for RawPtr<T> {}
unsafe impl<T> Sync for RawPtr<T> {}

impl<T: fmt::Debug> fmt::Debug for RawPtr<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Debug::fmt(self.get(), f) }
}
impl<T: fmt::Display> fmt::Display for RawPtr<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self.get(), f) }
}
impl<T: PartialEq> PartialEq for RawPtr<T>
{
    fn eq(&self, other: &Self) -> bool { self.get() == other.get() }
}
impl<T: Eq> Eq for RawPtr<T> {}
impl<T: PartialOrd> PartialOrd for RawPtr<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { self.get().partial_cmp(other.get()) }
}
impl<T: Ord> Ord for RawPtr<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.get().cmp(other.get()) }
}
impl<T: std::hash::Hash> std::hash::Hash for RawPtr<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.get().hash(state) }
}
