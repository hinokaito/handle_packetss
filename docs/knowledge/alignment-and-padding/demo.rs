// docs/alignment-and-padding/demo.rs
use std::mem;

// NOTE: i128 requires 16-byte alignment on many 64-bit systems.
// Total alignment of the struct becomes 16 bytes.

#[repr(C)]
struct BadLayout {
    a: u8,   // 1 byte
             // +7 bytes padding (align to 8 for u64)
    b: u64,  // 8 bytes
    c: u8,   // 1 byte
             // +15 bytes padding (align to 16 for i128)
    d: i128, // 16 bytes
} // Total: 48 bytes

#[repr(C)]
struct GoodLayout {
    d: i128, // 16 bytes
    b: u64,  // 8 bytes
    a: u8,   // 1 byte
    c: u8,   // 1 byte
             // +6 bytes padding (to reach multiple of 16)
} // Total: 32 bytes

// Rust default representation allows compiler reordering
struct OptimizedBadLayout {
    a: u8,
    b: u64,
    c: u8,
    d: i128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_memory_layout_size() {
        // repr(C) - Layout follows declaration order strictly
        assert_eq!(mem::size_of::<BadLayout>(), 48, "BadLayout should include heavy padding");
        assert_eq!(mem::size_of::<GoodLayout>(), 32, "GoodLayout should minimize padding");

        // Rust default - Compiler reorders fields for optimization
        assert_eq!(mem::size_of::<OptimizedBadLayout>(), 32, "Rust compiler should optimize field order automatically");
    }

    #[test]
    fn verify_alignment() {
        assert_eq!(mem::align_of::<BadLayout>(), 16);
    }
}

fn main() {
    println!("Running alignment demonstration...");
    println!("BadLayout (repr(C)): {} bytes", mem::size_of::<BadLayout>());
    println!("GoodLayout (repr(C)): {} bytes", mem::size_of::<GoodLayout>());
    println!("Optimized (Rust):    {} bytes", mem::size_of::<OptimizedBadLayout>());
}