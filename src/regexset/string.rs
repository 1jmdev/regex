use crate::{error::Error, regex::Regex};

/// A compiled set of regular expressions for searching Unicode string haystacks.
///
/// A `RegexSet` is created from many pattern strings and can be used to ask
/// which of those patterns match a haystack. It is useful when you only need to
/// know which regexes match, not where they match or which capture groups they
/// produced.
///
/// Patterns are tested with the same syntax and matching semantics as
/// [`Regex`]. Searching is unanchored by default — each regex is tried at every
/// position in the haystack. To force a match at the start or end use `^` / `$`.
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
/// use regex::RegexSet;
///
/// let set = RegexSet::new([r"\d+", r"\w+", r"^foo"]).unwrap();
/// let matches: Vec<usize> = set.matches("foo 123").iter().collect();
/// assert_eq!(matches, [0, 1, 2]);
/// ```
///
/// ## Example: test for any match
///
/// Use [`is_match`](RegexSet::is_match) when you only need a boolean answer.
///
/// ```
/// use regex::RegexSet;
///
/// let set = RegexSet::new([r"error", r"warning", r"critical"]).unwrap();
/// assert!(set.is_match("critical failure"));
/// assert!(!set.is_match("all clear"));
/// ```
///
/// ## Example: inspect individual patterns
///
/// The indexes reported by [`SetMatches`] correspond to the original pattern
/// order supplied to [`RegexSet::new`].
///
/// ```
/// use regex::RegexSet;
///
/// let set = RegexSet::new([r"foo", r"bar", r"baz"]).unwrap();
/// let matches = set.matches("bar and baz");
/// assert!(!matches.matched(0));
/// assert!(matches.matched(1));
/// assert!(matches.matched(2));
/// ```
#[derive(Clone, Debug)]
pub struct RegexSet {
    patterns: Vec<String>,
    regexes: Vec<Regex>,
}

/// The set of pattern indexes that matched a haystack.
///
/// Created by [`RegexSet::matches`]. A `SetMatches` value records one boolean
/// result for every pattern in the set. Use [`matched`](SetMatches::matched) to
/// test a specific pattern index or [`iter`](SetMatches::iter) to visit all
/// matching indexes from left to right.
///
/// ## Example
///
/// ```
/// use regex::RegexSet;
///
/// let set = RegexSet::new([r"\d+", r"[a-z]+", r"^foo"]).unwrap();
/// let matches = set.matches("foo 42");
/// assert!(matches.matched_any());
/// assert_eq!(matches.iter().collect::<Vec<_>>(), [0, 1, 2]);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetMatches {
    matched: Vec<bool>,
}

/// An iterator over the regex indexes that matched a haystack.
///
/// Created by [`SetMatches::iter`] or by iterating over `&SetMatches`. Yields
/// indexes in ascending pattern order.
///
/// ## Example
///
/// ```
/// use regex::RegexSet;
///
/// let set = RegexSet::new([r"foo", r"bar", r"baz"]).unwrap();
/// let matches = set.matches("foo baz");
/// let indexes: Vec<usize> = (&matches).into_iter().collect();
/// assert_eq!(indexes, [0, 2]);
/// ```
#[derive(Clone, Debug)]
pub struct SetMatchesIter<'m> {
    matched: &'m [bool],
    next: usize,
}

impl RegexSet {
    /// Compile many patterns into a `RegexSet`.
    ///
    /// Returns an [`Error`] if any pattern contains invalid syntax. Patterns
    /// are compiled in order, and the indexes returned by [`matches`](Self::matches)
    /// are the same indexes from the input iterator.
    ///
    /// ## Example
    ///
    /// ```
    /// use regex::RegexSet;
    ///
    /// let set = RegexSet::new([r"\d+", r"\w+"]).unwrap();
    /// assert_eq!(set.len(), 2);
    ///
    /// assert!(RegexSet::new([r"[unclosed"]).is_err());
    /// ```
    pub fn new<I, P>(patterns: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<str>,
    {
        let mut stored = Vec::new();
        let mut regexes = Vec::new();
        for pattern in patterns {
            let pattern = pattern.as_ref();
            regexes.push(Regex::new(pattern)?);
            stored.push(pattern.to_owned());
        }
        Ok(Self {
            patterns: stored,
            regexes,
        })
    }

    /// Returns the original pattern strings used to construct this `RegexSet`.
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// Returns the number of regexes in this set.
    pub fn len(&self) -> usize {
        self.regexes.len()
    }

    /// Returns `true` if this set contains no regexes.
    pub fn is_empty(&self) -> bool {
        self.regexes.is_empty()
    }

    /// Returns `true` if and only if at least one regex in this set matches.
    ///
    /// Prefer this over calling [`matches`](RegexSet::matches) when only a
    /// boolean answer is needed.
    pub fn is_match(&self, haystack: &str) -> bool {
        self.regexes.iter().any(|re| re.is_match(haystack))
    }

    /// Returns the set of regex indexes that match `haystack`.
    ///
    /// The returned [`SetMatches`] has one slot per regex in this set. Matching
    /// indexes are yielded in ascending pattern order.
    pub fn matches(&self, haystack: &str) -> SetMatches {
        SetMatches {
            matched: self
                .regexes
                .iter()
                .map(|re| re.is_match(haystack))
                .collect(),
        }
    }
}

impl SetMatches {
    /// Returns `true` if the regex at index `i` matched the haystack.
    ///
    /// Returns `false` when `i` is out of bounds.
    pub fn matched(&self, i: usize) -> bool {
        self.matched.get(i).copied().unwrap_or(false)
    }

    /// Returns `true` if at least one regex in the set matched the haystack.
    pub fn matched_any(&self) -> bool {
        self.matched.iter().any(|&matched| matched)
    }

    /// Returns the number of regexes represented by this match set.
    pub fn len(&self) -> usize {
        self.matched.len()
    }

    /// Returns `true` if this match set contains no regex results.
    pub fn is_empty(&self) -> bool {
        self.matched.is_empty()
    }

    /// Returns an iterator over matching regex indexes in ascending order.
    pub fn iter(&self) -> SetMatchesIter<'_> {
        SetMatchesIter {
            matched: &self.matched,
            next: 0,
        }
    }
}

impl<'m> Iterator for SetMatchesIter<'m> {
    type Item = usize;

    /// Advances the iterator and returns the next matching regex index, or `None` when exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.matched.len() {
            let i = self.next;
            self.next += 1;
            if self.matched[i] {
                return Some(i);
            }
        }
        None
    }
}

impl IntoIterator for SetMatches {
    type Item = usize;
    type IntoIter = std::vec::IntoIter<usize>;

    /// Consumes this match set and returns an iterator over matching regex indexes.
    fn into_iter(self) -> Self::IntoIter {
        self.matched
            .into_iter()
            .enumerate()
            .filter_map(|(i, matched)| matched.then_some(i))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<'m> IntoIterator for &'m SetMatches {
    type Item = usize;
    type IntoIter = SetMatchesIter<'m>;

    /// Borrows this match set and returns an iterator over matching regex indexes.
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
