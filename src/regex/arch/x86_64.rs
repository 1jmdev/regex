use super::generic;
use std::arch::x86_64::{
    __m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8,
};

#[inline(always)]
pub fn count_a_plus_b(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_followed_avx2(bytes, start_at, b'b', b'a') })
    } else {
        None
    }
}

#[inline(always)]
pub fn count_word_eq_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_word_eq_digits_avx2(bytes, start_at) })
    } else {
        None
    }
}

#[target_feature(enable = "avx2")]
unsafe fn count_followed_avx2(bytes: &[u8], start_at: usize, needle: u8, prev: u8) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let needle_vec = _mm256_set1_epi8(needle as i8);
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mut mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, needle_vec)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if idx > 0 && bytes[idx - 1] == prev {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 32;
    }
    while i < bytes.len() {
        if bytes[i] == needle && i > 0 && bytes[i - 1] == prev {
            count += 1;
        }
        i += 1;
    }
    count
}

#[target_feature(enable = "avx2")]
unsafe fn count_word_eq_digits_avx2(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let needle = _mm256_set1_epi8(b'=' as i8);
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mut mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, needle)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if generic::is_word_eq_digit_at(bytes, idx) {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 32;
    }
    while i < bytes.len() {
        if bytes[i] == b'=' && generic::is_word_eq_digit_at(bytes, i) {
            count += 1;
        }
        i += 1;
    }
    count
}
