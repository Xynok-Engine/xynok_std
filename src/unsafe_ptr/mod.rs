mod heap_ptr;
pub use heap_ptr::*;

mod heap_ref;
pub use heap_ref::*;

mod heap_ref_mut;
pub use heap_ref_mut::*;

pub fn leak<T>(val: T) -> *const u8
{
    Box::leak(Box::new(val)) as *const T as *const u8
}
