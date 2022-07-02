use std::collections::HashMap;
use std::hash::{Hash, BuildHasher};
use evmap::refs::MapReadRef;

use crate::api::{ConcurrentMap, WriteHandle, WriteGuard, ReadHandle, ReadGuard};

type EvWriteHandle<K, V, S> = evmap::handles::WriteHandle<K, V, (), S>;
type EvReadHandle<K, V, S> = evmap::handles::ReadHandle<K, V, (), S>;

pub struct EvMap;

impl<K, V, S> ConcurrentMap<K, V, S> for EvMap
where
    Self: Send + 'static,
    EvReadHandle<K, V, S>: Send + 'static,
    EvWriteHandle<K, V, S>: Send + 'static,
    K: Eq + Hash + Clone,
    V: Eq + Hash,
    S: BuildHasher + Clone
{
    type WriteHandle = EvWriteHandle<K, V, S>;
    type ReadHandle = EvReadHandle<K, V, S>;

    fn new(inner: HashMap<K, V, S>) -> (Self::WriteHandle, Self::ReadHandle) {
        let (mut write, read) = Self::with_capacity(inner.capacity(), inner.hasher().clone());

        for (key, value) in inner {
            write.insert(key, value);
        }
        write.publish();

        (write, read)
    }

    fn with_capacity(capacity: usize, hasher: S) -> (Self::WriteHandle, Self::ReadHandle) {
        unsafe {
            evmap::Options::default()
                .with_capacity(capacity)
                .with_hasher(hasher)
                .assert_stable()
        }
    }
}

impl<K, V, S> WriteHandle<K, V, S> for EvWriteHandle<K, V, S>
where
    Self: Send + 'static,
    K: Eq + Hash + Clone,
    V: Eq + Hash,
    S: BuildHasher + Clone,
{
    type Guard<'a> = &'a mut Self;

    fn guard(&mut self) -> Self::Guard<'_> {
        self
    }
}

// TODO: reconcile API differences
impl<'a, K, V, S> WriteGuard<K, V, S> for &'a mut EvWriteHandle<K, V, S>
where
    K: Eq + Hash + Clone,
    V: Eq + Hash,
    S: BuildHasher + Clone,
{
    fn insert(&mut self, key: K, value: V) -> bool {
        EvWriteHandle::insert(self, key, value).publish();
        true
    }

    fn remove(&mut self, key: K) -> bool {
        EvWriteHandle::remove_entry(self, key).publish();
        true
    }

    fn update(&mut self, key: K, value: V) -> bool {
        EvWriteHandle::update(self, key, value).publish();
        true
    }
}

impl<K, V, S> ReadHandle<K, V, S> for EvReadHandle<K, V, S>
where
    Self: Send + Clone + 'static,
    K: Eq + Hash,
    V: Eq + Hash,
    S: BuildHasher,
{
    type Guard<'a> = MapReadRef<'a, K, V, (), S>;

    fn guard(&self) -> Self::Guard<'_> {
        self.enter().unwrap()
    }
}

impl<'a, K, V, S> ReadGuard<K, V, S> for MapReadRef<'a, K, V, (), S>
where
    K: Eq + Hash,
    V: Eq + Hash,
    S: BuildHasher,
{
    fn get_and_test<F>(&self, key: &K, test: F) -> Option<bool>
    where
        F: FnOnce(&V) -> bool
    {
        MapReadRef::get_one(self, key).map(test)
    }

    fn len(&self) -> usize {
        MapReadRef::len(self)
    }
}