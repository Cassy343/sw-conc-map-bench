use std::{hash::{Hash, BuildHasher}, collections::HashMap, sync::Arc};
use flurry::Guard;

use crate::api::{ConcurrentMap, ReadHandle, ReadGuard, WriteHandle, WriteGuard};

pub struct FlurryMap<K, V, S> {
    inner: Arc<flurry::HashMap<K, V, S>>
}

impl<K, V, S> Clone for FlurryMap<K, V, S> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl<K, V, S> ConcurrentMap<K, V, S> for FlurryMap<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash + Send + Sync + Clone + Ord,
    V: Send + Sync,
    S: BuildHasher + Clone
{
    type WriteHandle = Self;
    type ReadHandle = Self;

    fn new(inner: HashMap<K, V, S>) -> (Self::WriteHandle, Self::ReadHandle) {
        let (mut write, read) = Self::with_capacity(inner.capacity(), inner.hasher().clone());

        let mut guard = WriteHandle::guard(&mut write);
        for (key, value) in inner {
            guard.insert(key, value);
        }
        drop(guard);

        (write, read)
    }

    fn with_capacity(capacity: usize, hasher: S) -> (Self::WriteHandle, Self::ReadHandle) {
        let me = Self {
            inner: Arc::new(flurry::HashMap::with_capacity_and_hasher(capacity, hasher))
        };

        (me.clone(), me)
    }
}

impl<K, V, S> WriteHandle<K, V, S> for FlurryMap<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash + Send + Sync + Clone + Ord,
    V: Send + Sync,
    S: BuildHasher,
{
    type Guard<'a> = FlurryWriteGuard<'a, K, V, S>;

    fn guard(&mut self) -> Self::Guard<'_> {
        FlurryWriteGuard {
            map: &*self.inner,
            guard: self.inner.guard()
        }
    }
}

pub struct FlurryWriteGuard<'a, K, V, S> {
    map: &'a flurry::HashMap<K, V, S>,
    guard: Guard<'a>
}

impl<'a, K, V, S> WriteGuard<K, V, S> for FlurryWriteGuard<'a, K, V, S>
where
    K: Eq + Hash + Ord + Send + Sync + Clone,
    V: Send + Sync,
    S: BuildHasher,
{
    fn insert(&mut self, key: K, value: V) -> bool {
        self.map.insert(key, value, &self.guard).is_none()
    }

    fn remove(&mut self, key: K) -> bool {
        self.map.remove(&key, &self.guard).is_some()
    }

    fn update(&mut self, key: K, value: V) -> bool {
        self.map.compute_if_present(&key, |_, _| Some(value), &self.guard).is_some()
    }
}

impl<K, V, S> ReadHandle<K, V, S> for FlurryMap<K, V, S>
where
    Self: Send + Clone + 'static,
    K: Eq + Hash + Ord,
    S: BuildHasher,
{
    type Guard<'a> = FlurryReadGuard<'a, K, V, S>;

    fn guard(&self) -> Self::Guard<'_> {
        FlurryReadGuard {
            map: &*self.inner,
            guard: self.inner.guard()
        }
    }
}

pub struct FlurryReadGuard<'a, K, V, S> {
    map: &'a flurry::HashMap<K, V, S>,
    guard: Guard<'a>
}

impl<'a, K, V, S> ReadGuard<K, V, S> for FlurryReadGuard<'a, K, V, S>
where
    K: Eq + Hash + Ord,
    S: BuildHasher,
{
    fn get_and_test<F>(&self, key: &K, test: F) -> Option<bool>
    where
        F: FnOnce(&V) -> bool
    {
        self.map.get(key, &self.guard).map(test)
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}
