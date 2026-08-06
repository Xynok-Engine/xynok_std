use core::fmt;

use crate::unsafe_ptr::heap_ref::HeapRef;

/// Non-owning mutable raw pointer. Equivalent to `&mut T` but without lifetimes.
pub struct HeapRefMut<T>
{
    pub(crate) ptr: *const T,
}

impl<T> HeapRefMut<T>
{
    pub(crate) fn get(&self) -> &T
    {
        unsafe { &*self.ptr }
    }
    /// trả về con trỏ thô, bypass borrow checker
    pub fn as_ref(&self) -> HeapRef<T>
    {
        HeapRef { ptr: self.ptr }
    }
    /// trả về con trỏ có lifetime, tuân thủ rule borrow checker của rust
    /// Đôi khi, để rõ ý đồ sử dụng của ptr này, cần khai báo tường minh lifetime khi dùng fn này
    /// nhưng Rust compiler sẽ đủ thông minh để xác định chính xác lifetime, nên về cơ bản hãy chú
    /// ý nhận thức dc ptr này là dùng tạm local fn lifetime hay là lưu trữ ở đâu đó. Và khi lưu
    /// trữ, thì bắt buộc là struct, enum đó sẽ cần có lifetime. Boom ! vậy đó là lý do rust cần
    /// khai báo tường minh lifetime cho struct khi nó chứa ptr.
    pub fn as_ref_with_caller_lifetime<'a>(&self) -> &'a T
    {
        unsafe { &*self.ptr }
    }

    pub fn as_ref_mut<'a>(&self) -> &'a mut T
    {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

impl<T> std::ops::Deref for HeapRefMut<T>
{
    type Target = T;
    fn deref(&self) -> &T
    {
        unsafe { &*self.ptr }
    }
}
impl<T> std::ops::DerefMut for HeapRefMut<T>
{
    fn deref_mut(&mut self) -> &mut T
    {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}

impl<T> Clone for HeapRefMut<T>
{
    fn clone(&self) -> Self
    {
        *self
    }
}
impl<T> Copy for HeapRefMut<T> {}
unsafe impl<T> Send for HeapRefMut<T> {}
unsafe impl<T> Sync for HeapRefMut<T> {}

impl<T: fmt::Debug> fmt::Debug for HeapRefMut<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Debug::fmt(self.get(), f)
    }
}
impl<T: fmt::Display> fmt::Display for HeapRefMut<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        fmt::Display::fmt(self.get(), f)
    }
}
impl<T: PartialEq> PartialEq for HeapRefMut<T>
{
    fn eq(&self, other: &Self) -> bool
    {
        self.get() == other.get()
    }
}
impl<T: Eq> Eq for HeapRefMut<T> {}
impl<T: PartialOrd> PartialOrd for HeapRefMut<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
        self.get().partial_cmp(other.get())
    }
}
impl<T: Ord> Ord for HeapRefMut<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
        self.get().cmp(other.get())
    }
}
impl<T: std::hash::Hash> std::hash::Hash for HeapRefMut<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H)
    {
        self.get().hash(state)
    }
}
