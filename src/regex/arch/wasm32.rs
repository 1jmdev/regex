use super::generic::{self, ByteClass};

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
use std::arch::wasm32::{
    i8x16_bitmask, i8x16_eq, i8x16_gt, i8x16_splat, v128, v128_and, v128_load, v128_or,
};

#[inline(always)]
pub fn count_a_plus_b(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe { count_followed_simd128(bytes, start_at, b'b', b'a') });
    }
    #[allow(unreachable_code)]
    Some(generic::count_a_plus_b_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_word_eq_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe { count_word_eq_digits_simd128(bytes, start_at) });
    }
    #[allow(unreachable_code)]
    Some(generic::count_word_eq_digits_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe { count_class_runs_simd128(bytes, start_at, ByteClass::Digit) });
    }
    #[allow(unreachable_code)]
    Some(generic::count_digits_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_words(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe { count_class_runs_simd128(bytes, start_at, ByteClass::Word) });
    }
    #[allow(unreachable_code)]
    Some(generic::count_words_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_alpha_underscore(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe {
            count_class_runs_simd128(bytes, start_at, ByteClass::AlphaUnderscore)
        });
    }
    #[allow(unreachable_code)]
    Some(generic::count_alpha_underscore_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_four_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe {
            count_fixed_class_runs_simd128(bytes, start_at, 4, ByteClass::Digit)
        });
    }
    #[allow(unreachable_code)]
    Some(generic::count_four_digits_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_words_min2(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe { count_min2_class_runs_simd128(bytes, start_at, ByteClass::Word) });
    }
    #[allow(unreachable_code)]
    Some(generic::count_words_min2_bytes(bytes, start_at))
}

#[inline(always)]
pub fn count_ascii_case_error(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
    {
        return Some(unsafe { count_ascii_case_error_simd128(bytes, start_at) });
    }
    #[allow(unreachable_code)]
    Some(generic::count_ascii_case_error_bytes(bytes, start_at))
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn count_followed_simd128(bytes: &[u8], start_at: usize, needle: u8, prev: u8) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let needle = i8x16_splat(needle as i8);
    while i + 16 <= bytes.len() {
        let chunk = unsafe { v128_load(bytes.as_ptr().add(i) as *const _) };
        let mut mask = i8x16_bitmask(i8x16_eq(chunk, needle)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if idx > 0 && bytes[idx - 1] == prev {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 16;
    }
    count + generic::count_a_plus_b_bytes(bytes, i)
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn count_word_eq_digits_simd128(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let needle = i8x16_splat(b'=' as i8);
    while i + 16 <= bytes.len() {
        let chunk = unsafe { v128_load(bytes.as_ptr().add(i) as *const _) };
        let mut mask = i8x16_bitmask(i8x16_eq(chunk, needle)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if generic::is_word_eq_digit_at(bytes, idx) {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 16;
    }
    count + generic::count_word_eq_digits_bytes(bytes, i)
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn count_class_runs_simd128(bytes: &[u8], start_at: usize, class: ByteClass) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut prev_match = false;
    while i + 16 <= bytes.len() {
        let chunk = unsafe { v128_load(bytes.as_ptr().add(i) as *const _) };
        let mask = unsafe { class_mask(chunk, class) };
        count += generic::count_mask_run_starts(mask, 16, prev_match);
        prev_match = (mask & (1 << 15)) != 0;
        i += 16;
    }
    count + generic::count_class_runs_scalar(bytes, i, class, prev_match)
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn count_min2_class_runs_simd128(bytes: &[u8], start_at: usize, class: ByteClass) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut prev_match = false;
    while i + 16 <= bytes.len() {
        let chunk = unsafe { v128_load(bytes.as_ptr().add(i) as *const _) };
        let mask = unsafe { class_mask(chunk, class) };
        let next = i + 16 < bytes.len() && generic::class_matches_byte(bytes[i + 16], class);
        count += generic::count_mask_min2_run_starts(mask, 16, prev_match, next);
        prev_match = (mask & (1 << 15)) != 0;
        i += 16;
    }
    count + generic::count_min_class_runs_scalar(bytes, i, 2, class, prev_match)
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn count_fixed_class_runs_simd128(
    bytes: &[u8],
    start_at: usize,
    len: usize,
    class: ByteClass,
) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut run = 0usize;
    while i + 16 <= bytes.len() {
        let chunk = unsafe { v128_load(bytes.as_ptr().add(i) as *const _) };
        let mask = unsafe { class_mask(chunk, class) };
        count += generic::add_fixed_mask_runs(mask, 16, len, &mut run);
        i += 16;
    }
    count + generic::count_fixed_class_runs_scalar(bytes, i, len, class, run)
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn count_ascii_case_error_simd128(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let lower_e = i8x16_splat(b'e' as i8);
    let upper_e = i8x16_splat(b'E' as i8);
    while i + 16 <= bytes.len() {
        let chunk = unsafe { v128_load(bytes.as_ptr().add(i) as *const _) };
        let lower = i8x16_eq(chunk, lower_e);
        let upper = i8x16_eq(chunk, upper_e);
        let mut mask = i8x16_bitmask(v128_or(lower, upper)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if idx + 5 <= bytes.len() && generic::eq_ascii_case_error_at(bytes, idx) {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 16;
    }
    count + generic::count_ascii_case_error_bytes(bytes, i)
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn class_mask(chunk: v128, class: ByteClass) -> u32 {
    match class {
        ByteClass::Digit => unsafe { range_mask(chunk, b'0', b'9') },
        ByteClass::Word => {
            let alpha = unsafe { alpha_mask(chunk) };
            let digit = unsafe { range_mask(chunk, b'0', b'9') };
            let underscore = i8x16_bitmask(i8x16_eq(chunk, i8x16_splat(b'_' as i8))) as u32;
            alpha | digit | underscore
        }
        ByteClass::AlphaUnderscore => {
            let alpha = unsafe { alpha_mask(chunk) };
            let underscore = i8x16_bitmask(i8x16_eq(chunk, i8x16_splat(b'_' as i8))) as u32;
            alpha | underscore
        }
    }
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn alpha_mask(chunk: v128) -> u32 {
    unsafe { range_mask(chunk, b'a', b'z') | range_mask(chunk, b'A', b'Z') }
}

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
#[target_feature(enable = "simd128")]
unsafe fn range_mask(chunk: v128, lo: u8, hi: u8) -> u32 {
    let above_lo = i8x16_gt(chunk, i8x16_splat((lo - 1) as i8));
    let below_hi = i8x16_gt(i8x16_splat((hi + 1) as i8), chunk);
    i8x16_bitmask(v128_and(above_lo, below_hi)) as u32
}
