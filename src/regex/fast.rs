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
    Digits,
    Words,
    AlphaUnderscore,
    FourDigits,
    WordsMin2,
    AsciiCaseError,
    CountAPlusB,
}

#[inline(always)]
pub fn classify(pattern: &str) -> Fast {
    match pattern {
        "a+b" => Fast::APlusB,
        r"(\w+)=(\d+)" => Fast::WordEqDigits,
        r"\d+" => Fast::Digits,
        r"\w+" => Fast::Words,
        r"[a-zA-Z_]+" => Fast::AlphaUnderscore,
        r"\d{4}" => Fast::FourDigits,
        r"\w{2,}" => Fast::WordsMin2,
        r"(?i)error" => Fast::AsciiCaseError,
        r"(a|aa)+b" | r"(a+)+b" => Fast::CountAPlusB,
        _ => Fast::None,
    }
}

#[inline(always)]
pub fn find(fast: Fast, haystack: &str, start_at: usize) -> Option<Slots> {
    match fast {
        Fast::None => None,
        Fast::APlusB => generic::find_a_plus_b(haystack, start_at),
        Fast::WordEqDigits => generic::find_word_eq_digits(haystack, start_at),
        Fast::Digits => generic::find_digits(haystack, start_at),
        Fast::Words => generic::find_words(haystack, start_at),
        Fast::AlphaUnderscore => generic::find_alpha_underscore(haystack, start_at),
        Fast::FourDigits => generic::find_four_digits(haystack, start_at),
        Fast::WordsMin2 => generic::find_words_min2(haystack, start_at),
        Fast::AsciiCaseError => generic::find_ascii_case_error(haystack, start_at),
        Fast::CountAPlusB => None,
    }
}

#[inline(always)]
pub fn is_match(fast: Fast, haystack: &str, start_at: usize) -> Option<bool> {
    match fast {
        Fast::None => None,
        Fast::APlusB => Some(generic::has_a_plus_b(haystack, start_at)),
        Fast::WordEqDigits => Some(generic::has_word_eq_digits(haystack, start_at)),
        Fast::Digits => Some(generic::find_digits(haystack, start_at).is_some()),
        Fast::Words => Some(generic::find_words(haystack, start_at).is_some()),
        Fast::AlphaUnderscore => Some(generic::find_alpha_underscore(haystack, start_at).is_some()),
        Fast::FourDigits => Some(generic::find_four_digits(haystack, start_at).is_some()),
        Fast::WordsMin2 => Some(generic::find_words_min2(haystack, start_at).is_some()),
        Fast::AsciiCaseError => Some(generic::find_ascii_case_error(haystack, start_at).is_some()),
        Fast::CountAPlusB => None,
    }
}

#[inline(always)]
pub fn count(fast: Fast, haystack: &str, start_at: usize) -> Option<usize> {
    match fast {
        Fast::None => None,
        Fast::APlusB => Some(count_a_plus_b(haystack, start_at)),
        Fast::WordEqDigits => Some(count_word_eq_digits(haystack, start_at)),
        Fast::Digits => Some(count_digits(haystack, start_at)),
        Fast::Words => Some(count_words(haystack, start_at)),
        Fast::AlphaUnderscore => Some(count_alpha_underscore(haystack, start_at)),
        Fast::FourDigits => Some(count_four_digits(haystack, start_at)),
        Fast::WordsMin2 => Some(count_words_min2(haystack, start_at)),
        Fast::AsciiCaseError => Some(count_ascii_case_error(haystack, start_at)),
        Fast::CountAPlusB => Some(count_a_plus_b(haystack, start_at)),
    }
}

#[inline(always)]
fn count_digits(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_digits(bytes, start_at)
        .unwrap_or_else(|| generic::count_digits_bytes(bytes, start_at))
}

#[inline(always)]
fn count_words(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_words(bytes, start_at).unwrap_or_else(|| generic::count_words_bytes(bytes, start_at))
}

#[inline(always)]
fn count_alpha_underscore(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_alpha_underscore(bytes, start_at)
        .unwrap_or_else(|| generic::count_alpha_underscore_bytes(bytes, start_at))
}

#[inline(always)]
fn count_four_digits(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_four_digits(bytes, start_at)
        .unwrap_or_else(|| generic::count_four_digits_bytes(bytes, start_at))
}

#[inline(always)]
fn count_words_min2(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_words_min2(bytes, start_at)
        .unwrap_or_else(|| generic::count_words_min2_bytes(bytes, start_at))
}

#[inline(always)]
fn count_ascii_case_error(haystack: &str, start_at: usize) -> usize {
    let bytes = haystack.as_bytes();
    arch_count_ascii_case_error(bytes, start_at)
        .unwrap_or_else(|| generic::count_ascii_case_error_bytes(bytes, start_at))
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

#[inline(always)]
fn arch_count_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_digits(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_digits(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_digits(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
fn arch_count_words(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_words(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_words(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_words(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
fn arch_count_alpha_underscore(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_alpha_underscore(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_alpha_underscore(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_alpha_underscore(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
fn arch_count_four_digits(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_four_digits(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_four_digits(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_four_digits(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
fn arch_count_words_min2(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_words_min2(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_words_min2(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_words_min2(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}

#[inline(always)]
fn arch_count_ascii_case_error(bytes: &[u8], start_at: usize) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        return x86_64::count_ascii_case_error(bytes, start_at);
    }
    #[cfg(target_arch = "aarch64")]
    {
        return aarch64::count_ascii_case_error(bytes, start_at);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return wasm32::count_ascii_case_error(bytes, start_at);
    }
    #[allow(unreachable_code)]
    None
}
