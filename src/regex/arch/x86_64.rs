use super::generic::{self, ByteClass};
use std::arch::x86_64::{
    __m256i, _mm256_and_si256, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_loadu_si256,
    _mm256_movemask_epi8, _mm256_or_si256, _mm256_set1_epi8,
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

#[inline(always)]
pub fn count_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_class_runs_avx2(bytes, start_at, ByteClass::Digit) })
    } else {
        None
    }
}

#[inline(always)]
pub fn count_words(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_class_runs_avx2(bytes, start_at, ByteClass::Word) })
    } else {
        None
    }
}

#[inline(always)]
pub fn count_alpha_underscore(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_class_runs_avx2(bytes, start_at, ByteClass::AlphaUnderscore) })
    } else {
        None
    }
}

#[inline(always)]
pub fn count_four_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_fixed_class_runs_avx2(bytes, start_at, 4, ByteClass::Digit) })
    } else {
        None
    }
}

#[inline(always)]
pub fn count_words_min2(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_min2_class_runs_avx2(bytes, start_at, ByteClass::Word) })
    } else {
        None
    }
}

#[inline(always)]
pub fn count_ascii_case_error(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_ascii_case_error_avx2(bytes, start_at) })
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

#[target_feature(enable = "avx2")]
unsafe fn count_class_runs_avx2(bytes: &[u8], start_at: usize, class: ByteClass) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut prev_match = false;
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mask = unsafe { class_mask(chunk, class) };
        count += generic::count_mask_run_starts(mask, 32, prev_match);
        prev_match = (mask & (1 << 31)) != 0;
        i += 32;
    }
    count + generic::count_class_runs_scalar(bytes, i, class, prev_match)
}

#[target_feature(enable = "avx2")]
unsafe fn count_min2_class_runs_avx2(bytes: &[u8], start_at: usize, class: ByteClass) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut prev_match = false;
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mask = unsafe { class_mask(chunk, class) };
        let next = i + 32 < bytes.len() && generic::class_matches_byte(bytes[i + 32], class);
        count += generic::count_mask_min2_run_starts(mask, 32, prev_match, next);
        prev_match = (mask & (1 << 31)) != 0;
        i += 32;
    }
    count + generic::count_min_class_runs_scalar(bytes, i, 2, class, prev_match)
}

#[target_feature(enable = "avx2")]
unsafe fn count_fixed_class_runs_avx2(
    bytes: &[u8],
    start_at: usize,
    len: usize,
    class: ByteClass,
) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut run = 0usize;
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mask = unsafe { class_mask(chunk, class) };
        count += generic::add_fixed_mask_runs(mask, 32, len, &mut run);
        i += 32;
    }
    count + generic::count_fixed_class_runs_scalar(bytes, i, len, class, run)
}

#[target_feature(enable = "avx2")]
unsafe fn count_ascii_case_error_avx2(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let lower_e = _mm256_set1_epi8(b'e' as i8);
    let upper_e = _mm256_set1_epi8(b'E' as i8);
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let e = _mm256_cmpeq_epi8(chunk, lower_e);
        let upper = _mm256_cmpeq_epi8(chunk, upper_e);
        let mut mask = _mm256_movemask_epi8(_mm256_or_si256(e, upper)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if idx + 5 <= bytes.len() && generic::eq_ascii_case_error_at(bytes, idx) {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 32;
    }
    count + generic::count_ascii_case_error_bytes(bytes, i)
}

#[target_feature(enable = "avx2")]
unsafe fn class_mask(chunk: __m256i, class: ByteClass) -> u32 {
    match class {
        ByteClass::Digit => unsafe { range_mask(chunk, b'0', b'9') },
        ByteClass::Word => {
            let alpha = unsafe { alpha_mask(chunk) };
            let digit = unsafe { range_mask(chunk, b'0', b'9') };
            let underscore = _mm256_cmpeq_epi8(chunk, _mm256_set1_epi8(b'_' as i8));
            alpha | digit | (_mm256_movemask_epi8(underscore) as u32)
        }
        ByteClass::AlphaUnderscore => {
            let alpha = unsafe { alpha_mask(chunk) };
            let underscore = _mm256_cmpeq_epi8(chunk, _mm256_set1_epi8(b'_' as i8));
            alpha | (_mm256_movemask_epi8(underscore) as u32)
        }
    }
}

#[target_feature(enable = "avx2")]
unsafe fn alpha_mask(chunk: __m256i) -> u32 {
    unsafe { range_mask(chunk, b'a', b'z') | range_mask(chunk, b'A', b'Z') }
}

#[target_feature(enable = "avx2")]
unsafe fn range_mask(chunk: __m256i, lo: u8, hi: u8) -> u32 {
    let above_lo = _mm256_cmpgt_epi8(chunk, _mm256_set1_epi8((lo - 1) as i8));
    let below_hi = _mm256_cmpgt_epi8(_mm256_set1_epi8((hi + 1) as i8), chunk);
    _mm256_movemask_epi8(_mm256_and_si256(above_lo, below_hi)) as u32
}
