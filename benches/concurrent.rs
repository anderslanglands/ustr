#[macro_use]
extern crate criterion;
use criterion::Criterion;
use crossbeam_channel::bounded;
use crossbeam_utils::thread::scope;
use std::hint::black_box;
use std::sync::Arc;
use ustr::*;

fn criterion_benchmark(c: &mut Criterion) {
    // Concurrent access patterns
    let test_data = Arc::new(
        (0..1000)
            .map(|i| format!("concurrent_test_string_{}", i))
            .collect::<Vec<_>>(),
    );

    // Single-threaded baseline
    c.bench_function("concurrent_baseline_single", |b| {
        let data = Arc::clone(&test_data);
        b.iter(|| {
            unsafe { ustr::_clear_cache() };
            for s in data.iter() {
                black_box(ustr(s));
            }
        });
    });

    // Test concurrent creation with different thread counts
    for num_threads in [2, 4, 8, 16].iter() {
        let num_threads = *num_threads;
        let data = Arc::clone(&test_data);

        c.bench_function(
            &format!("concurrent_creation_{}_threads", num_threads),
            move |b| {
                let (tx, rx) = bounded(0);
                let (done_tx, done_rx) = bounded(0);

                scope(|scope| {
                    // Spawn worker threads
                    for thread_id in 0..num_threads {
                        let rx = rx.clone();
                        let done_tx = done_tx.clone();
                        let data = Arc::clone(&data);

                        scope.spawn(move |_| {
                            while rx.recv().is_ok() {
                                // Each thread works on a different subset
                                let start =
                                    (thread_id * data.len()) / num_threads;
                                let end = ((thread_id + 1) * data.len())
                                    / num_threads;

                                for s in &data[start..end] {
                                    black_box(ustr(s));
                                }

                                done_tx.send(()).unwrap();
                            }
                        });
                    }

                    b.iter(|| {
                        unsafe { ustr::_clear_cache() };

                        // Signal all threads to start
                        for _ in 0..num_threads {
                            tx.send(()).unwrap();
                        }

                        // Wait for all threads to complete
                        for _ in 0..num_threads {
                            done_rx.recv().unwrap();
                        }
                    });

                    drop(tx);
                })
                .unwrap();
            },
        );
    }

    // Test concurrent access to existing strings
    let existing_strings: Vec<Ustr> = (0..1000)
        .map(|i| ustr(&format!("existing_string_{}", i)))
        .collect();
    let existing_strings = Arc::new(existing_strings);

    for num_threads in [2, 4, 8, 16].iter() {
        let num_threads = *num_threads;
        let strings = Arc::clone(&existing_strings);

        c.bench_function(
            &format!("concurrent_access_{}_threads", num_threads),
            move |b| {
                let (tx, rx) = bounded(0);
                let (done_tx, done_rx) = bounded(0);

                scope(|scope| {
                    for _thread_id in 0..num_threads {
                        let rx = rx.clone();
                        let done_tx = done_tx.clone();
                        let strings = Arc::clone(&strings);

                        scope.spawn(move |_| {
                            while rx.recv().is_ok() {
                                // Each thread accesses all strings (high
                                // contention)
                                for &ustr in strings.iter() {
                                    black_box(ustr.as_str());
                                    black_box(ustr.len());
                                    black_box(ustr.precomputed_hash());
                                }

                                done_tx.send(()).unwrap();
                            }
                        });
                    }

                    b.iter(|| {
                        for _ in 0..num_threads {
                            tx.send(()).unwrap();
                        }

                        for _ in 0..num_threads {
                            done_rx.recv().unwrap();
                        }
                    });

                    drop(tx);
                })
                .unwrap();
            },
        );
    }

    // Test mixed read/write workload
    let mixed_data = Arc::new(
        (0..500)
            .map(|i| format!("mixed_workload_{}", i))
            .collect::<Vec<_>>(),
    );

    for num_threads in [2, 4, 8].iter() {
        let num_threads = *num_threads;
        let data = Arc::clone(&mixed_data);

        c.bench_function(
            &format!("concurrent_mixed_{}_threads", num_threads),
            move |b| {
                let (tx, rx) = bounded(0);
                let (done_tx, done_rx) = bounded(0);

                scope(|scope| {
                    for thread_id in 0..num_threads {
                        let rx = rx.clone();
                        let done_tx = done_tx.clone();
                        let data = Arc::clone(&data);

                        scope.spawn(move |_| {
                            while rx.recv().is_ok() {
                                if thread_id % 2 == 0 {
                                    // Writer threads: create new strings
                                    for i in 0..100 {
                                        let s = format!(
                                            "thread_{}_string_{}",
                                            thread_id, i
                                        );
                                        black_box(ustr(&s));
                                    }
                                } else {
                                    // Reader threads: access existing strings
                                    for s in data.iter() {
                                        let u = ustr(s);
                                        black_box(u.as_str());
                                        black_box(u.len());
                                    }
                                }

                                done_tx.send(()).unwrap();
                            }
                        });
                    }

                    b.iter(|| {
                        unsafe { ustr::_clear_cache() };

                        for _ in 0..num_threads {
                            tx.send(()).unwrap();
                        }

                        for _ in 0..num_threads {
                            done_rx.recv().unwrap();
                        }
                    });

                    drop(tx);
                })
                .unwrap();
            },
        );
    }

    // Test hash collision scenarios under concurrency
    // Generate strings that are likely to hash to similar bins
    let collision_data = Arc::new(
        (0..200)
            .map(|i| {
                format!("collision_test_prefix_that_is_quite_long_{:08}", i)
            })
            .collect::<Vec<_>>(),
    );

    c.bench_function("concurrent_hash_collisions", |b| {
        let data = Arc::clone(&collision_data);
        let num_threads = 4;
        let (tx, rx) = bounded(0);
        let (done_tx, done_rx) = bounded(0);

        scope(|scope| {
            for _ in 0..num_threads {
                let rx = rx.clone();
                let done_tx = done_tx.clone();
                let data = Arc::clone(&data);

                scope.spawn(move |_| {
                    while rx.recv().is_ok() {
                        // All threads work on the same data (maximum
                        // contention)
                        for s in data.iter() {
                            black_box(ustr(s));
                        }

                        done_tx.send(()).unwrap();
                    }
                });
            }

            b.iter(|| {
                unsafe { ustr::_clear_cache() };

                for _ in 0..num_threads {
                    tx.send(()).unwrap();
                }

                for _ in 0..num_threads {
                    done_rx.recv().unwrap();
                }
            });

            drop(tx);
        })
        .unwrap();
    });

    // Test performance under cache pressure
    c.bench_function("concurrent_cache_pressure", |b| {
        let num_threads = 8;
        let (tx, rx) = bounded(0);
        let (done_tx, done_rx) = bounded(0);

        scope(|scope| {
            for thread_id in 0..num_threads {
                let rx = rx.clone();
                let done_tx = done_tx.clone();

                scope.spawn(move |_| {
                    while rx.recv().is_ok() {
                        // Each thread creates unique strings to maximize cache pressure
                        for i in 0..50 {
                            let s = format!("cache_pressure_thread_{}_iteration_{}_unique_string", thread_id, i);
                            black_box(ustr(&s));
                        }

                        done_tx.send(()).unwrap();
                    }
                });
            }

            b.iter(|| {
                unsafe { ustr::_clear_cache() };

                for _ in 0..num_threads {
                    tx.send(()).unwrap();
                }

                for _ in 0..num_threads {
                    done_rx.recv().unwrap();
                }
            });

            drop(tx);
        }).unwrap();
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
