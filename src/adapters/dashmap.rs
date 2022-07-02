use std::{hash::{Hash, BuildHasher}, collections::HashMap, sync::Arc};
use crate::api::{ConcurrentMap, ReadHandle, ReadGuard, WriteHandle, WriteGuard};

pub struct DashMap<K, V, S> {
    inner: Arc<dashmap::DashMap<K, V, S>>,
}

impl<K, V, S> Clone for DashMap<K, V, S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner)
        }
    }
}

impl<K, V, S> ConcurrentMap<K, V, S> for DashMap<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash,
    S: BuildHasher + Clone
{
    type WriteHandle = Self;
    type ReadHandle = Self;

    fn new(inner: HashMap<K, V, S>) -> (Self::WriteHandle, Self::ReadHandle) {
        let mut map = dashmap::DashMap::with_capacity_and_hasher(inner.len(), inner.hasher().clone());
        map.extend(inner);
        let me = DashMap {
            inner: Arc::new(map)
        };
        (me.clone(), me)
    }

    fn with_capacity(capacity: usize, hasher: S) -> (Self::WriteHandle, Self::ReadHandle) {
        let me = DashMap {
            inner: Arc::new(dashmap::DashMap::with_capacity_and_hasher(capacity, hasher))
        };

        (me.clone(), me)
    }
}

impl<K, V, S> WriteHandle<K, V, S> for DashMap<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    type Guard<'a> = &'a Self;

    fn guard(&mut self) -> Self::Guard<'_> {
        self
    }
}

impl<'a, K, V, S> WriteGuard<K, V, S> for &'a DashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn insert(&mut self, key: K, value: V) -> bool {
        self.inner.insert(key, value).is_none()
    }

    fn remove(&mut self, key: K) -> bool {
        self.inner.remove(&key).is_some()
    }

    fn update(&mut self, key: K, value: V) -> bool {
        self.inner.get_mut(&key)
            .map(|mut slot| *slot = value)
            .is_some()
    }
}

impl<K, V, S> ReadHandle<K, V, S> for DashMap<K, V, S>
where
    Self: Send + Clone + 'static,
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    type Guard<'a> = &'a Self;

    fn guard(&self) -> Self::Guard<'_> {
        self
    }
}

impl<'a, K, V, S> ReadGuard<K, V, S> for &'a DashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn get_and_test<F>(&self, key: &K, test: F) -> Option<bool>
    where
        F: FnOnce(&V) -> bool
    {
        self.inner.get(key).as_deref().map(test)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}
