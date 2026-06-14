use regex::Regex;
use rust_regex::Regex as RustRegex;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn main() {
    let small_iters = 5_000_000;
    let large_iters = 2_000;
    let small = "x=12 y=345 z=6789";
    let literal = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
    let words = generated_words(100_000);
    let literals = generated_literal(100_000);

    let ours_caps = Regex::new(r"(\w+)=(\d+)").unwrap();
    let rust_caps = RustRegex::new(r"(\w+)=(\d+)").unwrap();
    let ours_lit = Regex::new(r"a+b").unwrap();
    let rust_lit = RustRegex::new(r"a+b").unwrap();

    compare(
        "small captures",
        || {
            repeat(small_iters, || {
                black_box(ours_caps.captures(black_box(small)).map(|c| c[2].len()))
            })
        },
        || {
            repeat(small_iters, || {
                black_box(rust_caps.captures(black_box(small)).map(|c| c[2].len()))
            })
        },
    );

    compare(
        "small literal repeat",
        || {
            repeat(small_iters, || {
                black_box(ours_lit.is_match(black_box(literal)))
            })
        },
        || {
            repeat(small_iters, || {
                black_box(rust_lit.is_match(black_box(literal)))
            })
        },
    );

    compare(
        "large captures 100000",
        || {
            repeat(large_iters, || {
                black_box(ours_caps.captures_iter(black_box(&words)).count())
            })
        },
        || {
            repeat(large_iters, || {
                black_box(rust_caps.captures_iter(black_box(&words)).count())
            })
        },
    );

    compare(
        "large literal 100000",
        || {
            repeat(large_iters, || {
                black_box(ours_lit.find_iter(black_box(&literals)).count())
            })
        },
        || {
            repeat(large_iters, || {
                black_box(rust_lit.find_iter(black_box(&literals)).count())
            })
        },
    );
}

fn compare(
    name: &str,
    mut ours: impl FnMut() -> Duration,
    mut rust_regex: impl FnMut() -> Duration,
) {
    let ours = ours();
    let rust_regex = rust_regex();
    let ratio = rust_regex.as_secs_f64() / ours.as_secs_f64();
    println!("{name}");
    println!("ours: {:?}", ours);
    println!("rust-regex: {:?}", rust_regex);
    println!("ratio: {:.2}x", ratio);
    println!();
}

fn repeat<T>(iters: usize, mut f: impl FnMut() -> T) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    start.elapsed()
}

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
