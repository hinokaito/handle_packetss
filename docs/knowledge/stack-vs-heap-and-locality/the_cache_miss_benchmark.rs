// The "Cache Miss" Benchmark (Worst Case Edition)
// Intentionally sabotage the allocator to maximize cache misses.
//
// Strategy:
// - Insert garbage heap allocations between each LinkedList node
// - This forces nodes to be scattered across different cache lines
// - The CPU prefetcher cannot help, causing maximum cache misses

use std::collections::LinkedList;
use std::hint::black_box;
use std::time::Instant;

const SIZE: usize = 1_000_000;

// Padding size to push nodes apart (larger = more cache misses)
// 4KB is a common page size; crossing page boundaries is expensive
const PADDING_SIZE: usize = 4096;

fn main() {
    // =========================================================
    // 1. Vec<i32> - Contiguous memory, cache-friendly
    // =========================================================
    let vec: Vec<i32> = (0..SIZE as i32).collect();

    let start = Instant::now();
    let sum_vec: i64 = black_box(vec.iter().map(|&x| x as i64).sum());
    let vec_duration = start.elapsed();

    println!("=== Vec<i32> (contiguous memory) ===");
    println!("Sum: {}", sum_vec);
    println!("Time: {:?}", vec_duration);

    // =========================================================
    // 2. LinkedList<i32> - Sabotaged with padding allocations
    // =========================================================
    // We intentionally fragment memory by allocating garbage between nodes.
    // This simulates a worst-case scenario where nodes are scattered
    // across memory, causing cache misses on every access.

    let mut list: LinkedList<i32> = LinkedList::new();

    // Hold references to padding to prevent deallocation during iteration
    // If we drop them, the allocator might reuse that memory for nodes
    let mut padding_garbage: Vec<Box<[u8; PADDING_SIZE]>> = Vec::with_capacity(SIZE);

    println!("\nBuilding fragmented LinkedList (this may take a moment)...");

    for i in 0..SIZE as i32 {
        // Allocate garbage padding BEFORE the node
        // This pushes the next node's address further away
        let garbage = Box::new([0u8; PADDING_SIZE]);
        padding_garbage.push(garbage);

        // Now allocate the actual node - it will be far from the previous one
        list.push_back(i);

        // Prevent compiler from optimizing away our sabotage
        black_box(&padding_garbage);
    }

    // Prevent the padding from being optimized out entirely
    black_box(&padding_garbage);

    let start = Instant::now();
    let sum_list: i64 = black_box(list.iter().map(|&x| x as i64).sum());
    let list_duration = start.elapsed();

    println!("\n=== LinkedList<i32> (fragmented memory - worst case) ===");
    println!("Sum: {}", sum_list);
    println!("Time: {:?}", list_duration);
    println!("Padding per node: {} bytes", PADDING_SIZE);
    println!(
        "Total padding memory: {} MB",
        (SIZE * PADDING_SIZE) / (1024 * 1024)
    );

    // =========================================================
    // 3. Compare results
    // =========================================================
    println!("\n=== Result ===");
    let ratio = list_duration.as_nanos() as f64 / vec_duration.as_nanos() as f64;
    println!("LinkedList is {:.2}x slower than Vec", ratio);
    println!("\nThis demonstrates the true cost of cache misses!");
    println!("Each LinkedList node access likely causes a cache miss,");
    println!("while Vec benefits from prefetching contiguous memory.");

    // Keep padding alive until the end
    drop(padding_garbage);
}