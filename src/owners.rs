use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex, OnceLock, RwLock},
};

/// ── Arc<RwLock<T>> ────────────────────────────────────────────────────────────
/// Multi-threaded, shared ownership, concurrent reads, exclusive writes.
/// Use: AppData, shared engine state, resources accessed from multiple threads.
pub struct ArcRwLock<T>
{
    val: Arc<RwLock<T>>,
}
impl<T> ArcRwLock<T>
{
    pub fn new(val: T) -> Self { Self { val: Arc::new(RwLock::new(val)) } }
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, T> { self.val.read().unwrap() }
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, T> { self.val.write().unwrap() }
}
impl<T> Clone for ArcRwLock<T>
{
    fn clone(&self) -> Self { Self { val: Arc::clone(&self.val) } }
}

/// ── Arc<Mutex<T>> ─────────────────────────────────────────────────────────────
/// Multi-threaded, shared ownership, exclusive access always.
/// Use: write-heavy shared state, job queues, command buffers.
pub struct ArcMutex<T>
{
    val: Arc<Mutex<T>>,
}
impl<T> ArcMutex<T>
{
    pub fn new(val: T) -> Self { Self { val: Arc::new(Mutex::new(val)) } }
    pub fn lock(&self) -> std::sync::MutexGuard<'_, T> { self.val.lock().unwrap() }
}
impl<T> Clone for ArcMutex<T>
{
    fn clone(&self) -> Self { Self { val: Arc::clone(&self.val) } }
}

/// ── Arc<OnceLock<T>> ──────────────────────────────────────────────────────────
/// Multi-threaded, shared ownership, write once then read forever.
/// Use: global config, device handles, engine constants set at startup.
pub struct ArcOnceLock<T>
{
    val: Arc<OnceLock<T>>,
}
impl<T> Default for ArcOnceLock<T>
{
    fn default() -> Self { Self::new() }
}
impl<T> ArcOnceLock<T>
{
    pub fn new() -> Self { Self { val: Arc::new(OnceLock::new()) } }
    pub fn set(&self, val: T) { self.val.set(val).ok(); }
    pub fn get(&self) -> Option<&T> { self.val.get() }
}
impl<T> Clone for ArcOnceLock<T>
{
    fn clone(&self) -> Self { Self { val: Arc::clone(&self.val) } }
}

/// ── Rc<RefCell<T>> ────────────────────────────────────────────────────────────
/// Single-threaded, shared ownership, runtime borrow check, any type.
/// Use: UI trees, scene graphs, parent/child relationships on main thread.
#[derive(Default, Debug)]
pub struct RcRefCell<T>
{
    val: Rc<RefCell<T>>,
}
impl<T> RcRefCell<T>
{
    pub fn new(val: T) -> Self { Self { val: Rc::new(RefCell::new(val)) } }
    pub fn read(&self) -> std::cell::Ref<'_, T> { self.val.borrow() }
    pub fn write(&self) -> std::cell::RefMut<'_, T> { self.val.borrow_mut() }
}
impl<T> Clone for RcRefCell<T>
{
    fn clone(&self) -> Self { Self { val: Rc::clone(&self.val) } }
}

/// ── Rc<Cell<T>> ───────────────────────────────────────────────────────────────
/// Single-threaded, shared ownership, Copy types only, zero overhead.
/// Use: shared counters, flags, simple scalars on main thread.
pub struct RcCell<T: Copy>
{
    val: Rc<Cell<T>>,
}
impl<T: Copy> RcCell<T>
{
    pub fn new(val: T) -> Self { Self { val: Rc::new(Cell::new(val)) } }
    pub fn get(&self) -> T { self.val.get() }
    pub fn set(&self, val: T) { self.val.set(val) }
}
impl<T: Copy> Clone for RcCell<T>
{
    fn clone(&self) -> Self { Self { val: Rc::clone(&self.val) } }
}

/// ── Rc<OnceCell<T>> ───────────────────────────────────────────────────────────
/// Single-threaded, shared ownership, write once then read forever.
/// Use: lazy-initialized single-thread resources, cached computed values.
pub struct RcOnceCell<T>
{
    val: Rc<OnceCell<T>>,
}
impl<T> Default for RcOnceCell<T>
{
    fn default() -> Self { Self::new() }
}
impl<T> RcOnceCell<T>
{
    pub fn new() -> Self { Self { val: Rc::new(OnceCell::new()) } }
    pub fn set(&self, val: T) { self.val.set(val).ok(); }
    pub fn get(&self) -> Option<&T> { self.val.get() }
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T { self.val.get_or_init(f) }
}
impl<T> Clone for RcOnceCell<T>
{
    fn clone(&self) -> Self { Self { val: Rc::clone(&self.val) } }
}
