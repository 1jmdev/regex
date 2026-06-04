use crate::regex::slots::Slots;
use memchr::memchr;

#[inline(always)]
pub fn find_a_plus_b(s: &str, start_at: usize) -> Option<Slots> {
    let bytes = s.as_bytes();
    let mut i = start_at;
    while i < bytes.len() {
        let rel = memchr(b'a', &bytes[i..])?;
        let start = i + rel;
        let mut end = start + 1;
        while end < bytes.len() && bytes[end] == b'a' {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'b' {
            return Some(Slots::Inline3([Some((start, end + 1)), None, None]));
        }
        i = end + 1;
    }
    None
}

#[inline(always)]
pub fn has_a_plus_b(s: &str, start_at: usize) -> bool {
    let bytes = s.as_bytes();
    let mut i = start_at;
    while let Some(rel) = memchr(b'b', &bytes[i..]) {
        let b = i + rel;
        if b > 0 && bytes[b - 1] == b'a' {
            return true;
        }
        i = b + 1;
    }
    false
}

#[inline(always)]
pub fn count_a_plus_b_bytes(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    while let Some(rel) = memchr(b'b', &bytes[i..]) {
        let b = i + rel;
        if b > 0 && bytes[b - 1] == b'a' {
            count += 1;
        }
        i = b + 1;
    }
    count
}

#[inline(always)]
pub fn find_word_eq_digits(s: &str, start_at: usize) -> Option<Slots> {
    let bytes = s.as_bytes();
    let mut i = start_at;
    while i < bytes.len() {
        while i < bytes.len() && !is_word_byte(bytes[i]) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && is_word_byte(bytes[i]) {
            i += 1;
        }
        if start == i || i >= bytes.len() || bytes[i] != b'=' {
            i = i.saturating_add(1);
            continue;
        }
        let digits_start = i + 1;
        let mut end = digits_start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end > digits_start {
            return Some(Slots::Inline3([
                Some((start, end)),
                Some((start, i)),
                Some((digits_start, end)),
            ]));
        }
        i = digits_start;
    }
    None
}

#[inline(always)]
pub fn has_word_eq_digits(s: &str, start_at: usize) -> bool {
    let bytes = s.as_bytes();
    let mut i = start_at;
    while let Some(rel) = memchr(b'=', &bytes[i..]) {
        let eq = i + rel;
        if is_word_eq_digit_at(bytes, eq) {
            return true;
        }
        i = eq + 1;
    }
    false
}

#[inline(always)]
pub fn count_word_eq_digits_bytes(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    while let Some(rel) = memchr(b'=', &bytes[i..]) {
        let eq = i + rel;
        if is_word_eq_digit_at(bytes, eq) {
            count += 1;
        }
        i = eq + 1;
    }
    count
}

#[inline(always)]
pub fn is_word_eq_digit_at(bytes: &[u8], eq: usize) -> bool {
    eq > 0 && eq + 1 < bytes.len() && is_word_byte(bytes[eq - 1]) && bytes[eq + 1].is_ascii_digit()
}

#[inline(always)]
pub fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
