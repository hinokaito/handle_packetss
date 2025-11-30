// False Sharing Benchmark
// Adjacent memory (data stored in the same cache line) is rewritten simultaneously from different threads.
//
// What is False Sharing?
// - Different threads access "independent data"
// - But that data resides on the same cache line (typically 64 bytes)
// - Result: Frequent cache invalidation → Severe performance degradation

use std::cell::UnsafeCell;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

const ITERATIONS: u64 = 100_000_000;
const NUM_THREADS: usize = 4;
const CACHE_LINE_SIZE: usize = 64;

// ============================================================
// Case 1: False Sharing occurs (adjacent counters)
// ============================================================
// 4 u64 values placed contiguously → high chance they all fit in the same cache line
// (u64 = 8 bytes × 4 = 32 bytes < 64 bytes)
#[repr(C)]
struct SharedCounters {
    counters: [UnsafeCell<u64>; NUM_THREADS],
}

// Required to share a struct containing UnsafeCell across threads
unsafe impl Sync for SharedCounters {}

// ============================================================
// Case 2: False Sharing avoided (separated by padding)
// ============================================================
// Each counter aligned to 64-byte boundary → placed on separate cache lines
#[repr(C)]
struct PaddedCounter {
    value: UnsafeCell<u64>,
    _padding: [u8; CACHE_LINE_SIZE - 8], // 64 - 8 = 56 bytes of padding
}

#[repr(C)]
struct PaddedCounters {
    counters: [PaddedCounter; NUM_THREADS],
}

unsafe impl Sync for PaddedCounters {}

impl PaddedCounter {
    fn new() -> Self {
        PaddedCounter {
            value: UnsafeCell::new(0),
            _padding: [0; CACHE_LINE_SIZE - 8],
        }
    }
}

fn main() {
    println!("=== False Sharing Benchmark ===");
    println!("Threads: {}", NUM_THREADS);
    println!("Iterations per thread: {}", ITERATIONS);
    println!("Cache line size: {} bytes", CACHE_LINE_SIZE);
    println!();

    // ============================================================
    // Benchmark 1: False Sharing (Bad)
    // ============================================================
    println!("--- Case 1: False Sharing (adjacent counters) ---");
    println!("Memory layout: [counter0|counter1|counter2|counter3] <- same cache line!");

    let shared = Arc::new(SharedCounters {
        counters: [
            UnsafeCell::new(0),
            UnsafeCell::new(0),
            UnsafeCell::new(0),
            UnsafeCell::new(0),
        ],
    });

    // Print each counter's address
    for i in 0..NUM_THREADS {
        println!("  Counter[{}] address: {:p}", i, shared.counters[i].get());
    }

    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..NUM_THREADS {
        let shared_clone = Arc::clone(&shared);
        let handle = thread::spawn(move || {
            let ptr = shared_clone.counters[thread_id].get();
            for _ in 0..ITERATIONS {
                unsafe {
                    // Each thread only increments its own counter
                    *ptr += 1;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
    let false_sharing_duration = start.elapsed();
    println!("Time: {:?}", false_sharing_duration);
    println!();

    // ============================================================
    // Benchmark 2: No False Sharing (Good)
    // ============================================================
    println!("--- Case 2: No False Sharing (padded counters) ---");
    println!("Memory layout: [counter0|padding...][counter1|padding...] <- separate cache lines!");

    let padded = Arc::new(PaddedCounters {
        counters: [
            PaddedCounter::new(),
            PaddedCounter::new(),
            PaddedCounter::new(),
            PaddedCounter::new(),
        ],
    });

    // Print each counter's address (should be 64 bytes apart)
    for i in 0..NUM_THREADS {
        println!("  Counter[{}] address: {:p}", i, padded.counters[i].value.get());
    }

    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..NUM_THREADS {
        let padded_clone = Arc::clone(&padded);
        let handle = thread::spawn(move || {
            let ptr = padded_clone.counters[thread_id].value.get();
            for _ in 0..ITERATIONS {
                unsafe {
                    *ptr += 1;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
    let no_false_sharing_duration = start.elapsed();
    println!("Time: {:?}", no_false_sharing_duration);
    println!();

    // ============================================================
    // Result
    // ============================================================
    println!("=== Result ===");
    let ratio = false_sharing_duration.as_nanos() as f64 / no_false_sharing_duration.as_nanos() as f64;

    if ratio > 1.0 {
        println!(
            "False Sharing version is {:.2}x SLOWER than padded version!",
            ratio
        );
    } else {
        println!(
            "Padded version is {:.2}x slower (unexpected result - try release build)",
            1.0 / ratio
        );
    }

    println!();
    println!("=== Explanation ===");
    println!("When False Sharing occurs:");
    println!("1. Thread0 updates counter[0] -> cache line is modified");
    println!("2. Thread1 tries to update counter[1]");
    println!("3. But since it's the same cache line, Thread1's cache is invalidated");
    println!("4. Thread1 must reload from memory (Cache Miss!)");
    println!("5. This happens simultaneously across 4 threads -> severe cache contention");
    println!();
    println!("Solution with padding:");
    println!("- Align each counter to 64-byte boundary");
    println!("- Each thread owns an independent cache line");
    println!("- No cache invalidation occurs -> Fast!");
}
