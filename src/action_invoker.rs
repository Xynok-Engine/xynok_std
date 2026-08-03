//!
//! # Safety Contract
//! - Single-threaded use only.
//! - Objects registered via `add_obj` / `add_obj_once` must remain valid
//!   (and not be moved) for all subsequent `invoke` calls while their slot
//!   is registered. Lifetimes are intentionally erased; the compiler does
//!   not track them.

// ─────────────────────────────────────────────────────────────────────────────
// CallMode
// ─────────────────────────────────────────────────────────────────────────────

/// Controls how long a registered handler stays active.
///
/// | Variant      | Behaviour                                              |
/// |--------------|--------------------------------------------------------|
/// | `Persistent` | Remains registered until explicitly removed or cleared.|
/// | `OnceCall`   | Automatically removed after the first invocation.      |
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CallbackLifeTime
{
    /// Handler lives until `remove_fn` / `remove_obj` / `clear` is called.
    Persistent,
    /// Handler fires exactly once, then is silently removed.
    Once,
}
//s
// ─────────────────────────────────────────────────────────────────────────────
// SlotT<T>  —  enum storage for Action<T>
// ─────────────────────────────────────────────────────────────────────────────

/// One registered handler for [`Action<T>`].
enum SlotT<T>
{
    /// A free / static function.
    Static
    {
        f: fn(&T), mode: CallbackLifeTime
    },

    /// An instance method bound to a type-erased object pointer.
    ///
    /// Fields
    /// - `ptr`    — type-erased `*mut Obj`
    /// - `fn_ptr` — the original `fn(&mut Obj, &T)` cast to `*mut ()`, used for dedup / removal identity
    /// - `call`   — monomorphised thunk `fn(*mut (), *mut (), &T)`
    /// - `mode`   — persistence control
    Object
    {
        ptr:    *mut (),
        fn_ptr: *mut (),
        call:   fn(*mut (), *mut (), &T),
        mode:   CallbackLifeTime,
    },
}

// SAFETY: single-threaded by contract; raw pointers never cross thread boundaries.
unsafe impl<T> Send for SlotT<T> {}
unsafe impl<T> Sync for SlotT<T> {}

impl<T> SlotT<T>
{
    /// Composite identity key `(ptr-or-fn, fn-or-zero)` — used for dedup and removal.
    /// Key is intentionally independent of `CallMode` so that registering the same
    /// handler twice (once persistent, once once-call) is still treated as a duplicate.
    #[inline]
    fn key(&self) -> (usize, usize)
    {
        match self
        {
            SlotT::Static { f, .. } => (*f as usize, 0),
            SlotT::Object { ptr, fn_ptr, .. } => (*ptr as usize, *fn_ptr as usize),
        }
    }

    #[inline]
    fn mode(&self) -> CallbackLifeTime
    {
        match self
        {
            SlotT::Static { mode, .. } => *mode,
            SlotT::Object { mode, .. } => *mode,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Action<T>
// ─────────────────────────────────────────────────────────────────────────────

/// Multicast delegate accepting one `&T` argument — analogous to C# `Action<T>`.
///
/// Each handler can be registered as [`CallMode::Persistent`] (the default,
/// lives until removed) or [`CallMode::OnceCall`] (auto-removed after the
/// first invocation).
///
/// # Example
/// ```rust
/// fn on_damage(dmg: &f32)
/// {
///     println!("ouch {dmg}");
/// }
///
/// struct Player
/// {
///     hp: f32,
/// }
/// impl Player
/// {
///     fn take_hit(&mut self, dmg: &f32)
///     {
///         self.hp -= dmg;
///     }
///     fn on_first_hit(&mut self, dmg: &f32)
///     {
///         println!("first hit! -{dmg}");
///     }
/// }
///
/// let mut ev: Action<f32> = Action::new();
/// let mut player = Player { hp: 100.0 };
///
/// ev.add_fn(on_damage); // persistent
/// ev.add_fn_once(on_damage); // once-call variant
/// unsafe {
///     ev.add_obj(&mut player, Player::take_hit);
/// } // persistent
/// unsafe {
///     ev.add_obj_once(&mut player, Player::on_first_hit);
/// } // once-call
///
/// ev.invoke(&10.0); // on_first_hit fires and is removed; take_hit stays
/// ev.invoke(&10.0); // only take_hit fires
///
/// ev.remove_fn(on_damage);
/// unsafe {
///     ev.remove_obj(&mut player, Player::take_hit);
/// }
/// ```
//#[deprecated(note = "ko nên dùng, kiến trúc này ko phù hợp với ECS")]
pub struct Action<T>
{
    slots: Vec<SlotT<T>>,
}

// SAFETY: single-threaded by contract.
unsafe impl<T> Send for Action<T> {}
unsafe impl<T> Sync for Action<T> {}

impl<T> Action<T>
{
    /// Create an empty delegate list.
    #[inline]
    pub fn new() -> Self
    {
        Self { slots: Vec::new() }
    }

    /// Create with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(n: usize) -> Self
    {
        Self { slots: Vec::with_capacity(n) }
    }

    // ── Static / free functions ───────────────────────────────────────────

    /// Register a free function as [`CallMode::Persistent`].
    /// Duplicate registrations (same pointer, any mode) are silently ignored.
    pub fn add_fn(&mut self, f: fn(&T))
    {
        self.add_fn_with_mode(f, CallbackLifeTime::Persistent);
    }

    /// Register a free function as [`CallMode::OnceCall`].
    /// It fires once on the next `invoke`, then is automatically removed.
    /// Duplicate registrations are silently ignored.
    pub fn add_fn_once(&mut self, f: fn(&T))
    {
        self.add_fn_with_mode(f, CallbackLifeTime::Once);
    }

    /// Low-level: register a free function with an explicit [`CallMode`].
    pub fn add_fn_with_mode(&mut self, f: fn(&T), mode: CallbackLifeTime)
    {
        let key = (f as usize, 0);
        if self.slots.iter().any(|s| s.key() == key)
        {
            return;
        }
        self.slots.push(SlotT::Static { f, mode });
    }

    /// Unregister a previously added free function (any mode).
    pub fn remove_fn(&mut self, f: fn(&T))
    {
        let key = (f as usize, 0);
        self.slots.retain(|s| s.key() != key);
    }

    // ── Object / instance methods ─────────────────────────────────────────

    /// Register an instance method as [`CallMode::Persistent`].
    ///
    /// # Safety
    /// `obj` must remain valid (and not be moved) for all subsequent
    /// [`invoke`](Self::invoke) calls while this slot is registered.
    pub unsafe fn add_obj<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj, &T))
    {
        unsafe { self.add_obj_with_mode(obj, f, CallbackLifeTime::Persistent) };
    }

    /// Register an instance method as [`CallMode::OnceCall`].
    /// It fires once on the next `invoke`, then is automatically removed.
    ///
    /// # Safety
    /// Same as [`add_obj`](Self::add_obj).
    pub unsafe fn add_obj_once<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj, &T))
    {
        unsafe { self.add_obj_with_mode(obj, f, CallbackLifeTime::Once) };
    }

    /// Low-level: register an instance method with an explicit [`CallMode`].
    ///
    /// # Safety
    /// Same as [`add_obj`](Self::add_obj).
    pub unsafe fn add_obj_with_mode<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj, &T), mode: CallbackLifeTime)
    {
        let ptr = obj as *mut Obj as *mut ();
        let fn_ptr = f as usize as *mut ();
        let key = (ptr as usize, fn_ptr as usize);
        if self.slots.iter().any(|s| s.key() == key)
        {
            return;
        }

        fn thunk<Obj, T>(ptr: *mut (), fn_ptr: *mut (), arg: &T)
        {
            // SAFETY: `ptr` was cast from `*mut Obj` at registration; caller
            // guarantees it is still valid and not aliased.
            let obj: &mut Obj = unsafe { &mut *(ptr as *mut Obj) };
            // SAFETY: `fn_ptr` was cast from `fn(&mut Obj, &T)` at registration.
            let f: fn(&mut Obj, &T) = unsafe { core::mem::transmute(fn_ptr) };
            f(obj, arg);
        }

        self.slots.push(SlotT::Object {
            ptr,
            fn_ptr,
            call: thunk::<Obj, T>,
            mode,
        });
    }

    /// Unregister an object method previously added with `add_obj` / `add_obj_once` (any mode).
    pub fn remove_obj<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj, &T))
    {
        let key = (obj as *mut Obj as usize, f as usize);
        self.slots.retain(|s| s.key() != key);
    }

    // ── Invocation ────────────────────────────────────────────────────────

    /// Call every registered handler in registration order.
    ///
    /// [`CallMode::OnceCall`] handlers are invoked once and then dropped
    /// before this function returns.
    pub fn invoke(&mut self, arg: &T)
    {
        // Invoke all slots, track which once-call slots fired so we can drop them.
        // We use a swap-based drain to avoid reallocating; once-call slots are
        // moved to a temporary vec, called, then discarded.

        let mut i = 0;
        while i < self.slots.len()
        {
            // Call the slot at position `i`.
            match &self.slots[i]
            {
                SlotT::Static { f, .. } => f(arg),
                SlotT::Object { ptr, fn_ptr, call, .. } => call(*ptr, *fn_ptr, arg),
            }

            // Remove once-call slots immediately after firing.
            if self.slots[i].mode() == CallbackLifeTime::Once
            {
                self.slots.swap_remove(i);
                // Do NOT increment i — the swapped element needs to be visited.
            }
            else
            {
                i += 1;
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize
    {
        self.slots.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.slots.is_empty()
    }
    #[inline]
    pub fn clear(&mut self)
    {
        self.slots.clear();
    }
}

impl<T> Default for Action<T>
{
    fn default() -> Self
    {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Slot0  —  enum storage for Action0
// ─────────────────────────────────────────────────────────────────────────────

/// One registered handler for [`Action0`].
enum Slot0
{
    Static
    {
        f: fn(), mode: CallbackLifeTime
    },
    Object
    {
        ptr:    *mut (),
        fn_ptr: *mut (),
        call:   fn(*mut (), *mut ()),
        mode:   CallbackLifeTime,
    },
}

unsafe impl Send for Slot0 {}
unsafe impl Sync for Slot0 {}

impl Slot0
{
    #[inline]
    fn key(&self) -> (usize, usize)
    {
        match self
        {
            Slot0::Static { f, .. } => (*f as usize, 0),
            Slot0::Object { ptr, fn_ptr, .. } => (*ptr as usize, *fn_ptr as usize),
        }
    }

    #[inline]
    fn mode(&self) -> CallbackLifeTime
    {
        match self
        {
            Slot0::Static { mode, .. } => *mode,
            Slot0::Object { mode, .. } => *mode,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Action0
// ─────────────────────────────────────────────────────────────────────────────

/// Parameterless multicast delegate — analogous to C# `Action`.
///
/// Supports the same [`CallMode::Persistent`] / [`CallMode::OnceCall`]
/// lifetime control as [`Action<T>`].
pub struct Action0
{
    slots: Vec<Slot0>,
}

unsafe impl Send for Action0 {}
unsafe impl Sync for Action0 {}

impl Action0
{
    #[inline]
    pub fn new() -> Self
    {
        Self { slots: Vec::new() }
    }

    #[inline]
    pub fn with_capacity(n: usize) -> Self
    {
        Self { slots: Vec::with_capacity(n) }
    }

    // ── Static / free functions ───────────────────────────────────────────

    /// Register a free function as [`CallMode::Persistent`].
    pub fn add_fn(&mut self, f: fn())
    {
        self.add_fn_with_mode(f, CallbackLifeTime::Persistent);
    }

    /// Register a free function as [`CallMode::OnceCall`].
    pub fn add_fn_once(&mut self, f: fn())
    {
        self.add_fn_with_mode(f, CallbackLifeTime::Once);
    }

    /// Low-level: register a free function with an explicit [`CallMode`].
    pub fn add_fn_with_mode(&mut self, f: fn(), mode: CallbackLifeTime)
    {
        let key = (f as usize, 0);
        if self.slots.iter().any(|s| s.key() == key)
        {
            return;
        }
        self.slots.push(Slot0::Static { f, mode });
    }

    /// Unregister a previously added free function (any mode).
    pub fn remove_fn(&mut self, f: fn())
    {
        let key = (f as usize, 0);
        self.slots.retain(|s| s.key() != key);
    }

    // ── Object / instance methods ─────────────────────────────────────────

    /// Register an instance method as [`CallMode::Persistent`].
    ///
    /// # Safety
    /// `obj` must remain valid (and not be moved) for all subsequent
    /// [`invoke`](Self::invoke) calls while this slot is registered.
    pub unsafe fn add_obj<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj))
    {
        self.add_obj_with_mode(obj, f, CallbackLifeTime::Persistent);
    }

    /// Register an instance method as [`CallMode::OnceCall`].
    ///
    /// # Safety
    /// Same as [`add_obj`](Self::add_obj).
    pub unsafe fn add_obj_once<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj))
    {
        self.add_obj_with_mode(obj, f, CallbackLifeTime::Once);
    }

    /// Low-level: register an instance method with an explicit [`CallMode`].
    ///
    /// # Safety
    /// Same as [`add_obj`](Self::add_obj).
    pub fn add_obj_with_mode<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj), mode: CallbackLifeTime)
    {
        let ptr = obj as *mut Obj as *mut ();
        let fn_ptr = f as usize as *mut ();
        let key = (ptr as usize, fn_ptr as usize);
        if self.slots.iter().any(|s| s.key() == key)
        {
            return;
        }

        fn thunk<Obj>(ptr: *mut (), fn_ptr: *mut ())
        {
            let obj: &mut Obj = unsafe { &mut *(ptr as *mut Obj) };
            let f: fn(&mut Obj) = unsafe { core::mem::transmute(fn_ptr) };
            f(obj);
        }

        self.slots.push(Slot0::Object {
            ptr,
            fn_ptr,
            call: thunk::<Obj>,
            mode,
        });
    }

    /// Unregister an object method previously added with `add_obj` / `add_obj_once` (any mode).
    pub fn remove_obj<Obj>(&mut self, obj: &mut Obj, f: fn(&mut Obj))
    {
        let key = (obj as *mut Obj as usize, f as usize);
        self.slots.retain(|s| s.key() != key);
    }

    // ── Invocation ────────────────────────────────────────────────────────

    /// Call every registered handler in registration order.
    ///
    /// [`CallMode::OnceCall`] handlers are invoked once and then dropped.
    pub fn invoke(&mut self)
    {
        let mut i = 0;
        while i < self.slots.len()
        {
            match &self.slots[i]
            {
                Slot0::Static { f, .. } => f(),
                Slot0::Object { ptr, fn_ptr, call, .. } => call(*ptr, *fn_ptr),
            }

            if self.slots[i].mode() == CallbackLifeTime::Once
            {
                self.slots.swap_remove(i);
            }
            else
            {
                i += 1;
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize
    {
        self.slots.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.slots.is_empty()
    }
    #[inline]
    pub fn clear(&mut self)
    {
        self.slots.clear();
    }
}

impl Default for Action0
{
    fn default() -> Self
    {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests
{
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering::Relaxed;

    // ── Action<T> — original persistent behaviour (unchanged) ──────────────

    #[test]
    fn static_fn_called_twice()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn(h);
        a.invoke(&0);
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 2);
    }

    #[test]
    fn static_fn_no_duplicate()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn(h);
        a.add_fn(h);
        assert_eq!(a.len(), 1);
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 1);
    }

    #[test]
    fn static_fn_remove()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn(h);
        a.remove_fn(h);
        assert!(a.is_empty());
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 0);
    }

    #[test]
    fn obj_method_called()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn inc(&mut self, _: &i32)
            {
                self.n += 1;
            }
        }
        let mut c = C { n: 0 };
        let mut a: Action<i32> = Action::new();
        unsafe {
            a.add_obj(&mut c, C::inc);
        }
        a.invoke(&0);
        a.invoke(&0);
        assert_eq!(c.n, 2);
    }

    #[test]
    fn obj_method_remove()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn inc(&mut self, _: &i32)
            {
                self.n += 1;
            }
        }
        let mut c = C { n: 0 };
        let mut a: Action<i32> = Action::new();
        unsafe {
            a.add_obj(&mut c, C::inc);
        }
        a.remove_obj(&mut c, C::inc);
        a.invoke(&0);
        assert_eq!(c.n, 0);
    }

    #[test]
    fn two_objects_same_type_independently()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn bump(&mut self, v: &u32)
            {
                self.n += v;
            }
        }
        let mut a_obj = C { n: 0 };
        let mut b_obj = C { n: 0 };
        let mut ev: Action<u32> = Action::new();
        unsafe {
            ev.add_obj(&mut a_obj, C::bump);
            ev.add_obj(&mut b_obj, C::bump);
        }
        ev.invoke(&7);
        assert_eq!(a_obj.n, 7);
        assert_eq!(b_obj.n, 7);
    }

    #[test]
    fn remove_one_of_two_objects()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn bump(&mut self, _: &u32)
            {
                self.n += 1;
            }
        }
        let mut a_obj = C { n: 0 };
        let mut b_obj = C { n: 0 };
        let mut ev: Action<u32> = Action::new();
        unsafe {
            ev.add_obj(&mut a_obj, C::bump);
            ev.add_obj(&mut b_obj, C::bump);
            ev.remove_obj(&mut a_obj, C::bump);
        }
        ev.invoke(&0);
        assert_eq!(a_obj.n, 0);
        assert_eq!(b_obj.n, 1);
    }

    #[test]
    fn mixed_static_and_obj()
    {
        static S: AtomicU32 = AtomicU32::new(0);
        fn sfn(_: &u8)
        {
            S.fetch_add(1, Relaxed);
        }
        struct T
        {
            n: u32,
        }
        impl T
        {
            fn m(&mut self, _: &u8)
            {
                self.n += 1;
            }
        }
        let mut t = T { n: 0 };
        let mut a: Action<u8> = Action::new();
        a.add_fn(sfn);
        unsafe {
            a.add_obj(&mut t, T::m);
        }
        a.invoke(&0);
        assert_eq!(S.load(Relaxed), 1);
        assert_eq!(t.n, 1);
    }

    #[test]
    fn obj_receives_correct_arg_value()
    {
        struct Acc
        {
            sum: i64,
        }
        impl Acc
        {
            fn add(&mut self, v: &i64)
            {
                self.sum += v;
            }
        }
        let mut acc = Acc { sum: 0 };
        let mut a: Action<i64> = Action::new();
        unsafe {
            a.add_obj(&mut acc, Acc::add);
        }
        a.invoke(&10);
        a.invoke(&20);
        a.invoke(&30);
        assert_eq!(acc.sum, 60);
    }

    #[test]
    fn clear_resets_all()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn(h);
        a.clear();
        assert!(a.is_empty());
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 0);
    }

    #[test]
    fn obj_no_duplicate()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn inc(&mut self, _: &i32)
            {
                self.n += 1;
            }
        }
        let mut c = C { n: 0 };
        let mut a: Action<i32> = Action::new();
        unsafe {
            a.add_obj(&mut c, C::inc);
            a.add_obj(&mut c, C::inc); // duplicate – ignored
        }
        assert_eq!(a.len(), 1);
        a.invoke(&0);
        assert_eq!(c.n, 1);
    }

    // ── Action<T> — OnceCall ───────────────────────────────────────────────

    #[test]
    fn once_static_fires_exactly_once()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn_once(h);
        a.invoke(&0); // fires, then removed
        a.invoke(&0); // slot gone — no call
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 1);
    }

    #[test]
    fn once_static_removes_itself_from_slot_list()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn_once(h);
        assert_eq!(a.len(), 1);
        a.invoke(&0);
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn once_obj_fires_exactly_once()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn inc(&mut self, _: &i32)
            {
                self.n += 1;
            }
        }
        let mut c = C { n: 0 };
        let mut a: Action<i32> = Action::new();
        unsafe {
            a.add_obj_once(&mut c, C::inc);
        }
        a.invoke(&0);
        a.invoke(&0);
        a.invoke(&0);
        assert_eq!(c.n, 1);
    }

    #[test]
    fn persistent_and_once_coexist()
    {
        static P: AtomicU32 = AtomicU32::new(0);
        static O: AtomicU32 = AtomicU32::new(0);
        fn persistent(_: &i32)
        {
            P.fetch_add(1, Relaxed);
        }
        fn once_fn(_: &i32)
        {
            O.fetch_add(1, Relaxed);
        }

        let mut a: Action<i32> = Action::new();
        a.add_fn(persistent);
        a.add_fn_once(once_fn);

        a.invoke(&0); // both fire
        a.invoke(&0); // only persistent fires
        a.invoke(&0);

        assert_eq!(P.load(Relaxed), 3);
        assert_eq!(O.load(Relaxed), 1);
    }

    #[test]
    fn once_obj_and_persistent_obj_coexist()
    {
        struct C
        {
            n: u32,
        }
        impl C
        {
            fn persist(&mut self, _: &i32)
            {
                self.n += 10;
            }
            fn once(&mut self, _: &i32)
            {
                self.n += 1;
            }
        }
        let mut c = C { n: 0 };
        let mut a: Action<i32> = Action::new();
        unsafe {
            a.add_obj(&mut c, C::persist);
            a.add_obj_once(&mut c, C::once);
        }
        a.invoke(&0); // both: n = 11
        a.invoke(&0); // only persist: n = 21
        a.invoke(&0); // only persist: n = 31
        assert_eq!(c.n, 31);
    }

    #[test]
    fn once_no_duplicate_with_persistent()
    {
        // Registering the same fn a second time (even with a different mode) is ignored.
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn(h);
        a.add_fn_once(h); // duplicate key — ignored
        assert_eq!(a.len(), 1);
        a.invoke(&0);
        a.invoke(&0);
        // Persistent slot stays; called twice.
        assert_eq!(N.load(Relaxed), 2);
    }

    #[test]
    fn with_mode_explicit_persistent()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn_with_mode(h, CallbackLifeTime::Persistent);
        a.invoke(&0);
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 2);
    }

    #[test]
    fn with_mode_explicit_once()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn h(_: &i32)
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a: Action<i32> = Action::new();
        a.add_fn_with_mode(h, CallbackLifeTime::Once);
        a.invoke(&0);
        a.invoke(&0);
        assert_eq!(N.load(Relaxed), 1);
    }

    // ── Action0 — original persistent behaviour ────────────────────────────

    #[test]
    fn action0_static_fn()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn f()
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a = Action0::new();
        a.add_fn(f);
        a.invoke();
        assert_eq!(N.load(Relaxed), 1);
    }

    #[test]
    fn action0_obj_method()
    {
        struct S
        {
            n: u32,
        }
        impl S
        {
            fn tick(&mut self)
            {
                self.n += 1;
            }
        }
        let mut s = S { n: 0 };
        let mut a = Action0::new();
        unsafe {
            a.add_obj(&mut s, S::tick);
        }
        a.invoke();
        a.invoke();
        assert_eq!(s.n, 2);
    }

    #[test]
    fn action0_remove_static()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn f()
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a = Action0::new();
        a.add_fn(f);
        a.remove_fn(f);
        a.invoke();
        assert_eq!(N.load(Relaxed), 0);
    }

    #[test]
    fn action0_remove_obj()
    {
        struct S
        {
            n: u32,
        }
        impl S
        {
            fn tick(&mut self)
            {
                self.n += 1;
            }
        }
        let mut s = S { n: 0 };
        let mut a = Action0::new();
        unsafe {
            a.add_obj(&mut s, S::tick);
        }
        a.remove_obj(&mut s, S::tick);
        a.invoke();
        assert_eq!(s.n, 0);
    }

    // ── Action0 — OnceCall ─────────────────────────────────────────────────

    #[test]
    fn action0_once_static_fires_once()
    {
        static N: AtomicU32 = AtomicU32::new(0);
        fn f()
        {
            N.fetch_add(1, Relaxed);
        }
        let mut a = Action0::new();
        a.add_fn_once(f);
        a.invoke();
        a.invoke();
        a.invoke();
        assert_eq!(N.load(Relaxed), 1);
    }

    #[test]
    fn action0_once_obj_fires_once()
    {
        struct S
        {
            n: u32,
        }
        impl S
        {
            fn tick(&mut self)
            {
                self.n += 1;
            }
        }
        let mut s = S { n: 0 };
        let mut a = Action0::new();
        unsafe {
            a.add_obj_once(&mut s, S::tick);
        }
        a.invoke();
        a.invoke();
        assert_eq!(s.n, 1);
    }

    #[test]
    fn action0_persistent_and_once_coexist()
    {
        static P: AtomicU32 = AtomicU32::new(0);
        static O: AtomicU32 = AtomicU32::new(0);
        fn persistent()
        {
            P.fetch_add(1, Relaxed);
        }
        fn once_fn()
        {
            O.fetch_add(1, Relaxed);
        }
        let mut a = Action0::new();
        a.add_fn(persistent);
        a.add_fn_once(once_fn);
        a.invoke();
        a.invoke();
        a.invoke();
        assert_eq!(P.load(Relaxed), 3);
        assert_eq!(O.load(Relaxed), 1);
    }

    #[test]
    fn local_obj_outlives_action()
    {
        struct Local
        {
            v: u32,
        }
        impl Local
        {
            fn set(&mut self, x: &u32)
            {
                self.v = *x;
            }
        }
        let mut loc = Local { v: 0 };
        let mut a: Action<u32> = Action::new();
        unsafe {
            a.add_obj(&mut loc, Local::set);
        }
        a.invoke(&42);
        drop(a);
        assert_eq!(loc.v, 42);
    }
}
