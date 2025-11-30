# Stack vs Heap, Locality

## Overview of Memory Concepts
- Stack: Static memory allocation. LIFO. Very fast.
- Heap: Dynamic memory allocation. Flexible size but slower access via pointers.
- Data Locality: How close data elements are in memory.
- Cache Line: The unit of data transfer between memory and CPU cache (usually 64 bytes).


## Verification
### 1. Experiment: "address_visualization.rs"
Objective: / Key Findings:
- output variable(`i32`, `struct`) address on stack
- output variable after `Box::new()` <- heap allocation
- output Vec elements size

### 2. Experiment: "the_cache_miss_benchmark.rs"
Objective: / Key Findings:
- Loop `Vec<i32>` 1M times and calculate the total time.
- Loop `LinkedList<i32>` same times and calculate the total time.

### 3. Experiment: "false_sharing.rs"
Objective: / Key Findings:
- False Sharing: When threads access independent data on the same cache line
- Performance comparison: adjacent counters vs padded counters (64-byte aligned)
- Demonstrates how cache line contention can severely degrade multi-threaded performance

## Verification results
### 1. "address_visualization.rs"
```bash

--- Stack addresses (i32 variables) ---
a: 0x7ffcdf99e518
b: 0x7ffcdf99e51c
c: 0x7ffcdf99e520
d: 0x7ffcdf99e524

Address differences on stack:
&b - &a = 4 bytes
&c - &b = 4 bytes

--- Heap addresses (Box<i32>) ---
box_a points to: 0x625c61f65d00
box_b points to: 0x625c61f65d20
box_c points to: 0x625c61f65d40

Box variables themselves (on stack):
&box_a: 0x7ffcdf99e7c8
&box_b: 0x7ffcdf99e7d0
&box_c: 0x7ffcdf99e7d8

--- Vec elements (contiguous on heap) ---
vec[0]: value=10, address=0x625c61f65d60
vec[1]: value=20, address=0x625c61f65d64
vec[2]: value=30, address=0x625c61f65d68
vec[3]: value=40, address=0x625c61f65d6c

Vec elements are contiguous: difference = 4 bytes (size of i32)
```

### 2. "the_cache_miss_benchmark.rs"
```bash
=== Vec<i32> (contiguous memory) ===
Sum: 499999500000
Time: 281.9µs

Building fragmented LinkedList (this may take a moment)...

=== LinkedList<i32> (fragmented memory - worst case) ===
Sum: 499999500000
Time: 8.305ms
Padding per node: 4096 bytes
Total padding memory: 3906 MB

=== Result ===
LinkedList is 29.46x slower than Vec

This demonstrates the true cost of cache misses!
Each LinkedList node access likely causes a cache miss,
while Vec benefits from prefetching contiguous memory.
```

### 3. "false_sharing.rs"
```bash
=== False Sharing Benchmark ===
Threads: 4
Iterations per thread: 100000000
Cache line size: 64 bytes

--- Case 1: False Sharing (adjacent counters) ---
Memory layout: [counter0|counter1|counter2|counter3] <- same cache line!
  Counter[0] address: 0x60c67a6b0d10
  Counter[1] address: 0x60c67a6b0d18
  Counter[2] address: 0x60c67a6b0d20
  Counter[3] address: 0x60c67a6b0d28
Time: 4.90087731s

--- Case 2: No False Sharing (padded counters) ---
Memory layout: [counter0|padding...][counter1|padding...] <- separate cache lines!
  Counter[0] address: 0x60c67a6b1690
  Counter[1] address: 0x60c67a6b16d0
  Counter[2] address: 0x60c67a6b1710
  Counter[3] address: 0x60c67a6b1750
Time: 3.855945329s

=== Result ===
False Sharing version is 1.27x SLOWER than padded version!

=== Explanation ===
When False Sharing occurs:
1. Thread0 updates counter[0] -> cache line is modified
2. Thread1 tries to update counter[1]
3. But since it's the same cache line, Thread1's cache is invalidated
4. Thread1 must reload from memory (Cache Miss!)
5. This happens simultaneously across 4 threads -> severe cache contention

Solution with padding:
- Align each counter to 64-byte boundary
- Each thread owns an independent cache line
- No cache invalidation occurs -> Fast!
```


## Why these results? like 2,3
### 2. "the_cache_miss_benchmark.rs" result
Spatial Locality: The CPU reads data from memory in units of “Cache Lines (64 bytes)”. If it's an array, multiple i32s can fit into the cache with a single read. However, with scattered pointers, each access triggers a “Cache Miss” requiring a trip to memory, causing the CPU to stall.

### 3. "false_sharing.rs"
Although the data itself is independent, the phenomenon known as cache coherence traffic occurred, causing cache lines to compete and resulting in a drastic drop in performance.


## Application to "handle_packetss"
If many packets aren't contiguous? Performance degradation is inevitable.
Proper memory design maximizes the user experience.