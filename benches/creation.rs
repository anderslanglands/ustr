#[macro_use]
extern crate criterion;
use criterion::black_box;
use criterion::Criterion;
use crossbeam_channel::bounded;
use crossbeam_utils::thread::scope;
use std::sync::Arc;
use string_cache::DefaultAtom;
use string_interner::StringInterner;

use ustr::*;

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

    let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("data")
        .join("raft-large-directories.txt");
    let raft = std::fs::read_to_string(path).unwrap();
    let raft = Arc::new(
        raft.split_whitespace()
            .collect::<Vec<_>>()
            .chunks(3)
            .map(|s| {
                if s.len() == 3 {
                    format!("{}/{}/{}", s[0], s[1], s[2])
                } else {
                    s[0].to_owned()
                }
            })
            .collect::<Vec<_>>(),
    );

    for s in raft.iter().take(5) {
        println!("{}", s);
    }

    let num = 4_000;
    for num_threads in [1, 2, 4, 6, 8, 12].iter() {
        let num_threads = *num_threads;
        let s = blns.clone();
        c.bench_function(&format!("blns ustr x {} threads", num_threads), move |b| {
            let (tx1, rx1) = bounded(0);
            let (tx2, rx2) = bounded(0);
            ustr::_clear_cache();
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
        c.bench_function(
            &format!("blns string-interner x {} threads", num_threads),
            move |b| {
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
            },
        );

        let s = blns.clone();
        c.bench_function(
            &format!("blns string-cache x {} threads", num_threads),
            move |b| {
                let (tx1, rx1) = bounded(0);
                let (tx2, rx2) = bounded(0);
                scope(|scope| {
                    for _ in 0..num_threads {
                        scope.spawn(|_| {
                            while rx1.recv().is_ok() {
                                let mut v = Vec::with_capacity(num);
                                for s in s.iter().cycle().take(num) {
                                    v.push(DefaultAtom::from(s.as_str()));
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
            },
        );

        let s = blns.clone();
        c.bench_function(
            &format!("blns String::from x {} threads", num_threads),
            move |b| {
                let (tx1, rx1) = bounded(0);
                let (tx2, rx2) = bounded(0);
                scope(|scope| {
                    for _ in 0..num_threads {
                        scope.spawn(|_| {
                            while rx1.recv().is_ok() {
                                for s in s.iter().cycle().take(num) {
                                    black_box(String::from(s));
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
            },
        );
    }

    for num_threads in [1, 2, 4, 6, 8, 12].iter() {
        let num_threads = *num_threads;

        let s = Arc::clone(&raft);

        c.bench_function(&format!("raft ustr x {} threads", num_threads), move |b| {
            let (tx1, rx1) = bounded(0);
            let (tx2, rx2) = bounded(0);
            let s = Arc::clone(&s);
            scope(|scope| {
                ustr::_clear_cache();
                for tt in 0..num_threads {
                    let t = tt;
                    let rx1 = rx1.clone();
                    let tx2 = tx2.clone();
                    let s = Arc::clone(&s);
                    scope.spawn(move |_| {
                        while rx1.recv().is_ok() {
                            for s in s.iter().skip(t * 1000).take(1000) {
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

        let s = Arc::clone(&raft);
        c.bench_function(
            &format!("raft string-interner x {} threads", num_threads),
            move |b| {
                let (tx1, rx1) = bounded(0);
                let (tx2, rx2) = bounded(0);
                let interner = Arc::new(spin::Mutex::new(StringInterner::default()));
                scope(|scope| {
                    for tt in 0..num_threads {
                        let t = tt;
                        let rx1 = rx1.clone();
                        let tx2 = tx2.clone();
                        let s = Arc::clone(&s);
                        let interner = Arc::clone(&interner);
                        scope.spawn(move |_| {
                            while rx1.recv().is_ok() {
                                for s in s.iter().skip(t * 1000).take(1000) {
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
            },
        );

        let s = Arc::clone(&raft);
        c.bench_function(
            &format!("raft string-cache x {} threads", num_threads),
            move |b| {
                let (tx1, rx1) = bounded(0);
                let (tx2, rx2) = bounded(0);
                scope(|scope| {
                    for tt in 0..num_threads {
                        let t = tt;
                        let rx1 = rx1.clone();
                        let tx2 = tx2.clone();
                        let s = Arc::clone(&s);
                        scope.spawn(move |_| {
                            while rx1.recv().is_ok() {
                                let mut v = Vec::with_capacity(1000);
                                for s in s.iter().skip(t * 1000).take(1000) {
                                    v.push(DefaultAtom::from(s.as_str()));
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
            },
        );

        let s = Arc::clone(&raft);
        c.bench_function(
            &format!("raft String::from x {} threads", num_threads),
            move |b| {
                let (tx1, rx1) = bounded(0);
                let (tx2, rx2) = bounded(0);
                scope(|scope| {
                    for tt in 0..num_threads {
                        let t = tt;
                        let rx1 = rx1.clone();
                        let tx2 = tx2.clone();
                        let s = Arc::clone(&s);
                        scope.spawn(move |_| {
                            while rx1.recv().is_ok() {
                                for s in s.iter().skip(t * 1000).take(1000) {
                                    black_box(String::from(s));
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
            },
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
