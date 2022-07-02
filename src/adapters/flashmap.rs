use std::{hash::{Hash, BuildHasher}, collections::HashMap};
use flashmap::{InsertionResult, RemovalResult};

use crate::api::{ConcurrentMap, ReadHandle, ReadGuard, WriteHandle, WriteGuard};

pub struct FlashMap;

impl<K, V, S> ConcurrentMap<K, V, S> for FlashMap
where
    Self: Send + 'static,
    flashmap::ReadHandle<K, V, S>: Send + 'static,
    flashmap::WriteHandle<K, V, S>: Send + 'static,
    K: Eq + Hash,
    S: BuildHasher + Clone
{
    type WriteHandle = flashmap::WriteHandle<K, V, S>;
    type ReadHandle = flashmap::ReadHandle<K, V, S>;

    fn new(inner: HashMap<K, V, S>) -> (Self::WriteHandle, Self::ReadHandle) {
        let (mut write, read) = Self::with_capacity(inner.capacity(), inner.hasher().clone());

        let mut guard = write.guard();
        for (key, value) in inner {
            guard.insert(key, value);
        }
        drop(guard);

        (write, read)
    }

    fn with_capacity(capacity: usize, hasher: S) -> (Self::WriteHandle, Self::ReadHandle) {
        flashmap::Builder::new()
            .with_capacity(capacity)
            .with_hasher(hasher)
            .build()
    }
}

impl<K, V, S> WriteHandle<K, V, S> for flashmap::WriteHandle<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash,
    S: BuildHasher,
{
    type Guard<'a> = flashmap::WriteGuard<'a, K, V, S>;

    fn guard(&mut self) -> Self::Guard<'_> {
        self.guard()
    }
}

impl<'a, K, V, S> WriteGuard<K, V, S> for flashmap::WriteGuard<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn insert(&mut self, key: K, value: V) -> bool {
        flashmap::WriteGuard::insert(self, key, value) == InsertionResult::Inserted
    }

    fn remove(&mut self, key: K) -> bool {
        flashmap::WriteGuard::remove(self, key) == RemovalResult::Removed
    }

    fn update(&mut self, key: K, value: V) -> bool {
        flashmap::WriteGuard::rcu(self, key, |_| value)
    }
}

impl<K, V, S> ReadHandle<K, V, S> for flashmap::ReadHandle<K, V, S>
where
    Self: Send + Clone + 'static,
    K: Eq + Hash,
    S: BuildHasher,
{
    type Guard<'a> = flashmap::ReadGuard<'a, K, V, S>;

    fn guard(&self) -> Self::Guard<'_> {
        self.guard()
    }
}

impl<'a, K, V, S> ReadGuard<K, V, S> for flashmap::ReadGuard<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    fn get_and_test<F>(&self, key: &K, test: F) -> Option<bool>
    where
        F: FnOnce(&V) -> bool
    {
        flashmap::ReadGuard::get(self, key).map(test)
    }

    fn len(&self) -> usize {
        flashmap::ReadGuard::len(self)
    }
}
