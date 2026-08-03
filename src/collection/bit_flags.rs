#[macro_export]
macro_rules! bitflags {
    // entry point
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident : $ty:ty {
            $(
                $(#[$flag_meta:meta])*
               $flag_vis:vis const $flag:ident = $val:expr;
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $vis struct $name($ty);

        impl $name {
            $(
                $(#[$flag_meta])*
                $flag_vis const $flag: Self = Self($val);
            )*

            pub const FLAG_COUNT: usize = [$( $val ),*].len();

            pub const fn empty() -> Self { Self(0) }
            pub const fn all() -> Self {
                Self($( $val )|*)  // OR all values together
            }
            pub const fn bits(self) -> $ty { self.0 }
            pub const fn from_bits(bits: $ty) -> Self { Self(bits) }

            pub const fn contains(self, other: Self) -> bool {
                (self.0 & other.0) == other.0
            }
            pub const fn intersects(self, other: Self) -> bool {
                (self.0 & other.0) != 0
            }
            pub fn insert(&mut self, other: Self) { self.0 |= other.0; }
            pub fn remove(&mut self, other: Self) { self.0 &= !other.0; }
            pub fn toggle(&mut self, other: Self) { self.0 ^= other.0; }
            pub const fn is_empty(self) -> bool   { self.0 == 0 }
        }

        // operators
        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
        }
        impl std::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self { Self(self.0 & rhs.0) }
        }
        impl std::ops::BitXor for $name {
            type Output = Self;
            fn bitxor(self, rhs: Self) -> Self { Self(self.0 ^ rhs.0) }
        }
        impl std::ops::Not for $name {
            type Output = Self;
            fn not(self) -> Self { Self(!self.0) }
        }
        impl std::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
        }
        impl std::ops::BitAndAssign for $name {
            fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
        }

        // Auto-impl Flags so the type can be stored in FlagsBuffer.
        // BITS = position of the highest used flag bit + 1, computed
        // from the OR of every declared value (so unused high bits in
        // the backing integer don't waste space).
        impl $crate::Flags for $name {
            const BITS: u32 = {
                let mask: u64 = ($( $val )|*) as u64;
                if mask == 0 { 0 } else { 64 - mask.leading_zeros() }
            };
            fn to_bits(self) -> u64 { self.0 as u64 }
            fn from_bits(bits: u64) -> Self { Self(bits as $ty) }
        }

        // debug — prints flag names instead of raw number
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut first = true;
                let mut bits = self.0;
                $(
                    if (bits & $val) == $val && $val != 0 {
                        if !first { write!(f, " | ")?; }
                        write!(f, stringify!($flag))?;
                        bits &= !$val;
                        first = false;
                    }
                )*
                if bits != 0 || first {
                    if !first { write!(f, " | ")?; }
                    write!(f, "0x{:x}", bits)?;
                }
                Ok(())
            }
        }
    };
}

#[cfg(test)]
mod tests
{

    bitflags! {
        pub struct Perms: u8 {
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const EXECUTE = 1 << 2;
        }
    }

    #[test]
    fn empty_has_no_bits()
    {
        assert_eq!(Perms::empty().bits(), 0);
        assert!(Perms::empty().is_empty());
    }

    #[test]
    fn all_ors_every_flag()
    {
        assert_eq!(Perms::all().bits(), 0b0111);
        assert!(!Perms::all().is_empty());
    }

    #[test]
    fn from_bits_roundtrips()
    {
        let p = Perms::from_bits(0b0101);
        assert_eq!(p.bits(), 0b0101);
    }

    #[test]
    fn contains_exact_and_subset()
    {
        let rw = Perms::READ | Perms::WRITE;
        assert!(rw.contains(Perms::READ));
        assert!(rw.contains(Perms::WRITE));
        assert!(!rw.contains(Perms::EXECUTE));
        assert!(rw.contains(rw));
    }

    #[test]
    fn intersects_any_overlap()
    {
        let rw = Perms::READ | Perms::WRITE;
        let rx = Perms::READ | Perms::EXECUTE;
        assert!(rw.intersects(rx));
        assert!(!Perms::WRITE.intersects(Perms::EXECUTE));
    }

    #[test]
    fn insert_sets_bits()
    {
        let mut p = Perms::READ;
        p.insert(Perms::WRITE);
        assert!(p.contains(Perms::READ));
        assert!(p.contains(Perms::WRITE));
    }

    #[test]
    fn remove_clears_bits()
    {
        let mut p = Perms::READ | Perms::WRITE;
        p.remove(Perms::READ);
        assert!(!p.contains(Perms::READ));
        assert!(p.contains(Perms::WRITE));
    }

    #[test]
    fn toggle_flips_bits()
    {
        let mut p = Perms::READ;
        p.toggle(Perms::READ);
        assert!(p.is_empty());
        p.toggle(Perms::WRITE);
        assert!(p.contains(Perms::WRITE));
    }

    #[test]
    fn bitand_operator()
    {
        let result = (Perms::READ | Perms::WRITE) & Perms::READ;
        assert_eq!(result, Perms::READ);
    }

    #[test]
    fn bitxor_operator()
    {
        let result = (Perms::READ | Perms::WRITE) ^ Perms::READ;
        assert_eq!(result, Perms::WRITE);
    }

    #[test]
    fn not_operator_inverts()
    {
        let inverted = !Perms::empty();
        assert_eq!(inverted.bits(), !0u8);
    }

    #[test]
    fn bitor_assign_operator()
    {
        let mut p = Perms::READ;
        p |= Perms::WRITE;
        assert!(p.contains(Perms::READ | Perms::WRITE));
    }

    #[test]
    fn bitand_assign_operator()
    {
        let mut p = Perms::READ | Perms::WRITE;
        p &= Perms::READ;
        assert_eq!(p, Perms::READ);
    }

    #[test]
    fn debug_prints_flag_names()
    {
        assert_eq!(format!("{:?}", Perms::READ), "READ");
        assert_eq!(format!("{:?}", Perms::READ | Perms::WRITE), "READ | WRITE");
        assert_eq!(format!("{:?}", Perms::empty()), "0x0");
    }

    #[test]
    fn flag_count_matches_declared()
    {
        assert_eq!(Perms::FLAG_COUNT, 3);
    }

    #[test]
    fn copy_clone_equality()
    {
        let a = Perms::READ;
        let b = a;
        assert_eq!(a, b);
    }
}
