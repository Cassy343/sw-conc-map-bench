#![feature(generic_associated_types)]

use std::{collections::{hash_map::RandomState, HashMap, HashSet}, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::{Instant, Duration}};
use adapters::{ArcHashMap, FlashMap, EvMap, DashMap, FlurryMap};
use rand::prelude::*;
use api::ConcurrentMap;
use usync::Barrier;
use rand::seq::SliceRandom;

use crate::api::{ReadHandle, ReadGuard};

mod adapters;
mod api;

const TOTAL_OPS: usize = 20_000_000;

fn main() {
    // let num_physical = num_cpus::get_physical();
    let num_logical = num_cpus::get();

    // println!("Name              Throughput (op/s)       Latency (ns)");
    // bench_one_read_only::<ArcHashMap<_, _, _>>(num_logical, "Arc<HashMap>");
    // bench_one_read_only::<FlashMap>(num_logical, "flashmap");
    // bench_one_read_only::<EvMap>(num_logical, "evmap");
    // bench_one_read_only::<DashMap<_, _, _>>(num_logical, "dashmap");
    // bench_one_read_only::<FlurryMap<_, _, _>>(num_logical, "flurry");

    bench_one::<DashMap<_, _, _>>(num_logical, 1000, "flashmap");
}

fn bench_one_read_only<M: ConcurrentMap<u64, u64, RandomState>>(num_threads: usize, name: &str) {
    const NUM_INSERTED: usize = 1_000_000;

    // Put NUM_INSERTED elements in the map
    let mut rng = thread_rng();
    let mut map = HashMap::<u64, u64, RandomState>::with_capacity(NUM_INSERTED);
    let mut num_inserted = 0;
    while num_inserted < NUM_INSERTED {
        let val = rng.gen::<u64>() % TOTAL_OPS as u64;
        if map.insert(val, val * 2).is_none() {
            num_inserted += 1;
        }
    }
    
    let (write, read) = M::new(map);

    let ops_per_reader = TOTAL_OPS / num_threads;
    let total_ops = ops_per_reader * num_threads;

    let barrier = Arc::new(Barrier::new(num_threads));
    let join_handles = (0..num_threads).map(|i| thread::spawn({
        let read = read.clone();
        let barrier = Arc::clone(&barrier);

        move || {
            barrier.wait();

            let start = Instant::now();
            for key in (i * ops_per_reader .. (i + 1) * ops_per_reader).map(|k| k as u64) {
                assert!(read.guard().get_and_test(&key, |&k| k == key * 2).unwrap_or(true));
            }
            let end = Instant::now();

            ExecutionWindow {
                start,
                end,
                operations: ops_per_reader
            }
        }
    }))
    .collect::<Vec<_>>();
    
    let executions = join_handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>();

    drop(write);

    let throughput = executions.iter()
        .map(ExecutionWindow::throughput)
        .sum::<u64>();

    let total_time = executions.iter()
        .map(ExecutionWindow::time_elapsed)
        .sum::<Duration>();

    let avg_latency = total_time.as_nanos() / total_ops as u128;

    println!("{:<18}{:<24}{}", name, throughput, avg_latency);
}

fn bench_one<M: ConcurrentMap<u64, u64, RandomState>>(
    num_readers: usize,
    writes_per_second: usize,
    name: &str
) {
    const NUM_INSERTED: usize = 1_000_000;
    const RUN_TIME: usize = 1000; // milliseconds
    
    let writes_to_perform = (writes_per_second * RUN_TIME) / 1000;
    let updates = writes_to_perform / 2;
    let removes = writes_to_perform / 4;
    let inserts = writes_to_perform - updates - removes;
    let mut writes = Vec::with_capacity(writes_to_perform);

    assert!(writes_to_perform < NUM_INSERTED);

    let mut rng = thread_rng();

    // Generate NUM_INSERTED keys
    let mut keys = HashSet::with_capacity(NUM_INSERTED);
    while keys.len() < NUM_INSERTED {
        keys.insert(rng.gen::<u64>() % TOTAL_OPS as u64);
    }
    let mut keys = keys.into_iter().collect::<Vec<_>>();

    // Put NUM_INSERTED - inserts elements in the map
    
    let mut map = HashMap::<u64, u64, RandomState>::with_capacity(NUM_INSERTED);
    for key in keys.drain(.. NUM_INSERTED - inserts) {
        if writes.len() < removes {
            writes.push(WriteOperation::Remove(key));
        } else if writes.len() < removes + updates {
            writes.push(WriteOperation::Update(key, key * 2 + 1));
        }

        map.insert(key, key * 2);
    }
    writes.extend(keys.into_iter().map(|key| WriteOperation::Insert(key, key * 2)));
    assert_eq!(writes.len(), writes_to_perform);
    writes.shuffle(&mut rng);
    
    let (write, read) = M::new(map);

    let ops_per_reader = TOTAL_OPS / num_readers;
    let total_ops = ops_per_reader * num_readers;

    let barrier = Arc::new(Barrier::new(num_readers + 1));
    let writer_finished = Arc::new(AtomicBool::new(false));

    let join_handles = (0..num_readers).map(|i| thread::spawn({
        let read = read.clone();
        let barrier = Arc::clone(&barrier);
        let writer_finished = Arc::clone(&writer_finished);

        move || {
            barrier.wait();

            let mut it = (i * ops_per_reader .. (i + 1) * ops_per_reader).map(|k| k as u64).cycle();
            let mut operations = 0;

            let start = Instant::now();
            while !writer_finished.load(Ordering::Acquire) {
                let key = it.next().unwrap();
                let guard = read.guard();
                assert!(read.guard().get_and_test(&key, |&k| k == key * 2).unwrap_or(true));
                drop(guard);
                operations += 1;
            }
            let end = Instant::now();

            ExecutionWindow {
                start,
                end,
                operations
            }
        }
    }))
    .collect::<Vec<_>>();

    barrier.wait();
    for write in writes {
        match write {
            WriteOperation::Insert(key, value) => {
                let start = Instant::now();
            },
            WriteOperation::Remove(key) => {

            },
            WriteOperation::Update(key, value) => {

            }
        }
    }
    writer_finished.store(true, Ordering::Release);
    
    let executions = join_handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>();

    drop(write);

    let throughput = executions.iter()
        .map(ExecutionWindow::throughput)
        .sum::<u64>();

    let total_time = executions.iter()
        .map(ExecutionWindow::time_elapsed)
        .sum::<Duration>();

    let avg_latency = total_time.as_nanos() / total_ops as u128;

    println!("{:<18}{:<24}{}", name, throughput, avg_latency);
}

#[derive(Clone, Copy)]
struct ExecutionWindow {
    start: Instant,
    end: Instant,
    operations: usize,
}

impl ExecutionWindow {
    pub fn time_elapsed(&self) -> Duration {
        self.end - self.start
    }

    // Operations per second
    pub fn throughput(&self) -> u64 {
        let nanos_elapsed = self.time_elapsed().as_nanos();
        let operations = self.operations as u128;

        u64::try_from((operations.checked_mul(1_000_000_000).unwrap()) / nanos_elapsed).unwrap()
    }
}

enum WriteOperation<K, V> {
    Insert(K, V),
    Update(K, V),
    Remove(K)
}
