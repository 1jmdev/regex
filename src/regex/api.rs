use crate::{
    ast::Ast,
    error::Error,
    matcher, parser,
    regex::{
        fast::{self, Fast},
        slots::Slots,
    },
};
use core::ops::Index;
use std::{borrow::Cow, fmt};

/// A compiled regular expression for searching Unicode string haystacks.
///
/// A `Regex` is created from a pattern string and can be used to search
/// haystacks, iterate over all matches, extract capture groups, split
/// haystacks into substrings, or replace matched substrings.
///
/// Searching is unanchored by default — the regex is tried at every position
/// in the haystack. To force a match at the start or end use `^` / `$`.
///
/// ## Syntax
///
/// | Syntax          | Description                                      |
/// |-----------------|--------------------------------------------------|
/// | `.`             | Any character except `\n`                        |
/// | `^` / `$`       | Start / end of string                            |
/// | `\d` / `\D`     | ASCII digit / non-digit                          |
/// | `\w` / `\W`     | ASCII word character (`[a-zA-Z0-9_]`) / inverse  |
/// | `\s` / `\S`     | Whitespace / non-whitespace                      |
/// | `\b` / `\B`     | Word boundary / non-boundary                     |
/// | `[abc]`         | Character class                                  |
/// | `[^abc]`        | Negated character class                          |
/// | `[a-z]`         | Character range                                  |
/// | `*` / `+` / `?` | Zero-or-more / one-or-more / zero-or-one (greedy)|
/// | `{n}` / `{n,m}` | Exactly n / between n and m repetitions          |
/// | `*?` / `+?`     | Non-greedy variants                              |
/// | `(abc)`         | Capturing group                                  |
/// | `a\|b`          | Alternation                                      |
/// | `(?i)`          | Enable case-insensitive matching (ASCII only)    |
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();
/// assert!(re.is_match("today is 2024-01-15"));
/// ```
///
/// ## Example: capture groups
///
/// Capture groups let you extract individual parts of a match. Group `0` is
/// always the whole match; groups `1`, `2`, … correspond to the parentheses
/// in the pattern left-to-right.
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
/// let caps = re.captures("count=42 limit=100").unwrap();
/// assert_eq!(caps[1], *"count");
/// assert_eq!(caps[2], *"42");
/// ```
///
/// ## Example: replace with a closure
///
/// ```
/// use regex::{Regex, Captures};
///
/// let re = Regex::new(r"(\w+)").unwrap();
/// let result = re.replace("hello world", |caps: &Captures<'_>| {
///     caps[1].to_uppercase()
/// });
/// assert_eq!(result, "HELLO world");
/// ```
#[derive(Clone, Debug)]
pub struct Regex {
    pattern: String,
    ast: Ast,
    captures: usize,
    prefix: Option<char>,
    fast: Fast,
}

/// A configurable builder for compiling [`Regex`] values.
///
/// `RegexBuilder` lets you construct a regex while setting options before
/// compilation. Options that are not supported by this engine are accepted for
/// API compatibility and leave matching behavior unchanged.
///
/// ## Example
///
/// ```
/// use regex::RegexBuilder;
///
/// let re = RegexBuilder::new(r"abc")
///     .case_insensitive(true)
///     .build()
///     .unwrap();
///
/// assert!(re.is_match("ABC"));
/// ```
#[derive(Clone, Debug)]
pub struct RegexBuilder {
    pattern: String,
    case_insensitive: bool,
}

/// A single contiguous match within a haystack.
///
/// A `Match` stores a reference to the original haystack together with the
/// byte-offset range `[start, end)` of the match. You can obtain the matched
/// text with [`as_str`](Match::as_str) or the range with
/// [`range`](Match::range).
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"\d+").unwrap();
/// let m = re.find("foo 42 bar").unwrap();
/// assert_eq!(m.as_str(), "42");
/// assert_eq!(m.range(), 4..6);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Match<'h> {
    haystack: &'h str,
    start: usize,
    end: usize,
}

/// The capture groups produced by a single match of a [`Regex`].
///
/// Group `0` is always the whole match. Groups `1`, `2`, … correspond to the
/// parenthesised sub-expressions in the pattern, left-to-right.
///
/// Individual groups can be retrieved with [`get`](Captures::get) or indexed
/// with `caps[n]`. Indexing panics if the group did not participate in the
/// match.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
/// let caps = re.captures("date: 2024-01-15").unwrap();
/// assert_eq!(&caps[1], "2024");
/// assert_eq!(&caps[2], "01");
/// assert_eq!(&caps[3], "15");
/// ```
#[derive(Clone, Debug)]
pub struct Captures<'h> {
    haystack: &'h str,
    slots: Slots,
}

/// An iterator over all non-overlapping [`Match`]es in a haystack.
///
/// Created by [`Regex::find_iter`]. Yields successive [`Match`] values from
/// left to right. Zero-length matches advance by one character to avoid
/// infinite loops.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"\d+").unwrap();
/// let nums: Vec<&str> = re.find_iter("one 1 two 22 three 333").map(|m| m.as_str()).collect();
/// assert_eq!(nums, ["1", "22", "333"]);
/// ```
pub struct FindMatches<'r, 'h> {
    re: &'r Regex,
    haystack: &'h str,
    next: usize,
    done: bool,
}

/// An iterator over all non-overlapping [`Captures`] in a haystack.
///
/// Created by [`Regex::captures_iter`]. Yields successive [`Captures`] values
/// from left to right. Use this when you need access to capture groups for
/// every match; if only the match extents are needed prefer [`FindMatches`].
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
/// let pairs: Vec<(&str, &str)> = re
///     .captures_iter("x=1 y=22 z=333")
///     .map(|c| {
///         let h = c.get(0).unwrap().as_str();
///         // just return the whole match for brevity
///         (h, h)
///     })
///     .collect();
/// assert_eq!(pairs.len(), 3);
/// ```
pub struct CaptureMatches<'r, 'h> {
    re: &'r Regex,
    haystack: &'h str,
    next: usize,
    done: bool,
}

/// An iterator over substrings of a haystack split by a [`Regex`].
///
/// Created by [`Regex::split`]. Yields the parts of the haystack that lie
/// *between* successive matches. The delimiter matches themselves are not
/// included in the output.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"\s+").unwrap();
/// let words: Vec<&str> = re.split("one   two\tthree").collect();
/// assert_eq!(words, ["one", "two", "three"]);
/// ```
pub struct Split<'r, 'h> {
    matches: FindMatches<'r, 'h>,
    last: usize,
    finished: bool,
}

/// An iterator over at most `n` substrings of a haystack split by a [`Regex`].
///
/// Created by [`Regex::splitn`]. Yields the parts of the haystack that lie
/// between successive matches, stopping after the configured limit.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r",\s*").unwrap();
/// let fields: Vec<&str> = re.splitn("one, two, three", 2).collect();
/// assert_eq!(fields, ["one", "two, three"]);
/// ```
pub struct SplitN<'r, 'h> {
    splits: Split<'r, 'h>,
    limit: usize,
    count: usize,
}

/// An iterator over the capture group names in a [`Regex`].
///
/// Created by [`Regex::capture_names`]. Group `0` is always unnamed. This
/// engine does not currently support named capture syntax, so every item
/// yielded is `None`.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
/// let names: Vec<Option<&str>> = re.capture_names().collect();
/// assert_eq!(names, [None, None, None]);
/// ```
pub struct CaptureNames {
    remaining: usize,
}

/// A type that can produce replacement text for a [`Regex`] substitution.
///
/// Implement this trait to control how matched text is replaced in
/// [`Regex::replace`] and [`Regex::replace_all`]. The [`Captures`] argument
/// gives access to the full match and any capture groups.
///
/// Convenience implementations are provided for `&str`, `String`, and any
/// `FnMut(&Captures<'_>) -> String` closure.
///
/// In string replacements, `$1`, `$2`, … are expanded to the corresponding
/// capture group. Use `$$` to insert a literal `$`.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let re = Regex::new(r"(\w+)\s(\w+)").unwrap();
/// let result = re.replace("hello world", "$2 $1");
/// assert_eq!(result, "world hello");
/// ```
pub trait Replacer {
    /// Appends replacement text for `caps` to `dst`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String);
}

impl Regex {
    /// Compile `pattern` into a `Regex`.
    ///
    /// Returns an [`Error`] if the pattern contains invalid syntax.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert!(re.is_match("abc 123"));
    ///
    /// assert!(Regex::new(r"[unclosed").is_err());
    /// ```
    pub fn new(pattern: &str) -> Result<Self, Error> {
        let parsed = parser::parse(pattern)?;
        Ok(Self {
            pattern: pattern.to_owned(),
            fast: fast::classify(pattern),
            prefix: literal_prefix(&parsed.ast),
            ast: parsed.ast,
            captures: parsed.captures,
        })
    }

    /// Returns the original pattern string used to construct this `Regex`.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert_eq!(re.as_str(), r"\d+");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    /// Returns the number of capture groups in this regex.
    ///
    /// The count includes group `0`, which is always the entire match. Each
    /// explicit parenthesised group increments the count by one.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    /// assert_eq!(re.captures_len(), 3);
    /// ```
    pub fn captures_len(&self) -> usize {
        self.captures + 1
    }

    /// Returns an iterator over the capture group names in this regex.
    ///
    /// Group `0` is always unnamed. This engine does not currently support
    /// named capture syntax, so every item yielded is `None`.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    /// let names: Vec<Option<&str>> = re.capture_names().collect();
    /// assert_eq!(names, [None, None, None]);
    /// ```
    pub fn capture_names(&self) -> CaptureNames {
        CaptureNames {
            remaining: self.captures_len(),
        }
    }

    /// Returns the static number of captures for this regex, if it is known.
    ///
    /// This engine knows the capture count for every compiled regex, so this
    /// always returns `Some`.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    /// assert_eq!(re.static_captures_len(), Some(3));
    /// ```
    pub fn static_captures_len(&self) -> Option<usize> {
        Some(self.captures_len())
    }

    /// Returns `true` if and only if there is a match for the regex anywhere
    /// in the haystack.
    ///
    /// Prefer this over calling [`find`](Regex::find) and checking for
    /// `Some`, as the underlying engine may be able to do less work when only
    /// a boolean answer is needed.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\b\w{5}\b").unwrap();
    /// assert!(re.is_match("hello world"));
    /// assert!(!re.is_match("hi"));
    /// ```
    #[inline(always)]
    pub fn is_match(&self, haystack: &str) -> bool {
        if let Some(ok) = fast::is_match(self.fast, haystack, 0) {
            return ok;
        }
        self.find(haystack).is_some()
    }

    /// Returns `true` if and only if there is a match at or after `start`.
    ///
    /// The `start` offset must lie on a UTF-8 boundary.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert!(re.is_match_at("abc 123", 4));
    /// assert!(!re.is_match_at("abc 123", 7));
    /// ```
    pub fn is_match_at(&self, haystack: &str, start: usize) -> bool {
        self.find_at(haystack, start).is_some()
    }

    /// Returns the leftmost [`Match`] in `haystack`, or `None` if no match
    /// exists.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// let m = re.find("price: $42").unwrap();
    /// assert_eq!(m.as_str(), "42");
    /// assert_eq!(m.start(), 8);
    /// ```
    pub fn find<'h>(&self, haystack: &'h str) -> Option<Match<'h>> {
        self.captures(haystack).and_then(|c| c.get(0))
    }

    /// Returns the leftmost [`Match`] in `haystack` at or after `start`, or
    /// `None` if no match exists.
    ///
    /// The `start` offset must lie on a UTF-8 boundary.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert_eq!(re.find_at("1 22 333", 2).unwrap().as_str(), "22");
    /// ```
    pub fn find_at<'h>(&self, haystack: &'h str, start: usize) -> Option<Match<'h>> {
        self.captures_at(haystack, start).and_then(|c| c.get(0))
    }

    /// Returns the end offset of the leftmost match, or `None` if no match
    /// exists.
    ///
    /// This is equivalent to calling [`find`](Regex::find) and returning the
    /// end offset of the resulting [`Match`].
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert_eq!(re.shortest_match("abc 123 xyz"), Some(7));
    /// assert_eq!(re.shortest_match("abc"), None);
    /// ```
    pub fn shortest_match(&self, haystack: &str) -> Option<usize> {
        self.find(haystack).map(|m| m.end())
    }

    /// Returns the leftmost [`Captures`] for this regex in `haystack`, or
    /// `None` if no match exists.
    ///
    /// Group `0` is the whole match; groups `1`, `2`, … are the parenthesised
    /// sub-expressions.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\w+)@(\w+)\.(\w+)").unwrap();
    /// let caps = re.captures("user@example.com").unwrap();
    /// assert_eq!(&caps[1], "user");
    /// assert_eq!(&caps[2], "example");
    /// assert_eq!(&caps[3], "com");
    /// ```
    #[inline(always)]
    pub fn captures<'h>(&self, haystack: &'h str) -> Option<Captures<'h>> {
        self.captures_at(haystack, 0)
    }

    /// Returns the leftmost [`Captures`] in `haystack` at or after `start`, or
    /// `None` if no match exists.
    ///
    /// The `start` offset must lie on a UTF-8 boundary.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    /// let caps = re.captures_at("x=1 y=22", 4).unwrap();
    /// assert_eq!(&caps[1], "y");
    /// ```
    #[inline(always)]
    pub fn captures_at<'h>(&self, haystack: &'h str, start: usize) -> Option<Captures<'h>> {
        if let Some(slots) = fast::find(self.fast, haystack, 0) {
            if slots.get(0).is_some_and(|(s, _)| s >= start) {
                return Some(Captures { haystack, slots });
            }
        }
        if let Some(slots) = fast::find(self.fast, haystack, start) {
            return Some(Captures { haystack, slots });
        }
        matcher::find(&self.ast, haystack, self.captures, start, self.prefix).map(|slots| {
            Captures {
                haystack,
                slots: Slots::Heap(slots),
            }
        })
    }

    /// Returns an iterator over all non-overlapping [`Match`]es in
    /// `haystack`.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// let sum: u32 = re
    ///     .find_iter("1 plus 2 plus 3")
    ///     .map(|m| m.as_str().parse::<u32>().unwrap())
    ///     .sum();
    /// assert_eq!(sum, 6);
    /// ```
    pub fn find_iter<'r, 'h>(&'r self, haystack: &'h str) -> FindMatches<'r, 'h> {
        FindMatches {
            re: self,
            haystack,
            next: 0,
            done: false,
        }
    }

    /// Returns an iterator over all non-overlapping [`Captures`] in
    /// `haystack`.
    ///
    /// Use this when you need the capture groups for each match. If you only
    /// need the match boundaries prefer the cheaper [`find_iter`](Regex::find_iter).
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    /// let pairs: Vec<(&str, &str)> = re
    ///     .captures_iter("x=1 y=22 z=333")
    ///     .map(|c| (c.get(1).unwrap().as_str(), c.get(2).unwrap().as_str()))
    ///     .collect();
    /// assert_eq!(pairs, [("x", "1"), ("y", "22"), ("z", "333")]);
    /// ```
    pub fn captures_iter<'r, 'h>(&'r self, haystack: &'h str) -> CaptureMatches<'r, 'h> {
        CaptureMatches {
            re: self,
            haystack,
            next: 0,
            done: false,
        }
    }

    /// Returns an iterator over substrings of `haystack` delimited by matches
    /// of this regex.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r",\s*").unwrap();
    /// let fields: Vec<&str> = re.split("one, two,three").collect();
    /// assert_eq!(fields, ["one", "two", "three"]);
    /// ```
    pub fn split<'r, 'h>(&'r self, haystack: &'h str) -> Split<'r, 'h> {
        Split {
            matches: self.find_iter(haystack),
            last: 0,
            finished: false,
        }
    }

    /// Returns an iterator over at most `limit` substrings of `haystack`
    /// delimited by matches of this regex.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r",\s*").unwrap();
    /// let fields: Vec<&str> = re.splitn("one, two, three", 2).collect();
    /// assert_eq!(fields, ["one", "two, three"]);
    /// ```
    pub fn splitn<'r, 'h>(&'r self, haystack: &'h str, limit: usize) -> SplitN<'r, 'h> {
        SplitN {
            splits: self.split(haystack),
            limit,
            count: 0,
        }
    }

    /// Returns a copy-on-write string with the first match replaced by the output of
    /// `rep`.
    ///
    /// If no match is found the haystack is returned unchanged.
    ///
    /// `rep` can be a `&str` or `String` with `$1`-style group references, or
    /// a `FnMut(&Captures) -> String` closure. See [`Replacer`] for details.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert_eq!(re.replace("foo 1 bar 2", "X"), "foo X bar 2");
    /// ```
    pub fn replace<'h, R: Replacer>(&self, haystack: &'h str, mut rep: R) -> Cow<'h, str> {
        let Some(caps) = self.captures(haystack) else {
            return Cow::Borrowed(haystack);
        };
        let m = caps.get(0).unwrap();
        let mut dst = String::new();
        dst.push_str(&haystack[..m.start]);
        rep.replace_append(&caps, &mut dst);
        dst.push_str(&haystack[m.end..]);
        Cow::Owned(dst)
    }

    /// Returns a copy-on-write string with every non-overlapping match replaced
    /// by the output of `rep`.
    ///
    /// If no match is found the haystack is returned unchanged.
    ///
    /// `rep` can be a `&str` or `String` with `$1`-style group references, or
    /// a `FnMut(&Captures) -> String` closure. See [`Replacer`] for details.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert_eq!(re.replace_all("foo 1 bar 2 baz 3", "X"), "foo X bar X baz X");
    /// ```
    pub fn replace_all<'h, R: Replacer>(&self, haystack: &'h str, rep: R) -> Cow<'h, str> {
        self.replacen(haystack, 0, rep)
    }

    /// Returns a copy-on-write string with at most `limit` non-overlapping
    /// matches replaced by the output of `rep`.
    ///
    /// A `limit` of `0` replaces every match.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"\d+").unwrap();
    /// assert_eq!(re.replacen("foo 1 bar 2 baz 3", 2, "X"), "foo X bar X baz 3");
    /// ```
    pub fn replacen<'h, R: Replacer>(
        &self,
        haystack: &'h str,
        limit: usize,
        mut rep: R,
    ) -> Cow<'h, str> {
        let mut dst = String::new();
        let mut last = 0;
        let mut matched = false;
        for (i, caps) in self.captures_iter(haystack).enumerate() {
            if limit != 0 && i >= limit {
                break;
            }
            let Some(m) = caps.get(0) else {
                continue;
            };
            matched = true;
            dst.push_str(&haystack[last..m.start]);
            rep.replace_append(&caps, &mut dst);
            last = m.end;
        }
        if !matched {
            return Cow::Borrowed(haystack);
        }
        dst.push_str(&haystack[last..]);
        Cow::Owned(dst)
    }
}

impl RegexBuilder {
    /// Create a new builder for `pattern`.
    ///
    /// The builder can be configured with option methods and then compiled
    /// with [`build`](RegexBuilder::build).
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::RegexBuilder;
    ///
    /// let re = RegexBuilder::new(r"\d+").build().unwrap();
    /// assert!(re.is_match("abc 123"));
    /// ```
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_owned(),
            case_insensitive: false,
        }
    }

    /// Compile the configured regex.
    ///
    /// Returns an [`Error`] if the pattern contains invalid syntax.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::RegexBuilder;
    ///
    /// let re = RegexBuilder::new(r"\w+").build().unwrap();
    /// assert!(re.is_match("word"));
    ///
    /// assert!(RegexBuilder::new(r"[unclosed").build().is_err());
    /// ```
    pub fn build(&self) -> Result<Regex, Error> {
        if self.case_insensitive {
            Regex::new(&format!("(?i){}", self.pattern))
        } else {
            Regex::new(&self.pattern)
        }
    }

    /// Enable or disable ASCII case-insensitive matching.
    ///
    /// This has the same effect as prefixing the pattern with `(?i)`.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::RegexBuilder;
    ///
    /// let re = RegexBuilder::new("abc")
    ///     .case_insensitive(true)
    ///     .build()
    ///     .unwrap();
    /// assert!(re.is_match("ABC"));
    /// ```
    pub fn case_insensitive(&mut self, yes: bool) -> &mut Self {
        self.case_insensitive = yes;
        self
    }

    /// Enable or disable multi-line mode.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn multi_line(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Enable or disable allowing `.` to match `\n`.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn dot_matches_new_line(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Enable or disable CRLF-aware line anchors.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn crlf(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Enable or disable swapping greediness for repetition operators.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn swap_greed(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Enable or disable insignificant whitespace mode.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn ignore_whitespace(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Enable or disable Unicode mode.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn unicode(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Enable or disable octal escape syntax.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn octal(&mut self, _yes: bool) -> &mut Self {
        self
    }

    /// Set the approximate compiled regex size limit.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn size_limit(&mut self, _limit: usize) -> &mut Self {
        self
    }

    /// Set the approximate DFA cache size limit.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn dfa_size_limit(&mut self, _limit: usize) -> &mut Self {
        self
    }

    /// Set the nesting limit used while compiling a regex.
    ///
    /// This option is accepted for API compatibility and currently leaves
    /// matching behavior unchanged.
    pub fn nest_limit(&mut self, _limit: u32) -> &mut Self {
        self
    }
}

impl fmt::Display for Regex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'h> Match<'h> {
    /// Returns the byte offset of the start of the match.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the byte offset of the end of the match (exclusive).
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the byte range `start..end` of the match.
    pub fn range(&self) -> core::ops::Range<usize> {
        self.start..self.end
    }

    /// Returns the matched text as a string slice borrowed from the haystack.
    pub fn as_str(&self) -> &'h str {
        &self.haystack[self.start..self.end]
    }
}

impl<'h> Captures<'h> {
    /// Returns the `i`-th capture group as a [`Match`], or `None` if the
    /// group did not participate in the match.
    ///
    /// Group `0` is always the overall match. Groups `1`, `2`, … correspond
    /// to parenthesised sub-expressions.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::Regex;
    ///
    /// let re = Regex::new(r"(\d+)").unwrap();
    /// let caps = re.captures("price: 99").unwrap();
    /// assert_eq!(caps.get(1).unwrap().as_str(), "99");
    /// assert!(caps.get(2).is_none());
    /// ```
    pub fn get(&self, i: usize) -> Option<Match<'h>> {
        let (start, end) = self.slots.get(i)?;
        Some(Match {
            haystack: self.haystack,
            start,
            end,
        })
    }

    /// Returns the capture group with the given name, or `None` if no such
    /// group exists or the group did not participate in the match.
    ///
    /// This engine does not currently support named capture syntax, so this
    /// always returns `None`.
    pub fn name(&self, _name: &str) -> Option<Match<'h>> {
        None
    }

    /// Returns the total number of capture slots (including group 0).
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` if there are no capture slots.
    pub fn is_empty(&self) -> bool {
        self.slots.len() == 0
    }
}

impl Iterator for CaptureNames {
    type Item = Option<&'static str>;

    /// Advances the iterator and returns the next capture name, or `None` when exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            Some(None)
        }
    }
}

impl<'r, 'h> Iterator for SplitN<'r, 'h> {
    type Item = &'h str;

    /// Advances the iterator and returns the next split substring, or `None` when exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.limit {
            return None;
        }
        self.count += 1;
        if self.count == self.limit {
            self.splits.finished = true;
            Some(&self.splits.matches.haystack[self.splits.last..])
        } else {
            self.splits.next()
        }
    }
}

impl<'h> Index<&str> for Captures<'h> {
    type Output = str;

    /// Returns the text of a named capture group.
    ///
    /// ## Panics
    ///
    /// Panics because named capture groups are not currently supported.
    fn index(&self, name: &str) -> &Self::Output {
        self.name(name).unwrap().as_str()
    }
}

impl<'h> Index<usize> for Captures<'h> {
    type Output = str;

    /// Returns the text of capture group `index`.
    ///
    /// ## Panics
    ///
    /// Panics if `index` is out of bounds or the group did not participate in
    /// the match.
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        let (start, end) = self.slots.get(index).unwrap();
        unsafe { self.haystack.get_unchecked(start..end) }
    }
}

impl<'r, 'h> Iterator for FindMatches<'r, 'h> {
    type Item = Match<'h>;

    /// Advances the iterator and returns the next [`Match`], or `None` when exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let slots = if let Some(slots) = fast::find(self.re.fast, self.haystack, self.next) {
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

    /// Returns the total number of matches, using a fast-path counter when available.
    fn count(self) -> usize {
        if let Some(count) = fast::count(self.re.fast, self.haystack, self.next) {
            return count;
        }
        let mut count = 0;
        let mut iter = self;
        while iter.next().is_some() {
            count += 1;
        }
        count
    }
}

impl<'r, 'h> Iterator for CaptureMatches<'r, 'h> {
    type Item = Captures<'h>;

    /// Advances the iterator and returns the next [`Captures`], or `None` when exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let slots = if let Some(slots) = fast::find(self.re.fast, self.haystack, self.next) {
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

    /// Returns the total number of matches, using a fast-path counter when available.
    fn count(self) -> usize {
        if let Some(count) = fast::count(self.re.fast, self.haystack, self.next) {
            return count;
        }
        let mut count = 0;
        let mut iter = self;
        while iter.next().is_some() {
            count += 1;
        }
        count
    }
}

impl<'r, 'h> Iterator for Split<'r, 'h> {
    type Item = &'h str;

    /// Advances the iterator and returns the next substring between matches, or `None` when exhausted.
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
    /// Expands `$N` group references in this string and appends the result to `dst`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand(self, caps, dst);
    }
}

impl Replacer for String {
    /// Expands `$N` group references in this string and appends the result to `dst`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand(self, caps, dst);
    }
}

impl Replacer for &String {
    /// Expands `$N` group references in this string and appends the result to `dst`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        expand(self, caps, dst);
    }
}

impl<F> Replacer for F
where
    F: FnMut(&Captures<'_>) -> String,
{
    /// Calls the closure with `caps` and appends the returned string to `dst`.
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        dst.push_str(&self(caps));
    }
}

/// Expands `$N` references in `template` using `caps` and appends to `dst`.
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

/// Advances `pos` past the next UTF-8 character in `s`, or returns `pos` at end.
fn advance(s: &str, pos: usize) -> usize {
    if pos == s.len() {
        pos
    } else {
        s[pos..].chars().next().map_or(pos, |c| pos + c.len_utf8())
    }
}

/// Returns the first literal character of `ast` if the pattern starts with one.
fn literal_prefix(ast: &Ast) -> Option<char> {
    match ast {
        Ast::Literal(c) => Some(*c),
        Ast::Concat(nodes) => nodes.first().and_then(literal_prefix),
        Ast::Group { node, .. } => literal_prefix(node),
        _ => None,
    }
}
