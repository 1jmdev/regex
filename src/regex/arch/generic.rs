use crate::regex::slots::Slots;
use memchr::memchr;

#[derive(Clone, Copy)]
pub enum ByteClass {
    Digit,
    Word,
    AlphaUnderscore,
}

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

#[inline(always)]
pub fn class_matches_byte(b: u8, class: ByteClass) -> bool {
    match class {
        ByteClass::Digit => b.is_ascii_digit(),
        ByteClass::Word => is_word_byte(b),
        ByteClass::AlphaUnderscore => is_alpha_underscore_byte(b),
    }
}

#[inline(always)]
pub fn find_digits(s: &str, start_at: usize) -> Option<Slots> {
    find_run(s.as_bytes(), start_at, |b| b.is_ascii_digit())
}

#[inline(always)]
pub fn find_words(s: &str, start_at: usize) -> Option<Slots> {
    find_run(s.as_bytes(), start_at, is_word_byte)
}

#[inline(always)]
pub fn find_alpha_underscore(s: &str, start_at: usize) -> Option<Slots> {
    find_run(s.as_bytes(), start_at, is_alpha_underscore_byte)
}

#[inline(always)]
pub fn find_four_digits(s: &str, start_at: usize) -> Option<Slots> {
    find_fixed_run(s.as_bytes(), start_at, 4, |b| b.is_ascii_digit())
}

#[inline(always)]
pub fn find_words_min2(s: &str, start_at: usize) -> Option<Slots> {
    find_min_run(s.as_bytes(), start_at, 2, is_word_byte)
}

#[inline(always)]
pub fn find_ascii_case_error(s: &str, start_at: usize) -> Option<Slots> {
    let bytes = s.as_bytes();
    let mut i = start_at;
    while i + 5 <= bytes.len() {
        let rel1 = memchr(b'e', &bytes[i..]);
        let rel2 = memchr(b'E', &bytes[i..]);
        let rel = match (rel1, rel2) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return None,
        };
        i += rel;
        if i + 5 <= bytes.len() && eq_ignore_ascii_case(&bytes[i..i + 5], b"error") {
            return Some(Slots::Inline3([Some((i, i + 5)), None, None]));
        }
        i += 1;
    }
    None
}

#[inline(always)]
pub fn count_digits_bytes(bytes: &[u8], start_at: usize) -> usize {
    count_runs(bytes, start_at, |b| b.is_ascii_digit())
}

#[inline(always)]
pub fn count_words_bytes(bytes: &[u8], start_at: usize) -> usize {
    count_runs(bytes, start_at, is_word_byte)
}

#[inline(always)]
pub fn count_alpha_underscore_bytes(bytes: &[u8], start_at: usize) -> usize {
    count_runs(bytes, start_at, is_alpha_underscore_byte)
}

#[inline(always)]
pub fn count_four_digits_bytes(bytes: &[u8], start_at: usize) -> usize {
    count_fixed_runs(bytes, start_at, 4, |b| b.is_ascii_digit())
}

#[inline(always)]
pub fn count_words_min2_bytes(bytes: &[u8], start_at: usize) -> usize {
    count_min_runs(bytes, start_at, 2, is_word_byte)
}

#[inline(always)]
pub fn count_ascii_case_error_bytes(bytes: &[u8], start_at: usize) -> usize {
    let mut i = start_at;
    let mut count = 0;
    while i + 5 <= bytes.len() {
        let rel1 = memchr(b'e', &bytes[i..]);
        let rel2 = memchr(b'E', &bytes[i..]);
        let rel = match (rel1, rel2) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return count,
        };
        i += rel;
        if i + 5 <= bytes.len() && eq_ignore_ascii_case(&bytes[i..i + 5], b"error") {
            count += 1;
            i += 5;
        } else {
            i += 1;
        }
    }
    count
}

#[inline(always)]
fn find_run(bytes: &[u8], start_at: usize, pred: impl Fn(u8) -> bool) -> Option<Slots> {
    let mut i = start_at;
    while i < bytes.len() {
        while i < bytes.len() && !pred(bytes[i]) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && pred(bytes[i]) {
            i += 1;
        }
        if start < i {
            return Some(Slots::Inline3([Some((start, i)), None, None]));
        }
    }
    None
}

#[inline(always)]
fn find_min_run(
    bytes: &[u8],
    start_at: usize,
    min: usize,
    pred: impl Fn(u8) -> bool,
) -> Option<Slots> {
    let mut i = start_at;
    while i < bytes.len() {
        while i < bytes.len() && !pred(bytes[i]) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && pred(bytes[i]) {
            i += 1;
        }
        if i - start >= min {
            return Some(Slots::Inline3([Some((start, i)), None, None]));
        }
    }
    None
}

#[inline(always)]
fn find_fixed_run(
    bytes: &[u8],
    start_at: usize,
    len: usize,
    pred: impl Fn(u8) -> bool,
) -> Option<Slots> {
    let mut i = start_at;
    while i + len <= bytes.len() {
        while i < bytes.len() && !pred(bytes[i]) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && pred(bytes[i]) {
            i += 1;
        }
        if i - start >= len {
            return Some(Slots::Inline3([Some((start, start + len)), None, None]));
        }
    }
    None
}

#[inline(always)]
fn count_runs(bytes: &[u8], start_at: usize, pred: impl Fn(u8) -> bool) -> usize {
    count_min_runs(bytes, start_at, 1, pred)
}

#[inline(always)]
fn count_min_runs(bytes: &[u8], start_at: usize, min: usize, pred: impl Fn(u8) -> bool) -> usize {
    let mut i = start_at;
    let mut count = 0;
    while i < bytes.len() {
        while i < bytes.len() && !pred(bytes[i]) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && pred(bytes[i]) {
            i += 1;
        }
        if i - start >= min {
            count += 1;
        }
    }
    count
}

#[inline(always)]
fn count_fixed_runs(bytes: &[u8], start_at: usize, len: usize, pred: impl Fn(u8) -> bool) -> usize {
    let mut i = start_at;
    let mut count = 0;
    while i < bytes.len() {
        while i < bytes.len() && !pred(bytes[i]) {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && pred(bytes[i]) {
            i += 1;
        }
        count += (i - start) / len;
    }
    count
}

#[inline(always)]
pub fn count_class_runs_scalar(
    bytes: &[u8],
    start_at: usize,
    class: ByteClass,
    mut prev: bool,
) -> usize {
    let mut count = 0;
    for &byte in &bytes[start_at..] {
        let hit = class_matches_byte(byte, class);
        if hit && !prev {
            count += 1;
        }
        prev = hit;
    }
    count
}

#[inline(always)]
pub fn count_min_class_runs_scalar(
    bytes: &[u8],
    start_at: usize,
    min: usize,
    class: ByteClass,
    mut prev: bool,
) -> usize {
    let mut count = 0;
    let mut i = start_at;
    while i < bytes.len() {
        if !class_matches_byte(bytes[i], class) {
            prev = false;
            i += 1;
            continue;
        }
        if prev {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && class_matches_byte(bytes[i], class) {
            i += 1;
        }
        if i - start >= min {
            count += 1;
        }
        prev = true;
    }
    count
}

#[inline(always)]
pub fn count_fixed_class_runs_scalar(
    bytes: &[u8],
    start_at: usize,
    len: usize,
    class: ByteClass,
    mut run: usize,
) -> usize {
    let mut count = 0;
    for &byte in &bytes[start_at..] {
        if class_matches_byte(byte, class) {
            run += 1;
        } else {
            count += run / len;
            run = 0;
        }
    }
    count + run / len
}

#[inline(always)]
pub fn count_mask_run_starts(mask: u32, width: u32, prev_match: bool) -> usize {
    let prev = if prev_match { 1 } else { 0 };
    let used = if width == 32 {
        u32::MAX
    } else {
        (1u32 << width) - 1
    };
    (mask & used & !((mask << 1) | prev)).count_ones() as usize
}

#[inline(always)]
pub fn count_mask_min2_run_starts(
    mask: u32,
    width: u32,
    prev_match: bool,
    next_match: bool,
) -> usize {
    let prev = if prev_match { 1 } else { 0 };
    let used = if width == 32 {
        u32::MAX
    } else {
        (1u32 << width) - 1
    };
    let next = if next_match { 1 << (width - 1) } else { 0 };
    (mask & used & !((mask << 1) | prev) & ((mask >> 1) | next)).count_ones() as usize
}

#[inline(always)]
pub fn add_fixed_mask_runs(mask: u32, width: u32, len: usize, run: &mut usize) -> usize {
    let used = if width == 32 {
        u32::MAX
    } else {
        (1u32 << width) - 1
    };
    let mask = mask & used;
    if mask == used {
        *run += width as usize;
        return 0;
    }
    if mask == 0 {
        let count = *run / len;
        *run = 0;
        return count;
    }

    let mut count = 0;
    for bit in 0..width {
        if (mask & (1 << bit)) != 0 {
            *run += 1;
        } else {
            count += *run / len;
            *run = 0;
        }
    }
    count
}

#[inline(always)]
fn is_alpha_underscore_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

#[inline(always)]
pub fn eq_ascii_case_error_at(bytes: &[u8], i: usize) -> bool {
    bytes[i].eq_ignore_ascii_case(&b'e')
        && bytes[i + 1].eq_ignore_ascii_case(&b'r')
        && bytes[i + 2].eq_ignore_ascii_case(&b'r')
        && bytes[i + 3].eq_ignore_ascii_case(&b'o')
        && bytes[i + 4].eq_ignore_ascii_case(&b'r')
}

#[inline(always)]
fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
}
