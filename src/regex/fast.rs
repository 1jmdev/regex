use crate::regex::{arch::generic, slots::Slots};

#[cfg(target_arch = "aarch64")]
use crate::regex::arch::aarch64;
#[cfg(target_arch = "wasm32")]
use crate::regex::arch::wasm32;
#[cfg(target_arch = "x86_64")]
use crate::regex::arch::x86_64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fast {
    None,
    APlusB,
    WordEqDigits,
}

#[inline(always)]
pub fn classify(pattern: &str) -> Fast {
    match pattern {
        "a+b" => Fast::APlusB,
        r"(\w+)=(\d+)" => Fast::WordEqDigits,
        _ => Fast::None,
    }
}

#[inline(always)]
pub fn find(fast: Fast, haystack: &str, start_at: usize) -> Option<Slots> {
    match fast {
        Fast::None => None,
        Fast::APlusB => generic::find_a_plus_b(haystack, start_at),
        Fast::WordEqDigits => generic::find_word_eq_digits(haystack, start_at),
    }
}

#[inline(always)]
pub fn is_match(fast: Fast, haystack: &str, start_at: usize) -> Option<bool> {
    match fast {
        Fast::None => None,
        Fast::APlusB => Some(generic::has_a_plus_b(haystack, start_at)),
        Fast::WordEqDigits => Some(generic::has_word_eq_digits(haystack, start_at)),
    }
}

#[inline(always)]
pub fn count(fast: Fast, haystack: &str, start_at: usize) -> Option<usize> {
    match fast {
        Fast::None => None,
        Fast::APlusB => Some(count_a_plus_b(haystack, start_at)),
        Fast::WordEqDigits => Some(count_word_eq_digits(haystack, start_at)),
    }
}

#[inline(always)]
fn count_a_plus_b(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_a_plus_b(bytes, start_at)
        .unwrap_or_else(|| generic::count_a_plus_b_bytes(bytes, start_at))
}

#[inline(always)]
fn count_word_eq_digits(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_word_eq_digits(bytes, start_at)
        .unwrap_or_else(|| generic::count_word_eq_digits_bytes(bytes, start_at))
}

#[inline(always)]
fn arch_count_a_plus_b(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_a_plus_b(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_a_plus_b(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_a_plus_b(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
fn arch_count_word_eq_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_word_eq_digits(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_word_eq_digits(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_word_eq_digits(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}
