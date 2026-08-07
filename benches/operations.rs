#[macro_use]
extern crate criterion;
use criterion::Criterion;
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use ustr::*;

fn criterion_benchmark(c: &mut Criterion) {
    // Test data
    let test_strings = vec![
        "hello",
        "world",
        "rust",
        "programming",
        "benchmark",
        "performance",
        "string",
        "interning",
        "cache",
        "optimization",
        "the quick brown fox jumps over the lazy dog",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit",
        "supercalifragilisticexpialidocious",
        "antidisestablishmentarianism",
        "",
        "a",
        "ab",
        "abc",
        "abcd",
        "abcde",
    ];

    // Create Ustr instances
    let ustrs: Vec<Ustr> = test_strings.iter().map(|&s| ustr(s)).collect();

    // Benchmark Ustr creation
    c.bench_function("ustr_creation", |b| {
        b.iter(|| {
            for &s in &test_strings {
                black_box(ustr(s));
            }
        });
    });

    // Benchmark String creation for comparison
    c.bench_function("string_creation", |b| {
        b.iter(|| {
            for &s in &test_strings {
                black_box(String::from(s));
            }
        });
    });

    // Benchmark Ustr equality comparison
    c.bench_function("ustr_equality", |b| {
        b.iter(|| {
            for i in 0..ustrs.len() {
                for j in 0..ustrs.len() {
                    black_box(ustrs[i] == ustrs[j]);
                }
            }
        });
    });

    // Benchmark String equality comparison
    let strings: Vec<String> =
        test_strings.iter().map(|&s| String::from(s)).collect();
    c.bench_function("string_equality", |b| {
        b.iter(|| {
            for i in 0..strings.len() {
                for j in 0..strings.len() {
                    black_box(strings[i] == strings[j]);
                }
            }
        });
    });

    // Benchmark Ustr hashing
    c.bench_function("ustr_hashing", |b| {
        b.iter(|| {
            for ustr in &ustrs {
                black_box(ustr.precomputed_hash());
            }
        });
    });

    // Benchmark String hashing
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    c.bench_function("string_hashing", |b| {
        b.iter(|| {
            for s in &strings {
                let mut hasher = DefaultHasher::new();
                s.hash(&mut hasher);
                black_box(hasher.finish());
            }
        });
    });

    // Benchmark Ustr as HashMap key
    c.bench_function("ustr_hashmap_insert", |b| {
        b.iter(|| {
            let mut map: UstrMap<usize> = UstrMap::default();
            for (i, &ustr) in ustrs.iter().enumerate() {
                map.insert(ustr, i);
            }
            black_box(map);
        });
    });

    // Benchmark String as HashMap key
    c.bench_function("string_hashmap_insert", |b| {
        b.iter(|| {
            let mut map: HashMap<String, usize> = HashMap::new();
            for (i, s) in strings.iter().enumerate() {
                map.insert(s.clone(), i);
            }
            black_box(map);
        });
    });

    // Benchmark Ustr HashMap lookup
    let mut ustr_map: UstrMap<usize> = UstrMap::default();
    for (i, &ustr) in ustrs.iter().enumerate() {
        ustr_map.insert(ustr, i);
    }

    c.bench_function("ustr_hashmap_lookup", |b| {
        b.iter(|| {
            for &ustr in &ustrs {
                black_box(ustr_map.get(&ustr));
            }
        });
    });

    // Benchmark String HashMap lookup
    let mut string_map: HashMap<String, usize> = HashMap::new();
    for (i, s) in strings.iter().enumerate() {
        string_map.insert(s.clone(), i);
    }

    c.bench_function("string_hashmap_lookup", |b| {
        b.iter(|| {
            for s in &strings {
                black_box(string_map.get(s));
            }
        });
    });

    // Benchmark Ustr copying
    c.bench_function("ustr_copy", |b| {
        b.iter(|| {
            for &ustr in &ustrs {
                black_box(ustr);
            }
        });
    });

    // Benchmark String cloning
    c.bench_function("string_clone", |b| {
        b.iter(|| {
            for s in &strings {
                black_box(s.clone());
            }
        });
    });

    // Benchmark Ustr length access
    c.bench_function("ustr_len", |b| {
        b.iter(|| {
            for &ustr in &ustrs {
                black_box(ustr.len());
            }
        });
    });

    // Benchmark String length access
    c.bench_function("string_len", |b| {
        b.iter(|| {
            for s in &strings {
                black_box(s.len());
            }
        });
    });

    // Benchmark Ustr as_str conversion
    c.bench_function("ustr_as_str", |b| {
        b.iter(|| {
            for &ustr in &ustrs {
                black_box(ustr.as_str());
            }
        });
    });

    // Benchmark existing_ustr lookup
    c.bench_function("existing_ustr_lookup", |b| {
        b.iter(|| {
            for &s in &test_strings {
                black_box(existing_ustr(s));
            }
        });
    });

    // Benchmark Ustr in HashSet
    c.bench_function("ustr_hashset_insert", |b| {
        b.iter(|| {
            let mut set: UstrSet = UstrSet::default();
            for &ustr in &ustrs {
                set.insert(ustr);
            }
            black_box(set);
        });
    });

    // Benchmark String in HashSet
    c.bench_function("string_hashset_insert", |b| {
        b.iter(|| {
            let mut set: HashSet<String> = HashSet::new();
            for s in &strings {
                set.insert(s.clone());
            }
            black_box(set);
        });
    });

    // Benchmark Ustr ordering
    c.bench_function("ustr_ordering", |b| {
        b.iter(|| {
            let mut sorted = ustrs.clone();
            sorted.sort();
            black_box(sorted);
        });
    });

    // Benchmark String ordering
    c.bench_function("string_ordering", |b| {
        b.iter(|| {
            let mut sorted = strings.clone();
            sorted.sort();
            black_box(sorted);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
