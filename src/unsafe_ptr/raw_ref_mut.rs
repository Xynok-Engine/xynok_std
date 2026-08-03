use core::fmt;

use crate::unsafe_ptr::raw_ref::RawRef;

/// Non-owning mutable raw pointer. Equivalent to `&mut T` but without lifetimes.
pub struct RawRefMut<T>
{
    pub(crate) ptr: *const T,
}

impl<T> RawRefMut<T>
{
    pub(crate) fn get(&self) -> &T { unsafe { &*self.ptr } }
    /// trả về con trỏ thô, bypass borrow checker
    pub fn as_ref(&self) -> RawRef<T> { RawRef { ptr: self.ptr } }
    /// trả về con trỏ có lifetime, tuân thủ rule borrow checker của rust
    /// Đôi khi, để rõ ý đồ sử dụng của ptr này, cần khai báo tường minh lifetime khi dùng fn này
    /// nhưng Rust compiler sẽ đủ thông minh để xác định chính xác lifetime, nên về cơ bản hãy chú
    /// ý nhận thức dc ptr này là dùng tạm local fn lifetime hay là lưu trữ ở đâu đó. Và khi lưu
    /// trữ, thì bắt buộc là struct, enum đó sẽ cần có lifetime. Boom ! vậy đó là lý do rust cần
    /// khai báo tường minh lifetime cho struct khi nó chứa ptr.
    pub fn as_ref_with_caller_lifetime<'a>(&self) -> &'a T { unsafe { &*self.ptr } }

    pub fn as_ref_mut<'a>(&self) -> &'a mut T { unsafe { &mut *(self.ptr as *mut T) } }
}

impl<T> std::ops::Deref for RawRefMut<T>
{
    type Target = T;
    fn deref(&self) -> &T { unsafe { &*self.ptr } }
}
impl<T> std::ops::DerefMut for RawRefMut<T>
{
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *(self.ptr as *mut T) } }
}

impl<T> Clone for RawRefMut<T>
{
    fn clone(&self) -> Self { *self }
}
impl<T> Copy for RawRefMut<T> {}
unsafe impl<T> Send for RawRefMut<T> {}
unsafe impl<T> Sync for RawRefMut<T> {}

impl<T: fmt::Debug> fmt::Debug for RawRefMut<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Debug::fmt(self.get(), f) }
}
impl<T: fmt::Display> fmt::Display for RawRefMut<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt::Display::fmt(self.get(), f) }
}
impl<T: PartialEq> PartialEq for RawRefMut<T>
{
    fn eq(&self, other: &Self) -> bool { self.get() == other.get() }
}
impl<T: Eq> Eq for RawRefMut<T> {}
impl<T: PartialOrd> PartialOrd for RawRefMut<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { self.get().partial_cmp(other.get()) }
}
impl<T: Ord> Ord for RawRefMut<T>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.get().cmp(other.get()) }
}
impl<T: std::hash::Hash> std::hash::Hash for RawRefMut<T>
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.get().hash(state) }
}
