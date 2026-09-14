use core::fmt;

/// A non-owning immutable raw pointer. It’s equivalent to `&T` but without lifetimes.
pub struct HeapRef<T>
{
    pub(crate) ptr: *const T,
}

impl<T> HeapRef<T>
{
    #[inline]
    pub fn from_raw(input: *const ()) -> Self
    {
        Self { ptr: input as *const T }
    }
    #[inline]
    pub fn get(&self) -> &T
    {
        unsafe { &*self.ptr }
    }
}

impl<T> std::ops::Deref for HeapRef<T>
{
    type Target = T;
    fn deref(&self) -> &T
    {
        self.get()
    }
}

impl<T> Clone for HeapRef<T>
{
    fn clone(&self) -> Self
    {
        *self
    }
}
impl<T> Copy for HeapRef<T> {}
unsafe impl<T> Send for HeapRef<T> {}
unsafe impl<T> Sync for HeapRef<T> {}

impl<T: fmt::Debug> fmt::Debug for HeapRef<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(self.get(), f)
    }
}
impl<T: fmt::Display> fmt::Display for HeapRef<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(self.get(), f)
    }
}
impl<T: PartialEq> PartialEq for HeapRef<T>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.get() == other.get()
    }
}
impl<T: Eq> Eq for HeapRef<T> {}
impl<T: PartialOrd> PartialOrd for HeapRef<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        self.get().partial_cmp(other.get())
    }
}
impl<T: Ord> Ord for HeapRef<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
        self.get().cmp(other.get())
    }
}
impl<T: std::hash::Hash> std::hash::Hash for HeapRef<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        self.get().hash(state)
    }
}
