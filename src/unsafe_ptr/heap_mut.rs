use core::fmt;

use crate::unsafe_ptr::heap_ref::HeapRef;

/// Non-owning mutable raw pointer. Equivalent to `&mut T` but without lifetimes.
pub struct HeapMut<T>
{
    pub(crate) ptr: *const T,
}

impl<T> HeapMut<T>
{
    #[inline]
    pub(crate) fn get(&self) -> &T
    {
        unsafe { &*self.ptr }
    }
    /// Returns a raw pointer, bypassing the borrow checker
    #[inline]
    pub fn as_ref(&self) -> HeapRef<T>
    {
        HeapRef { ptr: self.ptr }
    }
    /// Returns a pointer with a lifetime that respects Rust's borrow checker rules.
    /// Sometimes, to clarify the intent behind this pointer, you may need to explicitly declare the lifetime when calling this function.
    /// However, the Rust compiler is usually smart enough to infer the correct lifetime, so just be mindful of whether this pointer is intended for a temporary local scope or for long-term storage.
    /// If you store it, the struct or enum holding it will require an explicit lifetime.
    /// Boom! That is exactly why Rust requires you to declare lifetimes for structs that contain pointers.
    #[inline]
    pub fn as_ref_with_caller_lifetime<'a>(&self) -> &'a T
    {
        unsafe { &*self.ptr }
    }
    #[inline]
    pub fn as_ref_mut<'a>(&self) -> &'a mut T
    {
        unsafe { &mut *(self.ptr as *mut T) }
    }
    #[inline]
    pub fn ptr(&self) -> *const T
    {
        self.ptr
    }
}

impl<T> std::ops::Deref for HeapMut<T>
{
    type Target = T;
    fn deref(&self) -> &T
    {
        unsafe { &*self.ptr }
    }
}
impl<T> std::ops::DerefMut for HeapMut<T>
{
    fn deref_mut(&mut self) -> &mut T
    {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

impl<T> Clone for HeapMut<T>
{
    fn clone(&self) -> Self
    {
        *self
    }
}
impl<T> Copy for HeapMut<T> {}
unsafe impl<T> Send for HeapMut<T> {}
unsafe impl<T> Sync for HeapMut<T> {}

impl<T: fmt::Debug> fmt::Debug for HeapMut<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(self.get(), f)
    }
}
impl<T: fmt::Display> fmt::Display for HeapMut<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(self.get(), f)
    }
}
impl<T: PartialEq> PartialEq for HeapMut<T>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.get() == other.get()
    }
}
impl<T: Eq> Eq for HeapMut<T> {}
impl<T: PartialOrd> PartialOrd for HeapMut<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        self.get().partial_cmp(other.get())
    }
}
impl<T: Ord> Ord for HeapMut<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
        self.get().cmp(other.get())
    }
}
impl<T: std::hash::Hash> std::hash::Hash for HeapMut<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        self.get().hash(state)
    }
}
