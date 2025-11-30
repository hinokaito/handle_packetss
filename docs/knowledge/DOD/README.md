# Data Oriented Design: AoS vs SoA

## Overview of Memory Layout Concepts
- **Array of Structs (AoS)**: Traditional OOP-style. Each entity is a complete object stored contiguously.
- **Struct of Arrays (SoA)**: DOD-style. Each field type is stored in its own contiguous array.
- **Cache Efficiency**: SoA allows the CPU to load only the data it needs, maximizing cache line utilization.

## Mental Model: The Filing Cabinet Analogy
Imagine you need to update the salary of 1000 employees.

**AoS (Traditional OOP):**
Each drawer contains one employee's complete file (name, address, salary, photo...).
To update salaries, you open 1000 drawers and dig through irrelevant papers in each.

**SoA (Data Oriented):**
One drawer contains ALL salaries. One drawer contains ALL names. etc.
To update salaries, you open ONE drawer and process everything in sequence.

## Visualizing Memory Access

```
AoS memory layout (update_positions):
┌──────────────────────────────────────────────────────────┐
│ pos[0] │ vel[0] │ health │ id │ pos[1] │ vel[1] │ ... │
└──────────────────────────────────────────────────────────┘
  ↑used    ↑used    ↑waste   ↑waste  ↑used ...
  
Cache line efficiency: ~37% (only position and velocity are needed)

SoA memory layout (update_positions):
┌─────────────────────────────────────┐
│ pos[0] │ pos[1] │ pos[2] │ pos[3] │ ...  ← 100% used!
└─────────────────────────────────────┘
┌─────────────────────────────────────┐
│ vel[0] │ vel[1] │ vel[2] │ vel[3] │ ...  ← 100% used!
└─────────────────────────────────────┘

Cache line efficiency: 100%
```

## When to Use Each

| Scenario | Recommended | Reason |
|----------|-------------|--------|
| Processing few fields across many entities | **SoA** | Cache-friendly, sequential access |
| Processing all fields of single entities | **AoS** | All data in 1-2 cache lines |
| Frequent add/remove of entities | **AoS** | SoA requires syncing multiple arrays |
| Read-only bulk processing | **SoA** | Prefetcher works optimally |
| Team collaboration / maintainability | **AoS** | More intuitive, less error-prone |
| Performance-critical hot paths | **Measure first** | Profile before optimizing |

## Trade-offs of SoA

1. **Index synchronization**: Removing entity N requires removing index N from ALL arrays
2. **Debugging difficulty**: Entity data is scattered across multiple arrays
3. **Single-entity access overhead**: Accessing all fields of one entity causes multiple cache misses
4. **Code complexity**: More boilerplate for CRUD operations


## Verification: "aos_vs_soa_benchmark.rs"
Objective / Key Findings:
- Compare position update performance between AoS and SoA layouts
- 1 million enemies, 100 iterations each
- Only `position` and `velocity` fields are accessed (simulating partial field access)

## Verification Results
```bash
AoS: 590.3671ms
SoA: 212.6525ms

=== Result ===
AoS / SoA = 2.78x
SoA is 2.78 times faster than AoS
```

## Why These Results?
The benchmark processes only `position` and `velocity` fields:

**AoS**: Each `Enemy` struct is ~40 bytes. Processing position+velocity (24 bytes) wastes ~40% of each cache line fetch on `health` and `id` fields that are never used.

**SoA**: The `positions` and `velocities` arrays are stored contiguously. Every byte fetched into cache is actually used. The CPU prefetcher can predict sequential access patterns perfectly.


## Application to "handle_packetss"
Network packet processing often involves:
- Filtering packets by a single field (e.g., destination port)
- Bulk updates to packet metadata
- High-throughput scenarios with millions of packets

SoA layout for packet headers could significantly improve filtering and routing performance where only specific fields are examined.
