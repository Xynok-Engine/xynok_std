use core::fmt;

/// A non-owning immutable raw pointer. It’s equivalent to `&T` but without lifetimes.
pub struct RawRef<T>
{
    pub(crate) ptr: *const T,
}

impl<T> RawRef<T>
{
    pub fn get(&self) -> &T { unsafe { &*self.ptr } }
}

impl<T> std::ops::Deref for RawRef<T>
{
    type Target = T;
    fn deref(&self) -> &T { self.get() }
}

impl<T> Clone for RawRef<T>
{
    fn clone(&self) -> Self { *self }
}
impl<T> Copy for RawRef<T> {}
unsafe impl<T> Send for RawRef<T> {}
unsafe impl<T> Sync for RawRef<T> {}

impl<T: fmt::Debug> fmt::Debug for RawRef<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Debug::fmt(self.get(), f) }
}
impl<T: fmt::Display> fmt::Display for RawRef<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self.get(), f) }
}
impl<T: PartialEq> PartialEq for RawRef<T>
{
    fn eq(&self, other: &Self) -> bool { self.get() == other.get() }
}
impl<T: Eq> Eq for RawRef<T> {}
impl<T: PartialOrd> PartialOrd for RawRef<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { self.get().partial_cmp(other.get()) }
}
impl<T: Ord> Ord for RawRef<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.get().cmp(other.get()) }
}
impl<T: std::hash::Hash> std::hash::Hash for RawRef<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.get().hash(state) }
}
