// Address visualization

fn main() {
    println!("=== Stack vs Heap Address Visualization ===\n");
    
// 1. output variable(i32, struct) address on stack
// Stack addresses are contiguous and grow in a specific direction (usually downwards).
    let a: i32 = 1;
    let b: i32 = 2;
    let c: i32 = 3;
    let d: i32 = 4;

    println!("--- Stack addresses (i32 variables) ---");
    println!("a: {:p}", &a);
    println!("b: {:p}", &b);
    println!("c: {:p}", &c);
    println!("d: {:p}", &d);

    // output address differences
    println!("\nAddress differences on stack:");
    println!("&b - &a = {} bytes", (&a as *const i32 as isize - &b as *const i32 as isize).abs());
    println!("&c - &b = {} bytes", (&b as *const i32 as isize - &c as *const i32 as isize).abs());


// 2. output variable after Box::new() <- heap allocation
// Heap addresses are often far apart or random-looking depending on the allocator.
    println!("\n--- Heap addresses (Box<i32>) ---");
    let box_a = Box::new(100);
    let box_b = Box::new(200);
    let box_c = Box::new(300);

    println!("box_a points to: {:p}", box_a.as_ref());
    println!("box_b points to: {:p}", box_b.as_ref());
    println!("box_c points to: {:p}", box_c.as_ref());

    // output Box variables themselves address
    println!("\nBox variables themselves (on stack):");
    println!("&box_a: {:p}", &box_a);
    println!("&box_b: {:p}", &box_b);
    println!("&box_c: {:p}", &box_c);


// 3. output Vec element
    println!("\n--- Vec elements (contiguous on heap) ---");
    let vec: Vec<i32> = vec![10, 20, 30, 40];

    for (i, elem) in vec.iter().enumerate() {
        println!("vec[{}]: value={}, address={:p}", i, elem, elem);
    }

    println!("\nVec elements are contiguous: difference = {} bytes (size of i32)",
        &vec[1] as *const i32 as isize - &vec[0] as *const i32 as isize);
}