pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
// 実験用コード
use std::mem;

#[repr(C)]
struct BadLayout {
    a: u8,   // 1 byte
    b: u64,  // 8 bytes
    c: u8,   // 1 byte
    d: i128,
}

#[repr(C)]
struct GoodLayout {
    d: i128,
    b: u64,  // 8 bytes
    a: u8,   // 1 byte
    c: u8,   // 1 byte
}


struct OptimizedBadLayout {
    a: u8,   // 1 byte
    b: u64,  // 8 bytes
    c: u8,   // 1 byte
    d: i128,
}

struct OptimizedGoodLayout {
    d: i128,
    b: u64,  // 8 bytes
    a: u8,   // 1 byte
    c: u8,   // 1 byte
}

fn main() {
    println!("Bad: {} bytes", mem::size_of::<BadLayout>());
    println!("Good: {} bytes", mem::size_of::<GoodLayout>());
    println!("OptimizedBad: {} bytes", mem::size_of::<OptimizedBadLayout>());
    println!("OptimizedGood: {} bytes", mem::size_of::<OptimizedGoodLayout>());
}