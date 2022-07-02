use std::{sync::Arc, collections::HashMap, hash::{Hash, BuildHasher}};

use crate::api::{ConcurrentMap, NopWriteHandle, ReadHandle, ReadGuard};

pub struct ArcHashMap<K, V, S> {
    inner: Arc<HashMap<K, V, S>>,
}

impl<K, V, S> Clone for ArcHashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone()
        }
    }
}

impl<K, V, S> ConcurrentMap<K, V, S> for ArcHashMap<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash,
    S: BuildHasher
{
    type WriteHandle = NopWriteHandle<K, V, S>;
    type ReadHandle = Self;

    fn new(inner: HashMap<K, V, S>) -> (Self::WriteHandle, Self::ReadHandle) {
        let me = Self {
            inner: Arc::new(inner)
        };

        (NopWriteHandle::new(), me)
    }

    fn with_capacity(capacity: usize, hasher: S) -> (Self::WriteHandle, Self::ReadHandle) {
        Self::new(HashMap::with_capacity_and_hasher(capacity, hasher))
    }
}

impl<K, V, S> ReadHandle<K, V, S> for ArcHashMap<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash,
    S: BuildHasher
{
    type Guard<'a> = ArcHashMapReadGuard<'a, K, V, S>;

    fn guard(&self) -> Self::Guard<'_> {
        ArcHashMapReadGuard {
            inner: &*self.inner
        }
    }
}

pub struct ArcHashMapReadGuard<'a, K, V, S> {
    inner: &'a HashMap<K, V, S>
}

impl<'a, K, V, S> ReadGuard<K, V, S> for ArcHashMapReadGuard<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher
{
    fn len(&self) -> usize {
        self.inner.len()
    }

    fn get_and_test<F>(&self, key: &K, test: F) -> Option<bool>
    where
        F: FnOnce(&V) -> bool
    {
        self.inner.get(key).map(test)
    }
}