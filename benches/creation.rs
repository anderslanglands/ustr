#[macro_use]
extern crate criterion;
use criterion::black_box;
use criterion::Criterion;
use crossbeam_channel::bounded;
use crossbeam_utils::thread::scope;
use std::sync::Arc;
use string_cache::DefaultAtom;
use string_interner::StringInterner;

use ustring::*;

fn create_ustrings(blns: &Vec<String>, num: usize) {
    for s in blns.iter().cycle().take(num) {
        black_box(u!(s));
    }
}

fn create_string_interner<S: string_interner::Symbol>(
    interner: &mut StringInterner<S>,
    blns: &Vec<String>,
    num: usize,
) {
    for s in blns.iter().cycle().take(num) {
        black_box(interner.get_or_intern(s));
    }
}

fn create_string_cache(blns: &Vec<String>, num: usize) {
    for s in blns.iter().cycle().take(num) {
        black_box(DefaultAtom::from(s.as_str()));
    }
}

fn create_strings(blns: &Vec<String>, num: usize) {
    for s in blns.iter().cycle().take(num) {
        black_box(String::from(s));
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("data")
        .join("blns.txt");
    let blns = std::fs::read_to_string(path).unwrap();
    let blns = Arc::new(
        blns.split_whitespace()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>(),
    );

    // there are 1315 unique tokens in blns.txt, so this will find an already-existing
    // string ~7.6 times for every string created
    // ~14ns
    let s = blns.clone();
    c.bench_function("create 10k", move |b| {
        let s = s.clone();
        b.iter(|| {
            _clear_cache();
            create_ustrings(&(*s), 10_000);
        });
    });

    // ~14ns
    let s = blns.clone();
    c.bench_function("create 10k no clear", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_ustrings(&(*s), 10_000);
        });
    });

    // ~15ns
    let s = blns.clone();
    c.bench_function("String::from", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_strings(&(*s), 10_000);
        });
    });

    // ~28ns
    let s = blns.clone();
    c.bench_function("string-interner create 10k", move |b| {
        let s = s.clone();
        let mut interner = StringInterner::default();
        b.iter(|| {
            create_string_interner(&mut interner, &(*s), 10_000);
        });
    });

    // ~28ns
    let s = blns.clone();
    let mut interner = StringInterner::default();
    c.bench_function("string-interner create 10k one interner", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_string_interner(&mut interner, &(*s), 10_000);
        });
    });

    // ~55ns
    let s = blns.clone();
    c.bench_function("string-cache create 10k", move |b| {
        let s = s.clone();
        b.iter(|| {
            create_string_cache(&(*s), 10_000);
        });
    });

    // test lookups.
    // 1) First pass gives ~1ns for the lookup
    // 2) Switching to custom hash table gives ~2ns per lookup?
    // 3) With allocator gets us back to ~1ns
    let ustrings: Vec<UString> = blns.iter().map(|s| u!(s)).collect();
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

    let s = blns.clone();
    let num_threads = 2;
    let num = 10_000;
    c.bench_function("create 10k 2 threads", move |b| {
        let (tx1, rx1) = bounded(0);
        let (tx2, rx2) = bounded(0);
        scope(|scope| {
            for _ in 0..num_threads {
                scope.spawn(|_| {
                    while rx1.recv().is_ok() {
                        for s in s.iter().cycle().take(num) {
                            black_box(u!(s));
                        }
                        tx2.send(()).unwrap();
                    }
                });
            }

            b.iter(|| {
                for _ in 0..num_threads {
                    tx1.send(()).unwrap();
                }

                for _ in 0..num_threads {
                    rx2.recv().unwrap();
                }
            });
            drop(tx1);
        })
        .unwrap();
    });

    let s = blns.clone();
    c.bench_function("create 10k 2 threads string interner", move |b| {
        let (tx1, rx1) = bounded(0);
        let (tx2, rx2) = bounded(0);
        let interner = spin::Mutex::new(StringInterner::default());
        scope(|scope| {
            for _ in 0..num_threads {
                scope.spawn(|_| {
                    while rx1.recv().is_ok() {
                        for s in s.iter().cycle().take(num) {
                            let mut int = interner.lock();
                            black_box(int.get_or_intern(s));
                        }
                        tx2.send(()).unwrap();
                    }
                });
            }

            b.iter(|| {
                for _ in 0..num_threads {
                    tx1.send(()).unwrap();
                }

                for _ in 0..num_threads {
                    rx2.recv().unwrap();
                }
            });
            drop(tx1);
        })
        .unwrap();
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
