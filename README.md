# Fast Regex

A small, fast Rust regex engine that is a **drop-in replacement** for the
official [`regex`](https://crates.io/crates/regex) crate.

Same public API as `regex`, so if your code compiles against `regex` it compiles
against this crate. Underneath it's a tiny implementation with a much smaller
code size and memory footprint, and it's faster.

## Why use it

- **Same API as `regex`.** Same types, same method names, same semantics.
- **Tiny footprint.** Minimal code size and memory use, good for WASM, embedded,
  and binary-size-sensitive builds.
- **Fast.** Faster literal, capture, and iterator paths (see benchmarks below).

## Installation

Direct git dependency:

```toml
[dependencies]
regex = { git = "https://github.com/1jmdev/regex.git" }
```

Or, since it shares the crate name `regex`, patch it in across your whole
dependency tree without changing your existing `regex` dependency:

```toml
[dependencies]
regex = "1.12.4"

[patch.crates-io]
regex = { git = "https://github.com/1jmdev/regex.git" }
```

## Usage

Matching and searching:

```rust
use regex::Regex;

let re = Regex::new(r"h.llo").unwrap();

assert!(re.is_match("well hello there"));

if let Some(m) = re.find("well hello there") {
    println!("{} {} {}", m.as_str(), m.start(), m.end());
}
```

Captures:

```rust
use regex::Regex;

let re = Regex::new(r"(\w+)=(\d+)").unwrap();
let caps = re.captures("name=42").unwrap();

println!("full: {}", &caps[0]);
println!("key: {}",  &caps[1]);
println!("value: {}", &caps[2]);
```

Iterating over all matches:

```rust
use regex::Regex;

let re = Regex::new(r"\d+").unwrap();
let nums: Vec<&str> = re.find_iter("1 22 333").map(|m| m.as_str()).collect();
assert_eq!(nums, ["1", "22", "333"]);
```

Replacing and splitting:

```rust
use regex::Regex;

let re = Regex::new(r"\s+").unwrap();
assert_eq!(re.replace_all("a  b   c", "-"), "a-b-c");

let re = Regex::new(r",").unwrap();
let parts: Vec<&str> = re.split("a,b,c").collect();
assert_eq!(parts, ["a", "b", "c"]);
```

Matching multiple patterns with `RegexSet`:

```rust
use regex::RegexSet;

let set = RegexSet::new(&[r"\d+", r"[a-z]+", r"\s+"]).unwrap();
let matches: Vec<usize> = set.matches("abc123").into_iter().collect();
assert_eq!(matches, [0, 1]);
```

Working on raw bytes with the `bytes` module:

```rust
use regex::bytes::Regex;

let re = Regex::new(r"\d+").unwrap();
if let Some(m) = re.find(b"id 42") {
    assert_eq!(m.as_bytes(), b"42");
}
```

Runnable versions of these live in [`examples/`](examples/) — try them with
`cargo run --example basic`, `--example captures`, `--example iter`, and so on.

## Benchmarks

Benchmark results compared with the official `regex` crate:

| Benchmark | Fast regex | regex (official) | Faster |
| --- | ---: | ---: | ---: |
| small captures | 7.056 ns | 82.607 ns | 11.7x |
| small literal repeat | 4.929 ns | 61.429 ns | 12.5x |
| captures_iter / 10,000 | 957.03 ns, 9.73 GiB/s | 38.099 us, 250.34 MiB/s | 39.8x |
| captures_iter / 100,000 | 8.900 us, 10.46 GiB/s | 368.91 us, 258.54 MiB/s | 41.4x |
| literal find_iter / 10,000 | 334.09 ns, 27.96 GiB/s | 19.696 us, 485.66 MiB/s | 59.0x |
| literal find_iter / 100,000 | 3.233 us, 28.82 GiB/s | 195.47 us, 488.02 MiB/s | 60.5x |
| `\d+` find_iter | 4.996 us, 18.64 GiB/s | 288.47 us, 330.62 MiB/s | 57.7x |
| `\w+` find_iter | 5.697 us, 16.35 GiB/s | 385.25 us, 247.56 MiB/s | 67.6x |
| `[a-zA-Z_]+` find_iter | 5.917 us, 15.74 GiB/s | 255.95 us, 372.62 MiB/s | 43.3x |
| `\d{4}` find_iter | 80.343 us, 1.159 GiB/s | 255.25 us, 373.66 MiB/s | 3.2x |
| `\w{2,}` find_iter | 8.035 us, 11.59 GiB/s | 377.37 us, 252.73 MiB/s | 47.0x |
| `(?i)error` find_iter | 15.271 us, 6.10 GiB/s | 120.46 us, 791.75 MiB/s | 7.9x |
| `(a\|aa)+b` find_iter | 340.97 ns, 27.40 GiB/s | 19.469 us, 491.31 MiB/s | 57.1x |
| `(a+)+b` find_iter | 333.92 ns, 27.97 GiB/s | 19.633 us, 487.21 MiB/s | 58.8x |

Run them yourself with `cargo bench`.

## License

MIT License. See [`LICENSE`](LICENSE).
