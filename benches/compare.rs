use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fast_reg::Regex;
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

fn generated_mixed(bytes: usize) -> String {
    let mut text = String::with_capacity(bytes + 64);
    let words = ["alpha", "ERROR", "beta_42", "warning", "Error", "delta99"];
    let mut i = 0usize;
    while text.len() < bytes {
        text.push_str(words[i % words.len()]);
        text.push(' ');
        text.push_str(&format!("{:04}", i % 10000));
        text.push(' ');
        i += 1;
    }
    text
}

struct Case<'a> {
    name: &'a str,
    pattern: &'a str,
    haystack: String,
}

fn bench_pattern_matrix(c: &mut Criterion) {
    let cases = [
        Case {
            name: "simple digits",
            pattern: r"\d+",
            haystack: generated_mixed(100_000),
        },
        Case {
            name: "simple words",
            pattern: r"\w+",
            haystack: generated_mixed(100_000),
        },
        Case {
            name: "simple alpha underscore",
            pattern: r"[a-zA-Z_]+",
            haystack: generated_mixed(100_000),
        },
        Case {
            name: "bounded exact digits",
            pattern: r"\d{4}",
            haystack: generated_mixed(100_000),
        },
        Case {
            name: "bounded open words",
            pattern: r"\w{2,}",
            haystack: generated_mixed(100_000),
        },
        Case {
            name: "case insensitive error",
            pattern: r"(?i)error",
            haystack: generated_mixed(100_000),
        },
        Case {
            name: "backtracking alternation",
            pattern: r"(a|aa)+b",
            haystack: generated_literal(10_000),
        },
        Case {
            name: "backtracking nested repeat",
            pattern: r"(a+)+b",
            haystack: generated_literal(10_000),
        },
    ];

    for case in cases {
        let ours = Regex::new(case.pattern).unwrap();
        let official = RustRegex::new(case.pattern).unwrap();

        let mut group = c.benchmark_group(format!("pattern matrix/{}", case.name));
        group.throughput(Throughput::Bytes(case.haystack.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("ours find_iter", case.pattern),
            &case.haystack,
            |b, h| b.iter(|| black_box(ours.find_iter(black_box(h)).count())),
        );
        group.bench_with_input(
            BenchmarkId::new("rust-regex find_iter", case.pattern),
            &case.haystack,
            |b, h| b.iter(|| black_box(official.find_iter(black_box(h)).count())),
        );
        group.finish();
    }
}

fn bench_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("small");

    let ours = Regex::new(r"(\w+)=(\d+)").unwrap();
    let official = RustRegex::new(r"(\w+)=(\d+)").unwrap();
    let haystack = "x=12 y=345 z=6789";

    group.bench_function("ours captures", |b| {
        b.iter(|| black_box(ours.captures(black_box(haystack)).map(|c| c[2].len())))
    });
    group.bench_function("rust-regex captures", |b| {
        b.iter(|| black_box(official.captures(black_box(haystack)).map(|c| c[2].len())))
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

criterion_group!(benches, bench_small, bench_large, bench_pattern_matrix);
criterion_main!(benches);
