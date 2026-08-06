use core::fmt;

use crate::unsafe_ptr::heap_ref::HeapRef;
use crate::unsafe_ptr::heap_ref_mut::HeapRefMut;

/// Owning raw pointer. Heap-allocates via `Box` and frees on drop.
/// Not `Copy` or `Clone` — sole owner of the allocation.
pub struct HeapPtr<T>
{
    ptr: *const T,
}

impl<T> HeapPtr<T>
{
    pub fn new(val: T) -> Self
    {
        Self {
            ptr: Box::into_raw(Box::new(val)) as *const T,
        }
    }

    pub fn as_ref(&self) -> HeapRef<T>
    {
        HeapRef { ptr: self.ptr }
    }
    pub fn as_ref_mut(&self) -> HeapRefMut<T>
    {
        HeapRefMut { ptr: self.ptr }
    }

    pub(crate) fn get(&self) -> &T
    {
        unsafe { &*self.ptr }
    }
    pub(crate) fn get_mut(&mut self) -> &mut T
    {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

impl<T> Drop for HeapPtr<T>
{
    fn drop(&mut self)
    {
        unsafe { drop(Box::from_raw(self.ptr as *mut T)) }
    }
}

impl<T> std::ops::Deref for HeapPtr<T>
{
    type Target = T;
    fn deref(&self) -> &T
    {
        self.get()
    }
}
impl<T> std::ops::DerefMut for HeapPtr<T>
{
    fn deref_mut(&mut self) -> &mut T
    {
        self.get_mut()
    }
}

unsafe impl<T> Send for HeapPtr<T> {}
unsafe impl<T> Sync for HeapPtr<T> {}

impl<T: fmt::Debug> fmt::Debug for HeapPtr<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(self.get(), f)
    }
}
impl<T: fmt::Display> fmt::Display for HeapPtr<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(self.get(), f)
    }
}
impl<T: PartialEq> PartialEq for HeapPtr<T>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.get() == other.get()
    }
}
impl<T: Eq> Eq for HeapPtr<T> {}
impl<T: PartialOrd> PartialOrd for HeapPtr<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        self.get().partial_cmp(other.get())
    }
}
impl<T: Ord> Ord for HeapPtr<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
        self.get().cmp(other.get())
    }
}
impl<T: std::hash::Hash> std::hash::Hash for HeapPtr<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        self.get().hash(state)
    }
}
