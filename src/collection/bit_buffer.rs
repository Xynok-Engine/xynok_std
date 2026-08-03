#[derive(Debug)]
pub struct BitBuffer
{
    data:    Vec<u8>,
    bit_len: usize,
}

impl BitBuffer
{
    pub fn new(bit_len: usize, initial_val: bool) -> Self
    {
        let default_val = match initial_val
        {
            true => 0xFFu8,
            false => 0u8,
        };
        Self {
            data: vec![default_val; bit_len.div_ceil(8)],
            bit_len,
        }
    }

    #[inline]
    pub fn set(&mut self, i: usize, val: bool)
    {
        debug_assert!(i < self.bit_len);
        let byte = i / 8;
        let bit = i % 8;
        if val
        {
            self.data[byte] |= 1 << bit;
        }
        else
        {
            self.data[byte] &= !(1 << bit);
        }
    }

    #[inline]
    pub fn get(&self, i: usize) -> bool
    {
        debug_assert!(i < self.bit_len);
        let val = unsafe { self.data.get_unchecked(i / 8) };
        (val >> (i % 8)) & 1 == 1
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn new_false_all_zero()
    {
        let buf = BitBuffer::new(16, false);
        for i in 0..16
        {
            assert!(!buf.get(i), "bit {i} should be 0");
        }
    }

    #[test]
    fn new_true_all_one()
    {
        let buf = BitBuffer::new(16, true);
        for i in 0..16
        {
            assert!(buf.get(i), "bit {i} should be 1");
        }
    }

    #[test]
    fn set_and_get_true()
    {
        let mut buf = BitBuffer::new(8, false);
        buf.set(3, true);
        assert!(buf.get(3));
    }

    #[test]
    fn set_and_get_false()
    {
        let mut buf = BitBuffer::new(8, false);
        buf.set(3, true);
        buf.set(3, false);
        assert!(!buf.get(3));
    }

    #[test]
    fn set_does_not_affect_neighbours()
    {
        let mut buf = BitBuffer::new(16, false);
        buf.set(5, true);
        for i in 0..16
        {
            if i == 5
            {
                assert!(buf.get(i));
            }
            else
            {
                assert!(!buf.get(i), "bit {i} should still be 0");
            }
        }
    }

    #[test]
    fn set_true_does_not_affect_neighbours()
    {
        let mut buf = BitBuffer::new(16, true);
        buf.set(5, false);
        for i in 0..16
        {
            if i == 5
            {
                assert!(!buf.get(i));
            }
            else
            {
                assert!(buf.get(i), "bit {i} should still be 1");
            }
        }
    }

    #[test]
    fn cross_byte_boundary()
    {
        let mut buf = BitBuffer::new(16, false);
        buf.set(7, true);
        buf.set(8, true);
        assert!(buf.get(7));
        assert!(buf.get(8));
        assert!(!buf.get(6));
        assert!(!buf.get(9));
    }

    #[test]
    fn non_multiple_of_8_length()
    {
        // 9 bits → 2 bytes allocated; bit 8 is the last valid bit
        let mut buf = BitBuffer::new(9, false);
        buf.set(8, true);
        assert!(buf.get(8));
        for i in 0..8
        {
            assert!(!buf.get(i));
        }
    }

    #[test]
    #[should_panic]
    fn get_out_of_bounds_panics()
    {
        let buf = BitBuffer::new(8, false);
        buf.get(8);
    }

    #[test]
    #[should_panic]
    fn set_out_of_bounds_panics()
    {
        let mut buf = BitBuffer::new(8, false);
        buf.set(8, true);
    }

    #[test]
    fn all_bits_set_then_cleared()
    {
        let mut buf = BitBuffer::new(24, false);
        for i in 0..24
        {
            buf.set(i, true);
        }
        for i in 0..24
        {
            assert!(buf.get(i));
        }
        for i in 0..24
        {
            buf.set(i, false);
        }
        for i in 0..24
        {
            assert!(!buf.get(i));
        }
    }
}
