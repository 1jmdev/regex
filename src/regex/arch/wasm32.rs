use super::generic;

#[cfg(any(target_feature = "simd128", feature = "wasm-simd128"))]
use std::arch::wasm32::{i8x16_bitmask, i8x16_eq, i8x16_splat, v128_load};

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
