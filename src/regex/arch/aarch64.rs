use super::generic;

#[cfg(target_feature = "neon")]
use std::arch::aarch64::{vceqq_u8, vdupq_n_u8, vld1q_u8, vst1q_u8};

#[inline(always)]
pub fn count_a_plus_b(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_feature = "neon")]
    {
        return Some(unsafe { count_followed_neon(bytes, start_at, b'b', b'a') });
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
pub fn count_word_eq_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_feature = "neon")]
    {
        return Some(unsafe { count_word_eq_digits_neon(bytes, start_at) });
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(target_feature = "neon")]
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

#[cfg(target_feature = "neon")]
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
