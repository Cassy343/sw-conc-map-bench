use std::{marker::PhantomData, collections::HashMap};

pub trait ConcurrentMap<K, V, S> {
    type WriteHandle: WriteHandle<K, V, S>;
    type ReadHandle: ReadHandle<K, V, S>;

    fn new(inner: HashMap<K, V, S>) -> (Self::WriteHandle, Self::ReadHandle);

    fn with_capacity(capacity: usize, hasher: S) -> (Self::WriteHandle, Self::ReadHandle);
}

pub trait WriteHandle<K, V, S>: Send + 'static {
    type Guard<'a>: WriteGuard<K, V, S>;

    fn guard(&mut self) -> Self::Guard<'_>;
}

pub trait WriteGuard<K, V, S> {
    fn insert(&mut self, key: K, value: V) -> bool;

    fn remove(&mut self, key: K) -> bool;

    fn update(&mut self, key: K, f: V) -> bool;
}

pub trait ReadHandle<K, V, S>: Send + Clone + 'static {
    type Guard<'a>: ReadGuard<K, V, S>;

    fn guard(&self) -> Self::Guard<'_>;
}

pub trait ReadGuard<K, V, S> {
    fn len(&self) -> usize;

    fn get_and_test<F>(&self, key: &K, test: F) -> Option<bool>
    where
        F: FnOnce(&V) -> bool;
}

pub struct NopWriteHandle<K, V, S> {
    _marker: PhantomData<(K, V, S)>
}

unsafe impl<K, V, S> Send for NopWriteHandle<K, V, S> {}

impl<K, V, S> Clone for NopWriteHandle<K, V, S> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<K, V, S> Copy for NopWriteHandle<K, V, S> {}

impl<K, V, S> NopWriteHandle<K, V, S> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<K, V, S> WriteHandle<K, V, S> for NopWriteHandle<K, V, S>
where
    Self: 'static
{
    type Guard<'a> = Self;

    fn guard(&mut self) -> Self::Guard<'_> {
        *self
    }
}

impl<K, V, S> WriteGuard<K, V, S> for NopWriteHandle<K, V, S> {
    fn insert(&mut self, _key: K, _value: V) -> bool {
        true
    }

    fn remove(&mut self, _key: K) -> bool {
        true
    }

    fn update(&mut self, _key: K, _value: V) -> bool {
        true
    }
}
