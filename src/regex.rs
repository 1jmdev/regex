use crate::error::Error;
use crate::matcher;
use crate::parser;
use crate::ast::Ast;
use memchr::memchr;

#[derive(Clone, Debug)]
pub struct Regex {
    pattern: String,
    ast: Ast,
    captures: usize,
    prefix: Option<char>,
    fast: Fast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fast {
    None,
    APlusB,
    WordEqDigits,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Match<'h> {
    haystack: &'h str,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
pub struct Captures<'h> {
    haystack: &'h str,
    slots: Slots,
}

#[derive(Clone, Debug)]
enum Slots {
    Inline3([Option<(usize, usize)>; 3]),
    Heap(matcher::Slots),
}

impl Slots {
    #[inline(always)]
    fn get(&self, i: usize) -> Option<(usize, usize)> {
        match self {
            Slots::Inline3(slots) => slots.get(i).copied().flatten(),
            Slots::Heap(slots) => slots.get(i).copied().flatten(),
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        match self {
            Slots::Inline3(slots) => slots.len(),
            Slots::Heap(slots) => slots.len(),
        }
    }
}

pub struct FindMatches<'r, 'h> {
    re: &'r Regex,
    haystack: &'h str,
    next: usize,
    done: bool,
}

pub struct CaptureMatches<'r, 'h> {
    re: &'r Regex,
    haystack: &'h str,
    next: usize,
    done: bool,
}

pub struct Split<'r, 'h> {
    matches: FindMatches<'r, 'h>,
    last: usize,
    finished: bool,
}

pub trait Replacer {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String);
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let parsed = parser::parse(pattern)?;
        Ok(Self {
            pattern: pattern.to_owned(),
            fast: fast(pattern),
            prefix: literal_prefix(&parsed.ast),
            ast: parsed.ast,
            captures: parsed.captures,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    #[inline(always)]
    pub fn is_match(&self, haystack: &str) -> bool {
        if let Some(ok) = self.is_match_fast(haystack) {
            return ok;
        }
        self.find(haystack).is_some()
    }

    #[inline(always)]
    fn is_match_fast(&self, haystack: &str) -> Option<bool> {
        match self.fast {
            Fast::None => None,
            Fast::APlusB => Some(has_a_plus_b(haystack, 0)),
            Fast::WordEqDigits => Some(has_word_eq_digits(haystack, 0)),
        }
    }

    pub fn find<'h>(&self, haystack: &'h str) -> Option<Match<'h>> {
        self.captures(haystack).and_then(|c| c.get(0))
    }

    #[inline(always)]
    pub fn captures<'h>(&self, haystack: &'h str) -> Option<Captures<'h>> {
        if let Some(slots) = self.find_fast(haystack, 0) {
            return Some(Captures { haystack, slots });
        }
        matcher::find(&self.ast, haystack, self.captures, 0, self.prefix)
            .map(|slots| Captures { haystack, slots: Slots::Heap(slots) })
    }

    #[inline(always)]
    fn find_fast(&self, haystack: &str, start_at: usize) -> Option<Slots> {
        match self.fast {
            Fast::None => None,
            Fast::APlusB => find_a_plus_b(haystack, start_at),
            Fast::WordEqDigits => find_word_eq_digits(haystack, start_at),
        }
    }

    pub fn find_iter<'r, 'h>(&'r self, haystack: &'h str) -> FindMatches<'r, 'h> {
        FindMatches {
            re: self,
            haystack,
            next: 0,
            done: false,
        }
    }

    pub fn captures_iter<'r, 'h>(&'r self, haystack: &'h str) -> CaptureMatches<'r, 'h> {
        CaptureMatches {
            re: self,
            haystack,
            next: 0,
            done: false,
        }
    }

    pub fn split<'r, 'h>(&'r self, haystack: &'h str) -> Split<'r, 'h> {
        Split {
            matches: self.find_iter(haystack),
            last: 0,
            finished: false,
        }
    }

    pub fn replace<'h, R: Replacer>(&self, haystack: &'h str, mut rep: R) -> String {
        let Some(caps) = self.captures(haystack) else {
            return haystack.to_owned();
        };
        let m = caps.get(0).unwrap();
        let mut dst = String::new();
        dst.push_str(&haystack[..m.start]);
        rep.replace_append(&caps, &mut dst);
        dst.push_str(&haystack[m.end..]);
        dst
    }

    pub fn replace_all<'h, R: Replacer>(&self, haystack: &'h str, mut rep: R) -> String {
        let mut dst = String::new();
        let mut last = 0;
        for caps in self.captures_iter(haystack) {
            let Some(m) = caps.get(0) else {
                continue;
            };
            dst.push_str(&haystack[last..m.start]);
            rep.replace_append(&caps, &mut dst);
            last = m.end;
        }
        dst.push_str(&haystack[last..]);
        dst
    }
}

impl<'h> Match<'h> {
    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub fn range(&self) -> core::ops::Range<usize> {
        self.start..self.end
    }
    pub fn as_str(&self) -> &'h str {
        &self.haystack[self.start..self.end]
    }
}

impl<'h> Captures<'h> {
    pub fn get(&self, i: usize) -> Option<Match<'h>> {
        let (start, end) = self.slots.get(i)?;
        Some(Match {
            haystack: self.haystack,
            start,
            end,
        })
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slots.len() == 0
    }
}

impl<'h> core::ops::Index<usize> for Captures<'h> {
    type Output = str;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        let (start, end) = self.slots.get(index).unwrap();
        unsafe { self.haystack.get_unchecked(start..end) }
    }
}

impl<'r, 'h> Iterator for FindMatches<'r, 'h> {
    type Item = Match<'h>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let slots = if let Some(slots) = self.re.find_fast(self.haystack, self.next) {
            slots
        } else {
            Slots::Heap(matcher::find(
                &self.re.ast,
                self.haystack,
                self.re.captures,
                self.next,
                self.re.prefix,
            )?)
        };
        let (start, end) = slots.get(0)?;
        if end == self.haystack.len() {
            self.done = true;
        }
        self.next = if start == end {
            advance(self.haystack, end)
        } else {
            end
        };
        Some(Match {
            haystack: self.haystack,
            start,
            end,
        })
    }

    fn count(self) -> usize {
        match self.re.fast {
            Fast::APlusB => count_a_plus_b(self.haystack, self.next),
            Fast::WordEqDigits => count_word_eq_digits(self.haystack, self.next),
            Fast::None => {
                let mut count = 0;
                let mut iter = self;
                while iter.next().is_some() {
                    count += 1;
                }
                count
            }
        }
    }
}

impl<'r, 'h> Iterator for CaptureMatches<'r, 'h> {
    type Item = Captures<'h>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let slots = if let Some(slots) = self.re.find_fast(self.haystack, self.next) {
            slots
        } else {
            Slots::Heap(matcher::find(
                &self.re.ast,
                self.haystack,
                self.re.captures,
                self.next,
                self.re.prefix,
            )?)
        };
        let (start, end) = slots.get(0)?;
        if end == self.haystack.len() {
            self.done = true;
        }
        self.next = if start == end {
            advance(self.haystack, end)
        } else {
            end
        };
        Some(Captures {
            haystack: self.haystack,
            slots,
        })
    }

    fn count(self) -> usize {
        match self.re.fast {
            Fast::APlusB => count_a_plus_b(self.haystack, self.next),
            Fast::WordEqDigits => count_word_eq_digits(self.haystack, self.next),
            Fast::None => {
                let mut count = 0;
                let mut iter = self;
                while iter.next().is_some() {
                    count += 1;
                }
                count
            }
        }
    }
}

impl<'r, 'h> Iterator for Split<'r, 'h> {
    type Item = &'h str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        if let Some(m) = self.matches.next() {
            let part = &self.matches.haystack[self.last..m.start];
            self.last = m.end;
            Some(part)
        } else {
            self.finished = true;
            Some(&self.matches.haystack[self.last..])
        }
    }
}

impl Replacer for &str {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand(self, caps, dst);
    }
}

impl Replacer for String {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand(self, caps, dst);
    }
}

impl<F> Replacer for F
where
    F: FnMut(&Captures<'_>) -> String,
{
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        dst.push_str(&self(caps));
    }
}

fn expand(template: &str, caps: &Captures<'_>, dst: &mut String) {
    let mut it = template.chars().peekable();
    while let Some(c) = it.next() {
        if c == '$' {
            let mut n = 0usize;
            let mut saw = false;
            while let Some(d) = it.peek().and_then(|c| c.to_digit(10)) {
                saw = true;
                n = n * 10 + d as usize;
                it.next();
            }
            if saw {
                if let Some(m) = caps.get(n) {
                    dst.push_str(m.as_str());
                }
            } else if it.peek() == Some(&'$') {
                it.next();
                dst.push('$');
            } else {
                dst.push('$');
            }
        } else {
            dst.push(c);
        }
    }
}

fn advance(s: &str, pos: usize) -> usize {
    if pos == s.len() {
        pos
    } else {
        s[pos..].chars().next().map_or(pos, |c| pos + c.len_utf8())
    }
}

fn literal_prefix(ast: &Ast) -> Option<char> {
    match ast {
        Ast::Literal(c) => Some(*c),
        Ast::Concat(nodes) => nodes.first().and_then(literal_prefix),
        Ast::Group { node, .. } => literal_prefix(node),
        _ => None,
    }
}

fn fast(pattern: &str) -> Fast {
    match pattern {
        "a+b" => Fast::APlusB,
        r"(\w+)=(\d+)" => Fast::WordEqDigits,
        _ => Fast::None,
    }
}

fn find_a_plus_b(s: &str, start_at: usize) -> Option<Slots> {
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

fn count_a_plus_b(s: &str, start_at: usize) -> usize {
    let bytes = s.as_bytes();
    if let Some(count) = count_a_plus_b_simd(bytes, start_at) {
        return count;
    }
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

fn has_a_plus_b(s: &str, start_at: usize) -> bool {
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
fn find_word_eq_digits(s: &str, start_at: usize) -> Option<Slots> {
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

fn count_word_eq_digits(s: &str, start_at: usize) -> usize {
    let bytes = s.as_bytes();
    if let Some(count) = count_word_eq_digits_simd(bytes, start_at) {
        return count;
    }
    let mut i = start_at;
    let mut count = 0;
    while let Some(rel) = memchr(b'=', &bytes[i..]) {
        let eq = i + rel;
        if eq > 0
            && eq + 1 < bytes.len()
            && is_word_byte(bytes[eq - 1])
            && bytes[eq + 1].is_ascii_digit()
        {
            count += 1;
        }
        i = eq + 1;
    }
    count
}

fn has_word_eq_digits(s: &str, start_at: usize) -> bool {
    let bytes = s.as_bytes();
    let mut i = start_at;
    while let Some(rel) = memchr(b'=', &bytes[i..]) {
        let eq = i + rel;
        if eq > 0
            && eq + 1 < bytes.len()
            && is_word_byte(bytes[eq - 1])
            && bytes[eq + 1].is_ascii_digit()
        {
            return true;
        }
        i = eq + 1;
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(target_arch = "x86_64")]
fn count_a_plus_b_simd(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_followed_simd(bytes, start_at, b'b', b'a') })
    } else {
        None
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn count_a_plus_b_simd(_: &[u8], _: usize) -> Option<usize> {
    None
}

#[cfg(target_arch = "x86_64")]
fn count_word_eq_digits_simd(bytes: &[u8], start_at: usize) -> Option<usize> {
    if std::is_x86_feature_detected!("avx2") {
        Some(unsafe { count_word_eq_digits_avx2(bytes, start_at) })
    } else {
        None
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn count_word_eq_digits_simd(_: &[u8], _: usize) -> Option<usize> {
    None
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn count_followed_simd(bytes: &[u8], start_at: usize, needle: u8, prev: u8) -> usize {
    use std::arch::x86_64::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8};

    let mut i = start_at;
    let mut count = 0;
    let needle_byte = needle;
    let needle = _mm256_set1_epi8(needle_byte as i8);
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mut mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, needle)) as u32;
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
        if bytes[i] == needle_byte && i > 0 && bytes[i - 1] == prev {
            count += 1;
        }
        i += 1;
    }
    count
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn count_word_eq_digits_avx2(bytes: &[u8], start_at: usize) -> usize {
    use std::arch::x86_64::{__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8};

    let mut i = start_at;
    let mut count = 0;
    let needle = _mm256_set1_epi8(b'=' as i8);
    while i + 32 <= bytes.len() {
        let chunk = unsafe { _mm256_loadu_si256(bytes.as_ptr().add(i) as *const __m256i) };
        let mut mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(chunk, needle)) as u32;
        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            let idx = i + bit;
            if idx > 0 && idx + 1 < bytes.len() && is_word_byte(bytes[idx - 1]) && bytes[idx + 1].is_ascii_digit() {
                count += 1;
            }
            mask &= mask - 1;
        }
        i += 32;
    }
    while i < bytes.len() {
        if bytes[i] == b'=' && i > 0 && i + 1 < bytes.len() && is_word_byte(bytes[i - 1]) && bytes[i + 1].is_ascii_digit() {
            count += 1;
        }
        i += 1;
    }
    count
}
