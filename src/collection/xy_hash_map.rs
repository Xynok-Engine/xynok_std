#![allow(unused)]

use std::hash::{Hash, Hasher};
use std::ops::{BitXor, Index, IndexMut};

// ══════════════════════════════════════════════════════════════════════════════
// FxHasher — same algorithm as rustc-hash / Firefox. Single multiply per word,
// branch-free, excellent distribution for integer and string keys.
// ══════════════════════════════════════════════════════════════════════════════

const FX_MUL: u64 = 0x517c_c1b7_2722_0a95;

#[derive(Default, Clone)]
struct FxHasher(u64);

impl Hasher for FxHasher
{
    #[inline]
    fn finish(&self) -> u64 { self.0 }

    #[inline]
    fn write(&mut self, bytes: &[u8])
    {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks
        {
            self.mix(u64::from_ne_bytes(chunk.try_into().unwrap()));
        }
        let rem = chunks.remainder();
        if !rem.is_empty()
        {
            let mut v = 0u64;
            for (i, &b) in rem.iter().enumerate()
            {
                v |= (b as u64) << (i * 8);
            }
            self.mix(v);
        }
    }

    // Overrides for scalar types bypass the `write` byte loop entirely.
    #[inline]
    fn write_u8(&mut self, i: u8) { self.mix(i as u64) }
    #[inline]
    fn write_u16(&mut self, i: u16) { self.mix(i as u64) }
    #[inline]
    fn write_u32(&mut self, i: u32) { self.mix(i as u64) }
    #[inline]
    fn write_u64(&mut self, i: u64) { self.mix(i) }
    #[inline]
    fn write_usize(&mut self, i: usize) { self.mix(i as u64) }
    #[inline]
    fn write_i8(&mut self, i: i8) { self.mix(i as u64) }
    #[inline]
    fn write_i16(&mut self, i: i16) { self.mix(i as u64) }
    #[inline]
    fn write_i32(&mut self, i: i32) { self.mix(i as u64) }
    #[inline]
    fn write_i64(&mut self, i: i64) { self.mix(i as u64) }
    #[inline]
    fn write_isize(&mut self, i: isize) { self.mix(i as u64) }
}

impl FxHasher
{
    #[inline]
    fn mix(&mut self, val: u64) { self.0 = self.0.rotate_left(5).bitxor(val).wrapping_mul(FX_MUL); }
}

#[inline]
fn fx_hash<K: Hash>(key: &K) -> u64
{
    let mut h = FxHasher(0);
    key.hash(&mut h);
    h.finish()
}

// ══════════════════════════════════════════════════════════════════════════════
// Table slot encoding  (u32 per slot — 4 bytes, fits in one cache line per 16)
//
//   EMPTY     = u32::MAX       — slot has never been used
//   TOMBSTONE = u32::MAX - 1   — slot was deleted (probe must continue past it)
//   0..=MAX-2                  — direct index into keys[] / values[]
// ══════════════════════════════════════════════════════════════════════════════

const EMPTY: u32 = u32::MAX;
const TOMBSTONE: u32 = u32::MAX - 1;
const INIT_CAP: usize = 8; // smallest table size, must be power of 2

#[inline]
fn table_size_for(n: usize) -> usize
{
    // Ensure load factor stays at or below 75%: table_size * 3 >= n * 4
    if n == 0
    {
        return INIT_CAP;
    }
    let raw = (n.saturating_mul(4) + 2) / 3; // ceil(n*4/3)
    raw.max(INIT_CAP).next_power_of_two()
}

// ══════════════════════════════════════════════════════════════════════════════
// XyHashMap
//
// Layout:
//   table  — flat open-addressed hash table of u32 slot indices, power-of-2 sized
//   keys   — contiguous Vec<K>, iterable without touching the table
//   values — contiguous Vec<V>, parallel to keys
//
// Complexity:
//   insert / get / remove — O(1) average, O(n) worst (linear probe on collision)
//   iter / iter_mut       — O(n), fully cache-friendly (scans two flat Vecs)
//   retain                — O(n) + O(n) rehash
// ══════════════════════════════════════════════════════════════════════════════

pub struct XyHashMap<K, V>
{
    keys:   Vec<K>,
    values: Vec<V>,
    table:  Vec<u32>, // EMPTY | TOMBSTONE | data_index
    len:    usize,    // live entries
    dead:   usize,    // tombstone slots (count toward load, but hold no data)
}

// ── Construction ──────────────────────────────────────────────────────────────

impl<K, V> XyHashMap<K, V>
{
    #[inline]
    pub fn new() -> Self
    {
        Self {
            keys:   Vec::new(),
            values: Vec::new(),
            table:  Vec::new(), // lazily allocated on first insert
            len:    0,
            dead:   0,
        }
    }

    #[inline]
    pub fn with_capacity(n: usize) -> Self
    {
        let table_size = table_size_for(n);
        Self {
            keys:   Vec::with_capacity(n),
            values: Vec::with_capacity(n),
            table:  vec![EMPTY; table_size],
            len:    0,
            dead:   0,
        }
    }
}

impl<K, V> Default for XyHashMap<K, V>
{
    fn default() -> Self { Self::new() }
}

// ── Hot-path internals ────────────────────────────────────────────────────────

impl<K, V> XyHashMap<K, V>
{
    /// Finds an occupied slot for `key`. Returns `(slot_pos, data_idx)` or `None`.
    #[inline]
    fn slot_of(&self, hash: u64, key: &K) -> Option<(usize, usize)>
    where K: Eq
    {
        if self.table.is_empty()
        {
            return None;
        }
        let mask = self.table.len() - 1;
        let mut pos = hash as usize & mask;
        loop
        {
            // SAFETY: pos is always in-bounds because mask = table.len()-1 and pos &= mask.
            let slot = unsafe { *self.table.get_unchecked(pos) };
            if slot == EMPTY
            {
                return None;
            }
            if slot != TOMBSTONE
            {
                let data_idx = slot as usize;
                // SAFETY: valid data_idx is always < keys.len() by construction.
                if unsafe { self.keys.get_unchecked(data_idx) } == key
                {
                    return Some((pos, data_idx));
                }
            }
            pos = (pos + 1) & mask;
        }
    }

    /// Finds the best slot for inserting at `hash` (first tombstone or first empty).
    /// Caller must ensure the table has room (try_grow already called).
    #[inline]
    fn vacant_slot(&self, hash: u64) -> usize
    {
        let mask = self.table.len() - 1;
        let mut pos = hash as usize & mask;
        let mut first_tomb: Option<usize> = None;
        loop
        {
            let slot = unsafe { *self.table.get_unchecked(pos) };
            if slot == EMPTY
            {
                return first_tomb.unwrap_or(pos);
            }
            if slot == TOMBSTONE && first_tomb.is_none()
            {
                first_tomb = Some(pos);
            }
            pos = (pos + 1) & mask;
        }
    }

    /// Grows or cleans tombstones before an insertion.
    #[inline]
    fn try_grow(&mut self)
    where K: Hash
    {
        if self.table.is_empty()
        {
            self.table = vec![EMPTY; INIT_CAP];
            return;
        }
        let t = self.table.len();
        if (self.len + 1) * 4 > t * 3
        {
            self.rehash(t * 2); // grow: halves the load factor
        }
        else if (self.len + self.dead + 1) * 4 > t * 3
        {
            self.rehash(t); // same size, tombstones cleared
        }
    }

    /// Rebuilds the hash table from scratch (no tombstones remain after).
    fn rehash(&mut self, new_size: usize)
    where K: Hash
    {
        debug_assert!(new_size.is_power_of_two());
        let mask = new_size - 1;
        // Reuse allocation if size matches, otherwise reallocate.
        if self.table.len() != new_size
        {
            self.table = vec![EMPTY; new_size];
        }
        else
        {
            self.table.fill(EMPTY);
        }
        for (data_idx, key) in self.keys.iter().enumerate()
        {
            let mut pos = fx_hash(key) as usize & mask;
            loop
            {
                if unsafe { *self.table.get_unchecked(pos) } == EMPTY
                {
                    unsafe {
                        *self.table.get_unchecked_mut(pos) = data_idx as u32;
                    }
                    break;
                }
                pos = (pos + 1) & mask;
            }
        }
        self.dead = 0;
    }

    /// After `keys.swap_remove(removed_idx)` / `values.swap_remove(removed_idx)`:
    /// the element formerly at `old_last = keys.len()` (post-removal) is now at
    /// `removed_idx`. We must update its hash-table slot from old_last → removed_idx.
    #[inline]
    fn swap_remove_fixup(&mut self, removed_idx: usize)
    where K: Hash
    {
        let old_last = self.keys.len(); // == new len after swap_remove
        if removed_idx == old_last
        {
            return;
        } // we removed the last element, no swap happened

        // keys[removed_idx] is the element that was at old_last. Find its slot.
        let hash = fx_hash(unsafe { self.keys.get_unchecked(removed_idx) });
        let mask = self.table.len() - 1;
        let mut pos = hash as usize & mask;
        loop
        {
            let slot = unsafe { *self.table.get_unchecked(pos) };
            debug_assert!(slot != EMPTY, "swap_remove_fixup: expected slot not found");
            if slot == old_last as u32
            {
                unsafe {
                    *self.table.get_unchecked_mut(pos) = removed_idx as u32;
                }
                return;
            }
            pos = (pos + 1) & mask;
        }
    }
}

// ── Core public API ───────────────────────────────────────────────────────────

impl<K, V> XyHashMap<K, V>
{
    #[inline]
    pub fn len(&self) -> usize { self.len }
    #[inline]
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Number of entries the map can hold before the next reallocation.
    #[inline]
    pub fn capacity(&self) -> usize
    {
        if self.table.is_empty()
        {
            return 0;
        }
        self.table.len() * 3 / 4
    }

    /// Removes all entries. Resets the table to all-empty (no reallocation).
    #[inline]
    pub fn clear(&mut self)
    {
        self.keys.clear();
        self.values.clear();
        self.table.fill(EMPTY);
        self.len = 0;
        self.dead = 0;
    }
}

impl<K: Hash + Eq, V> XyHashMap<K, V>
{
    /// Returns `true` if the map contains `key`.
    #[inline]
    pub fn contains_key(&self, key: &K) -> bool { self.slot_of(fx_hash(key), key).is_some() }

    /// Returns a shared reference to the value for `key`, or `None`.
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V> { self.slot_of(fx_hash(key), key).map(|(_, idx)| unsafe { self.values.get_unchecked(idx) }) }

    /// Returns a mutable reference to the value for `key`, or `None`.
    #[inline]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> { self.slot_of(fx_hash(key), key).map(|(_, idx)| unsafe { self.values.get_unchecked_mut(idx) }) }

    /// Inserts `(key, value)`. Returns the old value if the key already existed.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    {
        let hash = fx_hash(&key);
        if let Some((_, idx)) = self.slot_of(hash, &key)
        {
            return Some(std::mem::replace(unsafe { self.values.get_unchecked_mut(idx) }, value));
        }
        self.try_grow();
        let slot = self.vacant_slot(hash);
        let data_idx = self.keys.len() as u32;
        if unsafe { *self.table.get_unchecked(slot) } == TOMBSTONE
        {
            self.dead -= 1;
        }
        unsafe {
            *self.table.get_unchecked_mut(slot) = data_idx;
        }
        self.keys.push(key);
        self.values.push(value);
        self.len += 1;
        None
    }

    /// Removes the entry for `key` and returns the value, or `None`.
    pub fn remove(&mut self, key: &K) -> Option<V>
    {
        let (slot, idx) = self.slot_of(fx_hash(key), key)?;
        unsafe {
            *self.table.get_unchecked_mut(slot) = TOMBSTONE;
        }
        self.dead += 1;
        self.len -= 1;
        self.keys.swap_remove(idx);
        let v = self.values.swap_remove(idx);
        self.swap_remove_fixup(idx);
        Some(v)
    }

    /// Removes the entry and returns `(key, value)`, or `None`.
    pub fn remove_entry(&mut self, key: &K) -> Option<(K, V)>
    {
        let (slot, idx) = self.slot_of(fx_hash(key), key)?;
        unsafe {
            *self.table.get_unchecked_mut(slot) = TOMBSTONE;
        }
        self.dead += 1;
        self.len -= 1;
        let k = self.keys.swap_remove(idx);
        let v = self.values.swap_remove(idx);
        self.swap_remove_fixup(idx);
        Some((k, v))
    }

    /// Returns `&mut V`, inserting `default` if absent.
    pub fn get_or_insert(&mut self, key: K, default: V) -> &mut V { self.entry(key).or_insert(default) }

    /// Returns `&mut V`, inserting `f()` if absent.
    pub fn get_or_insert_with<F: FnOnce() -> V>(&mut self, key: K, f: F) -> &mut V { self.entry(key).or_insert_with(f) }

    /// Keeps only entries for which `f` returns `true`. Rebuilds the table afterward.
    pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F)
    {
        let mut i = 0;
        while i < self.keys.len()
        {
            if f(unsafe { self.keys.get_unchecked(i) }, unsafe { self.values.get_unchecked_mut(i) })
            {
                i += 1;
            }
            else
            {
                self.keys.swap_remove(i);
                self.values.swap_remove(i);
                self.len -= 1;
            }
        }
        // Rebuild hash table: indices have shifted due to multiple swap_removes.
        let new_size = table_size_for(self.len);
        self.rehash(new_size);
    }

    /// Returns an `Entry` for in-place get-or-insert patterns.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V>
    {
        let hash = fx_hash(&key);
        if let Some((slot_pos, data_idx)) = self.slot_of(hash, &key)
        {
            return Entry::Occupied(OccupiedEntry { map: self, slot_pos, data_idx });
        }
        // Reserve space *before* finding the vacant slot so the table pointer
        // is stable for the lifetime of the returned VacantEntry.
        self.try_grow();
        let slot_pos = self.vacant_slot(hash);
        Entry::Vacant(VacantEntry { map: self, key, slot_pos })
    }
}

// ── Raw slice access (unique to XyHashMap) ─────────────────────────────────────

impl<K, V> XyHashMap<K, V>
{
    /// All keys as a contiguous slice (insertion-like order, may change on remove).
    #[inline]
    pub fn keys(&self) -> &[K] { &self.keys }
    /// All values as a contiguous slice, parallel to `keys()`.
    #[inline]
    pub fn values(&self) -> &[V] { &self.values }
    /// Mutable slice of all values.
    #[inline]
    pub fn values_mut(&mut self) -> &mut [V] { &mut self.values }
    /// Consumes the map, returning the two backing `Vec`s as `(keys, values)`.
    #[inline]
    pub fn into_vecs(self) -> (Vec<K>, Vec<V>) { (self.keys, self.values) }
}

// ── Entry API ─────────────────────────────────────────────────────────────────

pub enum Entry<'a, K, V>
{
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

pub struct OccupiedEntry<'a, K, V>
{
    map:      &'a mut XyHashMap<K, V>,
    slot_pos: usize, // position in map.table
    data_idx: usize, // position in map.keys / map.values
}

pub struct VacantEntry<'a, K, V>
{
    map:      &'a mut XyHashMap<K, V>,
    key:      K,
    slot_pos: usize, // position in map.table (valid until next mutation)
}

impl<'a, K: Hash + Eq, V> Entry<'a, K, V>
{
    pub fn or_insert(self, default: V) -> &'a mut V
    {
        match self
        {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, f: F) -> &'a mut V
    {
        match self
        {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(f()),
        }
    }

    pub fn or_default(self) -> &'a mut V
    where V: Default
    {
        self.or_insert_with(V::default)
    }

    pub fn and_modify<F: FnOnce(&mut V)>(self, f: F) -> Self
    {
        match self
        {
            Entry::Occupied(mut e) =>
            {
                f(e.get_mut());
                Entry::Occupied(e)
            }
            Entry::Vacant(e) => Entry::Vacant(e),
        }
    }

    pub fn key(&self) -> &K
    {
        match self
        {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }
}

impl<'a, K: Hash + Eq, V> OccupiedEntry<'a, K, V>
{
    #[inline]
    pub fn key(&self) -> &K { unsafe { self.map.keys.get_unchecked(self.data_idx) } }
    #[inline]
    pub fn get(&self) -> &V { unsafe { self.map.values.get_unchecked(self.data_idx) } }
    #[inline]
    pub fn get_mut(&mut self) -> &mut V { unsafe { self.map.values.get_unchecked_mut(self.data_idx) } }
    #[inline]
    pub fn into_mut(self) -> &'a mut V { unsafe { self.map.values.get_unchecked_mut(self.data_idx) } }

    pub fn insert(&mut self, value: V) -> V { std::mem::replace(unsafe { self.map.values.get_unchecked_mut(self.data_idx) }, value) }

    pub fn remove(self) -> V
    {
        unsafe {
            *self.map.table.get_unchecked_mut(self.slot_pos) = TOMBSTONE;
        }
        self.map.dead += 1;
        self.map.len -= 1;
        self.map.keys.swap_remove(self.data_idx);
        let v = self.map.values.swap_remove(self.data_idx);
        self.map.swap_remove_fixup(self.data_idx);
        v
    }
}

impl<'a, K: Hash + Eq, V> VacantEntry<'a, K, V>
{
    #[inline]
    pub fn key(&self) -> &K { &self.key }
    #[inline]
    pub fn into_key(self) -> K { self.key }

    pub fn insert(self, value: V) -> &'a mut V
    {
        let VacantEntry { map, key, slot_pos } = self;
        let data_idx = map.keys.len();
        if unsafe { *map.table.get_unchecked(slot_pos) } == TOMBSTONE
        {
            map.dead -= 1;
        }
        unsafe {
            *map.table.get_unchecked_mut(slot_pos) = data_idx as u32;
        }
        map.keys.push(key);
        map.values.push(value);
        map.len += 1;
        unsafe { map.values.get_unchecked_mut(data_idx) }
    }
}

// ── Iterators ─────────────────────────────────────────────────────────────────

pub struct Iter<'a, K, V>
{
    keys:   std::slice::Iter<'a, K>,
    values: std::slice::Iter<'a, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
{
    type Item = (&'a K, &'a V);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> { Some((self.keys.next()?, self.values.next()?)) }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { self.keys.size_hint() }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {}

pub struct IterMut<'a, K, V>
{
    keys:   std::slice::Iter<'a, K>,
    values: std::slice::IterMut<'a, V>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
{
    type Item = (&'a K, &'a mut V);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> { Some((self.keys.next()?, self.values.next()?)) }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { self.keys.size_hint() }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {}

pub struct IntoIter<K, V>
{
    keys:   std::vec::IntoIter<K>,
    values: std::vec::IntoIter<V>,
}

impl<K, V> Iterator for IntoIter<K, V>
{
    type Item = (K, V);
    #[inline]
    fn next(&mut self) -> Option<Self::Item> { Some((self.keys.next()?, self.values.next()?)) }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { self.keys.size_hint() }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}

impl<K, V> XyHashMap<K, V>
{
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> { Iter { keys: self.keys.iter(), values: self.values.iter() } }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V>
    {
        IterMut {
            keys:   self.keys.iter(),
            values: self.values.iter_mut(),
        }
    }
}

impl<K, V> IntoIterator for XyHashMap<K, V>
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter
    {
        IntoIter {
            keys:   self.keys.into_iter(),
            values: self.values.into_iter(),
        }
    }
}

impl<'a, K, V> IntoIterator for &'a XyHashMap<K, V>
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter { self.iter() }
}

impl<'a, K, V> IntoIterator for &'a mut XyHashMap<K, V>
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter { self.iter_mut() }
}

// ── FromIterator / Extend ─────────────────────────────────────────────────────

impl<K: Hash + Eq, V> FromIterator<(K, V)> for XyHashMap<K, V>
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self
    {
        let iter = iter.into_iter();
        let (lo, _) = iter.size_hint();
        let mut map = Self::with_capacity(lo);
        for (k, v) in iter
        {
            map.insert(k, v);
        }
        map
    }
}

impl<K: Hash + Eq, V> Extend<(K, V)> for XyHashMap<K, V>
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I)
    {
        for (k, v) in iter
        {
            self.insert(k, v);
        }
    }
}

// ── Index / IndexMut ─────────────────────────────────────────────────────────

impl<K: Hash + Eq, V> Index<&K> for XyHashMap<K, V>
{
    type Output = V;
    #[inline]
    fn index(&self, key: &K) -> &V { self.get(key).expect("XyHashMap: key not found") }
}

impl<K: Hash + Eq, V> IndexMut<&K> for XyHashMap<K, V>
{
    #[inline]
    fn index_mut(&mut self, key: &K) -> &mut V { self.get_mut(key).expect("XyHashMap: key not found") }
}

// ── Debug / Clone / PartialEq / Eq ───────────────────────────────────────────

impl<K: std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug for XyHashMap<K, V>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        let mut m = f.debug_map();
        for (k, v) in self.iter()
        {
            m.entry(k, v);
        }
        m.finish()
    }
}

impl<K: Clone, V: Clone> Clone for XyHashMap<K, V>
{
    fn clone(&self) -> Self
    {
        Self {
            keys:   self.keys.clone(),
            values: self.values.clone(),
            table:  self.table.clone(),
            len:    self.len,
            dead:   self.dead,
        }
    }
}

impl<K: Hash + Eq, V: PartialEq> PartialEq for XyHashMap<K, V>
{
    fn eq(&self, other: &Self) -> bool { self.len == other.len && self.iter().all(|(k, v)| other.get(k) == Some(v)) }
}

impl<K: Hash + Eq, V: Eq> Eq for XyHashMap<K, V> {}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn insert_and_get()
    {
        let mut m: XyHashMap<&str, i32> = XyHashMap::new();
        assert!(m.insert("a", 1).is_none());
        assert_eq!(m.get(&"a"), Some(&1));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn insert_replaces_existing()
    {
        let mut m: XyHashMap<&str, i32> = XyHashMap::new();
        m.insert("a", 1);
        assert_eq!(m.insert("a", 2), Some(1));
        assert_eq!(m.get(&"a"), Some(&2));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn remove()
    {
        let mut m: XyHashMap<&str, i32> = XyHashMap::new();
        m.insert("a", 10);
        assert_eq!(m.remove(&"a"), Some(10));
        assert!(m.is_empty());
        assert!(m.get(&"a").is_none());
    }

    #[test]
    fn remove_swap_fixup()
    {
        // Ensures that removing a non-last element correctly fixes the swapped entry.
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..8
        {
            m.insert(i, i * 10);
        }
        m.remove(&0); // 0 was at index 0; last element swapped to index 0
        for i in 1..8
        {
            assert_eq!(m.get(&i), Some(&(i * 10)), "key {i} broken after remove");
        }
        assert!(m.get(&0).is_none());
    }

    #[test]
    fn contains_key()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        m.insert(1, 100);
        assert!(m.contains_key(&1));
        assert!(!m.contains_key(&2));
    }

    #[test]
    fn entry_or_insert_counter()
    {
        let mut m: XyHashMap<&str, u32> = XyHashMap::new();
        *m.entry("hits").or_insert(0) += 1;
        *m.entry("hits").or_insert(0) += 1;
        assert_eq!(m[&"hits"], 2);
    }

    #[test]
    fn entry_and_modify()
    {
        let mut m: XyHashMap<&str, i32> = XyHashMap::new();
        m.insert("x", 1);
        m.entry("x").and_modify(|v| *v += 9).or_insert(0);
        assert_eq!(m[&"x"], 10);
    }

    #[test]
    fn grow_and_rehash()
    {
        // Insert enough entries to trigger multiple rehashes.
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..100
        {
            m.insert(i, i * 2);
        }
        for i in 0..100
        {
            assert_eq!(m.get(&i), Some(&(i * 2)));
        }
        assert_eq!(m.len(), 100);
    }

    #[test]
    fn tombstone_reuse()
    {
        // Insert, remove, re-insert the same key — must find via tombstone slot.
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..10
        {
            m.insert(i, i);
        }
        for i in 0..5
        {
            m.remove(&i);
        }
        for i in 0..5
        {
            m.insert(i, i + 100);
        }
        for i in 0..5
        {
            assert_eq!(m.get(&i), Some(&(i + 100)));
        }
        for i in 5..10
        {
            assert_eq!(m.get(&i), Some(&i));
        }
        assert_eq!(m.len(), 10);
    }

    #[test]
    fn retain()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..8
        {
            m.insert(i, i);
        }
        m.retain(|k, _| k % 2 == 0);
        assert_eq!(m.len(), 4);
        for i in (0..8).step_by(2)
        {
            assert!(m.contains_key(&i));
        }
        for i in (1..8).step_by(2)
        {
            assert!(!m.contains_key(&i));
        }
    }

    #[test]
    fn from_iterator()
    {
        let m: XyHashMap<_, _> = vec![("x", 1), ("y", 2)].into_iter().collect();
        assert_eq!(m.len(), 2);
        assert_eq!(m[&"x"], 1);
        assert_eq!(m[&"y"], 2);
    }

    #[test]
    fn keys_values_slices()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        m.insert(1, 10);
        m.insert(2, 20);
        assert_eq!(m.keys().len(), 2);
        assert_eq!(m.values().len(), 2);
        assert!(m.keys().contains(&1));
        assert!(m.values().contains(&10));
    }

    #[test]
    fn into_vecs_round_trip()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        m.insert(1, 10);
        m.insert(2, 20);
        let (keys, values) = m.into_vecs();
        assert_eq!(keys.len(), 2);
        assert_eq!(values.len(), 2);
        assert!(keys.contains(&1) && keys.contains(&2));
    }

    #[test]
    fn index_operator()
    {
        let mut m: XyHashMap<&str, i32> = XyHashMap::new();
        m.insert("k", 42);
        assert_eq!(m[&"k"], 42);
        m[&"k"] = 99;
        assert_eq!(m[&"k"], 99);
    }

    #[test]
    fn clone_and_eq()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..10
        {
            m.insert(i, i);
        }
        let m2 = m.clone();
        assert_eq!(m, m2);
    }

    #[test]
    fn clear()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..20
        {
            m.insert(i, i);
        }
        m.clear();
        assert!(m.is_empty());
        m.insert(42, 1);
        assert_eq!(m.get(&42), Some(&1));
    }

    #[test]
    fn iter_covers_all_entries()
    {
        let mut m: XyHashMap<i32, i32> = XyHashMap::new();
        for i in 0..20
        {
            m.insert(i, i * 3);
        }
        let mut sum = 0i32;
        for (_, v) in &m
        {
            sum += v;
        }
        assert_eq!(sum, (0..20i32).map(|i| i * 3).sum());
    }
}
