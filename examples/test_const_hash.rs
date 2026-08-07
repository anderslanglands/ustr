// Test that const hashing works correctly
use ustr::{hash::string_hash, static_ustr, ustr};

fn main() {
    println!("=== Testing Const Hash Implementation ===\n");

    // These hashes are computed at compile time
    const HELLO_HASH: u64 = string_hash(b"hello");
    const WORLD_HASH: u64 = string_hash(b"world");

    println!("Compile-time hashes:");
    println!("  'hello': 0x{:016x}", HELLO_HASH);
    println!("  'world': 0x{:016x}", WORLD_HASH);

    // Runtime hashes using the same function
    let runtime_hello = string_hash(b"hello");
    let runtime_world = string_hash(b"world");

    println!("\nRuntime hashes (should match):");
    println!("  'hello': 0x{:016x}", runtime_hello);
    println!("  'world': 0x{:016x}", runtime_world);

    assert_eq!(HELLO_HASH, runtime_hello, "Hello hash mismatch!");
    assert_eq!(WORLD_HASH, runtime_world, "World hash mismatch!");
    println!("\n✓ Compile-time and runtime hashes match!");

    // Test the static_ustr! macro
    println!("\n=== Testing static_ustr! macro ===\n");

    // Clear cache for testing
    unsafe { ustr::_clear_cache() };

    // Create strings using static_ustr! (optimized for literals)
    let s1 = static_ustr!("compile-time-optimized");
    let s2 = static_ustr!("another-literal");

    // Create strings using regular ustr
    let d1 = ustr("runtime-string");
    let d2 = ustr("compile-time-optimized"); // Should find s1 in cache

    println!("Static strings created: {:?}, {:?}", s1, s2);
    println!("Dynamic strings created: {:?}, {:?}", d1, d2);

    assert_eq!(s1, d2, "Same string should be equal!");
    println!("\n✓ static_ustr! and ustr produce compatible Ustrs!");

    // Test that the macro works with variables too
    let var = "dynamic";
    let s3 = static_ustr!(var); // Should fall back to regular ustr
    let d3 = ustr(var);
    assert_eq!(s3, d3);
    println!("✓ static_ustr! handles variables correctly!");

    println!("\n=== All tests passed! ===");
    println!("\nCache stats:");
    println!("  Total entries: {}", ustr::num_entries());
    println!("  Total allocated: {} bytes", ustr::total_allocated());
}
