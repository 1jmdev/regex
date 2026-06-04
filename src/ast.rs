/// The abstract syntax tree produced by the regex parser.
///
/// Each variant represents one node in the parse tree. The tree is consumed
/// by both the interpreter in `matcher` and the fast-path classifier in
/// `fast`.
#[derive(Clone, Debug)]
pub enum Ast {
    /// Matches the empty string at the current position.
    Empty,
    /// Matches exactly one specific character.
    Literal(char),
    /// Matches any character except `\n`.
    Dot,
    /// Matches a single character that satisfies a [`Class`].
    Class(Class),
    /// Anchors the match to the start of the string (`^`).
    StartLine,
    /// Anchors the match to the end of the string (`$`).
    EndLine,
    /// Word boundary (`\b` when `true`) or non-boundary (`\B` when `false`).
    WordBoundary(bool),
    /// Matches all child nodes in sequence.
    Concat(Vec<Ast>),
    /// Matches any one of the child alternatives.
    Alt(Vec<Ast>),
    /// Matches `node` repeated between `min` and `max` times.
    Repeat {
        node: Box<Ast>,
        min: usize,
        max: Option<usize>,
        greedy: bool,
    },
    /// A numbered capturing group wrapping `node`.
    Group {
        index: usize,
        node: Box<Ast>,
    },
}

/// A character class such as `[a-z0-9]` or `\d`.
#[derive(Clone, Debug)]
pub struct Class {
    /// If `true` the class matches characters *not* in `items`.
    pub negated: bool,
    pub items: Vec<ClassItem>,
}

/// A single element inside a [`Class`].
#[derive(Clone, Debug)]
pub enum ClassItem {
    /// A literal character.
    Char(char),
    /// An inclusive character range `a..=b`.
    Range(char, char),
    /// ASCII digit shorthand (`\d`).
    Digit,
    /// ASCII word character shorthand (`\w`).
    Word,
    /// Whitespace shorthand (`\s`).
    Space,
}

impl Class {
    /// Returns `true` if `c` is matched by this character class.
    pub fn matches(&self, c: char) -> bool {
        let hit = self.items.iter().any(|item| match *item {
            ClassItem::Char(x) => x == c,
            ClassItem::Range(a, b) => a <= c && c <= b,
            ClassItem::Digit => c.is_ascii_digit(),
            ClassItem::Word => c.is_ascii_alphanumeric() || c == '_',
            ClassItem::Space => c.is_whitespace(),
        });
        hit != self.negated
    }
}
