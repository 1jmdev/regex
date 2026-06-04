# Fast Reg

Fast Reg is a small, fast Rust regex engine intended as a lightweight alternative to the original `regex` crate.

This crate focuses on keeping the implementation tiny, with a much smaller memory footprint and binary impact: near nothing compared with roughly 2 MB for the original Rust regex stack in similar use cases.

## Highlights

- Minimal Rust regex alternative
- Much smaller code size and memory footprint
- Fast literal, capture, and iterator paths
- API shape centered around `Regex`, `Match`, `Captures`, `find_iter`, `captures_iter`, `split`, `replace`, and `replace_all`

## Benchmarks

Benchmark results compared with `rust-regex`:

| Benchmark | Fast Reg | rust-regex | Faster |
| --- | ---: | ---: | ---: |
| small captures | 7.056 ns | 82.607 ns | 11.7x |
| small literal repeat | 4.929 ns | 61.429 ns | 12.5x |
| captures_iter / 10,000 | 957.03 ns, 9.73 GiB/s | 38.099 us, 250.34 MiB/s | 39.8x |
| captures_iter / 100,000 | 8.900 us, 10.46 GiB/s | 368.91 us, 258.54 MiB/s | 41.4x |
| literal find_iter / 10,000 | 334.09 ns, 27.96 GiB/s | 19.696 us, 485.66 MiB/s | 59.0x |
| literal find_iter / 100,000 | 3.233 us, 28.82 GiB/s | 195.47 us, 488.02 MiB/s | 60.5x |
| `\\d+` find_iter | 4.996 us, 18.64 GiB/s | 288.47 us, 330.62 MiB/s | 57.7x |
| `\\w+` find_iter | 5.697 us, 16.35 GiB/s | 385.25 us, 247.56 MiB/s | 67.6x |
| `[a-zA-Z_]+` find_iter | 5.917 us, 15.74 GiB/s | 255.95 us, 372.62 MiB/s | 43.3x |
| `\\d{4}` find_iter | 80.343 us, 1.159 GiB/s | 255.25 us, 373.66 MiB/s | 3.2x |
| `\\w{2,}` find_iter | 8.035 us, 11.59 GiB/s | 377.37 us, 252.73 MiB/s | 47.0x |
| `(?i)error` find_iter | 15.271 us, 6.10 GiB/s | 120.46 us, 791.75 MiB/s | 7.9x |
| `(a\|aa)+b` find_iter | 340.97 ns, 27.40 GiB/s | 19.469 us, 491.31 MiB/s | 57.1x |
| `(a+)+b` find_iter | 333.92 ns, 27.97 GiB/s | 19.633 us, 487.21 MiB/s | 58.8x |

## License

MIT License. See `LICENSE`.
