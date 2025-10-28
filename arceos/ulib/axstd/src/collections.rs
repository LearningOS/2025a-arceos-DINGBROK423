//! Collection types.
//! 
//! This module provides HashMap with custom RandomState that uses
//! the random() function from axhal.

use core::hash::{BuildHasher, Hasher};

// Re-export the standard collections from alloc
pub use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, LinkedList, VecDeque, TryReserveError};

// Import hashbrown's HashMap and HashSet
extern crate hashbrown;
use hashbrown::hash_map;
use hashbrown::hash_set;

/// A custom random state that uses axhal's random() function
#[derive(Clone)]
pub struct RandomState {
    k0: u64,
    k1: u64,
}

impl RandomState {
    /// Create a new random state using axhal's random function
    #[inline]
    pub fn new() -> Self {
        let random_value = axhal::misc::random();
        Self {
            k0: random_value as u64,
            k1: (random_value >> 64) as u64,
        }
    }
}

impl Default for RandomState {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        DefaultHasher {
            hasher: ahash::AHasher::default(),
            k0: self.k0,
            k1: self.k1,
        }
    }
}

/// A hasher that wraps ahash
pub struct DefaultHasher {
    hasher: ahash::AHasher,
    k0: u64,
    k1: u64,
}

impl Hasher for DefaultHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // Mix in the random keys
        self.hasher.write_u64(self.k0);
        self.hasher.write_u64(self.k1);
        self.hasher.write(bytes);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

/// A hash map implemented with quadratic probing and SIMD lookup.
///
/// This is a wrapper around hashbrown::HashMap that uses a custom RandomState
/// based on axhal's random() function.
pub struct HashMap<K, V, S = RandomState> {
    base: hashbrown::HashMap<K, V, S>,
}

impl<K, V> HashMap<K, V, RandomState> {
    /// Creates an empty `HashMap`.
    #[inline]
    pub fn new() -> Self {
        Self {
            base: hashbrown::HashMap::with_hasher(RandomState::new()),
        }
    }

    /// Creates an empty `HashMap` with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            base: hashbrown::HashMap::with_capacity_and_hasher(capacity, RandomState::new()),
        }
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Creates an empty `HashMap` which will use the given hash builder to hash keys.
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            base: hashbrown::HashMap::with_hasher(hash_builder),
        }
    }

    /// Creates an empty `HashMap` with the specified capacity, using `hash_builder`
    /// to hash the keys.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            base: hashbrown::HashMap::with_capacity_and_hasher(capacity, hash_builder),
        }
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.base.len()
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    /// Clears the map, removing all key-value pairs.
    #[inline]
    pub fn clear(&mut self) {
        self.base.clear();
    }

    /// Returns a reference to the map's hasher.
    #[inline]
    pub fn hasher(&self) -> &S {
        self.base.hasher()
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + core::hash::Hash,
    S: BuildHasher,
{
    /// Inserts a key-value pair into the map.
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.base.insert(k, v)
    }

    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.base.get(k)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.base.get_mut(k)
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.base.remove(k)
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.base.contains_key(k)
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    #[inline]
    pub fn iter(&self) -> hash_map::Iter<'_, K, V> {
        self.base.iter()
    }

    /// An iterator visiting all key-value pairs in arbitrary order,
    /// with mutable references to the values.
    #[inline]
    pub fn iter_mut(&mut self) -> hash_map::IterMut<'_, K, V> {
        self.base.iter_mut()
    }

    /// An iterator visiting all keys in arbitrary order.
    #[inline]
    pub fn keys(&self) -> hash_map::Keys<'_, K, V> {
        self.base.keys()
    }

    /// An iterator visiting all values in arbitrary order.
    #[inline]
    pub fn values(&self) -> hash_map::Values<'_, K, V> {
        self.base.values()
    }

    /// An iterator visiting all values mutably in arbitrary order.
    #[inline]
    pub fn values_mut(&mut self) -> hash_map::ValuesMut<'_, K, V> {
        self.base.values_mut()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.base.reserve(additional);
    }

    /// Shrinks the capacity of the map as much as possible.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.base.shrink_to_fit();
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    #[inline]
    pub fn entry(&mut self, key: K) -> hash_map::Entry<'_, K, V, S> {
        self.base.entry(key)
    }
}

impl<K, V, S> Default for HashMap<K, V, S>
where
    S: Default,
{
    #[inline]
    fn default() -> Self {
        Self {
            base: hashbrown::HashMap::default(),
        }
    }
}

impl<K, V, S> Clone for HashMap<K, V, S>
where
    K: Clone,
    V: Clone,
    S: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<K, V, S> core::fmt::Debug for HashMap<K, V, S>
where
    K: core::fmt::Debug,
    V: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.base.fmt(f)
    }
}

impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.base.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = hash_map::IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.base.iter_mut()
    }
}

impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = hash_map::IntoIter<K, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }
}

/// A hash set implemented as a `HashMap` where the value is `()`.
pub struct HashSet<T, S = RandomState> {
    base: hashbrown::HashSet<T, S>,
}

impl<T> HashSet<T, RandomState> {
    /// Creates an empty `HashSet`.
    #[inline]
    pub fn new() -> Self {
        Self {
            base: hashbrown::HashSet::with_hasher(RandomState::new()),
        }
    }

    /// Creates an empty `HashSet` with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            base: hashbrown::HashSet::with_capacity_and_hasher(capacity, RandomState::new()),
        }
    }
}

impl<T, S> HashSet<T, S> {
    /// Creates an empty `HashSet` which will use the given hash builder to hash keys.
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            base: hashbrown::HashSet::with_hasher(hash_builder),
        }
    }

    /// Creates an empty `HashSet` with the specified capacity, using `hash_builder`
    /// to hash the keys.
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            base: hashbrown::HashSet::with_capacity_and_hasher(capacity, hash_builder),
        }
    }

    /// Returns the number of elements the set can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }

    /// Returns the number of elements in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.base.len()
    }

    /// Returns `true` if the set contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    /// Clears the set, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.base.clear();
    }

    /// An iterator visiting all elements in arbitrary order.
    #[inline]
    pub fn iter(&self) -> hash_set::Iter<'_, T> {
        self.base.iter()
    }
}

impl<T, S> HashSet<T, S>
where
    T: Eq + core::hash::Hash,
    S: BuildHasher,
{
    /// Adds a value to the set.
    #[inline]
    pub fn insert(&mut self, value: T) -> bool {
        self.base.insert(value)
    }

    /// Returns `true` if the set contains a value.
    #[inline]
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
    where
        T: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.base.contains(value)
    }

    /// Removes a value from the set. Returns `true` if the value was present in the set.
    #[inline]
    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
    where
        T: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.base.remove(value)
    }
}

impl<T, S> Default for HashSet<T, S>
where
    S: Default,
{
    #[inline]
    fn default() -> Self {
        Self {
            base: hashbrown::HashSet::default(),
        }
    }
}

impl<T, S> Clone for HashSet<T, S>
where
    T: Clone,
    S: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<T, S> core::fmt::Debug for HashSet<T, S>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.base.fmt(f)
    }
}

impl<'a, T, S> IntoIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type IntoIter = hash_set::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.base.iter()
    }
}

impl<T, S> IntoIterator for HashSet<T, S> {
    type Item = T;
    type IntoIter = hash_set::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }
}
