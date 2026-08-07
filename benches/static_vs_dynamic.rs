use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use ustr::{static_ustr, ustr};

fn bench_static_literals(c: &mut Criterion) {
    c.bench_function("static_ustr_literals", |b| {
        b.iter(|| {
            // These hashes can be computed at compile time
            let s1 = static_ustr!("hello");
            let s2 = static_ustr!("world");
            let s3 = static_ustr!("foo");
            let s4 = static_ustr!("bar");
            let s5 = static_ustr!("baz");
            black_box((s1, s2, s3, s4, s5))
        });
    });
}

fn bench_dynamic_literals(c: &mut Criterion) {
    c.bench_function("dynamic_ustr_literals", |b| {
        b.iter(|| {
            // Regular runtime hashing
            let s1 = ustr("hello");
            let s2 = ustr("world");
            let s3 = ustr("foo");
            let s4 = ustr("bar");
            let s5 = ustr("baz");
            black_box((s1, s2, s3, s4, s5))
        });
    });
}

fn bench_static_const_hash(c: &mut Criterion) {
    c.bench_function("const_hash_compile_time", |b| {
        b.iter(|| {
            // These are computed at compile time!
            const H1: u64 = ustr::hash::string_hash(b"compile-time-hash-1");
            const H2: u64 = ustr::hash::string_hash(b"compile-time-hash-2");
            const H3: u64 = ustr::hash::string_hash(b"compile-time-hash-3");
            black_box((H1, H2, H3))
        });
    });
}

fn bench_runtime_hash(c: &mut Criterion) {
    c.bench_function("runtime_hash", |b| {
        b.iter(|| {
            let h1 = ustr::hash::runtime_hash(b"runtime-hash-1");
            let h2 = ustr::hash::runtime_hash(b"runtime-hash-2");
            let h3 = ustr::hash::runtime_hash(b"runtime-hash-3");
            black_box((h1, h2, h3))
        });
    });
}

fn bench_static_cached_lookup(c: &mut Criterion) {
    // Pre-populate cache
    let _ = static_ustr!("cached-string-1");
    let _ = static_ustr!("cached-string-2");
    let _ = static_ustr!("cached-string-3");

    c.bench_function("static_cached_lookup", |b| {
        b.iter(|| {
            // These should find the strings in cache
            let s1 = static_ustr!("cached-string-1");
            let s2 = static_ustr!("cached-string-2");
            let s3 = static_ustr!("cached-string-3");
            black_box((s1, s2, s3))
        });
    });
}

fn bench_dynamic_cached_lookup(c: &mut Criterion) {
    // Pre-populate cache
    let _ = ustr("dynamic-cached-1");
    let _ = ustr("dynamic-cached-2");
    let _ = ustr("dynamic-cached-3");

    c.bench_function("dynamic_cached_lookup", |b| {
        b.iter(|| {
            // These should find the strings in cache
            let s1 = ustr("dynamic-cached-1");
            let s2 = ustr("dynamic-cached-2");
            let s3 = ustr("dynamic-cached-3");
            black_box((s1, s2, s3))
        });
    });
}

fn bench_mixed_usage(c: &mut Criterion) {
    c.bench_function("mixed_static_dynamic", |b| {
        b.iter(|| {
            // Mix of static and dynamic
            let s1 = static_ustr!("static-literal");
            let s2 = ustr("dynamic-literal");
            let s3 = static_ustr!("another-static");

            // Comparison is always fast (pointer comparison)
            let eq = s1 == s2;
            let eq2 = s2 == s3;

            black_box((s1, s2, s3, eq, eq2))
        });
    });
}

criterion_group!(
    benches,
    bench_static_literals,
    bench_dynamic_literals,
    bench_static_const_hash,
    bench_runtime_hash,
    bench_static_cached_lookup,
    bench_dynamic_cached_lookup,
    bench_mixed_usage
);
criterion_main!(benches);
