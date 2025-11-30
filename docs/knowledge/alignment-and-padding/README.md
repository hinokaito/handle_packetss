# Memory Alignment and Padding Strategies

## Basic Concepts
**Memory alignment** is a constraint that places data at memory addresses that are multiples of its size (or a specific word size).

### Why It Matters (Performance and Cost)
CPUs access memory in word-sized chunks (e.g., 8 bytes on 64-bit systems). Unaligned access causes CPU overhead (fetching 2 words instead of 1) or crashes on some architectures (ARM/SPARC, etc.).

**Impact on Large-Scale Systems:**
For a backend system processing 100 million records in memory:
- `BadLayout` (48 bytes) × 100M = 4.8 GB
- `GoodLayout` (32 bytes) × 100M = 3.2 GB
=> Field reordering alone **saves 1.6 GB of RAM**. This directly impacts cloud infrastructure costs.

## Mental Model: The Bookshelf Analogy
Imagine a bookshelf designed for books of specific widths (2cm, 4cm).
If you place a 4cm book starting at a 3cm offset, it spans two "slots." To read it, you'd need to look at two slots and mentally combine them.
The CPU (the librarian) prefers books aligned to slot boundaries for instant access. "Padding" is empty spacers inserted to enforce this rule.

## Visualizing Padding
Example: `u8` (1 byte) followed by `u64` (8 bytes).

```rust
struct MyData {
    a: u8,
    b: u64,
}
```

**Actual Layout (16 bytes total):**

```
[a][.][.][.][.][.][.][.] [b][b][b][b][b][b][b][b]
 ^ a (1 byte)             ^ b (8 bytes, aligned to 8)
    ^ padding (7 bytes)
```

## Alignment Rules Quick Reference

| Type | Size | Typical Alignment (x64) | Valid Offsets |
|------|------|-------------------------|---------------|
| u8 / i8 | 1 byte | 1 byte | Any |
| u16 / i16 | 2 bytes | 2 bytes | 0, 2, 4... |
| u32 / f32 | 4 bytes | 4 bytes | 0, 4, 8... |
| u64 / f64 | 8 bytes | 8 bytes | 0, 8, 16... |
| u128 | 16 bytes | 16 bytes | 0, 16, 32... |

## Practical Guidelines

### Let the Rust Compiler Handle It (Default)

The Rust compiler automatically reorders fields to minimize size.

**In other words: Do nothing by default. Trust the compiler.**

### Manual Layout Control (#[repr(C)])

Manual layout control is required in the following scenarios:

- **FFI (Foreign Function Interface):** When passing structs to C/C++ libraries.
- **Network Protocols:** When directly mapping raw packet bytes to structs (e.g., parsing TCP/QUIC headers).
- **Hardware Drivers:** When mapping structs to specific memory-mapped registers.
- **Wasm Shared Memory:** When sharing data layouts between Rust and JavaScript/browsers.

In these cases, order fields by size (largest first) to minimize padding.




