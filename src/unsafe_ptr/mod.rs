mod raw_ptr;
pub use raw_ptr::*;

mod raw_ref;
pub use raw_ref::*;

mod raw_ref_mut;
pub use raw_ref_mut::*;

pub fn leak<T>(val: T) -> *const u8
{
    Box::leak(Box::new(val)) as *const T as *const u8
}
