#[macro_use]
extern crate criterion;
use criterion::black_box;
use criterion::Criterion;
use crossbeam_channel::bounded;
use crossbeam_utils::thread::scope;
use std::sync::Arc;

use ustring::*;

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

    let s = blns.clone();
    let num_threads = 4;
    let num = 10_000;
    c.bench_function("multi", move |b| {
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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);