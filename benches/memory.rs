#[macro_use]
extern crate criterion;
use criterion::Criterion;
use std::hint::black_box;
use ustr::*;

fn criterion_benchmark(c: &mut Criterion) {
    // Test memory usage patterns

    // Benchmark cache behavior with many unique strings
    c.bench_function("cache_many_unique", |b| {
        b.iter(|| {
            unsafe { ustr::_clear_cache() };
            for i in 0..10_000 {
                black_box(ustr(&format!("unique_string_{}", i)));
            }
        });
    });

    // Benchmark cache behavior with repeated strings
    c.bench_function("cache_repeated_strings", |b| {
        b.iter(|| {
            unsafe { ustr::_clear_cache() };
            let test_strings =
                vec!["hello", "world", "rust", "benchmark", "test"];
            for _ in 0..10_000 {
                for &s in &test_strings {
                    black_box(ustr(s));
                }
            }
        });
    });

    // Benchmark memory fragmentation with different string sizes
    c.bench_function("mixed_string_sizes", |b| {
        b.iter(|| {
            unsafe { ustr::_clear_cache() };

            // Short strings
            for i in 0..1000 {
                black_box(ustr(&format!("s{}", i)));
            }

            // Medium strings
            for i in 0..500 {
                black_box(ustr(&format!("medium_length_string_{}", i)));
            }

            // Long strings
            for i in 0..100 {
                black_box(ustr(&format!(
                    "this_is_a_very_long_string_that_tests_memory_allocation_patterns_{}",
                    i
                )));
            }
        });
    });

    // Benchmark cache access patterns
    let _pre_allocated: Vec<Ustr> = (0..1000)
        .map(|i| ustr(&format!("cached_string_{}", i)))
        .collect();

    c.bench_function("cache_access_existing", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(existing_ustr(&format!("cached_string_{}", i)));
            }
        });
    });

    // Benchmark cache miss pattern
    c.bench_function("cache_miss_pattern", |b| {
        b.iter(|| {
            for i in 10000..11000 {
                black_box(existing_ustr(&format!("non_existent_string_{}", i)));
            }
        });
    });

    // Benchmark allocation overhead
    c.bench_function("allocation_overhead", |b| {
        b.iter(|| {
            unsafe { ustr::_clear_cache() };
            // Create strings that will cause different allocation patterns
            for i in 0..100 {
                // Empty string
                black_box(ustr(""));
                // Single char
                black_box(ustr(&format!(
                    "{}",
                    (b'a' + (i % 26) as u8) as char
                )));
                // Power of 2 lengths
                black_box(ustr(&"x".repeat(1 << (i % 8))));
                // Prime lengths
                let primes = [3, 5, 7, 11, 13, 17, 19, 23];
                black_box(ustr(&"y".repeat(primes[i % primes.len()])));
            }
        });
    });

    // Benchmark with unicode strings
    let unicode_strings = vec![
        "Hello, 世界",
        "Γειά σου κόσμε",
        "Привіт, світ",
        "مرحبا بالعالم",
        "🦀 Rust 🚀",
        "τὴν γλῶσσάν μου ἔδωσαν ἑλληνικήν",
        "Καλησπέρα",
        "صباح الخير",
    ];

    c.bench_function("unicode_strings", |b| {
        b.iter(|| {
            unsafe { ustr::_clear_cache() };
            for &s in &unicode_strings {
                for _ in 0..100 {
                    black_box(ustr(s));
                }
            }
        });
    });

    // Benchmark precomputed hash usage
    let hash_test_strings: Vec<Ustr> = (0..1000)
        .map(|i| ustr(&format!("hash_test_{}", i)))
        .collect();

    c.bench_function("precomputed_hash_access", |b| {
        b.iter(|| {
            for &ustr in &hash_test_strings {
                black_box(ustr.precomputed_hash());
            }
        });
    });

    // Benchmark string interning patterns with duplicates
    c.bench_function("interning_with_duplicates", |b| {
        b.iter(|| {
            unsafe { ustr::_clear_cache() };

            // Create patterns that simulate real-world usage
            let patterns = [
                "user_id",
                "session_token",
                "api_key",
                "database_connection",
                "cache_key",
            ];

            // Each pattern repeated many times with slight variations
            for pattern in &patterns {
                for i in 0..200 {
                    black_box(ustr(&format!("{}_{}", pattern, i % 10))); // Only
                    // 10 unique
                    // per pattern
                }
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
