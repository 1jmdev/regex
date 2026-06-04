use super::generic::{self, ByteClass};
use std::arch::aarch64::{
    uint8x16_t, vandq_u8, vceqq_u8, vcgeq_u8, vcleq_u8, vdupq_n_u8, vld1q_u8, vorrq_u8, vst1q_u8,
};

#[inline(always)]
pub fn count_a_plus_b(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_followed_neon(bytes, start_at, b'b', b'a') })
}

#[inline(always)]
pub fn count_word_eq_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_word_eq_digits_neon(bytes, start_at) })
}

#[inline(always)]
pub fn count_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_class_runs_neon(bytes, start_at, ByteClass::Digit) })
}

#[inline(always)]
pub fn count_words(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_class_runs_neon(bytes, start_at, ByteClass::Word) })
}

#[inline(always)]
pub fn count_alpha_underscore(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_class_runs_neon(bytes, start_at, ByteClass::AlphaUnderscore) })
}

#[inline(always)]
pub fn count_four_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_fixed_class_runs_neon(bytes, start_at, 4, ByteClass::Digit) })
}

#[inline(always)]
pub fn count_words_min2(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_min2_class_runs_neon(bytes, start_at, ByteClass::Word) })
}

#[inline(always)]
pub fn count_ascii_case_error(bytes: &[u8], start_at: usize) -> Option<usize> {
    Some(unsafe { count_ascii_case_error_neon(bytes, start_at) })
}

unsafe fn count_followed_neon(bytes: &[u8], start_at: usize, needle: u8, prev: u8) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let needle = unsafe { vdupq_n_u8(needle) };
    let mut mask = [0u8; 16];
    while i + 16 <= bytes.len() {
        let chunk = unsafe { vld1q_u8(bytes.as_ptr().add(i)) };
        let cmp = unsafe { vceqq_u8(chunk, needle) };
        unsafe { vst1q_u8(mask.as_mut_ptr(), cmp) };
        for (byte, &hit) in mask.iter().enumerate() {
            if hit != 0 {
                let idx = i + byte;
                if idx > 0 && bytes[idx - 1] == prev {
                    count += 1;
                }
            }
        }
        i += 16;
    }
    count + generic::count_a_plus_b_bytes(bytes, i)
}

unsafe fn count_word_eq_digits_neon(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let needle = unsafe { vdupq_n_u8(b'=') };
    let mut mask = [0u8; 16];
    while i + 16 <= bytes.len() {
        let chunk = unsafe { vld1q_u8(bytes.as_ptr().add(i)) };
        let cmp = unsafe { vceqq_u8(chunk, needle) };
        unsafe { vst1q_u8(mask.as_mut_ptr(), cmp) };
        for (byte, &hit) in mask.iter().enumerate() {
            if hit != 0 {
                let idx = i + byte;
                if generic::is_word_eq_digit_at(bytes, idx) {
                    count += 1;
                }
            }
        }
        i += 16;
    }
    count + generic::count_word_eq_digits_bytes(bytes, i)
}

unsafe fn count_class_runs_neon(bytes: &[u8], start_at: usize, class: ByteClass) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut prev_match = false;
    let mut mask_bytes = [0u8; 16];
    while i + 16 <= bytes.len() {
        let chunk = unsafe { vld1q_u8(bytes.as_ptr().add(i)) };
        let mask = unsafe { class_mask(chunk, class, &mut mask_bytes) };
        count += generic::count_mask_run_starts(mask, 16, prev_match);
        prev_match = (mask & (1 << 15)) != 0;
        i += 16;
    }
    count + generic::count_class_runs_scalar(bytes, i, class, prev_match)
}

unsafe fn count_min2_class_runs_neon(bytes: &[u8], start_at: usize, class: ByteClass) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut prev_match = false;
    let mut mask_bytes = [0u8; 16];
    while i + 16 <= bytes.len() {
        let chunk = unsafe { vld1q_u8(bytes.as_ptr().add(i)) };
        let mask = unsafe { class_mask(chunk, class, &mut mask_bytes) };
        let next = i + 16 < bytes.len() && generic::class_matches_byte(bytes[i + 16], class);
        count += generic::count_mask_min2_run_starts(mask, 16, prev_match, next);
        prev_match = (mask & (1 << 15)) != 0;
        i += 16;
    }
    count + generic::count_min_class_runs_scalar(bytes, i, 2, class, prev_match)
}

unsafe fn count_fixed_class_runs_neon(
    bytes: &[u8],
    start_at: usize,
    len: usize,
    class: ByteClass,
) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let mut run = 0usize;
    let mut mask_bytes = [0u8; 16];
    while i + 16 <= bytes.len() {
        let chunk = unsafe { vld1q_u8(bytes.as_ptr().add(i)) };
        let mask = unsafe { class_mask(chunk, class, &mut mask_bytes) };
        count += generic::add_fixed_mask_runs(mask, 16, len, &mut run);
        i += 16;
    }
    count + generic::count_fixed_class_runs_scalar(bytes, i, len, class, run)
}

unsafe fn count_ascii_case_error_neon(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    let lower_e = unsafe { vdupq_n_u8(b'e') };
    let upper_e = unsafe { vdupq_n_u8(b'E') };
    let mut mask_bytes = [0u8; 16];
    while i + 16 <= bytes.len() {
        let chunk = unsafe { vld1q_u8(bytes.as_ptr().add(i)) };
        let lower = unsafe { vceqq_u8(chunk, lower_e) };
        let upper = unsafe { vceqq_u8(chunk, upper_e) };
        let mut mask = unsafe { mask_from_vec(vorrq_u8(lower, upper), &mut mask_bytes) };
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

unsafe fn class_mask(chunk: uint8x16_t, class: ByteClass, mask_bytes: &mut [u8; 16]) -> u32 {
    match class {
        ByteClass::Digit => unsafe { range_mask(chunk, b'0', b'9', mask_bytes) },
        ByteClass::Word => {
            let alpha = unsafe { alpha_vec(chunk) };
            let digit = unsafe { range_vec(chunk, b'0', b'9') };
            let underscore = unsafe { vceqq_u8(chunk, vdupq_n_u8(b'_')) };
            unsafe { mask_from_vec(vorrq_u8(vorrq_u8(alpha, digit), underscore), mask_bytes) }
        }
        ByteClass::AlphaUnderscore => {
            let alpha = unsafe { alpha_vec(chunk) };
            let underscore = unsafe { vceqq_u8(chunk, vdupq_n_u8(b'_')) };
            unsafe { mask_from_vec(vorrq_u8(alpha, underscore), mask_bytes) }
        }
    }
}

unsafe fn alpha_vec(chunk: uint8x16_t) -> uint8x16_t {
    unsafe { vorrq_u8(range_vec(chunk, b'a', b'z'), range_vec(chunk, b'A', b'Z')) }
}

unsafe fn range_vec(chunk: uint8x16_t, lo: u8, hi: u8) -> uint8x16_t {
    unsafe {
        vandq_u8(
            vcgeq_u8(chunk, vdupq_n_u8(lo)),
            vcleq_u8(chunk, vdupq_n_u8(hi)),
        )
    }
}

unsafe fn range_mask(chunk: uint8x16_t, lo: u8, hi: u8, mask_bytes: &mut [u8; 16]) -> u32 {
    unsafe { mask_from_vec(range_vec(chunk, lo, hi), mask_bytes) }
}

unsafe fn mask_from_vec(vec: uint8x16_t, mask_bytes: &mut [u8; 16]) -> u32 {
    unsafe { vst1q_u8(mask_bytes.as_mut_ptr(), vec) };
    let mut mask = 0u32;
    for (bit, &hit) in mask_bytes.iter().enumerate() {
        if hit != 0 {
            mask |= 1 << bit;
        }
    }
    mask
}
