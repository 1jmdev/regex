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
    /// Matches any character, optionally including `\n`.
    Dot { matches_new_line: bool },
    /// Matches a single character that satisfies a [`Class`].
    Class(Class),
    /// Anchors the match to the start of a line (`^`).
    StartLine { multi_line: bool, crlf: bool },
    /// Anchors the match to the end of a line (`$`).
    EndLine { multi_line: bool, crlf: bool },
    /// Anchors the match to the start of the string (`\A`).
    StartText,
    /// Anchors the match to the end of the string (`\z` or `\Z`).
    EndText,
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
    Group { index: usize, node: Box<Ast> },
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
    /// A Unicode general category or property shorthand (`\p{...}`).
    UnicodeProperty(String),
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
            ClassItem::UnicodeProperty(ref name) => matches_unicode_property(name, c),
        });
        hit != self.negated
    }
}

impl Ast {
    pub fn node_count(&self) -> usize {
        match self {
            Ast::Concat(nodes) | Ast::Alt(nodes) => {
                1 + nodes.iter().map(Ast::node_count).sum::<usize>()
            }
            Ast::Repeat { node, .. } | Ast::Group { node, .. } => 1 + node.node_count(),
            _ => 1,
        }
    }
}

fn matches_unicode_property(name: &str, c: char) -> bool {
    match name {
        "L" | "Letter" | "Alphabetic" | "Alpha" => c.is_alphabetic(),
        "N" | "Number" | "Nd" | "Decimal_Number" | "digit" => c.is_numeric(),
        "White_Space" | "Whitespace" | "space" => c.is_whitespace(),
        "Lowercase" | "Lower" | "Ll" => c.is_lowercase(),
        "Uppercase" | "Upper" | "Lu" => c.is_uppercase(),
        "Alnum" => c.is_alphanumeric(),
        "Any" => true,
        "ASCII" => c.is_ascii(),
        _ => false,
    }
}
