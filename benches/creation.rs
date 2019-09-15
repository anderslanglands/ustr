#[macro_use]
extern crate criterion;
use criterion::black_box;
use criterion::Criterion;
use std::sync::Arc;
use string_cache::DefaultAtom;
use string_interner::StringInterner;

use ustring::*;

fn create_ustrings(blns: &String, num: usize) {
    for s in blns.split_whitespace().cycle().take(num) {
        black_box(u!(s));
    }
}

fn create_string_interner<S: string_interner::Symbol>(
    interner: &mut StringInterner<S>,
    blns: &String,
    num: usize,
) {
    for s in blns.split_whitespace().cycle().take(num) {
        black_box(interner.get_or_intern(s));
    }
}

fn create_string_cache(blns: &String, num: usize) {
    for s in blns.split_whitespace().cycle().take(num) {
        black_box(DefaultAtom::from(s));
    }
}

fn create_strings(blns: &String, num: usize) {
    for s in blns.split_whitespace().cycle().take(num) {
        black_box(String::from(s));
    }
}

fn split_whitespace(blns: &String, num: usize) {
    for s in blns.split_whitespace().cycle().take(num) {
        black_box(s);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("data")
        .join("blns.txt");
    let blns = Arc::new(std::fs::read_to_string(path).unwrap());

    // there are 1315 unique tokens in blns.txt, so this will find an already-existing
    // string ~7.6 times for every string created
    // 1) First pass with a HashMap gives ~88ns per creation
    // 2) Switching to custom hash table gives ~55ns per creation (std Mutex gives ~60ns)
    // 3) City hash gets us ~36ns
    // 4) Bump allocator gets us ~34ns
    // 5) 65% of that time is doing the string split so it's more like 12ns
    let s = blns.clone();
    c.bench_function("create 10k", move |b| {
        let s = s.clone();
        b.iter(|| {
            _clear_cache();
            create_ustrings(&s, 10_000);
        });
    });

    let s = blns.clone();
    c.bench_function("create 10k no clear", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_ustrings(&s, 10_000);
        });
    });

    let s = blns.clone();
    c.bench_function("String::from", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_strings(&s, 10_000);
        });
    });

    let s = blns.clone();
    c.bench_function("split_whitespace (control)", move |b| {
        let s = s.clone();
        b.iter(|| {
            split_whitespace(&s, 10_000);
        });
    });

    let s = blns.clone();
    c.bench_function("string-interner create 10k", move |b| {
        let s = s.clone();
        let mut interner = StringInterner::default();
        b.iter(|| {
            create_string_interner(&mut interner, &s, 10_000);
        });
    });

    let s = blns.clone();
    c.bench_function("string-cache create 10k", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_string_cache(&s, 10_000);
        });
    });

    // test lookups.
    // 1) First pass gives ~1ns for the lookup
    // 2) Switching to custom hash table gives ~2ns per lookup?
    // 3) With allocator gets us back to ~1ns
    let ustrings: Vec<UString> = blns.split_whitespace().map(|s| u!(s)).collect();
    c.bench_function("lookup", move |b| {
        let us = &ustrings;
        b.iter(|| {
            for u in us {
                black_box({
                    u.as_str();
                })
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
