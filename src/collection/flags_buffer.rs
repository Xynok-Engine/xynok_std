use std::marker::PhantomData;

pub trait Flags: Copy
{
    // Number of meaningful bits per element. Must be <= 56 to keep
    // cross-byte read/write within a single u64 staging value.
    const BITS: u32;

    fn to_bits(self) -> u64;
    fn from_bits(bits: u64) -> Self;
}

#[derive(Debug)]
pub struct FlagsBuffer<F: Flags>
{
    data: Vec<u8>,
    len:  usize,
    _p:   PhantomData<F>,
}

impl<F: Flags> FlagsBuffer<F>
{
    #[inline]
    const fn mask() -> u64
    {
        // safe because BITS <= 56 < 64
        (1u64 << F::BITS) - 1
    }

    pub fn new(len: usize, initial: F) -> Self
    {
        debug_assert!(F::BITS > 0 && F::BITS <= 56, "Flags::BITS must be in 1..=56");

        let total_bits = len * F::BITS as usize;
        // +7 bytes padding so an unaligned u64 read/write starting at the
        // last element's byte index never goes out of bounds.
        let bytes = total_bits.div_ceil(8) + 7;
        let mut buf = Self { data: vec![0u8; bytes], len, _p: PhantomData };
        let bits = initial.to_bits();
        if bits != 0
        {
            for i in 0..len
            {
                buf.set_bits(i, bits);
            }
        }
        buf
    }

    #[inline]
    pub fn len(&self) -> usize
    {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.len == 0
    }

    #[inline]
    pub fn get(&self, i: usize) -> F
    {
        F::from_bits(self.get_bits(i))
    }

    /// Override slot `i` with `flag` — sets every bit in the slot to match `flag`.
    #[inline]
    pub fn set(&mut self, i: usize, flag: F)
    {
        self.set_bits(i, flag.to_bits());
    }

    /// OR `flag` into slot `i`. Other bits in the slot are preserved.
    #[inline]
    pub fn add(&mut self, i: usize, flag: F)
    {
        debug_assert!(i < self.len);
        let bit_pos = i * F::BITS as usize;
        let byte_idx = bit_pos / 8;
        let bit_offset = (bit_pos % 8) as u32;
        let ptr = unsafe { self.data.as_mut_ptr().add(byte_idx) as *mut [u8; 8] };
        let raw = u64::from_le_bytes(unsafe { std::ptr::read_unaligned(ptr) });
        let new_raw = raw | ((flag.to_bits() & Self::mask()) << bit_offset);
        unsafe { std::ptr::write_unaligned(ptr, new_raw.to_le_bytes()) };
    }

    /// AND-NOT `flag` from slot `i`. Bits not present in `flag` are preserved.
    #[inline]
    pub fn remove(&mut self, i: usize, flag: F)
    {
        debug_assert!(i < self.len);
        let bit_pos = i * F::BITS as usize;
        let byte_idx = bit_pos / 8;
        let bit_offset = (bit_pos % 8) as u32;
        let ptr = unsafe { self.data.as_mut_ptr().add(byte_idx) as *mut [u8; 8] };
        let raw = u64::from_le_bytes(unsafe { std::ptr::read_unaligned(ptr) });
        let new_raw = raw & !((flag.to_bits() & Self::mask()) << bit_offset);
        unsafe { std::ptr::write_unaligned(ptr, new_raw.to_le_bytes()) };
    }

    /// True if every bit in `flag` is set at slot `i`.
    #[inline]
    pub fn contains(&self, i: usize, flag: F) -> bool
    {
        let want = flag.to_bits() & Self::mask();
        (self.get_bits(i) & want) == want
    }

    #[inline]
    fn set_bits(&mut self, i: usize, val: u64)
    {
        debug_assert!(i < self.len);
        let bit_pos = i * F::BITS as usize;
        let byte_idx = bit_pos / 8;
        let bit_offset = (bit_pos % 8) as u32;
        // Safe: data is padded by +7 bytes, so an 8-byte access at
        // byte_idx <= total_bytes - 8 always stays in bounds.
        let ptr = unsafe { self.data.as_mut_ptr().add(byte_idx) as *mut [u8; 8] };
        let raw = u64::from_le_bytes(unsafe { std::ptr::read_unaligned(ptr) });
        let window = Self::mask() << bit_offset;
        let new_raw = (raw & !window) | ((val & Self::mask()) << bit_offset);
        unsafe { std::ptr::write_unaligned(ptr, new_raw.to_le_bytes()) };
    }

    #[inline]
    fn get_bits(&self, i: usize) -> u64
    {
        debug_assert!(i < self.len);
        let bit_pos = i * F::BITS as usize;
        let byte_idx = bit_pos / 8;
        let bit_offset = (bit_pos % 8) as u32;
        let bytes: [u8; 8] = unsafe { std::ptr::read_unaligned(self.data.as_ptr().add(byte_idx) as *const [u8; 8]) };
        (u64::from_le_bytes(bytes) >> bit_offset) & Self::mask()
    }
}
#[allow(unused)]
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::bitflags;

    bitflags! {
        pub struct Perms: u8 {
            const READ    = 1 << 0;
            const WRITE   = 1 << 1;
            const EXECUTE = 1 << 2;
        }
    }

    // A wider flags type to exercise cross-byte cases more.
    bitflags! {
        pub struct Wide: u16 {
            const A = 1 << 0;
            const B = 1 << 4;
            const C = 1 << 8;
            const D = 1 << 11;
        }
    }

    #[test]
    fn auto_impl_bits_uses_highest_used_bit()
    {
        // Perms highest bit = EXECUTE (1 << 2) → BITS = 3
        assert_eq!(<Perms as Flags>::BITS, 3);
        // Wide highest bit = D (1 << 11) → BITS = 12
        assert_eq!(<Wide as Flags>::BITS, 12);
    }

    #[test]
    fn new_zero_initial_is_empty_everywhere()
    {
        let buf = FlagsBuffer::<Perms>::new(10, Perms::empty());
        for i in 0..10
        {
            assert!(buf.get(i).is_empty(), "slot {i} should be empty");
        }
    }

    #[test]
    fn new_non_zero_initial_fills_all()
    {
        let init = Perms::READ | Perms::WRITE;
        let buf = FlagsBuffer::<Perms>::new(10, init);
        for i in 0..10
        {
            assert_eq!(buf.get(i), init, "slot {i} should be RW");
        }
    }

    #[test]
    fn add_empty_slot()
    {
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(3, Perms::READ | Perms::EXECUTE);
        buf.add(3, Perms::empty());
        assert_eq!(buf.get(3), Perms::READ | Perms::EXECUTE);
    }

    #[test]
    fn set_and_get_single_slot()
    {
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(3, Perms::READ | Perms::EXECUTE);
        assert_eq!(buf.get(3), Perms::READ | Perms::EXECUTE);
    }

    #[test]
    fn set_does_not_affect_neighbours()
    {
        let mut buf = FlagsBuffer::<Perms>::new(16, Perms::empty());
        buf.set(5, Perms::all());
        for i in 0..16
        {
            if i == 5
            {
                assert_eq!(buf.get(i), Perms::all());
            }
            else
            {
                assert!(buf.get(i).is_empty(), "slot {i} should still be empty");
            }
        }
    }

    #[test]
    fn overwrite_clears_old_bits()
    {
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(3, Perms::all());
        buf.set(3, Perms::READ);
        assert_eq!(buf.get(3), Perms::READ);
    }

    #[test]
    fn cross_byte_boundary_3bits()
    {
        // 3 bits/element → slot 2 occupies bits 6..9 (crosses byte 0/1)
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(2, Perms::all());
        assert_eq!(buf.get(2), Perms::all());
        for i in 0..8
        {
            if i != 2
            {
                assert!(buf.get(i).is_empty(), "slot {i} leaked");
            }
        }
    }

    #[test]
    fn wide_12bits_cross_byte()
    {
        // 12 bits/element always crosses bytes for at least some slots.
        let mut buf = FlagsBuffer::<Wide>::new(8, Wide::empty());
        let v = Wide::A | Wide::B | Wide::C | Wide::D;
        for i in 0..8
        {
            buf.set(i, v);
        }
        for i in 0..8
        {
            assert_eq!(buf.get(i), v, "slot {i} mismatch");
        }
    }

    #[test]
    fn wide_alternating_pattern_isolates_slots()
    {
        let mut buf = FlagsBuffer::<Wide>::new(8, Wide::empty());
        for i in 0..8
        {
            if i % 2 == 0
            {
                buf.set(i, Wide::A | Wide::C);
            }
            else
            {
                buf.set(i, Wide::B | Wide::D);
            }
        }
        for i in 0..8
        {
            let expected = if i % 2 == 0 { Wide::A | Wide::C } else { Wide::B | Wide::D };
            assert_eq!(buf.get(i), expected, "slot {i}");
        }
    }

    #[test]
    fn non_multiple_of_8_total_bits()
    {
        // 5 slots * 3 bits = 15 bits → 2 bytes allocated, slot 4 ends at bit 14.
        let mut buf = FlagsBuffer::<Perms>::new(5, Perms::empty());
        buf.set(4, Perms::all());
        assert_eq!(buf.get(4), Perms::all());
        for i in 0..4
        {
            assert!(buf.get(i).is_empty());
        }
    }

    #[test]
    #[should_panic]
    fn get_out_of_bounds_panics()
    {
        let buf = FlagsBuffer::<Perms>::new(4, Perms::empty());
        buf.get(4);
    }

    #[test]
    #[should_panic]
    fn set_out_of_bounds_panics()
    {
        let mut buf = FlagsBuffer::<Perms>::new(4, Perms::empty());
        buf.set(4, Perms::READ);
    }

    #[test]
    fn add_flag_preserves_other_bits()
    {
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(3, Perms::READ);
        buf.add(3, Perms::EXECUTE);
        assert_eq!(buf.get(3), Perms::READ | Perms::EXECUTE);
    }

    #[test]
    fn remove_flag_clears_only_targeted_bits()
    {
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(3, Perms::all());
        buf.remove(3, Perms::WRITE);
        assert_eq!(buf.get(3), Perms::READ | Perms::EXECUTE);
    }

    #[test]
    fn set_flag_overrides_whole_slot()
    {
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(3, Perms::all());
        buf.set(3, Perms::WRITE);
        assert_eq!(buf.get(3), Perms::WRITE);
        buf.set(3, Perms::empty());
        assert!(buf.get(3).is_empty());
    }

    #[test]
    fn add_flag_cross_byte_does_not_leak()
    {
        // slot 2 with 3 bits/element occupies bits 6..9 → crosses byte boundary.
        let mut buf = FlagsBuffer::<Perms>::new(8, Perms::empty());
        buf.set(1, Perms::all());
        buf.set(3, Perms::all());
        buf.add(2, Perms::WRITE);
        assert_eq!(buf.get(2), Perms::WRITE);
        assert_eq!(buf.get(1), Perms::all(), "neighbour 1 corrupted");
        assert_eq!(buf.get(3), Perms::all(), "neighbour 3 corrupted");
    }

    #[test]
    fn contains_checks_all_requested_bits()
    {
        let mut buf = FlagsBuffer::<Perms>::new(4, Perms::empty());
        buf.set(0, Perms::READ | Perms::WRITE);
        assert!(buf.contains(0, Perms::READ));
        assert!(buf.contains(0, Perms::WRITE));
        assert!(buf.contains(0, Perms::READ | Perms::WRITE));
        assert!(!buf.contains(0, Perms::EXECUTE));
        assert!(!buf.contains(0, Perms::READ | Perms::EXECUTE));
    }

    #[test]
    fn len_and_is_empty()
    {
        let a = FlagsBuffer::<Perms>::new(0, Perms::empty());
        assert_eq!(a.len(), 0);
        assert!(a.is_empty());

        let b = FlagsBuffer::<Perms>::new(7, Perms::empty());
        assert_eq!(b.len(), 7);
        assert!(!b.is_empty());
    }
}
