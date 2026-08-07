// Example demonstrating const evaluation possibilities with stable Rust
// Run with: cargo run --example const_example

// Demonstrates what const evaluation can do in current Rust
const fn const_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

// Compile-time string wrapper
#[derive(Debug)]
struct ConstStr {
    hash: u64,
    data: &'static str,
}

impl ConstStr {
    const fn new(s: &'static str) -> Self {
        Self {
            hash: const_hash(s.as_bytes()),
            data: s,
        }
    }
}

// Create compile-time constants
const HELLO: ConstStr = ConstStr::new("hello");
const WORLD: ConstStr = ConstStr::new("world");
const FOO: ConstStr = ConstStr::new("foo");

// Compile-time string pool using const generics
struct StringPool<const N: usize> {
    strings: [ConstStr; N],
}

impl<const N: usize> StringPool<N> {
    const fn new(strings: [ConstStr; N]) -> Self {
        Self { strings }
    }

    const fn get(&self, index: usize) -> Option<&ConstStr> {
        if index < N {
            Some(&self.strings[index])
        } else {
            None
        }
    }
}

// Create a compile-time string pool
const POOL: StringPool<3> = StringPool::new([HELLO, WORLD, FOO]);

// With Rust 2024 improvements, we can do more complex const operations
const fn build_string_table() -> [(u64, &'static str); 3] {
    [
        (const_hash(b"one"), "one"),
        (const_hash(b"two"), "two"),
        (const_hash(b"three"), "three"),
    ]
}

const STRING_TABLE: [(u64, &str); 3] = build_string_table();

fn main() {
    println!("=== Const String Evaluation Demo ===\n");

    println!("Compile-time hashed strings:");
    println!("  HELLO: {:?} (hash: 0x{:016x})", HELLO.data, HELLO.hash);
    println!("  WORLD: {:?} (hash: 0x{:016x})", WORLD.data, WORLD.hash);
    println!("  FOO:   {:?} (hash: 0x{:016x})", FOO.data, FOO.hash);

    println!("\nCompile-time string pool:");
    for i in 0..4 {
        match POOL.get(i) {
            Some(s) => {
                println!("  [{}]: {:?} (hash: 0x{:016x})", i, s.data, s.hash)
            }
            None => println!("  [{}]: <out of bounds>", i),
        }
    }

    println!("\nCompile-time string table:");
    for (hash, s) in &STRING_TABLE {
        println!("  0x{:016x} => {:?}", hash, s);
    }

    // These are all evaluated at compile time!
    const COMPILE_TIME_HASH: u64 = const_hash(b"compile-time");
    println!(
        "\nHash of 'compile-time' computed at compile time: 0x{:016x}",
        COMPILE_TIME_HASH
    );

    // Verify it matches runtime computation
    let runtime_hash = const_hash(b"compile-time");
    assert_eq!(COMPILE_TIME_HASH, runtime_hash);
    println!("Runtime hash matches: ✓");
}
