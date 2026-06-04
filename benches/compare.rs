use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use regex::Regex;
use rust_regex::Regex as RustRegex;
use std::hint::black_box;

fn generated_words(bytes: usize) -> String {
    let mut text = String::with_capacity(bytes + 32);
    let mut i = 0usize;
    while text.len() < bytes {
        text.push_str("key");
        text.push_str(&(i % 1000).to_string());
        text.push('=');
        text.push_str(&((i * 17) % 100000).to_string());
        text.push_str(", ");
        i += 1;
    }
    text
}

fn generated_literal(bytes: usize) -> String {
    let mut text = String::with_capacity(bytes + 16);
    while text.len() < bytes {
        text.push_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab");
    }
    text
}

fn bench_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("small");

    let ours = Regex::new(r"(\w+)=(\d+)").unwrap();
    let official = RustRegex::new(r"(\w+)=(\d+)").unwrap();
    let haystack = "x=12 y=345 z=6789";

    group.bench_function("ours captures", |b| {
        b.iter(|| black_box(ours.captures(black_box(haystack)).map(|c| c[2].to_string())))
    });
    group.bench_function("rust-regex captures", |b| {
        b.iter(|| {
            black_box(
                official
                    .captures(black_box(haystack))
                    .map(|c| c[2].to_string()),
            )
        })
    });

    let ours = Regex::new(r"a+b").unwrap();
    let official = RustRegex::new(r"a+b").unwrap();
    let haystack = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";

    group.bench_function("ours literal repeat", |b| {
        b.iter(|| black_box(ours.is_match(black_box(haystack))))
    });
    group.bench_function("rust-regex literal repeat", |b| {
        b.iter(|| black_box(official.is_match(black_box(haystack))))
    });

    group.finish();
}

fn bench_large(c: &mut Criterion) {
    let sizes = [10_000usize, 100_000usize];

    for size in sizes {
        let haystack = generated_words(size);
        let ours = Regex::new(r"(\w+)=(\d+)").unwrap();
        let official = RustRegex::new(r"(\w+)=(\d+)").unwrap();

        let mut group = c.benchmark_group(format!("large captures {size}"));
        group.throughput(Throughput::Bytes(haystack.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("ours captures_iter", size),
            &haystack,
            |b, h| b.iter(|| black_box(ours.captures_iter(black_box(h)).count())),
        );
        group.bench_with_input(
            BenchmarkId::new("rust-regex captures_iter", size),
            &haystack,
            |b, h| b.iter(|| black_box(official.captures_iter(black_box(h)).count())),
        );
        group.finish();
    }

    for size in sizes {
        let haystack = generated_literal(size);
        let ours = Regex::new(r"a+b").unwrap();
        let official = RustRegex::new(r"a+b").unwrap();

        let mut group = c.benchmark_group(format!("large literal {size}"));
        group.throughput(Throughput::Bytes(haystack.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("ours find_iter", size),
            &haystack,
            |b, h| b.iter(|| black_box(ours.find_iter(black_box(h)).count())),
        );
        group.bench_with_input(
            BenchmarkId::new("rust-regex find_iter", size),
            &haystack,
            |b, h| b.iter(|| black_box(official.find_iter(black_box(h)).count())),
        );
        group.finish();
    }
}

criterion_group!(benches, bench_small, bench_large);
criterion_main!(benches);
